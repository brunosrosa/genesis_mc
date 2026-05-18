use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use oxc::{
    allocator::Allocator,
    ast::ast::CallExpression,
    ast_visit::{walk, Visit as OxcVisit},
    parser::{ParseOptions, Parser},
    span::SourceType,
};
use rusqlite::params;
use thiserror::Error;
use serde::Deserialize;
use syn::visit::Visit as SynVisit;
use tracing::error;
use crate::harvester::PHASE1_HEAVY_BLOB_MAX_CHARS;
use crate::harvester::detect::{SingleStack, StackProfile};
use crate::harvester::sandbox::SandboxError;

/// Trait para abstrair a execução no sandbox, permitindo mocks nos testes.
#[allow(async_fn_in_trait)]
pub trait SandboxExecutor {
    async fn execute(&self, command: &str, args: &[&str], timeout_secs: u64) -> Result<Vec<u8>, SandboxError>;
    fn repo_path(&self) -> &Path;
}

/// Implementação da trait SandboxExecutor para o SandboxHandle concreto.
impl SandboxExecutor for crate::harvester::sandbox::SandboxHandle {
    async fn execute(&self, command: &str, args: &[&str], timeout_secs: u64) -> Result<Vec<u8>, SandboxError> {
        self.execute(command, args, timeout_secs).await
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

pub struct JCodemunchInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
    pub persist_artifacts: Option<PersistArtifactConfig<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistArtifactConfig<'a> {
    pub repo_id: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidecarExitPolicy {
    StrictZeroOnly,
    AllowFindingsExitOne,
}

fn stdout_is_blank(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| byte.is_ascii_whitespace())
}

fn digest_json_is_empty(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::Object(map) => map.is_empty(),
        serde_json::Value::Array(items) => items.is_empty(),
        serde_json::Value::String(text) => text.trim().is_empty(),
        _ => false,
    }
}

fn stdout_contains_json_payload(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes).is_ok()
}

const BLOB_06_UNSAFE_HOTSPOTS_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const BLOB_08_HEALTH_REPORT_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const BLOB_04_REPO_OUTLINE_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const BLOB_05_ARCHITECTURE_MAP_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const SEMGREP_SECURITY_RULE_FILE: &str = ".soda_semgrep_blob_06_security.yml";
const SEMGREP_HEALTH_RULE_FILE: &str = ".soda_semgrep_blob_08_health.yml";
const SEMGREP_SECURITY_RULE_SOURCE: &str = include_str!("../../semgrep/blob_06_security.yml");
const SEMGREP_HEALTH_RULE_SOURCE: &str = include_str!("../../semgrep/blob_08_health.yml");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JCodemunchArtifacts {
    pub repo_outline_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
    pub architecture_map_blob: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemgrepArtifacts {
    pub unsafe_hotspots_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
}

pub struct SemgrepInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedTextBlock {
    pub file_path: String,
    pub items: Vec<String>,
    pub omitted_count: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemgrepRuleSet {
    Security,
    Health,
}

impl SemgrepRuleSet {
    fn artifact_title(self) -> &'static str {
        match self {
            Self::Security => "# Unsafe Hotspots",
            Self::Health => "# Health Report",
        }
    }

    fn rule_file_name(self) -> &'static str {
        match self {
            Self::Security => SEMGREP_SECURITY_RULE_FILE,
            Self::Health => SEMGREP_HEALTH_RULE_FILE,
        }
    }

    fn rule_source(self) -> &'static str {
        match self {
            Self::Security => SEMGREP_SECURITY_RULE_SOURCE,
            Self::Health => SEMGREP_HEALTH_RULE_SOURCE,
        }
    }

    fn default_message(self) -> &'static str {
        match self {
            Self::Security => "Sem hotspots estaticos relevantes do semgrep",
            Self::Health => "Sem divida tecnica estatica relevante do semgrep",
        }
    }

    fn max_chars(self) -> usize {
        match self {
            Self::Security => BLOB_06_UNSAFE_HOTSPOTS_MAX_CHARS,
            Self::Health => BLOB_08_HEALTH_REPORT_MAX_CHARS,
        }
    }
}

fn code_index_path_for_repo(repo_path: &Path) -> String {
    repo_path
        .parent()
        .unwrap_or(repo_path)
        .join(".jcodemunch_index")
        .display()
        .to_string()
}

fn code_index_db_path_for_repo(repo_path: &Path) -> Result<std::path::PathBuf, SidecarError> {
    let storage_path = repo_path
        .parent()
        .unwrap_or(repo_path)
        .join(".jcodemunch_index");
    let owner = repo_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: "Nao foi possivel resolver o owner do repositório para localizar o banco do jcodemunch".to_string(),
        })?;
    let repo = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: "Nao foi possivel resolver o nome do repositório para localizar o banco do jcodemunch".to_string(),
        })?;
    Ok(storage_path.join(format!("{}-{}.db", owner, repo)))
}

#[derive(Debug, Deserialize)]
struct IndexedImport {
    specifier: String,
    #[serde(default)]
    names: Vec<String>,
}

const VISUAL_ASSET_EXTENSIONS: &[&str] = &[".svg", ".css", ".scss", ".png", ".jpg", ".ico"];
const VISUAL_ONLY_DIRS: &[&str] = &["icons", "assets", "styles", "fonts"];
const NON_CORE_PATH_SEGMENTS: &[&str] = &[
    "tests",
    "test",
    "examples",
    "fixtures",
    "test_support",
    "e2e",
    "scenario_tests",
    "evals",
    "documentation",
    "ui",
    "components",
    "demo",
    "samples",
    "bench",
    "benches",
    "benchmark",
    "benchmarks",
    "playground",
];
const NON_CORE_FILE_PATTERNS: &[&str] = &["tests.rs", ".test.", ".spec.", "mock_"];
const BACKEND_PRIORITY_PREFIXES: &[(&str, u8)] = &[
    ("src/core/", 0),
    ("src/backend/", 1),
    ("lib/", 2),
    ("api/", 3),
    ("daemon/", 4),
    ("crates/", 5),
    ("services/", 6),
];

fn normalize_topology_key(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace('\\', "/")
        .replace("::", "/")
}

fn has_visual_extension(value: &str) -> bool {
    let normalized = normalize_topology_key(value);
    VISUAL_ASSET_EXTENSIONS
        .iter()
        .any(|extension| normalized.ends_with(extension))
}

fn has_visual_only_dir(value: &str) -> bool {
    let normalized = normalize_topology_key(value);
    normalized
        .split('/')
        .any(|segment| VISUAL_ONLY_DIRS.contains(&segment))
}

fn should_skip_visual_topology(value: &str) -> bool {
    has_visual_extension(value) || has_visual_only_dir(value)
}

