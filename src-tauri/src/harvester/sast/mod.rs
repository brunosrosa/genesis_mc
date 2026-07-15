use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use ignore::WalkBuilder;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{error, info, warn};

use crate::harvester::ast_parser;
use crate::harvester::detect::StackProfile;
use crate::harvester::router::{route_static_analysis_blades, StaticAnalysisBlade};
use crate::harvester::sandbox::{SandboxError, truncated_args_preview};

pub mod test_utils;
pub mod native_ast;
pub mod oxc;
pub mod test_discovery;
pub mod opengrep;
pub mod clippy;
pub mod biome;
pub mod cppcheck;
pub mod govulncheck;
pub mod ruff;
pub mod bandit;
pub mod sobelow;

// Public re-exports for external callers
pub use native_ast::{NativeAstInput, NativeAstArtifacts, NativeAstParser};
pub use oxc::{PropDeclaration, ComponentContract, UxContractsPayload, OxcInput, OxcSidecar};
pub use test_discovery::{TestIntentPayload, NativeTestDiscoveryInput, NativeTestDiscoverySidecar};
pub use opengrep::{SemgrepInput, SemgrepSidecar, SemgrepRuleSet};

pub(crate) const MONOREPO_SAST_MAX_PARALLEL: usize = 4;
pub(crate) const RUST_CLIPPY_MAX_PARALLEL: usize = 1;
pub(crate) const OPENGREP_FILE_LIST_CHUNK_SIZE: usize = 96;
pub(crate) const CPPCHECK_FILE_LIST_CHUNK_SIZE: usize = 96;
pub(crate) const JS_LINT_FILE_LIST_CHUNK_SIZE: usize = 96;
pub(crate) const PYTHON_LINT_FILE_LIST_CHUNK_SIZE: usize = 96;
pub(crate) const BLOB_04_REPO_OUTLINE_MAX_CHARS: usize = 3_000_000;

pub(crate) fn cached_regex<'a>(cache: &'a OnceLock<Option<Regex>>, pattern: &str) -> Option<&'a Regex> {
    cache.get_or_init(|| Regex::new(pattern).ok()).as_ref()
}

pub(crate) fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Trait para abstrair a execução no sandbox, permitindo mocks nos testes.
pub trait SandboxExecutor {
    fn execute<'a>(
        &'a self,
        command: &'a str,
        args: &'a [&'a str],
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>>;
    fn execute_in_dir<'a>(
        &'a self,
        command: &'a str,
        args: &'a [&'a str],
        timeout_secs: u64,
        execution_root: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>>;
    fn repo_path(&self) -> &Path;
}

