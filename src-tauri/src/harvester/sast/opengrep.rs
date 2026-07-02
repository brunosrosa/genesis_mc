use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as AsyncMutex;
use serde::{Deserialize, Serialize};

use super::{SandboxExecutor, SidecarError, SidecarExitPolicy, ScopedTextBlock, sanitize_repo_relative_path, execute_sidecar, parse_json_payload, render_scoped_text_blocks, SemgrepArtifacts};

pub const SEMGREP_SECURITY_RULE_FILE: &str = ".soda_semgrep_blob_06_security.yml";
pub const SEMGREP_HEALTH_RULE_FILE: &str = ".soda_semgrep_blob_08_health.yml";
const SEMGREP_SECURITY_RULE_SOURCE: &str = include_str!("../../../semgrep/blob_06_security.yml");
const SEMGREP_HEALTH_RULE_SOURCE: &str = include_str!("../../../semgrep/blob_08_health.yml");

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SemgrepRuleSet {
    Security,
    Health,
}

impl SemgrepRuleSet {
    pub(crate) fn config_dir_name(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::Health => "health",
        }
    }

    pub(crate) fn rule_file_name(self) -> &'static str {
        match self {
            Self::Security => SEMGREP_SECURITY_RULE_FILE,
            Self::Health => SEMGREP_HEALTH_RULE_FILE,
        }
    }

    pub(crate) fn rule_source(self) -> &'static str {
        match self {
            Self::Security => SEMGREP_SECURITY_RULE_SOURCE,
            Self::Health => SEMGREP_HEALTH_RULE_SOURCE,
        }
    }

    pub(crate) fn copies_workspace_rules(self) -> bool {
        matches!(self, Self::Security)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SemgrepNormalizedPayload {
    pub blocks: Vec<ScopedTextBlock>,
    pub files_analyzed: usize,
    pub findings_count: usize,
}

pub struct SemgrepInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}

pub struct SemgrepSidecar;

impl SemgrepSidecar {
    pub async fn extract<E: SandboxExecutor>(
        input: SemgrepInput<'_, E>,
    ) -> Result<SemgrepArtifacts, SidecarError> {
        let security_bytes = run_semgrep_scan(input.executor, SemgrepRuleSet::Security, input.timeout_secs).await?;
        let security_payload = normalize_semgrep_payload(input.executor.repo_path(), &security_bytes)?;

        let health_bytes = run_semgrep_scan(input.executor, SemgrepRuleSet::Health, input.timeout_secs).await?;
        let health_payload = normalize_semgrep_payload(input.executor.repo_path(), &health_bytes)?;

        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            unsafe_hotspots_bytes = render_semgrep_blob(SemgrepRuleSet::Security, &security_payload).len(),
            health_report_bytes = render_semgrep_blob(SemgrepRuleSet::Health, &health_payload).len(),
            "Semgrep: artefatos normalizados"
        );
        Ok(SemgrepArtifacts {
            unsafe_hotspots_blob: render_semgrep_blob(SemgrepRuleSet::Security, &security_payload),
            health_report_blob: render_semgrep_blob(SemgrepRuleSet::Health, &health_payload),
        })
    }
}

pub(crate) fn semgrep_support_dir(repo_path: &Path) -> Result<PathBuf, SidecarError> {
    let repo_name = repo_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: format!("Nome inválido para repositório virtualizado '{}'", repo_path.display()),
        })?;
    Ok(
        repo_path
            .parent()
            .unwrap_or(repo_path)
            .join(".soda_semgrep")
            .join(repo_name),
    )
}

fn workspace_semgrep_rules_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("semgrep")
        .join("rules")
}

fn semgrep_bundle_locks() -> &'static Mutex<BTreeMap<String, Arc<AsyncMutex<()>>>> {
    static LOCKS: OnceLock<Mutex<BTreeMap<String, Arc<AsyncMutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn semgrep_bundle_lock_for_support_dir(support_dir: &Path) -> Arc<AsyncMutex<()>> {
    let key = support_dir
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let mut guard = super::lock_unpoisoned(semgrep_bundle_locks());
    guard
        .entry(key)
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}

