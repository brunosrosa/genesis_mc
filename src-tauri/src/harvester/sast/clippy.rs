use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::harvester::router::StaticAnalysisBlade;
use super::{SandboxExecutor, SidecarError, SidecarExitPolicy, SastExecutionTarget, SodaHealthIssue, execute_sidecar_in_dir, parse_json_payload, push_issue, sort_and_dedup_issues, sanitize_host_paths_in_text, DiscoveredManifest, ManifestKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustClippyPlan {
    pub command_args: Vec<String>,
}

const RUST_NATIVE_BUILD_MARKERS: &[&str] = &[
    "cuda",
    "cudarc",
    "cublas",
    "cudnn",
    "nccl",
    "metal",
    "objc",
    "objc2",
    "core-ml",
    "bindgen",
    "autocxx",
    "cxx",
    "cmake",
    "pkg-config",
    "openssl-sys",
    "libz-sys",
    "clang-sys",
    "torch-sys",
];

pub fn clippy_args_for_package(package_name: &str) -> Vec<String> {
    vec![
        "clippy".to_string(),
        "--workspace".to_string(),
        "--offline".to_string(),
        "--no-deps".to_string(),
        "--message-format=json".to_string(),
        "-p".to_string(),
        package_name.to_string(),
    ]
}

pub fn default_clippy_args() -> Vec<String> {
    vec![
        "clippy".to_string(),
        "--workspace".to_string(),
        "--offline".to_string(),
        "--no-deps".to_string(),
        "--message-format=json".to_string(),
    ]
}

pub fn cargo_lockfile_path(manifest_path: &Path) -> PathBuf {
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("Cargo.lock")
}

pub fn cargo_fetch_args(manifest_path: &Path, use_locked: bool) -> Vec<String> {
    let mut args = vec!["fetch".to_string()];
    if use_locked {
        args.push("--locked".to_string());
    }
    args.push("--manifest-path".to_string());
    args.push(manifest_path.display().to_string());
    args
}

pub fn cargo_metadata_args(manifest_path: &Path, use_locked: bool) -> Vec<String> {
    let mut args = vec![
        "metadata".to_string(),
        "--format-version".to_string(),
        "1".to_string(),
        "--no-deps".to_string(),
    ];
    if use_locked {
        args.push("--locked".to_string());
    }
    args.push("--offline".to_string());
    args.push("--manifest-path".to_string());
    args.push(manifest_path.display().to_string());
    args
}

pub fn rust_clippy_manifest_path(execution_root: &Path) -> PathBuf {
    execution_root.join("Cargo.toml")
}

pub fn find_cargo_workspace_root(repo_path: &Path, execution_root: &Path) -> PathBuf {
    let mut current = execution_root.to_path_buf();
    let mut workspace_root = execution_root.to_path_buf();

    while current.starts_with(repo_path) {
        if current.join("Cargo.toml").is_file() {
            workspace_root = current.clone();
        }
        if current == repo_path {
            break;
        }
        if !current.pop() {
            break;
        }
    }
    workspace_root
}

pub fn rust_clippy_preflight_timeout_secs(timeout_secs: u64) -> u64 {
    timeout_secs.clamp(60, 180)
}

pub fn manifest_effectively_has_build_script(
    package_root: &Path,
    package_table: &toml::value::Table,
) -> bool {
    match package_table.get("build") {
        Some(toml::Value::Boolean(false)) => false,
        Some(toml::Value::String(value)) => !value.trim().is_empty(),
        Some(_) => true,
        None => package_root.join("build.rs").is_file(),
    }
}

pub fn rust_manifest_native_marker(value: &toml::Value) -> Option<&'static str> {
    fn marker_in_text(text: &str) -> Option<&'static str> {
        let normalized = text.to_ascii_lowercase();
        RUST_NATIVE_BUILD_MARKERS
            .iter()
            .copied()
            .find(|marker| normalized.contains(marker))
    }

    match value {
        toml::Value::String(text) => marker_in_text(text),
        toml::Value::Array(items) => items.iter().find_map(rust_manifest_native_marker),
        toml::Value::Table(entries) => entries.iter().find_map(|(key, inner)| {
            marker_in_text(key).or_else(|| rust_manifest_native_marker(inner))
        }),
        _ => None,
    }
}