fn should_skip_non_core_topology(value: &str) -> bool {
    let normalized = normalize_topology_key(value);
    normalized
        .split('/')
        .any(|segment| NON_CORE_PATH_SEGMENTS.contains(&segment))
        || NON_CORE_FILE_PATTERNS
        .iter()
        .any(|pattern| normalized.contains(pattern))
}

fn should_skip_topology_entry(value: &str) -> bool {
    should_skip_visual_topology(value) || should_skip_non_core_topology(value)
}

fn topology_priority_score(path: &str) -> u8 {
    let normalized = normalize_topology_key(path);
    for (prefix, score) in BACKEND_PRIORITY_PREFIXES {
        if normalized.starts_with(prefix) {
            return *score;
        }
    }

    if normalized.starts_with("src/") {
        return 10;
    }

    if normalized.starts_with("documentation/") || normalized.starts_with("docs/") {
        return 95;
    }

    if normalized.split('/').any(|segment| segment == "pages") {
        return 96;
    }

    if normalized.split('/').any(|segment| matches!(segment, "ui" | "components" | "frontend")) {
        return 100;
    }

    20
}

fn normalize_architecture_map(repo_path: &Path) -> Result<Vec<u8>, SidecarError> {
    let db_path = code_index_db_path_for_repo(repo_path)?;
    let project_prefixes = collect_project_prefixes(repo_path)?;

    let summary = tokio::task::block_in_place(|| {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao abrir banco topologico do jcodemunch: {}", e),
        })?;
        let mut stmt = conn
            .prepare("SELECT path, imports FROM files WHERE imports IS NOT NULL AND imports != '' ORDER BY path ASC")
            .map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao preparar query topologica do jcodemunch: {}", e),
            })?;

        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao consultar topologia do jcodemunch: {}", e),
            })?;

        let mut modules = Vec::new();
        for row in rows {
            let (path, imports_json) = row.map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao iterar topologia do jcodemunch: {}", e),
            })?;
            if should_skip_topology_entry(&path) {
                continue;
            }
            let imports: Vec<IndexedImport> = serde_json::from_str(&imports_json).map_err(|e| SidecarError::ParseError {
                reason: e.to_string(),
            })?;
            let mut relevant = imports
                .into_iter()
                .filter(|import| is_project_specifier(&import.specifier, &project_prefixes))
                .filter(|import| !should_skip_topology_entry(&import.specifier))
                .map(|import| {
                    if import.names.is_empty() {
                        import.specifier
                    } else {
                        format!("{} ({})", import.specifier, import.names.join(", "))
                    }
                })
                .collect::<Vec<_>>();

            relevant.sort_by(|a, b| {
                topology_priority_score(a)
                    .cmp(&topology_priority_score(b))
                    .then_with(|| a.cmp(b))
            });

            if !relevant.is_empty() {
                modules.push((path, relevant));
            }
        }

        if modules.is_empty() {
            return Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch index nao gerou relacoes topologicas internas para blob_05_architecture_map".to_string(),
            });
        }

        modules.sort_by(|a, b| {
            topology_priority_score(&a.0)
                .cmp(&topology_priority_score(&b.0))
                .then_with(|| b.1.len().cmp(&a.1.len()))
                .then_with(|| a.0.cmp(&b.0))
        });

        let mut blocks = Vec::new();
        for (path, imports) in modules {
            blocks.push(ScopedTextBlock {
                file_path: path,
                items: imports,
                omitted_count: 0,
            });
        }

        let mut summary = String::from("# Architecture Map");
        let packed_blocks = pack_scoped_text_blocks(&blocks, BLOB_05_ARCHITECTURE_MAP_MAX_CHARS.saturating_sub(summary.len()));
        if !packed_blocks.trim().is_empty() {
            summary.push_str("\n\n");
            summary.push_str(&packed_blocks);
        }
        Ok(summary)
    })?;

    let truncated = truncate_utf8(
        &summary,
        BLOB_05_ARCHITECTURE_MAP_MAX_CHARS,
        BLOB_05_ARCHITECTURE_MAP_MAX_CHARS,
    );
    if truncated.trim().is_empty() {
        return Err(SidecarError::ExecutionFailed {
            reason: "blob_05_architecture_map ficou vazio apos a truncagem".to_string(),
        });
    }

    Ok(truncated.into_bytes())
}

fn collect_project_prefixes(repo_path: &Path) -> Result<Vec<String>, SidecarError> {
    let mut prefixes = vec![
        "crate::".to_string(),
        "super::".to_string(),
        "self::".to_string(),
        "./".to_string(),
        "../".to_string(),
        "~/".to_string(),
        "@/".to_string(),
    ];

    let crates_dir = repo_path.join("crates");
    if crates_dir.is_dir() {
        for entry in std::fs::read_dir(&crates_dir).map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao ler diretório de crates para blob_05_architecture_map: {}", e),
        })? {
            let entry = entry.map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao iterar diretório de crates para blob_05_architecture_map: {}", e),
            })?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.is_empty() {
                prefixes.push(format!("{}::", name));
                prefixes.push(format!("{}::", name.replace('-', "_")));
            }
        }
    }

    prefixes.sort();
    prefixes.dedup();
    Ok(prefixes)
}

fn is_project_specifier(specifier: &str, project_prefixes: &[String]) -> bool {
    project_prefixes
        .iter()
        .any(|prefix| specifier.starts_with(prefix))
}

fn validate_index_response(bytes: &[u8]) -> Result<(), SidecarError> {
    if stdout_is_blank(bytes) {
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp index returned empty stdout".to_string(),
        });
    }

    let index_json = serde_json::from_slice::<serde_json::Value>(bytes).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })?;

    match index_json.get("success").and_then(|value| value.as_bool()) {
        Some(true) => Ok(()),
        Some(false) => {
            let detail = index_json
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown indexing failure");
            Err(SidecarError::ExecutionFailed {
                reason: format!("jcodemunch-mcp index failed: {}", detail),
            })
        }
        None => Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp index returned a payload without success flag".to_string(),
        }),
    }
}

async fn persist_artifact_blob(
    config: PersistArtifactConfig<'_>,
    artifact_type: &str,
    payload_blob: Vec<u8>,
) -> Result<(), SidecarError> {
    let repo_id = config.repo_id.to_string();
    let artifact_type = artifact_type.to_string();
    tokio::task::spawn_blocking(move || {
        let db_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
            .join(".soda_data")
            .join("soda_heuristic_vault.db");

        let conn = rusqlite::Connection::open(&db_path).map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao conectar no SQLite para persistir artefato: {}", e),
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao calcular timestamp de extracao: {}", e),
            })?
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(repo_id, artifact_type) DO UPDATE SET
                payload_blob = excluded.payload_blob,
                timestamp_extracao = excluded.timestamp_extracao",
            params![repo_id, artifact_type, payload_blob, now],
        )
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao persistir artefato no SQLite: {}", e),
        })?;

        Ok(())
    })
    .await
    .map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao aguardar persistencia do artefato: {}", e),
    })?
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
}

