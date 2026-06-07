use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rusqlite::params;
use thiserror::Error;
use serde::Deserialize;
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

fn jcodemunch_exit_code_one_is_retryable_success(args: &[&str], stdout_bytes: &[u8]) -> bool {
    let Some(subcommand) = args.first().copied() else {
        return false;
    };
    if stdout_is_blank(stdout_bytes) {
        return false;
    }

    match subcommand {
        "index" => {
            let Ok(index_json) = serde_json::from_slice::<serde_json::Value>(stdout_bytes) else {
                return false;
            };
            matches!(index_json.get("success").and_then(|value| value.as_bool()), Some(true))
        }
        "digest" => stdout_contains_json_payload(stdout_bytes),
        _ => false,
    }
}

fn stdout_preview(bytes: &[u8], max_chars: usize) -> String {
    if stdout_is_blank(bytes) {
        return String::new();
    }
    let text = String::from_utf8_lossy(bytes);
    truncate_chars(&text.replace(['\r', '\n'], " "), max_chars)
}

fn fallback_jcodemunch_repo_outline(repo_path: &Path, reason: &str) -> Vec<u8> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(repo_path) {
        for item in read_dir.take(80) {
            let Ok(item) = item else { continue };
            let name = item
                .file_name()
                .to_str()
                .unwrap_or_default()
                .trim()
                .to_string();
            if !name.is_empty() {
                entries.push(name);
            }
        }
    }
    entries.sort();
    let mut out = String::new();
    out.push_str("# Repository Outline\n\n");
    out.push_str("Fallback: ");
    out.push_str(reason.trim());
    out.push_str("\n\n## Root Entries\n");
    if entries.is_empty() {
        out.push_str("- <vazio>\n");
    } else {
        for entry in entries {
            out.push_str("- ");
            out.push_str(&entry);
            out.push('\n');
        }
    }
    out.into_bytes()
}

fn fallback_jcodemunch_health_report(reason: &str) -> Vec<u8> {
    serde_json::to_string(&serde_json::json!({
        "fallback": true,
        "source": "jcodemunch-mcp",
        "reason": truncate_chars(reason.trim(), 800),
    }))
    .unwrap_or_else(|_| "{\"fallback\":true}".to_string())
    .into_bytes()
}

fn fallback_jcodemunch_architecture_map(reason: &str) -> Vec<u8> {
    format!(
        "# Architecture Map\n\nFallback: {}\n",
        truncate_chars(reason.trim(), 800)
    )
    .into_bytes()
}

fn is_no_source_files_found(reason: &str) -> bool {
    let lower = reason.to_ascii_lowercase();
    lower.contains("no source files found")
        || lower.contains("no source files were found")
        || lower.contains("no source file found")
        || lower.contains("no source files")
        || lower.contains("no indexable source")
        || lower.contains("no supported source")
}

fn extract_urls_from_text(text: &str, max_urls: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    let bytes = text.as_bytes();
    while idx < bytes.len() && out.len() < max_urls {
        let rest = &text[idx..];
        let Some(rel_pos) = rest.find("http") else {
            break;
        };
        idx = idx.saturating_add(rel_pos);
        let candidate = &text[idx..];
        let end = candidate
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '"' | '\'' | '<' | '>'))
            .unwrap_or(candidate.len());
        let mut url = candidate[..end].trim().trim_end_matches(['.', ',', ';', ':']).to_string();
        if url.starts_with("http://") || url.starts_with("https://") {
            if url.len() > 2048 {
                url.truncate(2048);
            }
            out.push(url);
        }
        idx = idx.saturating_add(end.max(1));
    }
    out
}

fn extract_github_repo_ids(urls: &[String], max_repos: usize) -> Vec<String> {
    let mut out = BTreeSet::<String>::new();
    for url in urls {
        if out.len() >= max_repos {
            break;
        }
        let lower = url.to_ascii_lowercase();
        let marker = "github.com/";
        let Some(pos) = lower.find(marker) else {
            continue;
        };
        let mut rest = url[(pos + marker.len())..].to_string();
        if let Some(hash) = rest.find('#') {
            rest.truncate(hash);
        }
        if let Some(q) = rest.find('?') {
            rest.truncate(q);
        }
        rest = rest.trim_end_matches('/').trim_end_matches(".git").to_string();
        let mut parts = rest.split('/').map(|p| p.trim()).filter(|p| !p.is_empty());
        let Some(owner) = parts.next() else { continue };
        let Some(repo) = parts.next() else { continue };
        if owner.eq_ignore_ascii_case("topics")
            || owner.eq_ignore_ascii_case("search")
            || owner.eq_ignore_ascii_case("orgs")
            || owner.eq_ignore_ascii_case("users")
        {
            continue;
        }
        out.insert(format!("{owner}/{repo}"));
    }
    out.into_iter().take(max_repos).collect()
}