pub fn build_rust_clippy_plan(manifest: &DiscoveredManifest) -> Result<RustClippyPlan, String> {
    let manifest_text = std::fs::read_to_string(&manifest.manifest_path).map_err(|error| {
        format!(
            "nao foi possivel ler {}: {error}",
            manifest.manifest_path.display()
        )
    })?;
    let manifest_value = manifest_text
        .parse::<toml::Table>()
        .map_err(|error| format!("manifesto TOML invalido em {}: {error}", manifest.scope))?;
    let package_table = manifest_value
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "manifesto virtual/workspace sem [package]".to_string())?;
    let package_name = package_table
        .get("name")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "manifesto sem [package].name".to_string())?;

    Ok(RustClippyPlan {
        command_args: clippy_args_for_package(package_name),
    })
}

#[derive(Debug, serde::Deserialize)]
struct CargoMetadataPackage {
    manifest_path: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
struct CargoMetadataPayload {
    packages: Vec<CargoMetadataPackage>,
}

pub fn fail_closed_rust_manifest(reason: String) -> SidecarError {
    SidecarError::ExecutionFailed {
        reason: format!("cargo-clippy fail-closed: {reason}"),
    }
}

pub fn rust_clippy_should_fallback_to_opengrep(err: &SidecarError) -> Option<String> {
    match err {
        SidecarError::ExecutionFailed { reason }
            if reason.starts_with("cargo-clippy fail-closed:") =>
        {
            Some(
                reason
                    .trim_start_matches("cargo-clippy fail-closed:")
                    .trim()
                    .to_string(),
            )
        }
        _ => None,
    }
}

pub fn audit_transitive_rust_manifests(
    repo_path: &Path,
    metadata_bytes: &[u8],
) -> Result<(), SidecarError> {
    let payload = parse_json_payload::<CargoMetadataPayload>(metadata_bytes)?;
    let mut manifests = payload
        .packages
        .into_iter()
        .map(|package| crate::harvester::path_sanitizer::soda_strip_unc_prefix(&package.manifest_path))
        .collect::<Vec<_>>();
    manifests.sort();
    manifests.dedup();

    if manifests.is_empty() {
        return Err(fail_closed_rust_manifest(
            "cargo metadata nao retornou manifestos para auditoria transitiva".to_string(),
        ));
    }

    for manifest_path in manifests {
        let manifest_path = if manifest_path.is_absolute() {
            manifest_path
        } else {
            repo_path.join(manifest_path)
        };
        let manifest_text = std::fs::read_to_string(&manifest_path).map_err(|error| {
            fail_closed_rust_manifest(format!(
                "nao foi possivel ler manifesto transitivo '{}': {error}",
                sanitize_host_paths_in_text(repo_path, &manifest_path.display().to_string())
            ))
        })?;
        let _manifest_value = manifest_text.parse::<toml::Table>().map_err(|error| {
            fail_closed_rust_manifest(format!(
                "manifesto transitivo invalido em '{}': {error}",
                sanitize_host_paths_in_text(repo_path, &manifest_path.display().to_string())
            ))
        })?;
        // PRD-033: blindagem removida — crates com build.rs/FFI/proc-macro agora
        // tentam compilar; falhas são capturadas forensicamente em mod.rs.
    }

    Ok(())
}

pub fn expand_cargo_workspace_wildcards(workspace_root: &Path) -> Result<(), String> {
    let manifest_path = workspace_root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Ok(());
    }

    let manifest_text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Falha ao ler manifest raiz: {e}"))?;

    let mut manifest_value = manifest_text
        .parse::<toml::Table>()
        .map_err(|e| format!("TOML invalido no manifest raiz: {e}"))?;

    let workspace_table = match manifest_value.get_mut("workspace").and_then(|v| v.as_table_mut()) {
        Some(t) => t,
        None => return Ok(()),
    };

    let members_array = match workspace_table.get_mut("members").and_then(|v| v.as_array_mut()) {
        Some(arr) => arr,
        None => return Ok(()),
    };

