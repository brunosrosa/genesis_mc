use oxc::{
    allocator::Allocator,
    ast::ast::{FormalParameter, TSInterfaceDeclaration, TSTypeAliasDeclaration, VariableDeclarator},
    ast_visit::{walk, Visit},
    parser::{ParseOptions, Parser},
    span::{GetSpan, SourceType, Span},
};
use regex::Regex;
use tokio::fs;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use crate::harvester::PHASE1_HEAVY_BLOB_MAX_CHARS;
use super::detect::StackProfile;
use super::git::RepoPath;
use super::persist::ArtifactBlob;
use super::sidecar::{pack_scoped_text_blocks, NativeTestDiscoveryInput, NativeTestDiscoverySidecar, ScopedTextBlock};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::{info, warn};

/// Tamanho máximo permitido para um arquivo de manifesto (1 MiB).
const MAX_MANIFEST_SIZE: u64 = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestPayload {
    pub manifests: Vec<ManifestInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestInfo {
    pub file_name: String,
    pub dependencies: Vec<DependencyEntry>,
    pub dev_dependencies: Vec<DependencyEntry>,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyEntry {
    pub name: String,
    pub version_spec: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpsPayload {
    pub infra_files: Vec<InfraFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InfraFile {
    pub path: String,
    pub content: String,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ExtractionError {
    #[error("No manifest files found in repository root")]
    NotFound,

    #[error("Required artifact '{artifact_type}' not found. Expected one of: {candidates}")]
    RequiredArtifactMissing {
        artifact_type: String,
        candidates: String,
    },

    #[error("Required artifact '{artifact_type}' from '{file}' is empty")]
    EmptyArtifact {
        artifact_type: String,
        file: String,
    },

    #[error("Failed to parse manifest '{file}': {reason}")]
    ParseError { file: String, reason: String },

    #[error("Manifest file exceeds size limit ({size_bytes} bytes > {limit_bytes} bytes): {file}")]
    FileTooLarge {
        file: String,
        size_bytes: u64,
        limit_bytes: u64,
    },

    #[error("Filesystem error reading '{file}': {reason}")]
    IoError { file: String, reason: String },
}

pub struct ManifestInput<'a> {
    pub repo_path: &'a RepoPath,
}

pub struct TestIntentInput<'a> {
    pub repo_path: &'a RepoPath,
    pub profile: &'a StackProfile,
}

pub struct ManifestExtractor;
pub struct DomainMechanicsExtractor;
pub struct TestIntentExtractor;
pub struct UnsafeHotspotsExtractor;
pub struct UxContractsExtractor;

pub struct OpsInput<'a> {
    pub repo_path: &'a RepoPath,
}

pub struct OpsBlueprintExtractor;
pub struct LocalStaticExtractor;

const README_MAX_CHARS: usize = 8_000;
const README_MISSING_COGNITIVE_DIRECTIVE: &str = "[DIRETIVA SODA COGNITIVA: DOCUMENTAÇÃO RAIZ NÃO ENCONTRADA. O repositório não possui um arquivo README padrão na raiz (.md, .rst, .txt). Lentes de Avaliação: penalizem a experiência de onboarding e documentação (Lente A e Lente C), mas prossigam com a análise arquitetural utilizando apenas a AST e os manifestos.]";
const MANIFEST_BLOB_MAX_CHARS: usize = 3_000;
const OPS_BLOB_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const COMMUNITY_META_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const TEST_INTENT_BLOB_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const UNSAFE_HOTSPOTS_BLOB_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const UX_CONTRACTS_BLOB_MAX_CHARS: usize = PHASE1_HEAVY_BLOB_MAX_CHARS;
const MAX_SCAN_FILE_BYTES: u64 = 262_144;
const STATE_CALL_NAMES: [&str; 5] = ["useState", "createSignal", "writable", "useReducer", "$state"];
const README_MAX_ALLOWED_SECTIONS: usize = 3;
const MANIFEST_MAX_DEPENDENCIES_PER_FILE: usize = 24;
const MANIFEST_DEPENDENCY_CHUNK_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CappedFileEntries {
    items: Vec<String>,
    omitted_count: usize,
}

#[derive(Debug, Default)]
struct UxAstCollector<'a> {
    source_text: &'a str,
    entries: Vec<String>,
    seen: BTreeSet<String>,
}

impl<'a> UxAstCollector<'a> {
    fn new(source_text: &'a str) -> Self {
        Self {
            source_text,
            entries: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    fn finish(self) -> Vec<String> {
        self.entries
    }

    fn push_props_parameter(&mut self, parameter: &FormalParameter<'a>) {
        let Some(snippet) = compact_props_parameter_entry(self.source_text, parameter)
        else {
            return;
        };
        self.push_entry(snippet);
    }

    fn push_entry(&mut self, entry: String) {
        if self.seen.insert(entry.clone()) {
            self.entries.push(entry);
        }
    }
}

impl<'a> Visit<'a> for UxAstCollector<'a> {
    fn visit_ts_interface_declaration(&mut self, declaration: &TSInterfaceDeclaration<'a>) {
        if is_ux_contract_type_name(declaration.id.name.as_str()) {
            self.push_entry(format!("interface {}", declaration.id.name.as_str()));
        }
        walk::walk_ts_interface_declaration(self, declaration);
    }

    fn visit_ts_type_alias_declaration(&mut self, declaration: &TSTypeAliasDeclaration<'a>) {
        if is_ux_contract_type_name(declaration.id.name.as_str()) {
            self.push_entry(format!("type {}", declaration.id.name.as_str()));
        }
        walk::walk_ts_type_alias_declaration(self, declaration);
    }

    fn visit_formal_parameter(&mut self, parameter: &FormalParameter<'a>) {
        self.push_props_parameter(parameter);
        walk::walk_formal_parameter(self, parameter);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(entry) = compact_state_entry(self.source_text, declarator) {
            self.push_entry(entry);
        } else if let Some(entry) = compact_props_binding_entry(self.source_text, declarator) {
            self.push_entry(entry);
        }

        walk::walk_variable_declarator(self, declarator);
    }
}

impl LocalStaticExtractor {
    pub async fn extract_all(repo_path: &Path) -> Result<Vec<ArtifactBlob>, ExtractionError> {
        let mut blobs = Vec::new();

        blobs.push(Self::extract_optional_blob(
            repo_path,
            &[
                "README.md",
                "README.rst",
                "README.txt",
                "README",
                "readme.md",
                "readme.rst",
                "readme.txt",
                "readme",
            ],
            "blob_01_promessa_readme",
            README_MAX_CHARS,
            README_MISSING_COGNITIVE_DIRECTIVE,
        )
        .await);

        Ok(blobs)
    }

    async fn extract_blob(
        repo_path: &Path,
        candidates: &[&str],
        artifact_type: &str,
        max_chars: usize,
    ) -> Result<ArtifactBlob, ExtractionError> {
        let mut case_insensitive = HashMap::<String, PathBuf>::new();
        let mut entries = fs::read_dir(repo_path)
            .await
            .map_err(|e| ExtractionError::IoError {
                file: repo_path.display().to_string(),
                reason: e.to_string(),
            })?;
        while let Some(entry) = entries.next_entry().await.map_err(|e| ExtractionError::IoError {
            file: repo_path.display().to_string(),
            reason: e.to_string(),
        })? {
            let file_type = entry.file_type().await.map_err(|e| ExtractionError::IoError {
                file: repo_path.display().to_string(),
                reason: e.to_string(),
            })?;
            if !file_type.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            case_insensitive.insert(name.to_ascii_lowercase(), entry.path());
        }

        for candidate in candidates {
            let preferred = repo_path.join(candidate);
            let metadata = fs::metadata(&preferred).await.ok();
            let path = if metadata.as_ref().is_some_and(|m| m.is_file()) {
                preferred
            } else if let Some(found) = case_insensitive.get(&candidate.to_ascii_lowercase()) {
                found.clone()
            } else {
                warn!(
                    artifact_type,
                    candidate,
                    repo_root = %repo_path.display(),
                    "Arquivo candidato nao encontrado para artefato"
                );
                continue;
            };

            info!(
                artifact_type,
                candidate,
                abs_path = %path.display(),
                "Tentando ler arquivo para artefato"
            );

            match fs::read_to_string(&path).await {
                Ok(content) => {
                    if content.trim().is_empty() {
                        return Err(ExtractionError::EmptyArtifact {
                            artifact_type: artifact_type.to_string(),
                            file: path.display().to_string(),
                        });
                    }
                    let sanitized = if artifact_type == "blob_01_promessa_readme" {
                        sanitize_readme_blob(&content)
                    } else {
                        content
                    };
                    let truncated = truncate_chars(&sanitized, max_chars);
                    return Ok(ArtifactBlob {
                        artifact_type: artifact_type.to_string(),
                        payload_blob: truncated.into_bytes(),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    warn!(
                        artifact_type,
                        candidate,
                        abs_path = %path.display(),
                        error = %e,
                        "Falha ao ler arquivo (not found)"
                    );
                    continue;
                }
                Err(e) => {
                    warn!(
                        artifact_type,
                        candidate,
                        abs_path = %path.display(),
                        error = %e,
                        "Falha ao ler arquivo"
                    );
                    return Err(ExtractionError::IoError {
                        file: path.display().to_string(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        Err(ExtractionError::RequiredArtifactMissing {
            artifact_type: artifact_type.to_string(),
            candidates: candidates.join(", "),
        })
    }

    async fn extract_optional_blob(
        repo_path: &Path,
        candidates: &[&str],
        artifact_type: &str,
        max_chars: usize,
        missing_directive: &str,
    ) -> ArtifactBlob {
        match Self::extract_blob(repo_path, candidates, artifact_type, max_chars).await {
            Ok(blob) => blob,
            Err(ExtractionError::RequiredArtifactMissing { .. }) | Err(ExtractionError::NotFound) => {
                blob_from_text(artifact_type, missing_directive.to_string())
            }
            Err(e) => {
                warn!(
                    artifact_type,
                    error = %e,
                    repo_root = %repo_path.display(),
                    "Falha ao extrair artefato opcional; seguindo com fallback cognitivo"
                );
                blob_from_text(artifact_type, missing_directive.to_string())
            }
        }
    }
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
}

fn html_anchor_image_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r#"(?is)<a\b[^>]*>\s*<img\b[^>]*>\s*</a>"#).ok())
        .as_ref()
}

fn html_image_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r#"(?is)<img\b[^>]*>"#).ok())
        .as_ref()
}

fn markdown_badge_link_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r#"\[\!\[[^\]]*\]\([^)]+\)\]\([^)]+\)"#).ok())
        .as_ref()
}

fn markdown_image_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r#"\!\[[^\]]*\]\([^)]+\)"#).ok())
        .as_ref()
}

fn strip_html_badge_links(content: &str) -> String {
    let Some(anchor_re) = html_anchor_image_regex() else {
        return content.to_string();
    };
    let Some(img_re) = html_image_regex() else {
        return content.to_string();
    };
    let no_anchor_images = anchor_re.replace_all(content, "");
    img_re.replace_all(&no_anchor_images, "").into_owned()
}

fn strip_markdown_badges(content: &str) -> String {
    let Some(re) = markdown_badge_link_regex() else {
        return content.to_string();
    };
    re.replace_all(content, "").into_owned()
}

fn strip_markdown_images(content: &str) -> String {
    let Some(re) = markdown_image_regex() else {
        return content.to_string();
    };
    re.replace_all(content, "").into_owned()
}

fn normalize_blank_lines(content: &str) -> String {
    let mut lines = Vec::new();
    let mut previous_blank = false;

    for line in content.lines() {
        let trimmed_end = line.trim_end();
        let is_blank = trimmed_end.trim().is_empty();

        if is_blank {
            if !previous_blank {
                lines.push(String::new());
            }
        } else {
            lines.push(trimmed_end.to_string());
        }

        previous_blank = is_blank;
    }

    lines.join("\n").trim().to_string()
}

fn markdown_heading_parts(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if level == 0 || level > 6 {
        return None;
    }

    let title = trimmed.get(level..)?.trim();
    if title.is_empty() {
        None
    } else {
        Some((level, title))
    }
}

fn should_drop_readme_section(title: &str) -> bool {
    let normalized = title.to_ascii_lowercase();
    let denied = [
        "installation",
        "install",
        "getting started",
        "setup",
        "contributing",
        "contribution",
        "development",
        "license",
        "licence",
        "security",
        "release",
        "deployment",
        "docker",
        "build",
        "testing",
        "test",
    ];

    denied.iter().any(|needle| normalized.contains(needle))
}

fn prune_readme_sections(content: &str) -> String {
    let mut intro_lines = Vec::new();
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_body: Vec<String> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim_end().to_string();
        let trimmed = line.trim();

        if let Some((level, _)) = markdown_heading_parts(trimmed) {
            if level == 1 && current_heading.is_none() && current_body.is_empty() {
                intro_lines.push(line);
                continue;
            }

            if let Some(heading) = current_heading.take() {
                sections.push((heading, std::mem::take(&mut current_body)));
            }
            current_heading = Some(line);
        } else if current_heading.is_some() {
            current_body.push(line);
        } else {
            intro_lines.push(line);
        }
    }

    if let Some(heading) = current_heading {
        sections.push((heading, current_body));
    }

    let mut kept_chunks = Vec::new();
    let intro = normalize_blank_lines(&intro_lines.join("\n"));
    if !intro.is_empty() {
        kept_chunks.push(intro);
    }

    for (heading, body_lines) in sections.into_iter().take(README_MAX_ALLOWED_SECTIONS + 8) {
        let Some((_, title)) = markdown_heading_parts(heading.trim()) else {
            continue;
        };
        if should_drop_readme_section(title) {
            continue;
        }

        let body = normalize_blank_lines(&body_lines.join("\n"));
        if body.is_empty() {
            continue;
        }

        kept_chunks.push(format!("{}\n\n{}", heading.trim(), body));
        if kept_chunks.len().saturating_sub(1) >= README_MAX_ALLOWED_SECTIONS {
            break;
        }
    }

    kept_chunks.join("\n\n")
}

fn sanitize_readme_blob(content: &str) -> String {
    let without_html_badges = strip_html_badge_links(content);
    let without_markdown_badges = strip_markdown_badges(&without_html_badges);
    let without_markdown_images = strip_markdown_images(&without_markdown_badges);
    let pruned = prune_readme_sections(&without_markdown_images);
    normalize_blank_lines(&pruned)
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

fn blob_from_text(artifact_type: &str, text: String) -> ArtifactBlob {
    ArtifactBlob {
        artifact_type: artifact_type.to_string(),
        payload_blob: text.into_bytes(),
    }
}

fn default_test_intent_message() -> String {
    "Sem cobertura de testes explicita".to_string()
}

fn default_ux_contracts_message() -> String {
    "Backend puro, sem interface UX".to_string()
}

fn default_unsafe_hotspots_message() -> String {
    "Sem hotspots explicitos de risco".to_string()
}

fn should_skip_dir(name: &str) -> bool {
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

fn has_code_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()),
        Some(ext)
            if matches!(
                ext.as_str(),
                "rs" | "js" | "jsx" | "ts" | "tsx" | "py" | "go" | "java" | "kt" | "swift" | "svelte" | "vue" | "mjs" | "cjs"
            )
    )
}

fn is_frontend_file(path: &Path) -> bool {
    if should_skip_documentation_path(path) {
        return false;
    }

    if should_skip_ux_noise_path(path) {
        return false;
    }

    let in_frontend_dir = path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|part| {
                let lower = part.to_ascii_lowercase();
                lower == "ui" || lower == "components" || lower == "frontend"
            })
            .unwrap_or(false)
    });
    let valid_extension = matches!(
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "ts" | "tsx" | "js" | "jsx" | "svelte" | "vue")
    );
    let has_real_ui_scope = has_path_segment(path, "src") || has_path_segment(path, "components");

    in_frontend_dir && valid_extension && has_real_ui_scope
}