fn truncate_utf8(content: &str, max_chars: usize, max_bytes: usize) -> String {
    let mut out = String::new();
    for ch in content.chars().take(max_chars) {
        let ch_len = ch.len_utf8();
        if out.len() + ch_len > max_bytes {
            break;
        }
        out.push(ch);
    }
    out
}

fn normalize_health_report(bytes: &[u8]) -> Result<Vec<u8>, SidecarError> {
    if stdout_is_blank(bytes) {
        error!(binary = "jcodemunch-mcp", "Sidecar digest retornou stdout vazio");
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp digest returned empty stdout".to_string(),
        });
    }

    let digest_json = serde_json::from_slice::<serde_json::Value>(bytes).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })?;
    if digest_json_is_empty(&digest_json) {
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp digest returned an empty health payload".to_string(),
        });
    }

    let normalized = serde_json::to_string(&digest_json).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })?;
    let truncated = truncate_chars(&normalized, BLOB_08_HEALTH_REPORT_MAX_CHARS);
    if truncated.trim().is_empty() {
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp digest returned an empty health payload".to_string(),
        });
    }

    Ok(truncated.into_bytes())
}

fn looks_like_repo_outline_path(value: &str) -> bool {
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

fn normalize_repo_outline_markdown(text: &str) -> String {
    let mut leading = Vec::new();
    let mut blocks = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_items = Vec::new();

    let flush_current = |blocks: &mut Vec<ScopedTextBlock>, current_path: &mut Option<String>, current_items: &mut Vec<String>| {
        let Some(file_path) = current_path.take() else {
            return;
        };
        blocks.push(ScopedTextBlock {
            file_path,
            items: std::mem::take(current_items),
            omitted_count: 0,
        });
    };

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('#') {
            flush_current(&mut blocks, &mut current_path, &mut current_items);
            leading.push(trimmed.to_string());
            continue;
        }

        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .map(str::trim);

        let Some(content) = bullet else {
            if current_path.is_some() {
                current_items.push(trimmed.to_string());
            } else {
                leading.push(trimmed.to_string());
            }
            continue;
        };

        if looks_like_repo_outline_path(content) {
            flush_current(&mut blocks, &mut current_path, &mut current_items);
            current_path = Some(content.to_string());
        } else if current_path.is_some() {
            current_items.push(content.to_string());
        } else {
            leading.push(content.to_string());
        }
    }

    flush_current(&mut blocks, &mut current_path, &mut current_items);
    if blocks.is_empty() {
        return text.trim().to_string();
    }

    let mut normalized = leading.join("\n");
    let packed_blocks = pack_scoped_text_blocks(
        &blocks,
        BLOB_04_REPO_OUTLINE_MAX_CHARS.saturating_sub(normalized.len()),
    );
    if !packed_blocks.trim().is_empty() {
        if !normalized.is_empty() {
            normalized.push_str("\n\n");
        }
        normalized.push_str(&packed_blocks);
    }
    normalized
}

fn normalize_repo_outline(bytes: &[u8]) -> Result<Vec<u8>, SidecarError> {
    if stdout_is_blank(bytes) {
        error!(binary = "jcodemunch-mcp", "Sidecar claude-md retornou stdout vazio");
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp claude-md returned empty stdout".to_string(),
        });
    }

    let text = String::from_utf8_lossy(bytes);
    let normalized = normalize_repo_outline_markdown(&text);
    let truncated = truncate_chars(&normalized, BLOB_04_REPO_OUTLINE_MAX_CHARS);
    if truncated.trim().is_empty() {
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp claude-md returned an empty repo outline".to_string(),
        });
    }

    Ok(truncated.into_bytes())
}

