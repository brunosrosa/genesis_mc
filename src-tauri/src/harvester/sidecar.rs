use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ignore::WalkBuilder;
use quick_xml::de::from_str as xml_from_str;
use regex::Regex;
use rusqlite::params;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use crate::harvester::PHASE1_HEAVY_BLOB_MAX_CHARS;
use crate::harvester::ast_parser::{self, AstParserError};
use crate::harvester::detect::{SingleStack, StackProfile};
use crate::harvester::router::{route_static_analysis_blades, StaticAnalysisBlade};
use crate::harvester::sandbox::{SandboxError, truncated_args_preview};
use crate::harvester::web_scraper;

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

/// Implementação da trait SandboxExecutor para o SandboxHandle concreto.
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

pub struct NativeAstInput<'a, E: SandboxExecutor> {
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

fn stdout_preview(bytes: &[u8], max_chars: usize) -> String {
    if stdout_is_blank(bytes) {
        return String::new();
    }
    let text = String::from_utf8_lossy(bytes);
    truncate_chars(&text.replace(['\r', '\n'], " "), max_chars)
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

fn score_external_doc_url(url: &str) -> i32 {
    let lower = url.to_ascii_lowercase();
    let mut score = 0i32;
    for (needle, weight) in [
        ("codewiki", 30),
        ("deepwiki", 24),
        ("readthedocs", 20),
        ("docs.", 18),
        ("/docs", 18),
        ("documentation", 16),
        ("/wiki", 16),
        ("guide", 12),
        ("manual", 12),
        ("reference", 10),
        ("api", 8),
        ("tutorial", 8),
    ] {
        if lower.contains(needle) {
            score += weight;
        }
    }
    score
}

fn prioritized_external_doc_urls(urls: &BTreeSet<String>, max_urls: usize) -> Vec<String> {
    let mut ranked = urls
        .iter()
        .map(|url| (score_external_doc_url(url), url.clone()))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|(score_l, url_l), (score_r, url_r)| {
        score_r.cmp(score_l).then_with(|| url_l.cmp(url_r))
    });
    ranked
        .into_iter()
        .map(|(_, url)| url)
        .take(max_urls)
        .collect()
}

fn remote_block_label(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|parsed| {
            let host = parsed.host_str().unwrap_or("remote");
            let path = parsed.path().trim_matches('/');
            if path.is_empty() {
                format!("remote::{host}")
            } else {
                format!("remote::{host}/{}", truncate_chars(path, 80))
            }
        })
        .unwrap_or_else(|| format!("remote::{}", truncate_chars(url, 96)))
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

async fn content_repo_artifacts(repo_path: &Path, why: &str) -> Result<NativeAstArtifacts, SidecarError> {
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
    let prioritized_remote_docs = prioritized_external_doc_urls(&external, 4);
    let mut remote_blocks = Vec::<ScopedTextBlock>::new();
    let mut remote_fetch_failures = Vec::<String>::new();
    for url in &prioritized_remote_docs {
        match web_scraper::fetch_markdown_with_guarantee(url).await {
            Ok(markdown) => {
                if markdown.trim().is_empty() {
                    remote_fetch_failures.push(format!("{url} => markdown vazio"));
                } else {
                    remote_blocks.push(ScopedTextBlock {
                        file_path: remote_block_label(url),
                        items: vec![truncate_chars(&markdown, 6000)],
                        omitted_count: 0,
                    });
                }
            }
            Err(err) => {
                remote_fetch_failures.push(format!(
                    "{} => {}",
                    url,
                    truncate_chars(&err.to_string(), 400)
                ));
            }
        }
    }
    if !prioritized_remote_docs.is_empty() && remote_blocks.is_empty() {
        return Err(SidecarError::ExecutionFailed {
            reason: format!(
                "Falha ao garantir scraping remoto para documentação externa. urls={} falhas={}",
                prioritized_remote_docs.join(", "),
                remote_fetch_failures.join(" | ")
            ),
        });
    }

    let mut link_map = String::new();
    link_map.push_str("# Link Map\n\n");
    link_map.push_str("kind: ");
    link_map.push_str(kind);
    link_map.push('\n');
    link_map.push_str(&format!("markdown_files: {}\n", md_files.len()));
    link_map.push_str(&format!("github_repo_links: {}\n", gh_repos.len()));
    link_map.push_str(&format!("external_links: {}\n", external.len()));
    link_map.push_str(&format!("remote_doc_candidates: {}\n", prioritized_remote_docs.len()));
    link_map.push_str(&format!("remote_doc_fetched: {}\n", remote_blocks.len()));
    link_map.push_str(&format!("remote_doc_failed: {}\n\n", remote_fetch_failures.len()));
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
    if !prioritized_remote_docs.is_empty() {
        link_map.push_str("\n## Remote Docs Scraped\n");
        if remote_blocks.is_empty() {
            link_map.push_str("- <nenhum>\n");
        } else {
            for url in &prioritized_remote_docs {
                link_map.push_str("- ");
                link_map.push_str(url);
                link_map.push('\n');
            }
        }
    }
    if !remote_fetch_failures.is_empty() {
        link_map.push_str("\n## Remote Docs Failures\n");
        for failure in &remote_fetch_failures {
            link_map.push_str("- ");
            link_map.push_str(failure);
            link_map.push('\n');
        }
    }

    let health = serde_json::to_string(&serde_json::json!({
        "kind": kind,
        "why": truncate_chars(why.trim(), 800),
        "markdown_files": md_files.len(),
        "github_repo_links": gh_repos.len(),
        "external_links": external.len(),
        "remote_doc_candidates": prioritized_remote_docs.len(),
        "remote_doc_fetched": remote_blocks.len(),
        "remote_doc_failed": remote_fetch_failures.len(),
        "skill_signal": skill_signal,
    }))
    .unwrap_or_else(|_| "{\"kind\":\"ContentRepo\"}".to_string());

    if !remote_blocks.is_empty() {
        outline.push_str("\n\n## Remote Documentation (Guaranteed)\n\n");
        outline.push_str(&pack_scoped_text_blocks(
            &remote_blocks,
            BLOB_04_REPO_OUTLINE_MAX_CHARS / 2,
        ));
    }

    Ok(NativeAstArtifacts {
        repo_outline_blob: truncate_chars(&outline, BLOB_04_REPO_OUTLINE_MAX_CHARS).into_bytes(),
        architecture_map_blob: truncate_chars(&link_map, BLOB_05_ARCHITECTURE_MAP_MAX_CHARS).into_bytes(),
        health_report_blob: truncate_chars(&health, BLOB_08_HEALTH_REPORT_MAX_CHARS).into_bytes(),
    })
}

fn stdout_contains_json_payload(bytes: &[u8]) -> bool {
    extract_json_payload(bytes).is_some()
}

fn is_sobelow_mix_invocation<S: AsRef<str>>(binary: &str, args: &[S]) -> bool {
    binary == "mix" && args.first().map(|arg| arg.as_ref()) == Some("sobelow")
}

fn extract_cppcheck_xml_payload(text: &str) -> Option<&str> {
    extract_xml_payload(text.as_bytes())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
}