    let mut new_members = Vec::new();
    let mut modified = false;

    for member_val in members_array.iter() {
        let member_str = match member_val.as_str() {
            Some(s) => s,
            None => {
                new_members.push(member_val.clone());
                continue;
            }
        };

        if member_str.contains('*') {
            modified = true;
            let pattern = member_str.replace('\\', "/");
            let parts: Vec<&str> = pattern.split('/').collect();
            
            if parts.len() == 2 && parts[1] == "*" {
                let prefix_dir = workspace_root.join(parts[0]);
                if prefix_dir.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(&prefix_dir) {
                        let mut sorted_entries = Vec::new();
                        for entry in entries.flatten() {
                            if let Ok(file_type) = entry.file_type() {
                                if file_type.is_dir() {
                                    let sub_dir = entry.path();
                                    if sub_dir.join("Cargo.toml").is_file() {
                                        if let Some(name) = sub_dir.file_name().and_then(|n| n.to_str()) {
                                            sorted_entries.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        sorted_entries.sort();
                        for name in sorted_entries {
                            let expanded_member = format!("{}/{}", parts[0], name);
                            new_members.push(toml::Value::String(expanded_member));
                        }
                    }
                }
            } else {
                new_members.push(member_val.clone());
            }
        } else {
            new_members.push(member_val.clone());
        }
    }

    if modified {
        *members_array = new_members;
        let new_text = toml::to_string(&manifest_value)
            .map_err(|e| format!("Falha ao serializar manifesto modificado: {e}"))?;
        std::fs::write(&manifest_path, new_text)
            .map_err(|e| format!("Falha ao gravar manifesto modificado: {e}"))?;
        info!(
            path = %manifest_path.display(),
            "SAST rust-clippy: wildcard de workspace expandido com sucesso no Cargo.toml"
        );
    }

    Ok(())
}

pub async fn run_rust_clippy_preflight<E: SandboxExecutor>(
    executor: &E,
    execution_root: &Path,
    timeout_secs: u64,
) -> Result<(), SidecarError> {
    let manifest_path = rust_clippy_manifest_path(execution_root);
    if !manifest_path.is_file() {
        return Err(fail_closed_rust_manifest(format!(
            "manifest-path ausente para preflight: {}",
            sanitize_host_paths_in_text(executor.repo_path(), &manifest_path.display().to_string())
        )));
    }

    // Expand wildcard workspace members before running cargo fetch/metadata/clippy
    let workspace_root = find_cargo_workspace_root(executor.repo_path(), execution_root);
    if let Err(e) = expand_cargo_workspace_wildcards(&workspace_root) {
        warn!(
            workspace_root = %workspace_root.display(),
            error = %e,
            "SAST rust-clippy: Falha ao expandir wildcards de workspace (prosseguindo com manifesto original)"
        );
    }


    let preflight_timeout_secs = rust_clippy_preflight_timeout_secs(timeout_secs);
    let lockfile_path = cargo_lockfile_path(&manifest_path);

    // Pre-Flight Fetch nativo no host com rede habilitada para alimentar o cache do Cargo de forma assíncrona.
    // Nos testes unitários mockados, usamos o mock do executor para simular a chamada e evitar conexões de rede reais.
    if cfg!(test) {
        let fetch_args = cargo_fetch_args(&manifest_path, lockfile_path.is_file());
        let fetch_arg_refs = fetch_args.iter().map(String::as_str).collect::<Vec<_>>();
        execute_sidecar_in_dir(
            executor,
            "cargo",
            &fetch_arg_refs,
            preflight_timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
            execution_root,
        )
        .await?;
    } else {
        info!(
            manifest_path = %manifest_path.display(),
            "SAST rust-clippy: Executando Pre-Flight cargo fetch assincrono no host com rede habilitada"
        );
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.arg("fetch")
           .arg("--manifest-path")
           .arg(&manifest_path)
           .current_dir(execution_root);

        if lockfile_path.is_file() {
            cmd.arg("--locked");
        }

        cmd.stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped())
           .kill_on_drop(true);

        match cmd.spawn() {
            Ok(child) => {
                let wait_fut = child.wait_with_output();
                match tokio::time::timeout(std::time::Duration::from_secs(120), wait_fut).await {
                    Ok(Ok(output)) => {
                        if output.status.success() {
                            info!(
                                manifest_path = %manifest_path.display(),
                                "SAST rust-clippy: Pre-Flight cargo fetch concluido com sucesso"
                            );
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            return Err(SidecarError::ExecutionFailed {
                                reason: format!(
                                    "Pre-Flight cargo fetch falhou para '{}': {}",
                                    manifest_path.display(),
                                    stderr.trim()
                                ),
                            });
                        }
                    }
                    Ok(Err(e)) => {
                        return Err(SidecarError::ExecutionFailed {
                            reason: format!(
                                "Erro ao executar Pre-Flight cargo fetch para '{}': {e}",
                                manifest_path.display()
                            ),
                        });
                    }
                    Err(_) => {
                        return Err(SidecarError::ExecutionFailed {
                            reason: format!(
                                "Timeout no Pre-Flight cargo fetch para '{}'",
                                manifest_path.display()
                            ),
                        });
                    }
                }
            }
            Err(e) => {
                return Err(SidecarError::ExecutionFailed {
                    reason: format!(
                        "Falha ao iniciar Pre-Flight cargo fetch assincrono para '{}': {e}",
                        manifest_path.display()
                    ),
                });
            }
        }
    }

    // L14: Inteligência Topológica de Workspace (ADR-025).
    // O cargo metadata com --manifest-path pointing to a sub-crate of a workspace
    // RETURNS exit_code=1 because it can't resolve workspace members from a sub-crate manifest.
    // We MUST run cargo metadata from the WORKSPACE ROOT (no --manifest-path) to get
    // proper workspace member information. This is the CORRECT behavior, not a band-aid.
    let metadata_run_dir: &std::path::Path;
    let metadata_args_for_run: Vec<String>;

    let ws_root = find_cargo_workspace_root(executor.repo_path(), execution_root);
    if ws_root != execution_root {
        // Workspace detectado — executar cargo metadata a partir da RAIZ do workspace
        // SEM --manifest-path para que o cargo auto-descubra todos os membros do workspace.
        // Passing --manifest-path pointing to a sub-crate from the workspace root still
        // causes cargo to only show that sub-crate's view (exit_code=1 on full workspaces).
        debug!(
            execution_root = %execution_root.display(),
            workspace_root = %ws_root.display(),
            "SAST rust-clippy: Detectado workspace. Executando cargo metadata a partir da raiz do workspace."
        );
        metadata_run_dir = ws_root.as_path();
        // Build args WITHOUT --manifest-path and WITHOUT --locked for workspace root.
        // The --locked flag is unsafe for workspace root (may not have a lockfile for root).
        metadata_args_for_run = vec![
            "metadata".to_string(),
            "--format-version".to_string(),
            "1".to_string(),
            "--offline".to_string(),
            "--no-deps".to_string(),
        ];
    } else {
        // Não é workspace — usar o manifest do sub-crate como antes.
        metadata_run_dir = execution_root;
        metadata_args_for_run = cargo_metadata_args(&manifest_path, lockfile_path.is_file());
    }

    let metadata_arg_refs = metadata_args_for_run.iter().map(String::as_str).collect::<Vec<_>>();

    let metadata_bytes = execute_sidecar_in_dir(
        executor,
        "cargo",
        &metadata_arg_refs,
        preflight_timeout_secs,
        SidecarExitPolicy::StrictZeroOnly,
        metadata_run_dir,
    )
    .await?;

    audit_transitive_rust_manifests(executor.repo_path(), &metadata_bytes)
}

pub fn derive_rust_clippy_execution_targets(manifests: &[DiscoveredManifest]) -> Vec<SastExecutionTarget> {
    manifests
        .iter()
        .filter(|manifest| manifest.kind == ManifestKind::CargoToml)
        .filter_map(|manifest| match build_rust_clippy_plan(manifest) {
            Ok(plan) => Some(SastExecutionTarget {
                blade: StaticAnalysisBlade::RustClippy,
                execution_root: manifest.execution_root.clone(),
                scope: manifest.scope.clone(),
                scan_targets: vec![".".to_string()],
                command_args: Some(plan.command_args),
                forced_channel: None,
            }),
            Err(reason) => {
                // PRD-033: build_rust_clippy_plan agora só falha em erros
                // de leitura de TOML/manifest — não filtra mais por FFI.
                info!(
                    manifest = %manifest.manifest_path.display(),
                    scope = %manifest.scope,
                    reason = %reason,
                    "SAST rust-clippy: manifesto ignorado (erro de leitura/parse)"
                );
                None
            }
        })
        .collect()
}

pub fn normalize_clippy_output(
    repo_path: &Path,
    execution_root: &Path,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    let text = String::from_utf8_lossy(bytes);
    let mut issues = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("reason").and_then(|v| v.as_str()) != Some("compiler-message") {
            continue;
        }
        let message_obj = match value.get("message") {
            Some(value) => value,
            None => continue,
        };
        let level = message_obj
            .get("level")
            .and_then(|value| value.as_str())
            .unwrap_or("warning");
        let message = message_obj
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or("clippy finding");
        let file = message_obj
            .get("spans")
            .and_then(|value| value.as_array())
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|span| span.get("is_primary").and_then(|value| value.as_bool()).unwrap_or(false))
                    .or_else(|| spans.first())
            })
            .and_then(|span| span.get("file_name"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::RustClippy,
            level,
            file,
            message,
        );
    }
    sort_and_dedup_issues(&mut issues);
    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::sast::test_utils::MockExecutor;
    use crate::harvester::sast::run_sast_blade;
    use crate::harvester::sandbox::SandboxError;