/// Executa um binário sidecar no sandbox e retorna os bytes brutos do stdout.
/// Centraliza a tradução SandboxError → SidecarError para todos os sidecars.
async fn execute_sidecar<E: SandboxExecutor>(
    executor: &E,
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
    exit_policy: SidecarExitPolicy,
) -> Result<Vec<u8>, SidecarError> {
    match executor.execute(binary, args, timeout_secs).await {
        Ok(bytes) => Ok(bytes),
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
        // Match numérico explícito no exit code — sem manipulação de string.
        // Exit code 1: linters sinalizam violações encontradas (sucesso de negócio).
        // Exit code 2+: erro real de execução (config inválida, crash).
        Err(SandboxError::ProcessNonZeroExit { exit_code, stderr, stdout }) => {
            if binary == "semgrep" && !stdout_is_blank(&stdout) && stdout_contains_json_payload(&stdout) {
                Ok(stdout)
            } else if exit_code == 1 && matches!(exit_policy, SidecarExitPolicy::AllowFindingsExitOne) {
                Ok(stdout)
            } else {
                error!(
                    binary = %binary,
                    exit_code,
                    stderr = %stderr,
                    "Sidecar terminou com exit code nao zero"
                );
                Err(SidecarError::ExecutionFailed {
                    reason: format!("exit code {exit_code}: {stderr}"),
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

pub struct JCodemunchSidecar;

impl JCodemunchSidecar {
    /// Extrai o health report e o repo outline usando o jcodemunch no sandbox.
    pub async fn extract<E: SandboxExecutor>(
        input: JCodemunchInput<'_, E>,
    ) -> Result<JCodemunchArtifacts, SidecarError> {
        let storage_path = code_index_path_for_repo(input.executor.repo_path());
        let index_args = vec!["index".to_string(), "--no-ai-summaries".to_string()];
        let index_arg_refs: Vec<&str> = index_args.iter().map(String::as_str).collect();
        let index_bytes = execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &index_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await?;
        validate_index_response(&index_bytes)?;

        let digest_args = vec![
            "digest".to_string(),
            "--json".to_string(),
            "--storage-path".to_string(),
            storage_path,
        ];
        let digest_arg_refs: Vec<&str> = digest_args.iter().map(String::as_str).collect();
        let bytes = execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &digest_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await?;
        let health_report_blob = normalize_health_report(&bytes)?;

        let claude_md_args = vec!["claude-md".to_string(), "--generate".to_string()];
        let claude_md_arg_refs: Vec<&str> = claude_md_args.iter().map(String::as_str).collect();
        let claude_md_bytes = execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &claude_md_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await?;
        let repo_outline_blob = normalize_repo_outline(&claude_md_bytes)?;
        let architecture_map_blob = normalize_architecture_map(input.executor.repo_path())?;

        if let Some(config) = input.persist_artifacts {
            persist_artifact_blob(
                config,
                "blob_04_repo_outline",
                repo_outline_blob.clone(),
            )
            .await?;
            persist_artifact_blob(
                config,
                "blob_05_architecture_map",
                architecture_map_blob.clone(),
            )
            .await?;
        }

        Ok(JCodemunchArtifacts {
            repo_outline_blob,
            health_report_blob,
            architecture_map_blob,
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PropDeclaration {
    pub name: String,
    pub prop_type: String,
    pub has_default: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ComponentContract {
    pub name: String,
    pub file_path: String,
    pub framework: String,
    pub props: Vec<PropDeclaration>,
    pub events: Vec<String>,
    pub is_default_export: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UxContractsPayload {
    pub components: Vec<ComponentContract>,
    pub files_analyzed: u32,
}

pub struct OxcInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}

pub struct OxcSidecar;

impl OxcSidecar {
    /// Extrai os contratos UX (props, events, etc) usando o oxlint no sandbox.
    pub async fn extract<E: SandboxExecutor>(
        input: OxcInput<'_, E>,
    ) -> Result<UxContractsPayload, SidecarError> {
        let args = ["lint", "--format", "json", "--quiet", "."];
        let bytes = execute_sidecar(
            input.executor,
            "oxlint",
            &args,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await?;
        serde_json::from_slice::<UxContractsPayload>(&bytes).map_err(|e| SidecarError::ParseError {
            reason: e.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestIntentPayload {
    pub runner_name: String,
    pub blocks: Vec<ScopedTextBlock>,
}

pub struct NativeTestDiscoveryInput<'a> {
    pub repo_path: &'a Path,
    pub profile: &'a StackProfile,
}

const UNIVERSAL_TEST_SKIP_SEGMENTS: [&str; 8] = [
    "docs",
    "documentation",
    "examples",
    "mock",
    "mocks",
    "fixtures",
    "test_support",
    "e2e",
];
const UNIVERSAL_TEST_SKIP_SUBSTRINGS: [&str; 3] = ["integration_mocks", "mock_", "/docs/"];
const STATIC_TEST_DISCOVERY_MAX_FILE_BYTES: u64 = 262_144;
fn primary_stack(profile: &StackProfile) -> Option<SingleStack> {
    match profile {
        StackProfile::Rust => Some(SingleStack::Rust),
        StackProfile::NodeJS => Some(SingleStack::NodeJS),
        StackProfile::Python => Some(SingleStack::Python),
        StackProfile::Go => Some(SingleStack::Go),
        StackProfile::JVM => Some(SingleStack::JVM),
        StackProfile::DotNet => Some(SingleStack::DotNet),
        StackProfile::Mixed(stacks) => stacks.first().cloned(),
        StackProfile::Unknown => None,
    }
}

fn should_skip_discovered_test_entry(value: &str) -> bool {
    let normalized = value.trim().replace('\\', "/").to_ascii_lowercase();
    if normalized.is_empty() {
        return true;
    }

    if UNIVERSAL_TEST_SKIP_SUBSTRINGS
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    normalized
        .split(|ch| matches!(ch, '/' | ':' | '>' | ' '))
        .filter(|part| !part.is_empty())
        .any(|part| UNIVERSAL_TEST_SKIP_SEGMENTS.contains(&part))
}

fn is_known_test_file_path(value: &str) -> bool {
    let lower = value.trim().replace('\\', "/").to_ascii_lowercase();
    [
        ".test.ts",
        ".test.tsx",
        ".test.js",
        ".test.jsx",
        ".spec.ts",
        ".spec.tsx",
        ".spec.js",
        ".spec.jsx",
        "_test.go",
        "_test.py",
        "test_",
        "__tests__",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn supports_stack(profile: &StackProfile, target: SingleStack) -> bool {
    match profile {
        StackProfile::Mixed(stacks) => stacks.contains(&target),
        _ => primary_stack(profile) == Some(target),
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn should_skip_test_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".jj"
            | ".svn"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | ".jcodemunch_index"
            | "docs"
            | "documentation"
            | "examples"
            | "mock"
            | "mocks"
    )
}

fn is_supported_test_file(profile: &StackProfile, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    let normalized = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();

    match extension.as_deref() {
        Some("rs") => supports_stack(profile, SingleStack::Rust),
        Some("py") => {
            supports_stack(profile, SingleStack::Python)
                && (normalized.contains("/tests/")
                    || normalized.ends_with("_test.py")
                    || path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_ascii_lowercase().starts_with("test_"))
                        .unwrap_or(false))
        }
        Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts") => {
            supports_stack(profile, SingleStack::NodeJS) && is_known_test_file_path(&normalized)
        }
        _ => false,
    }
}

fn read_static_test_file(path: &Path) -> Result<Option<String>, SidecarError> {
    let metadata = std::fs::metadata(path).map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao ler metadata de '{}': {}", path.display(), e),
    })?;
    if metadata.len() > STATIC_TEST_DISCOVERY_MAX_FILE_BYTES {
        return Ok(None);
    }

    let bytes = std::fs::read(path).map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao ler '{}': {}", path.display(), e),
    })?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

fn collect_static_test_files(
    root: &Path,
    profile: &StackProfile,
    out: &mut Vec<PathBuf>,
) -> Result<(), SidecarError> {
    for entry in std::fs::read_dir(root).map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao listar '{}': {}", root.display(), e),
    })? {
        let entry = entry.map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao iterar '{}': {}", root.display(), e),
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao ler tipo de '{}': {}", path.display(), e),
        })?;

        if file_type.is_dir() {
            if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                if should_skip_test_dir(name) {
                    continue;
                }
            }
            collect_static_test_files(&path, profile, out)?;
            continue;
        }

        if !file_type.is_file() || !is_supported_test_file(profile, &path) {
            continue;
        }

        let relative = relative_display(root, &path);
        if should_skip_discovered_test_entry(&relative) {
            continue;
        }
        out.push(path);
    }

    Ok(())
}

fn capture_line_at_offset(source: &str, offset: u32) -> Option<&str> {
    let safe_offset = usize::try_from(offset).ok()?.min(source.len());
    let start = source[..safe_offset].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let end = source[safe_offset..]
        .find('\n')
        .map(|idx| safe_offset + idx)
        .unwrap_or(source.len());
    source.get(start..end)
}

fn normalize_code_line_snippet(snippet: &str) -> Option<String> {
    let normalized = snippet.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn compact_signature_text(signature: &str) -> Option<String> {
    let compact = signature
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_end_matches('{')
        .trim()
        .to_string();
    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

fn frontend_test_ast_input(path: &Path, content: &str) -> Option<(String, PathBuf)> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())?;

    match extension.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "mts" | "cts" => {
            Some((content.to_string(), path.to_path_buf()))
        }
        _ => None,
    }
}

#[derive(Default)]
struct JsTestAstCollector<'a> {
    source_text: &'a str,
    entries: BTreeSet<String>,
}

impl<'a> JsTestAstCollector<'a> {
    fn finish(self) -> Vec<String> {
        self.entries.into_iter().collect()
    }
}

impl<'a> OxcVisit<'a> for JsTestAstCollector<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(callee_name) = call.callee_name() {
            if matches!(callee_name, "describe" | "it" | "test") {
                if let Some(signature) = capture_line_at_offset(self.source_text, call.span.start)
                    .and_then(normalize_code_line_snippet)
                    .and_then(|line| compact_signature_text(&line))
                {
                    self.entries.insert(signature);
                }
            }
        }
        walk::walk_call_expression(self, call);
    }
}

fn parse_frontend_test_entries(path: &Path, content: &str) -> Vec<String> {
    let Some((source_text, parse_path)) = frontend_test_ast_input(path, content) else {
        return Vec::new();
    };
    let Ok(source_type) = SourceType::from_path(&parse_path) else {
        return Vec::new();
    };

    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, &source_text, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();
    if parser_return.panicked {
        return Vec::new();
    }

    let mut collector = JsTestAstCollector {
        source_text: &source_text,
        entries: BTreeSet::new(),
    };
    collector.visit_program(&parser_return.program);
    collector.finish()
}

fn rust_attr_is_test(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .map(|segment| {
            let ident = segment.ident.to_string();
            ident == "test" || ident == "rstest"
        })
        .unwrap_or(false)
}

#[derive(Default)]
struct RustTestAstCollector {
    entries: BTreeSet<String>,
}

impl<'ast> SynVisit<'ast> for RustTestAstCollector {
    fn visit_item_fn(&mut self, item_fn: &'ast syn::ItemFn) {
        let is_test = item_fn.sig.ident.to_string().starts_with("test_")
            || item_fn.attrs.iter().any(rust_attr_is_test);
        if is_test {
            let prefix = if item_fn.sig.asyncness.is_some() {
                "async fn"
            } else {
                "fn"
            };
            self.entries
                .insert(format!("{prefix} {}", item_fn.sig.ident));
        }
        syn::visit::visit_item_fn(self, item_fn);
    }
}

fn parse_rust_test_entries(content: &str) -> Vec<String> {
    let Ok(file) = syn::parse_file(content) else {
        return Vec::new();
    };

    let mut collector = RustTestAstCollector {
        entries: BTreeSet::new(),
    };
    collector.visit_file(&file);
    collector.entries.into_iter().collect()
}

fn parse_python_test_entries(content: &str) -> Vec<String> {
    let mut entries = BTreeSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let signature = if trimmed.starts_with("async def test_") {
            Some(trimmed.trim_end_matches(':'))
        } else if trimmed.starts_with("def test_") {
            Some(trimmed.trim_end_matches(':'))
        } else {
            None
        };

        if let Some(signature) = signature.and_then(compact_signature_text) {
            entries.insert(signature);
        }
    }
    entries.into_iter().collect()
}

fn build_scoped_blocks_from_pairs(
    pairs: Vec<(String, String)>,
) -> Vec<ScopedTextBlock> {
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    let mut seen = BTreeMap::<String, BTreeSet<String>>::new();

    for (file_path, item) in pairs {
        let is_new = seen
            .entry(file_path.clone())
            .or_default()
            .insert(item.clone());
        if is_new {
            grouped.entry(file_path).or_default().push(item);
        }
    }

    grouped
        .into_iter()
        .filter_map(|(file_path, items)| {
            if items.is_empty() {
                return None;
            }
            Some(ScopedTextBlock {
                file_path,
                items,
                omitted_count: 0,
            })
        })
        .collect()
}

fn discover_static_test_entries(
    repo_path: &Path,
    profile: &StackProfile,
) -> Result<Vec<ScopedTextBlock>, SidecarError> {
    let mut files = Vec::new();
    collect_static_test_files(repo_path, profile, &mut files)?;

    let mut entries = Vec::new();
    for path in files {
        let Some(content) = read_static_test_file(&path)? else {
            continue;
        };
        let relative = relative_display(repo_path, &path);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());

        let discovered = match extension.as_deref() {
            Some("rs") => parse_rust_test_entries(&content),
            Some("py") => parse_python_test_entries(&content),
            Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts") => {
                parse_frontend_test_entries(&path, &content)
            }
            _ => Vec::new(),
        };

        for entry in discovered {
            let candidate = format!("{} :: {}", relative, entry);
            if !should_skip_discovered_test_entry(&candidate) {
                entries.push((relative.clone(), entry));
            }
        }
    }

    Ok(build_scoped_blocks_from_pairs(entries))
}