impl SandboxExecutor for crate::harvester::sandbox::SandboxHandle {
    fn execute<'a>(
        &'a self,
        command: &'a str,
        args: &'a [&'a str],
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>> {
        Box::pin(async move {
            crate::harvester::sandbox::SandboxHandle::execute(self, command, args, timeout_secs)
                .await
        })
    }

    fn execute_in_dir<'a>(
        &'a self,
        command: &'a str,
        args: &'a [&'a str],
        timeout_secs: u64,
        execution_root: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>> {
        Box::pin(async move {
            crate::harvester::sandbox::SandboxHandle::execute_in_dir(
                self,
                command,
                args,
                timeout_secs,
                execution_root,
            )
            .await
        })
    }

    fn repo_path(&self) -> &Path {
        self.repo_path()
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SidecarError {
    #[error("Sidecar binary not found: {binary}")]
    BinaryNotFound { binary: String },

    #[error("Execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("Execution timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("Failed to parse sidecar output: {reason}")]
    ParseError { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarExitPolicy {
    StrictZeroOnly,
    AllowFindingsExitOne,
    AllowNonZeroWithStdout,
}

pub(crate) fn stdout_is_blank(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| byte.is_ascii_whitespace())
}

fn stdout_preview(bytes: &[u8], max_chars: usize) -> String {
    if stdout_is_blank(bytes) {
        return String::new();
    }
    let text = String::from_utf8_lossy(bytes);
    truncate_chars(&text.replace(['\r', '\n'], " "), max_chars)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidecarObservabilityClass {
    Ok,
    InformationalNonZero,
    LethalNonZero,
}

fn classify_sidecar_observability(exit_code: i32, stdout: &[u8]) -> SidecarObservabilityClass {
    if exit_code == 0 {
        SidecarObservabilityClass::Ok
    } else if !stdout.is_empty() {
        SidecarObservabilityClass::InformationalNonZero
    } else {
        SidecarObservabilityClass::LethalNonZero
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsLintProfile {
    UnsafeHotspot,
    Health,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DomainTag {
    Rust,
    CppCuda,
    ObjectiveCMetal,
    JavascriptTypescript,
    Python,
    Go,
    Elixir,
    Other,
}

impl DomainTag {
    fn label(self) -> &'static str {
        match self {
            Self::Rust => "RUST",
            Self::CppCuda => "C++ / CUDA",
            Self::ObjectiveCMetal => "OBJECTIVE-C / METAL",
            Self::JavascriptTypescript => "JAVASCRIPT / TYPESCRIPT",
            Self::Python => "PYTHON",
            Self::Go => "GO",
            Self::Elixir => "ELIXIR",
            Self::Other => "OTHER",
        }
    }
}

const DOMAIN_SECTION_DIVIDER: &str =
    "=================================================================";

fn classify_domain_from_path(value: &str) -> DomainTag {
    let normalized = value.trim().replace('\\', "/").to_ascii_lowercase();
    if normalized.is_empty() {
        return DomainTag::Other;
    }

    let extension = Path::new(value)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    let has_any_marker = |markers: &[&str]| markers.iter().any(|marker| normalized.contains(marker));

    if has_any_marker(&["/candle-metal-kernels/", "/metal/", "objc", "objc2", "core-ml"])
        || matches!(extension.as_deref(), Some("m" | "mm" | "metal"))
    {
        return DomainTag::ObjectiveCMetal;
    }

    if has_any_marker(&["/cuda/", "/candle-kernels/", "cudarc", "cuda", "kernel"])
        || matches!(
            extension.as_deref(),
            Some("c" | "cc" | "cpp" | "cxx" | "cu" | "cuh" | "h" | "hh" | "hpp" | "hxx")
        )
    {
        return DomainTag::CppCuda;
    }

    match extension.as_deref() {
        Some("rs") => DomainTag::Rust,
        Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts") => {
            DomainTag::JavascriptTypescript
        }
        Some("py") => DomainTag::Python,
        Some("go") => DomainTag::Go,
        Some("ex" | "exs") => DomainTag::Elixir,
        _ => DomainTag::Other,
    }
}

fn classify_issue_domain(issue: &SodaHealthIssue) -> DomainTag {
    let from_file = classify_domain_from_path(&issue.file);
    if from_file != DomainTag::Other {
        return from_file;
    }

    let blade = issue.source_blade.to_ascii_lowercase();
    if blade.contains("clippy") {
        DomainTag::Rust
    } else if blade.contains("cppcheck") {
        DomainTag::CppCuda
    } else {
        DomainTag::Other
    }
}

fn productive_domains_from_clean_files(clean_files: &[PathBuf]) -> Vec<DomainTag> {
    let mut domains = clean_files
        .iter()
        .map(|path| classify_domain_from_path(&path.to_string_lossy()))
        .filter(|domain| *domain != DomainTag::Other)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if domains.is_empty() {
        domains.push(DomainTag::Other);
    }
    domains
}

fn merge_domain_inventory(
    clean_files: &[PathBuf],
    grouped: &BTreeMap<DomainTag, Vec<&SodaHealthIssue>>,
) -> Vec<DomainTag> {
    let mut domains = productive_domains_from_clean_files(clean_files)
        .into_iter()
        .collect::<BTreeSet<_>>();
    domains.extend(grouped.keys().copied());
    domains.into_iter().collect()
}

fn render_domain_header(domain: DomainTag) -> String {
    format!(
        "{divider}\n[DOMAIN: {label}]\n{divider}",
        divider = DOMAIN_SECTION_DIVIDER,
        label = domain.label()
    )
}

fn format_scoped_text_block(block: &ScopedTextBlock) -> String {
    let mut lines = vec![format!("[{}]", block.file_path)];
    for item in &block.items {
        lines.push(format!("- {}", item));
    }
    if block.omitted_count > 0 {
        lines.push(format!("- ... [{} itens omitidos]", block.omitted_count));
    }
    lines.join("\n")
}

pub(crate) fn render_scoped_text_blocks(blocks: &[ScopedTextBlock]) -> String {
    let mut grouped = BTreeMap::<DomainTag, Vec<String>>::new();
    for block in blocks {
        let domain = classify_domain_from_path(&block.file_path);
        grouped
            .entry(domain)
            .or_default()
            .push(format_scoped_text_block(block));
    }

    grouped
        .into_iter()
        .map(|(domain, entries)| {
            format!("{}\n{}", render_domain_header(domain), entries.join("\n\n"))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn render_scoped_text_block_refs(blocks: &[&ScopedTextBlock]) -> String {
    let mut grouped = BTreeMap::<DomainTag, Vec<String>>::new();
    for block in blocks {
        let domain = classify_domain_from_path(&block.file_path);
        grouped
            .entry(domain)
            .or_default()
            .push(format_scoped_text_block(block));
    }

    grouped
        .into_iter()
        .map(|(domain, entries)| {
            format!("{}\n{}", render_domain_header(domain), entries.join("\n\n"))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn pack_scoped_text_blocks(blocks: &[ScopedTextBlock], max_chars: usize) -> String {
    let mut packed = String::new();

    for block in blocks {
        let section = format_scoped_text_block(block);
        let candidate_len = if packed.is_empty() {
            section.chars().count()
        } else {
            packed.chars().count() + 2 + section.chars().count()
        };

        if candidate_len >= max_chars {
            break;
        }

        if !packed.is_empty() {
            packed.push_str("\n\n");
        }
        packed.push_str(&section);
    }

    packed
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScopedTextBlock {
    pub file_path: String,
    pub items: Vec<String>,
    pub omitted_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum SastIssueChannel {
    UnsafeHotspot,
    Health,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SodaHealthIssue {
    pub level: String,
    pub file: String,
    pub message: String,
    pub source_blade: String,
    pub channel: SastIssueChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestKind {
    CargoToml,
    PackageJson,
    MixExs,
    GoMod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredManifest {
    pub kind: ManifestKind,
    pub manifest_path: PathBuf,
    pub scope: String,
    pub execution_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SastExecutionTarget {
    pub blade: StaticAnalysisBlade,
    pub execution_root: PathBuf,
    pub scope: String,
    pub scan_targets: Vec<String>,
    pub command_args: Option<Vec<String>>,
    pub forced_channel: Option<SastIssueChannel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyglotSastArtifacts {
    pub unsafe_hotspots_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemgrepArtifacts {
    pub unsafe_hotspots_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
}

pub struct PolyglotSastInput<'a, E: SandboxExecutor> {
    pub executor: Arc<E>,
    pub timeout_secs: u64,
    pub profile: &'a StackProfile,
    pub clean_files: Arc<Vec<PathBuf>>,
}

pub struct PolyglotSastSidecar;

struct SastExecutionOutcome {
    requested_blade: StaticAnalysisBlade,
    effective_blade: StaticAnalysisBlade,
    execution_root: PathBuf,
    scope: String,
    forced_channel: Option<SastIssueChannel>,
    result: Result<Vec<u8>, SidecarError>,
    /// PRD-033: payload forense capturado quando a lâmina RustClippy falha
    /// em vez de abortar o pipeline inteiro.
    forensic_rust_diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SastBladeResult {
    pub effective_blade: StaticAnalysisBlade,
    pub bytes: Vec<u8>,
}

pub fn blade_name(blade: StaticAnalysisBlade) -> &'static str {
    match blade {
        StaticAnalysisBlade::RustClippy => "rust-clippy",
        StaticAnalysisBlade::Cppcheck => "cppcheck",
        StaticAnalysisBlade::Sobelow => "sobelow",
        StaticAnalysisBlade::Biome => "biome",
        StaticAnalysisBlade::Oxc => "oxlint",
        StaticAnalysisBlade::Ruff => "ruff",
        StaticAnalysisBlade::Bandit => "bandit",
        StaticAnalysisBlade::Govulncheck => "govulncheck",
        StaticAnalysisBlade::Opengrep => "opengrep",
    }
}

pub fn blade_command(
    blade: StaticAnalysisBlade,
    scan_targets: &[String],
    command_args: Option<&[String]>,
) -> (&'static str, Vec<String>) {
    if let Some(args) = command_args {
        return (
            match blade {
                StaticAnalysisBlade::RustClippy => "cargo",
                StaticAnalysisBlade::Cppcheck => "cppcheck",
                StaticAnalysisBlade::Sobelow => "mix",
                StaticAnalysisBlade::Biome => "biome",
                StaticAnalysisBlade::Oxc => "oxlint",
                StaticAnalysisBlade::Ruff => "ruff",
                StaticAnalysisBlade::Bandit => "bandit",
                StaticAnalysisBlade::Govulncheck => "govulncheck",
                StaticAnalysisBlade::Opengrep => "opengrep",
            },
            args.to_vec(),
        );
    }
    match blade {
        StaticAnalysisBlade::RustClippy => ("cargo", clippy::default_clippy_args()),
        StaticAnalysisBlade::Cppcheck => ("cppcheck", cppcheck::cppcheck_args(scan_targets)),
        StaticAnalysisBlade::Sobelow => ("mix", sobelow::sobelow_args_for_root(".")),
        StaticAnalysisBlade::Biome => ("biome", biome::biome_args(scan_targets)),
        StaticAnalysisBlade::Oxc => ("oxlint", oxc::oxc_args(scan_targets)),
        StaticAnalysisBlade::Ruff => ("ruff", ruff::ruff_args(scan_targets)),
        StaticAnalysisBlade::Bandit => ("bandit", bandit::bandit_args(scan_targets)),
        StaticAnalysisBlade::Govulncheck => ("govulncheck", govulncheck::govulncheck_args_for_module()),
        StaticAnalysisBlade::Opengrep => ("opengrep", Vec::new()),
    }
}

pub fn blade_parallelism_limit(blade: StaticAnalysisBlade) -> usize {
    match blade {
        StaticAnalysisBlade::RustClippy => RUST_CLIPPY_MAX_PARALLEL,
        _ => MONOREPO_SAST_MAX_PARALLEL,
    }
}

pub fn has_global_opengrep_coverage(targets: &[SastExecutionTarget]) -> bool {
    targets.iter().any(|target| {
        target.blade == StaticAnalysisBlade::Opengrep
            && (target.scope == "."
                || target.scope.starts_with(".::files-")
                || target.scope.starts_with(".::unsafe::files-")
                || target.scope.starts_with(".::health::files-"))
    })
}

impl PolyglotSastSidecar {
    pub async fn extract<E: SandboxExecutor + Send + Sync + 'static>(
        input: PolyglotSastInput<'_, E>,
    ) -> Result<PolyglotSastArtifacts, SidecarError> {
        let blades = route_static_analysis_blades(input.profile);
        let repo_path = input.executor.repo_path().to_path_buf();
        let manifests = discover_monorepo_manifests(&repo_path);
        let manifest_summary = manifests
            .iter()
            .map(|manifest| format!("{}:{}", manifest.scope, manifest.manifest_path.display()))
            .collect::<Vec<_>>();
        info!(
            repo_path = %repo_path.display(),
            manifest_count = manifests.len(),
            manifests = ?manifest_summary,
            concurrency_limit = MONOREPO_SAST_MAX_PARALLEL,
            "SAST monorepo: manifestos detectados"
        );

        let mut all_issues = Vec::<SodaHealthIssue>::new();
        let mut had_successful_payload = false;
        let mut had_failed_payload = false;
        let global_semaphore = Arc::new(Semaphore::new(MONOREPO_SAST_MAX_PARALLEL));
        let cargo_semaphore = Arc::new(Semaphore::new(RUST_CLIPPY_MAX_PARALLEL));
        let global_opengrep_targets = if blades.contains(&StaticAnalysisBlade::Opengrep) {
            execution_targets_for_blade(
                &repo_path,
                &input.clean_files,
                &manifests,
                StaticAnalysisBlade::Opengrep,
            )
        } else {
            Vec::new()
        };
        let has_global_opengrep_coverage = has_global_opengrep_coverage(&global_opengrep_targets);
        let mut join_set = JoinSet::new();

        for blade in &blades {
            let targets =
                execution_targets_for_blade(&repo_path, &input.clean_files, &manifests, *blade);
            if targets.is_empty() {
                let reason = format!(
                    "nenhum manifesto compatível foi encontrado para {}",
                    blade_name(*blade)
                );
                warn!(
                    blade = blade_name(*blade),
                    repo_path = %repo_path.display(),
                    reason = %reason,
                    "SAST monorepo: lâmina sem manifesto compatível"
                );
                continue;
            }

            for target in targets {
                let executor = Arc::clone(&input.executor);
                let global_semaphore = Arc::clone(&global_semaphore);
                let cargo_semaphore = Arc::clone(&cargo_semaphore);
                let blade_parallelism = blade_parallelism_limit(target.blade);
                join_set.spawn(async move {
                    let SastExecutionTarget {
                        blade,
                        execution_root,
                        scope,
                        scan_targets,
                        command_args,
                        forced_channel,
                    } = target;
                    let _cargo_permit = if blade == StaticAnalysisBlade::RustClippy {
                        Some(
                            Arc::clone(&cargo_semaphore)
                                .acquire_owned()
                                .await
                                .map_err(|e| SidecarError::ExecutionFailed {
                                    reason: format!(
                                        "falha ao adquirir permissão serial do cargo-clippy: {e}"
                                    ),
                                })?,
                        )
                    } else {
                        None
                    };
                    let _global_permit = Arc::clone(&global_semaphore)
                        .acquire_owned()
                        .await
                        .map_err(|e| SidecarError::ExecutionFailed {
                            reason: format!("falha ao adquirir permissão do semáforo SAST: {e}"),
                        })?;
                    info!(
                        blade = blade_name(blade),
                        scope = %scope,
                        cwd = %execution_root.display(),
                        concurrency_limit = blade_parallelism,
                        global_in_flight = MONOREPO_SAST_MAX_PARALLEL
                            .saturating_sub(global_semaphore.available_permits()),
                        cargo_in_flight = RUST_CLIPPY_MAX_PARALLEL
                            .saturating_sub(cargo_semaphore.available_permits()),
                        "SAST monorepo: permissão adquirida"
                    );
                    let result = run_sast_blade(
                        executor.as_ref(),
                        blade,
                        input.timeout_secs,
                        &execution_root,
                        &scope,
                        &scan_targets,
                        command_args.as_deref(),
                        forced_channel,
                        has_global_opengrep_coverage,
                    )
                    .await;
                    info!(
                        blade = blade_name(blade),
                        scope = %scope,
                        cwd = %execution_root.display(),
                        available_global_permits = global_semaphore.available_permits(),
                        available_cargo_permits = cargo_semaphore.available_permits(),
                        "SAST monorepo: sub-scan concluído"
                    );
                    // Intercepta qualquer erro na lâmina e o transforma em diagnóstico forense
                    // sem interromper as outras lâminas.
                    let forensic_blade_diagnostic = if let Err(ref err) = result {
                        let stderr_limpo = match err {
                            SidecarError::ExecutionFailed { reason } => reason.clone(),
                            SidecarError::Timeout { timeout_secs } => {
                                format!("timeout após {timeout_secs}s")
                            }
                            other => other.to_string(),
                        };
                        if blade == StaticAnalysisBlade::RustClippy {
                            Some(format!(
                                "[DIAGNÓSTICO ESTRUTURAL RUST: FALHA FATAL DE COMPILAÇÃO OU RCE BLOQUEADO] -> {}",
                                stderr_limpo
                            ))
                        } else {
                            Some(format!(
                                "[DIAGNÓSTICO ESTRUTURAL: Lâmina '{}' ignorada por violação/ausência] -> {}",
                                blade_name(blade), stderr_limpo
                            ))
                        }
                    } else {
                        None
                    };
                    // Se o diagnóstico forense foi capturado, substituir o Err
                    // por Ok(SastBladeResult vazio) para que a lâmina não aborte o pipeline.
                    let result: Result<SastBladeResult, SidecarError> = if forensic_blade_diagnostic.is_some() {
                        Ok(SastBladeResult {
                            effective_blade: blade,
                            bytes: Vec::new(),
                        })
                    } else {
                        result
                    };
                    let (effective_blade, result) = match result {
                        Ok(result) => (result.effective_blade, Ok(result.bytes)),
                        Err(err) => (blade, Err(err)),
                    };
                    Ok::<SastExecutionOutcome, SidecarError>(SastExecutionOutcome {
                        requested_blade: blade,
                        effective_blade,
                        execution_root,
                        scope,
                        forced_channel,
                        result,
                        forensic_rust_diagnostic: forensic_blade_diagnostic,
                    })
                });
            }
        }

        let mut forensic_blade_diagnostics: Vec<String> = Vec::new();
        while let Some(joined) = join_set.join_next().await {
            let outcome = match joined {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(err)) => {
                    error!(
                        repo_path = %repo_path.display(),
                        error = %err,
                        "SAST monorepo: worker falhou de forma fatal"
                    );
                    return Err(err);
                }
                Err(err) => {
                    error!(
                        repo_path = %repo_path.display(),
                        error = %err,
                        "SAST monorepo: join do worker falhou de forma fatal"
                    );
                    return Err(SidecarError::ExecutionFailed {
                        reason: format!("Join error no worker da lâmina SAST: {}", err),
                    });
                }
            };

            // PRD-033: acumular diagnóstico forense (não aborta pipeline)
            if let Some(diag) = outcome.forensic_rust_diagnostic {
                warn!(
                    scope = %outcome.scope,
                    "SAST monorepo: falha forense da lâmina capturada"
                );
                forensic_blade_diagnostics.push(diag);
                had_failed_payload = true;
                continue;
            }

            match outcome.result {
                Ok(bytes) => match normalize_sast_output(
                    &repo_path,
                    &outcome.execution_root,
                    outcome.effective_blade,
                    &bytes,
                ) {
                    Ok(mut issues) => {
                        if let Some(forced_channel) = outcome.forced_channel {
                            for issue in &mut issues {
                                issue.channel = forced_channel;
                            }
                        }
                        if issues.is_empty() {
                            let blade_label = blade_name(outcome.effective_blade);
                            issues.push(SodaHealthIssue {
                                level: "info".to_string(),
                                file: String::new(),
                                message: format!("[INFO] Nenhuma vulnerabilidade ou pendência encontrada pela lâmina '{}' no escopo '{}'.", blade_label, outcome.scope),
                                source_blade: blade_label.to_string(),
                                channel: SastIssueChannel::Health,
                            });
                        }
                        had_successful_payload = true;
                        all_issues.append(&mut issues);
                    }
                    Err(err) => {
                        had_failed_payload = true;
                        let blade_label = blade_name(outcome.effective_blade);
                        let error_msg = format!("Lâmina '{}' falhou na normalização dos resultados no escopo '{}': {}", blade_label, outcome.scope, err);
                        all_issues.push(SodaHealthIssue {
                            level: "warning".to_string(),
                            file: String::new(),
                            message: format!("[FALHA_NORMALIZACAO] {}", error_msg),
                            source_blade: blade_label.to_string(),
                            channel: SastIssueChannel::Health,
                        });
                        had_successful_payload = true;
                        warn!(
                            blade = blade_name(outcome.effective_blade),
                            requested_blade = blade_name(outcome.requested_blade),
                            scope = %outcome.scope,
                            cwd = %outcome.execution_root.display(),
                            error = %err,
                            "SAST monorepo: normalizacao falhou; descartando payload bruto"
                        );
                    }
                },
                Err(err) => {
                    let blade_label = blade_name(outcome.requested_blade);
                    error!(
                        blade = blade_label,
                        scope = %outcome.scope,
                        cwd = %outcome.execution_root.display(),
                        error = %err,
                        "SAST monorepo: execucao da lamina falhou com erro letal (Fail-Closed)"
                    );
                    return Err(err);
                }
            }
        }

        // PRD-033: só retorna zero-byte se não há NENHUM sinal (nem issues, nem forense)
        if had_failed_payload && !had_successful_payload && forensic_blade_diagnostics.is_empty() {
            error!(
                repo_path = %repo_path.display(),
                "SAST monorepo: todas as laminas falharam; retornando blobs zero-byte"
            );
            return Ok(PolyglotSastArtifacts {
                unsafe_hotspots_blob: Vec::new(),
                health_report_blob: Vec::new(),
            });
        }

        sort_and_dedup_issues(&mut all_issues);
        let unsafe_issues = all_issues
            .iter()
            .filter(|issue| is_unsafe_hotspot(issue))
            .filter(|issue| should_keep_blob06_issue(issue))
            .cloned()
            .collect::<Vec<_>>();
        let health_issues = all_issues
            .iter()
            .filter(|issue| !is_unsafe_hotspot(issue))
            .filter(|issue| should_keep_blob08_issue(issue))
            .cloned()
            .collect::<Vec<_>>();

        let health_report_body = render_soda_health_report(&health_issues);

        // PRD-033: prepend dos diagnósticos forenses ao TOPO do Blob 08
        let health_report_blob = if forensic_blade_diagnostics.is_empty() {
            health_report_body
        } else {
            let header = forensic_blade_diagnostics.join("\n");
            let mut blob = header.into_bytes();
            if !health_report_body.is_empty() {
                blob.push(b'\n');
                blob.extend_from_slice(&health_report_body);
            }
            blob
        };

        Ok(PolyglotSastArtifacts {
            unsafe_hotspots_blob: render_unsafe_hotspots_report(&unsafe_issues, &input.clean_files),
            health_report_blob,
        })
    }
}

pub(crate) fn is_biome_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts")
}

pub(crate) fn is_oxlint_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" | "svelte")
}

pub(crate) fn is_cpp_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx"
    )
}

pub(crate) fn derive_js_lint_file_list_targets(
    execution_root: &Path,
    scope_prefix: &str,
    clean_files: &[PathBuf],
    boundary_roots: &[&Path],
    blade: StaticAnalysisBlade,
) -> Vec<SastExecutionTarget> {
    let scan_targets = derive_repo_relative_clean_targets(execution_root, clean_files, boundary_roots, |path| {
        if blade == StaticAnalysisBlade::Biome {
            return is_biome_supported_file(path);
        }
        if blade == StaticAnalysisBlade::Oxc {
            return is_oxlint_supported_file(path);
        }
        false
    });

    if scan_targets.is_empty() {
        return Vec::new();
    }

    let normalized_scope = if scope_prefix.trim().is_empty() {
        ".".to_string()
    } else {
        scope_prefix.to_string()
    };

    let mut targets = Vec::new();
    for profile in [JsLintProfile::UnsafeHotspot, JsLintProfile::Health] {
        let profile_scope = format!(
            "{}::{}",
            normalized_scope,
            match profile {
                JsLintProfile::UnsafeHotspot => "unsafe",
                JsLintProfile::Health => "health",
            }
        );
        for (idx, chunk) in scan_targets.chunks(JS_LINT_FILE_LIST_CHUNK_SIZE).enumerate() {
            let chunk_targets = chunk.to_vec();
            let command_args = match blade {
                StaticAnalysisBlade::Biome => biome::biome_args_for_profile(&chunk_targets, profile),
                StaticAnalysisBlade::Oxc => oxc::oxc_args_for_profile(&chunk_targets, profile),
                _ => Vec::new(),
            };
            targets.push(SastExecutionTarget {
                blade,
                execution_root: execution_root.to_path_buf(),
                scope: blade_file_batch_scope(&profile_scope, idx + 1),
                scan_targets: chunk_targets,
                command_args: Some(command_args),
                forced_channel: Some(match profile {
                    JsLintProfile::UnsafeHotspot => SastIssueChannel::UnsafeHotspot,
                    JsLintProfile::Health => SastIssueChannel::Health,
                }),
            });
        }
    }
    targets
}

pub(crate) fn derive_js_lint_execution_targets(
    repo_path: &Path,
    manifests: &[DiscoveredManifest],
    blade: StaticAnalysisBlade,
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let kind = ManifestKind::PackageJson;
    let package_manifests = manifests
        .iter()
        .filter(|manifest| manifest.kind == kind)
        .collect::<Vec<_>>();

    let mut targets = Vec::new();
    if package_manifests.is_empty() {
        targets.extend(derive_js_lint_file_list_targets(
            &repo_root,
            ".",
            clean_files,
            &[],
            blade,
        ));
        return targets;
    }

    for manifest in package_manifests {
        let boundaries = descendant_roots_for_manifest(manifests, &manifest.execution_root, kind);
        targets.extend(derive_js_lint_file_list_targets(
            &manifest.execution_root,
            &manifest.scope,
            clean_files,
            &boundaries,
            blade,
        ));
    }
    targets.sort_by(|left, right| {
        left.scope
            .cmp(&right.scope)
            .then_with(|| left.execution_root.cmp(&right.execution_root))
            .then_with(|| left.scan_targets.cmp(&right.scan_targets))
    });
    targets.dedup_by(|left, right| {
        left.execution_root == right.execution_root
            && left.scope == right.scope
            && left.scan_targets == right.scan_targets
    });
    targets
}

pub(crate) fn group_files_by_manifest(
    repo_root: &Path,
    manifests: &[DiscoveredManifest],
    clean_files: &[PathBuf],
) -> BTreeMap<PathBuf, (String, Vec<PathBuf>)> {
    let mut groups: BTreeMap<PathBuf, (String, Vec<PathBuf>)> = BTreeMap::new();

    for file in clean_files {
        let abs_file = if file.is_absolute() {
            file.clone()
        } else {
            repo_root.join(file)
        };
        let abs_file_clean = abs_file.canonicalize().unwrap_or(abs_file);

        let mut closest_manifest: Option<&DiscoveredManifest> = None;
        for manifest in manifests {
            let manifest_root_clean = manifest.execution_root.canonicalize().unwrap_or_else(|_| manifest.execution_root.clone());
            if abs_file_clean.starts_with(&manifest_root_clean) {
                if let Some(current) = closest_manifest {
                    let current_root_clean = current.execution_root.canonicalize().unwrap_or_else(|_| current.execution_root.clone());
                    if manifest_root_clean.as_os_str().len() > current_root_clean.as_os_str().len() {
                        closest_manifest = Some(manifest);
                    }
                } else {
                    closest_manifest = Some(manifest);
                }
            }
        }

        if let Some(manifest) = closest_manifest {
            let manifest_root_clean = manifest.execution_root.canonicalize().unwrap_or_else(|_| manifest.execution_root.clone());
            let entry = groups.entry(manifest_root_clean).or_insert_with(|| (manifest.scope.clone(), Vec::new()));
            entry.1.push(abs_file_clean);
        } else {
            let entry = groups.entry(repo_root.to_path_buf()).or_insert_with(|| (".".to_string(), Vec::new()));
            entry.1.push(abs_file_clean);
        }
    }

    groups
}

pub(crate) fn derive_opengrep_execution_targets(
    repo_path: &Path,
    manifests: &[DiscoveredManifest],
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    
    let groups = group_files_by_manifest(&repo_root, manifests, clean_files);
    let mut targets = Vec::new();

    for (execution_root, (scope, files)) in groups {
        let scan_targets = derive_repo_relative_clean_targets(&execution_root, &files, &[], |_| true);
        if scan_targets.is_empty() {
            continue;
        }

        let normalized_scope = if scope.trim().is_empty() {
            ".".to_string()
        } else {
            scope.clone()
        };

        for (profile_scope, forced_channel) in [
            (format!("{normalized_scope}::unsafe"), SastIssueChannel::UnsafeHotspot),
            (format!("{normalized_scope}::health"), SastIssueChannel::Health),
        ] {
            for (idx, chunk) in scan_targets.chunks(OPENGREP_FILE_LIST_CHUNK_SIZE).enumerate() {
                targets.push(SastExecutionTarget {
                    blade: StaticAnalysisBlade::Opengrep,
                    execution_root: execution_root.clone(),
                    scope: blade_file_batch_scope(&profile_scope, idx + 1),
                    scan_targets: chunk.to_vec(),
                    command_args: None,
                    forced_channel: Some(forced_channel),
                });
            }
        }
    }
    targets
}

fn execution_targets_for_blade(
    repo_path: &Path,
    clean_files: &[PathBuf],
    manifests: &[DiscoveredManifest],
    blade: StaticAnalysisBlade,
) -> Vec<SastExecutionTarget> {
    if blade == StaticAnalysisBlade::Opengrep {
        return derive_opengrep_execution_targets(repo_path, manifests, clean_files);
    }
    if blade == StaticAnalysisBlade::Cppcheck {
        return cppcheck::derive_cppcheck_execution_targets(repo_path, clean_files);
    }
    if blade == StaticAnalysisBlade::RustClippy {
        return clippy::derive_rust_clippy_execution_targets(manifests);
    }
    if blade == StaticAnalysisBlade::Govulncheck {
        return govulncheck::derive_go_execution_targets(manifests, clean_files);
    }
    if blade == StaticAnalysisBlade::Sobelow {
        return sobelow::derive_elixir_execution_targets(manifests, clean_files);
    }
    if blade == StaticAnalysisBlade::Biome || blade == StaticAnalysisBlade::Oxc {
        return derive_js_lint_execution_targets(repo_path, manifests, blade, clean_files);
    }
    if let Some(kind) = manifest_kind_for_blade(blade) {
        return manifests
            .iter()
            .filter(|manifest| manifest.kind == kind)
            .map(|manifest| SastExecutionTarget {
                blade,
                execution_root: manifest.execution_root.clone(),
                scope: manifest.scope.clone(),
                scan_targets: vec![".".to_string()],
                command_args: None,
                forced_channel: None,
            })
            .collect();
    }

    vec![SastExecutionTarget {
        blade,
        execution_root: repo_path.to_path_buf(),
        scope: ".".to_string(),
        scan_targets: vec![".".to_string()],
        command_args: None,
        forced_channel: None,
    }]
}

fn manifest_kind_for_blade(blade: StaticAnalysisBlade) -> Option<ManifestKind> {
    match blade {
        StaticAnalysisBlade::RustClippy => Some(ManifestKind::CargoToml),
        StaticAnalysisBlade::Biome | StaticAnalysisBlade::Oxc => Some(ManifestKind::PackageJson),
        StaticAnalysisBlade::Sobelow => Some(ManifestKind::MixExs),
        StaticAnalysisBlade::Govulncheck => Some(ManifestKind::GoMod),
        _ => None,
    }
}

pub(crate) fn blade_file_batch_scope(scope: &str, batch_idx: usize) -> String {
    format!("{scope}::files-{batch_idx:02}")
}

pub(crate) fn descendant_roots_for_manifest<'a>(
    manifests: &'a [DiscoveredManifest],
    execution_root: &Path,
    kind: ManifestKind,
) -> Vec<&'a Path> {
    manifests
        .iter()
        .filter(|manifest| manifest.kind == kind && manifest.execution_root != execution_root)
        .map(|manifest| manifest.execution_root.as_path())
        .filter(|root| root.starts_with(execution_root))
        .collect()
}

pub(crate) fn derive_repo_relative_clean_targets(
    execution_root: &Path,
    clean_files: &[PathBuf],
    boundary_roots: &[&Path],
    predicate: impl Fn(&Path) -> bool,
) -> Vec<String> {
    let normalized_root = execution_root
        .canonicalize()
        .unwrap_or_else(|_| execution_root.to_path_buf());
    let mut out = Vec::new();
    for path in clean_files {
        if !path.starts_with(&normalized_root) {
            continue;
        }
        if boundary_roots.iter().any(|boundary| path.starts_with(boundary)) {
            continue;
        }
        if !predicate(path) {
            continue;
        }
        let Some(rel) = path.strip_prefix(&normalized_root).ok() else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if rel.is_empty() {
            continue;
        }
        if should_skip_sast_relative_target(&rel) {
            continue;
        }
        out.push(rel);
    }
    out.sort();
    out.dedup();
    out
}

pub(crate) fn should_skip_sast_relative_target(rel: &str) -> bool {
    if ast_parser::should_skip_architecture_relative_path(rel) {
        return true;
    }

    let normalized = rel.replace('\\', "/").to_ascii_lowercase();
    let segments = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let has_test_like_segment = segments.iter().any(|segment| {
        matches!(
            *segment,
            "test"
                | "tests"
                | "__tests__"
                | "testutil"
                | "vendor"
                | "libs"
                | "spec"
                | "specs"
                | "integration"
                | "e2e"
                | "mock"
                | "mocks"
                | "__mocks__"
                | "fixture"
                | "fixtures"
                | "__fixtures__"
                | "snapshot"
                | "snapshots"
                | "__snapshots__"
                | "sample"
                | "samples"
                | "playground"
                | "playgrounds"
                | "benchmark"
                | "benchmarks"
                | "benchmarking"
                | "docs"
                | "documentation"
                | "examples"
                | "example"
        )
    });
    if has_test_like_segment {
        return true;
    }

    if segments.windows(2).any(|pair| pair == ["public", "libs"]) {
        return true;
    }

    let file_name = segments.last().copied().unwrap_or_default();
    file_name.contains(".spec.")
        || file_name.contains(".test.")
        || file_name.ends_with("test.go")
        || file_name.ends_with("test.rs")
        || file_name.contains(".min.")
        || file_name.contains(".iife.")
        || file_name.contains(".umd.")
        || file_name.contains(".bundle.")
        || file_name.contains(".pack.")
        || file_name.contains(".vendor.")
}

pub(crate) fn is_go_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("go"))
        .unwrap_or(false)
}

fn collapse_inline_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_issue_level(level: &str) -> String {
    let lower = level.trim().to_ascii_lowercase();
    match lower.as_str() {
        "error" | "critical" | "high" => "error".to_string(),
        "warning" | "warn" | "medium" => "warning".to_string(),
        _ => "info".to_string(),
    }
}

fn normalize_relative_issue_file(repo_path: &Path, execution_root: &Path, value: &str) -> String {
    let raw = value.trim().trim_matches('"');
    if raw.is_empty() {
        return String::new();
    }

    let candidate = Path::new(raw);
    if candidate.is_absolute() {
        return sanitize_repo_relative_path(repo_path, raw)
            .unwrap_or_else(|| sanitize_host_paths_in_text(repo_path, raw).replace('\\', "/"));
    }

    let mut joined = PathBuf::new();
    if let Ok(relative_root) = execution_root.strip_prefix(repo_path) {
        joined.push(relative_root);
    }
    for component in candidate.components() {
        match component {
            Component::Normal(value) => joined.push(value),
            Component::ParentDir => {
                joined.pop();
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    joined.to_string_lossy().replace('\\', "/")
}

fn sanitize_issue_file(repo_path: &Path, execution_root: &Path, value: &str) -> String {
    normalize_relative_issue_file(repo_path, execution_root, value)
}

fn sanitize_issue_message(repo_path: &Path, value: &str) -> String {
    collapse_inline_whitespace(&sanitize_host_paths_in_text(repo_path, value))
}

fn classify_sast_issue(blade: StaticAnalysisBlade, level: &str, message: &str) -> SastIssueChannel {
    let normalized = message.to_ascii_lowercase();
    let is_health_debt = [
        "soda.tech-debt",
        "soda.flow-debt",
        "soda.golden-pattern",
        "soda.fragility",
        "nested-ternary",
        "ternario aninhado",
        "complexidade",
        "ciclomat",
        "todo",
        "fixme",
        "hack",
        "xxx",
        "console.log",
        "console.warn",
        "console.error",
        "unwrap",
        "expect",
        "panic",
        "copy_from_slice",
        "style",
        "performance",
        "portability",
        "manual memcpy",
        "boolean chain",
        "debug residual",
        "unused variable",
        "unused import",
        "unused assignment",
        "unused mut",
        "unused result",
        "dead code",
        "unreachable code",
        "cognitive complexity",
        "cyclomatic complexity",
        "too many branches",
        "too many arguments",
        "too many statements",
        "too many lines",
        "function is too complex",
        "function is too long",
        "long method",
        "monolithic function",
        "debugger",
    ]
    .iter()
    .any(|needle| normalized.contains(needle));
    if is_health_debt {
        return SastIssueChannel::Health;
    }

    let has_red_flag_keyword = [
        "cve-",
        "osv-",
        "go-20",
        "vulnerability",
        "vulnerabilidade",
        "hardcoded secret",
        "hardcoded password",
        "hardcoded token",
        "hardcoded credential",
        "segredo hardcoded",
        "api key",
        "aws_access_key",
        "command injection",
        "os command injection",
        "sql injection",
        "code injection",
        "remote code execution",
        "path traversal",
        "insecure deserialization",
        "unsafe deserialization",
        "deserialization",
        "execucao dinamica",
        " eval",
        "eval(",
        "exec(",
        "shell=true",
        "unsafe block",
        "memory-unsafety",
        "memory unsafety",
        "raw pointer",
        "ponteiro cru",
        "pointer arithmetic",
        "null pointer",
        "dangling pointer",
        "double free",
        "use-after-free",
        "use after free",
        "buffer overflow",
        "stack overflow",
        "heap overflow",
        "out-of-bounds",
        "out of bounds",
        "secret",
        "password",
        "token",
        "credential",
        "xss",
        "innerhtml",
        "dangerouslysetinnerhtml",
        "pickle",
        "yaml.load",
    ]
    .iter()
    .any(|needle| normalized.contains(needle));

    if matches!(blade, StaticAnalysisBlade::Govulncheck | StaticAnalysisBlade::Sobelow) {
        return SastIssueChannel::UnsafeHotspot;
    }

    if blade == StaticAnalysisBlade::Bandit && level != "info" {
        return SastIssueChannel::UnsafeHotspot;
    }

    if blade == StaticAnalysisBlade::Cppcheck {
        let has_memory_danger = [
            "memory leak",
            "memleak",
            "buffer",
            "overflow",
            "null pointer",
            "dangling",
            "double free",
            "use after free",
            "use-after-free",
            "invalid free",
            "pointer",
        ]
        .iter()
        .any(|needle| normalized.contains(needle));
        return if has_memory_danger {
            SastIssueChannel::UnsafeHotspot
        } else {
            SastIssueChannel::Health
        };
    }

    if has_red_flag_keyword {
        return SastIssueChannel::UnsafeHotspot;
    }

    SastIssueChannel::Health
}

pub(crate) fn push_issue(
    issues: &mut Vec<SodaHealthIssue>,
    repo_path: &Path,
    execution_root: &Path,
    blade: StaticAnalysisBlade,
    level: &str,
    file: &str,
    message: &str,
) {
    let message = sanitize_issue_message(repo_path, message);
    if message.trim().is_empty() {
        return;
    }
    if should_drop_sast_issue(blade, level, &message) {
        return;
    }
    let channel = classify_sast_issue(blade, level, &message);
    issues.push(SodaHealthIssue {
        level: sanitize_issue_level(level),
        file: sanitize_issue_file(repo_path, execution_root, file),
        message,
        source_blade: blade_name(blade).to_string(),
        channel,
    });
}

pub(crate) fn should_drop_sast_issue(blade: StaticAnalysisBlade, level: &str, message: &str) -> bool {
    is_aesthetic_or_minor_warning(blade, message)
        || is_blob_06_semantic_slop(blade, level, message)
}

pub(crate) fn is_aesthetic_or_minor_warning(blade: StaticAnalysisBlade, message: &str) -> bool {
    if !matches!(
        blade,
        StaticAnalysisBlade::Ruff
            | StaticAnalysisBlade::Bandit
            | StaticAnalysisBlade::Biome
            | StaticAnalysisBlade::Oxc
    ) {
        return false;
    }

    let normalized = message.to_ascii_lowercase();
    [
        "use of assert detected",
        "docstring",
        "alt text",
        "alternative text",
        "aria",
        "unused",
        "never used",
        "escape character",
        "f-string",
        "image",
        "picture",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

pub(crate) fn is_blob_06_semantic_slop(blade: StaticAnalysisBlade, level: &str, message: &str) -> bool {
    if !matches!(
        blade,
        StaticAnalysisBlade::Cppcheck | StaticAnalysisBlade::Biome | StaticAnalysisBlade::Oxc
    ) {
        return false;
    }

    let normalized_level = level.to_ascii_lowercase();
    let normalized = message.to_ascii_lowercase();
    let preserve_signal = normalized_level.contains("error")
        || [
            "error",
            "security",
            "unsafe",
            "leak",
            "injection",
            "timeout",
            "vulnerability",
            "vulnerabilidade",
            "hardcoded",
            "secret",
            "password",
            "token",
            "credential",
            "overflow",
            "double free",
            "use-after-free",
            "use after free",
            "null pointer",
            "dangling",
        ]
        .iter()
        .any(|needle| normalized.contains(needle));

    if preserve_signal {
        return false;
    }

    [
        "[info]",
        " style ",
        "style:",
        "import specifier",
        "never used",
        "can be declared as",
        "can be const",
        "dependency",
        "could not find or open",
        "could not resolve",
        "cannot resolve",
        "unresolved import",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn matches_blob06_allowlist(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    [
        "security",
        "unsafe",
        "dangerouslysetinnerhtml",
        "injection",
        "leak",
        "vulnerability",
        "hardcoded",
        "password",
        "secret",
        "credential",
        "overflow",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

pub(crate) fn should_keep_blob06_issue(issue: &SodaHealthIssue) -> bool {
    if issue.channel != SastIssueChannel::UnsafeHotspot {
        return true;
    }

    if !issue.source_blade.eq_ignore_ascii_case("biome")
        && !issue.source_blade.eq_ignore_ascii_case("cppcheck")
    {
        return true;
    }

    matches_blob06_allowlist(&issue.message)
}

fn matches_blob08_allowlist(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    [
        "complexity",
        "cognitive",
        "cyclomatic",
        "panic",
        "unwrap",
        "expect",
        "todo",
        "fixme",
        "temp-dir",
        "deprecated",
        "debt",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

pub(crate) fn should_keep_blob08_issue(issue: &SodaHealthIssue) -> bool {
    if issue.channel != SastIssueChannel::Health {
        return true;
    }

    if !issue.source_blade.eq_ignore_ascii_case("biome")
        && !issue.source_blade.eq_ignore_ascii_case("oxc")
        && !issue.source_blade.eq_ignore_ascii_case("opengrep")
    {
        return true;
    }

    matches_blob08_allowlist(&issue.message)
}

pub(crate) fn sort_and_dedup_issues(issues: &mut Vec<SodaHealthIssue>) {
    issues.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then_with(|| left.level.cmp(&right.level))
            .then_with(|| left.message.cmp(&right.message))
    });
    issues.dedup();
}

pub(crate) fn looks_like_repo_outline_path(value: &str) -> bool {
    let normalized = value.trim().trim_start_matches("- ").trim();
    if normalized.is_empty() {
        return false;
    }

    normalized.contains('/')
        || normalized.ends_with(".rs")
        || normalized.ends_with(".ts")
        || normalized.ends_with(".tsx")
        || normalized.ends_with(".js")
        || normalized.ends_with(".jsx")
        || normalized.ends_with(".py")
        || normalized.ends_with(".go")
        || normalized.ends_with(".java")
        || normalized.ends_with(".kt")
        || normalized.ends_with(".swift")
}

pub(crate) fn truncate_chars(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
}

pub(crate) fn sanitize_host_paths_in_text(repo_path: &Path, text: &str) -> String {
    let mut sanitized = text.to_string();
    let repo_prefix = repo_path.to_string_lossy().to_string();
    sanitized = replace_host_prefix_variants(sanitized, &repo_prefix, "");

    if let Ok(semgrep_root) = opengrep::semgrep_support_dir(repo_path) {
        sanitized = replace_host_prefix_variants(
            sanitized,
            &semgrep_root.to_string_lossy(),
            ".soda_semgrep/",
        );
    }

    sanitized = replace_host_prefix_variants(
        sanitized,
        &native_ast::native_ast_cache_path_for_repo(repo_path),
        ".native_ast_cache/",
    );

    sanitized
}

fn replace_host_prefix_variants(mut text: String, prefix: &str, replacement: &str) -> String {
    if prefix.is_empty() {
        return text;
    }

    let raw = prefix.to_string();
    let slash = raw.replace('\\', "/");
    let escaped = raw.replace('\\', "\\\\");
    let escaped_slash = slash.replace('/', "\\/");
    let mut variants = vec![
        format!("{raw}\\"),
        format!("{raw}/"),
        raw,
        format!("{slash}/"),
        slash,
        format!("{escaped}\\\\"),
        escaped,
        format!("{escaped_slash}\\/"),
        escaped_slash,
    ];
    variants.sort();
    variants.dedup();
    variants.sort_by_key(|value| std::cmp::Reverse(value.len()));

    for variant in variants {
        text = text.replace(&variant, replacement);
    }

    text
}

pub(crate) fn sanitize_repo_relative_path(repo_path: &Path, value: &str) -> Option<String> {
    let sanitized = sanitize_host_paths_in_text(repo_path, value);
    let mut normalized = sanitized
        .trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string();
    if let (Some(owner), Some(repo)) = (
        repo_path.parent().and_then(|path| path.file_name()).and_then(|value| value.to_str()),
        repo_path.file_name().and_then(|value| value.to_str()),
    ) {
        let repo_anchor = format!("repos/{owner}/{repo}/").to_ascii_lowercase();
        let normalized_lower = normalized.to_ascii_lowercase();
        if let Some(index) = normalized_lower.find(&repo_anchor) {
            normalized = normalized[index + repo_anchor.len()..].to_string();
        }
    }
    if normalized.is_empty() {
        return None;
    }

    let lower = normalized.to_ascii_lowercase();
    let host_drive = lower.as_bytes().get(1) == Some(&b':');
    let internal = lower.starts_with(".soda_semgrep/")
        || lower.starts_with(".native_ast_cache/")
        || lower.starts_with(".soda_scratchpad/")
        || lower.starts_with("sandbox/")
        || lower.starts_with("diagnostics/")
        || lower.contains(".souls_workspaces");
    if host_drive || internal {
        return None;
    }

    Some(normalized)
}

pub(crate) fn sanitize_sidecar_output(repo_path: &Path, bytes: &[u8]) -> Vec<u8> {
    sanitize_host_paths_in_text(repo_path, &String::from_utf8_lossy(bytes)).into_bytes()
}

pub(crate) fn is_unsafe_hotspot(issue: &SodaHealthIssue) -> bool {
    issue.channel == SastIssueChannel::UnsafeHotspot
}

pub(crate) fn render_unsafe_hotspots_report(issues: &[SodaHealthIssue], clean_files: &[PathBuf]) -> Vec<u8> {
    let mut text = String::from("# Unsafe Hotspots\n");
    text.push_str(&format!("\nsummary: findings={}", issues.len()));

    let mut grouped = BTreeMap::<DomainTag, Vec<&SodaHealthIssue>>::new();
    for issue in issues {
        let domain = classify_issue_domain(issue);
        grouped.entry(domain).or_default().push(issue);
    }

    text.push_str("\n\n");
    let mut first_domain = true;
    for domain in merge_domain_inventory(clean_files, &grouped) {
        if !first_domain {
            text.push_str("\n\n");
        }
        first_domain = false;
        text.push_str(&render_domain_header(domain));
        text.push('\n');
        if let Some(domain_issues) = grouped.get(&domain) {
            for issue in domain_issues {
                text.push_str("- [");
                text.push_str(&issue.level);
                text.push_str("] [");
                text.push_str(&issue.source_blade);
                text.push_str("] ");
                if !issue.file.trim().is_empty() {
                    text.push_str(&issue.file);
                    text.push_str(" :: ");
                }
                text.push_str(&issue.message);
                text.push('\n');
            }
        } else {
            text.push_str("- clean: Sem linhas vermelhas estaticas relevantes.\n");
        }
    }
    text.into_bytes()
}

pub(crate) fn render_soda_health_report(issues: &[SodaHealthIssue]) -> Vec<u8> {
    let mut text = String::from("# Health Report\n");
    text.push_str(&format!("\nsummary: findings={}", issues.len()));

    if issues.is_empty() {
        text.push_str("\n\nSem divida tecnica estatica relevante.");
        return text.into_bytes();
    }

    let mut grouped = BTreeMap::<DomainTag, Vec<&SodaHealthIssue>>::new();
    for issue in issues {
        let domain = classify_issue_domain(issue);
        grouped.entry(domain).or_default().push(issue);
    }

    text.push_str("\n\n");
    let mut first_domain = true;
    for (domain, domain_issues) in grouped {
        if !first_domain {
            text.push_str("\n\n");
        }
        first_domain = false;
        text.push_str(&render_domain_header(domain));
        text.push('\n');
        for issue in domain_issues {
            text.push_str("- [");
            text.push_str(&issue.level);
            text.push_str("] [");
            text.push_str(&issue.source_blade);
            text.push_str("] ");
            if !issue.file.trim().is_empty() {
                text.push_str(&issue.file);
                text.push_str(" :: ");
            }
            text.push_str(&issue.message);
            text.push('\n');
        }
    }
    text.into_bytes()
}

pub(crate) async fn execute_sidecar<E: SandboxExecutor>(
    executor: &E,
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
    exit_policy: SidecarExitPolicy,
) -> Result<Vec<u8>, SidecarError> {
    execute_sidecar_in_dir(
        executor,
        binary,
        args,
        timeout_secs,
        exit_policy,
        executor.repo_path(),
    )
    .await
}

pub(crate) async fn execute_sidecar_in_dir<E: SandboxExecutor>(
    executor: &E,
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
    exit_policy: SidecarExitPolicy,
    execution_root: &Path,
) -> Result<Vec<u8>, SidecarError> {
    tracing::debug!(
        binary = %binary,
        args = ?truncated_args_preview(args),
        repo_path = %executor.repo_path().display(),
        cwd = %execution_root.display(),
        timeout_secs,
        "Invocando sidecar"
    );
    match executor
        .execute_in_dir(binary, args, timeout_secs, execution_root)
        .await
    {
        Ok(bytes) => {
            let sanitized_bytes = sanitize_sidecar_output(executor.repo_path(), &bytes);
            tracing::debug!(
                binary = %binary,
                stdout_bytes = sanitized_bytes.len(),
                repo_path = %executor.repo_path().display(),
                cwd = %execution_root.display(),
                "Sidecar concluido"
            );
            Ok(sanitized_bytes)
        }
        Err(SandboxError::Timeout) => {
            Err(SidecarError::Timeout { timeout_secs })
        }
        Err(SandboxError::ProcessSpawnFailed { reason }) => {
            error!(binary = %binary, reason = %reason, "Falha ao iniciar sidecar");
            let lower_reason = reason.to_lowercase();
            if lower_reason.contains("not found") || lower_reason.contains("os error 2") {
                Err(SidecarError::BinaryNotFound {
                    binary: binary.to_string(),
                })
            } else {
                Err(SidecarError::ExecutionFailed { reason })
            }
        }
        Err(SandboxError::ProcessNonZeroExit { exit_code, stderr, stdout }) => {
            let sanitized_stdout = sanitize_sidecar_output(executor.repo_path(), &stdout);
            let sanitized_stderr = sanitize_host_paths_in_text(executor.repo_path(), &stderr);
            // L14: Fail-Soft para lâminas SAST. Se stdout contém dados (payload de achados),
            // o processo é considerado sucesso mesmo com exit code 1 ou 7.
            // Isso trata biome, ruff, opengrep e outras ferramentas que retornam exit_code=1
            // quando encontram vulnerabilidades.
            let has_findings_payload = !stdout_is_blank(&sanitized_stdout);
            let is_sast_tool_with_findings = matches!(binary, "biome" | "ruff" | "bandit" | "opengrep" | "semgrep" | "cppcheck" | "oxlint" | "sobelow" | "govulncheck");
            let is_informational_exit = exit_code == 1 || exit_code == 7;
            let should_capture_payload = has_findings_payload && is_sast_tool_with_findings && is_informational_exit;
            
            if ((binary == "semgrep" || binary == "opengrep")
                && !stdout_is_blank(&sanitized_stdout)
                && stdout_contains_json_payload(&sanitized_stdout))
                || (exit_code == 1 && matches!(exit_policy, SidecarExitPolicy::AllowFindingsExitOne) && (!stdout_is_blank(&sanitized_stdout) || (binary == "cppcheck" && !sanitized_stderr.is_empty())))
                || (binary == "opengrep" && exit_code == 7)
                || should_capture_payload
            {
                if binary == "cppcheck" {
                    let mut merged = sanitized_stderr.into_bytes();
                    if !stdout_is_blank(&sanitized_stdout) {
                        if !merged.is_empty() {
                            merged.push(b'\n');
                        }
                        merged.extend_from_slice(&sanitized_stdout);
                    }
                    if merged.is_empty() {
                        Ok(sanitized_stdout)
                    } else {
                        Ok(merged)
                    }
                } else if (is_sobelow_mix_invocation(binary, args)
                    || (binary == "opengrep" && exit_code == 7))
                    && stdout_is_blank(&sanitized_stdout)
                    && !sanitized_stderr.trim().is_empty()
                {
                    Ok(sanitized_stderr.into_bytes())
                } else {
                    Ok(sanitized_stdout)
                }
            } else if binary == "ruff" && exit_code == 2 {
                let stdout_hint = stdout_preview(&sanitized_stdout, 400);
                warn!(
                    binary = %binary,
                    exit_code,
                    stderr = %sanitized_stderr,
                    stdout = %stdout_hint,
                    semantic_outcome = "informational_non_zero",
                    "Ruff falhou devido a erro de configuracao (Fail-Soft)"
                );
                Ok(b"[]".to_vec())
            } else {
                let stdout_hint = stdout_preview(&sanitized_stdout, 400);
                if classify_sidecar_observability(exit_code, &sanitized_stdout)
                    == SidecarObservabilityClass::InformationalNonZero
                {
                    warn!(
                        binary = %binary,
                        exit_code,
                        stderr = %sanitized_stderr,
                        stdout = %stdout_hint,
                        semantic_outcome = "informational_non_zero",
                        "Sidecar terminou com exit code nao zero"
                    );
                } else {
                    error!(
                        binary = %binary,
                        exit_code,
                        stderr = %sanitized_stderr,
                        stdout = %stdout_hint,
                        semantic_outcome = "lethal_non_zero",
                        "Sidecar terminou com exit code nao zero"
                    );
                }
                let reason = if sanitized_stderr.trim().is_empty() && !stdout_hint.trim().is_empty() {
                    format!("exit code {exit_code}: stdout={stdout_hint}")
                } else {
                    format!("exit code {exit_code}: {sanitized_stderr}")
                };
                Err(SidecarError::ExecutionFailed {
                    reason,
                })
            }
        }
        Err(e) => {
            Err(SidecarError::ExecutionFailed {
                reason: e.to_string(),
            })
        }
    }
}

pub(crate) fn parse_json_payload<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SidecarError> {
    let payload = extract_json_payload(bytes).ok_or_else(|| SidecarError::ParseError {
        reason: "Falha ao localizar payload JSON no stdout do sidecar".to_string(),
    })?;
    serde_json::from_slice::<T>(payload).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })
}

fn stdout_contains_json_payload(bytes: &[u8]) -> bool {
    extract_json_payload(bytes).is_some()
}

fn is_sobelow_mix_invocation<S: AsRef<str>>(binary: &str, args: &[S]) -> bool {
    binary == "mix" && args.first().map(|arg| arg.as_ref()) == Some("sobelow")
}


pub(crate) fn extract_json_payload(bytes: &[u8]) -> Option<&[u8]> {
    let first_candidate = bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| matches!(**byte, b'{' | b'['))
        .map(|(index, _)| index)
        .next()?;

    for index in bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| matches!(**byte, b'{' | b'['))
        .map(|(index, _)| index)
    {
        let candidate = &bytes[index..];
        let mut stream = serde_json::Deserializer::from_slice(candidate).into_iter::<serde_json::Value>();
        if stream.next().and_then(Result::ok).is_some() {
            return Some(&candidate[..stream.byte_offset()]);
        }
    }

    Some(&bytes[first_candidate..])
}

pub(crate) fn extract_xml_payload(bytes: &[u8]) -> Option<&[u8]> {
    let first_candidate = bytes.iter().position(|byte| *byte == b'<')?;
    let text = std::str::from_utf8(bytes).ok()?;

    if let Some(index) = text.find("<?xml").or_else(|| text.find("<results")) {
        return Some(&bytes[index..]);
    }

    Some(&bytes[first_candidate..])
}

fn discover_monorepo_manifests(repo_path: &Path) -> Vec<DiscoveredManifest> {
    let mut builder = WalkBuilder::new(repo_path);
    builder.hidden(false);
    builder.git_ignore(false);
    builder.git_global(false);
    builder.git_exclude(false);
    builder.parents(false);
    builder.threads(1);
    builder.filter_entry(|entry| !should_skip_monorepo_dir(entry.path()));

    let mut manifests = Vec::new();
    for entry in builder.build() {
        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        let Some(file_name) = entry.path().file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(kind) = monorepo_manifest_kind_for_name(file_name) else {
            continue;
        };
        let execution_root = entry
            .path()
            .parent()
            .unwrap_or(repo_path)
            .to_path_buf();
        manifests.push(DiscoveredManifest {
            kind,
            manifest_path: entry.path().to_path_buf(),
            scope: scope_label_for_path(repo_path, &execution_root),
            execution_root,
        });
    }
    manifests.sort_by(|left, right| {
        left.scope
            .cmp(&right.scope)
            .then_with(|| left.manifest_path.cmp(&right.manifest_path))
    });
    manifests.dedup_by(|left, right| left.kind == right.kind && left.execution_root == right.execution_root);
    manifests
}

fn monorepo_manifest_kind_for_name(file_name: &str) -> Option<ManifestKind> {
    match file_name {
        "Cargo.toml" => Some(ManifestKind::CargoToml),
        "package.json" => Some(ManifestKind::PackageJson),
        "mix.exs" => Some(ManifestKind::MixExs),
        "go.mod" => Some(ManifestKind::GoMod),
        _ => None,
    }
}

fn should_skip_monorepo_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|name| matches!(name, ".git" | "node_modules" | "target" | "venv" | "dist"))
        .unwrap_or(false)
}

fn scope_label_for_path(repo_path: &Path, execution_root: &Path) -> String {
    execution_root
        .strip_prefix(repo_path)
        .ok()
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

async fn cleanup_rust_cargo_sandbox_state(execution_root: &Path) {
    for tool_name in ["cargo-clippy-target", "cargo-target", "cargo-home"] {
        let target_dir = crate::harvester::sandbox::sandbox_tool_state_root(
            execution_root,
            tool_name,
        );
        if !target_dir.exists() {
            continue;
        }

        match tokio::fs::remove_dir_all(&target_dir).await {
            Ok(_) => {
                info!(
                    target_dir = %target_dir.display(),
                    tool_name,
                    "cargo-clippy: estado efemero removido"
                );
            }
            Err(err) => {
                warn!(
                    target_dir = %target_dir.display(),
                    tool_name,
                    error = %err,
                    "cargo-clippy: falha ao remover estado efemero"
                );
            }
        }
    }
}

async fn run_opengrep_scan<E: SandboxExecutor>(
    executor: &E,
    timeout_secs: u64,
    execution_root: &Path,
    scan_targets: &[String],
    forced_channel: Option<SastIssueChannel>,
) -> Result<Vec<u8>, SidecarError> {
    // L12: Escreve o arquivo .semgrepignore no diretório de trabalho do processo
    opengrep::write_semgrepignore_file(execution_root)?;
    let rule_set = match forced_channel {
        Some(SastIssueChannel::UnsafeHotspot) => SemgrepRuleSet::Security,
        _ => SemgrepRuleSet::Health,
    };
    let rules_dir: PathBuf = opengrep::ensure_semgrep_rule_bundle(executor.repo_path(), rule_set).await?;
    let rules_file = rules_dir.join(rule_set.rule_file_name());
    let rules_arg = rules_file.display().to_string();

    let args = opengrep::opengrep_args(&rules_arg, scan_targets, rule_set);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();

    execute_sidecar_in_dir(
        executor,
        "opengrep",
        &arg_refs,
        timeout_secs,
        SidecarExitPolicy::AllowFindingsExitOne,
        execution_root,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_sast_blade<E: SandboxExecutor>(
    executor: &E,
    blade: StaticAnalysisBlade,
    timeout_secs: u64,
    execution_root: &Path,
    scope: &str,
    scan_targets: &[String],
    command_args: Option<&[String]>,
    forced_channel: Option<SastIssueChannel>,
    has_global_opengrep_coverage: bool,
) -> Result<SastBladeResult, SidecarError> {
    if blade == StaticAnalysisBlade::Opengrep {
        return run_opengrep_scan(executor, timeout_secs, execution_root, scan_targets, forced_channel)
            .await
            .map(|bytes| SastBladeResult {
                effective_blade: StaticAnalysisBlade::Opengrep,
                bytes,
            });
    }
    if blade == StaticAnalysisBlade::Govulncheck {
        // Pre-Flight Fetch para Govulncheck: go mod download
        // Nos testes unitários mockados, evitamos a execução real no host.
        if !cfg!(test) {
            info!(
                execution_root = %execution_root.display(),
                "SAST govulncheck: Executando Pre-Flight go mod download assincrono no sub-scan path"
            );
            let mut cmd = tokio::process::Command::new("go");
            cmd.arg("mod")
               .arg("download")
               .current_dir(execution_root)
               .stdout(std::process::Stdio::piped())
               .stderr(std::process::Stdio::piped())
               .kill_on_drop(true);

            match cmd.spawn() {
                Ok(child) => {
                    let wait_fut = child.wait_with_output();
                    match tokio::time::timeout(std::time::Duration::from_secs(60), wait_fut).await {
                        Ok(Ok(output)) => {
                            if output.status.success() {
                                info!(
                                    execution_root = %execution_root.display(),
                                    "SAST govulncheck: Pre-Flight go mod download concluido com sucesso"
                                );
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                warn!(
                                    execution_root = %execution_root.display(),
                                    stderr = %stderr.trim(),
                                    "SAST govulncheck: Pre-Flight go mod download falhou (prosseguindo offline)"
                                );
                            }
                        }
                        Ok(Err(e)) => {
                            warn!(
                                execution_root = %execution_root.display(),
                                error = %e,
                                "SAST govulncheck: Erro ao executar Pre-Flight go mod download (prosseguindo offline)"
                            );
                        }
                        Err(_) => {
                            warn!(
                                execution_root = %execution_root.display(),
                                "SAST govulncheck: Timeout no Pre-Flight go mod download (prosseguindo offline)"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        execution_root = %execution_root.display(),
                        error = %e,
                        "SAST govulncheck: Falha ao iniciar Pre-Flight go mod download assincrono (prosseguindo offline)"
                    );
                }
            }
        }
    }
    let result = if blade == StaticAnalysisBlade::RustClippy {
        // L14: Inteligência Topológica de Workspace (ADR-025).
        // Se execution_root é uma sub-pasta de um workspace, o cargo clippy
        // DEVE ser executado a partir da raiz do workspace (ws_root), não da sub-pasta.
        // Isso evita o erro "manifest not found" quando o cargo tenta resolver
        // dependências do workspace a partir de um sub-crate.
        let repo_path = executor.repo_path();
        let ws_root = clippy::find_cargo_workspace_root(repo_path, execution_root);
        let run_dir = if ws_root != execution_root {
            info!(
                execution_root = %execution_root.display(),
                workspace_root = %ws_root.display(),
                "SAST rust-clippy: Monorepo detectado. Executando clippy a partir da raiz do workspace."
            );
            ws_root
        } else {
            execution_root.to_path_buf()
        };
        match clippy::run_rust_clippy_preflight(executor, &run_dir, timeout_secs).await {
            Ok(()) => {
                let (binary, args) = blade_command(blade, scan_targets, command_args);
                let manifest_path = run_dir.join("Cargo.toml");
                let manifest_path_str = manifest_path.display().to_string();
                
                let mut final_args = Vec::new();
                for arg in args {
                    final_args.push(arg);
                    if final_args.last().map(|s| s.as_str()) == Some("clippy") {
                        final_args.push("--manifest-path".to_string());
                        final_args.push(manifest_path_str.clone());
                    }
                }
                
                if !final_args.iter().any(|arg| arg == "--no-deps") {
                    if let Some(pos) = final_args.iter().position(|arg| arg == "--") {
                        final_args.insert(pos + 1, "--no-deps".to_string());
                    } else {
                        final_args.push("--".to_string());
                        final_args.push("--no-deps".to_string());
                    }
                }
                
                let arg_refs = final_args.iter().map(String::as_str).collect::<Vec<_>>();
                execute_sidecar_in_dir(
                    executor,
                    binary,
                    &arg_refs,
                    timeout_secs,
                    SidecarExitPolicy::AllowFindingsExitOne,
                    &run_dir,
                )
                .await
                .map(|bytes| SastBladeResult {
                    effective_blade: StaticAnalysisBlade::RustClippy,
                    bytes,
                })
            }
            Err(err) => {
                if let Some(reason) = clippy::rust_clippy_should_fallback_to_opengrep(&err) {
                    if has_global_opengrep_coverage {
                        info!(
                            scope = %scope,
                            cwd = %execution_root.display(),
                            reason = %reason,
                            "Fallback para Opengrep ignorado em {}: Opengrep global ja cobre a base",
                            scope
                        );
                        Ok(SastBladeResult {
                            effective_blade: StaticAnalysisBlade::RustClippy,
                            bytes: Vec::new(),
                        })
                    } else {
                        warn!(
                            cwd = %execution_root.display(),
                            reason = %reason,
                            "Clippy bloqueado por Trava C de seguranca. Realizando fallback para Opengrep SAST."
                        );
                        run_opengrep_scan(
                            executor,
                            timeout_secs,
                            execution_root,
                            scan_targets,
                            Some(SastIssueChannel::UnsafeHotspot),
                        )
                            .await
                            .map(|bytes| SastBladeResult {
                                effective_blade: StaticAnalysisBlade::Opengrep,
                                bytes,
                            })
                    }
                } else {
                    Err(err)
                }
            }
        }
    } else {
        let (binary, args) = blade_command(blade, scan_targets, command_args);
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        execute_sidecar_in_dir(
            executor,
            binary,
            &arg_refs,
            timeout_secs,
            SidecarExitPolicy::AllowFindingsExitOne,
            execution_root,
        )
        .await
        .map(|bytes| SastBladeResult {
            effective_blade: blade,
            bytes,
        })
    };
    if blade == StaticAnalysisBlade::RustClippy {
        cleanup_rust_cargo_sandbox_state(execution_root).await;
    }
    result
}

fn normalize_sast_output(
    repo_path: &Path,
    execution_root: &Path,
    blade: StaticAnalysisBlade,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    match blade {
        StaticAnalysisBlade::RustClippy => clippy::normalize_clippy_output(repo_path, execution_root, bytes),
        StaticAnalysisBlade::Cppcheck => cppcheck::normalize_cppcheck_output(repo_path, execution_root, bytes),
        StaticAnalysisBlade::Opengrep => opengrep::normalize_semgrep_payload(repo_path, bytes).map(|p| {
            p.blocks.into_iter().flat_map(|b| {
                b.items.into_iter().map(move |item| {
                    let channel = classify_sast_issue(StaticAnalysisBlade::Opengrep, "warning", &item);
                    SodaHealthIssue {
                        level: "warning".to_string(),
                        file: b.file_path.clone(),
                        message: item,
                        source_blade: blade_name(StaticAnalysisBlade::Opengrep).to_string(),
                        channel,
                    }
                })
            }).collect()
        }),
        StaticAnalysisBlade::Govulncheck => govulncheck::normalize_govulncheck_output(repo_path, execution_root, bytes),
        StaticAnalysisBlade::Ruff
        | StaticAnalysisBlade::Bandit
        | StaticAnalysisBlade::Biome
        | StaticAnalysisBlade::Oxc
        | StaticAnalysisBlade::Sobelow => {
            normalize_json_object_issues(repo_path, execution_root, blade, bytes)
        }
    }
}


fn normalize_json_object_issues(
    repo_path: &Path,
    execution_root: &Path,
    blade: StaticAnalysisBlade,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    if blade == StaticAnalysisBlade::Sobelow && stdout_is_blank(bytes) {
        return Ok(Vec::new());
    }
    let value = match parse_json_payload::<serde_json::Value>(bytes) {
        Ok(value) => value,
        Err(err) if blade == StaticAnalysisBlade::Sobelow => {
            let fallback =
                sobelow::normalize_sobelow_text_issues(repo_path, execution_root, &String::from_utf8_lossy(bytes));
            if fallback.is_empty() {
                return Err(SidecarError::ParseError {
                    reason: err.to_string(),
                });
            }
            return Ok(fallback);
        }
        Err(err) => {
            return Err(SidecarError::ParseError {
                reason: err.to_string(),
            });
        }
    };
    let issues = match blade {
        StaticAnalysisBlade::Ruff => value
            .as_array()
            .map(|items| {
                normalize_json_array_issues(
                    repo_path,
                    execution_root,
                    StaticAnalysisBlade::Ruff,
                    items,
                    JsonIssueFieldMap {
                        file_keys: &["filename", "file"],
                        level_keys: &["level", "severity"],
                        message_keys: &["message"],
                        line_keys: &["location.row", "line"],
                    },
                )
            })
            .unwrap_or_default(),
        StaticAnalysisBlade::Bandit => value
            .get("results")
            .and_then(|value| value.as_array())
            .map(|items| {
                normalize_json_array_issues(
                    repo_path,
                    execution_root,
                    StaticAnalysisBlade::Bandit,
                    items,
                    JsonIssueFieldMap {
                        file_keys: &["filename", "file"],
                        level_keys: &["issue_severity", "severity"],
                        message_keys: &["issue_text", "message"],
                        line_keys: &["line_number", "line"],
                    },
                )
            })
            .unwrap_or_default(),
        StaticAnalysisBlade::Biome | StaticAnalysisBlade::Oxc | StaticAnalysisBlade::Sobelow => {
            let items = value
                .get("diagnostics")
                .and_then(|value| value.as_array())
                .or_else(|| value.get("findings").and_then(|value| value.as_array()))
                .or_else(|| value.as_array());
            items.map(|items| {
                normalize_json_array_issues(
                    repo_path,
                    execution_root,
                    blade,
                    items,
                    JsonIssueFieldMap {
                        file_keys: &["file", "path", "filename"],
                        level_keys: &["severity", "level", "confidence"],
                        message_keys: &["message", "description", "title", "type"],
                        line_keys: &["line", "line_number"],
                    },
                )
            })
            .unwrap_or_default()
        }
        _ => Vec::new(),
    };
    Ok(issues)
}

struct JsonIssueFieldMap<'a> {
    file_keys: &'a [&'a str],
    level_keys: &'a [&'a str],
    message_keys: &'a [&'a str],
    line_keys: &'a [&'a str],
}

fn normalize_json_array_issues(
    repo_path: &Path,
    execution_root: &Path,
    blade: StaticAnalysisBlade,
    items: &[serde_json::Value],
    field_map: JsonIssueFieldMap<'_>,
) -> Vec<SodaHealthIssue> {
    let mut issues = Vec::new();
    for item in items {
        let file = field_map
            .file_keys
            .iter()
            .find_map(|key| json_value_at_path(item, key).and_then(|value| value.as_str()))
            .unwrap_or("");
        let level = field_map
            .level_keys
            .iter()
            .find_map(|key| json_value_at_path(item, key).and_then(|value| value.as_str()))
            .unwrap_or("warning");
        let message = field_map
            .message_keys
            .iter()
            .find_map(|key| json_value_at_path(item, key).and_then(|value| value.as_str()))
            .unwrap_or("diagnostic");
        let line_suffix = field_map
            .line_keys
            .iter()
            .find_map(|key| value_as_u32(json_value_at_path(item, key)))
            .map(|line| format!(" (line {line})"))
            .unwrap_or_default();
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            blade,
            level,
            file,
            &format!("{message}{line_suffix}"),
        );
    }
    sort_and_dedup_issues(&mut issues);
    issues
}

fn json_value_at_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn value_as_u32(value: Option<&serde_json::Value>) -> Option<u32> {
    value.and_then(|value| value.as_u64()).and_then(|value| u32::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::router::StaticAnalysisBlade;

    #[test]
    fn test_render_scoped_text_blocks_slices_domains_orthogonally() {
        let rendered = render_scoped_text_blocks(&[
            ScopedTextBlock {
                file_path: "src/lib.rs".to_string(),
                items: vec!["pub fn run()".to_string()],
                omitted_count: 0,
            },
            ScopedTextBlock {
                file_path: "candle-kernels/conv.cu".to_string(),
                items: vec!["__global__ void conv_kernel".to_string()],
                omitted_count: 0,
            },
            ScopedTextBlock {
                file_path: "candle-metal-kernels/ops.metal".to_string(),
                items: vec!["kernel void softmax".to_string()],
                omitted_count: 0,
            },
        ]);

        assert!(rendered.contains("[DOMAIN: RUST]"));
        assert!(rendered.contains("[DOMAIN: C++ / CUDA]"));
        assert!(rendered.contains("[DOMAIN: OBJECTIVE-C / METAL]"));
        assert!(rendered.contains("[src/lib.rs]"));
        assert!(rendered.contains("[candle-kernels/conv.cu]"));
        assert!(rendered.contains("[candle-metal-kernels/ops.metal]"));
    }

    #[test]
    fn test_render_unsafe_hotspots_report_keeps_domain_headers_without_findings() {
        let rendered = String::from_utf8(render_unsafe_hotspots_report(
            &[],
            &[
                PathBuf::from("services/api/main.go"),
                PathBuf::from("web/app.ts"),
            ],
        ))
        .unwrap();

        assert!(rendered.contains("# Unsafe Hotspots"));
        assert!(rendered.contains("[DOMAIN: GO]"));
        assert!(rendered.contains("[DOMAIN: JAVASCRIPT / TYPESCRIPT]"));
        assert!(rendered.contains("Sem linhas vermelhas estaticas relevantes."));
    }

    #[test]
    fn test_sanitize_repo_relative_path_strips_windows_host_prefix() {
        let repo_path = Path::new(r"C:\host\projfs\owner\repo");
        let sanitized = sanitize_repo_relative_path(repo_path, r"C:\host\projfs\owner\repo\crates\goose\src\main.rs");
        assert_eq!(sanitized.as_deref(), Some("crates/goose/src/main.rs"));
    }

    #[test]
    fn test_sanitize_repo_relative_path_drops_semgrep_support_paths() {
        let repo_path = Path::new(r"C:\host\projfs\owner\repo");
        let support_path = r"C:\host\projfs\owner\.soda_semgrep\repo\sandbox\.semgrep\settings.yml";
        assert_eq!(sanitize_repo_relative_path(repo_path, support_path), None);
    }

    #[test]
    fn test_aesthetic_warning_filter_drops_minor_js_python_noise_but_preserves_signal() {
        assert!(is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Ruff,
            "Property docstring should not start with a verb"
        ));
        assert!(is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Ruff,
            "f-string without any placeholders"
        ));
        assert!(is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Biome,
            "Alternative text title element cannot be empty"
        ));
        assert!(is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Biome,
            "ARIA attributes should be valid"
        ));
        assert!(is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Oxc,
            "Catch parameter 'e' is caught but never used"
        ));
        assert!(is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Bandit,
            "Use of assert detected."
        ));
        assert!(!is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Bandit,
            "Potential shell injection via subprocess"
        ));
        assert!(!is_aesthetic_or_minor_warning(
            StaticAnalysisBlade::Opengrep,
            "panic! found in hot path"
        ));
    }

    #[test]
    fn test_blob06_semantic_filter_drops_cppcheck_and_biome_slop_but_preserves_security_signal() {
        assert!(should_drop_sast_issue(
            StaticAnalysisBlade::Cppcheck,
            "warning",
            "[INFO] style issue: variable can be declared as const"
        ));
        assert!(should_drop_sast_issue(
            StaticAnalysisBlade::Biome,
            "warning",
            "Import specifier could not resolve and is never used"
        ));
        assert!(should_drop_sast_issue(
            StaticAnalysisBlade::Biome,
            "warning",
            "Dependency react isn't specified"
        ));
        assert!(should_drop_sast_issue(
            StaticAnalysisBlade::Cppcheck,
            "warning",
            "could not find or open any of the paths given"
        ));
        assert!(!should_drop_sast_issue(
            StaticAnalysisBlade::Cppcheck,
            "warning",
            "Memory leak: ptr"
        ));
        assert!(!should_drop_sast_issue(
            StaticAnalysisBlade::Biome,
            "error",
            "Potential command injection vulnerability"
        ));
        assert!(!should_drop_sast_issue(
            StaticAnalysisBlade::Bandit,
            "warning",
            "Potential shell injection via subprocess"
        ));
    }

    #[test]
    fn test_blob06_allowlist_only_keeps_biome_and_cppcheck_when_message_has_security_signal() {
        let biome_slop = SodaHealthIssue {
            level: "error".to_string(),
            file: "src/app.tsx".to_string(),
            message: "An empty interface is equivalent to {}.".to_string(),
            source_blade: "biome".to_string(),
            channel: SastIssueChannel::UnsafeHotspot,
        };
        let biome_security = SodaHealthIssue {
            message: "dangerouslySetInnerHTML may enable injection".to_string(),
            ..biome_slop.clone()
        };
        let cppcheck_overflow = SodaHealthIssue {
            level: "error".to_string(),
            file: "src/main.c".to_string(),
            message: "Potential buffer overflow in parser".to_string(),
            source_blade: "cppcheck".to_string(),
            channel: SastIssueChannel::UnsafeHotspot,
        };
        let bandit_signal = SodaHealthIssue {
            level: "warning".to_string(),
            file: "service.py".to_string(),
            message: "Possible shell injection via subprocess".to_string(),
            source_blade: "bandit".to_string(),
            channel: SastIssueChannel::UnsafeHotspot,
        };
        let health_biome = SodaHealthIssue {
            message: "Function is too complex".to_string(),
            channel: SastIssueChannel::Health,
            ..biome_slop.clone()
        };

        assert!(!should_keep_blob06_issue(&biome_slop));
        assert!(should_keep_blob06_issue(&biome_security));
        assert!(should_keep_blob06_issue(&cppcheck_overflow));
        assert!(should_keep_blob06_issue(&bandit_signal));
        assert!(should_keep_blob06_issue(&health_biome));
    }

    #[test]
    fn test_should_skip_sast_relative_target_handles_testutil_and_test_file_patterns() {
        assert!(should_skip_sast_relative_target("pkg/testutil/helpers.go"));
        assert!(should_skip_sast_relative_target("pkg/service/foo_test.go"));
        assert!(should_skip_sast_relative_target("src/app.test.ts"));
        assert!(should_skip_sast_relative_target("src/app.spec.ts"));
        assert!(should_skip_sast_relative_target("crates/core/render_test.rs"));
        assert!(should_skip_sast_relative_target("vendor/prism.js"));
        assert!(should_skip_sast_relative_target("public/libs/prism.js"));
        assert!(should_skip_sast_relative_target("src/vendor.bundle.js"));
        assert!(should_skip_sast_relative_target("src/prism.min.js"));
        assert!(!should_skip_sast_relative_target("src/app.ts"));
    }

    #[test]
    fn test_blob08_allowlist_only_keeps_health_findings_with_technical_debt_signal() {
        let biome_slop = SodaHealthIssue {
            level: "warning".to_string(),
            file: "src/app.ts".to_string(),
            message: "An empty interface is equivalent to {}.".to_string(),
            source_blade: "biome".to_string(),
            channel: SastIssueChannel::Health,
        };
        let biome_complexity = SodaHealthIssue {
            message: "complexity threshold exceeded in request mapper".to_string(),
            ..biome_slop.clone()
        };
        let opengrep_unwrap = SodaHealthIssue {
            message: "unwrap encontrado em caminho critico".to_string(),
            source_blade: "opengrep".to_string(),
            ..biome_slop.clone()
        };
        let clippy_signal = SodaHealthIssue {
            message: "use of deprecated item".to_string(),
            source_blade: "clippy".to_string(),
            ..biome_slop.clone()
        };
        let unsafe_issue = SodaHealthIssue {
            message: "unsafe merece auditoria manual".to_string(),
            channel: SastIssueChannel::UnsafeHotspot,
            ..biome_slop.clone()
        };

        assert!(!should_keep_blob08_issue(&biome_slop));
        assert!(should_keep_blob08_issue(&biome_complexity));
        assert!(should_keep_blob08_issue(&opengrep_unwrap));
        assert!(should_keep_blob08_issue(&clippy_signal));
        assert!(should_keep_blob08_issue(&unsafe_issue));
    }

    #[test]
    fn test_normalize_relative_issue_file_prefixes_subproject_scope() {
        let repo_path = Path::new("C:/repos/firecrawl");
        let execution_root = Path::new("C:/repos/firecrawl/apps/rust-sdk");
        let normalized = normalize_relative_issue_file(repo_path, execution_root, "src/lib.rs");
        assert_eq!(normalized, "apps/rust-sdk/src/lib.rs");
    }

    #[test]
    fn test_render_soda_health_report_groups_findings_by_domain() {
        let issues = vec![
            SodaHealthIssue {
                level: "warning".to_string(),
                file: "src/lib.rs".to_string(),
                message: "unwrap precisa de contexto".to_string(),
                source_blade: "rust-clippy".to_string(),
                channel: SastIssueChannel::Health,
            },
            SodaHealthIssue {
                level: "warning".to_string(),
                file: "candle-kernels/sgemm.cu".to_string(),
                message: "kernel sem bounds check".to_string(),
                source_blade: "cppcheck".to_string(),
                channel: SastIssueChannel::Health,
            },
            SodaHealthIssue {
                level: "warning".to_string(),
                file: "candle-metal-kernels/reduce.metal".to_string(),
                message: "metal path requer auditoria".to_string(),
                source_blade: "opengrep".to_string(),
                channel: SastIssueChannel::Health,
            },
        ];

        let rendered = String::from_utf8(render_soda_health_report(&issues)).unwrap();

        assert!(rendered.contains("[DOMAIN: RUST]"));
        assert!(rendered.contains("[DOMAIN: C++ / CUDA]"));
        assert!(rendered.contains("[DOMAIN: OBJECTIVE-C / METAL]"));
        assert!(rendered.contains("src/lib.rs"));
        assert!(rendered.contains("candle-kernels/sgemm.cu"));
        assert!(rendered.contains("candle-metal-kernels/reduce.metal"));
    }

    #[test]
    fn test_render_soda_health_report_keeps_cppcheck_clean_info_under_cpp_domain() {
        let issues = vec![SodaHealthIssue {
            level: "info".to_string(),
            file: String::new(),
            message: "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck.".to_string(),
            source_blade: "cppcheck".to_string(),
            channel: SastIssueChannel::Health,
        }];

        let rendered = String::from_utf8(render_soda_health_report(&issues)).unwrap();

        assert!(rendered.contains("[DOMAIN: C++ / CUDA]"));
        assert!(rendered.contains("[cppcheck]"));
        assert!(rendered.contains("[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck."));
    }
    /// PRD-033 TDD (RED→GREEN)
    /// Verifica que quando a lâmina RustClippy falha (ex: crate com build.rs/FFI que
    /// não compila no sandbox), o diagnóstico forense é injetado no TOPO do Blob 08
    /// e o pipeline NÃO aborta — retornando Ok(PolyglotSastArtifacts).
    #[tokio::test]
    async fn test_prd033_clippy_failure_injects_forensic_header_at_top_of_blob08() {
        use crate::harvester::sast::test_utils::MockExecutor;
        use crate::harvester::sandbox::SandboxError;
        use crate::harvester::detect::StackProfile;

        // Montar um executor com um Cargo.toml simples (sem lockfile = sem --locked)
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file(
            "Cargo.toml",
            "[package]\nname='toxic-crate'\nversion='0.1.0'\nbuild='build.rs'\n",
        );
        // Forçar falha no preflight (cargo fetch) simulando erro de compilação
        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 101,
                stderr: "error[E0463]: can't find crate for `proc_macro`".to_string(),
                stdout: Vec::new(),
            }),
        ]);

        let executor = std::sync::Arc::new(executor);
        let clean_files = std::sync::Arc::new(Vec::new());

        let result = PolyglotSastSidecar::extract(PolyglotSastInput {
            executor,
            timeout_secs: 30,
            profile: &StackProfile::Rust,
            clean_files,
        })
        .await;

        // Pipeline NÃO deve abortar
        let artifacts = result.expect("pipeline nao deve abortar quando clippy falha (PRD-033)");

        // Blob 08 deve começar com o marcador forense
        let blob08 = String::from_utf8_lossy(&artifacts.health_report_blob);
        assert!(
            blob08.starts_with("[DIAGNÓSTICO ESTRUTURAL RUST: FALHA FATAL DE COMPILAÇÃO OU RCE BLOQUEADO]"),
            "Blob 08 deve comecar com marcador forense PRD-033, mas foi:\n{blob08}"
        );
        // O erro original deve estar presente no payload
        assert!(
            blob08.contains("E0463") || blob08.contains("proc_macro") || blob08.contains("101"),
            "Blob 08 deve conter o stderr do erro, mas foi:\n{blob08}"
        );
    }

    #[tokio::test]
    async fn test_fail_soft_continues_after_generic_blade_violation() {
        let executor = test_utils::MockExecutor::new(vec![
            Err(crate::harvester::sandbox::SandboxError::PolicyViolation {
                detail: "Policy Violation simulated".to_string(),
            }),
            Err(crate::harvester::sandbox::SandboxError::PolicyViolation {
                detail: "Policy Violation simulated".to_string(),
            }),
            Err(crate::harvester::sandbox::SandboxError::PolicyViolation {
                detail: "Policy Violation simulated".to_string(),
            }),
            Err(crate::harvester::sandbox::SandboxError::PolicyViolation {
                detail: "Policy Violation simulated".to_string(),
            }),
            Err(crate::harvester::sandbox::SandboxError::PolicyViolation {
                detail: "Policy Violation simulated".to_string(),
            }),
            Err(crate::harvester::sandbox::SandboxError::PolicyViolation {
                detail: "Policy Violation simulated".to_string(),
            }),
        ]);

        let executor = std::sync::Arc::new(executor);
        let repo_path = executor.repo_path().canonicalize().unwrap();
        let clean_files = std::sync::Arc::new(vec![repo_path.join("src/app.ts")]);

        let result = PolyglotSastSidecar::extract(PolyglotSastInput {
            executor,
            timeout_secs: 30,
            profile: &StackProfile::NodeJS,
            clean_files,
        })
        .await;

        let artifacts = result.expect("Lâmina com erro não deve abortar o pipeline");
        let blob08 = String::from_utf8_lossy(&artifacts.health_report_blob);

        assert!(
            blob08.contains("[DIAGNÓSTICO ESTRUTURAL: Lâmina 'biome' ignorada por violação/ausência]"),
            "Blob 08 deve registrar a falha de biome de forma fail-soft, mas foi:\n{blob08}"
        );
    }
}
