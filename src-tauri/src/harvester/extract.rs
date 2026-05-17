use oxc::{
    allocator::Allocator,
    ast::ast::{FormalParameter, TSInterfaceDeclaration, TSTypeAliasDeclaration, VariableDeclarator},
    ast_visit::{walk, Visit},
    parser::{ParseOptions, Parser},
    span::{GetSpan, SourceType, Span},
};
use tokio::fs;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use super::git::RepoPath;
use super::persist::ArtifactBlob;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

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

pub struct ManifestExtractor;
pub struct TestIntentExtractor;
pub struct UnsafeHotspotsExtractor;
pub struct UxContractsExtractor;

pub struct OpsInput<'a> {
    pub repo_path: &'a RepoPath,
}

pub struct OpsBlueprintExtractor;
pub struct LocalStaticExtractor;

const README_MAX_CHARS: usize = 8_000;
const MANIFEST_BLOB_MAX_CHARS: usize = 8_000;
const OPS_BLOB_MAX_CHARS: usize = 25_000;
const COMMUNITY_META_MAX_CHARS: usize = 1_000;
const TEST_INTENT_BLOB_MAX_CHARS: usize = 30_000;
const UNSAFE_HOTSPOTS_BLOB_MAX_CHARS: usize = 6_000;
const UX_CONTRACTS_BLOB_MAX_CHARS: usize = 30_000;
const MAX_SCAN_FILE_BYTES: u64 = 262_144;
const DOMAIN_TEST_NOISE_MARKERS: [&str; 5] = ["mock", "fixtures", "test_support", "e2e", "integration_mocks"];
const STATE_CALL_NAMES: [&str; 5] = ["useState", "createSignal", "writable", "useReducer", "$state"];

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

    fn push_block(&mut self, span: Span) {
        if let Some(snippet) = slice_source_span(self.source_text, span).and_then(normalize_code_block_snippet) {
            self.push_entry(snippet);
        }
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
            self.push_block(declaration.span);
        }
        walk::walk_ts_interface_declaration(self, declaration);
    }

    fn visit_ts_type_alias_declaration(&mut self, declaration: &TSTypeAliasDeclaration<'a>) {
        if is_ux_contract_type_name(declaration.id.name.as_str()) {
            self.push_block(declaration.span);
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

        blobs.push(Self::extract_blob(
            repo_path,
            &["README.md", "readme.md", "README.txt"],
            "blob_01_promessa_readme",
            README_MAX_CHARS,
        )
        .await?);

        Ok(blobs)
    }

    async fn extract_blob(
        repo_path: &Path,
        candidates: &[&str],
        artifact_type: &str,
        max_chars: usize,
    ) -> Result<ArtifactBlob, ExtractionError> {
        for candidate in candidates {
            let path = repo_path.join(candidate);
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    if content.trim().is_empty() {
                        return Err(ExtractionError::EmptyArtifact {
                            artifact_type: artifact_type.to_string(),
                            file: candidate.to_string(),
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
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    return Err(ExtractionError::IoError {
                        file: candidate.to_string(),
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
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
}

fn strip_html_badge_links(content: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(start_rel) = content[cursor..].find("<a") {
        let start = cursor + start_rel;
        output.push_str(&content[cursor..start]);

        let Some(end_rel) = content[start..].find("</a>") else {
            output.push_str(&content[start..]);
            return output;
        };

        let end = start + end_rel + "</a>".len();
        let anchor = &content[start..end];
        if anchor.to_ascii_lowercase().contains("<img") {
            cursor = end;
            continue;
        }

        output.push_str(anchor);
        cursor = end;
    }

    output.push_str(&content[cursor..]);
    output
}

fn strip_markdown_badges(content: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(start_rel) = content[cursor..].find("[![") {
        let start = cursor + start_rel;
        output.push_str(&content[cursor..start]);

        let Some(mid_rel) = content[start + 3..].find(")](") else {
            output.push_str(&content[start..]);
            return output;
        };
        let mid = start + 3 + mid_rel;
        let outer_start = mid + 3;

        let Some(end_rel) = content[outer_start..].find(')') else {
            output.push_str(&content[start..]);
            return output;
        };

        cursor = outer_start + end_rel + 1;
    }

    output.push_str(&content[cursor..]);
    output
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

fn sanitize_readme_blob(content: &str) -> String {
    let without_html_badges = strip_html_badge_links(content);
    let without_markdown_badges = strip_markdown_badges(&without_html_badges);
    normalize_blank_lines(&without_markdown_badges)
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
        ".git" | ".jj" | ".svn" | "node_modules" | "target" | "dist" | "build" | ".jcodemunch_index"
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

fn is_test_file(path: &Path) -> bool {
    if should_skip_documentation_path(path) {
        return false;
    }

    if should_skip_non_domain_test_path(path) {
        return false;
    }

    if should_skip_frontend_test_path(path) {
        return false;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default();

    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|part| {
                let lower = part.to_ascii_lowercase();
                lower == "tests" || lower == "__tests__"
            })
            .unwrap_or(false)
    }) || file_name.contains("test") || file_name.contains("spec")
}

fn should_skip_frontend_test_path(path: &Path) -> bool {
    ["ui", "frontend", "storybook", "stories"].iter().any(|segment| has_path_segment(path, segment))
}

fn should_skip_non_domain_test_path(path: &Path) -> bool {
    path_contains_semantic_marker(path, &DOMAIN_TEST_NOISE_MARKERS)
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

fn path_contains_semantic_marker(path: &Path, markers: &[&str]) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default();

    if markers.iter().any(|marker| file_name.contains(marker)) {
        return true;
    }

    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|part| {
                let lower = part.to_ascii_lowercase();
                markers.iter().any(|marker| lower.contains(marker))
            })
            .unwrap_or(false)
    })
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

fn normalize_code_block_snippet(snippet: &str) -> Option<String> {
    let mut normalized = Vec::new();
    let mut previous_blank = false;

    for line in snippet.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            if !previous_blank {
                normalized.push(String::new());
                previous_blank = true;
            }
            continue;
        }

        normalized.push(trimmed.to_string());
        previous_blank = false;
    }

    let joined = normalized.join("\n").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
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
    if !STATE_CALL_NAMES.iter().any(|expected| callee_name == *expected) {
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

fn extract_frontend_contracts_from_content(path: &Path, content: &str) -> Vec<String> {
    let Some((source_text, parse_path)) = frontend_ast_input(path, content) else {
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

    let mut collector = UxAstCollector::new(&source_text);
    collector.visit_program(&parser_return.program);
    collector.finish()
}

fn collect_repo_files(root: &Path) -> Result<Vec<PathBuf>, ExtractionError> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ExtractionError> {
        let entries = std::fs::read_dir(dir).map_err(|e| ExtractionError::IoError {
            file: dir.display().to_string(),
            reason: e.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| ExtractionError::IoError {
                file: dir.display().to_string(),
                reason: e.to_string(),
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|e| ExtractionError::IoError {
                file: path.display().to_string(),
                reason: e.to_string(),
            })?;

            if file_type.is_dir() {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    if should_skip_dir(name) {
                        continue;
                    }
                }
                walk(&path, out)?;
            } else if file_type.is_file() {
                out.push(path);
            }
        }

        Ok(())
    }

    let mut out = Vec::new();
    walk(root, &mut out)?;
    Ok(out)
}

fn read_small_text_file(path: &Path) -> Result<Option<String>, ExtractionError> {
    let metadata = std::fs::metadata(path).map_err(|e| ExtractionError::IoError {
        file: path.display().to_string(),
        reason: e.to_string(),
    })?;

    if metadata.len() > MAX_SCAN_FILE_BYTES {
        return Ok(None);
    }

    let bytes = std::fs::read(path).map_err(|e| ExtractionError::IoError {
        file: path.display().to_string(),
        reason: e.to_string(),
    })?;

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
    pub async fn extract_blob(repo_path: &RepoPath) -> Result<ArtifactBlob, ExtractionError> {
        let root = repo_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let files = collect_repo_files(&root)?;
            let mut intents = BTreeSet::new();

            for path in files.into_iter().filter(|path| is_test_file(path) && has_code_extension(path)) {
                let Some(content) = read_small_text_file(&path)? else {
                    continue;
                };
                let rel = relative_display(&root, &path);
                let mut waiting_attr = false;

                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if trimmed.starts_with("#[test")
                        || trimmed.starts_with("#[tokio::test")
                        || trimmed.starts_with("#[rstest")
                    {
                        waiting_attr = true;
                        continue;
                    }

                    if trimmed.starts_with("async fn ") || trimmed.starts_with("fn ") {
                        let name = trimmed
                            .trim_start_matches("async ")
                            .trim_start_matches("fn ")
                            .split('(')
                            .next()
                            .unwrap_or(trimmed)
                            .trim();
                        if waiting_attr || name.to_ascii_lowercase().contains("test") {
                            intents.insert(format!("{} :: {}", rel, trimmed.trim_end_matches('{').trim()));
                        }
                        waiting_attr = false;
                        continue;
                    }

                    waiting_attr = false;

                    if trimmed.starts_with("def test_") || trimmed.starts_with("async def test_") {
                        intents.insert(format!("{} :: {}", rel, trimmed.trim_end_matches(':').trim()));
                    } else if trimmed.starts_with("func Test") {
                        intents.insert(format!("{} :: {}", rel, trimmed.trim_end_matches('{').trim()));
                    } else if trimmed.starts_with("test(") || trimmed.starts_with("it(") {
                        intents.insert(format!("{} :: {}", rel, trimmed.trim_end_matches('{').trim()));
                    }
                }
            }

            let body = if intents.is_empty() {
                default_test_intent_message()
            } else {
                let mut ordered = intents.into_iter().collect::<Vec<_>>();
                ordered.sort_by(|left, right| {
                    let left_path = left.split(" :: ").next().unwrap_or(left.as_str());
                    let right_path = right.split(" :: ").next().unwrap_or(right.as_str());
                    test_intent_priority(left_path)
                        .cmp(&test_intent_priority(right_path))
                        .then_with(|| left_path.cmp(right_path))
                        .then_with(|| left.cmp(right))
                });
                truncate_utf8(
                    &ordered.join("\n"),
                    TEST_INTENT_BLOB_MAX_CHARS,
                    TEST_INTENT_BLOB_MAX_CHARS,
                )
            };

            Ok(blob_from_text("blob_03_test_intent", body))
        })
        .await
        .map_err(|e| ExtractionError::IoError {
            file: "blob_03_test_intent".to_string(),
            reason: e.to_string(),
        })?
    }
}

fn test_intent_priority(path: &str) -> usize {
    let normalized = path.to_ascii_lowercase();
    if normalized.contains("/src/core/")
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

                if !semantics.is_empty() {
                    let section = format!("### {}\n{}", rel, semantics.join("\n"));
                    sections.push(section);
                }
            }

            let body = if sections.is_empty() {
                default_ux_contracts_message()
            } else {
                truncate_utf8(
                    &sections.join("\n\n"),
                    UX_CONTRACTS_BLOB_MAX_CHARS,
                    UX_CONTRACTS_BLOB_MAX_CHARS,
                )
            };

            Ok(blob_from_text("blob_03_ux_contracts", body))
        })
        .await
        .map_err(|e| ExtractionError::IoError {
            file: "blob_03_ux_contracts".to_string(),
            reason: e.to_string(),
        })?
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
            return Err(ExtractionError::RequiredArtifactMissing {
                artifact_type: "blob_07_ops_blueprint".to_string(),
                candidates: "Dockerfile, docker-compose.yml, docker-compose.yaml, .github/workflows/*.yml".to_string(),
            });
        }

        let packed = truncate_utf8(&sections.join("\n"), OPS_BLOB_MAX_CHARS, OPS_BLOB_MAX_CHARS);
        if packed.trim().is_empty() {
            return Err(ExtractionError::EmptyArtifact {
                artifact_type: "blob_07_ops_blueprint".to_string(),
                file: "ops_blueprint_bundle".to_string(),
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
                while let Some(entry) = entries.next_entry().await.map_err(|e| ExtractionError::IoError {
                    file: ".github/workflows".to_string(),
                    reason: e.to_string(),
                })? {
                    let file_type = entry.file_type().await.map_err(|e| ExtractionError::IoError {
                        file: entry.path().display().to_string(),
                        reason: e.to_string(),
                    })?;

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
        let mut sections = Vec::new();

        for file_name in ["Cargo.toml", "package.json"] {
            let path = input.repo_path.join(file_name);
            let metadata = match fs::metadata(&path).await {
                Ok(metadata) => metadata,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    return Err(ExtractionError::IoError {
                        file: file_name.to_string(),
                        reason: e.to_string(),
                    });
                }
            };

            let size = metadata.len();
            if size > MAX_MANIFEST_SIZE {
                return Err(ExtractionError::FileTooLarge {
                    file: file_name.to_string(),
                    size_bytes: size,
                    limit_bytes: MAX_MANIFEST_SIZE,
                });
            }

            let content = fs::read_to_string(&path).await.map_err(|e| ExtractionError::IoError {
                file: file_name.to_string(),
                reason: e.to_string(),
            })?;

            if content.trim().is_empty() {
                return Err(ExtractionError::EmptyArtifact {
                    artifact_type: "blob_02_dependency_manifest".to_string(),
                    file: file_name.to_string(),
                });
            }

            if let Some(filtered) = Self::extract_manifest_blob_sections(file_name, &content)? {
                sections.push(format!("### {}\n{}\n", file_name, filtered.trim()));
            }
        }

        if sections.is_empty() {
            return Err(ExtractionError::RequiredArtifactMissing {
                artifact_type: "blob_02_dependency_manifest".to_string(),
                candidates: "Cargo.toml, package.json".to_string(),
            });
        }

        let packed = truncate_utf8(
            &sections.join("\n"),
            MANIFEST_BLOB_MAX_CHARS,
            MANIFEST_BLOB_MAX_CHARS,
        );
        if packed.trim().is_empty() {
            return Err(ExtractionError::EmptyArtifact {
                artifact_type: "blob_02_dependency_manifest".to_string(),
                file: "dependency_manifest_bundle".to_string(),
            });
        }

        Ok(ArtifactBlob {
            artifact_type: "blob_02_dependency_manifest".to_string(),
            payload_blob: packed.into_bytes(),
        })
    }

    fn extract_manifest_blob_sections(file_name: &str, content: &str) -> Result<Option<String>, ExtractionError> {
        match file_name {
            "Cargo.toml" => Self::extract_cargo_blob_sections(content, file_name),
            "package.json" => Self::extract_package_json_blob_sections(content, file_name),
            _ => Ok(None),
        }
    }

    fn extract_cargo_blob_sections(content: &str, file: &str) -> Result<Option<String>, ExtractionError> {
        let document: toml::Value = toml::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        let mut sections = Vec::new();

        if let Some(section) = Self::render_toml_section("dependencies", document.get("dependencies"), file)? {
            sections.push(section);
        }
        if let Some(section) = Self::render_toml_section(
            "workspace.dependencies",
            document.get("workspace").and_then(|workspace| workspace.get("dependencies")),
            file,
        )? {
            sections.push(section);
        }
        if let Some(section) = Self::render_toml_section("build-dependencies", document.get("build-dependencies"), file)? {
            sections.push(section);
        }
        if let Some(section) = Self::render_toml_section("dev-dependencies", document.get("dev-dependencies"), file)? {
            sections.push(section);
        }

        if sections.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sections.join("\n\n")))
        }
    }

    fn render_toml_section(
        section_name: &str,
        value: Option<&toml::Value>,
        file: &str,
    ) -> Result<Option<String>, ExtractionError> {
        let Some(value) = value else {
            return Ok(None);
        };
        let Some(table) = value.as_table() else {
            return Err(ExtractionError::ParseError {
                file: file.to_string(),
                reason: format!("Secao [{}] nao e uma tabela TOML valida", section_name),
            });
        };
        if table.is_empty() {
            return Ok(None);
        }

        let mut lines = vec![format!("[{}]", section_name)];
        for (name, entry) in table {
            lines.push(format!("{} = {}", name, Self::render_toml_value(entry)));
        }
        Ok(Some(lines.join("\n")))
    }

    fn render_toml_value(value: &toml::Value) -> String {
        match value {
            toml::Value::Array(items) => {
                let rendered = items
                    .iter()
                    .map(Self::render_toml_value)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", rendered)
            }
            toml::Value::Table(table) => {
                let rendered = table
                    .iter()
                    .map(|(key, value)| format!("{} = {}", key, Self::render_toml_value(value)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", rendered)
            }
            _ => value.to_string(),
        }
    }

    fn extract_package_json_blob_sections(content: &str, file: &str) -> Result<Option<String>, ExtractionError> {
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

        let mut sections = Vec::new();
        for key in ["dependencies", "devDependencies", "peerDependencies"] {
            let Some(value) = root.get(key) else {
                continue;
            };
            let Some(object) = value.as_object() else {
                return Err(ExtractionError::ParseError {
                    file: file.to_string(),
                    reason: format!("Secao '{}' precisa ser um objeto JSON", key),
                });
            };
            if object.is_empty() {
                continue;
            }
            sections.push(Self::render_json_dependency_section(key, object));
        }

        if sections.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sections.join("\n\n")))
        }
    }

    fn render_json_dependency_section(
        key: &str,
        object: &serde_json::Map<String, serde_json::Value>,
    ) -> String {
        let mut lines = vec![format!("\"{}\": {{", key)];
        let total = object.len();
        for (index, (name, value)) in object.iter().enumerate() {
            let suffix = if index + 1 == total { "" } else { "," };
            let rendered_name = match serde_json::to_string(name) {
                Ok(text) => text,
                Err(_) => format!("\"{}\"", name),
            };
            let rendered_value = match serde_json::to_string(value) {
                Ok(text) => text,
                Err(_) => "\"*\"".to_string(),
            };
            lines.push(format!("  {}: {}{}", rendered_name, rendered_value, suffix));
        }
        lines.push("}".to_string());
        lines.join("\n")
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

            let content = fs::read_to_string(&path).await.map_err(|e| ExtractionError::IoError {
                file: file_name.to_string(),
                reason: e.to_string(),
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

        assert_eq!(blobs.len(), 3);
        assert!(blobs.iter().any(|blob| blob.artifact_type == "blob_01_promessa_readme"));
        assert!(blobs.iter().any(|blob| blob.artifact_type == "blob_02_dependency_manifest"));
        assert!(blobs.iter().any(|blob| blob.artifact_type == "blob_07_ops_blueprint"));
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

        fs::write(dir.path().join("Cargo.toml"), cargo_toml).await.unwrap();
        fs::write(dir.path().join("package.json"), package_json).await.unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = ManifestExtractor::extract_blob(ManifestInput { repo_path: &repo_path }).await.unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(text.contains("[dependencies]"));
        assert!(text.contains("serde = \"1.0\""));
        assert!(text.contains("[workspace.dependencies]"));
        assert!(text.contains("tokio = { features = [\"full\"], version = \"1\" }") || text.contains("tokio = { version = \"1\", features = [\"full\"] }"));
        assert!(text.contains("[build-dependencies]"));
        assert!(text.contains("cc = \"1.1\""));
        assert!(text.contains("[dev-dependencies]"));
        assert!(text.contains("\"dependencies\": {"));
        assert!(text.contains("\"react\": \"^18.2.0\""));
        assert!(text.contains("\"devDependencies\": {"));
        assert!(text.contains("\"typescript\": \"^5.4.0\""));
        assert!(text.contains("\"peerDependencies\": {"));
        assert!(text.contains("\"zod\": \"^3.23.0\""));

        assert!(!text.contains("[package]"));
        assert!(!text.contains("\"name\": \"demo\""));
        assert!(!text.contains("\"version\": \"0.1.0\""));
    }

    #[tokio::test]
    async fn test_test_intent_skips_documentation_paths() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("docs/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/tests")).await.unwrap();

        fs::write(
            dir.path().join("docs/tests/readme_test.rs"),
            "#[test]\nfn test_docs_should_not_enter_blob() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/tests/core_rules.rs"),
            "#[test]\nfn test_real_business_rule() {}\n",
        )
        .await
        .unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = TestIntentExtractor::extract_blob(&repo_path).await.unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(!text.contains("docs/tests/readme_test.rs"));
        assert!(text.contains("crates/app/tests/core_rules.rs :: fn test_real_business_rule()"));
    }

    #[tokio::test]
    async fn test_test_intent_skips_mock_fixture_and_e2e_noise() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("crates/app/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/mock/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/fixtures/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/test_support/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/e2e/tests")).await.unwrap();
        fs::create_dir_all(dir.path().join("crates/app/integration_mocks/tests")).await.unwrap();

        fs::write(
            dir.path().join("crates/app/tests/domain_rules.rs"),
            "#[test]\nfn test_domain_logic_stays() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/mock/tests/mock_rule.rs"),
            "#[test]\nfn test_mock_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/fixtures/tests/fixture_rule.rs"),
            "#[test]\nfn test_fixture_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/test_support/tests/support_rule.rs"),
            "#[test]\nfn test_support_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/e2e/tests/e2e_rule.rs"),
            "#[test]\nfn test_e2e_should_be_ignored() {}\n",
        )
        .await
        .unwrap();
        fs::write(
            dir.path().join("crates/app/integration_mocks/tests/integration_mock_rule.rs"),
            "#[test]\nfn test_integration_mock_should_be_ignored() {}\n",
        )
        .await
        .unwrap();

        let repo_path = RepoPath(dir.path().to_path_buf());
        let blob = TestIntentExtractor::extract_blob(&repo_path).await.unwrap();
        let text = String::from_utf8_lossy(&blob.payload_blob);

        assert!(text.contains("crates/app/tests/domain_rules.rs :: fn test_domain_logic_stays()"));
        assert!(!text.contains("mock_rule.rs"));
        assert!(!text.contains("fixture_rule.rs"));
        assert!(!text.contains("support_rule.rs"));
        assert!(!text.contains("e2e_rule.rs"));
        assert!(!text.contains("integration_mock_rule.rs"));
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
        assert!(text.contains("ui/desktop/src/components/AppShell.tsx"));
        assert!(text.contains("type AppShellProps = { title: string }"));
        assert!(text.contains("props: AppShellProps"));
        assert!(text.contains("state [state] = useState()"));
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
        assert!(text.contains("ui/desktop/src/components/AppShell.tsx"));
        assert!(text.contains("interface AppShellProps"));
        assert!(text.contains("state [open, setOpen] = useState()"));
        assert!(text.contains("props: AppShellProps"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains("toast.error"));
        assert!(!text.contains("<section>"));
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
        
        // Criar subdiretório para provar que a recursão ignora
        let nested_dir = workflows_dir.join("nested");
        fs::create_dir_all(&nested_dir).await.unwrap();
        fs::write(nested_dir.join("ignored.yml"), "should be ignored").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = OpsBlueprintExtractor::extract(OpsInput { repo_path: &repo_path }).await.unwrap();
        
        // Dockerfile/Makefile não existem aqui, então só os 2 workflows da raiz da pasta
        assert_eq!(result.infra_files.len(), 2);
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/ci.yml"));
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/deploy.yaml"));
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