fn should_skip_documentation_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|part| {
                let lower = part.to_ascii_lowercase();
                lower == "documentation" || lower == "docs" || lower == "website" || lower == "examples"
            })
            .unwrap_or(false)
    })
}

fn has_path_segment(path: &Path, expected: &str) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|part| part.eq_ignore_ascii_case(expected))
            .unwrap_or(false)
    })
}

fn should_skip_ux_noise_path(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default();

    let noisy_file_patterns = [
        ".test.", ".spec.", ".stories.", ".story.", ".config.", ".gen.", "eslint.config",
        "forge.config", "vite.config", "vitest.config", "jest.config", "playwright.config",
        "tailwind.config", "postcss.config",
    ];
    if noisy_file_patterns.iter().any(|pattern| file_name.contains(pattern)) {
        return true;
    }

    ["__tests__", "tests", "test", "scripts", "storybook", "stories", "api"]
        .iter()
        .any(|segment| has_path_segment(path, segment))
}

fn slice_source_span(source: &str, span: Span) -> Option<&str> {
    source.get(span.start as usize..span.end as usize)
}

fn capture_line_at_offset(source: &str, offset: u32) -> Option<&str> {
    let offset = usize::try_from(offset).ok()?;
    let safe_offset = offset.min(source.len());
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

fn format_props_entry(snippet: &str) -> String {
    if snippet.trim_start().to_ascii_lowercase().starts_with("props") {
        snippet.to_string()
    } else {
        format!("props {}", snippet)
    }
}

fn is_ux_contract_type_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("props") || lower.contains("state")
}