    #[test]
    fn test_expand_cargo_workspace_wildcards() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        
        let initial_toml = r#"[workspace]
members = ["crates/*"]
"#;
        std::fs::write(root.join("Cargo.toml"), initial_toml).unwrap();
        
        std::fs::create_dir_all(root.join("crates").join("foo")).unwrap();
        std::fs::write(root.join("crates").join("foo").join("Cargo.toml"), "[package]\nname = 'foo'").unwrap();
        
        std::fs::create_dir_all(root.join("crates").join("bar")).unwrap();
        std::fs::write(root.join("crates").join("bar").join("Cargo.toml"), "[package]\nname = 'bar'").unwrap();

        // ignore directory without Cargo.toml
        std::fs::create_dir_all(root.join("crates").join("baz")).unwrap();

        expand_cargo_workspace_wildcards(root).unwrap();

        let modified_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(modified_toml.contains("crates/foo"));
        assert!(modified_toml.contains("crates/bar"));
        assert!(!modified_toml.contains("crates/baz"));
        assert!(!modified_toml.contains("crates/*"));
    }

    #[test]
    fn test_normalize_clippy_messages_to_soda_health_issue() {
        let repo_path = Path::new(r"C:\host\projfs\owner\repo");
        let payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"manual memcpy can be replaced with copy_from_slice","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}
{"reason":"compiler-message","message":{"level":"error","message":"called `Result::unwrap()` on an `Err` value","spans":[{"file_name":"src\\main.rs","is_primary":true}]}}"#;

        let normalized = normalize_clippy_output(
            repo_path,
            repo_path,
            payload.as_bytes(),
        )
        .unwrap();

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].level, "warning");
        assert_eq!(normalized[0].file, "src/lib.rs");
        assert!(normalized[0].message.contains("copy_from_slice"));
        assert_eq!(normalized[1].level, "error");
        assert_eq!(normalized[1].file, "src/main.rs");
        assert!(normalized[1].message.contains("unwrap"));
    }

    #[test]
    fn test_derive_rust_clippy_targets_includes_all_manifests_including_ffi() {
        // PRD-033 Lei 1: a blindagem foi removida. Todos os manifestos Rust são alvos agora.
        // Crates com cudarc/objc2/metal são incluídos; se falharem ao compilar,
        // o diagnóstico forense é injetado no Blob 08 pelo mod.rs.
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("Cargo.toml", "[package]\nname='root'\nversion='0.1.0'\n");
        executor.write_repo_file(
            "crates/cuda/Cargo.toml",
            "[package]\nname='cuda-kernel'\nversion='0.1.0'\n[dependencies]\ncudarc='0.12'\n",
        );
        executor.write_repo_file(
            "crates/apple/Cargo.toml",
            "[package]\nname='metal-kernel'\nversion='0.1.0'\n[dependencies]\nobjc2='0.6'\nmetal='0.31'\n",
        );
        let manifests = crate::harvester::sast::discover_monorepo_manifests(executor.repo_path());

        let targets = derive_rust_clippy_execution_targets(&manifests);

        // Todos os 3 manifestos são alvo agora (sem blindagem)
        assert_eq!(targets.len(), 3);
        let scopes: Vec<&str> = targets.iter().map(|t| t.scope.as_str()).collect();
        assert!(scopes.contains(&"."), "root deve ser alvo");
        assert!(scopes.iter().any(|s| s.contains("cuda")), "cuda deve ser alvo");
        assert!(scopes.iter().any(|s| s.contains("apple")), "apple deve ser alvo");
    }

    #[tokio::test]
    async fn test_run_sast_blade_cleans_clippy_target_dir_after_execution() {
        let clippy_payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"lint in workspace member","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}"#;
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("apps/rust-sdk/Cargo.toml", "[package]\nname='sdk'\nversion='0.1.0'\n");
        let execution_root = executor.repo_path().join("apps").join("rust-sdk");
        executor.write_repo_file("apps/rust-sdk/Cargo.lock", "version = 3\n");
        let manifest_path = execution_root.join("Cargo.toml");
        let metadata_payload = serde_json::json!({
            "packages": [
                {
                    "manifest_path": manifest_path.display().to_string()
                }
            ]
        })
        .to_string();
        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Ok(Vec::new()),
            Ok(metadata_payload.as_bytes().to_vec()),
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 1,
                stderr: "findings".to_string(),
                stdout: clippy_payload.as_bytes().to_vec(),
            }),
        ]);
        let cache_root =
            crate::harvester::sandbox::sandbox_tool_state_root(&execution_root, "cargo-clippy-target");
        let cargo_home =
            crate::harvester::sandbox::sandbox_tool_state_root(&execution_root, "cargo-home");
        std::fs::create_dir_all(cache_root.join("debug")).unwrap();
        std::fs::write(cache_root.join("debug").join(".keep"), "temp").unwrap();
        std::fs::create_dir_all(cargo_home.join("registry").join("cache")).unwrap();
        std::fs::write(cargo_home.join("registry").join("cache").join(".keep"), "temp").unwrap();

        let payload = run_sast_blade(
            &executor,
            StaticAnalysisBlade::RustClippy,
            60,
            &execution_root,
            "apps/rust-sdk",
            &[".".to_string()],
            Some(&clippy_args_for_package("sdk")),
            None,
            false,
        )
        .await
        .unwrap();

        let calls = executor.calls();
        assert_eq!(payload.effective_blade, StaticAnalysisBlade::RustClippy);
        assert!(!payload.bytes.is_empty());
        assert!(calls[0].starts_with("cargo fetch --locked --manifest-path "));
        assert!(calls[1].starts_with("cargo metadata --format-version 1 --no-deps --locked --offline --manifest-path "));
        assert!(calls[2].starts_with("cargo clippy --manifest-path "));
        assert!(calls[2].contains("-p sdk"));
        assert!(calls[2].contains("--no-deps"));
        assert!(!cache_root.exists());
        assert!(!cargo_home.exists());
    }

    #[tokio::test]
    async fn test_run_sast_blade_clippy_runs_on_crate_with_build_deps_transitive() {
        // PRD-033 Lei 1: a blindagem foi removida. Crates com build-dependencies
        // transitivos não disparam mais o fallback para opengrep;
        // o clippy tenta compilar e falhas são capturadas forensicamente.
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("apps/rust-sdk/Cargo.toml", "[package]\nname='sdk'\nversion='0.1.0'\n");
        executor.write_repo_file(
            "vendor/toxic/Cargo.toml",
            "[package]\nname='toxic'\nversion='0.1.0'\n\n[build-dependencies]\ncc='1'\n",
        );
        let execution_root = executor.repo_path().join("apps").join("rust-sdk");
        let metadata_payload = serde_json::json!({
            "packages": [
                {
                    "manifest_path": execution_root.join("Cargo.toml").display().to_string()
                },
                {
                    "manifest_path": executor
                        .repo_path()
                        .join("vendor")
                        .join("toxic")
                        .join("Cargo.toml")
                        .display()
                        .to_string()
                }
            ]
        })
        .to_string();
        // Clippy falha ao compilar (cc build-dep não disponivel no sandbox)
        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Ok(Vec::new()),                           // cargo fetch (sem lockfile)
            Ok(metadata_payload.as_bytes().to_vec()), // cargo metadata
            Err(SandboxError::ProcessNonZeroExit {    // cargo clippy falha
                exit_code: 1,
                stderr: "error: could not compile `toxic` (build script)".to_string(),
                stdout: Vec::new(),
            }),
        ]);

        let result = run_sast_blade(
            &executor,
            StaticAnalysisBlade::RustClippy,
            60,
            &execution_root,
            "apps/rust-sdk",
            &[".".to_string()],
            Some(&clippy_args_for_package("sdk")),
            None,
            false,
        )
        .await;

        let calls = executor.calls();
        // Clippy agora tenta compilar (3 calls: fetch + metadata + clippy)
        assert_eq!(calls.len(), 3, "fetch + metadata + clippy devem ser chamados");
        assert!(calls[0].starts_with("cargo fetch --manifest-path "));
        assert!(calls[1].starts_with("cargo metadata"));
        assert!(calls[2].starts_with("cargo clippy"));
        // run_sast_blade retorna Err (o tratamento forense ocorre em mod.rs acima)
        assert!(
            result.is_err(),
            "run_sast_blade deve retornar Err quando clippy falha fatalmente"
        );
    }

    #[tokio::test]
    async fn test_run_sast_blade_with_build_deps_and_global_coverage_runs_clippy_normally() {
        // PRD-033: com global_coverage=true e crate com build-deps transitivos,
        // o clippy ainda tenta rodar (blindagem removida). Se não há lockfile,
        // fá fetch sem --locked e metadata sem --locked antes do clippy.
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("apps/rust-sdk/Cargo.toml", "[package]\nname='sdk'\nversion='0.1.0'\n");
        executor.write_repo_file(
            "vendor/toxic/Cargo.toml",
            "[package]\nname='toxic'\nversion='0.1.0'\n\n[build-dependencies]\ncc='1'\n",
        );
        let execution_root = executor.repo_path().join("apps").join("rust-sdk");
        let metadata_payload = serde_json::json!({
            "packages": [
                {
                    "manifest_path": execution_root.join("Cargo.toml").display().to_string()
                },
                {
                    "manifest_path": executor
                        .repo_path()
                        .join("vendor")
                        .join("toxic")
                        .join("Cargo.toml")
                        .display()
                        .to_string()
                }
            ]
        })
        .to_string();
        // Com global_coverage=true, o clippy tenta rodar com bytes vazio em caso de sucesso sem achados
        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Ok(Vec::new()),                           // cargo fetch
            Ok(metadata_payload.as_bytes().to_vec()), // cargo metadata
            Ok(Vec::new()),                           // cargo clippy (sem achados)
        ]);

        let result = run_sast_blade(
            &executor,
            StaticAnalysisBlade::RustClippy,
            60,
            &execution_root,
            "apps/rust-sdk",
            &[".".to_string()],
            Some(&clippy_args_for_package("sdk")),
            None,
            true, // global_coverage=true
        )
        .await
        .unwrap();

        let calls = executor.calls();
        assert_eq!(result.effective_blade, StaticAnalysisBlade::RustClippy);
        // 3 calls: fetch + metadata + clippy
        assert_eq!(calls.len(), 3);
        assert!(calls[0].starts_with("cargo fetch --manifest-path "));
        assert!(!calls[0].contains("--locked"));
        assert!(calls[1].starts_with("cargo metadata --format-version 1 --no-deps --offline --manifest-path "));
        assert!(!calls[1].contains("--locked"));
        assert!(calls[2].starts_with("cargo clippy"));
    }
}
