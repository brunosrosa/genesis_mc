use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use ignore::WalkBuilder;
use quick_xml::de::from_str as xml_from_str;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Mutex as AsyncMutex, Semaphore};
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use crate::harvester::ast_parser::{self, AstParserError};
use crate::harvester::detect::{SingleStack, StackProfile};
use crate::harvester::repo_radar;
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

fn cached_regex<'a>(cache: &'a OnceLock<Option<Regex>>, pattern: &str) -> Option<&'a Regex> {
    cache.get_or_init(|| Regex::new(pattern).ok()).as_ref()
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
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
    pub clean_files: Arc<Vec<PathBuf>>,
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
        blocks.push((
            score,
            ScopedTextBlock {
                file_path: rel,
                items: vec![content],
                omitted_count: 0,
            },
        ));
    }

    blocks.sort_by(|(score_l, block_l), (score_r, block_r)| {
        score_r.cmp(score_l).then_with(|| block_l.file_path.cmp(&block_r.file_path))
    });
    let packed = {
        let block_refs = blocks.iter().map(|(_, block)| block).collect::<Vec<_>>();
        render_scoped_text_block_refs(&block_refs)
    };
    let kind = if skill_signal { "SkillLibrary" } else { "ContentRepo" };
    let mut outline = String::new();
    outline.push_str("# Repository Outline\n\n");
    outline.push_str("kind: ");
    outline.push_str(kind);
    outline.push('\n');
    outline.push_str("note: Repositório sem arquivos de código indexáveis (curadoria/documentação/skills).\n");
    outline.push_str("why: ");
    outline.push_str(why.trim());
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
                        items: vec![markdown],
                        omitted_count: 0,
                    });
                }
            }
            Err(err) => {
                remote_fetch_failures.push(format!(
                    "{} => {}",
                    url,
                    err
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

    let mut health = String::from("# Health Report\n");
    health.push_str("\nsummary: findings=0");
    health.push_str("\nsource: content-repo-fallback");
    health.push_str("\nkind: ");
    health.push_str(kind);
    health.push_str("\nwhy: ");
    health.push_str(why.trim());
    health.push_str("\nmarkdown_files: ");
    health.push_str(&md_files.len().to_string());
    health.push_str("\ngithub_repo_links: ");
    health.push_str(&gh_repos.len().to_string());
    health.push_str("\nexternal_links: ");
    health.push_str(&external.len().to_string());
    health.push_str("\nremote_doc_candidates: ");
    health.push_str(&prioritized_remote_docs.len().to_string());
    health.push_str("\nremote_doc_fetched: ");
    health.push_str(&remote_blocks.len().to_string());
    health.push_str("\nremote_doc_failed: ");
    health.push_str(&remote_fetch_failures.len().to_string());
    health.push_str("\nskill_signal: ");
    health.push_str(if skill_signal { "true" } else { "false" });

    if !remote_blocks.is_empty() {
        outline.push_str("\n\n## Remote Documentation (Guaranteed)\n\n");
        outline.push_str(&render_scoped_text_blocks(&remote_blocks));
    }

    Ok(NativeAstArtifacts {
        repo_outline_blob: outline.into_bytes(),
        architecture_map_blob: link_map.into_bytes(),
        health_report_blob: health.into_bytes(),
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

const BLOB_04_REPO_OUTLINE_MAX_CHARS: usize = 3_000_000;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DomainTag {
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

fn render_scoped_text_blocks(blocks: &[ScopedTextBlock]) -> String {
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

fn render_scoped_text_block_refs(blocks: &[&ScopedTextBlock]) -> String {
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

    fn copies_workspace_rules(self) -> bool {
        matches!(self, Self::Security)
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
    if let Some(path) = all_candidates.iter().position(|path| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_ascii_lowercase().contains(&repo_lower))
            .unwrap_or(false)
    }) {
        return Ok(all_candidates.swap_remove(path));
    }

    if all_candidates.len() == 1 {
        return Ok(all_candidates.swap_remove(0));
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

fn truncate_chars(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
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
        tracing::debug!(binary = "native-ast-parser", "Sidecar claude-md retornou stdout vazio");
        return Err(SidecarError::ExecutionFailed {
            reason: "native-ast-parser claude-md returned empty stdout".to_string(),
        });
    }

    let text = String::from_utf8_lossy(bytes);
    let normalized = if text.contains("[DOMAIN: ") && text.contains("## Productive Tree") {
        text.trim().to_string()
    } else {
        normalize_repo_outline_markdown(&text)
    };
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
                || (binary == "opengrep" && exit_code == 7)
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
                } else if is_sobelow_mix_invocation(binary, args)
                    && stdout_is_blank(&sanitized_stdout)
                    && !sanitized_stderr.trim().is_empty()
                {
                    Ok(sanitized_stderr.into_bytes())
                } else if binary == "opengrep"
                    && exit_code == 7
                    && stdout_is_blank(&sanitized_stdout)
                    && !sanitized_stderr.trim().is_empty()
                {
                    Ok(sanitized_stderr.into_bytes())
                } else {
                    Ok(sanitized_stdout)
                }
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
        let clean_files = Arc::clone(&input.clean_files);
        let native_artifacts = tokio::task::spawn_blocking(move || {
            ast_parser::extract_repository_outline_native_from_clean_files(&repo_path, &clean_files)
        })
        .await
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao aguardar parser AST nativo: {}", e),
        })?;
        let native_artifacts = match native_artifacts {
            Ok(artifacts) => NativeAstArtifacts {
                repo_outline_blob: artifacts.repo_outline_blob,
                health_report_blob: artifacts.health_report_blob,
                architecture_map_blob: artifacts.architecture_map_blob,
            },
            Err(AstParserError::EmptyRepository { path }) => {
                return content_repo_artifacts(
                    input.executor.repo_path(),
                    &format!("no source files found in {}", path),
                )
                .await;
            }
            Err(AstParserError::NoStructuralSymbols { .. }) => {
                let architecture_map = ast_parser::build_architecture_map_blob_from_clean_files(
                    input.executor.repo_path(),
                    &input.clean_files,
                );
                NativeAstArtifacts {
                    repo_outline_blob: Vec::new(),
                    health_report_blob: Vec::new(),
                    architecture_map_blob: architecture_map,
                }
            }
            Err(other) => {
                return Err(SidecarError::ExecutionFailed {
                    reason: other.to_string(),
                });
            }
        };

        let repo_outline_blob = if native_artifacts.repo_outline_blob.is_empty() {
            Vec::new()
        } else {
            normalize_repo_outline(&native_artifacts.repo_outline_blob)?
        };
        let health_report_blob = native_artifacts.health_report_blob;
        let architecture_map_blob = native_artifacts.architecture_map_blob;
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            repo_outline_bytes = repo_outline_blob.len(),
            architecture_map_bytes = architecture_map_blob.len(),
            health_report_bytes = health_report_blob.len(),
            "ast-native: artefatos normalizados"
        );

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
        ".test.mjs",
        ".test.cjs",
        ".test.mts",
        ".test.cts",
        ".spec.ts",
        ".spec.tsx",
        ".spec.js",
        ".spec.jsx",
        ".spec.mjs",
        ".spec.cjs",
        ".spec.mts",
        ".spec.cts",
        "_test.go",
        "_test.py",
        "_test.exs",
        "test_",
        "__tests__",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_inline_test_candidate_source_file(profile: &StackProfile, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("rs") => supports_stack(profile, SingleStack::Rust),
        Some("go") => supports_stack(profile, SingleStack::Go),
        _ => false,
    }
}

fn supports_stack(profile: &StackProfile, target: SingleStack) -> bool {
    match profile {
        StackProfile::Mixed(stacks) => stacks.contains(&target),
        StackProfile::Unknown => true,
        _ => primary_stack(profile) == Some(target),
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    let normalized_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let normalized_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    normalized_path
        .strip_prefix(&normalized_root)
        .unwrap_or(normalized_path.as_path())
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_supported_test_file(profile: &StackProfile, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    let normalized = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();

    if is_inline_test_candidate_source_file(profile, path) {
        return true;
    }

    match extension.as_deref() {
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
        Some("exs") => {
            supports_stack(profile, SingleStack::Elixir)
                && (normalized.ends_with("_test.exs")
                    || normalized.contains("/test/")
                    || normalized.contains("/tests/"))
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

fn is_rust_test_attribute(trimmed: &str) -> bool {
    trimmed.starts_with("#[")
        && (trimmed.contains("test]") || trimmed.contains("rstest") || trimmed.contains("fixture"))
}

fn extract_python_test_entries_shallow(content: &str) -> Vec<String> {
    static PYTHON_TEST_DEF_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(re) = cached_regex(
        &PYTHON_TEST_DEF_RE,
        r#"(?m)^\s*(?:async\s+def|def)\s+(test_[A-Za-z0-9_]+)\s*\("#,
    ) else {
        return Vec::new();
    };
    let mut entries = BTreeSet::new();
    for captures in re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("def {}", name.as_str()));
        }
    }
    entries.into_iter().collect()
}

fn extract_rust_test_entries_shallow(content: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    let mut saw_test_attr = false;
    for line in content.lines().take(2_000) {
        let trimmed = line.trim();
        if is_rust_test_attribute(trimmed) {
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

fn extract_go_test_entries_shallow(content: &str) -> Vec<String> {
    static GO_TEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    static GO_SUBTEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(test_re) = cached_regex(
        &GO_TEST_RE,
        r#"(?m)^\s*(func\s+(?:\([^)]+\)\s+)?(?:Test|Fuzz)[A-Z][A-Za-z0-9_]*\s*\([^)]*\))"#,
    ) else {
        return Vec::new();
    };
    let Some(subtest_re) = cached_regex(
        &GO_SUBTEST_RE,
        r#"\b[A-Za-z0-9_]+\.(?:Run|Fuzz)\(\s*"([^"]+)""#,
    ) else {
        return Vec::new();
    };
    let mut entries = BTreeSet::new();
    for captures in test_re.captures_iter(content) {
        if let Some(signature) = captures.get(1) {
            if let Some(signature) = compact_signature_text(signature.as_str()) {
                entries.insert(signature);
            }
        }
    }
    for captures in subtest_re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("subtest \"{}\"", name.as_str()));
        }
    }
    entries.into_iter().collect()
}

fn extract_elixir_test_entries_shallow(content: &str) -> Vec<String> {
    static ELIXIR_TEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    static ELIXIR_DESCRIBE_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(test_re) = cached_regex(&ELIXIR_TEST_RE, r#"(?m)^\s*test\s+"([^"]+)"\s+do"#) else {
        return Vec::new();
    };
    let Some(describe_re) = cached_regex(
        &ELIXIR_DESCRIBE_RE,
        r#"(?m)^\s*describe\s+"([^"]+)"\s+do"#,
    ) else {
        return Vec::new();
    };
    let mut entries = BTreeSet::new();
    for captures in describe_re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("describe \"{}\"", name.as_str()));
        }
    }
    for captures in test_re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("test \"{}\"", name.as_str()));
        }
    }
    entries.into_iter().collect()
}