fn compact_props_parameter_entry(source: &str, parameter: &FormalParameter<'_>) -> Option<String> {
    let snippet = slice_source_span(source, parameter.span)
        .or_else(|| capture_line_at_offset(source, parameter.span.start))
        .and_then(normalize_code_line_snippet)?;
    let lower = snippet.to_ascii_lowercase();
    if lower.contains("props") {
        Some(format_props_entry(&snippet))
    } else {
        None
    }
}

fn compact_props_binding_entry(source: &str, declarator: &VariableDeclarator<'_>) -> Option<String> {
    let snippet = slice_source_span(source, declarator.id.span())
        .or_else(|| capture_line_at_offset(source, declarator.id.span().start))
        .and_then(normalize_code_line_snippet)?;
    if snippet.to_ascii_lowercase().contains("props") {
        Some(format_props_entry(&snippet))
    } else {
        None
    }
}

fn compact_state_entry(source: &str, declarator: &VariableDeclarator<'_>) -> Option<String> {
    let init = declarator.init.as_ref()?;
    let oxc::ast::ast::Expression::CallExpression(call) = init else {
        return None;
    };
    let callee_name = call.callee_name()?;
    if !STATE_CALL_NAMES.contains(&callee_name) {
        return None;
    }

    let binding = slice_source_span(source, declarator.id.span())
        .or_else(|| capture_line_at_offset(source, declarator.id.span().start))
        .and_then(normalize_code_line_snippet)?;
    Some(format!("state {} = {}()", binding, callee_name))
}

fn extract_embedded_script_source(path: &Path, content: &str) -> Option<(String, PathBuf)> {
    let lower = content.to_ascii_lowercase();
    let mut cursor = 0;
    let mut blocks = Vec::new();
    let mut is_typescript = false;

    while let Some(start_rel) = lower[cursor..].find("<script") {
        let start = cursor + start_rel;
        let Some(header_rel) = lower[start..].find('>') else {
            break;
        };
        let header_end = start + header_rel;
        let header = &lower[start..=header_end];
        if header.contains("lang=\"ts\"")
            || header.contains("lang='ts'")
            || header.contains("lang=\"tsx\"")
            || header.contains("lang='tsx'")
        {
            is_typescript = true;
        }

        let body_start = header_end + 1;
        let Some(close_rel) = lower[body_start..].find("</script>") else {
            break;
        };
        let body_end = body_start + close_rel;
        let snippet = content[body_start..body_end].trim();
        if !snippet.is_empty() {
            blocks.push(snippet.to_string());
        }
        cursor = body_end + "</script>".len();
    }

    if blocks.is_empty() {
        None
    } else {
        Some((
            blocks.join("\n\n"),
            path.with_extension(if is_typescript { "ts" } else { "js" }),
        ))
    }
}

fn frontend_ast_input(path: &Path, content: &str) -> Option<(String, PathBuf)> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())?;

    match extension.as_str() {
        "ts" | "tsx" | "js" | "jsx" => Some((content.to_string(), path.to_path_buf())),
        "svelte" | "vue" => extract_embedded_script_source(path, content),
        _ => None,
    }
}

fn extract_frontend_contracts_from_content(path: &Path, content: &str) -> CappedFileEntries {
    let Some((source_text, parse_path)) = frontend_ast_input(path, content) else {
        return CappedFileEntries {
            items: Vec::new(),
            omitted_count: 0,
        };
    };
    let Ok(source_type) = SourceType::from_path(&parse_path) else {
        return CappedFileEntries {
            items: Vec::new(),
            omitted_count: 0,
        };
    };

    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, &source_text, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();

    if parser_return.panicked {
        return CappedFileEntries {
            items: Vec::new(),
            omitted_count: 0,
        };
    }

    let mut collector = UxAstCollector::new(&source_text);
    collector.visit_program(&parser_return.program);
    prioritize_ux_entries(collector.finish())
}

fn prioritize_ux_entries(entries: Vec<String>) -> CappedFileEntries {
    let mut types = Vec::new();
    let mut props = Vec::new();
    let mut states = Vec::new();
    let mut other = Vec::new();

    for entry in entries {
        if entry.starts_with("interface ") || entry.starts_with("type ") {
            types.push(entry);
        } else if entry.starts_with("props") {
            props.push(entry);
        } else if entry.starts_with("state ") {
            states.push(entry);
        } else {
            other.push(entry);
        }
    }

    let mut prioritized = Vec::new();
    prioritized.extend(types);
    prioritized.extend(props);
    prioritized.extend(states);
    prioritized.extend(other);

    CappedFileEntries {
        items: prioritized,
        omitted_count: 0,
    }
}

fn collect_repo_files(root: &Path) -> Result<Vec<PathBuf>, ExtractionError> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ExtractionError> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                if dir == root {
                    return Err(ExtractionError::IoError {
                        file: dir.display().to_string(),
                        reason: e.to_string(),
                    });
                }
                warn!(dir = %dir.display(), error = %e, "Falha ao listar diretório; ignorando");
                return Ok(());
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    warn!(dir = %dir.display(), error = %e, "Falha ao iterar entrada; ignorando");
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "Falha ao ler tipo da entrada; ignorando");
                    continue;
                }
            };

            if file_type.is_dir() {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    if should_skip_dir(name) {
                        continue;
                    }
                }
                walk(root, &path, out)?;
            } else if file_type.is_file() {
                out.push(path);
            }
        }

        Ok(())
    }

    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

fn read_small_text_file(path: &Path) -> Result<Option<String>, ExtractionError> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };

    if metadata.len() > MAX_SCAN_FILE_BYTES {
        return Ok(None);
    }

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };

    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

impl TestIntentExtractor {
    pub fn default_blob() -> ArtifactBlob {
        blob_from_text("blob_03_test_intent", default_test_intent_message())
    }

    pub async fn extract_blob(input: TestIntentInput<'_>) -> Result<ArtifactBlob, ExtractionError> {
        let body = Self::extract_body(input).await?;
        Ok(blob_from_text("blob_03_test_intent", body))
    }

    pub async fn extract_body(input: TestIntentInput<'_>) -> Result<String, ExtractionError> {
        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: input.repo_path.as_ref(),
            profile: input.profile,
        })
        .await
        .map_err(|e| ExtractionError::IoError {
            file: "blob_03_test_intent".to_string(),
            reason: e.to_string(),
        })?;

        let timed_out = payload.timed_out;
        let mut blocks = payload.blocks;

        let mut body = if blocks.is_empty() {
            default_test_intent_message()
        } else {
            if !timed_out {
                blocks.sort_by(|left, right| {
                    test_intent_priority(&left.file_path)
                        .cmp(&test_intent_priority(&right.file_path))
                        .then_with(|| left.file_path.cmp(&right.file_path))
                });
            };
            pack_scoped_text_blocks(&blocks, TEST_INTENT_BLOB_MAX_CHARS)
        };

        if timed_out {
            body.push_str("\n\n[AVISO SODA FINOPS: Repositório massivo. Extração profunda abortada aos 60s. O texto acima representa a topologia ampla extraída em Busca em Largura.]");
            body = truncate_utf8(&body, TEST_INTENT_BLOB_MAX_CHARS, TEST_INTENT_BLOB_MAX_CHARS);
        }

        Ok(body)
    }
}

fn test_intent_priority(path: &str) -> usize {
    let normalized = path.to_ascii_lowercase();
    if normalized.starts_with("cargo::")
        || normalized.contains("/src/core/")
        || normalized.contains("/src/backend/")
        || normalized.contains("/daemon/")
        || normalized.contains("/services/")
        || normalized.contains("/lib/")
        || normalized.contains("/api/")
        || normalized.contains("crates/goose/src/")
        || normalized.contains("crates/goose-server/src/")
    {
        0
    } else if normalized.contains("crates/goose/tests/")
        || normalized.contains("crates/goose-server/tests/")
        || normalized.contains("/integration/")
    {
        1
    } else if normalized.contains("crates/goose-cli/")
        || normalized.contains("/cli/")
        || normalized.contains("/session/")
    {
        2
    } else {
        3
    }
}

impl UnsafeHotspotsExtractor {
    pub async fn extract_blob(repo_path: &RepoPath) -> Result<ArtifactBlob, ExtractionError> {
        let root = repo_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let files = collect_repo_files(&root)?;
            let patterns = ["unsafe {", "eval(", "TODO:", "FIXME:"];
            let mut hits = Vec::new();

            for path in files.into_iter().filter(|path| has_code_extension(path)) {
                let Some(content) = read_small_text_file(&path)? else {
                    continue;
                };
                let rel = relative_display(&root, &path);

                for (idx, line) in content.lines().enumerate() {
                    for pattern in patterns {
                        if line.contains(pattern) {
                            hits.push(format!(
                                "{}:{} [{}] {}",
                                rel,
                                idx + 1,
                                pattern,
                                line.trim()
                            ));
                            break;
                        }
                    }
                }
            }

            let body = if hits.is_empty() {
                default_unsafe_hotspots_message()
            } else {
                truncate_utf8(
                    &hits.join("\n"),
                    UNSAFE_HOTSPOTS_BLOB_MAX_CHARS,
                    UNSAFE_HOTSPOTS_BLOB_MAX_CHARS,
                )
            };

            Ok(blob_from_text("blob_06_unsafe_hotspots", body))
        })
        .await
        .map_err(|e| ExtractionError::IoError {
            file: "blob_06_unsafe_hotspots".to_string(),
            reason: e.to_string(),
        })?
    }
}