fn copy_semgrep_rule_tree(source_root: &Path, current: &Path, target_root: &Path) -> Result<usize, SidecarError> {
    if !current.exists() {
        return Ok(0);
    }

    let mut copied = 0_usize;
    let entries = std::fs::read_dir(current).map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao listar árvore de regras '{}': {}", current.display(), e),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao ler entrada da árvore de regras '{}': {}", current.display(), e),
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao inspecionar item '{}' da árvore de regras: {}", path.display(), e),
        })?;

        if file_type.is_dir() {
            copied += copy_semgrep_rule_tree(source_root, &path, target_root)?;
            continue;
        }

        let is_rule = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "yml" | "yaml"))
            .unwrap_or(false);
        if !is_rule {
            continue;
        }

        let relative = path.strip_prefix(source_root).map_err(|e| SidecarError::ExecutionFailed {
            reason: format!(
                "Falha ao relativizar regra '{}' contra '{}': {}",
                path.display(),
                source_root.display(),
                e
            ),
        })?;
        let target = target_root.join(relative);
        if target.exists() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao preparar diretório de regra '{}': {}", parent.display(), e),
            })?;
        }
        match std::fs::copy(&path, &target) {
            Ok(_) => {}
            Err(_) if target.exists() => {
                continue;
            }
            Err(e) => {
                return Err(SidecarError::ExecutionFailed {
                    reason: format!(
                        "Falha ao copiar regra '{}' para '{}': {}",
                        path.display(),
                        target.display(),
                        e
                    ),
                });
            }
        }
        copied += 1;
    }

    Ok(copied)
}

pub async fn ensure_semgrep_rule_bundle(repo_path: &Path, rule_set: SemgrepRuleSet) -> Result<PathBuf, SidecarError> {
    let support_dir = semgrep_support_dir(repo_path)?.join(rule_set.config_dir_name());
    let bundle_lock = semgrep_bundle_lock_for_support_dir(&support_dir);
    let _guard = bundle_lock.lock().await;
    tokio::fs::create_dir_all(&support_dir)
        .await
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao preparar diretório auxiliar do semgrep '{}': {}", support_dir.display(), e),
        })?;
    let built_in_rule_path = support_dir.join(rule_set.rule_file_name());
    tokio::fs::write(&built_in_rule_path, rule_set.rule_source())
        .await
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!(
                "Falha ao materializar regra base do semgrep '{}': {}",
                built_in_rule_path.display(),
                e
            ),
        })?;

    let workspace_rules_dir = workspace_semgrep_rules_dir();
    if rule_set.copies_workspace_rules() && workspace_rules_dir.exists() {
        let copied = copy_semgrep_rule_tree(&workspace_rules_dir, &workspace_rules_dir, &support_dir)?;
        tracing::info!(
            repo_path = %repo_path.display(),
            rule_set = ?rule_set,
            copied_rule_files = copied,
            workspace_rules_dir = %workspace_rules_dir.display(),
            support_dir = %support_dir.display(),
            "Semgrep: ruleset air-gapped materializado"
        );
    } else {
        tracing::info!(
            repo_path = %repo_path.display(),
            rule_set = ?rule_set,
            support_dir = %support_dir.display(),
            "Semgrep: ruleset slim sem workspace rules para evitar slop"
        );
    }

    Ok(support_dir)
}

async fn run_semgrep_scan<E: SandboxExecutor>(
    executor: &E,
    rule_set: SemgrepRuleSet,
    timeout_secs: u64,
) -> Result<Vec<u8>, SidecarError> {
    let rule_path = ensure_semgrep_rule_bundle(executor.repo_path(), rule_set).await?;
    tracing::info!(
        repo_path = %executor.repo_path().display(),
        rule_set = ?rule_set,
        rule_path = %rule_path.display(),
        "Semgrep: iniciando scan"
    );
    let rule_arg = rule_path.to_string_lossy().to_string();
    let args = semgrep_args(&rule_arg);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    execute_sidecar(
        executor,
        "semgrep",
        &arg_refs,
        timeout_secs,
        SidecarExitPolicy::AllowFindingsExitOne,
    )
    .await
}