pub struct NativeTestDiscoverySidecar;

impl NativeTestDiscoverySidecar {
    pub async fn extract(input: NativeTestDiscoveryInput<'_>) -> Result<TestIntentPayload, SidecarError> {
        let repo_path = input.repo_path.to_path_buf();
        let profile = input.profile.clone();
        let entries = tokio::task::spawn_blocking(move || discover_static_test_entries(&repo_path, &profile))
            .await
            .map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Static test discovery join failed: {}", e),
            })??;

        Ok(TestIntentPayload {
            runner_name: "static-ast".to_string(),
            blocks: entries,
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LintViolation {
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub file_path: String,
    pub line: u32,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StaticAnalysisPayload {
    pub violations: Vec<LintViolation>,
    pub files_analyzed: u32,
    pub linter_name: String,
}

pub struct StaticAnalysisInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}

pub struct StaticAnalysisSidecar;

impl StaticAnalysisSidecar {
    /// Extrai as violações de qualidade de código usando um linter no sandbox.
    pub async fn extract<E: SandboxExecutor>(
        input: StaticAnalysisInput<'_, E>,
        linter: &str,
        args: &[&str],
    ) -> Result<StaticAnalysisPayload, SidecarError> {
        let bytes = execute_sidecar(
            input.executor,
            linter,
            args,
            input.timeout_secs,
            SidecarExitPolicy::AllowFindingsExitOne,
        )
        .await?;
        serde_json::from_slice::<StaticAnalysisPayload>(&bytes).map_err(|e| SidecarError::ParseError {
            reason: e.to_string(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct SemgrepJsonPayload {
    #[serde(default)]
    results: Vec<SemgrepJsonResult>,
    #[serde(default)]
    paths: Option<SemgrepJsonPaths>,
}

#[derive(Debug, Deserialize)]
struct SemgrepJsonPaths {
    #[serde(default)]
    scanned: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SemgrepJsonResult {
    check_id: String,
    path: String,
    start: SemgrepJsonPosition,
    extra: SemgrepJsonExtra,
}

#[derive(Debug, Deserialize)]
struct SemgrepJsonPosition {
    line: u32,
}

#[derive(Debug, Deserialize)]
struct SemgrepJsonExtra {
    message: String,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemgrepNormalizedPayload {
    blocks: Vec<ScopedTextBlock>,
    files_analyzed: usize,
    findings_count: usize,
}

fn normalize_semgrep_path(repo_path: &Path, value: &str) -> String {
    let candidate = Path::new(value);
    let path = if candidate.is_absolute() {
        candidate
            .strip_prefix(repo_path)
            .unwrap_or(candidate)
            .to_path_buf()
    } else {
        candidate.to_path_buf()
    };
    path.to_string_lossy().replace('\\', "/")
}

fn semgrep_item_label(result: &SemgrepJsonResult) -> String {
    let severity = result.extra.severity.as_deref().unwrap_or("INFO");
    let category = result
        .extra
        .metadata
        .as_ref()
        .and_then(|value| value.get("category"))
        .and_then(|value| value.as_str());
    let message = result
        .extra
        .message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    match category {
        Some(category) => format!(
            "[{}] {}:L{} {} - {}",
            severity,
            result.check_id,
            result.start.line,
            category,
            message
        ),
        None => format!(
            "[{}] {}:L{} {}",
            severity,
            result.check_id,
            result.start.line,
            message
        ),
    }
}

fn normalize_semgrep_payload(
    repo_path: &Path,
    bytes: &[u8],
) -> Result<SemgrepNormalizedPayload, SidecarError> {
    let payload = serde_json::from_slice::<SemgrepJsonPayload>(bytes).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })?;

    let files_analyzed = payload
        .paths
        .as_ref()
        .map(|paths| paths.scanned.len())
        .unwrap_or(0);
    let findings_count = payload.results.len();
    let pairs = payload
        .results
        .into_iter()
        .map(|result| {
            (
                normalize_semgrep_path(repo_path, &result.path),
                semgrep_item_label(&result),
            )
        })
        .collect::<Vec<_>>();

    Ok(SemgrepNormalizedPayload {
        blocks: build_scoped_blocks_from_pairs(pairs),
        files_analyzed,
        findings_count,
    })
}

fn render_semgrep_blob(rule_set: SemgrepRuleSet, payload: &SemgrepNormalizedPayload) -> Vec<u8> {
    let mut text = format!(
        "{}\nsummary: files_analyzed={} findings={}",
        rule_set.artifact_title(),
        payload.files_analyzed,
        payload.findings_count
    );

    if payload.blocks.is_empty() {
        text.push_str("\n\n");
        text.push_str(rule_set.default_message());
    } else {
        let head_len = text.chars().count() + 2;
        let packed = pack_scoped_text_blocks(
            &payload.blocks,
            rule_set.max_chars().saturating_sub(head_len),
        );
        if !packed.trim().is_empty() {
            text.push_str("\n\n");
            text.push_str(&packed);
        }
    }

    truncate_utf8(&text, rule_set.max_chars(), rule_set.max_chars()).into_bytes()
}

async fn ensure_semgrep_rule_file(repo_path: &Path, rule_set: SemgrepRuleSet) -> Result<PathBuf, SidecarError> {
    let path = repo_path.join(rule_set.rule_file_name());
    tokio::fs::write(&path, rule_set.rule_source())
        .await
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao materializar regra do semgrep '{}': {}", path.display(), e),
        })?;
    Ok(path)
}

async fn run_semgrep_scan<E: SandboxExecutor>(
    executor: &E,
    rule_set: SemgrepRuleSet,
    timeout_secs: u64,
) -> Result<Vec<u8>, SidecarError> {
    let rule_path = ensure_semgrep_rule_file(executor.repo_path(), rule_set).await?;
    let rule_arg = rule_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: format!("Nome invalido para regra do semgrep '{}'", rule_path.display()),
        })?;
    let args = [
        "scan",
        "--config",
        rule_arg,
        "--json",
        "--jobs",
        "1",
        "--disable-version-check",
        "--metrics",
        "off",
        "--exclude",
        rule_arg,
        "--exclude",
        SEMGREP_SECURITY_RULE_FILE,
        "--exclude",
        SEMGREP_HEALTH_RULE_FILE,
        "--exclude",
        "docs",
        "--exclude",
        "documentation",
        "--exclude",
        "examples",
        "--exclude",
        "mock",
        "--exclude",
        "mocks",
        "--exclude",
        "fixtures",
        "--exclude",
        "test_support",
        "--exclude",
        "e2e",
        ".",
    ];
    execute_sidecar(
        executor,
        "semgrep",
        &args,
        timeout_secs,
        SidecarExitPolicy::AllowFindingsExitOne,
    )
    .await
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

        Ok(SemgrepArtifacts {
            unsafe_hotspots_blob: render_semgrep_blob(SemgrepRuleSet::Security, &security_payload),
            health_report_blob: render_semgrep_blob(SemgrepRuleSet::Health, &health_payload),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mock do SandboxExecutor que simula respostas customizadas para os testes.
    struct MockExecutor {
        _temp_dir: TempDir,
        repo_path: PathBuf,
        responses: Mutex<VecDeque<Result<Vec<u8>, SandboxError>>>,
        calls: Mutex<Vec<String>>,
    }

    impl MockExecutor {
        fn new(responses: Vec<Result<Vec<u8>, SandboxError>>) -> Self {
            let temp_dir = TempDir::new().unwrap();
            let owner_dir = temp_dir.path().join("owner");
            let repo_path = owner_dir.join("repo");
            std::fs::create_dir_all(&repo_path).unwrap();

            let index_dir = owner_dir.join(".jcodemunch_index");
            std::fs::create_dir_all(&index_dir).unwrap();
            let db_path = index_dir.join("owner-repo.db");
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute(
                "CREATE TABLE files (
                    path TEXT PRIMARY KEY,
                    hash TEXT,
                    mtime_ns INTEGER,
                    language TEXT,
                    summary TEXT,
                    blob_sha TEXT,
                    imports TEXT,
                    size_bytes INTEGER
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO files (path, imports) VALUES (?1, ?2)",
                params![
                    "src/main.rs",
                    r#"[{"specifier":"crate::config","names":["AppConfig"]},{"specifier":"goose_cli::session","names":["run_session"]},{"specifier":"serde_json","names":["json"]}]"#
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO files (path, imports) VALUES (?1, ?2)",
                params![
                    "src/lib.rs",
                    r#"[{"specifier":"super::utils","names":["normalize"]},{"specifier":"../shared/logger","names":["logger"]}]"#
                ],
            )
            .unwrap();

            Self {
                _temp_dir: temp_dir,
                repo_path,
                responses: Mutex::new(VecDeque::from(responses)),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

    }

    impl SandboxExecutor for MockExecutor {
        async fn execute(&self, command: &str, args: &[&str], _timeout_secs: u64) -> Result<Vec<u8>, SandboxError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", command, args.join(" ")).trim().to_string());
            let mut guard = self.responses.lock().unwrap();
            guard.pop_front().unwrap_or_else(|| {
                Err(SandboxError::ProcessSpawnFailed {
                    reason: "no mock response configured".to_string(),
                })
            })
        }

        fn repo_path(&self) -> &Path {
            &self.repo_path
        }
    }


    #[tokio::test]
    async fn test_extract_success() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let claude_md = "# Repository Outline\n\n- src/main.rs\n";

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
            Ok(claude_md.as_bytes().to_vec()),
        ]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ter sucesso: {:?}", result);
        let payload = result.unwrap();
        assert_eq!(
            String::from_utf8(payload.health_report_blob).unwrap(),
            r#"{"hotspots":[{"complexity":12,"path":"src/main.rs"}]}"#
        );
        assert_eq!(
            String::from_utf8(payload.repo_outline_blob).unwrap(),
            "# Repository Outline\n\n[src/main.rs]"
        );
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();
        assert!(architecture_map.contains("[src/main.rs]"));
        assert!(architecture_map.contains("- crate::config (AppConfig)"));
    }

    #[tokio::test]
    async fn test_extract_success_repo_outline_tolerates_invalid_utf8() {
        let claude_md = b"# Repository Outline\n\xff\n- src/main.rs\n".to_vec();

        let result = normalize_repo_outline(&claude_md);
        assert!(result.is_ok(), "Normalização deveria tolerar repo outline com UTF-8 inválido: {:?}", result);
        let repo_outline = String::from_utf8(result.unwrap()).unwrap();
        assert!(repo_outline.contains("# Repository Outline"));
        assert!(repo_outline.contains("[src/main.rs]"));
    }

    #[tokio::test]
    async fn test_architecture_map_skips_visual_noise_and_prioritizes_backend() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let claude_md = "# Repository Outline\n\n- src/main.rs\n";

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
            Ok(claude_md.as_bytes().to_vec()),
        ]);

        let db_path = code_index_db_path_for_repo(executor.repo_path()).unwrap();
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "icons/logo.svg",
                r#"[{"specifier":"crate::theme","names":["Palette"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "src/backend/service.rs",
                r#"[{"specifier":"crate::core::engine","names":["Engine"]},{"specifier":"./icons/logo.svg","names":["Logo"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "ui/panel.tsx",
                r#"[{"specifier":"../src/backend/service","names":["renderPanel"]},{"specifier":"./styles/panel.css","names":["panel"]}]"#
            ],
        )
        .unwrap();

        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };
        let payload = JCodemunchSidecar::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(!architecture_map.contains("icons/logo.svg"));
        assert!(!architecture_map.contains("panel.css"));

        let backend_pos = architecture_map.find("src/backend/service.rs ->").unwrap();
        let ui_pos = architecture_map.find("ui/panel.tsx ->").unwrap();
        assert!(backend_pos < ui_pos, "backend deve vir antes de ui: {}", architecture_map);
    }

    #[tokio::test]
    async fn test_architecture_map_skips_tests_examples_and_fixtures() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let claude_md = "# Repository Outline\n\n- src/main.rs\n";

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
            Ok(claude_md.as_bytes().to_vec()),
        ]);

        let db_path = code_index_db_path_for_repo(executor.repo_path()).unwrap();
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "crates/goose/tests/session_id_propagation_test.rs",
                r#"[{"specifier":"goose::conversation::message::Message","names":["Message"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "examples/demo/main.rs",
                r#"[{"specifier":"crate::app","names":["run"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "src/backend/service.rs",
                r#"[{"specifier":"crate::core::engine","names":["Engine"]},{"specifier":"./fixtures/sample","names":["SAMPLE"]},{"specifier":"./test_support/helpers","names":["helper"]},{"specifier":"./e2e/flow","names":["flow"]}]"#
            ],
        )
        .unwrap();

        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };
        let payload = JCodemunchSidecar::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(!architecture_map.contains("/tests/"));
        assert!(!architecture_map.contains("tests.rs"));
        assert!(!architecture_map.contains("/examples/"));
        assert!(!architecture_map.contains("fixtures"));
        assert!(!architecture_map.contains("test_support"));
        assert!(!architecture_map.contains("e2e"));
        assert!(architecture_map.contains("src/backend/service.rs -> crate::core::engine (Engine)"));
    }

    #[tokio::test]
    async fn test_architecture_map_skips_scenarios_docs_ui_and_bench_noise() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let claude_md = "# Repository Outline\n\n- src/main.rs\n";

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
            Ok(claude_md.as_bytes().to_vec()),
        ]);

        let db_path = code_index_db_path_for_repo(executor.repo_path()).unwrap();
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "crates/goose-cli/src/scenario_tests/message_generator.rs",
                r#"[{"specifier":"crate::scenario_tests::scenario_runner","names":["MessageGenerator"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "documentation/src/pages/index.tsx",
                r#"[{"specifier":"../components/GooseLogo","names":["GooseLogo"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "ui/desktop/src/App.tsx",
                r#"[{"specifier":"./contexts/ChatContext","names":["ChatProvider"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "oidc-proxy/test/index.test.js",
                r#"[{"specifier":"../src/index.js","names":["worker"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "evals/open-model-gym/suite/src/runner.ts",
                r#"[{"specifier":"./types.js","names":["Scenario"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "crates/goose/benches/parser.rs",
                r#"[{"specifier":"goose::parser","names":["parse"]}]"#
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (path, imports) VALUES (?1, ?2)",
            params![
                "src/backend/engine.rs",
                r#"[{"specifier":"crate::core::runtime","names":["Runtime"]}]"#
            ],
        )
        .unwrap();

        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };
        let payload = JCodemunchSidecar::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(!architecture_map.contains("scenario_tests"));
        assert!(!architecture_map.contains("documentation/"));
        assert!(!architecture_map.contains("/ui/"));
        assert!(!architecture_map.contains(".test.js"));
        assert!(!architecture_map.contains("/evals/"));
        assert!(!architecture_map.contains("/benches/"));
        assert!(architecture_map.contains("src/backend/engine.rs -> crate::core::runtime (Runtime)"));
    }

    #[tokio::test]
    async fn test_binary_not_found() {
        // Simula erro de comando não encontrado
        let spawn_err = SandboxError::ProcessSpawnFailed {
            reason: "program not found (os error 2)".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::BinaryNotFound {
                binary: "jcodemunch-mcp".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_execution_failed() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 2,
            stderr: "fatal error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(vec![Err(run_err)]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "exit code 2: fatal error".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_timeout_propagation() {
        let executor = MockExecutor::new(vec![Err(SandboxError::Timeout)]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 45,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::Timeout { timeout_secs: 45 })
        );
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let index_json = r#"{"success": true}"#;
        let corrup_bytes = b"{invalid_json_here".to_vec();
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(corrup_bytes),
        ]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        match result {
            Err(SidecarError::ParseError { reason }) => {
                assert!(reason.contains("key must be a string") || reason.contains("expected"));
            }
            other => panic!("Esperava ParseError, obteve: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_repo_payload_fails_closed() {
        let index_json = r#"{"success": true}"#;
        let empty_json = r#"{}"#;
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(empty_json.as_bytes().to_vec()),
        ]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch-mcp digest returned an empty health payload".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_empty_stdout_fails_closed() {
        let index_json = r#"{"success": true}"#;
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(Vec::new()),
        ]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch-mcp digest returned empty stdout".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_exit_code_1_fails_for_jcodemunch() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "usage error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(vec![Err(run_err)]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "exit code 1: usage error".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_claude_md_empty_stdout_fails_closed() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
            Ok(Vec::new()),
        ]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch-mcp claude-md returned empty stdout".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_oxc_extract_success() {
        let valid_json = r#"{
            "components": [
                {
                    "name": "Button",
                    "file_path": "src/components/Button.tsx",
                    "framework": "react",
                    "props": [
                        {
                            "name": "disabled",
                            "prop_type": "boolean",
                            "has_default": true,
                            "required": false
                        }
                    ],
                    "events": ["click"],
                    "is_default_export": true
                }
            ],
            "files_analyzed": 5
        }"#;

        let executor = MockExecutor::new(vec![Ok(valid_json.as_bytes().to_vec())]);
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert!(result.is_ok(), "Extração do OXC deveria ter sucesso: {:?}", result);
        let payload = result.unwrap();
        assert_eq!(payload.files_analyzed, 5);
        assert_eq!(payload.components.len(), 1);
        assert_eq!(payload.components[0].name, "Button");
        assert_eq!(payload.components[0].framework, "react");
        assert_eq!(payload.components[0].props.len(), 1);
        assert_eq!(payload.components[0].props[0].name, "disabled");
        assert_eq!(payload.components[0].props[0].prop_type, "boolean");
        assert!(!payload.components[0].props[0].required);
        assert_eq!(payload.components[0].events.len(), 1);
        assert_eq!(payload.components[0].events[0], "click");
        assert!(payload.components[0].is_default_export);
    }

    #[tokio::test]
    async fn test_oxc_binary_not_found() {
        let spawn_err = SandboxError::ProcessSpawnFailed {
            reason: "program not found (os error 2)".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::BinaryNotFound {
                binary: "oxlint".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_oxc_execution_failed() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 2,
            stderr: "oxlint crashed".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(vec![Err(run_err)]);
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "exit code 2: oxlint crashed".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_oxc_timeout_propagation() {
        let executor = MockExecutor::new(vec![Err(SandboxError::Timeout)]);
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 45,
        };

        let result = OxcSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::Timeout { timeout_secs: 45 })
        );
    }

    #[tokio::test]
    async fn test_oxc_invalid_json() {
        let corrup_bytes = b"{invalid_json".to_vec();
        let executor = MockExecutor::new(vec![Ok(corrup_bytes)]);
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        match result {
            Err(SidecarError::ParseError { reason }) => {
                assert!(reason.contains("expected value") || reason.contains("key must be a string") || reason.contains("expected"));
            }
            other => panic!("Esperava ParseError, obteve: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_oxc_empty_repo_valid_json() {
        let empty_json = r#"{
            "components": [],
            "files_analyzed": 0
        }"#;

        let executor = MockExecutor::new(vec![Ok(empty_json.as_bytes().to_vec())]);
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert_eq!(payload.files_analyzed, 0);
        assert!(payload.components.is_empty());
    }

    #[tokio::test]
    async fn test_native_test_discovery_uses_static_ast_for_rust() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        std::fs::create_dir_all(dir.path().join("mock")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            "#[cfg(test)]\nmod tests {\n    #[tokio::test]\n    async fn test_domain_logic_stays() {}\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("docs/test_docs.rs"),
            "#[test]\nfn test_docs_should_not_enter_blob() {}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mock/test_mock.rs"),
            "#[test]\nfn test_mock_should_be_ignored() {}\n",
        )
        .unwrap();
        let profile = StackProfile::Rust;

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &profile,
        })
        .await
        .unwrap();

        assert_eq!(payload.runner_name, "static-ast");
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "src/lib.rs"
                && block.items.contains(&"async fn test_domain_logic_stays".to_string())));
        assert!(!payload.blocks.iter().any(|block| block.file_path.contains("docs")));
        assert!(!payload.blocks.iter().any(|block| block.file_path.contains("mock")));
    }

    #[tokio::test]
    async fn test_native_test_discovery_preserves_all_items_per_file() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let mut content = String::new();
        for index in 0..8 {
            content.push_str(&format!("#[test]\nfn test_case_{index}() {{}}\n"));
        }
        std::fs::write(dir.path().join("src/lib.rs"), content).unwrap();

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &StackProfile::Rust,
        })
        .await
        .unwrap();

        assert_eq!(payload.blocks.len(), 1);
        assert_eq!(payload.blocks[0].file_path, "src/lib.rs");
        assert_eq!(payload.blocks[0].items.len(), 8);
        assert_eq!(payload.blocks[0].omitted_count, 0);
        assert!(payload.blocks[0].items.contains(&"fn test_case_7".to_string()));
    }

    #[tokio::test]
    async fn test_static_analysis_success_exit_1() {
        let valid_json = r#"{
            "violations": [
                {
                    "rule_id": "rule_1",
                    "severity": "error",
                    "message": "msg 1",
                    "file_path": "src/main.rs",
                    "line": 10,
                    "column": 5
                }
            ],
            "files_analyzed": 1,
            "linter_name": "ruff"
        }"#;

        // Simula exit code 1 via variante estruturada
        let err_exit_1 = SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "issues found".to_string(),
            stdout: valid_json.as_bytes().to_vec(),
        };
        let executor = MockExecutor::new(vec![Err(err_exit_1)]);
        let input = StaticAnalysisInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = StaticAnalysisSidecar::extract(input, "ruff", &["check"]).await;
        assert!(result.is_ok(), "Exit code 1 deveria ser sucesso para análise estática: {:?}", result);
        let payload = result.unwrap();
        assert_eq!(payload.violations.len(), 1);
        assert_eq!(payload.violations[0].rule_id, "rule_1");
    }

    #[tokio::test]
    async fn test_static_analysis_success_exit_0() {
        let empty_json = r#"{
            "violations": [],
            "files_analyzed": 10,
            "linter_name": "ruff"
        }"#;

        let executor = MockExecutor::new(vec![Ok(empty_json.as_bytes().to_vec())]);
        let input = StaticAnalysisInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = StaticAnalysisSidecar::extract(input, "ruff", &["check"]).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert!(payload.violations.is_empty());
        assert_eq!(payload.files_analyzed, 10);
    }

    #[tokio::test]
    async fn test_static_analysis_execution_failed_exit_2() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 2,
            stderr: "config error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(vec![Err(run_err)]);
        let input = StaticAnalysisInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = StaticAnalysisSidecar::extract(input, "ruff", &["check"]).await;
        assert!(matches!(result, Err(SidecarError::ExecutionFailed { .. })));
    }

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
}