impl UxContractsExtractor {
    pub async fn extract_blob(repo_path: &RepoPath) -> Result<ArtifactBlob, ExtractionError> {
        let body = Self::extract_body(repo_path).await?;
        Ok(blob_from_text("blob_11_ux_contracts", body))
    }

    pub async fn extract_body(repo_path: &RepoPath) -> Result<String, ExtractionError> {
        let root = repo_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let files = collect_repo_files(&root)?;
            let mut sections = Vec::new();

            for path in files.into_iter().filter(|path| is_frontend_file(path)) {
                let Some(content) = read_small_text_file(&path)? else {
                    continue;
                };
                let rel = relative_display(&root, &path);
                let semantics = extract_frontend_contracts_from_content(&path, &content);

                if !semantics.items.is_empty() {
                    sections.push(ScopedTextBlock {
                        file_path: rel,
                        items: semantics.items,
                        omitted_count: semantics.omitted_count,
                    });
                }
            }

            let body = if sections.is_empty() {
                default_ux_contracts_message()
            } else {
                pack_scoped_text_blocks(&sections, UX_CONTRACTS_BLOB_MAX_CHARS)
            };

            Ok(body)
        })
        .await
        .map_err(|e| ExtractionError::IoError {
            file: "blob_11_ux_contracts".to_string(),
            reason: e.to_string(),
        })?
    }
}

impl DomainMechanicsExtractor {
    pub async fn extract_body(input: TestIntentInput<'_>) -> Result<String, ExtractionError> {
        let test_body = TestIntentExtractor::extract_body(TestIntentInput {
            repo_path: input.repo_path,
            profile: input.profile,
        })
        .await?;
        let ux_body = UxContractsExtractor::extract_body(input.repo_path).await?;

        let merged = Self::compose_blob_body(test_body, ux_body);
        let packed = truncate_utf8(
            &merged,
            PHASE1_HEAVY_BLOB_MAX_CHARS,
            PHASE1_HEAVY_BLOB_MAX_CHARS,
        );

        if packed.trim().is_empty() {
            return Err(ExtractionError::EmptyArtifact {
                artifact_type: "blob_03_domain_mechanics".to_string(),
                file: "domain_mechanics_bundle".to_string(),
            });
        }

        Ok(packed)
    }

    fn compose_blob_body(test_body: String, ux_body: String) -> String {
        format!(
            "# Domain Mechanics\n\n## Test Intent\n{}\n\n## UX Contracts\n{}",
            test_body.trim(),
            ux_body.trim()
        )
    }
}

impl OpsBlueprintExtractor {
    pub async fn extract_blob(input: OpsInput<'_>) -> Result<ArtifactBlob, ExtractionError> {
        let payload = Self::extract(input).await?;
        let mut sections = Vec::new();

        for priority_path in ["Dockerfile", "docker-compose.yml", "docker-compose.yaml"] {
            if let Some(file) = payload.infra_files.iter().find(|file| file.path == priority_path) {
                sections.push(format!("### {}\n{}\n", file.path, file.content.trim()));
            }
        }

        for file in payload
            .infra_files
            .iter()
            .filter(|file| file.path.starts_with(".github/workflows/"))
        {
            sections.push(format!("### {}\n{}\n", file.path, file.content.trim()));
        }

        if sections.is_empty() {
            return Ok(ArtifactBlob {
                artifact_type: "blob_07_ops_blueprint".to_string(),
                payload_blob: b"Nenhum artefato operacional detectado (Dockerfile/docker-compose/.github/workflows/Makefile).\n".to_vec(),
            });
        }

        let packed = truncate_utf8(&sections.join("\n"), OPS_BLOB_MAX_CHARS, OPS_BLOB_MAX_CHARS);
        if packed.trim().is_empty() {
            return Ok(ArtifactBlob {
                artifact_type: "blob_07_ops_blueprint".to_string(),
                payload_blob: b"Nenhum artefato operacional detectado (bundle vazio apos truncagem).\n".to_vec(),
            });
        }

        Ok(ArtifactBlob {
            artifact_type: "blob_07_ops_blueprint".to_string(),
            payload_blob: packed.into_bytes(),
        })
    }

    pub async fn extract(input: OpsInput<'_>) -> Result<OpsPayload, ExtractionError> {
        let root_targets = [
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.yaml",
            "Makefile",
        ];

        let mut infra_files = Vec::new();

        // 1. Root files
        for &file_name in &root_targets {
            let path = input.repo_path.join(file_name);
            if let Some(infra) = Self::read_infra_file(&path, file_name).await? {
                infra_files.push(infra);
            }
        }

        // 2. Workflows (.github/workflows/) - Level 1 only
        let workflows_path = input.repo_path.join(".github/workflows");
        match fs::read_dir(&workflows_path).await {
            Ok(mut entries) => {
                loop {
                    let entry = match entries.next_entry().await {
                        Ok(Some(entry)) => entry,
                        Ok(None) => break,
                        Err(e) => {
                            warn!(
                                dir = %workflows_path.display(),
                                error = %e,
                                "Falha ao iterar workflows; ignorando restante"
                            );
                            break;
                        }
                    };
                    let file_type = match entry.file_type().await {
                        Ok(file_type) => file_type,
                        Err(e) => {
                            warn!(path = %entry.path().display(), error = %e, "Falha ao ler tipo do workflow; ignorando");
                            continue;
                        }
                    };

                    if file_type.is_file() {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        if file_name.ends_with(".yml") || file_name.ends_with(".yaml") {
                            let path = entry.path();
                            let rel_path = format!(".github/workflows/{}", file_name);
                            if let Some(infra) = Self::read_infra_file(&path, &rel_path).await? {
                                infra_files.push(infra);
                            }
                        }
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(ExtractionError::IoError {
                    file: workflows_path.display().to_string(),
                    reason: e.to_string(),
                });
            }
        }

        if infra_files.is_empty() {
            Err(ExtractionError::NotFound)
        } else {
            Ok(OpsPayload { infra_files })
        }
    }

    async fn read_infra_file(path: &std::path::Path, rel_path: &str) -> Result<Option<InfraFile>, ExtractionError> {
        let metadata = match fs::metadata(path).await {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ExtractionError::IoError {
                file: rel_path.to_string(),
                reason: e.to_string(),
            }),
        };

        let size = metadata.len();
        if size > MAX_MANIFEST_SIZE {
            return Err(ExtractionError::FileTooLarge {
                file: rel_path.to_string(),
                size_bytes: size,
                limit_bytes: MAX_MANIFEST_SIZE,
            });
        }

        let content = fs::read_to_string(path).await.map_err(|e| ExtractionError::IoError {
            file: rel_path.to_string(),
            reason: e.to_string(),
        })?;

        Ok(Some(InfraFile {
            path: rel_path.to_string(),
            content,
        }))
    }
}

impl ManifestExtractor {
    pub async fn extract_blob(input: ManifestInput<'_>) -> Result<ArtifactBlob, ExtractionError> {
        let root = input.repo_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let files = collect_repo_files(&root)?;
            let mut blocks = Vec::new();
            let mut manifest_files_seen = BTreeSet::<String>::new();

            for path in files.into_iter().filter(|path| Self::is_manifest_blob_target(path)) {
                let rel_path = relative_display(&root, &path);
                if Self::should_skip_manifest_fixture_path(&rel_path) {
                    continue;
                }
                let file_name = Path::new(&rel_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string();
                if !file_name.is_empty() {
                    manifest_files_seen.insert(file_name);
                }
                let metadata = match std::fs::metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        warn!(
                            artifact_type = "blob_02_dependency_manifest",
                            manifest = %rel_path,
                            abs_path = %path.display(),
                            error = %e,
                            "Falha ao ler metadata do manifesto; ignorando"
                        );
                        continue;
                    }
                };

                let size = metadata.len();
                if size > MAX_MANIFEST_SIZE {
                    return Err(ExtractionError::FileTooLarge {
                        file: rel_path,
                        size_bytes: size,
                        limit_bytes: MAX_MANIFEST_SIZE,
                    });
                }

                info!(
                    artifact_type = "blob_02_dependency_manifest",
                    manifest = %rel_path,
                    abs_path = %path.display(),
                    "Tentando ler manifesto"
                );
                let content = match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(e) => {
                        warn!(
                            artifact_type = "blob_02_dependency_manifest",
                            manifest = %rel_path,
                            abs_path = %path.display(),
                            error = %e,
                            "Falha ao ler manifesto; ignorando"
                        );
                        continue;
                    }
                };