const SEMGREP_SCAN_EXCLUDES: &[&str] = &[
    ".git",
    "**/.git/**",
    "node_modules",
    "**/node_modules/**",
    "dist",
    "**/dist/**",
    "build",
    "**/build/**",
    "vendor",
    "**/vendor/**",
    "libs",
    "**/libs/**",
    "public/libs",
    "**/public/libs/**",
    "tests",
    "**/tests/**",
    "__tests__",
    "**/__tests__/**",
    "test",
    "**/test/**",
    "spec",
    "**/spec/**",
    "specs",
    "**/specs/**",
    "mock",
    "mocks",
    "__mocks__",
    "**/mocks/**",
    "**/__mocks__/**",
    "fixture",
    "fixtures",
    "__fixtures__",
    "**/fixtures/**",
    "**/__fixtures__/**",
    "snapshot",
    "snapshots",
    "__snapshots__",
    "**/snapshots/**",
    "**/__snapshots__/**",
    "sample",
    "samples",
    "**/samples/**",
    "example",
    "playground",
    "playgrounds",
    "**/playgrounds/**",
    "benchmark",
    "bench",
    "benches",
    "benchmarking",
    "**/benchmarking/**",
    "generated",
    "**/generated/**",
    "**/output.json",
    "**/*.generated.*",
    "**/*.min.js",
    "**/*.iife.js",
    "**/*.umd.js",
    "**/*.min.cjs",
    "**/*.min.mjs",
    "**/*.bundle.js",
    "**/*.pack.js",
    "**/*.vendor.js",
    "test_support",
    "**/test_support/**",
    "e2e",
    "testutil",
    "**/testutil/**",
    "docs",
    "**/docs/**",
    "documentation",
    "examples",
    "**/examples/**",
    "**/*.spec.*",
    "**/*.test.*",
    "**/*test.go",
    "**/*test.rs",
];

#[derive(Debug, Clone, Copy)]
struct SemgrepScanOptions {
    disable_version_check: bool,
    metrics_off: bool,
    taint_intrafile: bool,
    allow_rule_timeout_control: bool,
    exclude_minified_files: bool,
}