fn extract_json_payload(bytes: &[u8]) -> Option<&[u8]> {
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

fn extract_xml_payload(bytes: &[u8]) -> Option<&[u8]> {
    let first_candidate = bytes.iter().position(|byte| *byte == b'<')?;
    let text = std::str::from_utf8(bytes).ok()?;

    if let Some(index) = text.find("<?xml").or_else(|| text.find("<results")) {
        return Some(&bytes[index..]);
    }

    Some(&bytes[first_candidate..])
}

fn parse_json_payload<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SidecarError> {
    let payload = extract_json_payload(bytes).ok_or_else(|| SidecarError::ParseError {
        reason: "Falha ao localizar payload JSON no stdout do sidecar".to_string(),
    })?;
    serde_json::from_slice::<T>(payload).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })
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
pub struct NativeAstArtifacts {
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
    fn config_dir_name(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::Health => "health",
        }
    }

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

fn native_ast_cache_path_for_repo(repo_path: &Path) -> String {
    repo_path
        .parent()
        .unwrap_or(repo_path)
        .join(".native_ast_cache")
        .display()
        .to_string()
}

#[cfg(test)]
fn native_ast_cache_global_storage_dir() -> Option<PathBuf> {
    if let Ok(configured) = env::var("JCODEMUNCH_STORAGE_PATH") {
        let trimmed = configured.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    let home = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".code-index"))
}

#[cfg(test)]
fn native_ast_cache_db_path_for_repo(repo_path: &Path) -> Result<std::path::PathBuf, SidecarError> {
    native_ast_cache_db_path_for_repo_id(repo_path, None)
}