                if content.trim().is_empty() {
                    warn!(
                        artifact_type = "blob_02_dependency_manifest",
                        manifest = %rel_path,
                        abs_path = %path.display(),
                        "Manifesto vazio; ignorando"
                    );
                    continue;
                }

                if let Some(block) = Self::extract_manifest_block(&rel_path, &content)? {
                    blocks.push(block);
                }
            }

            let stack_base = Self::stack_base_from_manifest_files(&manifest_files_seen);

            blocks.sort_by(|left, right| {
                Self::manifest_blob_priority(&left.file_path)
                    .cmp(&Self::manifest_blob_priority(&right.file_path))
                    .then_with(|| left.file_path.cmp(&right.file_path))
            });

            let header = format!("stack_base: {}\n\n", stack_base);
            let packed = if blocks.is_empty() {
                "Nenhum manifesto detectado (Cargo.toml, package.json, pyproject.toml, requirements.txt, go.mod)."
                    .to_string()
            } else {
                pack_scoped_text_blocks(&blocks, MANIFEST_BLOB_MAX_CHARS)
            };
            let mut final_text = format!("{}{}", header, packed);
            final_text = truncate_chars(&final_text, MANIFEST_BLOB_MAX_CHARS);
            if final_text.trim().is_empty() {
                final_text = header;
            }