fn extract_frontend_test_entries_shallow(content: &str) -> Vec<String> {
    static FRONTEND_TEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(re) = cached_regex(
        &FRONTEND_TEST_RE,
        r#"(?:^|[^\w])(describe|it|test)(?:\.(?:only|skip|concurrent|each|todo|failing))*\s*\(\s*(?:"([^"\r\n]+)"|'([^'\r\n]+)'|`([^`\r\n]+)`)"#,
    ) else {
        return Vec::new();
    };
    let mut out = BTreeSet::new();
    for captures in re.captures_iter(content) {
        let kind = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
        let label = captures
            .get(2)
            .or_else(|| captures.get(3))
            .or_else(|| captures.get(4))
            .map(|m| m.as_str())
            .unwrap_or_default()
            .trim();
        if !kind.is_empty() && !label.is_empty() {
            out.insert(format!(r#"{kind} "{label}""#));
        }
    }
    out.into_iter().collect()
}

fn build_scoped_blocks_from_pairs(pairs: Vec<(String, String)>) -> Vec<ScopedTextBlock> {
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

fn discover_static_test_entries_bfs(
    repo_path: &Path,
    profile: &StackProfile,
) -> Result<Vec<ScopedTextBlock>, SidecarError> {
    let mut blocks = Vec::new();
    let radar = repo_radar::build_repo_radar(repo_path);
    for path in radar.all_files() {
        if !is_supported_test_file(profile, path) {
            continue;
        }

        let relative = relative_display(repo_path, path);
        if should_skip_discovered_test_entry(&relative) {
            continue;
        }

        let Some(content) = read_static_test_file(path)? else {
            continue;
        };
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());

        let discovered = match extension.as_deref() {
            Some("rs") => extract_rust_test_entries_shallow(&content),
            Some("py") => extract_python_test_entries_shallow(&content),
            Some("go") => extract_go_test_entries_shallow(&content),
            Some("exs") => extract_elixir_test_entries_shallow(&content),
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
        if items.is_empty() {
            continue;
        }

        blocks.push(ScopedTextBlock {
            file_path: relative,
            items,
            omitted_count: 0,
        });
    }

    Ok(blocks)
}

pub struct NativeTestDiscoverySidecar;

impl NativeTestDiscoverySidecar {
    pub async fn extract(input: NativeTestDiscoveryInput<'_>) -> Result<TestIntentPayload, SidecarError> {
        let repo_path = input.repo_path.to_path_buf();
        let profile = input.profile.clone();
        let blocks =
            tokio::task::spawn_blocking(move || discover_static_test_entries_bfs(&repo_path, &profile))
                .await
                .map_err(|e| SidecarError::ExecutionFailed {
                    reason: format!("Static test discovery join failed: {}", e),
                })??;
        Ok(TestIntentPayload {
            runner_name: "static-ast-radar".to_string(),
            timed_out: false,
            blocks,
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
        parse_json_payload::<StaticAnalysisPayload>(&bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SodaHealthIssue {
    pub level: String,
    pub file: String,
    pub message: String,
    #[serde(skip)]
    source_blade: String,
    #[serde(skip)]
    channel: SastIssueChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SastIssueChannel {
    UnsafeHotspot,
    #[default]
    Health,
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
    pub clean_files: Arc<Vec<PathBuf>>,
}

const MONOREPO_SAST_MAX_PARALLEL: usize = 8;
const RUST_CLIPPY_MAX_PARALLEL: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ManifestKind {
    CargoToml,
    PackageJson,
    MixExs,
    GoMod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsLintProfile {
    UnsafeHotspot,
    Health,
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
    scan_targets: Vec<String>,
    command_args: Option<Vec<String>>,
    forced_channel: Option<SastIssueChannel>,
}

#[derive(Debug)]
struct SastExecutionOutcome {
    requested_blade: StaticAnalysisBlade,
    effective_blade: StaticAnalysisBlade,
    execution_root: PathBuf,
    scope: String,
    forced_channel: Option<SastIssueChannel>,
    result: Result<Vec<u8>, SidecarError>,
}

#[derive(Debug)]
struct SastBladeResult {
    effective_blade: StaticAnalysisBlade,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustClippyPlan {
    command_args: Vec<String>,
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

fn clippy_args_for_package(package_name: &str) -> Vec<String> {
    vec![
        "clippy".to_string(),
        "--message-format=json".to_string(),
        "--workspace".to_string(),
        "--offline".to_string(),
        "--frozen".to_string(),
        "-p".to_string(),
        package_name.to_string(),
        "--".to_string(),
        "--no-deps".to_string(),
    ]
}

fn default_clippy_args() -> Vec<String> {
    vec![
        "clippy".to_string(),
        "--message-format=json".to_string(),
        "--workspace".to_string(),
        "--offline".to_string(),
        "--frozen".to_string(),
        "--".to_string(),
        "--no-deps".to_string(),
    ]
}

fn cargo_lockfile_path(manifest_path: &Path) -> PathBuf {
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("Cargo.lock")
}

fn cargo_fetch_args(manifest_path: &Path, use_locked: bool) -> Vec<String> {
    let mut args = vec!["fetch".to_string()];
    if use_locked {
        args.push("--locked".to_string());
    }
    args.push("--manifest-path".to_string());
    args.push(manifest_path.display().to_string());
    args
}

fn cargo_metadata_args(manifest_path: &Path, use_locked: bool) -> Vec<String> {
    let mut args = vec![
        "metadata".to_string(),
        "--format-version".to_string(),
        "1".to_string(),
    ];
    if use_locked {
        args.push("--locked".to_string());
    }
    args.push("--offline".to_string());
    args.push("--manifest-path".to_string());
    args.push(manifest_path.display().to_string());
    args
}

fn rust_clippy_manifest_path(execution_root: &Path) -> PathBuf {
    execution_root.join("Cargo.toml")
}

fn rust_clippy_preflight_timeout_secs(timeout_secs: u64) -> u64 {
    timeout_secs.clamp(60, 180)
}

fn manifest_effectively_has_build_script(
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

fn rust_manifest_native_marker(value: &toml::Value) -> Option<&'static str> {
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

fn build_rust_clippy_plan(manifest: &DiscoveredManifest) -> Result<RustClippyPlan, String> {
    let manifest_text = std::fs::read_to_string(&manifest.manifest_path).map_err(|error| {
        format!(
            "nao foi possivel ler {}: {error}",
            manifest.manifest_path.display()
        )
    })?;
    let manifest_value = manifest_text
        .parse::<toml::Value>()
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

    if manifest_effectively_has_build_script(&manifest.execution_root, package_table) {
        return Err("package contem build.rs efetivo".to_string());
    }

    if let Some(links) = package_table.get("links").and_then(toml::Value::as_str) {
        return Err(format!("package declara links={links}"));
    }

    if let Some(marker) = rust_manifest_native_marker(&manifest_value) {
        return Err(format!("manifesto referencia dependencia nativa/FFI marker={marker}"));
    }

    Ok(RustClippyPlan {
        command_args: clippy_args_for_package(package_name),
    })
}

#[derive(Debug, Deserialize)]
struct CargoMetadataPackage {
    manifest_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CargoMetadataPayload {
    packages: Vec<CargoMetadataPackage>,
}

fn rust_manifest_declares_build_dependencies(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(entries) => entries.iter().any(|(key, inner)| {
            key.eq_ignore_ascii_case("build-dependencies")
                || rust_manifest_declares_build_dependencies(inner)
        }),
        toml::Value::Array(items) => items.iter().any(rust_manifest_declares_build_dependencies),
        _ => false,
    }
}

fn rust_manifest_declares_proc_macro(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(entries) => entries.iter().any(|(key, inner)| {
            (key.eq_ignore_ascii_case("proc-macro")
                && inner.as_bool().unwrap_or(false))
                || rust_manifest_declares_proc_macro(inner)
        }),
        toml::Value::Array(items) => items.iter().any(rust_manifest_declares_proc_macro),
        _ => false,
    }
}

fn fail_closed_rust_manifest(reason: String) -> SidecarError {
    SidecarError::ExecutionFailed {
        reason: format!("cargo-clippy fail-closed: {reason}"),
    }
}

fn rust_clippy_should_fallback_to_opengrep(err: &SidecarError) -> Option<String> {
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

fn audit_transitive_rust_manifests(
    repo_path: &Path,
    metadata_bytes: &[u8],
) -> Result<(), SidecarError> {
    let payload = parse_json_payload::<CargoMetadataPayload>(metadata_bytes)?;
    let mut manifests = payload
        .packages
        .into_iter()
        .map(|package| package.manifest_path)
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
        let manifest_value = manifest_text.parse::<toml::Value>().map_err(|error| {
            fail_closed_rust_manifest(format!(
                "manifesto transitivo invalido em '{}': {error}",
                sanitize_host_paths_in_text(repo_path, &manifest_path.display().to_string())
            ))
        })?;
        let manifest_root = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let manifest_label =
            sanitize_host_paths_in_text(repo_path, &manifest_path.display().to_string());

        if let Some(package_table) = manifest_value.get("package").and_then(toml::Value::as_table) {
            if manifest_effectively_has_build_script(manifest_root, package_table) {
                return Err(fail_closed_rust_manifest(format!(
                    "manifesto transitivo '{}' contem build.rs efetivo",
                    manifest_label
                )));
            }

            if let Some(links) = package_table.get("links").and_then(toml::Value::as_str) {
                return Err(fail_closed_rust_manifest(format!(
                    "manifesto transitivo '{}' declara links={links}",
                    manifest_label
                )));
            }
        }

        if rust_manifest_declares_build_dependencies(&manifest_value) {
            return Err(fail_closed_rust_manifest(format!(
                "manifesto transitivo '{}' declara build-dependencies",
                manifest_label
            )));
        }

        if rust_manifest_declares_proc_macro(&manifest_value) {
            return Err(fail_closed_rust_manifest(format!(
                "manifesto transitivo '{}' declara proc-macro = true",
                manifest_label
            )));
        }

        if let Some(marker) = rust_manifest_native_marker(&manifest_value) {
            return Err(fail_closed_rust_manifest(format!(
                "manifesto transitivo '{}' referencia dependencia nativa/FFI marker={marker}",
                manifest_label
            )));
        }
    }

    Ok(())
}

async fn run_rust_clippy_preflight<E: SandboxExecutor>(
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

    let preflight_timeout_secs = rust_clippy_preflight_timeout_secs(timeout_secs);
    let lockfile_path = cargo_lockfile_path(&manifest_path);

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

    let metadata_args = cargo_metadata_args(&manifest_path, lockfile_path.is_file());
    let metadata_arg_refs = metadata_args.iter().map(String::as_str).collect::<Vec<_>>();
    let metadata_bytes = execute_sidecar_in_dir(
        executor,
        "cargo",
        &metadata_arg_refs,
        preflight_timeout_secs,
        SidecarExitPolicy::StrictZeroOnly,
        execution_root,
    )
    .await?;

    audit_transitive_rust_manifests(executor.repo_path(), &metadata_bytes)
}

fn derive_rust_clippy_execution_targets(manifests: &[DiscoveredManifest]) -> Vec<SastExecutionTarget> {
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
                info!(
                    manifest = %manifest.manifest_path.display(),
                    scope = %manifest.scope,
                    reason = %reason,
                    "SAST rust-clippy: manifesto blindado para evitar build.rs/FFI"
                );
                None
            }
        })
        .collect()
}

fn is_go_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("go"))
        .unwrap_or(false)
}

fn is_elixir_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "ex" | "exs"))
        .unwrap_or(false)
}

fn govulncheck_args_for_module() -> Vec<String> {
    vec![
        "-format".to_string(),
        "json".to_string(),
        "./...".to_string(),
    ]
}

fn sobelow_args_for_root(root: &str) -> Vec<String> {
    vec![
        "sobelow".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--private".to_string(),
        "--root".to_string(),
        root.to_string(),
    ]
}

fn derive_go_execution_targets(
    manifests: &[DiscoveredManifest],
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    manifests
        .iter()
        .filter(|manifest| manifest.kind == ManifestKind::GoMod)
        .filter_map(|manifest| {
            let boundaries =
                descendant_roots_for_manifest(manifests, &manifest.execution_root, ManifestKind::GoMod);
            let productive_go_files = derive_repo_relative_clean_targets(
                &manifest.execution_root,
                clean_files,
                &boundaries,
                is_go_supported_file,
            );
            if productive_go_files.is_empty() {
                info!(
                    manifest = %manifest.manifest_path.display(),
                    scope = %manifest.scope,
                    "SAST govulncheck: manifesto ignorado por ausencia de pacotes Go produtivos"
                );
                return None;
            }

            Some(SastExecutionTarget {
                blade: StaticAnalysisBlade::Govulncheck,
                execution_root: manifest.execution_root.clone(),
                scope: manifest.scope.clone(),
                scan_targets: vec!["./...".to_string()],
                command_args: Some(govulncheck_args_for_module()),
                forced_channel: None,
            })
        })
        .collect()
}

fn derive_elixir_execution_targets(
    manifests: &[DiscoveredManifest],
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    manifests
        .iter()
        .filter(|manifest| manifest.kind == ManifestKind::MixExs)
        .filter_map(|manifest| {
            let boundaries =
                descendant_roots_for_manifest(manifests, &manifest.execution_root, ManifestKind::MixExs);
            let productive_elixir_files = derive_repo_relative_clean_targets(
                &manifest.execution_root,
                clean_files,
                &boundaries,
                is_elixir_supported_file,
            );
            if productive_elixir_files.is_empty() {
                info!(
                    manifest = %manifest.manifest_path.display(),
                    scope = %manifest.scope,
                    "SAST sobelow: manifesto ignorado por ausencia de codigo Elixir produtivo"
                );
                return None;
            }

            Some(SastExecutionTarget {
                blade: StaticAnalysisBlade::Sobelow,
                execution_root: manifest.execution_root.clone(),
                scope: manifest.scope.clone(),
                scan_targets: vec![".".to_string()],
                command_args: Some(sobelow_args_for_root(".")),
                forced_channel: None,
            })
        })
        .collect()
}

fn blade_parallelism_limit(blade: StaticAnalysisBlade) -> usize {
    match blade {
        StaticAnalysisBlade::RustClippy => RUST_CLIPPY_MAX_PARALLEL,
        _ => MONOREPO_SAST_MAX_PARALLEL,
    }
}

fn has_global_opengrep_coverage(targets: &[SastExecutionTarget]) -> bool {
    targets.iter().any(|target| {
        target.blade == StaticAnalysisBlade::Opengrep
            && (target.scope == "."
                || target.scope.starts_with(".::files-")
                || target.scope.starts_with(".::unsafe::files-")
                || target.scope.starts_with(".::health::files-"))
    })
}

fn execution_targets_for_blade(
    repo_path: &Path,
    clean_files: &[PathBuf],
    manifests: &[DiscoveredManifest],
    blade: StaticAnalysisBlade,
) -> Vec<SastExecutionTarget> {
    if blade == StaticAnalysisBlade::Opengrep {
        return derive_opengrep_execution_targets(repo_path, clean_files);
    }
    if blade == StaticAnalysisBlade::Cppcheck {
        return derive_cppcheck_execution_targets(repo_path, clean_files);
    }
    if blade == StaticAnalysisBlade::RustClippy {
        return derive_rust_clippy_execution_targets(manifests);
    }
    if blade == StaticAnalysisBlade::Govulncheck {
        return derive_go_execution_targets(manifests, clean_files);
    }
    if blade == StaticAnalysisBlade::Sobelow {
        return derive_elixir_execution_targets(manifests, clean_files);
    }
    if blade == StaticAnalysisBlade::Biome {
        return derive_js_lint_execution_targets(repo_path, manifests, blade, clean_files);
    }
    if matches!(blade, StaticAnalysisBlade::Ruff | StaticAnalysisBlade::Bandit) {
        return derive_python_execution_targets(repo_path, blade, clean_files);
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

const OPENGREP_FILE_LIST_CHUNK_SIZE: usize = 96;
const CPPCHECK_FILE_LIST_CHUNK_SIZE: usize = 96;
const JS_LINT_FILE_LIST_CHUNK_SIZE: usize = 96;
const PYTHON_LINT_FILE_LIST_CHUNK_SIZE: usize = 96;

fn blade_file_batch_scope(scope: &str, batch_idx: usize) -> String {
    format!("{scope}::files-{batch_idx:02}")
}

fn descendant_roots_for_manifest<'a>(
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

fn derive_js_lint_execution_targets(
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

fn derive_opengrep_execution_targets(
    repo_path: &Path,
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let scan_targets = derive_repo_relative_clean_targets(&repo_root, clean_files, &[], |_| true);

    let mut targets = Vec::new();
    for (profile_scope, forced_channel) in [
        (".::unsafe", SastIssueChannel::UnsafeHotspot),
        (".::health", SastIssueChannel::Health),
    ] {
        for (idx, chunk) in scan_targets.chunks(OPENGREP_FILE_LIST_CHUNK_SIZE).enumerate() {
            targets.push(SastExecutionTarget {
                blade: StaticAnalysisBlade::Opengrep,
                execution_root: repo_root.clone(),
                scope: blade_file_batch_scope(profile_scope, idx + 1),
                scan_targets: chunk.to_vec(),
                command_args: None,
                forced_channel: Some(forced_channel),
            });
        }
    }
    targets
}

fn derive_cppcheck_execution_targets(
    repo_path: &Path,
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let scan_targets = derive_repo_relative_clean_targets(&repo_root, clean_files, &[], is_cpp_supported_file);
    if scan_targets.is_empty() {
        return Vec::new();
    }

    scan_targets
        .chunks(CPPCHECK_FILE_LIST_CHUNK_SIZE)
        .enumerate()
        .map(|(idx, chunk)| {
            let chunk_targets = chunk.to_vec();
            SastExecutionTarget {
                blade: StaticAnalysisBlade::Cppcheck,
                execution_root: repo_root.clone(),
                scope: blade_file_batch_scope(".", idx + 1),
                scan_targets: chunk_targets.clone(),
                command_args: Some(cppcheck_args_for_targets(&chunk_targets)),
                forced_channel: None,
            }
        })
        .collect()
}

fn derive_python_execution_targets(
    repo_path: &Path,
    blade: StaticAnalysisBlade,
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let scan_targets =
        derive_repo_relative_clean_targets(&repo_root, clean_files, &[], is_python_supported_file);

    if scan_targets.is_empty() {
        return Vec::new();
    }

    scan_targets
        .chunks(PYTHON_LINT_FILE_LIST_CHUNK_SIZE)
        .enumerate()
        .map(|(idx, chunk)| SastExecutionTarget {
            blade,
            execution_root: repo_root.clone(),
            scope: blade_file_batch_scope(".", idx + 1),
            scan_targets: chunk.to_vec(),
            command_args: None,
            forced_channel: None,
        })
        .collect()
}

fn derive_js_lint_file_list_targets(
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
                StaticAnalysisBlade::Biome => biome_args_for_profile(&chunk_targets, profile),
                StaticAnalysisBlade::Oxc => oxc_args_for_profile(&chunk_targets, profile),
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

fn derive_repo_relative_clean_targets(
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

fn should_skip_sast_relative_target(rel: &str) -> bool {
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

fn is_python_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("py"))
        .unwrap_or(false)
}

fn is_biome_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts")
}

fn is_cpp_supported_file(path: &Path) -> bool {
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

fn is_oxlint_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" | "svelte")
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

fn push_issue(
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

fn should_drop_sast_issue(blade: StaticAnalysisBlade, level: &str, message: &str) -> bool {
    is_aesthetic_or_minor_warning(blade, message)
        || is_blob_06_semantic_slop(blade, level, message)
}

fn is_aesthetic_or_minor_warning(blade: StaticAnalysisBlade, message: &str) -> bool {
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

fn is_blob_06_semantic_slop(blade: StaticAnalysisBlade, level: &str, message: &str) -> bool {
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

fn should_keep_blob06_issue(issue: &SodaHealthIssue) -> bool {
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

fn should_keep_blob08_issue(issue: &SodaHealthIssue) -> bool {
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

fn sort_and_dedup_issues(issues: &mut Vec<SodaHealthIssue>) {
    issues.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then_with(|| left.level.cmp(&right.level))
            .then_with(|| left.message.cmp(&right.message))
    });
    issues.dedup();
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

const CLI_PATH_EXCLUDES: &[&str] = &[
    "test",
    "tests",
    "testutil",
    "mocks",
    "vendor",
    "libs",
    "public/libs",
    "*test.go",
    "*test.rs",
    "*.spec.*",
    "*.test.*",
    "*.min.js",
    "*.min.cjs",
    "*.min.mjs",
    "*.iife.js",
    "*.umd.js",
    "*.bundle.js",
    "*.bundle.cjs",
    "*.bundle.mjs",
    "*.pack.js",
    "*.pack.cjs",
    "*.pack.mjs",
    "*.vendor.js",
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

fn cppcheck_args_for_targets(scan_targets: &[String]) -> Vec<String> {
    let mut args = vec![
        "--xml".to_string(),
        "--xml-version=2".to_string(),
        "--quiet".to_string(),
        "--enable=warning".to_string(),
        "--disable=style,performance,portability,information".to_string(),
    ];
    if scan_targets.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

fn cppcheck_args() -> Vec<String> {
    cppcheck_args_for_targets(&[".".to_string()])
}

fn sobelow_args() -> Vec<String> {
    sobelow_args_for_root(".")
}

fn biome_args(scan_targets: &[String]) -> Vec<String> {
    biome_args_for_profile(scan_targets, JsLintProfile::Health)
}

fn biome_args_for_profile(scan_targets: &[String], profile: JsLintProfile) -> Vec<String> {
    let mut args = vec![
        "lint".to_string(),
        "--reporter=json".to_string(),
        "--no-errors-on-unmatched".to_string(),
        "--skip-parse-errors".to_string(),
        "--vcs-enabled=true".to_string(),
        "--vcs-client-kind=git".to_string(),
        "--vcs-use-ignore-file=true".to_string(),
        "--files-ignore-unknown=true".to_string(),
    ];
    match profile {
        JsLintProfile::UnsafeHotspot => {
            args.push("--only=lint/security".to_string());
        }
        JsLintProfile::Health => {
            args.push("--only=lint/complexity".to_string());
        }
    }
    if scan_targets.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

fn oxc_args(scan_targets: &[String]) -> Vec<String> {
    oxc_args_for_profile(scan_targets, JsLintProfile::Health)
}

fn oxc_args_for_profile(scan_targets: &[String], _profile: JsLintProfile) -> Vec<String> {
    let mut args = vec![
        "lint".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
        "-A".to_string(),
        "all".to_string(),
        "-D".to_string(),
        "suspicious".to_string(),
        "--no-error-on-unmatched-pattern".to_string(),
        "--ignore-pattern".to_string(),
        "**/*.min.js".to_string(),
        "--ignore-pattern".to_string(),
        "**/*.min.cjs".to_string(),
        "--ignore-pattern".to_string(),
        "**/*.min.mjs".to_string(),
        "--ignore-pattern".to_string(),
        "**/*.bundle.js".to_string(),
        "--ignore-pattern".to_string(),
        "**/*.generated.*".to_string(),
        "--ignore-pattern".to_string(),
        "**/output.json".to_string(),
        "--ignore-pattern".to_string(),
        "**/coverage/**".to_string(),
        "--ignore-pattern".to_string(),
        "**/storybook-static/**".to_string(),
        "--ignore-pattern".to_string(),
        "**/.svelte-kit/**".to_string(),
        "--ignore-pattern".to_string(),
        "**/.next/**".to_string(),
        "--ignore-pattern".to_string(),
        "**/.nuxt/**".to_string(),
    ];
    if scan_targets.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

fn ruff_args(scan_targets: &[String]) -> Vec<String> {
    let mut args = vec![
        "check".to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        "--ignore".to_string(),
        "D,F401,UP,W".to_string(),
        "--force-exclude".to_string(),
    ];
    for exclude in CLI_PATH_EXCLUDES {
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

fn bandit_args(scan_targets: &[String]) -> Vec<String> {
    let mut args = vec![
        "-f".to_string(),
        "json".to_string(),
        "-s".to_string(),
        "B101".to_string(),
    ];
    if scan_targets.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

fn govulncheck_args() -> Vec<String> {
    govulncheck_args_for_module()
}

fn opengrep_args(rule_arg: &str, scan_targets: &[String], rule_set: SemgrepRuleSet) -> Vec<String> {
    build_semgrep_like_scan_args(
        rule_arg,
        SemgrepScanOptions {
            disable_version_check: true,
            metrics_off: false,
            taint_intrafile: matches!(rule_set, SemgrepRuleSet::Security),
            allow_rule_timeout_control: true,
            // Compat mode: o opengrep 1.23.0 no Windows anuncia esta flag no --help,
            // mas a rejeita em runtime durante `scan`.
            exclude_minified_files: false,
        },
        scan_targets,
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
        &[".".to_string()],
    )
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
    let rule_set = match forced_channel {
        Some(SastIssueChannel::Health) => SemgrepRuleSet::Health,
        _ => SemgrepRuleSet::Security,
    };
    let rule_path = ensure_semgrep_rule_bundle(executor.repo_path(), rule_set).await?;
    let rule_arg = rule_path.to_string_lossy().to_string();
    let args = opengrep_args(&rule_arg, scan_targets, rule_set);
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

fn blade_command(
    blade: StaticAnalysisBlade,
    scan_targets: &[String],
    command_args: Option<&[String]>,
) -> (&'static str, Vec<String>) {
    match blade {
        StaticAnalysisBlade::RustClippy => (
            "cargo",
            command_args
                .map(|value| value.to_vec())
                .unwrap_or_else(default_clippy_args),
        ),
        StaticAnalysisBlade::Cppcheck => (
            "cppcheck",
            command_args
                .map(|value| value.to_vec())
                .unwrap_or_else(cppcheck_args),
        ),
        StaticAnalysisBlade::Sobelow => (
            "mix",
            command_args
                .map(|value| value.to_vec())
                .unwrap_or_else(sobelow_args),
        ),
        StaticAnalysisBlade::Biome => (
            "biome",
            command_args
                .map(|value| value.to_vec())
                .unwrap_or_else(|| biome_args(scan_targets)),
        ),
        StaticAnalysisBlade::Oxc => (
            "oxlint",
            command_args
                .map(|value| value.to_vec())
                .unwrap_or_else(|| oxc_args(scan_targets)),
        ),
        StaticAnalysisBlade::Ruff => ("ruff", ruff_args(scan_targets)),
        StaticAnalysisBlade::Bandit => ("bandit", bandit_args(scan_targets)),
        StaticAnalysisBlade::Govulncheck => (
            "govulncheck",
            command_args
                .map(|value| value.to_vec())
                .unwrap_or_else(govulncheck_args),
        ),
        StaticAnalysisBlade::Opengrep => ("opengrep", Vec::new()),
    }
}

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
    let result = if blade == StaticAnalysisBlade::RustClippy {
        match run_rust_clippy_preflight(executor, execution_root, timeout_secs).await {
            Ok(()) => {
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
                    effective_blade: StaticAnalysisBlade::RustClippy,
                    bytes,
                })
            }
            Err(err) => {
                if let Some(reason) = rust_clippy_should_fallback_to_opengrep(&err) {
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

fn normalize_cppcheck_output(
    repo_path: &Path,
    execution_root: &Path,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    let text = String::from_utf8_lossy(bytes);
    let Some(xml_payload) = extract_cppcheck_xml_payload(&text) else {
        let mut issues = Vec::new();
        let preview = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .take(3)
            .collect::<Vec<_>>()
            .join(" | ");

        if preview.is_empty() {
            push_issue(
                &mut issues,
                repo_path,
                execution_root,
                StaticAnalysisBlade::Cppcheck,
                "info",
                "",
                "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck.",
            );
        } else {
            push_issue(
                &mut issues,
                repo_path,
                execution_root,
                StaticAnalysisBlade::Cppcheck,
                "warning",
                "",
                &format!("cppcheck output nao estruturado preservado: {preview}"),
            );
        }
        sort_and_dedup_issues(&mut issues);
        return Ok(issues);
    };
    let compact_xml = xml_payload.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
    if compact_xml.contains("<results></results>") || compact_xml.contains("<results/>") {
        let mut issues = Vec::new();
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Cppcheck,
            "info",
            "",
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck.",
        );
        sort_and_dedup_issues(&mut issues);
        return Ok(issues);
    }
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
            StaticAnalysisBlade::Cppcheck,
            error.severity.as_deref().unwrap_or("warning"),
            file,
            &format!("{rule}: {msg}{line}"),
        );
    }
    if issues.is_empty() {
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Cppcheck,
            "info",
            "",
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck.",
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
            StaticAnalysisBlade::Opengrep,
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

fn normalize_sobelow_text_issues(
    repo_path: &Path,
    execution_root: &Path,
    text: &str,
) -> Vec<SodaHealthIssue> {
    let Ok(finding_re) = Regex::new(
        r#"%\{file: "(?P<file>[^"]+)", line: (?P<line>\d+), type: "(?P<kind>[^"]+)"(?:, variable: (?P<variable>"[^"]+"|:[^,}]+|[A-Za-z_][A-Za-z0-9_]*))?\}"#,
    ) else {
        return Vec::new();
    };

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
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Sobelow,
            "warning",
            file,
            &message,
        );
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
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Govulncheck,
            "error",
            file,
            &message,
        );
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

fn render_unsafe_hotspots_report(issues: &[SodaHealthIssue], clean_files: &[PathBuf]) -> Vec<u8> {
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

fn render_soda_health_report(issues: &[SodaHealthIssue]) -> Vec<u8> {
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

fn is_unsafe_hotspot(issue: &SodaHealthIssue) -> bool {
    issue.channel == SastIssueChannel::UnsafeHotspot
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
                    let cargo_permit = if blade == StaticAnalysisBlade::RustClippy {
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
                    let global_permit = Arc::clone(&global_semaphore)
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
                    drop(global_permit);
                    drop(cargo_permit);
                    info!(
                        blade = blade_name(blade),
                        scope = %scope,
                        cwd = %execution_root.display(),
                        available_global_permits = global_semaphore.available_permits(),
                        available_cargo_permits = cargo_semaphore.available_permits(),
                        "SAST monorepo: sub-scan concluído"
                    );
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
                    })
                });
            }
        }

        while let Some(joined) = join_set.join_next().await {
            let outcome = match joined {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(err)) => {
                    had_failed_payload = true;
                    warn!(
                        repo_path = %repo_path.display(),
                        error = %err,
                        "SAST monorepo: worker falhou; descartando sub-scan"
                    );
                    continue;
                }
                Err(err) => {
                    had_failed_payload = true;
                    warn!(
                        repo_path = %repo_path.display(),
                        error = %err,
                        "SAST monorepo: join do worker falhou; descartando sub-scan"
                    );
                    continue;
                }
            };

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
                        had_successful_payload = true;
                        all_issues.append(&mut issues);
                    }
                    Err(err) => {
                        had_failed_payload = true;
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
                    had_failed_payload = true;
                    warn!(
                        blade = blade_name(outcome.requested_blade),
                        scope = %outcome.scope,
                        cwd = %outcome.execution_root.display(),
                        error = %err,
                        "SAST monorepo: execucao da lamina falhou; descartando sub-scan"
                    );
                }
            }
        }

        if had_failed_payload && !had_successful_payload {
            warn!(
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

        Ok(PolyglotSastArtifacts {
            unsafe_hotspots_blob: render_unsafe_hotspots_report(&unsafe_issues, &input.clean_files),
            health_report_blob: render_soda_health_report(&health_issues),
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
        text.push_str("\n\n");
        text.push_str(
            &payload
                .blocks
                .iter()
                .map(format_scoped_text_block)
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
    }

    text.into_bytes()
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

fn semgrep_bundle_locks() -> &'static Mutex<BTreeMap<String, Arc<AsyncMutex<()>>>> {
    static LOCKS: OnceLock<Mutex<BTreeMap<String, Arc<AsyncMutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn semgrep_bundle_lock_for_support_dir(support_dir: &Path) -> Arc<AsyncMutex<()>> {
    let key = support_dir
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let mut guard = lock_unpoisoned(semgrep_bundle_locks());
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

async fn ensure_semgrep_rule_bundle(repo_path: &Path, rule_set: SemgrepRuleSet) -> Result<PathBuf, SidecarError> {
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
    use rusqlite::params;
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

    fn canonicalize_or_self(path: PathBuf) -> PathBuf {
        path.canonicalize().unwrap_or(path)
    }

    fn test_clean_files(repo_root: &Path, rels: &[&str]) -> Arc<Vec<PathBuf>> {
        Arc::new(
            rels.iter()
                .map(|rel| canonicalize_or_self(repo_root.join(rel)))
                .collect(),
        )
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
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file(
            "src/main.rs",
            r#"
use crate::config::AppConfig;

fn main() {
    let _cfg = AppConfig::default();
}
"#,
        );
        executor.write_repo_file(
            "src/lib.rs",
            r#"
pub mod config {
    #[derive(Default)]
    pub struct AppConfig;
}
"#,
        );
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(executor.repo_path(), &["src/main.rs", "src/lib.rs"]),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ter sucesso: {:?}", result);
        let payload = result.unwrap();
        let health_report = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(health_report.contains("# Health Report"));
        assert!(health_report.contains(
            "source: native-rust multi-strategy (language-pack + targeted-tree-sitter + regex-fallback)"
        ));
        assert!(health_report.contains("parsed_files: 2"));
        let repo_outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        assert!(repo_outline.contains("# Repository Outline"));
        assert!(repo_outline.contains("## Productive Tree"));
        assert!(repo_outline.contains("example/"));
        assert!(repo_outline.contains("[DOMAIN: RUST]"));
        assert!(repo_outline.contains("[src/lib.rs]"));
        assert!(repo_outline.contains("[src/main.rs]"));
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();
        assert!(architecture_map.contains("[src]"));
        assert!(architecture_map.contains("src/main.rs"));
        assert!(architecture_map.contains("src/lib.rs"));
    }

    #[tokio::test]
    async fn test_extract_success_repo_outline_tolerates_invalid_utf8() {
        let claude_md = b"# Repository Outline\n\xff\n- src/main.rs\n".to_vec();

        let result = normalize_repo_outline(&claude_md);
        assert!(result.is_ok(), "Normalização deveria tolerar repo outline com UTF-8 inválido: {:?}", result);
        let repo_outline = String::from_utf8(result.unwrap()).unwrap();
        assert!(repo_outline.contains("# Repository Outline"));
        assert!(repo_outline.contains("[DOMAIN: RUST]"));
        assert!(repo_outline.contains("[src/main.rs]"));
    }

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

    #[tokio::test]
    async fn test_architecture_map_skips_visual_noise_and_prioritizes_backend() {
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file("icons/logo.svg", "<svg />");
        executor.write_repo_file(
            "src/backend/service.rs",
            r#"
pub struct Engine;

pub fn render_service() -> Engine {
    Engine
}
"#,
        );
        executor.write_repo_file(
            "web/panel.tsx",
            r#"
export function Panel() {
    return <div>panel</div>;
}
"#,
        );

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(
                executor.repo_path(),
                &["icons/logo.svg", "src/backend/service.rs", "web/panel.tsx"],
            ),
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(!architecture_map.contains("icons/logo.svg"));
        let backend_pos = architecture_map.find("[src/backend]").unwrap();
        let ui_pos = architecture_map.find("[web]").unwrap();
        assert!(backend_pos < ui_pos, "backend deve vir antes de ui: {}", architecture_map);
        assert!(architecture_map.contains("src/backend/service.rs"));
        assert!(architecture_map.contains("web/panel.tsx"));
    }

    #[tokio::test]
    async fn test_architecture_map_keeps_backend_visible_amid_tests_examples_and_fixtures() {
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file(
            "crates/goose/tests/session_id_propagation_test.rs",
            "pub fn ignored_test_helper() {}",
        );
        executor.write_repo_file("examples/demo/main.rs", "fn main() {}");
        executor.write_repo_file("src/backend/fixtures/sample.rs", "pub fn fixture_only() {}");
        executor.write_repo_file("src/backend/test_support/helpers.rs", "pub fn helper() {}");
        executor.write_repo_file("src/backend/e2e/flow.rs", "pub fn flow() {}");
        executor.write_repo_file(
            "src/backend/service.rs",
            r#"
pub struct Engine;

pub fn run(_engine: Engine) {}
"#,
        );

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(
                executor.repo_path(),
                &[
                    "crates/goose/tests/session_id_propagation_test.rs",
                    "examples/demo/main.rs",
                    "src/backend/fixtures/sample.rs",
                    "src/backend/test_support/helpers.rs",
                    "src/backend/e2e/flow.rs",
                    "src/backend/service.rs",
                ],
            ),
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(architecture_map.contains("[src/backend]"));
        assert!(architecture_map.contains("src/backend/service.rs"));
    }

    #[tokio::test]
    async fn test_architecture_map_keeps_backend_visible_amid_scenarios_docs_ui_and_bench_noise() {
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file(
            "crates/goose-cli/src/scenario_tests/message_generator.rs",
            "pub fn scenario_noise() {}",
        );
        executor.write_repo_file(
            "documentation/src/pages/index.tsx",
            "export function DocsPage() { return <div />; }",
        );
        executor.write_repo_file(
            "ui/desktop/src/App.tsx",
            "export function App() { return <main />; }",
        );
        executor.write_repo_file("oidc-proxy/test/index.test.js", "export function worker() {}");
        executor.write_repo_file(
            "evals/open-model-gym/suite/src/runner.ts",
            "export function runScenario() {}",
        );
        executor.write_repo_file("crates/goose/benches/parser.rs", "pub fn bench_parser() {}");
        executor.write_repo_file(
            "src/backend/engine.rs",
            r#"
pub struct Runtime;

pub fn boot(_runtime: Runtime) {}
"#,
        );

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(
                executor.repo_path(),
                &[
                    "crates/goose-cli/src/scenario_tests/message_generator.rs",
                    "documentation/src/pages/index.tsx",
                    "ui/desktop/src/App.tsx",
                    "oidc-proxy/test/index.test.js",
                    "evals/open-model-gym/suite/src/runner.ts",
                    "crates/goose/benches/parser.rs",
                    "src/backend/engine.rs",
                ],
            ),
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(architecture_map.contains("[src/backend]"));
        assert!(architecture_map.contains("src/backend/engine.rs"));
    }

    #[tokio::test]
    async fn test_binary_not_found() {
        let spawn_err = SandboxError::ProcessSpawnFailed {
            reason: "program not found (os error 2)".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(outline.contains("kind: ContentRepo"), "Outline deveria cair no modo ContentRepo");
        assert!(outline.contains("no source files found"), "Outline deveria registrar a causa estrutural");
        assert!(health.contains("# Health Report"));
        assert!(health.contains("kind: ContentRepo"));
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
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(outline.contains("kind: ContentRepo"), "Outline deveria cair no modo ContentRepo");
        assert!(outline.contains("no source files found"), "Outline deveria registrar a causa estrutural");
        assert!(health.contains("# Health Report"));
        assert!(health.contains("kind: ContentRepo"));
    }

    #[tokio::test]
    async fn test_timeout_propagation() {
        let executor = MockExecutor::new(vec![Err(SandboxError::Timeout)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 45,
            clean_files: Arc::new(Vec::new()),
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
            clean_files: Arc::new(Vec::new()),
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
            clean_files: Arc::new(Vec::new()),
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
            clean_files: Arc::new(Vec::new()),
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
            clean_files: Arc::new(Vec::new()),
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
            clean_files: Arc::new(Vec::new()),
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
            clean_files: Arc::new(Vec::new()),
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

        assert_eq!(payload.runner_name, "static-ast-radar");
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
    async fn test_native_test_discovery_detects_go_python_elixir_and_frontend_intent() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("go")).unwrap();
        std::fs::create_dir_all(dir.path().join("python")).unwrap();
        std::fs::create_dir_all(dir.path().join("elixir")).unwrap();
        std::fs::create_dir_all(dir.path().join("web")).unwrap();

        std::fs::write(
            dir.path().join("go/math_test.go"),
            r#"
package demo

import "testing"

func TestSum(t *testing.T) {
    t.Run("adds positives", func(t *testing.T) {})
}
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("python/test_api.py"),
            r#"
def helper():
    return 1

async def test_async_healthcheck():
    assert True

def test_sync_healthcheck():
    assert True
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("elixir/user_test.exs"),
            r#"
defmodule Demo.UserTest do
  use ExUnit.Case

  describe "create_user/1" do
    test "persists valid payload" do
      assert true
    end
  end
end
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("web/login.spec.ts"),
            r#"
describe("login flow", () => {
  it("renders button", () => {});
  test.skip("shows errors", () => {});
});
"#,
        )
        .unwrap();

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &StackProfile::Mixed(vec![
                SingleStack::Go,
                SingleStack::Python,
                SingleStack::Elixir,
                SingleStack::NodeJS,
            ]),
        })
        .await
        .unwrap();

        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "go/math_test.go"
                && block.items.contains(&"func TestSum(t *testing.T)".to_string())
                && block.items.contains(&r#"subtest "adds positives""#.to_string())));
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "python/test_api.py"
                && block.items.contains(&"def test_async_healthcheck".to_string())
                && block.items.contains(&"def test_sync_healthcheck".to_string())));
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "elixir/user_test.exs"
                && block.items.contains(&r#"describe "create_user/1""#.to_string())
                && block.items.contains(&r#"test "persists valid payload""#.to_string())));
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "web/login.spec.ts"
                && block.items.contains(&r#"describe "login flow""#.to_string())
                && block.items.contains(&r#"it "renders button""#.to_string())
                && block.items.contains(&r#"test "shows errors""#.to_string())));
    }

    #[tokio::test]
    async fn test_native_test_discovery_detects_inline_go_tests_outside_test_dirs() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("pkg")).unwrap();
        std::fs::write(
            dir.path().join("pkg/smoke.go"),
            r#"
package demo

import "testing"

func helper() {}

func TestSmokePath(t *testing.T) {
    if t == nil {
        panic("unreachable")
    }
}
"#,
        )
        .unwrap();

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &StackProfile::Go,
        })
        .await
        .unwrap();

        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "pkg/smoke.go"
                && block.items.contains(&"func TestSmokePath(t *testing.T)".to_string())
                && !block.items.iter().any(|item| item.contains("panic"))));
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

        let normalized = normalize_sast_output(
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
                && is_unsafe_hotspot(issue)
        }));
        assert!(normalized.iter().any(|issue| {
            issue.file == "src/ui.ts"
                && issue.message.contains("nested-ternary")
                && !is_unsafe_hotspot(issue)
        }));
    }

    #[tokio::test]
    async fn test_polyglot_sast_sidecar_routes_rust_and_cpp_and_breaks_blob06_from_blob08() {
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
            clean_files: test_clean_files(executor.repo_path(), &["Cargo.toml"]),
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
        assert!(!health_blob.contains("[opengrep]"));
        assert!(!health_blob.contains("README.md"));
        assert!(!health_blob.contains("execution failed"));
        assert!(!health_blob.contains("normalization failed"));
        assert!(!health_blob.contains("\"router\""));
        assert!(!health_blob.contains("\"schema\""));
    }

    #[test]
    fn test_cppcheck_blade_enforces_xml_v2_args() {
        let (binary, args) = blade_command(StaticAnalysisBlade::Cppcheck, &[".".to_string()], None);
        assert_eq!(binary, "cppcheck");
        assert!(args.iter().any(|arg| arg == "--xml"));
        assert!(args.iter().any(|arg| arg == "--xml-version=2"));
        assert!(!args.iter().any(|arg| arg.starts_with("--error-exitcode")));
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
    fn test_normalize_cppcheck_output_accepts_empty_results_as_clean_info() {
        let repo_path = Path::new("C:/repos/example");
        let payload = r#"<?xml version="1.0"?><results></results>"#;

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, "info");
        assert_eq!(issues[0].file, "");
        assert_eq!(
            issues[0].message,
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck."
        );
        assert_eq!(issues[0].source_blade, "cppcheck");
    }

    #[test]
    fn test_normalize_cppcheck_output_falls_back_to_unstructured_text_without_crashing() {
        let repo_path = Path::new("C:/repos/example");
        let payload = "cppcheck: progress 100%\nmain.c:42: warning: suspicious arithmetic";

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, "warning");
        assert_eq!(issues[0].source_blade, "cppcheck");
        assert!(issues[0]
            .message
            .contains("cppcheck output nao estruturado preservado"));
    }

    #[test]
    fn test_normalize_cppcheck_output_accepts_self_closing_results_as_clean_info() {
        let repo_path = Path::new("C:/repos/example");
        let payload = r#"<?xml version="1.0"?><results/>"#;

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, "info");
        assert_eq!(
            issues[0].message,
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck."
        );
        assert_eq!(issues[0].source_blade, "cppcheck");
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
    fn test_classify_sidecar_observability_distinguishes_ok_info_and_lethal() {
        assert_eq!(
            classify_sidecar_observability(0, b"{}"),
            SidecarObservabilityClass::Ok
        );
        assert_eq!(
            classify_sidecar_observability(1, br#"{"diagnostics":[]}"#),
            SidecarObservabilityClass::InformationalNonZero
        );
        assert_eq!(
            classify_sidecar_observability(101, b""),
            SidecarObservabilityClass::LethalNonZero
        );
    }

    #[test]
    fn test_normalize_bandit_output_drops_assert_noise_even_if_tool_leaks_it() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("src/app.py", "def run():\n    return 1\n");
        let payload = br#"{
            "results": [
                {
                    "filename": "tests/test_app.py",
                    "issue_severity": "LOW",
                    "issue_text": "Use of assert detected.",
                    "line_number": 7
                },
                {
                    "filename": "src/app.py",
                    "issue_severity": "HIGH",
                    "issue_text": "Potential shell injection via subprocess",
                    "line_number": 12
                }
            ]
        }"#;

        let issues = normalize_sast_output(
            executor.repo_path(),
            executor.repo_path(),
            StaticAnalysisBlade::Bandit,
            payload,
        )
        .unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].file, "src/app.py");
        assert!(issues[0].message.contains("Potential shell injection"));
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
        let (binary, args) = blade_command(StaticAnalysisBlade::Sobelow, &[".".to_string()], None);
        assert_eq!(binary, "mix");
        assert_eq!(args, vec!["sobelow", "--format", "json", "--private", "--root", "."]);
    }

    #[test]
    fn test_biome_args_accept_explicit_scan_targets_with_health_profile() {
        let scan_targets = vec!["src/index.ts".to_string(), "src/server.ts".to_string()];
        let (biome_binary, biome_args) =
            blade_command(StaticAnalysisBlade::Biome, &scan_targets, None);

        assert_eq!(biome_binary, "biome");
        assert_eq!(biome_args.first().map(String::as_str), Some("lint"));
        assert!(!biome_args.iter().any(|arg| arg == "check"));
        assert!(biome_args.iter().any(|arg| arg == "--no-errors-on-unmatched"));
        assert!(biome_args.iter().any(|arg| arg == "--only=lint/complexity"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/security"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/suspicious"));
        assert!(biome_args.ends_with(&scan_targets));
    }

    #[test]
    fn test_biome_args_use_security_only_for_blob06_profile() {
        let scan_targets = vec!["src/index.ts".to_string()];
        let biome_args = biome_args_for_profile(&scan_targets, JsLintProfile::UnsafeHotspot);

        assert!(biome_args.iter().any(|arg| arg == "--only=lint/security"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/complexity"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/suspicious"));
        assert!(biome_args.ends_with(&scan_targets));
    }

    #[test]
    fn test_cppcheck_args_are_security_scoped() {
        let (binary, args) = blade_command(StaticAnalysisBlade::Cppcheck, &[".".to_string()], None);
        assert_eq!(binary, "cppcheck");
        assert!(args.iter().any(|arg| arg == "--enable=warning"));
        assert!(args
            .iter()
            .any(|arg| arg == "--disable=style,performance,portability,information"));
        assert!(!args.iter().any(|arg| arg == "--enable=all"));
    }

    #[test]
    fn test_python_linter_args_accept_explicit_scan_targets_and_skip_bandit_b101() {
        let scan_targets = vec!["src/app.py".to_string(), "services/api.py".to_string()];
        let (ruff_binary, ruff_args) = blade_command(StaticAnalysisBlade::Ruff, &scan_targets, None);
        let (bandit_binary, bandit_args) =
            blade_command(StaticAnalysisBlade::Bandit, &scan_targets, None);

        assert_eq!(ruff_binary, "ruff");
        assert_eq!(bandit_binary, "bandit");
        assert_eq!(ruff_args[..3], ["check", "--output-format", "json"]);
        assert!(ruff_args.windows(2).any(|pair| pair == ["--ignore", "D,F401,UP,W"]));
        assert!(ruff_args.ends_with(&scan_targets));
        assert!(bandit_args.windows(2).any(|pair| pair == ["-s", "B101"]));
        assert!(bandit_args.ends_with(&scan_targets));
        assert!(!bandit_args.iter().any(|arg| arg == "-r"));
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
    fn test_derive_rust_clippy_targets_skip_toxic_manifests_and_scope_to_package() {
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
        let manifests = discover_monorepo_manifests(executor.repo_path());

        let targets = derive_rust_clippy_execution_targets(&manifests);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].scope, ".");
        assert_eq!(
            targets[0].command_args.as_ref(),
            Some(&clippy_args_for_package("root"))
        );
    }

    #[test]
    fn test_blade_parallelism_limit_serializes_rust_clippy_only() {
        assert_eq!(
            blade_parallelism_limit(StaticAnalysisBlade::RustClippy),
            RUST_CLIPPY_MAX_PARALLEL
        );
        assert_eq!(
            blade_parallelism_limit(StaticAnalysisBlade::Cppcheck),
            MONOREPO_SAST_MAX_PARALLEL
        );
        assert_eq!(
            blade_parallelism_limit(StaticAnalysisBlade::Opengrep),
            MONOREPO_SAST_MAX_PARALLEL
        );
    }

    #[test]
    fn test_derive_go_execution_targets_anchor_govulncheck_to_real_modules_only() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("go.mod", "module root.example\n\ngo 1.22\n");
        executor.write_repo_file("README.md", "docs only\n");
        executor.write_repo_file("services/api/go.mod", "module api.example\n\ngo 1.22\n");
        executor.write_repo_file("services/api/cmd/api/main.go", "package main\nfunc main() {}\n");
        executor.write_repo_file("tools/empty/go.mod", "module empty.example\n\ngo 1.22\n");
        let clean_files = vec![
            canonicalize_or_self(executor.repo_path().join("README.md")),
            canonicalize_or_self(executor.repo_path().join("services/api/cmd/api/main.go")),
        ];

        let manifests = discover_monorepo_manifests(executor.repo_path());
        let targets = derive_go_execution_targets(&manifests, &clean_files);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].scope, "services/api");
        assert_eq!(targets[0].scan_targets, vec!["./...".to_string()]);
        assert_eq!(
            targets[0].command_args.as_ref(),
            Some(&govulncheck_args_for_module())
        );
    }

    #[test]
    fn test_derive_elixir_execution_targets_set_explicit_root_and_skip_empty_apps() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("mix.exs", "defmodule Root.MixProject do end\n");
        executor.write_repo_file("apps/web/mix.exs", "defmodule Web.MixProject do end\n");
        executor.write_repo_file("apps/web/lib/web/router.ex", "defmodule Web.Router do end\n");
        let clean_files = vec![canonicalize_or_self(
            executor.repo_path().join("apps/web/lib/web/router.ex"),
        )];

        let manifests = discover_monorepo_manifests(executor.repo_path());
        let targets = derive_elixir_execution_targets(&manifests, &clean_files);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].scope, "apps/web");
        assert_eq!(targets[0].scan_targets, vec![".".to_string()]);
        assert_eq!(
            targets[0].command_args.as_ref(),
            Some(&sobelow_args_for_root("."))
        );
    }

    #[test]
    fn test_derive_python_execution_targets_use_clean_files_instead_of_repo_root() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("pyproject.toml", "[project]\nname='demo'\nversion='0.1.0'\n");
        executor.write_repo_file("src/app.py", "def run():\n    return 1\n");
        executor.write_repo_file("tests/test_app.py", "def test_run():\n    assert True\n");
        executor.write_repo_file("dist/generated.py", "def noise():\n    return 0\n");
        executor.write_repo_file("build/generated.py", "def noise():\n    return 0\n");
        executor.write_repo_file("node_modules/pkg/index.py", "def noise():\n    return 0\n");

        let clean_files = vec![
            canonicalize_or_self(executor.repo_path().join("src/app.py")),
            canonicalize_or_self(executor.repo_path().join("tests/test_app.py")),
            canonicalize_or_self(executor.repo_path().join("dist/generated.py")),
            canonicalize_or_self(executor.repo_path().join("build/generated.py")),
            canonicalize_or_self(executor.repo_path().join("node_modules/pkg/index.py")),
        ];

        let ruff_targets =
            derive_python_execution_targets(executor.repo_path(), StaticAnalysisBlade::Ruff, &clean_files);
        let bandit_targets =
            derive_python_execution_targets(executor.repo_path(), StaticAnalysisBlade::Bandit, &clean_files);

        assert_eq!(ruff_targets.len(), 1);
        assert_eq!(bandit_targets.len(), 1);
        assert_eq!(ruff_targets[0].scope, ".::files-01");
        assert_eq!(ruff_targets[0].scan_targets, vec!["src/app.py".to_string()]);
        assert_eq!(bandit_targets[0].scan_targets, vec!["src/app.py".to_string()]);
    }

    #[test]
    fn test_derive_cppcheck_execution_targets_requires_productive_cpp_files() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("src/main.c", "int main() { return 0; }\n");
        executor.write_repo_file("tests/main_test.c", "int main() { return 1; }\n");
        executor.write_repo_file("pkg/service.go", "package service\n");

        let clean_files = vec![
            canonicalize_or_self(executor.repo_path().join("src/main.c")),
            canonicalize_or_self(executor.repo_path().join("tests/main_test.c")),
            canonicalize_or_self(executor.repo_path().join("pkg/service.go")),
        ];

        let targets = derive_cppcheck_execution_targets(executor.repo_path(), &clean_files);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].scan_targets, vec!["src/main.c".to_string()]);
        assert_eq!(
            targets[0].command_args.as_ref(),
            Some(&cppcheck_args_for_targets(&["src/main.c".to_string()]))
        );
    }

    #[test]
    fn test_derive_opengrep_execution_targets_uses_scoped_ast_roots() {
        let executor = MockExecutor::new(Vec::new());
        let mut clean_files = Vec::new();
        for idx in 0..90 {
            let alpha = format!("packages/web/src/compiler/phases/1-parse/file_{idx}.ts");
            executor.write_repo_file(&alpha, "export const alpha = 1;\n");
            clean_files.push(canonicalize_or_self(executor.repo_path().join(&alpha)));

            let beta = format!("packages/web/src/compiler/phases/2-analyze/file_{idx}.ts");
            executor.write_repo_file(&beta, "export const beta = 2;\n");
            clean_files.push(canonicalize_or_self(executor.repo_path().join(&beta)));
        }
        executor.write_repo_file("packages/web/tests/samples/case.ts", "export const noisy = 1;\n");
        executor.write_repo_file("playgrounds/sandbox/src/main.ts", "export const preview = 1;\n");

        let targets = derive_opengrep_execution_targets(executor.repo_path(), &clean_files);
        let all_targets = targets
            .iter()
            .flat_map(|target| target.scan_targets.iter())
            .cloned()
            .collect::<Vec<_>>();

        assert!(targets.iter().all(|target| {
            target.scope.starts_with(".::unsafe::files-") || target.scope.starts_with(".::health::files-")
        }));
        assert!(targets
            .iter()
            .any(|target| target.forced_channel == Some(SastIssueChannel::UnsafeHotspot)));
        assert!(targets
            .iter()
            .any(|target| target.forced_channel == Some(SastIssueChannel::Health)));
        assert!(all_targets.iter().any(|target| target.contains("packages/web/src/compiler/phases/1-parse/file_0.ts")));
        assert!(all_targets.iter().any(|target| target.contains("packages/web/src/compiler/phases/2-analyze/file_0.ts")));
        assert!(!all_targets.iter().any(|target| target.contains("packages/web/tests/")));
        assert!(!all_targets.iter().any(|target| target.contains("playgrounds/")));
    }

    #[test]
    fn test_derive_opengrep_execution_targets_batches_direct_files_without_parent_scope() {
        let executor = MockExecutor::new(Vec::new());
        let mut clean_files = Vec::new();
        for idx in 0..90 {
            let rel = format!("apps/api/src/controllers/file_{idx}.ts");
            executor.write_repo_file(&rel, "export const controller = 1;\n");
            clean_files.push(canonicalize_or_self(executor.repo_path().join(&rel)));
        }
        executor.write_repo_file("apps/api/src/index.ts", "export const index = 1;\n");
        executor.write_repo_file("apps/api/src/server.ts", "export const server = 1;\n");
        clean_files.push(canonicalize_or_self(executor.repo_path().join("apps/api/src/index.ts")));
        clean_files.push(canonicalize_or_self(
            executor.repo_path().join("apps/api/src/server.ts"),
        ));

        let targets = derive_opengrep_execution_targets(executor.repo_path(), &clean_files);
        let all_targets = targets
            .iter()
            .flat_map(|target| target.scan_targets.iter())
            .cloned()
            .collect::<Vec<_>>();

        assert!(targets
            .iter()
            .any(|target| target.scope.starts_with(".::unsafe::files-")
                && target.forced_channel == Some(SastIssueChannel::UnsafeHotspot)));
        assert!(targets
            .iter()
            .any(|target| target.scope.starts_with(".::health::files-")
                && target.forced_channel == Some(SastIssueChannel::Health)));
        assert!(all_targets.iter().any(|target| target == "apps/api/src/index.ts"));
        assert!(all_targets.iter().any(|target| target == "apps/api/src/server.ts"));
        assert!(all_targets.iter().any(|target| target == "apps/api/src/controllers/file_0.ts"));
    }

    #[test]
    fn test_derive_js_lint_targets_split_root_package_without_reopening_subpackages() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("package.json", r#"{"name":"root"}"#);
        executor.write_repo_file("scripts/build.ts", "export const build = 1;\n");
        executor.write_repo_file("packages/web/package.json", r#"{"name":"web"}"#);
        let mut clean_files = vec![
            canonicalize_or_self(executor.repo_path().join("package.json")),
            canonicalize_or_self(executor.repo_path().join("scripts/build.ts")),
            canonicalize_or_self(executor.repo_path().join("packages/web/package.json")),
        ];
        for idx in 0..90 {
            let rel = format!("packages/web/src/compiler/file_{idx}.ts");
            executor.write_repo_file(&rel, "export const web = 1;\n");
            clean_files.push(canonicalize_or_self(executor.repo_path().join(&rel)));
        }

        let manifests = discover_monorepo_manifests(executor.repo_path());
        let targets = derive_js_lint_execution_targets(
            executor.repo_path(),
            &manifests,
            StaticAnalysisBlade::Biome,
            &clean_files,
        );
        let root_targets = targets
            .iter()
            .filter(|target| target.scope.starts_with(".::"))
            .flat_map(|target| target.scan_targets.iter())
            .cloned()
            .collect::<Vec<_>>();
        let web_targets = targets
            .iter()
            .filter(|target| target.scope.starts_with("packages/web::"))
            .flat_map(|target| target.scan_targets.iter())
            .cloned()
            .collect::<Vec<_>>();

        assert!(targets.iter().any(|target| {
            target.scope.starts_with(".::unsafe::files-")
                && target.forced_channel == Some(SastIssueChannel::UnsafeHotspot)
        }));
        assert!(targets.iter().any(|target| {
            target.scope.starts_with(".::health::files-")
                && target.forced_channel == Some(SastIssueChannel::Health)
        }));
        assert!(root_targets.iter().any(|target| target == "scripts/build.ts"));
        assert!(web_targets.iter().any(|target| target == "src/compiler/file_0.ts"));
    }

    #[test]
    fn test_derive_js_lint_targets_batch_direct_files_for_nested_package_scope() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("apps/api/package.json", r#"{"name":"api"}"#);
        let mut clean_files = vec![canonicalize_or_self(
            executor.repo_path().join("apps/api/package.json"),
        )];
        for idx in 0..90 {
            let rel = format!("apps/api/src/controllers/file_{idx}.ts");
            executor.write_repo_file(&rel, "export const controller = 1;\n");
            clean_files.push(canonicalize_or_self(executor.repo_path().join(&rel)));
        }
        executor.write_repo_file("apps/api/src/index.ts", "export const index = 1;\n");
        executor.write_repo_file("apps/api/src/server.ts", "export const server = 1;\n");
        clean_files.push(canonicalize_or_self(executor.repo_path().join("apps/api/src/index.ts")));
        clean_files.push(canonicalize_or_self(
            executor.repo_path().join("apps/api/src/server.ts"),
        ));

        let manifests = discover_monorepo_manifests(executor.repo_path());
        let targets = derive_js_lint_execution_targets(
            executor.repo_path(),
            &manifests,
            StaticAnalysisBlade::Biome,
            &clean_files,
        );
        let api_targets = targets
            .iter()
            .filter(|target| target.scope.starts_with("apps/api::"))
            .flat_map(|target| target.scan_targets.iter())
            .cloned()
            .collect::<Vec<_>>();

        assert!(targets.iter().any(|target| {
            target.scope.starts_with("apps/api::unsafe::files-")
                && target.forced_channel == Some(SastIssueChannel::UnsafeHotspot)
        }));
        assert!(targets.iter().any(|target| {
            target.scope.starts_with("apps/api::health::files-")
                && target.forced_channel == Some(SastIssueChannel::Health)
        }));
        assert!(api_targets.iter().any(|target| target == "src/index.ts"));
        assert!(api_targets.iter().any(|target| target == "src/server.ts"));
        assert!(api_targets.iter().any(|target| target == "src/controllers/file_0.ts"));
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
        assert!(calls[1].starts_with("cargo metadata --format-version 1 --locked --offline --manifest-path "));
        assert!(calls[2].starts_with("cargo clippy --message-format=json --frozen -p sdk -- --no-deps"));
        assert!(!cache_root.exists());
        assert!(!cargo_home.exists());
    }

    #[tokio::test]
    async fn test_run_sast_blade_falls_back_to_opengrep_when_transitive_manifest_declares_build_dependencies() {
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
        let opengrep_payload = br#"{
            "results": [
                {
                    "check_id": "soda.fragility.unwrap",
                    "path": "src/lib.rs",
                    "extra": {
                        "message": "unwrap encontrado em caminho critico",
                        "severity": "WARNING"
                    }
                }
            ]
        }"#;
        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Ok(Vec::new()),
            Ok(metadata_payload.as_bytes().to_vec()),
            Ok(opengrep_payload.to_vec()),
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
        .await
        .unwrap();

        let calls = executor.calls();
        assert_eq!(result.effective_blade, StaticAnalysisBlade::Opengrep);
        assert_eq!(result.bytes, opengrep_payload.to_vec());
        assert_eq!(calls.len(), 3);
        assert!(calls[0].starts_with("cargo fetch --manifest-path "));
        assert!(!calls[0].contains("--locked"));
        assert!(calls[1].starts_with("cargo metadata --format-version 1 --offline --manifest-path "));
        assert!(!calls[1].contains("--locked"));
        assert!(calls[2].starts_with("opengrep "));
        assert!(!calls[2].contains("cargo clippy"));
    }

    #[tokio::test]
    async fn test_run_sast_blade_skips_opengrep_fallback_when_global_coverage_exists() {
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
        *executor.responses.lock().unwrap() = std::collections::VecDeque::from(vec![
            Ok(Vec::new()),
            Ok(metadata_payload.as_bytes().to_vec()),
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
            true,
        )
        .await
        .unwrap();

        let calls = executor.calls();
        assert_eq!(result.effective_blade, StaticAnalysisBlade::RustClippy);
        assert!(result.bytes.is_empty());
        assert_eq!(calls.len(), 2);
        assert!(calls[0].starts_with("cargo fetch --manifest-path "));
        assert!(!calls[0].contains("--locked"));
        assert!(calls[1].starts_with("cargo metadata --format-version 1 --offline --manifest-path "));
        assert!(!calls[1].contains("--locked"));
    }
}