fn collect_markdown_files(repo_path: &Path, max_files: usize) -> Vec<PathBuf> {
    fn should_skip_dir(name: &str) -> bool {
        matches!(name, ".git" | "node_modules" | "target" | "vendor" | ".jj" | ".svn")
    }

    let mut out = Vec::new();
    let mut stack = vec![repo_path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= max_files {
            break;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.take(512) {
            if out.len() >= max_files {
                break;
            }
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if should_skip_dir(name) {
                        continue;
                    }
                }
                stack.push(path);
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());
            if matches!(ext.as_deref(), Some("md" | "markdown" | "mdx")) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn content_repo_artifacts(repo_path: &Path, why: &str) -> JCodemunchArtifacts {
    let md_files = collect_markdown_files(repo_path, 24);
    let mut blocks: Vec<(i32, ScopedTextBlock)> = Vec::new();
    let mut all_text = String::new();
    let mut skill_signal = false;
    for path in &md_files {
        let rel = sanitize_repo_relative_path(repo_path, &path.to_string_lossy())
            .unwrap_or_else(|| path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string());
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut score = 0i32;
        if !skill_signal {
            let rel_l = rel.to_ascii_lowercase();
            let c_l = content.to_ascii_lowercase();
            if rel_l.contains("skill")
                || rel_l.contains("prompt")
                || c_l.contains("skills for ai")
                || c_l.contains("coding agents")
                || c_l.contains("diagram")
                || c_l.contains("visualization")
            {
                skill_signal = true;
            }
        }
        {
            let rel_l = rel.to_ascii_lowercase();
            if rel_l.contains("readme") {
                score += 5;
            }
            if rel_l.contains("skill") || rel_l.contains("prompt") {
                score += 3;
            }
            let c_l = content.to_ascii_lowercase();
            if c_l.contains("problems_and_diagnostics") {
                score += 10;
            }
        }
        all_text.push_str(&content);
        all_text.push('\n');
        let snippet = truncate_chars(&content, 6000);
        blocks.push((
            score,
            ScopedTextBlock {
                file_path: rel,
                items: vec![snippet],
                omitted_count: 0,
            },
        ));
    }

    blocks.sort_by(|(score_l, block_l), (score_r, block_r)| {
        score_r.cmp(score_l).then_with(|| block_l.file_path.cmp(&block_r.file_path))
    });
    let packed_blocks = blocks.iter().map(|(_, b)| b.clone()).collect::<Vec<_>>();
    let packed = pack_scoped_text_blocks(&packed_blocks, BLOB_04_REPO_OUTLINE_MAX_CHARS);
    let kind = if skill_signal { "SkillLibrary" } else { "ContentRepo" };
    let mut outline = String::new();
    outline.push_str("# Repository Outline\n\n");
    outline.push_str("kind: ");
    outline.push_str(kind);
    outline.push('\n');
    outline.push_str("note: Repositório sem arquivos de código indexáveis (curadoria/documentação/skills).\n");
    outline.push_str("why: ");
    outline.push_str(truncate_chars(why.trim(), 600).trim());
    outline.push_str("\n\n");
    if packed.trim().is_empty() {
        outline.push_str("Sem markdown legível encontrado.\n");
    } else {
        outline.push_str("## Markdown Extract (amostra)\n\n");
        outline.push_str(&packed);
    }

    let urls = extract_urls_from_text(&all_text, 600);
    let gh_repos = extract_github_repo_ids(&urls, 250);
    let mut external = BTreeSet::<String>::new();
    for url in urls {
        if url.to_ascii_lowercase().contains("github.com/") {
            continue;
        }
        external.insert(url);
    }

    let mut link_map = String::new();
    link_map.push_str("# Link Map\n\n");
    link_map.push_str("kind: ");
    link_map.push_str(kind);
    link_map.push('\n');
    link_map.push_str(&format!("markdown_files: {}\n", md_files.len()));
    link_map.push_str(&format!("github_repo_links: {}\n", gh_repos.len()));
    link_map.push_str(&format!("external_links: {}\n\n", external.len()));
    link_map.push_str("## GitHub Repos\n");
    if gh_repos.is_empty() {
        link_map.push_str("- <nenhum>\n");
    } else {
        for repo in &gh_repos {
            link_map.push_str("- ");
            link_map.push_str(repo);
            link_map.push('\n');
        }
    }
    link_map.push_str("\n## External URLs\n");
    if external.is_empty() {
        link_map.push_str("- <nenhum>\n");
    } else {
        for url in external.iter().take(200) {
            link_map.push_str("- ");
            link_map.push_str(url);
            link_map.push('\n');
        }
    }

    let health = serde_json::to_string(&serde_json::json!({
        "kind": kind,
        "why": truncate_chars(why.trim(), 800),
        "markdown_files": md_files.len(),
        "github_repo_links": gh_repos.len(),
        "external_links": external.len(),
        "skill_signal": skill_signal,
    }))
    .unwrap_or_else(|_| "{\"kind\":\"ContentRepo\"}".to_string());

    JCodemunchArtifacts {
        repo_outline_blob: truncate_chars(&outline, BLOB_04_REPO_OUTLINE_MAX_CHARS).into_bytes(),
        architecture_map_blob: truncate_chars(&link_map, BLOB_05_ARCHITECTURE_MAP_MAX_CHARS).into_bytes(),
        health_report_blob: truncate_chars(&health, BLOB_08_HEALTH_REPORT_MAX_CHARS).into_bytes(),
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
    let canonical = storage_path.join(format!("{}-{}.db", owner, repo));
    if canonical.is_file() {
        return Ok(canonical);
    }

    let mut candidates = std::fs::read_dir(&storage_path)
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!(
                "Falha ao listar diretório do índice do jcodemunch '{}': {}",
                storage_path.display(),
                e
            ),
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("db"))
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let repo_lower = repo.to_ascii_lowercase();
    if let Some(path) = candidates.iter().find(|path| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_ascii_lowercase().contains(&repo_lower))
            .unwrap_or(false)
    }) {
        return Ok(path.clone());
    }

    if let [single] = candidates.as_slice() {
        return Ok(single.clone());
    }

    Err(SidecarError::ExecutionFailed {
        reason: format!(
            "Nao foi possivel localizar o banco SQLite do jcodemunch em '{}'; esperado='{}', candidatos={:?}",
            storage_path.display(),
            canonical.display(),
            candidates
        ),
    })
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

fn sanitize_host_paths_in_text(repo_path: &Path, text: &str) -> String {
    let mut sanitized = text.to_string();
    let repo_prefix = repo_path.to_string_lossy().to_string();
    sanitized = replace_host_prefix_variants(sanitized, &repo_prefix, "");

    if let Ok(semgrep_root) = semgrep_support_dir(repo_path) {
        sanitized = replace_host_prefix_variants(
            sanitized,
            &semgrep_root.to_string_lossy(),
            ".soda_semgrep/",
        );
    }

    sanitized = replace_host_prefix_variants(
        sanitized,
        &code_index_path_for_repo(repo_path),
        ".jcodemunch_index/",
    );

    sanitized
}

fn sanitize_sidecar_output(repo_path: &Path, bytes: &[u8]) -> Vec<u8> {
    sanitize_host_paths_in_text(repo_path, &String::from_utf8_lossy(bytes)).into_bytes()
}

fn sanitize_repo_relative_path(repo_path: &Path, value: &str) -> Option<String> {
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
        || lower.starts_with(".jcodemunch_index/")
        || lower.starts_with(".soda_scratchpad/")
        || lower.starts_with("sandbox/")
        || lower.starts_with("diagnostics/")
        || lower.contains(".souls_workspaces");
    if host_drive || internal {
        return None;
    }

    Some(normalized)
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
    super::normalize_repo_path_key(value)
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
    super::normalized_path_has_any_segment(&normalized, NON_CORE_PATH_SEGMENTS)
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
    tracing::info!(
        repo_path = %repo_path.display(),
        db_path = %db_path.display(),
        "jcodemunch: abrindo banco topologico para blob_05_architecture_map"
    );

    let summary = {
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
            let Some(path) = sanitize_repo_relative_path(repo_path, &path) else {
                continue;
            };
            if should_skip_topology_entry(&path) {
                continue;
            }
            let imports: Vec<IndexedImport> = serde_json::from_str(&imports_json).map_err(|e| SidecarError::ParseError {
                reason: e.to_string(),
            })?;
            let mut relevant = imports
                .into_iter()
                .filter_map(|import| {
                    let specifier = sanitize_host_paths_in_text(repo_path, &import.specifier);
                    if !is_project_specifier(&specifier, &project_prefixes)
                        || should_skip_topology_entry(&specifier)
                    {
                        return None;
                    }

                    if import.names.is_empty() {
                        Some(specifier)
                    } else {
                        Some(format!("{} ({})", specifier, import.names.join(", ")))
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
    }?;

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
    tracing::info!(
        binary = %binary,
        args = ?args,
        repo_path = %executor.repo_path().display(),
        timeout_secs,
        "Invocando sidecar"
    );
    match executor.execute(binary, args, timeout_secs).await {
        Ok(bytes) => {
            let sanitized_bytes = sanitize_sidecar_output(executor.repo_path(), &bytes);
            tracing::info!(
                binary = %binary,
                stdout_bytes = sanitized_bytes.len(),
                repo_path = %executor.repo_path().display(),
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
        // Match numérico explícito no exit code — sem manipulação de string.
        // Exit code 1: linters sinalizam violações encontradas (sucesso de negócio).
        // Exit code 2+: erro real de execução (config inválida, crash).
        Err(SandboxError::ProcessNonZeroExit { exit_code, stderr, stdout }) => {
            let sanitized_stdout = sanitize_sidecar_output(executor.repo_path(), &stdout);
            let sanitized_stderr = sanitize_host_paths_in_text(executor.repo_path(), &stderr);
            if (binary == "semgrep" && !stdout_is_blank(&sanitized_stdout) && stdout_contains_json_payload(&sanitized_stdout))
                || (binary == "jcodemunch-mcp"
                    && exit_code == 1
                    && jcodemunch_exit_code_one_is_retryable_success(args, &sanitized_stdout))
                || (exit_code == 1 && matches!(exit_policy, SidecarExitPolicy::AllowFindingsExitOne))
            {
                if binary == "jcodemunch-mcp" && exit_code == 1 {
                    tracing::warn!(
                        binary = %binary,
                        args = ?args,
                        "Sidecar retornou exit code 1, mas stdout indica sucesso; tolerando"
                    );
                }
                Ok(sanitized_stdout)
            } else {
                let stdout_hint = stdout_preview(&sanitized_stdout, 400);
                error!(
                    binary = %binary,
                    exit_code,
                    stderr = %sanitized_stderr,
                    stdout = %stdout_hint,
                    "Sidecar terminou com exit code nao zero"
                );
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

pub struct JCodemunchSidecar;

impl JCodemunchSidecar {
    /// Extrai o health report e o repo outline usando o jcodemunch no sandbox.
    pub async fn extract<E: SandboxExecutor>(
        input: JCodemunchInput<'_, E>,
    ) -> Result<JCodemunchArtifacts, SidecarError> {
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            "jcodemunch: iniciando indexacao e digest"
        );
        let storage_path = code_index_path_for_repo(input.executor.repo_path());
        if let Err(e) = tokio::fs::create_dir_all(&storage_path).await {
            tracing::warn!(
                repo_path = %input.executor.repo_path().display(),
                error = %e,
                "jcodemunch: falha ao preparar diretório de índice; seguindo"
            );
        }
        let repo_path_arg = if input.executor.repo_path().is_absolute() {
            input.executor.repo_path().display().to_string()
        } else {
            std::fs::canonicalize(input.executor.repo_path())
                .unwrap_or_else(|_| input.executor.repo_path().to_path_buf())
                .display()
                .to_string()
        };
        let index_args = vec![
            "index".to_string(),
            repo_path_arg,
            "--no-ai-summaries".to_string(),
            "--extra-ignore".to_string(),
            "documentation".to_string(),
            "docs".to_string(),
            "examples".to_string(),
            "evals".to_string(),
            "vendor".to_string(),
            "node_modules".to_string(),
            "target".to_string(),
            ".git".to_string(),
        ];
        let index_arg_refs: Vec<&str> = index_args.iter().map(String::as_str).collect();
        let _index_bytes = match execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &index_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await
        {
            Ok(bytes) => {
                if let Err(e) = validate_index_response(&bytes) {
                    let reason = format!("index inválido: {}", e);
                    if is_no_source_files_found(&reason) {
                        return Ok(content_repo_artifacts(input.executor.repo_path(), &reason));
                    }
                    tracing::warn!(
                        repo_path = %input.executor.repo_path().display(),
                        error = %reason,
                        "jcodemunch: index falhou; aplicando fallback (fail-soft)"
                    );
                    return Ok(JCodemunchArtifacts {
                        repo_outline_blob: fallback_jcodemunch_repo_outline(input.executor.repo_path(), &reason),
                        health_report_blob: fallback_jcodemunch_health_report(&reason),
                        architecture_map_blob: fallback_jcodemunch_architecture_map(&reason),
                    });
                }
                bytes
            }
            Err(e) => {
                let reason = e.to_string();
                if is_no_source_files_found(&reason) {
                    return Ok(content_repo_artifacts(input.executor.repo_path(), &reason));
                }
                tracing::warn!(
                    repo_path = %input.executor.repo_path().display(),
                    error = %reason,
                    "jcodemunch: index falhou; aplicando fallback (fail-soft)"
                );
                return Ok(JCodemunchArtifacts {
                    repo_outline_blob: fallback_jcodemunch_repo_outline(input.executor.repo_path(), &reason),
                    health_report_blob: fallback_jcodemunch_health_report(&reason),
                    architecture_map_blob: fallback_jcodemunch_architecture_map(&reason),
                });
            }
        };

        let digest_args: Vec<&str> = vec![
            "digest",
            "--json",
            "--storage-path",
            &storage_path,
        ];
        let digest_arg_refs: Vec<&str> = digest_args;
        let bytes = match execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &digest_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await
        {
            Ok(bytes) => bytes,
            Err(e) => {
                let reason = format!("digest falhou: {}", e);
                tracing::warn!(
                    repo_path = %input.executor.repo_path().display(),
                    error = %reason,
                    "jcodemunch: digest falhou; seguindo com fallback do health report"
                );
                Vec::new()
            }
        };
        let health_report_blob = match normalize_health_report(&bytes) {
            Ok(blob) => blob,
            Err(e) => fallback_jcodemunch_health_report(&format!("health report inválido: {}", e)),
        };

        let claude_md_args: Vec<&str> = ["claude-md", "--generate"].to_vec();
        let claude_md_arg_refs: Vec<&str> = claude_md_args;
        let claude_md_bytes = match execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &claude_md_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await
        {
            Ok(bytes) => bytes,
            Err(e) => {
                let reason = format!("claude-md falhou: {}", e);
                tracing::warn!(
                    repo_path = %input.executor.repo_path().display(),
                    error = %reason,
                    "jcodemunch: repo outline indisponível; usando fallback"
                );
                Vec::new()
            }
        };
        let repo_outline_blob = match normalize_repo_outline(&claude_md_bytes) {
            Ok(blob) => blob,
            Err(e) => fallback_jcodemunch_repo_outline(
                input.executor.repo_path(),
                &format!("repo outline inválido: {}", e),
            ),
        };
        let architecture_map_blob = match normalize_architecture_map(input.executor.repo_path()) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!(
                    repo_path = %input.executor.repo_path().display(),
                    error = %e,
                    "jcodemunch: blob_05_architecture_map falhou; aplicando fallback (fail-soft)"
                );
                fallback_jcodemunch_architecture_map(&e.to_string())
            }
        };
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            repo_outline_bytes = repo_outline_blob.len(),
            architecture_map_bytes = architecture_map_blob.len(),
            health_report_bytes = health_report_blob.len(),
            "jcodemunch: artefatos normalizados"
        );

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
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            "Invocando sidecar oxc/oxlint para contratos UX"
        );
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
    pub timed_out: bool,
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
const STATIC_TEST_DISCOVERY_READ_BYTES: usize = 50 * 1024;
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
        .split(is_semgrep_path_separator)
        .filter(|part| !part.is_empty())
        .any(|part| UNIVERSAL_TEST_SKIP_SEGMENTS.contains(&part))
}

#[allow(clippy::manual_pattern_char_comparison)]
fn is_semgrep_path_separator(ch: char) -> bool {
    matches!(ch, '/' | ':' | '>' | ' ')
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
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao abrir '{}': {}", path.display(), e),
    })?;
    let mut buf = Vec::new();
    let _ = (&mut file)
        .take(STATIC_TEST_DISCOVERY_READ_BYTES as u64)
        .read_to_end(&mut buf)
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao ler primeiros {} bytes de '{}': {}", STATIC_TEST_DISCOVERY_READ_BYTES, path.display(), e),
        })?;
    match String::from_utf8(buf) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
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

fn extract_python_test_entries_shallow(content: &str) -> Vec<String> {
    let mut entries = BTreeSet::new();
    for line in content.lines().take(2_000) {
        let trimmed = line.trim();
        let signature = if trimmed.starts_with("async def test_") || trimmed.starts_with("def test_") {
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

fn extract_rust_test_entries_shallow(content: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    let mut saw_test_attr = false;
    for line in content.lines().take(2_000) {
        let trimmed = line.trim();
        if trimmed.starts_with("#[test]") || trimmed.starts_with("#[rstest]") {
            saw_test_attr = true;
            continue;
        }

        let is_fn_line = trimmed.starts_with("fn ") || trimmed.starts_with("async fn ");
        if is_fn_line {
            let is_test_name = trimmed.contains(" fn test_") || trimmed.starts_with("fn test_") || trimmed.starts_with("async fn test_");
            if saw_test_attr || is_test_name {
                saw_test_attr = false;
                let normalized = trimmed
                    .trim_end_matches('{')
                    .trim()
                    .trim_end_matches(';')
                    .trim();
                if let Some(signature) = compact_signature_text(normalized) {
                    out.insert(signature);
                }
                continue;
            }
            saw_test_attr = false;
        } else if !trimmed.starts_with("#[") && !trimmed.is_empty() {
            saw_test_attr = false;
        }
    }
    out.into_iter().collect()
}

fn extract_frontend_test_entries_shallow(content: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    for line in content.lines().take(2_000) {
        let trimmed = line.trim();
        if !(trimmed.contains("describe(") || trimmed.contains("it(") || trimmed.contains("test(")) {
            continue;
        }
        if let Some(signature) = compact_signature_text(trimmed) {
            out.insert(signature);
        }
    }
    out.into_iter().collect()
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

#[derive(Debug, Clone, Default)]
struct TestDiscoveryProgress {
    blocks: Vec<ScopedTextBlock>,
    bfs_dirs: Vec<String>,
    bfs_test_files: Vec<String>,
}

fn discover_static_test_entries_bfs(
    repo_path: &Path,
    profile: &StackProfile,
    progress: Option<&Arc<Mutex<TestDiscoveryProgress>>>,
) -> Result<Vec<ScopedTextBlock>, SidecarError> {
    const MAX_SNAPSHOT_DIRS: usize = 64;
    const MAX_SNAPSHOT_FILES: usize = 96;
    const PROGRESS_FLUSH_EVERY: usize = 32;

    let mut blocks = Vec::new();
    let mut bfs_dirs = Vec::new();
    let mut bfs_test_files = Vec::new();

    let mut queue = VecDeque::new();
    queue.push_back(repo_path.to_path_buf());

    let mut pending_blocks = Vec::new();
    let mut steps = 0usize;

    while let Some(dir) = queue.pop_front() {
        let rel_dir = relative_display(repo_path, &dir);
        let rel_dir = if rel_dir.is_empty() { ".".to_string() } else { rel_dir };
        if bfs_dirs.len() < MAX_SNAPSHOT_DIRS && !should_skip_discovered_test_entry(&rel_dir) {
            bfs_dirs.push(rel_dir);
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_e) => {
                steps += 1;
                if steps.is_multiple_of(PROGRESS_FLUSH_EVERY) {
                    if let Some(progress) = progress {
                        if let Ok(mut guard) = progress.lock() {
                            if guard.bfs_dirs.len() < MAX_SNAPSHOT_DIRS {
                                for item in &bfs_dirs {
                                    if guard.bfs_dirs.len() >= MAX_SNAPSHOT_DIRS {
                                        break;
                                    }
                                    if !guard.bfs_dirs.contains(item) {
                                        guard.bfs_dirs.push(item.clone());
                                    }
                                }
                            }
                            if guard.bfs_test_files.len() < MAX_SNAPSHOT_FILES {
                                for item in &bfs_test_files {
                                    if guard.bfs_test_files.len() >= MAX_SNAPSHOT_FILES {
                                        break;
                                    }
                                    if !guard.bfs_test_files.contains(item) {
                                        guard.bfs_test_files.push(item.clone());
                                    }
                                }
                            }
                            if !pending_blocks.is_empty() {
                                guard.blocks.append(&mut pending_blocks);
                            }
                        }
                    }
                }
                continue;
            }
        };

        for entry in entries {
            let entry = entry.map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao iterar '{}': {}", dir.display(), e),
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
                queue.push_back(path);
                continue;
            }

            if !file_type.is_file() || !is_supported_test_file(profile, &path) {
                continue;
            }

            let relative = relative_display(repo_path, &path);
            if should_skip_discovered_test_entry(&relative) {
                continue;
            }

            if bfs_test_files.len() < MAX_SNAPSHOT_FILES {
                bfs_test_files.push(relative.clone());
            }

            let Some(content) = read_static_test_file(&path)? else {
                continue;
            };
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase());

            let discovered = match extension.as_deref() {
                Some("rs") => extract_rust_test_entries_shallow(&content),
                Some("py") => extract_python_test_entries_shallow(&content),
                Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts") => {
                    extract_frontend_test_entries_shallow(&content)
                }
                _ => Vec::new(),
            };

            let mut items = Vec::new();
            for entry in discovered {
                let candidate = format!("{} :: {}", relative, entry);
                if !should_skip_discovered_test_entry(&candidate) {
                    items.push(entry);
                }
            }

            if !items.is_empty() {
                let block = ScopedTextBlock {
                    file_path: relative,
                    items,
                    omitted_count: 0,
                };
                blocks.push(block.clone());
                pending_blocks.push(block);
            }

            steps += 1;
            if steps.is_multiple_of(PROGRESS_FLUSH_EVERY) {
                if let Some(progress) = progress {
                    if let Ok(mut guard) = progress.lock() {
                        if guard.bfs_dirs.len() < MAX_SNAPSHOT_DIRS {
                            for item in &bfs_dirs {
                                if guard.bfs_dirs.len() >= MAX_SNAPSHOT_DIRS {
                                    break;
                                }
                                if !guard.bfs_dirs.contains(item) {
                                    guard.bfs_dirs.push(item.clone());
                                }
                            }
                        }
                        if guard.bfs_test_files.len() < MAX_SNAPSHOT_FILES {
                            for item in &bfs_test_files {
                                if guard.bfs_test_files.len() >= MAX_SNAPSHOT_FILES {
                                    break;
                                }
                                if !guard.bfs_test_files.contains(item) {
                                    guard.bfs_test_files.push(item.clone());
                                }
                            }
                        }
                        if !pending_blocks.is_empty() {
                            guard.blocks.append(&mut pending_blocks);
                        }
                    }
                }
            }
        }
    }

    if let Some(progress) = progress {
        if let Ok(mut guard) = progress.lock() {
            if guard.bfs_dirs.is_empty() {
                guard.bfs_dirs = bfs_dirs;
            }
            if guard.bfs_test_files.is_empty() {
                guard.bfs_test_files = bfs_test_files;
            }
            if !pending_blocks.is_empty() {
                guard.blocks.extend(pending_blocks);
            }
        }
    }

    Ok(blocks)
}

pub struct NativeTestDiscoverySidecar;

impl NativeTestDiscoverySidecar {
    pub async fn extract(input: NativeTestDiscoveryInput<'_>) -> Result<TestIntentPayload, SidecarError> {
        let repo_path = input.repo_path.to_path_buf();
        let profile = input.profile.clone();
        let progress = Arc::new(Mutex::new(TestDiscoveryProgress::default()));
        let progress_ref = progress.clone();

        let mut handle = tokio::task::spawn_blocking(move || {
            discover_static_test_entries_bfs(&repo_path, &profile, Some(&progress_ref))
        });

        match tokio::time::timeout(Duration::from_secs(60), &mut handle).await {
            Ok(joined) => {
                let blocks = joined
                    .map_err(|e| SidecarError::ExecutionFailed {
                        reason: format!("Static test discovery join failed: {}", e),
                    })??;
                Ok(TestIntentPayload {
                    runner_name: "static-ast-bfs".to_string(),
                    timed_out: false,
                    blocks,
                })
            }
            Err(_) => {
                handle.abort();
                let snapshot = progress.lock().unwrap().clone();
                let mut blocks = Vec::new();
                if !snapshot.bfs_dirs.is_empty() {
                    blocks.push(ScopedTextBlock {
                        file_path: "bfs_snapshot::dirs".to_string(),
                        items: snapshot.bfs_dirs,
                        omitted_count: 0,
                    });
                }
                if !snapshot.bfs_test_files.is_empty() {
                    blocks.push(ScopedTextBlock {
                        file_path: "bfs_snapshot::test_files".to_string(),
                        items: snapshot.bfs_test_files,
                        omitted_count: 0,
                    });
                }
                blocks.extend(snapshot.blocks);
                Ok(TestIntentPayload {
                    runner_name: "static-ast-bfs".to_string(),
                    timed_out: true,
                    blocks,
                })
            }
        }
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

fn normalize_semgrep_path(repo_path: &Path, value: &str) -> Option<String> {
    sanitize_repo_relative_path(repo_path, value)
}

fn normalize_semgrep_check_id(repo_path: &Path, value: &str) -> String {
    let sanitized = sanitize_host_paths_in_text(repo_path, value)
        .replace(['\\', '/'], ".")
        .trim_matches('.')
        .to_string();
    sanitized
        .find("soda.")
        .map(|index| sanitized[index..].to_string())
        .unwrap_or(sanitized)
}

fn semgrep_item_label(repo_path: &Path, result: &SemgrepJsonResult) -> String {
    let severity = result.extra.severity.as_deref().unwrap_or("INFO");
    let check_id = normalize_semgrep_check_id(repo_path, &result.check_id);
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
    let message = sanitize_host_paths_in_text(repo_path, &message);

    match category {
        Some(category) => format!(
            "[{}] {}:L{} {} - {}",
            severity,
            check_id,
            result.start.line,
            category,
            message
        ),
        None => format!(
            "[{}] {}:L{} {}",
            severity,
            check_id,
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
        .filter_map(|result| {
            normalize_semgrep_path(repo_path, &result.path).map(|path| {
                (
                    path,
                    semgrep_item_label(repo_path, &result),
                )
            })
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

fn semgrep_support_dir(repo_path: &Path) -> Result<PathBuf, SidecarError> {
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

async fn ensure_semgrep_rule_file(repo_path: &Path, rule_set: SemgrepRuleSet) -> Result<PathBuf, SidecarError> {
    let support_dir = semgrep_support_dir(repo_path)?;
    tokio::fs::create_dir_all(&support_dir)
        .await
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao preparar diretório auxiliar do semgrep '{}': {}", support_dir.display(), e),
        })?;
    let path = support_dir.join(rule_set.rule_file_name());
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
    tracing::info!(
        repo_path = %executor.repo_path().display(),
        rule_set = ?rule_set,
        rule_path = %rule_path.display(),
        "Semgrep: iniciando scan"
    );
    let rule_arg = rule_path.to_string_lossy().to_string();
    let args = [
        "scan",
        "--config",
        rule_arg.as_str(),
        "--json",
        "--jobs",
        "1",
        "--disable-version-check",
        "--metrics",
        "off",
        "--exclude",
        rule_arg.as_str(),
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

    #[test]
    fn test_code_index_db_path_accepts_local_repo_index_name() {
        let temp_dir = TempDir::new().unwrap();
        let owner_dir = temp_dir.path().join("aaif-goose");
        let repo_path = owner_dir.join("goose");
        let index_dir = owner_dir.join(".jcodemunch_index");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::fs::create_dir_all(&index_dir).unwrap();

        let expected = index_dir.join("local-goose-0a8be5b6.db");
        std::fs::write(&expected, b"").unwrap();

        let resolved = code_index_db_path_for_repo(&repo_path).unwrap();
        assert_eq!(resolved, expected);
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
                "web/panel.tsx",
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

        let backend_pos = architecture_map.find("[src/backend/service.rs]").unwrap();
        let ui_pos = architecture_map.find("[web/panel.tsx]").unwrap();
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
        assert!(architecture_map.contains("[src/backend/service.rs]"));
        assert!(architecture_map.contains("- crate::core::engine (Engine)"));
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
        assert!(architecture_map.contains("[src/backend/engine.rs]"));
        assert!(architecture_map.contains("- crate::core::runtime (Runtime)"));
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        assert!(outline.contains("Fallback:"), "Outline deveria conter fallback");
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        assert!(outline.contains("Fallback:"), "Outline deveria conter fallback");
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_jcodemunch_index_exit_code_1_with_success_json_is_allowed() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let claude_md = "# Repository Outline\n\n- src/main.rs\n";
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "".to_string(),
            stdout: index_json.as_bytes().to_vec(),
        };
        let executor = MockExecutor::new(vec![
            Err(run_err),
            Ok(digest_json.as_bytes().to_vec()),
            Ok(claude_md.as_bytes().to_vec()),
        ]);
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert!(result.is_ok(), "Extração deveria tolerar exit 1 no index: {:?}", result);
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
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
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
}