            Ok(blob_from_text("blob_02_dependency_manifest", final_text))
        })
        .await
        .map_err(|e| ExtractionError::IoError {
            file: "blob_02_dependency_manifest".to_string(),
            reason: e.to_string(),
        })?
    }

    fn should_skip_manifest_fixture_path(rel_path: &str) -> bool {
        const SKIP_DIRS: [&str; 7] = [
            "tests",
            "test",
            "fixtures",
            "vendor",
            "node_modules",
            "__mocks__",
            "__tests__",
        ];
        let normalized = super::normalize_repo_path_key(rel_path);
        super::normalized_path_has_any_segment(&normalized, &SKIP_DIRS)
    }

    fn is_manifest_blob_target(path: &Path) -> bool {
        matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some("Cargo.toml" | "package.json" | "go.mod" | "pyproject.toml" | "requirements.txt")
        )
    }

    fn manifest_blob_priority(path: &str) -> (u8, usize) {
        let file_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let kind_score = match file_name {
            "Cargo.toml" => 0,
            "package.json" => 1,
            "pyproject.toml" | "requirements.txt" => 2,
            "go.mod" => 3,
            _ => 9,
        };
        (kind_score, path.matches('/').count())
    }

    fn extract_manifest_block(file_path: &str, content: &str) -> Result<Option<ScopedTextBlock>, ExtractionError> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ExtractionError::ParseError {
                file: file_path.to_string(),
                reason: "Nome do manifesto invalido".to_string(),
            })?;

        let names = match file_name {
            "Cargo.toml" => Self::extract_cargo_dependency_names(content, file_path)?,
            "package.json" => Self::extract_package_json_dependency_names(content, file_path)?,
            "go.mod" => Self::extract_go_mod_dependency_names(content),
            "requirements.txt" => Self::extract_requirements_dependency_names(content),
            "pyproject.toml" => Self::extract_pyproject_dependency_names(content, file_path)?,
            _ => Vec::new(),
        };

        if names.is_empty() {
            return Ok(None);
        }

        let total_names = names.len();
        let mut limited_names = names;
        limited_names.truncate(MANIFEST_MAX_DEPENDENCIES_PER_FILE);
        let omitted_count = total_names.saturating_sub(limited_names.len());
        let items = limited_names
            .chunks(MANIFEST_DEPENDENCY_CHUNK_SIZE)
            .map(|chunk| chunk.join(", "))
            .collect::<Vec<_>>();

        Ok(Some(ScopedTextBlock {
            file_path: file_path.to_string(),
            items,
            omitted_count,
        }))
    }

    fn extract_cargo_dependency_names(content: &str, file: &str) -> Result<Vec<String>, ExtractionError> {
        let document: toml::Value = toml::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        let mut names = BTreeSet::new();
        Self::collect_toml_dependency_table_names(document.get("dependencies"), file, "dependencies", &mut names)?;
        Self::collect_toml_dependency_table_names(
            document.get("workspace").and_then(|workspace| workspace.get("dependencies")),
            file,
            "workspace.dependencies",
            &mut names,
        )?;
        Self::collect_toml_dependency_table_names(
            document.get("build-dependencies"),
            file,
            "build-dependencies",
            &mut names,
        )?;
        Self::collect_toml_dependency_table_names(
            document.get("dev-dependencies"),
            file,
            "dev-dependencies",
            &mut names,
        )?;

        if let Some(targets) = document.get("target") {
            let Some(target_table) = targets.as_table() else {
                return Err(ExtractionError::ParseError {
                    file: file.to_string(),
                    reason: "Secao [target] precisa ser uma tabela TOML valida".to_string(),
                });
            };

            for (target_name, target_value) in target_table {
                let Some(target_sections) = target_value.as_table() else {
                    return Err(ExtractionError::ParseError {
                        file: file.to_string(),
                        reason: format!("Secao [target.{}] precisa ser uma tabela TOML valida", target_name),
                    });
                };

                for section_name in ["dependencies", "build-dependencies", "dev-dependencies"] {
                    Self::collect_toml_dependency_table_names(
                        target_sections.get(section_name),
                        file,
                        &format!("target.{}.{}", target_name, section_name),
                        &mut names,
                    )?;
                }
            }
        }

        Ok(names.into_iter().collect())
    }

    fn collect_toml_dependency_table_names(
        value: Option<&toml::Value>,
        file: &str,
        section_name: &str,
        names: &mut BTreeSet<String>,
    ) -> Result<(), ExtractionError> {
        let Some(value) = value else {
            return Ok(());
        };
        let Some(table) = value.as_table() else {
            return Err(ExtractionError::ParseError {
                file: file.to_string(),
                reason: format!("Secao [{}] nao e uma tabela TOML valida", section_name),
            });
        };

        for name in table.keys() {
            names.insert(name.to_string());
        }
        Ok(())
    }

    fn extract_package_json_dependency_names(content: &str, file: &str) -> Result<Vec<String>, ExtractionError> {
        let document: serde_json::Value = serde_json::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;
        let Some(root) = document.as_object() else {
            return Err(ExtractionError::ParseError {
                file: file.to_string(),
                reason: "package.json precisa ser um objeto JSON".to_string(),
            });
        };

        let mut names = BTreeSet::new();
        for key in ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"] {
            let Some(value) = root.get(key) else {
                continue;
            };
            let Some(object) = value.as_object() else {
                return Err(ExtractionError::ParseError {
                    file: file.to_string(),
                    reason: format!("Secao '{}' precisa ser um objeto JSON", key),
                });
            };

            for name in object.keys() {
                names.insert(name.to_string());
            }
        }

        Ok(names.into_iter().collect())
    }

    fn extract_go_mod_dependency_names(content: &str) -> Vec<String> {
        let mut names = BTreeSet::new();
        let mut in_require_block = false;

        for raw_line in content.lines() {
            let line_without_comment = raw_line.split("//").next().unwrap_or("");
            let line = line_without_comment.trim();
            if line.is_empty() {
                continue;
            }

            if in_require_block {
                if line == ")" {
                    in_require_block = false;
                    continue;
                }

                if let Some(name) = line.split_whitespace().next() {
                    names.insert(name.to_string());
                }
                continue;
            }

            let Some(rest) = line.strip_prefix("require") else {
                continue;
            };
            let rest = rest.trim();
            if rest == "(" {
                in_require_block = true;
                continue;
            }

            if let Some(inline) = rest.strip_prefix('(') {
                in_require_block = true;
                let inline = inline.trim();
                if !inline.is_empty() && inline != ")" {
                    if let Some(name) = inline.split_whitespace().next() {
                        names.insert(name.to_string());
                    }
                }
                continue;
            }

            if let Some(name) = rest.split_whitespace().next() {
                names.insert(name.to_string());
            }
        }

        names.into_iter().collect()
    }

    fn extract_requirements_dependency_names(content: &str) -> Vec<String> {
        let info = Self::parse_requirements_txt(content, "requirements.txt", content.len() as u64);
        info.dependencies.into_iter().map(|dep| dep.name).collect()
    }

    fn extract_pyproject_dependency_names(content: &str, file: &str) -> Result<Vec<String>, ExtractionError> {
        let info = Self::parse_pyproject_toml(content, file, content.len() as u64)?;
        Ok(info.dependencies.into_iter().map(|dep| dep.name).collect())
    }

    fn stack_base_from_manifest_files(files: &BTreeSet<String>) -> String {
        let has_cargo = files.iter().any(|name| name == "Cargo.toml");
        let has_package = files.iter().any(|name| name == "package.json");
        let has_go = files.iter().any(|name| name == "go.mod");
        let has_python = files.iter().any(|name| name == "pyproject.toml" || name == "requirements.txt");

        let mut stacks = Vec::new();
        if has_python { stacks.push("Python"); }
        if has_package { stacks.push("TypeScript"); }
        if has_go { stacks.push("Go"); }
        if has_cargo { stacks.push("Rust"); }
        if stacks.is_empty() {
            "UNKNOWN".to_string()
        } else {
            stacks.join(", ")
        }
    }

    pub async fn extract(input: ManifestInput<'_>) -> Result<ManifestPayload, ExtractionError> {
        let targets = [
            "Cargo.toml",
            "package.json",
            "go.mod",
            "pyproject.toml",
            "requirements.txt",
            "pom.xml",
            "build.gradle",
            "build.gradle.kts",
        ];

        let mut manifests = Vec::new();

        for &file_name in &targets {
            let path = input.repo_path.join(file_name);
            
            let metadata = match fs::metadata(&path).await {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(ExtractionError::IoError {
                    file: file_name.to_string(),
                    reason: e.to_string(),
                }),
            };

            let size = metadata.len();
            if size > MAX_MANIFEST_SIZE {
                return Err(ExtractionError::FileTooLarge {
                    file: file_name.to_string(),
                    size_bytes: size,
                    limit_bytes: MAX_MANIFEST_SIZE,
                });
            }

            info!(
                artifact_type = "manifest",
                file = file_name,
                abs_path = %path.display(),
                "Tentando ler manifesto base"
            );
            let content = fs::read_to_string(&path).await.map_err(|e| {
                warn!(
                    artifact_type = "manifest",
                    file = file_name,
                    abs_path = %path.display(),
                    error = %e,
                    "Falha ao ler manifesto base"
                );
                ExtractionError::IoError {
                    file: file_name.to_string(),
                    reason: e.to_string(),
                }
            })?;

            if content.trim().is_empty() {
                return Err(ExtractionError::EmptyArtifact {
                    artifact_type: "manifest".to_string(),
                    file: file_name.to_string(),
                });
            }

            let info_res = match file_name {
                "Cargo.toml" => Self::parse_cargo_toml(&content, file_name, size),
                "package.json" => Self::parse_package_json(&content, file_name, size),
                "go.mod" => Ok(Self::parse_go_mod(&content, file_name, size)),
                "requirements.txt" => Ok(Self::parse_requirements_txt(&content, file_name, size)),
                "pyproject.toml" => Self::parse_pyproject_toml(&content, file_name, size),
                _ => Ok(ManifestInfo {
                    file_name: file_name.to_string(),
                    dependencies: Vec::new(),
                    dev_dependencies: Vec::new(),
                    file_size_bytes: size,
                }),
            };

            manifests.push(info_res?);
        }

        if manifests.is_empty() {
            Err(ExtractionError::NotFound)
        } else {
            Ok(ManifestPayload { manifests })
        }
    }

    fn parse_cargo_toml(content: &str, file: &str, size: u64) -> Result<ManifestInfo, ExtractionError> {
        #[derive(Deserialize)]
        struct CargoManifest {
            dependencies: Option<BTreeMap<String, toml::Value>>,
            #[serde(rename = "dev-dependencies")]
            dev_dependencies: Option<BTreeMap<String, toml::Value>>,
        }

        let manifest: CargoManifest = toml::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        Ok(ManifestInfo {
            file_name: file.to_string(),
            dependencies: Self::map_toml_deps(manifest.dependencies),
            dev_dependencies: Self::map_toml_deps(manifest.dev_dependencies),
            file_size_bytes: size,
        })
    }

    fn map_toml_deps(deps: Option<BTreeMap<String, toml::Value>>) -> Vec<DependencyEntry> {
        deps.unwrap_or_default()
            .into_iter()
            .map(|(name, value)| {
                let version = match value {
                    toml::Value::String(s) => s,
                    toml::Value::Table(t) => t.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };
                DependencyEntry { name, version_spec: version }
            })
            .collect()
    }

    fn parse_package_json(content: &str, file: &str, size: u64) -> Result<ManifestInfo, ExtractionError> {
        #[derive(Deserialize)]
        struct PackageJson {
            dependencies: Option<BTreeMap<String, String>>,
            #[serde(rename = "devDependencies")]
            dev_dependencies: Option<BTreeMap<String, String>>,
        }

        let manifest: PackageJson = serde_json::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        Ok(ManifestInfo {
            file_name: file.to_string(),
            dependencies: manifest.dependencies.unwrap_or_default()
                .into_iter()
                .map(|(name, version_spec)| DependencyEntry { name, version_spec })
                .collect(),
            dev_dependencies: manifest.dev_dependencies.unwrap_or_default()
                .into_iter()
                .map(|(name, version_spec)| DependencyEntry { name, version_spec })
                .collect(),
            file_size_bytes: size,
        })
    }

    fn parse_go_mod(content: &str, file: &str, size: u64) -> ManifestInfo {
        let dependencies = Self::extract_go_mod_dependency_names(content)
            .into_iter()
            .map(|name| DependencyEntry {
                name,
                version_spec: "*".to_string(),
            })
            .collect();

        ManifestInfo {
            file_name: file.to_string(),
            dependencies,
            dev_dependencies: Vec::new(),
            file_size_bytes: size,
        }
    }

    fn parse_requirements_txt(content: &str, file: &str, size: u64) -> ManifestInfo {
        let mut dependencies = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Simple parsing: name==version, name>=version, or just name
            let parts: Vec<&str> = if line.contains("==") {
                line.split("==").collect()
            } else if line.contains(">=") {
                line.split(">=").collect()
            } else {
                vec![line]
            };

            let name = parts[0].trim().to_string();
            let version = if parts.len() > 1 {
                parts[1].split_whitespace().next().unwrap_or("*").to_string()
            } else {
                "*".to_string()
            };

            dependencies.push(DependencyEntry { name, version_spec: version });
        }

        ManifestInfo {
            file_name: file.to_string(),
            dependencies,
            dev_dependencies: Vec::new(),
            file_size_bytes: size,
        }
    }

    fn parse_pyproject_toml(content: &str, file: &str, size: u64) -> Result<ManifestInfo, ExtractionError> {
        // pyproject.toml structure can vary (poetry, setuptools, flit)
        // We'll look for [project.dependencies] or [tool.poetry.dependencies]
        let doc: toml::Value = toml::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        let mut dependencies = Vec::new();
        let dev_dependencies = Vec::new();

        // 1. Standard PEP 621 [project.dependencies]
        if let Some(deps) = doc.get("project").and_then(|p| p.get("dependencies")).and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(s) = dep.as_str() {
                    dependencies.push(DependencyEntry { name: s.to_string(), version_spec: "*".to_string() });
                }
            }
        }

        // 2. Poetry [tool.poetry.dependencies]
        if let Some(deps) = doc.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("dependencies")).and_then(|d| d.as_table()) {
            for (name, val) in deps {
                if name == "python" { continue; }
                let version = match val {
                    toml::Value::String(s) => s.clone(),
                    _ => "*".to_string(),
                };
                dependencies.push(DependencyEntry { name: name.clone(), version_spec: version });
            }
        }

        Ok(ManifestInfo {
            file_name: file.to_string(),
            dependencies,
            dev_dependencies,
            file_size_bytes: size,
        })
    }
}