#[cfg(test)]
fn native_ast_cache_db_path_for_repo_id(
    repo_path: &Path,
    index_repo_id: Option<&str>,
) -> Result<std::path::PathBuf, SidecarError> {
    let owner = repo_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: "Nao foi possivel resolver o owner do repositório para localizar o cache AST nativo".to_string(),
        })?;
    let repo = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: "Nao foi possivel resolver o nome do repositório para localizar o cache AST nativo".to_string(),
        })?;
    let mut roots = vec![repo_path.parent().unwrap_or(repo_path).join(".native_ast_cache")];
    if let Some(global_root) = native_ast_cache_global_storage_dir() {
        roots.push(global_root);
    }

    let mut exact_stems = Vec::new();
    if let Some(repo_id) = index_repo_id {
        let sanitized = repo_id
            .trim()
            .replace(['\\', '/'], "-")
            .replace(':', "-");
        if !sanitized.is_empty() {
            exact_stems.push(sanitized);
        }
    }
    exact_stems.push(format!("{}-{}", owner, repo));

    let mut all_candidates = Vec::new();
    for root in roots {
        for stem in &exact_stems {
            let candidate = root.join(format!("{stem}.db"));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        if let Ok(entries) = std::fs::read_dir(&root) {
            let mut candidates = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("db"))
                .collect::<Vec<_>>();
            candidates.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            all_candidates.extend(candidates);
        }
    }

    let repo_lower = repo.to_ascii_lowercase();
    if let Some(path) = all_candidates.iter().find(|path| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_ascii_lowercase().contains(&repo_lower))
            .unwrap_or(false)
    }) {
        return Ok(path.clone());
    }

    if let [single] = all_candidates.as_slice() {
        return Ok(single.clone());
    }

    Err(SidecarError::ExecutionFailed {
        reason: format!(
            "Nao foi possivel localizar o cache SQLite do AST nativo para '{}'; repo_id={:?}; candidatos={:?}",
            repo_path.display(),
            index_repo_id,
            all_candidates
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
        &native_ast_cache_path_for_repo(repo_path),
        ".native_ast_cache/",
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn normalize_repo_outline(bytes: &[u8]) -> Result<Vec<u8>, SidecarError> {
    if stdout_is_blank(bytes) {
        tracing::debug!(binary = "native-ast-parser", "Sidecar claude-md retornou stdout vazio");
        return Err(SidecarError::ExecutionFailed {
            reason: "native-ast-parser claude-md returned empty stdout".to_string(),
        });
    }

    let text = String::from_utf8_lossy(bytes);
    let normalized = normalize_repo_outline_markdown(&text);
    let truncated = truncate_chars(&normalized, BLOB_04_REPO_OUTLINE_MAX_CHARS);
    if truncated.trim().is_empty() {
        return Err(SidecarError::ExecutionFailed {
            reason: "native-ast-parser claude-md returned an empty repo outline".to_string(),
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

async fn execute_sidecar_in_dir<E: SandboxExecutor>(
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
        // Match numérico explícito no exit code — sem manipulação de string.
        // Exit code 1: linters sinalizam violações encontradas (sucesso de negócio).
        // Exit code 2+: erro real de execução (config inválida, crash).
        Err(SandboxError::ProcessNonZeroExit { exit_code, stderr, stdout }) => {
            let sanitized_stdout = sanitize_sidecar_output(executor.repo_path(), &stdout);
            let sanitized_stderr = sanitize_host_paths_in_text(executor.repo_path(), &stderr);
            if ((binary == "semgrep" || binary == "opengrep")
                && !stdout_is_blank(&sanitized_stdout)
                && stdout_contains_json_payload(&sanitized_stdout))
                || (exit_code == 1 && matches!(exit_policy, SidecarExitPolicy::AllowFindingsExitOne))
            {
                if binary == "cppcheck" {
                    if let Some(xml_payload) = extract_cppcheck_xml_payload(&sanitized_stderr) {
                        Ok(xml_payload.as_bytes().to_vec())
                    } else if let Some(xml_payload) =
                        extract_cppcheck_xml_payload(&String::from_utf8_lossy(&sanitized_stdout))
                    {
                        Ok(xml_payload.as_bytes().to_vec())
                    } else {
                        Ok(sanitized_stdout)
                    }
                } else if is_sobelow_mix_invocation(binary, args)
                    && stdout_is_blank(&sanitized_stdout)
                    && sanitized_stderr.contains("total_findings:")
                {
                    Ok(sanitized_stderr.into_bytes())
                } else {
                    Ok(sanitized_stdout)
                }
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

pub struct NativeAstParser;

impl NativeAstParser {
    /// Extrai os artefatos estruturais de código usando parser AST nativo em Rust.
    pub async fn extract<E: SandboxExecutor>(
        input: NativeAstInput<'_, E>,
    ) -> Result<NativeAstArtifacts, SidecarError> {
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            "ast-native: iniciando extração estrutural"
        );
        let repo_path = input.executor.repo_path().to_path_buf();
        let native_artifacts = tokio::task::spawn_blocking(move || {
            ast_parser::extract_repository_outline_native(
                &repo_path,
                BLOB_04_REPO_OUTLINE_MAX_CHARS,
                BLOB_05_ARCHITECTURE_MAP_MAX_CHARS,
                BLOB_08_HEALTH_REPORT_MAX_CHARS,
            )
        })
        .await
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao aguardar parser AST nativo: {}", e),
        })?;
        let native_artifacts = match native_artifacts {
            Ok(artifacts) => artifacts,
            Err(AstParserError::EmptyRepository { path }) => {
                return content_repo_artifacts(
                    input.executor.repo_path(),
                    &format!("no source files found in {}", path),
                )
                .await;
            }
            Err(other) => {
                return Err(SidecarError::ExecutionFailed {
                    reason: other.to_string(),
                });
            }
        };

        let repo_outline_blob = native_artifacts.repo_outline_blob;
        let health_report_blob = native_artifacts.health_report_blob;
        let architecture_map_blob = native_artifacts.architecture_map_blob;
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            repo_outline_bytes = repo_outline_blob.len(),
            architecture_map_bytes = architecture_map_blob.len(),
            health_report_bytes = health_report_blob.len(),
            "ast-native: artefatos normalizados"
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

        Ok(NativeAstArtifacts {
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
        tracing::debug!(
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
        parse_json_payload::<UxContractsPayload>(&bytes)
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
        StackProfile::CCpp => Some(SingleStack::CCpp),
        StackProfile::Elixir => Some(SingleStack::Elixir),
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
            | ".native_ast_cache"
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

fn normalize_rust_test_signature(signature: &str) -> Option<String> {
    let head = signature.split('{').next().unwrap_or(signature).trim();
    let compact = compact_signature_text(head)?;
    let compact = compact.strip_suffix("()").unwrap_or(&compact).trim().to_string();
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
                let normalized = trimmed.trim().trim_end_matches(';').trim();
                if let Some(signature) = normalize_rust_test_signature(normalized) {
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
        parse_json_payload::<StaticAnalysisPayload>(&bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SodaHealthIssue {
    pub level: String,
    pub file: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SodaHealthToolResult {
    blade: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    status: String,
    issue_count: usize,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SodaHealthReport {
    schema: String,
    focus: String,
    router: Vec<String>,
    tool_results: Vec<SodaHealthToolResult>,
    issues: Vec<SodaHealthIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyglotSastArtifacts {
    pub unsafe_hotspots_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
}

pub struct PolyglotSastInput<'a, E: SandboxExecutor> {
    pub executor: Arc<E>,
    pub timeout_secs: u64,
    pub profile: &'a StackProfile,
}

const MONOREPO_SAST_MAX_PARALLEL: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ManifestKind {
    CargoToml,
    PackageJson,
    MixExs,
    GoMod,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DiscoveredManifest {
    kind: ManifestKind,
    manifest_path: PathBuf,
    execution_root: PathBuf,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SastExecutionTarget {
    blade: StaticAnalysisBlade,
    execution_root: PathBuf,
    scope: String,
}

#[derive(Debug)]
struct SastExecutionOutcome {
    blade: StaticAnalysisBlade,
    execution_root: PathBuf,
    scope: String,
    result: Result<Vec<u8>, SidecarError>,
}

#[derive(Debug, Deserialize)]
struct CppcheckResults {
    #[serde(default)]
    errors: Option<CppcheckErrors>,
}

#[derive(Debug, Deserialize)]
struct CppcheckErrors {
    #[serde(rename = "error", default)]
    items: Vec<CppcheckError>,
}

#[derive(Debug, Deserialize)]
struct CppcheckError {
    #[serde(rename = "@id", default)]
    id: Option<String>,
    #[serde(rename = "@severity", default)]
    severity: Option<String>,
    #[serde(rename = "@msg", default)]
    msg: Option<String>,
    #[serde(rename = "location", default)]
    locations: Vec<CppcheckLocation>,
}

#[derive(Debug, Deserialize)]
struct CppcheckLocation {
    #[serde(rename = "@file", default)]
    file: Option<String>,
    #[serde(rename = "@line", default)]
    line: Option<u32>,
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

fn manifest_kind_for_blade(blade: StaticAnalysisBlade) -> Option<ManifestKind> {
    match blade {
        StaticAnalysisBlade::RustClippy => Some(ManifestKind::CargoToml),
        StaticAnalysisBlade::Biome | StaticAnalysisBlade::Oxc => Some(ManifestKind::PackageJson),
        StaticAnalysisBlade::Sobelow => Some(ManifestKind::MixExs),
        StaticAnalysisBlade::Govulncheck => Some(ManifestKind::GoMod),
        _ => None,
    }
}

fn execution_targets_for_blade(
    repo_path: &Path,
    manifests: &[DiscoveredManifest],
    blade: StaticAnalysisBlade,
) -> Vec<SastExecutionTarget> {
    if blade == StaticAnalysisBlade::Opengrep {
        return derive_opengrep_execution_targets(repo_path);
    }

    if let Some(kind) = manifest_kind_for_blade(blade) {
        return manifests
            .iter()
            .filter(|manifest| manifest.kind == kind)
            .map(|manifest| SastExecutionTarget {
                blade,
                execution_root: manifest.execution_root.clone(),
                scope: manifest.scope.clone(),
            })
            .collect();
    }

    vec![SastExecutionTarget {
        blade,
        execution_root: repo_path.to_path_buf(),
        scope: ".".to_string(),
    }]
}

const OPENGREP_DYNAMIC_ROOT_LIMIT: usize = 24;

fn derive_opengrep_execution_targets(repo_path: &Path) -> Vec<SastExecutionTarget> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    match ast_parser::derive_scannable_roots_native(&repo_root, OPENGREP_DYNAMIC_ROOT_LIMIT) {
        Ok(roots) if !roots.is_empty() => {
            let mut targets = roots
                .into_iter()
                .filter(|root| root.exists())
                .map(|execution_root| {
                    let scope = execution_root
                        .strip_prefix(&repo_root)
                        .ok()
                        .map(|value| value.to_string_lossy().replace('\\', "/"))
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| ".".to_string());
                    SastExecutionTarget {
                        blade: StaticAnalysisBlade::Opengrep,
                        execution_root,
                        scope,
                    }
                })
                .collect::<Vec<_>>();
            targets.sort_by(|left, right| {
                left.scope
                    .cmp(&right.scope)
                    .then_with(|| left.execution_root.cmp(&right.execution_root))
            });
            targets.dedup_by(|left, right| left.execution_root == right.execution_root);
            targets
        }
        Ok(_) | Err(_) => vec![SastExecutionTarget {
            blade: StaticAnalysisBlade::Opengrep,
            execution_root: repo_root,
            scope: ".".to_string(),
        }],
    }
}

fn scoped_blade_label(blade: StaticAnalysisBlade, scope: &str) -> String {
    if scope == "." {
        blade_name(blade).to_string()
    } else {
        format!("{}@{scope}", blade_name(blade))
    }
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

fn push_issue(
    issues: &mut Vec<SodaHealthIssue>,
    repo_path: &Path,
    execution_root: &Path,
    level: &str,
    file: &str,
    message: &str,
) {
    let message = sanitize_issue_message(repo_path, message);
    if message.trim().is_empty() {
        return;
    }
    issues.push(SodaHealthIssue {
        level: sanitize_issue_level(level),
        file: sanitize_issue_file(repo_path, execution_root, file),
        message,
    });
}

fn sort_and_dedup_issues(issues: &mut Vec<SodaHealthIssue>) {
    issues.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then_with(|| left.level.cmp(&right.level))
            .then_with(|| left.message.cmp(&right.message))
    });
    issues.dedup();
}

fn clippy_args() -> Vec<String> {
    vec![
        "clippy".to_string(),
        "--message-format=json".to_string(),
    ]
}

const SEMGREP_SCAN_EXCLUDES: &[&str] = &[
    "tests",
    "**/tests/**",
    "__tests__",
    "**/__tests__/**",
    "test",
    "**/test/**",
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
    "playground",
    "playgrounds",
    "**/playgrounds/**",
    "benchmark",
    "benchmarking",
    "**/benchmarking/**",
    "generated",
    "**/generated/**",
    "**/output.json",
    "**/*.generated.*",
    "**/*.min.js",
    "**/*.min.cjs",
    "**/*.min.mjs",
    "**/*.bundle.js",
    "test_support",
    "e2e",
    "docs",
    "documentation",
    "examples",
];

#[derive(Debug, Clone, Copy)]
struct SemgrepScanOptions {
    disable_version_check: bool,
    metrics_off: bool,
    taint_intrafile: bool,
    allow_rule_timeout_control: bool,
    exclude_minified_files: bool,
}

fn build_semgrep_like_scan_args(rule_arg: &str, options: SemgrepScanOptions) -> Vec<String> {
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

    for exclude in SEMGREP_SCAN_EXCLUDES {
        args.push("--exclude".to_string());
        args.push((*exclude).to_string());
    }

    args.push(".".to_string());
    args
}

fn cppcheck_args() -> Vec<String> {
    vec![
        "--xml".to_string(),
        "--xml-version=2".to_string(),
        "--enable=warning,style,performance,portability,information".to_string(),
        "--error-exitcode=1".to_string(),
        ".".to_string(),
    ]
}

fn sobelow_args() -> Vec<String> {
    vec![
        "sobelow".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--private".to_string(),
    ]
}

fn biome_args() -> Vec<String> {
    vec![
        "check".to_string(),
        "--reporter=json".to_string(),
        ".".to_string(),
    ]
}

fn oxc_args() -> Vec<String> {
    vec![
        "-f".to_string(),
        "json".to_string(),
        ".".to_string(),
    ]
}

fn ruff_args() -> Vec<String> {
    vec![
        "check".to_string(),
        ".".to_string(),
        "--output-format".to_string(),
        "json".to_string(),
    ]
}

fn bandit_args() -> Vec<String> {
    vec![
        "-r".to_string(),
        ".".to_string(),
        "-f".to_string(),
        "json".to_string(),
    ]
}

fn govulncheck_args() -> Vec<String> {
    vec![
        "-format".to_string(),
        "json".to_string(),
        "./...".to_string(),
    ]
}

fn opengrep_args(rule_arg: &str) -> Vec<String> {
    build_semgrep_like_scan_args(
        rule_arg,
        SemgrepScanOptions {
            disable_version_check: true,
            metrics_off: false,
            taint_intrafile: true,
            allow_rule_timeout_control: true,
            // Compat mode: o opengrep 1.23.0 no Windows anuncia esta flag no --help,
            // mas a rejeita em runtime durante `scan`.
            exclude_minified_files: false,
        },
    )
}

fn semgrep_args(rule_arg: &str) -> Vec<String> {
    build_semgrep_like_scan_args(
        rule_arg,
        SemgrepScanOptions {
            disable_version_check: true,
            metrics_off: true,
            taint_intrafile: false,
            allow_rule_timeout_control: true,
            exclude_minified_files: true,
        },
    )
}

async fn cleanup_clippy_target_dir(execution_root: &Path) {
    let target_dir = execution_root.join("target");
    if !target_dir.exists() {
        return;
    }

    match tokio::fs::remove_dir_all(&target_dir).await {
        Ok(_) => {
            info!(target_dir = %target_dir.display(), "clippy: target/ efemero removido");
        }
        Err(err) => {
            warn!(
                target_dir = %target_dir.display(),
                error = %err,
                "clippy: falha ao remover target/ efemero"
            );
        }
    }
}

async fn run_opengrep_scan<E: SandboxExecutor>(
    executor: &E,
    timeout_secs: u64,
    execution_root: &Path,
) -> Result<Vec<u8>, SidecarError> {
    let rule_path = ensure_semgrep_rule_bundle(executor.repo_path(), SemgrepRuleSet::Health).await?;
    let rule_arg = rule_path.to_string_lossy().to_string();
    let args = opengrep_args(&rule_arg);
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

fn blade_name(blade: StaticAnalysisBlade) -> &'static str {
    match blade {
        StaticAnalysisBlade::RustClippy => "rust-clippy",
        StaticAnalysisBlade::Cppcheck => "cppcheck",
        StaticAnalysisBlade::Sobelow => "sobelow",
        StaticAnalysisBlade::Biome => "biome",
        StaticAnalysisBlade::Oxc => "oxc",
        StaticAnalysisBlade::Ruff => "ruff",
        StaticAnalysisBlade::Bandit => "bandit",
        StaticAnalysisBlade::Govulncheck => "govulncheck",
        StaticAnalysisBlade::Opengrep => "opengrep",
    }
}

fn blade_command(blade: StaticAnalysisBlade) -> (&'static str, Vec<String>) {
    match blade {
        StaticAnalysisBlade::RustClippy => ("cargo", clippy_args()),
        StaticAnalysisBlade::Cppcheck => ("cppcheck", cppcheck_args()),
        StaticAnalysisBlade::Sobelow => ("mix", sobelow_args()),
        StaticAnalysisBlade::Biome => ("biome", biome_args()),
        StaticAnalysisBlade::Oxc => ("oxlint", oxc_args()),
        StaticAnalysisBlade::Ruff => ("ruff", ruff_args()),
        StaticAnalysisBlade::Bandit => ("bandit", bandit_args()),
        StaticAnalysisBlade::Govulncheck => ("govulncheck", govulncheck_args()),
        StaticAnalysisBlade::Opengrep => ("opengrep", Vec::new()),
    }
}

async fn run_sast_blade<E: SandboxExecutor>(
    executor: &E,
    blade: StaticAnalysisBlade,
    timeout_secs: u64,
    execution_root: &Path,
) -> Result<Vec<u8>, SidecarError> {
    if blade == StaticAnalysisBlade::Opengrep {
        return run_opengrep_scan(executor, timeout_secs, execution_root).await;
    }
    let (binary, args) = blade_command(blade);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let result = execute_sidecar_in_dir(
        executor,
        binary,
        &arg_refs,
        timeout_secs,
        SidecarExitPolicy::AllowFindingsExitOne,
        execution_root,
    )
    .await;
    if blade == StaticAnalysisBlade::RustClippy {
        cleanup_clippy_target_dir(execution_root).await;
    }
    result
}

fn normalize_clippy_output(
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
        push_issue(&mut issues, repo_path, execution_root, level, file, message);
    }
    sort_and_dedup_issues(&mut issues);
    Ok(issues)
}

fn normalize_cppcheck_output(
    repo_path: &Path,
    execution_root: &Path,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    let text = String::from_utf8_lossy(bytes);
    let xml_payload = extract_cppcheck_xml_payload(&text).ok_or_else(|| SidecarError::ParseError {
        reason: "Falha ao localizar payload XML do cppcheck".to_string(),
    })?;
    let parsed: CppcheckResults = xml_from_str(xml_payload).map_err(|err| SidecarError::ParseError {
        reason: format!("Falha ao parsear XML do cppcheck: {err}"),
    })?;
    let mut issues = Vec::new();
    for error in parsed.errors.map(|value| value.items).unwrap_or_default() {
        let file = error
            .locations
            .first()
            .and_then(|location| location.file.as_deref())
            .unwrap_or("");
        let line = error
            .locations
            .first()
            .and_then(|location| location.line)
            .map(|line| format!(" (line {line})"))
            .unwrap_or_default();
        let rule = error.id.unwrap_or_else(|| "cppcheck".to_string());
        let msg = error.msg.unwrap_or_else(|| "cppcheck finding".to_string());
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            error.severity.as_deref().unwrap_or("warning"),
            file,
            &format!("{rule}: {msg}{line}"),
        );
    }
    sort_and_dedup_issues(&mut issues);
    Ok(issues)
}

fn normalize_semgrep_like_json(
    repo_path: &Path,
    execution_root: &Path,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    let payload = parse_json_payload::<SemgrepJsonPayload>(bytes)?;
    let mut issues = Vec::new();
    for result in payload.results {
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            result.extra.severity.as_deref().unwrap_or("warning"),
            &result.path,
            &format!("{}: {}", result.check_id, result.extra.message),
        );
    }
    sort_and_dedup_issues(&mut issues);
    Ok(issues)
}

fn value_as_u32(value: Option<&serde_json::Value>) -> Option<u32> {
    value.and_then(|value| value.as_u64()).and_then(|value| u32::try_from(value).ok())
}

fn json_value_at_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn normalize_json_array_issues(
    repo_path: &Path,
    execution_root: &Path,
    items: &[serde_json::Value],
    file_keys: &[&str],
    level_keys: &[&str],
    message_keys: &[&str],
    line_keys: &[&str],
) -> Vec<SodaHealthIssue> {
    let mut issues = Vec::new();
    for item in items {
        let file = file_keys
            .iter()
            .find_map(|key| json_value_at_path(item, key).and_then(|value| value.as_str()))
            .unwrap_or("");
        let level = level_keys
            .iter()
            .find_map(|key| json_value_at_path(item, key).and_then(|value| value.as_str()))
            .unwrap_or("warning");
        let message = message_keys
            .iter()
            .find_map(|key| json_value_at_path(item, key).and_then(|value| value.as_str()))
            .unwrap_or("diagnostic");
        let line_suffix = line_keys
            .iter()
            .find_map(|key| value_as_u32(json_value_at_path(item, key)))
            .map(|line| format!(" (line {line})"))
            .unwrap_or_default();
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            level,
            file,
            &format!("{message}{line_suffix}"),
        );
    }
    sort_and_dedup_issues(&mut issues);
    issues
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
                normalize_sobelow_text_issues(repo_path, execution_root, &String::from_utf8_lossy(bytes));
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
                    items,
                    &["filename", "file"],
                    &["level", "severity"],
                    &["message"],
                    &["location.row", "line"],
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
                    items,
                    &["filename", "file"],
                    &["issue_severity", "severity"],
                    &["issue_text", "message"],
                    &["line_number", "line"],
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
                    items,
                    &["file", "path", "filename"],
                    &["severity", "level", "confidence"],
                    &["message", "description", "title", "type"],
                    &["line", "line_number"],
                )
            })
            .unwrap_or_default()
        }
        _ => Vec::new(),
    };
    Ok(issues)
}

fn normalize_sobelow_text_issues(
    repo_path: &Path,
    execution_root: &Path,
    text: &str,
) -> Vec<SodaHealthIssue> {
    let finding_re = Regex::new(
        r#"%\{file: "(?P<file>[^"]+)", line: (?P<line>\d+), type: "(?P<kind>[^"]+)"(?:, variable: (?P<variable>"[^"]+"|:[^,}]+|[A-Za-z_][A-Za-z0-9_]*))?\}"#,
    )
    .expect("regex sobelow invalida");

    let mut issues = Vec::new();
    for captures in finding_re.captures_iter(text) {
        let file = captures.name("file").map(|value| value.as_str()).unwrap_or("");
        let line = captures.name("line").map(|value| value.as_str()).unwrap_or("0");
        let kind = captures.name("kind").map(|value| value.as_str()).unwrap_or("Sobelow finding");
        let variable = captures
            .name("variable")
            .map(|value| value.as_str().trim_matches('"'))
            .unwrap_or("");
        let message = if variable.is_empty() {
            format!("{kind} (line {line})")
        } else {
            format!("{kind}: {variable} (line {line})")
        };
        push_issue(&mut issues, repo_path, execution_root, "warning", file, &message);
    }
    sort_and_dedup_issues(&mut issues);
    issues
}

fn normalize_govulncheck_output(
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
        let Some(finding) = value.get("finding") else {
            continue;
        };
        let osv = finding.get("osv").and_then(|value| value.as_str()).unwrap_or("govulncheck");
        let trace = finding
            .get("trace")
            .and_then(|value| value.as_array())
            .and_then(|trace| trace.first());
        let file = trace
            .and_then(|frame| frame.get("position"))
            .and_then(|position| position.get("filename"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let package = trace
            .and_then(|frame| frame.get("module"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let symbol = trace
            .and_then(|frame| frame.get("function"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let message = if package.is_empty() && symbol.is_empty() {
            format!("{osv}: vulnerabilidade detectada")
        } else {
            format!("{osv}: {}", format!("{package} {symbol}").trim())
        };
        push_issue(&mut issues, repo_path, execution_root, "error", file, &message);
    }
    sort_and_dedup_issues(&mut issues);
    Ok(issues)
}

fn normalize_sast_output(
    repo_path: &Path,
    execution_root: &Path,
    blade: StaticAnalysisBlade,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    match blade {
        StaticAnalysisBlade::RustClippy => normalize_clippy_output(repo_path, execution_root, bytes),
        StaticAnalysisBlade::Cppcheck => normalize_cppcheck_output(repo_path, execution_root, bytes),
        StaticAnalysisBlade::Opengrep => normalize_semgrep_like_json(repo_path, execution_root, bytes),
        StaticAnalysisBlade::Govulncheck => normalize_govulncheck_output(repo_path, execution_root, bytes),
        StaticAnalysisBlade::Ruff
        | StaticAnalysisBlade::Bandit
        | StaticAnalysisBlade::Biome
        | StaticAnalysisBlade::Oxc
        | StaticAnalysisBlade::Sobelow => {
            normalize_json_object_issues(repo_path, execution_root, blade, bytes)
        }
    }
}

fn build_tool_result(
    blade: StaticAnalysisBlade,
    scope: Option<String>,
    status: &str,
    issue_count: usize,
    error: Option<String>,
) -> SodaHealthToolResult {
    SodaHealthToolResult {
        blade: blade_name(blade).to_string(),
        scope,
        status: status.to_string(),
        issue_count,
        error,
    }
}

fn render_soda_health_report(
    focus: &str,
    blades: &[StaticAnalysisBlade],
    tool_results: Vec<SodaHealthToolResult>,
    issues: Vec<SodaHealthIssue>,
) -> Vec<u8> {
    let report = SodaHealthReport {
        schema: "soda.health.v1".to_string(),
        focus: focus.to_string(),
        router: blades.iter().map(|blade| blade_name(*blade).to_string()).collect(),
        tool_results,
        issues,
    };
    serde_json::to_vec_pretty(&report).unwrap_or_else(|_| {
        format!(
            r#"{{"schema":"soda.health.v1","focus":"{focus}","router":[],"tool_results":[],"issues":[]}}"#
        )
        .into_bytes()
    })
}

pub struct PolyglotSastSidecar;

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
        let mut tool_results = Vec::<SodaHealthToolResult>::new();
        let semaphore = Arc::new(Semaphore::new(MONOREPO_SAST_MAX_PARALLEL));
        let mut join_set = JoinSet::new();

        for blade in &blades {
            let targets = execution_targets_for_blade(&repo_path, &manifests, *blade);
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
                tool_results.push(build_tool_result(*blade, None, "skipped", 0, Some(reason)));
                continue;
            }

            for target in targets {
                let executor = Arc::clone(&input.executor);
                let semaphore = Arc::clone(&semaphore);
                let scope = target.scope.clone();
                let execution_root = target.execution_root.clone();
                join_set.spawn(async move {
                    let permit = Arc::clone(&semaphore)
                        .acquire_owned()
                        .await
                        .map_err(|e| SidecarError::ExecutionFailed {
                            reason: format!("falha ao adquirir permissão do semáforo SAST: {e}"),
                        })?;
                    info!(
                        blade = blade_name(target.blade),
                        scope = %scope,
                        cwd = %execution_root.display(),
                        concurrency_limit = MONOREPO_SAST_MAX_PARALLEL,
                        in_flight = MONOREPO_SAST_MAX_PARALLEL.saturating_sub(semaphore.available_permits()),
                        "SAST monorepo: permissão adquirida"
                    );
                    let result = run_sast_blade(
                        executor.as_ref(),
                        target.blade,
                        input.timeout_secs,
                        &execution_root,
                    )
                    .await;
                    drop(permit);
                    info!(
                        blade = blade_name(target.blade),
                        scope = %scope,
                        cwd = %execution_root.display(),
                        available_permits = semaphore.available_permits(),
                        "SAST monorepo: sub-scan concluído"
                    );
                    Ok::<SastExecutionOutcome, SidecarError>(SastExecutionOutcome {
                        blade: target.blade,
                        execution_root,
                        scope,
                        result,
                    })
                });
            }
        }

        while let Some(joined) = join_set.join_next().await {
            let outcome = match joined {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(err)) => {
                    push_issue(
                        &mut all_issues,
                        &repo_path,
                        &repo_path,
                        "warning",
                        "",
                        &format!("sast worker execution failed: {err}"),
                    );
                    continue;
                }
                Err(err) => {
                    push_issue(
                        &mut all_issues,
                        &repo_path,
                        &repo_path,
                        "warning",
                        "",
                        &format!("sast worker join failed: {err}"),
                    );
                    continue;
                }
            };

            let scoped_blade = scoped_blade_label(outcome.blade, &outcome.scope);
            match outcome.result {
                Ok(bytes) => match normalize_sast_output(
                    &repo_path,
                    &outcome.execution_root,
                    outcome.blade,
                    &bytes,
                ) {
                    Ok(mut issues) => {
                        let issue_count = issues.len();
                        all_issues.append(&mut issues);
                        tool_results.push(build_tool_result(
                            outcome.blade,
                            Some(outcome.scope),
                            "ok",
                            issue_count,
                            None,
                        ));
                    }
                    Err(err) => {
                        let reason = err.to_string();
                        push_issue(
                            &mut all_issues,
                            &repo_path,
                            &outcome.execution_root,
                            "warning",
                            "",
                            &format!("{scoped_blade} normalization failed: {reason}"),
                        );
                        tool_results.push(build_tool_result(
                            outcome.blade,
                            Some(outcome.scope),
                            "parse_error",
                            0,
                            Some(reason),
                        ));
                    }
                },
                Err(err) => {
                    let reason = err.to_string();
                    push_issue(
                        &mut all_issues,
                        &repo_path,
                        &outcome.execution_root,
                        "warning",
                        "",
                        &format!("{scoped_blade} execution failed: {reason}"),
                    );
                    tool_results.push(build_tool_result(
                        outcome.blade,
                        Some(outcome.scope),
                        "exec_error",
                        0,
                        Some(reason),
                    ));
                }
            }
        }

        sort_and_dedup_issues(&mut all_issues);
        tool_results.sort_by(|left, right| {
            left.blade
                .cmp(&right.blade)
                .then_with(|| left.scope.cmp(&right.scope))
                .then_with(|| left.status.cmp(&right.status))
        });
        let unsafe_issues = all_issues
            .iter()
            .filter(|issue| issue.level != "info")
            .cloned()
            .collect::<Vec<_>>();

        Ok(PolyglotSastArtifacts {
            unsafe_hotspots_blob: render_soda_health_report(
                "unsafe_hotspots",
                &blades,
                tool_results.clone(),
                unsafe_issues,
            ),
            health_report_blob: render_soda_health_report(
                "health_report",
                &blades,
                tool_results,
                all_issues,
            ),
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
    let payload = parse_json_payload::<SemgrepJsonPayload>(bytes)?;

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

fn workspace_semgrep_rules_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("semgrep")
        .join("rules")
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
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao preparar diretório de regra '{}': {}", parent.display(), e),
            })?;
        }
        std::fs::copy(&path, &target).map_err(|e| SidecarError::ExecutionFailed {
            reason: format!(
                "Falha ao copiar regra '{}' para '{}': {}",
                path.display(),
                target.display(),
                e
            ),
        })?;
        copied += 1;
    }

    Ok(copied)
}

async fn ensure_semgrep_rule_bundle(repo_path: &Path, rule_set: SemgrepRuleSet) -> Result<PathBuf, SidecarError> {
    let support_dir = semgrep_support_dir(repo_path)?.join(rule_set.config_dir_name());
    let _ = tokio::fs::remove_dir_all(&support_dir).await;
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
    if workspace_rules_dir.exists() {
        let copied = copy_semgrep_rule_tree(&workspace_rules_dir, &workspace_rules_dir, &support_dir)?;
        tracing::info!(
            repo_path = %repo_path.display(),
            rule_set = ?rule_set,
            copied_rule_files = copied,
            workspace_rules_dir = %workspace_rules_dir.display(),
            support_dir = %support_dir.display(),
            "Semgrep: ruleset air-gapped materializado"
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

            let index_dir = owner_dir.join(".native_ast_cache");
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

        fn write_repo_file(&self, relative_path: &str, contents: &str) {
            let path = self.repo_path.join(relative_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, contents).unwrap();
        }

    }

    #[test]
    fn test_code_index_db_path_accepts_local_repo_index_name() {
        let temp_dir = TempDir::new().unwrap();
        let owner_dir = temp_dir.path().join("aaif-goose");
        let repo_path = owner_dir.join("goose");
        let index_dir = owner_dir.join(".native_ast_cache");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::fs::create_dir_all(&index_dir).unwrap();

        let expected = index_dir.join("local-goose-0a8be5b6.db");
        std::fs::write(&expected, b"").unwrap();

        let resolved = native_ast_cache_db_path_for_repo(&repo_path).unwrap();
        assert_eq!(resolved, expected);
    }

    impl SandboxExecutor for MockExecutor {
        fn execute<'a>(
            &'a self,
            command: &'a str,
            args: &'a [&'a str],
            _timeout_secs: u64,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>> {
            Box::pin(async move {
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
            })
        }

        fn execute_in_dir<'a>(
            &'a self,
            command: &'a str,
            args: &'a [&'a str],
            _timeout_secs: u64,
            execution_root: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>> {
            Box::pin(async move {
                self.calls.lock().unwrap().push(format!(
                    "{} {} [cwd={}]",
                    command,
                    args.join(" "),
                    execution_root.display()
                ).trim().to_string());
                let mut guard = self.responses.lock().unwrap();
                guard.pop_front().unwrap_or_else(|| {
                    Err(SandboxError::ProcessSpawnFailed {
                        reason: "no mock response configured".to_string(),
                    })
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

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ter sucesso: {:?}", result);
        let payload = result.unwrap();
        assert_eq!(
            String::from_utf8(payload.health_report_blob).unwrap(),
            r#"{"hotspots":[{"complexity":12,"path":"src/main.rs"}]}"#
        );
        let repo_outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        assert!(repo_outline.contains("# Repository Outline"));
        assert!(repo_outline.contains("[src/lib.rs]"));
        assert!(repo_outline.contains("[src/main.rs]"));
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

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
        ]);

        let db_path = native_ast_cache_db_path_for_repo(executor.repo_path()).unwrap();
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

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
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

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
        ]);

        let db_path = native_ast_cache_db_path_for_repo(executor.repo_path()).unwrap();
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

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
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

        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
        ]);

        let db_path = native_ast_cache_db_path_for_repo(executor.repo_path()).unwrap();
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

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
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
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
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
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        assert!(outline.contains("Fallback:"), "Outline deveria conter fallback");
    }

    #[tokio::test]
    async fn test_timeout_propagation() {
        let executor = MockExecutor::new(vec![Err(SandboxError::Timeout)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 45,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
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
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
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
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_empty_stdout_fails_closed() {
        let index_json = r#"{"success": true}"#;
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(Vec::new()),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_exit_code_1_fails_soft_for_native_ast_parser() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "usage error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(vec![Err(run_err)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_native_ast_cache_exit_code_1_with_success_json_is_allowed() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "".to_string(),
            stdout: index_json.as_bytes().to_vec(),
        };
        let executor = MockExecutor::new(vec![
            Err(run_err),
            Ok(digest_json.as_bytes().to_vec()),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
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
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifacts: None,
        };

        let result = NativeAstParser::extract(input).await;
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

        assert_eq!(payload.runner_name, "static-ast-bfs");
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

    #[test]
    fn test_normalize_clippy_messages_to_soda_health_issue() {
        let repo_path = Path::new(r"C:\host\projfs\owner\repo");
        let payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"manual memcpy can be replaced with copy_from_slice","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}
{"reason":"compiler-message","message":{"level":"error","message":"called `Result::unwrap()` on an `Err` value","spans":[{"file_name":"src\\main.rs","is_primary":true}]}}"#;

        let normalized = normalize_sast_output(
            repo_path,
            repo_path,
            StaticAnalysisBlade::RustClippy,
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

    #[tokio::test]
    async fn test_polyglot_sast_sidecar_routes_rust_and_cpp_and_emits_soda_json() {
        let clippy_payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"manual memcpy can be replaced with copy_from_slice","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}"#;
        let cppcheck_payload = r#"<results><errors><error id="memleak" severity="warning" msg="Memory leak: ptr"><location file="native/bridge.cpp" line="42"/></error></errors></results>"#;
        let opengrep_payload = r#"{"results":[{"check_id":"soda.tech-debt.todo-fixme","path":"README.md","extra":{"message":"Marcador de divida tecnica encontrado","severity":"INFO"}}]}"#;

        let executor = Arc::new(MockExecutor::new(vec![
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 1,
                stderr: "findings".to_string(),
                stdout: clippy_payload.as_bytes().to_vec(),
            }),
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 1,
                stderr: cppcheck_payload.to_string(),
                stdout: Vec::new(),
            }),
            Ok(opengrep_payload.as_bytes().to_vec()),
        ]));
        executor.write_repo_file("Cargo.toml", "[package]\nname='repo'\nversion='0.1.0'\n");

        let artifacts = PolyglotSastSidecar::extract(PolyglotSastInput {
            executor: Arc::clone(&executor),
            timeout_secs: 60,
            profile: &StackProfile::Mixed(vec![SingleStack::Rust, SingleStack::CCpp]),
        })
        .await
        .unwrap();

        let unsafe_blob = String::from_utf8(artifacts.unsafe_hotspots_blob).unwrap();
        let health_blob = String::from_utf8(artifacts.health_report_blob).unwrap();

        assert!(executor.calls().iter().any(|call| call.starts_with("cargo clippy")));
        assert!(executor.calls().iter().any(|call| call.starts_with("cppcheck ")));
        assert!(executor.calls().iter().any(|call| {
            call.starts_with("opengrep scan --config")
                && call.contains("--json")
                && call.contains("--disable-version-check")
                && call.contains("--taint-intrafile")
                && call.contains("[cwd=")
        }));
        assert!(unsafe_blob.contains("\"issues\""));
        assert!(unsafe_blob.contains("src/lib.rs"));
        assert!(unsafe_blob.contains("native/bridge.cpp"));
        assert!(health_blob.contains("\"router\""));
        assert!(health_blob.contains("\"opengrep\""));
        assert!(health_blob.contains(r#""blade": "opengrep""#));
        assert!(health_blob.contains(r#""scope": ".""#));
    }

    #[test]
    fn test_cppcheck_blade_enforces_xml_v2_args() {
        let (binary, args) = blade_command(StaticAnalysisBlade::Cppcheck);
        assert_eq!(binary, "cppcheck");
        assert!(args.iter().any(|arg| arg == "--xml"));
        assert!(args.iter().any(|arg| arg == "--xml-version=2"));
    }

    #[test]
    fn test_opengrep_args_use_runtime_compatible_flags_and_test_excludes() {
        let args = opengrep_args("C:/rules");
        assert!(args.iter().any(|arg| arg == "--allow-rule-timeout-control"));
        assert!(!args.iter().any(|arg| arg == "--exclude-minified-files"));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "tests"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/mocks/**"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/samples/**"]));
        assert!(args.windows(2).any(|pair| pair == ["--exclude", "**/output.json"]));
        assert!(!args.windows(2).any(|pair| pair == ["--exclude", "C:/rules"]));
        assert!(!args.windows(2).any(|pair| pair == ["--exclude", SEMGREP_SECURITY_RULE_FILE]));
        assert!(!args.windows(2).any(|pair| pair == ["--exclude", SEMGREP_HEALTH_RULE_FILE]));
        assert!(!args.iter().any(|arg| arg.contains("Cargo.lock")));
        assert!(!args.iter().any(|arg| arg.contains("package-lock.json")));
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

    #[test]
    fn test_normalize_cppcheck_output_ignores_progress_prefix() {
        let repo_path = Path::new("C:/repos/example");
        let payload = concat!(
            "Checking src\\\\main.c ...\r\n",
            "1/1 files checked 100% done\r\n",
            "<results><errors><error id=\"memleak\" severity=\"warning\" msg=\"Memory leak: ptr\">",
            "<location file=\"src/main.c\" line=\"42\"/></error></errors></results>"
        );

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].file, "src/main.c");
        assert!(issues[0].message.contains("memleak"));
    }

    #[test]
    fn test_extract_json_payload_discards_terminal_noise_prefix() {
        let bytes = br#"warning: compiling helper
progress 10%
{"results":[{"ok":true}]}"#;
        let payload = extract_json_payload(bytes).unwrap();
        let value: serde_json::Value = serde_json::from_slice(payload).unwrap();
        assert_eq!(value["results"][0]["ok"], true);
    }

    #[test]
    fn test_extract_json_payload_discards_terminal_noise_suffix() {
        let bytes = br#"{"results":[{"ok":true}]}
Done in 1.23s"#;
        let payload = extract_json_payload(bytes).unwrap();
        let value: serde_json::Value = serde_json::from_slice(payload).unwrap();
        assert_eq!(value["results"][0]["ok"], true);
    }

    #[test]
    fn test_extract_xml_payload_discards_terminal_noise_prefix() {
        let text = "Checking src/main.c ...\n<results><errors></errors></results>";
        let payload = extract_xml_payload(text.as_bytes()).unwrap();
        let payload = std::str::from_utf8(payload).unwrap();
        assert!(payload.starts_with("<results>"));
    }

    #[test]
    fn test_sobelow_blade_uses_mix_with_private_json_flags() {
        let (binary, args) = blade_command(StaticAnalysisBlade::Sobelow);
        assert_eq!(binary, "mix");
        assert_eq!(args, vec!["sobelow", "--format", "json", "--private"]);
    }

    #[test]
    fn test_sobelow_empty_payload_degrades_without_parse_error() {
        let repo_path = Path::new("C:/repos/example");
        let issues =
            normalize_sast_output(repo_path, repo_path, StaticAnalysisBlade::Sobelow, b"").unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_normalize_sobelow_text_fallback_extracts_findings() {
        let repo_path = Path::new("C:/repos/jido");
        let payload = r#"** (UndefinedFunctionError) function Jason.encode!/2 is undefined (module Jason is not available)
    Jason.encode!(%{findings: %{high_confidence: [%{file: "C:/repos/jido/lib/jido/storage/redis.ex", line: 337, type: "Misc.BinToTerm: Unsafe `binary_to_term`", variable: :binary}], low_confidence: [%{file: "C:/repos/jido/lib/jido/plugin/instance.ex", line: 123, type: "DOS.StringToAtom: Unsafe `String.to_atom`", variable: "base_key and as_alias"}], medium_confidence: []}, sobelow_version: "0.14.1", total_findings: 2}, [pretty: true])"#;

        let issues = normalize_sobelow_text_issues(repo_path, repo_path, payload);
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].file, "lib/jido/plugin/instance.ex");
        assert!(issues[0].message.contains("String.to_atom"));
        assert_eq!(issues[1].file, "lib/jido/storage/redis.ex");
        assert!(issues[1].message.contains("binary_to_term"));
    }

    #[test]
    fn test_discover_monorepo_manifests_ignores_heavy_directories() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("Cargo.toml", "[package]\nname='root'\nversion='0.1.0'\n");
        executor.write_repo_file("apps/rust-sdk/Cargo.toml", "[package]\nname='sdk'\nversion='0.1.0'\n");
        executor.write_repo_file("node_modules/ignored/package.json", "{}");
        executor.write_repo_file("target/ignored/Cargo.toml", "[package]\nname='ignored'\nversion='0.1.0'\n");

        let manifests = discover_monorepo_manifests(executor.repo_path());
        let scopes = manifests.iter().map(|manifest| manifest.scope.clone()).collect::<Vec<_>>();

        assert!(scopes.contains(&".".to_string()));
        assert!(scopes.contains(&"apps/rust-sdk".to_string()));
        assert!(!scopes.iter().any(|scope| scope.contains("node_modules")));
        assert!(!scopes.iter().any(|scope| scope.contains("target")));
    }

    #[test]
    fn test_derive_opengrep_execution_targets_uses_scoped_ast_roots() {
        let executor = MockExecutor::new(Vec::new());
        for idx in 0..90 {
            executor.write_repo_file(
                &format!("packages/web/src/compiler/phases/1-parse/file_{idx}.ts"),
                "export const alpha = 1;\n",
            );
            executor.write_repo_file(
                &format!("packages/web/src/compiler/phases/2-analyze/file_{idx}.ts"),
                "export const beta = 2;\n",
            );
        }
        executor.write_repo_file("packages/web/tests/samples/case.ts", "export const noisy = 1;\n");
        executor.write_repo_file("playgrounds/sandbox/src/main.ts", "export const preview = 1;\n");

        let targets = derive_opengrep_execution_targets(executor.repo_path());
        let scopes = targets
            .iter()
            .map(|target| target.scope.clone())
            .collect::<Vec<_>>();

        assert!(
            scopes.contains(&"packages/web/src/compiler/phases/1-parse".to_string()),
            "scopes={scopes:?}"
        );
        assert!(
            scopes.contains(&"packages/web/src/compiler/phases/2-analyze".to_string()),
            "scopes={scopes:?}"
        );
        assert!(!scopes.iter().any(|scope| scope.contains("tests")), "scopes={scopes:?}");
        assert!(
            !scopes.iter().any(|scope| scope.contains("playgrounds")),
            "scopes={scopes:?}"
        );
        assert!(!scopes.iter().any(|scope| scope == "."), "scopes={scopes:?}");
    }

    #[test]
    fn test_normalize_relative_issue_file_prefixes_subproject_scope() {
        let repo_path = Path::new("C:/repos/firecrawl");
        let execution_root = Path::new("C:/repos/firecrawl/apps/rust-sdk");
        let normalized = normalize_relative_issue_file(repo_path, execution_root, "src/lib.rs");
        assert_eq!(normalized, "apps/rust-sdk/src/lib.rs");
    }

    #[tokio::test]
    async fn test_polyglot_sast_sidecar_executes_rust_subprojects_with_scoped_cwd() {
        let clippy_payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"lint in workspace member","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}"#;
        let executor = Arc::new(MockExecutor::new(vec![
            Err(SandboxError::ProcessNonZeroExit {
                exit_code: 1,
                stderr: "findings".to_string(),
                stdout: clippy_payload.as_bytes().to_vec(),
            }),
            Ok(br#"{"results":[]}"#.to_vec()),
        ]));
        executor.write_repo_file("apps/rust-sdk/Cargo.toml", "[package]\nname='sdk'\nversion='0.1.0'\n");

        let artifacts = PolyglotSastSidecar::extract(PolyglotSastInput {
            executor: Arc::clone(&executor),
            timeout_secs: 60,
            profile: &StackProfile::Rust,
        })
        .await
        .unwrap();
        let health_blob = String::from_utf8(artifacts.health_report_blob).unwrap();

        assert!(executor.calls().iter().any(|call| {
            call.starts_with("cargo clippy")
                && (call.contains("apps\\rust-sdk") || call.contains("apps/rust-sdk"))
        }));
        assert!(health_blob.contains("apps/rust-sdk/src/lib.rs"));
        assert!(health_blob.contains(r#""scope": "apps/rust-sdk""#));
    }

    #[tokio::test]
    async fn test_run_sast_blade_cleans_clippy_target_dir_after_execution() {
        let clippy_payload = r#"{"reason":"compiler-message","message":{"level":"warning","message":"lint in workspace member","spans":[{"file_name":"src\\lib.rs","is_primary":true}]}}"#;
        let executor = MockExecutor::new(vec![Err(SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "findings".to_string(),
            stdout: clippy_payload.as_bytes().to_vec(),
        })]);
        executor.write_repo_file("apps/rust-sdk/Cargo.toml", "[package]\nname='sdk'\nversion='0.1.0'\n");
        executor.write_repo_file("apps/rust-sdk/target/debug/.keep", "temp");
        let execution_root = executor.repo_path().join("apps").join("rust-sdk");

        let payload = run_sast_blade(
            &executor,
            StaticAnalysisBlade::RustClippy,
            60,
            &execution_root,
        )
        .await
        .unwrap();

        assert!(!payload.is_empty());
        assert!(!execution_root.join("target").exists());
    }
}