fn build_semgrep_like_scan_args(
    rule_arg: &str,
    options: SemgrepScanOptions,
    scan_targets: &[String],
) -> Vec<String> {
    let mut args = vec![
        "scan".to_string(),
        "--config".to_string(),
        rule_arg.to_string(),
        "--json".to_string(),
        "--jobs".to_string(),
        "1".to_string(),
    ];

    if options.disable_version_check {
        args.push("--disable-version-check".to_string());
    }
    if options.metrics_off {
        args.push("--metrics".to_string());
        args.push("off".to_string());
    }
    if options.taint_intrafile {
        args.push("--taint-intrafile".to_string());
    }
    if options.allow_rule_timeout_control {
        args.push("--allow-rule-timeout-control".to_string());
    }
    if options.exclude_minified_files {
        args.push("--exclude-minified-files".to_string());
    }
    args.push("--force-exclude".to_string());

    for exclude in SEMGREP_SCAN_EXCLUDES {
        args.push("--exclude".to_string());
        args.push((*exclude).to_string());
    }

    if scan_targets.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

pub fn opengrep_args(
    rules_path: &str,
    scan_targets: &[String],
    rule_set: SemgrepRuleSet,
) -> Vec<String> {
    let options = SemgrepScanOptions {
        disable_version_check: true,
        metrics_off: false,
        taint_intrafile: rule_set == SemgrepRuleSet::Security,
        allow_rule_timeout_control: true,
        exclude_minified_files: false,
    };
    build_semgrep_like_scan_args(rules_path, options, scan_targets)
}

pub fn semgrep_args(rules_path: &str) -> Vec<String> {
    let options = SemgrepScanOptions {
        disable_version_check: true,
        metrics_off: true,
        taint_intrafile: false,
        allow_rule_timeout_control: true,
        exclude_minified_files: true,
    };
    build_semgrep_like_scan_args(rules_path, options, &[])
}

pub fn normalize_semgrep_payload(
    repo_path: &Path,
    bytes: &[u8],
) -> Result<SemgrepNormalizedPayload, SidecarError> {
    let payload = match parse_json_payload::<serde_json::Value>(bytes) {
        Ok(payload) => payload,
        Err(error) => {
            return Err(SidecarError::ParseError {
                reason: format!("Semgrep output JSON corrompido: {error}"),
            });
        }
    };

    let empty_array = Vec::new();
    let results = payload
        .get("results")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_array);
    let scanned_paths = payload
        .get("paths")
        .and_then(|v| v.get("scanned"))
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);

    let mut blocks_map = BTreeMap::<String, Vec<String>>::new();
    let mut findings_count = 0;

    for result in results {
        let Some(raw_path) = result.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(file_path) = sanitize_repo_relative_path(repo_path, raw_path) else {
            continue;
        };
        let check_id = result
            .get("check_id")
            .and_then(|v| v.as_str())
            .unwrap_or("soda.sast.unknown");
        let line = result
            .get("start")
            .and_then(|v| v.get("line"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let extra = result.get("extra");
        let message = extra
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("sast issue");
        let severity = extra
            .and_then(|v| v.get("severity"))
            .and_then(|v| v.as_str())
            .unwrap_or("warning");
        let category = extra
            .and_then(|v| v.get("metadata"))
            .and_then(|v| v.get("category"))
            .and_then(|v| v.as_str())
            .unwrap_or("general-debt");

        let finding = format!(
            "L{line}: [{check_id}] ({severity} / {category}) -> {message}"
        );
        blocks_map.entry(file_path).or_default().push(finding);
        findings_count += 1;
    }

    let blocks = blocks_map
        .into_iter()
        .map(|(file_path, items)| ScopedTextBlock {
            file_path,
            items,
            omitted_count: 0,
        })
        .collect::<Vec<_>>();

    let files_analyzed = scanned_paths.max(blocks.len());

    Ok(SemgrepNormalizedPayload {
        blocks,
        files_analyzed,
        findings_count,
    })
}

pub fn render_semgrep_blob(
    rule_set: SemgrepRuleSet,
    payload: &SemgrepNormalizedPayload,
) -> Vec<u8> {
    let mut out = String::new();
    match rule_set {
        SemgrepRuleSet::Security => {
            out.push_str("# Unsafe Hotspots\n\n");
            if payload.findings_count == 0 {
                out.push_str("Sem hotspots estaticos relevantes do semgrep.\n");
            } else {
                out.push_str(&render_scoped_text_blocks(&payload.blocks));
            }
        }
        SemgrepRuleSet::Health => {
            out.push_str("# Health Report\n\n");
            if payload.findings_count == 0 {
                out.push_str("Sem divida tecnica estatica relevante do semgrep.\n");
            } else {
                out.push_str(&render_scoped_text_blocks(&payload.blocks));
            }
        }
    }
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::harvester::sast::test_utils::{MockExecutor, test_clean_files};
    use crate::harvester::sast::PolyglotSastSidecar;
    use crate::harvester::sast::PolyglotSastInput;
    use crate::harvester::detect::{StackProfile, SingleStack};
    use crate::harvester::router::StaticAnalysisBlade;
    use crate::harvester::sast::BLOB_04_REPO_OUTLINE_MAX_CHARS;
    use crate::harvester::sandbox::SandboxError;

    #[tokio::test]
    async fn test_semgrep_sidecar_extracts_security_and_health_without_panic() {
        let security_json = r#"{
            "results": [
                {
                    "check_id": "soda.rust.unsafe.block",
                    "path": "src/main.rs",
                    "start": { "line": 12 },
                    "extra": {
                        "message": "Rust unsafe block requer auditoria manual",
                        "severity": "WARNING",
                        "metadata": { "category": "memory-unsafety" }
                    }
                }
            ],
            "paths": { "scanned": ["src/main.rs", "src/lib.rs"] }
        }"#;
        let health_json = r#"{
            "results": [
                {
                    "check_id": "soda.tech-debt.todo-fixme",
                    "path": "src/lib.rs",
                    "start": { "line": 48 },
                    "extra": {
                        "message": "Marcador de divida tecnica encontrado",
                        "severity": "INFO"
                    }
                }
            ],
            "paths": { "scanned": ["src/main.rs", "src/lib.rs"] }
        }"#;
        let executor = MockExecutor::new(vec![
            Ok(security_json.as_bytes().to_vec()),
            Ok(health_json.as_bytes().to_vec()),
        ]);

        let payload = SemgrepSidecar::extract(SemgrepInput {
            executor: &executor,
            timeout_secs: 30,
        })
        .await
        .unwrap();

        let unsafe_blob = String::from_utf8(payload.unsafe_hotspots_blob).unwrap();
        let health_blob = String::from_utf8(payload.health_report_blob).unwrap();

        assert!(executor.calls().iter().all(|call| call.starts_with("semgrep scan")));
        assert!(unsafe_blob.contains("# Unsafe Hotspots"));
        assert!(unsafe_blob.contains("[src/main.rs]"));
        assert!(unsafe_blob.contains("memory-unsafety"));
        assert!(health_blob.contains("# Health Report"));
        assert!(health_blob.contains("[src/lib.rs]"));
        assert!(health_blob.contains("soda.tech-debt.todo-fixme"));
    }

    #[tokio::test]
    async fn test_semgrep_sidecar_accepts_json_stdout_even_with_exit_code_2() {
        let security_json = r#"{
            "results": [
                {
                    "check_id": "soda.rust.unsafe.block",
                    "path": "src/main.rs",
                    "start": { "line": 7 },
                    "extra": {
                        "message": "Rust unsafe block requer auditoria manual",
                        "severity": "WARNING"
                    }
                }
            ],
            "paths": { "scanned": ["src/main.rs"] }
        }"#;
        let health_json = r#"{
            "results": [],
            "paths": { "scanned": ["src/main.rs"] }
        }"#;

        let executor = MockExecutor::new(vec![
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 2,
                stderr: String::new(),
                stdout: security_json.as_bytes().to_vec(),
            }),
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 2,
                stderr: String::new(),
                stdout: health_json.as_bytes().to_vec(),
            }),
        ]);

        let payload = SemgrepSidecar::extract(SemgrepInput {
            executor: &executor,
            timeout_secs: 30,
        })
        .await
        .unwrap();

        let unsafe_blob = String::from_utf8(payload.unsafe_hotspots_blob).unwrap();
        let health_blob = String::from_utf8(payload.health_report_blob).unwrap();

        assert!(unsafe_blob.contains("[src/main.rs]"));
        assert!(health_blob.contains("Sem divida tecnica estatica relevante do semgrep"));
    }

    #[test]
    fn test_render_semgrep_security_blob_keeps_long_tail_without_truncation() {
        let long_tail = "RISK".repeat(BLOB_04_REPO_OUTLINE_MAX_CHARS);
        let payload = SemgrepNormalizedPayload {
            blocks: vec![ScopedTextBlock {
                file_path: "src/main.ts".to_string(),
                items: vec![format!("tail-marker-{long_tail}")],
                omitted_count: 0,
            }],
            files_analyzed: 1,
            findings_count: 1,
        };

        let rendered = String::from_utf8(render_semgrep_blob(SemgrepRuleSet::Security, &payload)).unwrap();

        assert!(rendered.contains("tail-marker-"));
        assert!(rendered.ends_with(&long_tail));
        assert!(rendered.len() > BLOB_04_REPO_OUTLINE_MAX_CHARS);
    }

    #[test]
    fn test_render_semgrep_health_blob_keeps_long_tail_without_truncation() {
        let long_tail = "FLOW".repeat(BLOB_04_REPO_OUTLINE_MAX_CHARS);
        let payload = SemgrepNormalizedPayload {
            blocks: vec![ScopedTextBlock {
                file_path: "src/entropy.ts".to_string(),
                items: vec![format!("tail-marker-{long_tail}")],
                omitted_count: 0,
            }],
            files_analyzed: 1,
            findings_count: 1,
        };

        let rendered = String::from_utf8(render_semgrep_blob(SemgrepRuleSet::Health, &payload)).unwrap();

        assert!(rendered.contains("tail-marker-"));
        assert!(rendered.ends_with(&long_tail));
        assert!(rendered.len() > BLOB_04_REPO_OUTLINE_MAX_CHARS);
    }

    #[test]
    fn test_normalize_semgrep_payload_keeps_only_repo_relative_paths() {
        let repo_path = Path::new(r"C:\host\projfs\owner\repo");
        let payload = format!(
            r#"{{
                "results": [
                    {{
                        "check_id": "soda.rust.unsafe.block",
                        "path": "{repo}\\src\\main.rs",
                        "start": {{ "line": 12 }},
                        "extra": {{
                            "message": "unsafe em {repo}\\src\\main.rs",
                            "severity": "WARNING"
                        }}
                    }},
                    {{
                        "check_id": "soda.support.noise",
                        "path": "C:\\host\\projfs\\owner\\.soda_semgrep\\repo\\sandbox\\.semgrep\\settings.yml",
                        "start": {{ "line": 1 }},
                        "extra": {{
                            "message": "noise",
                            "severity": "INFO"
                        }}
                    }}
                ],
                "paths": {{ "scanned": ["{repo}\\src\\main.rs"] }}
            }}"#,
            repo = r"C:\\host\\projfs\\owner\\repo"
        );

        let normalized = normalize_semgrep_payload(repo_path, payload.as_bytes()).unwrap();
        assert_eq!(normalized.blocks.len(), 1);
        assert_eq!(normalized.blocks[0].file_path, "src/main.rs");
        assert!(!normalized.blocks[0].items[0].contains("C:/host"));
    }

    #[test]
    fn test_opengrep_args_use_runtime_compatible_flags_and_test_excludes() {
        let args = opengrep_args(
            "C:/rules",
            &["src/main.ts".to_string(), "src/lib.ts".to_string()],
            SemgrepRuleSet::Security,
        );
        assert!(args.iter().any(|arg| arg == "--allow-rule-timeout-control"));
        assert!(args.iter().any(|arg| arg == "--force-exclude"));
        assert!(args.iter().any(|arg| arg == "--taint-intrafile"));
        assert!(!args.iter().any(|arg| arg == "--exclude-minified-files"));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", ".git"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "node_modules"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "dist"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "build"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "vendor"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "tests"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "testutil"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/examples/**"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/docs/**"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/mocks/**"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/*.min.js"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/*.iife.js"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/samples/**"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/output.json"]));
        assert!(!args.windows(2).any(|pair| pair == ["--exclude", "C:/rules"]));
        assert!(!args.windows(2).any(|pair| pair == ["--exclude", SEMGREP_SECURITY_RULE_FILE]));
        assert!(!args.windows(2).any(|pair| pair == ["--exclude", SEMGREP_HEALTH_RULE_FILE]));
        assert!(!args.iter().any(|arg| arg.contains("Cargo.lock")));
        assert!(!args.iter().any(|arg| arg.contains("package-lock.json")));
        assert!(args.ends_with(&["src/main.ts".to_string(), "src/lib.ts".to_string()]));
    }

    #[test]
    fn test_opengrep_health_args_disable_security_taint_profile() {
        let args = opengrep_args("C:/rules", &["src/main.ts".to_string()], SemgrepRuleSet::Health);
        assert!(!args.iter().any(|arg| arg == "--taint-intrafile"));
        assert!(args.iter().any(|arg| arg == "--force-exclude"));
    }

    #[test]
    fn test_semgrep_args_share_common_sast_performance_flags() {
        let args = semgrep_args("C:/rules");
        assert!(args.iter().any(|arg| arg == "--allow-rule-timeout-control"));
        assert!(args.iter().any(|arg| arg == "--exclude-minified-files"));
        assert!(args.iter().any(|arg| arg == "--disable-version-check"));
        assert!(args.windows(2).any(|pair| pair == ["--metrics", "off"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "tests"]));
        assert!(!args.iter().any(|arg| arg == "--taint-intrafile"));
    }

    #[tokio::test]
    async fn test_ensure_semgrep_rule_bundle_is_idempotent_under_concurrency() {
        let executor = MockExecutor::new(vec![]);
        let (left, right) = tokio::join!(
            ensure_semgrep_rule_bundle(executor.repo_path(), SemgrepRuleSet::Health),
            ensure_semgrep_rule_bundle(executor.repo_path(), SemgrepRuleSet::Health)
        );

        let left = left.unwrap();
        let right = right.unwrap();
        assert_eq!(left, right);
        assert!(left.is_absolute());
        assert!(left.join(SEMGREP_HEALTH_RULE_FILE).exists());

        let workspace_rule = workspace_semgrep_rules_dir().join("soda-golden-patterns.yaml");
        if workspace_rule.exists() {
            assert!(!left.join("soda-golden-patterns.yaml").exists());
        }
    }

    #[test]
    fn test_normalize_opengrep_findings_separates_red_lines_from_flow_debt() {
        let repo_path = Path::new(r"C:\host\projfs\owner\repo");
        let payload = r#"{
            "results": [
                {
                    "check_id": "soda.javascript.dynamic-eval",
                    "path": "src/main.ts",
                    "start": { "line": 4 },
                    "extra": {
                        "message": "Execucao dinamica via eval aumenta risco de injecao",
                        "severity": "ERROR"
                    }
                },
                {
                    "check_id": "soda.javascript.nested-ternary",
                    "path": "src/ui.ts",
                    "start": { "line": 9 },
                    "extra": {
                        "message": "Ternario aninhado sugere alta complexidade de fluxo",
                        "severity": "WARNING"
                    }
                }
            ]
        }"#;

        let normalized = crate::harvester::sast::normalize_sast_output(
            repo_path,
            repo_path,
            StaticAnalysisBlade::Opengrep,
            payload.as_bytes(),
        )
        .unwrap();

        assert_eq!(normalized.len(), 2);
        assert!(normalized.iter().any(|issue| {
            issue.file == "src/main.ts"
                && issue.message.contains("dynamic-eval")
                && crate::harvester::sast::is_unsafe_hotspot(issue)
        }));
        assert!(normalized.iter().any(|issue| {
            issue.file == "src/ui.ts"
                && issue.message.contains("nested-ternary")
                && !crate::harvester::sast::is_unsafe_hotspot(issue)
        }));
    }

    #[tokio::test]
    async fn test_polyglot_sast_sidecar_routes_rust_and_cpp_and_breaks_blob06_from_blob08() {
        let clippy_payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"manual memcpy can be replaced with copy_from_slice","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}"#;
        let cppcheck_payload = r#"<results><errors><error id="memleak" severity="warning" msg="Memory leak: ptr"><location file="native/bridge.cpp" line="42"/></error></errors></results>"#;
        let opengrep_payload = r#"{"results":[{"check_id":"soda.tech-debt.todo-fixme","path":"README.md","extra":{"message":"Marcador de divida tecnica encontrado","severity":"INFO"}}]}"#;

        let executor = Arc::new(MockExecutor::new(vec![]));
        executor.write_repo_file("Cargo.toml", "[package]\nname='repo'\nversion='0.1.0'\n");

        let metadata_payload = serde_json::json!({
            "packages": [
                {
                    "manifest_path": executor.repo_path().join("Cargo.toml").display().to_string()
                }
            ]
        })
        .to_string();

        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Ok(Vec::new()), // consumed by cargo fetch in preflight
            Ok(metadata_payload.as_bytes().to_vec()), // consumed by cargo metadata in preflight
            Err(SandboxError::ProcessNonZeroExit { // consumed by cargo clippy
                exit_code: 1,
                stderr: "findings".to_string(),
                stdout: clippy_payload.as_bytes().to_vec(),
            }),
            Err(SandboxError::ProcessNonZeroExit { // consumed by cppcheck
                exit_code: 1,
                stderr: cppcheck_payload.to_string(),
                stdout: Vec::new(),
            }),
            Ok(br#"{"results":[]}"#.to_vec()), // consumed by opengrep .::unsafe
            Ok(opengrep_payload.as_bytes().to_vec()), // consumed by opengrep .::health
        ]);

        executor.write_repo_file("native/bridge.cpp", "void foo() {}");

        let artifacts = PolyglotSastSidecar::extract(PolyglotSastInput {
            executor: Arc::clone(&executor),
            timeout_secs: 60,
            profile: &StackProfile::Mixed(vec![SingleStack::Rust, SingleStack::CCpp]),
            clean_files: test_clean_files(executor.repo_path(), &["Cargo.toml", "native/bridge.cpp"]),
        })
        .await
        .unwrap();

        let unsafe_blob = String::from_utf8(artifacts.unsafe_hotspots_blob).unwrap();
        let health_blob = String::from_utf8(artifacts.health_report_blob).unwrap();

        assert!(executor.calls().iter().any(|call| {
            call.starts_with("cargo clippy")
                && call.contains("-p repo")
                && call.contains("-- --no-deps")
        }));
        assert!(executor.calls().iter().any(|call| call.starts_with("cppcheck ")));
        assert!(executor.calls().iter().any(|call| {
            call.starts_with("opengrep scan --config")
                && call.contains("--json")
                && call.contains("--disable-version-check")
                && call.contains("--taint-intrafile")
                && call.contains("[cwd=")
        }));
        assert!(unsafe_blob.contains("# Unsafe Hotspots"));
        assert!(unsafe_blob.contains("[DOMAIN: C++ / CUDA]"));
        assert!(unsafe_blob.contains("native/bridge.cpp"));
        assert!(unsafe_blob.contains("[cppcheck]"));
        assert!(!unsafe_blob.contains("\"issues\""));
        assert!(!unsafe_blob.contains("src/lib.rs"));
        assert!(!unsafe_blob.contains("README.md"));
        assert!(health_blob.contains("# Health Report"));
        assert!(health_blob.contains("[DOMAIN: RUST]"));
        assert!(health_blob.contains("[rust-clippy]"));
        assert!(health_blob.contains("src/lib.rs"));
        assert!(health_blob.contains("[opengrep]"));
        assert!(health_blob.contains("README.md"));
        assert!(!health_blob.contains("execution failed"));
        assert!(!health_blob.contains("normalization failed"));
        assert!(!health_blob.contains("\"router\""));
        assert!(!health_blob.contains("\"schema\""));
    }

    #[tokio::test]
    async fn test_polyglot_sast_sidecar_returns_zero_byte_when_all_scanners_fail() {
        let executor = Arc::new(MockExecutor::new(vec![
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 2,
                stderr: "fatal clippy failure".to_string(),
                stdout: Vec::new(),
            }),
            Err(SandboxError::Timeout),
        ]));
        executor.write_repo_file("Cargo.toml", "[package]\nname='repo'\nversion='0.1.0'\n");

        let artifacts = PolyglotSastSidecar::extract(PolyglotSastInput {
            executor: Arc::clone(&executor),
            timeout_secs: 60,
            profile: &StackProfile::Rust,
            clean_files: test_clean_files(executor.repo_path(), &["Cargo.toml"]),
        })
        .await
        .unwrap();

        assert!(artifacts.unsafe_hotspots_blob.is_empty());
        assert!(artifacts.health_report_blob.is_empty());
    }

    #[tokio::test]
    async fn test_polyglot_sast_sidecar_executes_rust_subprojects_with_scoped_cwd() {
        let clippy_payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"lint in workspace member","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}"#;
        let executor = Arc::new(MockExecutor::new(vec![]));
        executor.write_repo_file("apps/rust-sdk/Cargo.toml", "[package]\nname='sdk'\nversion='0.1.0'\n");

        let metadata_payload = serde_json::json!({
            "packages": [
                {
                    "manifest_path": executor.repo_path().join("apps/rust-sdk/Cargo.toml").display().to_string()
                }
            ]
        })
        .to_string();

        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Ok(Vec::new()), // consumed by cargo fetch in preflight
            Ok(metadata_payload.as_bytes().to_vec()), // consumed by cargo metadata in preflight
            Err(SandboxError::ProcessNonZeroExit { // consumed by cargo clippy
                exit_code: 1,
                stderr: "findings".to_string(),
                stdout: clippy_payload.as_bytes().to_vec(),
            }),
            Ok(br#"{"results":[]}"#.to_vec()), // consumed by opengrep if any
        ]);

        let artifacts = PolyglotSastSidecar::extract(PolyglotSastInput {
            executor: Arc::clone(&executor),
            timeout_secs: 60,
            profile: &StackProfile::Rust,
            clean_files: test_clean_files(executor.repo_path(), &["apps/rust-sdk/Cargo.toml"]),
        })
        .await
        .unwrap();
        let health_blob = String::from_utf8(artifacts.health_report_blob).unwrap();

        assert!(executor.calls().iter().any(|call| {
            call.starts_with("cargo clippy")
                && call.contains("-p sdk")
                && call.contains("-- --no-deps")
                && (call.contains("apps\\rust-sdk") || call.contains("apps/rust-sdk"))
        }));
        assert!(health_blob.contains("apps/rust-sdk/src/lib.rs"));
        assert!(health_blob.contains("[DOMAIN: RUST]"));
        assert!(health_blob.contains("[rust-clippy]"));
        assert!(!health_blob.contains("\"scope\""));
    }
}