pub fn truncate_community_meta_json<T: Serialize>(payload: &T) -> Result<Vec<u8>, ExtractionError> {
    let json = serde_json::to_string(payload).map_err(|e| ExtractionError::ParseError {
        file: "blob_09_community_meta".to_string(),
        reason: e.to_string(),
    })?;
    let truncated = truncate_chars(&json, COMMUNITY_META_MAX_CHARS);
    if truncated.trim().is_empty() {
        return Err(ExtractionError::EmptyArtifact {
            artifact_type: "blob_09_community_meta".to_string(),
            file: "community_meta".to_string(),
        });
    }
    Ok(truncated.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;

    #[tokio::test]
    async fn test_extract_cargo_toml() {
        let dir = TempDir::new().unwrap();
        let content = r#"[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
tempfile = "3"
"#;
        fs::write(dir.path().join("Cargo.toml"), content).await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        assert_eq!(result.manifests.len(), 1);
        let m = &result.manifests[0];
        assert_eq!(m.file_name, "Cargo.toml");
        assert!(m.dependencies.iter().any(|d| d.name == "serde" && d.version_spec == "1.0"));
        assert!(m.dev_dependencies.iter().any(|d| d.name == "tempfile" && d.version_spec == "3"));
    }

    #[tokio::test]
    async fn test_local_static_extractor_collects_super_pacote_raw() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("README.md"), "README principal").await.unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"demo\"").await.unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM rust:1.80").await.unwrap();

        let blobs = LocalStaticExtractor::extract_all(dir.path()).await.unwrap();

        assert_eq!(blobs.len(), 1);
        assert!(blobs.iter().any(|blob| blob.artifact_type == "blob_01_promessa_readme"));
    }

    #[tokio::test]
    async fn test_local_static_extractor_truncates_readme() {
        let dir = TempDir::new().unwrap();
        let oversized = "a".repeat(README_MAX_CHARS + 25);
        fs::write(dir.path().join("README.md"), oversized).await.unwrap();

        let blobs = LocalStaticExtractor::extract_all(dir.path()).await.unwrap();
        let readme_blob = blobs.iter()
            .find(|blob| blob.artifact_type == "blob_01_promessa_readme")
            .expect("README blob deve existir");

        assert_eq!(String::from_utf8_lossy(&readme_blob.payload_blob).chars().count(), README_MAX_CHARS);
    }

    #[tokio::test]
    async fn test_local_static_extractor_removes_readme_badges() {
        let dir = TempDir::new().unwrap();
        let readme = r#"<a href="https://ci.example"><img src="badge.svg"/></a>
[![Build](https://img.shields.io/test.svg)](https://ci.example)

# Goose

Backend orchestration engine.
"#;
        fs::write(dir.path().join("README.md"), readme).await.unwrap();

        let blobs = LocalStaticExtractor::extract_all(dir.path()).await.unwrap();
        let readme_blob = blobs.iter()
            .find(|blob| blob.artifact_type == "blob_01_promessa_readme")
            .expect("README blob deve existir");
        let text = String::from_utf8_lossy(&readme_blob.payload_blob);

        assert!(!text.contains("<img"));
        assert!(!text.contains("[!["));
        assert!(text.contains("# Goose"));
        assert!(text.contains("Backend orchestration engine."));
    }

    #[tokio::test]
    async fn test_local_static_extractor_prunes_infra_readme_sections() {
        let dir = TempDir::new().unwrap();
        let readme = r#"# Goose

Goose is an orchestration engine for local-first coding.

It accelerates repository harvesting with deterministic blobs.

## Features

- Fast AST extraction

## Installation

Run the bootstrap script and install many packages.

## Contributing

Please open a PR.
"#;
        fs::write(dir.path().join("README.md"), readme).await.unwrap();

        let blobs = LocalStaticExtractor::extract_all(dir.path()).await.unwrap();
        let readme_blob = blobs.iter()
            .find(|blob| blob.artifact_type == "blob_01_promessa_readme")
            .expect("README blob deve existir");
        let text = String::from_utf8_lossy(&readme_blob.payload_blob);

        assert!(text.contains("Goose is an orchestration engine for local-first coding."));
        assert!(text.contains("It accelerates repository harvesting with deterministic blobs."));
        assert!(text.contains("## Features"));
        assert!(!text.contains("## Installation"));
        assert!(!text.contains("## Contributing"));
    }

    #[tokio::test]
    async fn test_manifest_blob_whitelists_dependency_sections_only() {
        let dir = TempDir::new().unwrap();
        let cargo_toml = r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = "1.0"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }

[build-dependencies]
cc = "1.1"

[dev-dependencies]
tempfile = "3"
"#;
        let package_json = r#"{
  "name": "demo",
  "version": "0.1.0",
  "dependencies": {
    "react": "^18.2.0"
  },
  "devDependencies": {
    "typescript": "^5.4.0"
  },
  "peerDependencies": {
    "zod": "^3.23.0"
  }
}"#;
        let go_mod = r#"module example.com/demo

go 1.23.0

require (
    github.com/gin-gonic/gin v1.10.0
    golang.org/x/sync v0.9.0 // indirect
)
"#;

        fs::write(dir.path().join("Cargo.toml"), cargo_toml).await.unwrap();
        fs::create_dir_all(dir.path().join("frontend")).await.unwrap();
        fs::create_dir_all(dir.path().join("backend")).await.unwrap();
        fs::write(dir.path().join("frontend/package.json"), package_json).await.unwrap();
        fs::write(dir.path().join("backend/go.mod"), go_mod).await.unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = ManifestExtractor::extract_blob(ManifestInput { repo_path: &repo_path }).await.unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(text.contains("[Cargo.toml]"));
        assert!(text.contains("- cc, serde, tempfile, tokio"));
        assert!(text.contains("[frontend/package.json]"));
        assert!(text.contains("- react, typescript, zod"));
        assert!(text.contains("[backend/go.mod]"));
        assert!(text.contains("- github.com/gin-gonic/gin, golang.org/x/sync"));
        assert!(!text.contains("[package]"));
        assert!(!text.contains("\"name\": \"demo\""));
        assert!(!text.contains("\"version\": \"0.1.0\""));
        assert!(!text.contains("v1.10.0"));
    }

    #[tokio::test]
    async fn test_test_intent_skips_documentation_paths() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("docs")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/src")).await.unwrap();
        fs::write(
            dir.path().join("docs/test_docs.rs"),
            "#[test]\nfn test_docs_should_not_enter_blob() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/src/core_rules.rs"),
            "#[test]\nfn test_real_business_rule() {}\n",
        )
        .await
        .unwrap();
        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = TestIntentExtractor::extract_blob(TestIntentInput {
            repo_path: &repo_path,
            profile: &StackProfile::Rust,
        })
        .await
        .unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(!text.contains("test_docs_should_not_enter_blob"));
        assert!(text.contains("[crates/app/src/core_rules.rs]"));
        assert!(text.contains("- fn test_real_business_rule"));
    }

    #[tokio::test]
    async fn test_test_intent_skips_mock_fixture_and_e2e_noise() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("crates/app/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/mock/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/fixtures/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/test_support/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/e2e/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/integration_mocks/tests"))
            .await
            .unwrap();
        fs::write(
            dir.path().join("crates/app/tests/domain.rs"),
            "#[test]\nfn test_domain_logic_stays() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/mock/tests/mock.rs"),
            "#[test]\nfn test_mock_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/fixtures/tests/fixture.rs"),
            "#[test]\nfn test_fixture_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/test_support/tests/support.rs"),
            "#[test]\nfn test_support_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/e2e/tests/e2e.rs"),
            "#[test]\nfn test_e2e_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/integration_mocks/tests/mock.rs"),
            "#[test]\nfn test_integration_mock_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = TestIntentExtractor::extract_blob(TestIntentInput {
            repo_path: &repo_path,
            profile: &StackProfile::Rust,
        })
        .await
        .unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(text.contains("[crates/app/tests/domain.rs]"));
        assert!(text.contains("- fn test_domain_logic_stays"));
        assert!(!text.contains("test_mock_should_be_ignored"));
        assert!(!text.contains("test_fixture_should_be_ignored"));
        assert!(!text.contains("test_support_should_be_ignored"));
        assert!(!text.contains("test_e2e_should_be_ignored"));
        assert!(!text.contains("test_integration_mock_should_be_ignored"));
    }

    #[tokio::test]
    async fn test_test_intent_preserves_all_items_per_file() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("crates/app/tests")).await.unwrap();
        let mut content = String::new();
        for index in 0..8 {
            content.push_str(&format!("#[test]\nfn test_case_{index}() {{}}\n"));
        }
        fs::write(dir.path().join("crates/app/tests/domain.rs"), content)
            .await
            .unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = TestIntentExtractor::extract_blob(TestIntentInput {
            repo_path: &repo_path,
            profile: &StackProfile::Rust,
        })
        .await
        .unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(text.contains("[crates/app/tests/domain.rs]"));
        assert!(text.contains("- fn test_case_0"));
        assert!(text.contains("- fn test_case_7"));
        assert!(!text.contains("itens omitidos"));
    }

    #[tokio::test]
    async fn test_ux_contracts_skips_documentation_and_keeps_real_ui() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("documentation/src/components")).await.unwrap();
        fs::create_dir_all(dir.path().join("ui/desktop/src/components")).await.unwrap();

        fs::write(
            dir.path().join("documentation/src/components/MarketingCard.tsx"),
            "type MarketingCardProps = { title: string }\nconst [state] = useState('docs')\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("ui/desktop/src/components/AppShell.tsx"),
            "type AppShellProps = { title: string }\nfunction AppShell(props: AppShellProps) { const [state] = useState('app'); return <main>{props.title}</main>; }\n",
        )
        .await
        .unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = UxContractsExtractor::extract_blob(&repo_path).await.unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(!text.contains("documentation/src/components/MarketingCard.tsx"));
        assert!(text.contains("[ui/desktop/src/components/AppShell.tsx]"));
        assert!(text.contains("- type AppShellProps"));
        assert!(text.contains("- props: AppShellProps"));
        assert!(text.contains("- state [state] = useState()"));
    }

    #[tokio::test]
    async fn test_ux_contracts_skips_config_generated_and_test_noise() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("ui/desktop/src/components")).await.unwrap();
        fs::create_dir_all(dir.path().join("ui/desktop/src/api")).await.unwrap();

        fs::write(
            dir.path().join("ui/desktop/eslint.config.js"),
            "description: 'lint rule'\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("ui/desktop/src/App.test.tsx"),
            "const [value, setValue] = useState(false)\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("ui/desktop/src/api/types.gen.ts"),
            "description: string;\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("ui/desktop/src/components/AppShell.tsx"),
            "interface AppShellProps {\n  title: string;\n}\nfunction AppShell(props: AppShellProps) {\n  console.log(props.title);\n  try {\n    toast.error('noisy');\n  } catch (_error) {}\n  const [open, setOpen] = useState(false);\n  return <section>{props.title}</section>;\n}\n",
        )
        .await
        .unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = UxContractsExtractor::extract_blob(&repo_path).await.unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(!text.contains("ui/desktop/eslint.config.js"));
        assert!(!text.contains("ui/desktop/src/App.test.tsx"));
        assert!(!text.contains("ui/desktop/src/api/types.gen.ts"));
        assert!(text.contains("[ui/desktop/src/components/AppShell.tsx]"));
        assert!(text.contains("- interface AppShellProps"));
        assert!(text.contains("- state [open, setOpen] = useState()"));
        assert!(text.contains("- props: AppShellProps"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains("toast.error"));
        assert!(!text.contains("<section>"));
    }

    #[tokio::test]
    async fn test_ux_contracts_preserve_all_items_per_file() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("ui/desktop/src/components")).await.unwrap();
        fs::write(
            dir.path().join("ui/desktop/src/components/ComplexPanel.tsx"),
            "interface A {}\ninterface B {}\ninterface C {}\ninterface D {}\nfunction ComplexPanel(props: A) { const [a, setA] = useState(false); const [b, setB] = useState(false); const [c, setC] = useState(false); const [d, setD] = useState(false); const [e, setE] = useState(false); return <main />; }\n",
        )
        .await
        .unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = UxContractsExtractor::extract_blob(&repo_path).await.unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(text.contains("[ui/desktop/src/components/ComplexPanel.tsx]"));
        assert!(text.contains("- props: A"));
        assert!(text.contains("- state [a, setA] = useState()"));
        assert!(text.contains("- state [e, setE] = useState()"));
        assert!(!text.contains("itens omitidos"));
    }

    #[tokio::test]
    async fn test_extract_package_json() {
        let dir = TempDir::new().unwrap();
        let content = r#"{
            "dependencies": {
                "react": "^18.0.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        }"#;
        fs::write(dir.path().join("package.json"), content).await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        let m = result.manifests.iter().find(|m| m.file_name == "package.json").unwrap();
        assert!(m.dependencies.iter().any(|d| d.name == "react" && d.version_spec == "^18.0.0"));
        assert!(m.dev_dependencies.iter().any(|d| d.name == "typescript" && d.version_spec == "^5.0.0"));
    }

    #[tokio::test]
    async fn test_extract_requirements_txt() {
        let dir = TempDir::new().unwrap();
        let content = "flask==2.0.0\nrequests>=2.25.0\n# comment\npydantic\n";
        fs::write(dir.path().join("requirements.txt"), content).await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        let m = result.manifests.iter().find(|m| m.file_name == "requirements.txt").unwrap();
        assert!(m.dependencies.iter().any(|d| d.name == "flask" && d.version_spec == "2.0.0"));
        assert!(m.dependencies.iter().any(|d| d.name == "requests" && d.version_spec == "2.25.0"));
        assert!(m.dependencies.iter().any(|d| d.name == "pydantic" && d.version_spec == "*"));
    }

    #[tokio::test]
    async fn test_no_manifests() {
        let dir = TempDir::new().unwrap();
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;
        assert_eq!(result.unwrap_err(), ExtractionError::NotFound);
    }

    #[tokio::test]
    async fn test_file_too_large() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Cargo.toml");
        
        // Criar um arquivo com mais de 1MB
        let mut file = std::fs::File::create(&path).unwrap();
        let buffer = vec![0u8; (MAX_MANIFEST_SIZE + 100) as usize];
        file.write_all(&buffer).unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;
        
        match result {
            Err(ExtractionError::FileTooLarge { file, .. }) => assert_eq!(file, "Cargo.toml"),
            _ => panic!("Deveria ter falhado com FileTooLarge, mas retornou {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_corrupted_manifest() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "invalid [ toml").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;
        
        match result {
            Err(ExtractionError::ParseError { file, .. }) => assert_eq!(file, "Cargo.toml"),
            // Se houver apenas um manifesto e ele falhar, o erro propaga.
            // Se houver mais, ele seria ignorado e se sobrasse nenhum, NotFound ou ParseError do último?
            // O PRD diz: "O erro só propaga se TODOS os manifestos falharem."
            _ => panic!("Deveria ter falhado com ParseError, mas retornou {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_multiple_manifests() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[dependencies]").await.unwrap();
        fs::write(dir.path().join("package.json"), "{}").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        assert_eq!(result.manifests.len(), 2);
    }

    #[tokio::test]
    async fn test_partial_failure_aborts_fail_closed() {
        let dir = TempDir::new().unwrap();
        // Um válido e um corrompido
        fs::write(dir.path().join("Cargo.toml"), "[dependencies]").await.unwrap();
        fs::write(dir.path().join("package.json"), "invalid { json").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;

        match result {
            Err(ExtractionError::ParseError { file, .. }) => assert_eq!(file, "package.json"),
            _ => panic!("Deveria ter falhado com ParseError para o manifesto corrompido"),
        }
    }

    #[tokio::test]
    async fn test_local_static_extractor_missing_required_blob_aborts() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"demo\"").await.unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM rust:1.80").await.unwrap();

        let result = LocalStaticExtractor::extract_all(dir.path()).await;

        match result {
            Err(ExtractionError::RequiredArtifactMissing { artifact_type, .. }) => {
                assert_eq!(artifact_type, "blob_01_promessa_readme");
            }
            _ => panic!("Deveria ter falhado quando o README obrigatorio nao existe"),
        }
    }

    #[tokio::test]
    async fn test_ops_extract_root_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM rust").await.unwrap();
        fs::write(dir.path().join("Makefile"), "build:").await.unwrap();
        fs::write(dir.path().join("docker-compose.yml"), "version: '3'").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = OpsBlueprintExtractor::extract(OpsInput { repo_path: &repo_path }).await.unwrap();
        
        assert_eq!(result.infra_files.len(), 3);
        assert!(result.infra_files.iter().any(|f| f.path == "Dockerfile"));
        assert!(result.infra_files.iter().any(|f| f.path == "Makefile"));
    }

    #[tokio::test]
    async fn test_ops_extract_workflows_shallow() {
        let dir = TempDir::new().unwrap();
        let workflows_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&workflows_dir).await.unwrap();
        
        fs::write(workflows_dir.join("ci.yml"), "name: CI").await.unwrap();
        fs::write(workflows_dir.join("deploy.yaml"), "name: Deploy").await.unwrap();
        fs::write(workflows_dir.join("lint.yml"), "name: Lint").await.unwrap();
        fs::write(workflows_dir.join("docker-release.yml"), "name: Docker Release").await.unwrap();
        
        // Criar subdiretório para provar que a recursão ignora
        let nested_dir = workflows_dir.join("nested");
        fs::create_dir_all(&nested_dir).await.unwrap();
        fs::write(nested_dir.join("ignored.yml"), "should be ignored").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = OpsBlueprintExtractor::extract(OpsInput { repo_path: &repo_path }).await.unwrap();
        
        assert_eq!(result.infra_files.len(), 4);
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/deploy.yaml"));
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/docker-release.yml"));
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/ci.yml"));
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/lint.yml"));
        assert!(!result.infra_files.iter().any(|f| f.path.contains("ignored.yml")));
    }

    #[tokio::test]
    async fn test_ops_file_too_large() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Dockerfile");
        
        let mut file = std::fs::File::create(&path).unwrap();
        let buffer = vec![0u8; (MAX_MANIFEST_SIZE + 100) as usize];
        file.write_all(&buffer).unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = OpsBlueprintExtractor::extract(OpsInput { repo_path: &repo_path }).await;
        
        match result {
            Err(ExtractionError::FileTooLarge { file, .. }) => assert_eq!(file, "Dockerfile"),
            _ => panic!("Deveria ter falhado com FileTooLarge para Dockerfile gigante"),
        }
    }
}
