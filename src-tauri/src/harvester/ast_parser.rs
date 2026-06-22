use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use thiserror::Error;
use tree_sitter::{Language, Node, Parser};
use tree_sitter_language_pack::{process, ProcessConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAstArtifacts {
    pub repo_outline_blob: Vec<u8>,
    pub architecture_map_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFile {
    relative_path: String,
    language: String,
    signatures: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AstParserError {
    #[error("Repositorio vazio ou sem arquivos-fonte elegiveis em '{path}'")]
    EmptyRepository { path: String },

    #[error("Falha ao caminhar repositório '{path}': {reason}")]
    WalkFailure { path: String, reason: String },

    #[error("Falha ao ler arquivo '{file}': {reason}")]
    ReadFailure { file: String, reason: String },

    #[error("Falha ao extrair AST de '{file}' ({language}): {reason}")]
    ParseFailure {
        file: String,
        language: String,
        reason: String,
    },

    #[error("Nenhum símbolo estrutural foi extraído do repositório '{path}'")]
    NoStructuralSymbols { path: String },

    #[error("Falha ao serializar health report AST nativo: {reason}")]
    SerializationFailure { reason: String },
}

pub fn extract_repository_outline_native(
    repo_path: &Path,
    max_outline_chars: usize,
    max_architecture_chars: usize,
    max_health_chars: usize,
) -> Result<NativeAstArtifacts, AstParserError> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let source_files = collect_source_files(&repo_root)?;
    if source_files.is_empty() {
        return Err(AstParserError::EmptyRepository {
            path: repo_root.display().to_string(),
        });
    }

    let mut parsed_files = Vec::new();
    let mut languages = BTreeMap::<String, usize>::new();
    let mut total_signatures = 0usize;
    let mut total_import_edges = 0usize;
    let mut directories = BTreeMap::<String, Vec<String>>::new();

    for file_path in source_files {
        let relative_path = sanitize_relative_path(&repo_root, &file_path);
        let Some(language) = detect_language(&file_path) else {
            continue;
        };

        let source = std::fs::read_to_string(&file_path).map_err(|e| AstParserError::ReadFailure {
            file: relative_path.clone(),
            reason: e.to_string(),
        })?;

        let (signatures, import_edges) =
            extract_structural_signatures(&source, &language, &relative_path)?;

        if signatures.is_empty() {
            continue;
        }

        total_import_edges += import_edges;
        total_signatures += signatures.len();
        *languages.entry(language.clone()).or_insert(0) += 1;

        let dir_key = directory_key(&relative_path);
        directories
            .entry(dir_key)
            .or_default()
            .push(format!("{relative_path} [{} symbols]", signatures.len()));

        parsed_files.push(ParsedFile {
            relative_path,
            language,
            signatures,
        });
    }

    if parsed_files.is_empty() || total_signatures == 0 {
        return Err(AstParserError::NoStructuralSymbols {
            path: repo_root.display().to_string(),
        });
    }

    parsed_files.sort_by(|left, right| {
        right
            .signatures
            .len()
            .cmp(&left.signatures.len())
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });

    let repo_outline = build_repo_outline(
        &repo_root,
        &parsed_files,
        max_outline_chars,
    );
    let architecture_map = build_architecture_map(&directories, max_architecture_chars);
    let health_report = build_health_report(
        &repo_root,
        &languages,
        parsed_files.len(),
        total_signatures,
        total_import_edges,
        max_health_chars,
    )?;

    Ok(NativeAstArtifacts {
        repo_outline_blob: repo_outline.into_bytes(),
        architecture_map_blob: architecture_map.into_bytes(),
        health_report_blob: health_report.into_bytes(),
    })
}

pub fn derive_scannable_roots_native(
    repo_path: &Path,
    max_roots: usize,
) -> Result<Vec<PathBuf>, AstParserError> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let source_files = collect_source_files(&repo_root)?;
    if source_files.is_empty() {
        return Err(AstParserError::EmptyRepository {
            path: repo_root.display().to_string(),
        });
    }

    let relative_paths = source_files
        .iter()
        .map(|path| sanitize_relative_path(&repo_root, path))
        .collect::<Vec<_>>();
    let roots = derive_scannable_roots_from_relative_paths(&relative_paths, max_roots);
    Ok(roots
        .into_iter()
        .map(|root| repo_root.join(root))
        .collect::<Vec<_>>())
}

const SCANNABLE_ROOT_FILE_THRESHOLD: usize = 80;
const SCANNABLE_ROOT_MAX_SPLIT_DEPTH: usize = 3;

fn derive_scannable_roots_from_relative_paths(
    relative_paths: &[String],
    max_roots: usize,
) -> Vec<String> {
    let mut grouped = BTreeMap::<String, Vec<Vec<String>>>::new();

    for relative_path in relative_paths {
        if should_skip_scannable_relative_path(relative_path) {
            continue;
        }
        let segments = split_relative_segments(relative_path);
        let Some(anchor_idx) = scannable_anchor_index(&segments) else {
            continue;
        };
        let anchor_root = segments[..=anchor_idx].join("/");
        let tail_dirs = if segments.len() > anchor_idx + 1 {
            segments[anchor_idx + 1..segments.len().saturating_sub(1)].to_vec()
        } else {
            Vec::new()
        };
        grouped.entry(anchor_root).or_default().push(tail_dirs);
    }

    let mut selected = BTreeSet::new();
    for (anchor_root, tails) in grouped {
        collect_scannable_roots(
            &anchor_root,
            &tails,
            0,
            &mut selected,
        );
    }

    selected.into_iter().take(max_roots).collect()
}

fn collect_scannable_roots(
    prefix: &str,
    tails: &[Vec<String>],
    depth: usize,
    out: &mut BTreeSet<String>,
) {
    if tails.len() <= SCANNABLE_ROOT_FILE_THRESHOLD || depth >= SCANNABLE_ROOT_MAX_SPLIT_DEPTH {
        out.insert(prefix.to_string());
        return;
    }

    let mut by_child = BTreeMap::<String, Vec<Vec<String>>>::new();
    let mut direct_files = 0usize;
    for tail in tails {
        if let Some((head, rest)) = tail.split_first() {
            by_child
                .entry(head.clone())
                .or_default()
                .push(rest.to_vec());
        } else {
            direct_files += 1;
        }
    }

    if by_child.is_empty() {
        out.insert(prefix.to_string());
        return;
    }

    if by_child.len() == 1 && direct_files == 0 {
        let (child, child_tails) = by_child.into_iter().next().unwrap();
        let child_prefix = format!("{prefix}/{child}");
        collect_scannable_roots(&child_prefix, &child_tails, depth + 1, out);
        return;
    }

    if by_child.len() < 2 {
        out.insert(prefix.to_string());
        return;
    }

    if direct_files > 0 {
        out.insert(prefix.to_string());
    }

    for (child, child_tails) in by_child {
        let child_prefix = format!("{prefix}/{child}");
        collect_scannable_roots(&child_prefix, &child_tails, depth + 1, out);
    }
}

fn split_relative_segments(relative_path: &str) -> Vec<String> {
    relative_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>()
}

fn scannable_anchor_index(segments: &[String]) -> Option<usize> {
    segments.iter().position(|segment| {
        let lower = segment.to_ascii_lowercase();
        matches!(
            lower.as_str(),
            "src" | "lib" | "app" | "cmd" | "internal" | "server" | "client" | "scripts" | "script"
        )
    })
}

fn should_skip_scannable_relative_path(relative_path: &str) -> bool {
    let normalized = relative_path.replace('\\', "/").to_ascii_lowercase();
    let segments = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.iter().any(|segment| {
        matches!(
            *segment,
            "test"
                | "tests"
                | "__tests__"
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
                | "benchmarking"
                | "coverage"
                | "generated"
                | ".svelte-kit"
                | ".next"
                | ".nuxt"
                | ".storybook"
                | "storybook-static"
        )
    }) {
        return true;
    }

    normalized.ends_with("/output.json")
        || normalized.ends_with(".min.js")
        || normalized.ends_with(".min.cjs")
        || normalized.ends_with(".min.mjs")
        || normalized.ends_with(".bundle.js")
        || normalized.contains(".generated.")
}

fn collect_source_files(repo_root: &Path) -> Result<Vec<PathBuf>, AstParserError> {
    let mut builder = WalkBuilder::new(repo_root);
    builder.hidden(false);
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);
    builder.require_git(false);
    builder.filter_entry(|entry| {
        let path = entry.path();
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            return !should_skip_dir(name);
        }
        true
    });

    let mut files = Vec::new();
    for item in builder.build() {
        let entry = item.map_err(|e| AstParserError::WalkFailure {
            path: repo_root.display().to_string(),
            reason: e.to_string(),
        })?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if should_skip_file(entry.path()) {
            continue;
        }
        files.push(entry.into_path());
    }
    files.sort();
    Ok(files)
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
            | "coverage"
            | "vendor"
            | ".native_ast_cache"
            | "__pycache__"
            | ".venv"
            | "venv"
            | "documentation"
            | "docs"
            | "examples"
            | "evals"
    )
}

fn should_skip_file(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("/documentation/")
        || normalized.contains("/docs/")
        || normalized.contains("/examples/")
        || normalized.contains("/evals/")
        || normalized.contains("/node_modules/")
        || normalized.contains("/target/")
        || normalized.contains("/vendor/")
        || normalized.contains("/dist/")
        || normalized.contains("/build/")
        || normalized.contains("/.git/")
        || normalized.contains("/.native_ast_cache/")
    {
        return true;
    }

    detect_language(path).is_none()
}

fn detect_language(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let language = match ext.as_str() {
        "rs" => "rust",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "swift" => "swift",
        "cs" => "c_sharp",
        "yaml" | "yml" => "yaml",
        "rb" => "ruby",
        "php" => "php",
        "scala" => "scala",
        "sh" | "bash" | "zsh" => "bash",
        "dart" => "dart",
        "lua" => "lua",
        "ex" | "exs" => "elixir",
        "zig" => "zig",
        "sol" => "solidity",
        _ => return None,
    };
    Some(language.to_string())
}

fn extract_structural_signatures(
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<String>, usize), AstParserError> {
    match language {
        "c_sharp" | "yaml" => extract_with_tree_sitter_fallback(source, language, relative_path),
        _ => extract_with_language_pack(source, language, relative_path),
    }
}

fn extract_with_language_pack(
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<String>, usize), AstParserError> {
    let mut config = ProcessConfig::new(language);
    config.structure = true;
    config.imports = true;
    config.exports = false;
    config.comments = false;
    config.docstrings = false;
    config.symbols = false;
    config.diagnostics = true;

    let processed = process(source, &config).map_err(|e| AstParserError::ParseFailure {
        file: relative_path.to_string(),
        language: language.to_string(),
        reason: e.to_string(),
    })?;

    let signatures = processed
        .structure
        .iter()
        .flat_map(flatten_structure_signatures)
        .collect::<Vec<_>>();

    Ok((signatures, processed.imports.len()))
}

fn extract_with_tree_sitter_fallback(
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<String>, usize), AstParserError> {
    let ts_language = fallback_tree_sitter_language(language).ok_or_else(|| AstParserError::ParseFailure {
        file: relative_path.to_string(),
        language: language.to_string(),
        reason: "grammar tree-sitter nativa indisponivel".to_string(),
    })?;

    let mut parser = Parser::new();
    parser
        .set_language(&ts_language)
        .map_err(|e| AstParserError::ParseFailure {
            file: relative_path.to_string(),
            language: language.to_string(),
            reason: format!("falha ao registrar grammar nativa: {e}"),
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| AstParserError::ParseFailure {
        file: relative_path.to_string(),
        language: language.to_string(),
        reason: "parser nativo retornou arvore vazia".to_string(),
    })?;

    let mut signatures = Vec::new();
    collect_fallback_signatures(language, source.as_bytes(), tree.root_node(), &mut signatures);
    signatures.sort();
    signatures.dedup();
    if signatures.is_empty() {
        signatures.push(match language {
            "yaml" => "yaml document <root>".to_string(),
            "c_sharp" => "c# compilation_unit <root>".to_string(),
            _ => format!("{language} <root>"),
        });
    }

    Ok((signatures, 0))
}

fn fallback_tree_sitter_language(language: &str) -> Option<Language> {
    match language {
        "c_sharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "yaml" => Some(tree_sitter_yaml::LANGUAGE.into()),
        _ => None,
    }
}

fn collect_fallback_signatures(
    language: &str,
    source: &[u8],
    node: Node<'_>,
    out: &mut Vec<String>,
) {
    if let Some(signature) = fallback_signature_for_node(language, source, node) {
        out.push(signature);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_fallback_signatures(language, source, child, out);
    }
}

fn fallback_signature_for_node(language: &str, source: &[u8], node: Node<'_>) -> Option<String> {
    match language {
        "c_sharp" => csharp_signature_for_node(source, node),
        "yaml" => yaml_signature_for_node(source, node),
        _ => None,
    }
}

fn csharp_signature_for_node(source: &[u8], node: Node<'_>) -> Option<String> {
    let label = match node.kind() {
        "namespace_declaration" => "namespace",
        "class_declaration" => "class",
        "interface_declaration" => "interface",
        "struct_declaration" => "struct",
        "enum_declaration" => "enum",
        "record_declaration" => "record",
        "method_declaration" => "method",
        "constructor_declaration" => "constructor",
        "property_declaration" => "property",
        "field_declaration" => "field",
        _ => return None,
    };

    let name = node_text_by_field(node, source, &["name", "identifier"])
        .unwrap_or_else(|| compact_node_text(node, source, 80));
    Some(format!("c# {label} {name}"))
}

fn yaml_signature_for_node(source: &[u8], node: Node<'_>) -> Option<String> {
    match node.kind() {
        "stream" => Some("yaml stream <root>".to_string()),
        "document" => Some("yaml document <document>".to_string()),
        "block_mapping_pair" | "flow_pair" => {
            let key = node_text_by_field(node, source, &["key"])
                .or_else(|| node.named_child(0).map(|child| compact_node_text(child, source, 80)))
                .unwrap_or_else(|| "<pair>".to_string());
            Some(format!("yaml key {key}"))
        }
        "block_sequence_item" => Some(format!(
            "yaml sequence-item {}",
            compact_node_text(node, source, 80)
        )),
        _ => None,
    }
}

fn node_text_by_field(node: Node<'_>, source: &[u8], fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| node.child_by_field_name(field))
        .map(|child| compact_node_text(child, source, 80))
        .filter(|value| !value.is_empty())
}

fn compact_node_text(node: Node<'_>, source: &[u8], max_chars: usize) -> String {
    let raw = node.utf8_text(source).unwrap_or("").replace('\n', " ");
    let compact = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(compact.trim(), max_chars)
}

fn flatten_structure_signatures(
    item: &tree_sitter_language_pack::StructureItem,
) -> Vec<String> {
    let mut out = Vec::new();
    let rendered = render_signature(item);
    if !rendered.is_empty() {
        out.push(rendered);
    }
    for child in &item.children {
        out.extend(flatten_structure_signatures(child));
    }
    out
}

fn render_signature(item: &tree_sitter_language_pack::StructureItem) -> String {
    let fallback_name = item.name.clone().unwrap_or_else(|| "<anonymous>".to_string());
    let mut signature = item
        .signature
        .clone()
        .unwrap_or_else(|| format!("{:?} {}", item.kind, fallback_name));
    signature = signature.replace('\n', " ");
    signature = signature.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(visibility) = &item.visibility {
        if !visibility.trim().is_empty() && !signature.starts_with(visibility) {
            signature = format!("{visibility} {signature}");
        }
    }
    signature.trim().to_string()
}

fn build_repo_outline(
    repo_root: &Path,
    parsed_files: &[ParsedFile],
    max_chars: usize,
) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("repo");
    let mut out = String::new();
    out.push_str("# Repository Outline\n\n");
    out.push_str(&format!("repo: {repo_name}\n"));
    out.push_str(&format!("symbol_files: {}\n", parsed_files.len()));
    out.push_str("source: native-rust tree-sitter-language-pack\n\n");
    out.push_str("## Indexed Symbol Files\n");
    for file in parsed_files {
        out.push_str(&format!(
            "- {} [{}; {} symbols]\n",
            file.relative_path,
            file.language,
            file.signatures.len()
        ));
    }
    out.push_str("\n## AST Blueprint\n\n");
    for file in parsed_files {
        out.push_str(&format!("[{}]\n", file.relative_path));
        for signature in &file.signatures {
            out.push_str("- ");
            out.push_str(signature);
            out.push('\n');
        }
        out.push('\n');
        if out.len() >= max_chars {
            break;
        }
    }
    truncate_chars(&out, max_chars)
}

fn build_architecture_map(
    directories: &BTreeMap<String, Vec<String>>,
    max_chars: usize,
) -> String {
    let mut out = String::from("# Architecture Map\n\n");
    for (directory, files) in directories {
        out.push_str(&format!("[{}]\n", directory));
        for file in files {
            out.push_str("- ");
            out.push_str(file);
            out.push('\n');
        }
        out.push('\n');
        if out.len() >= max_chars {
            break;
        }
    }
    truncate_chars(&out, max_chars)
}

fn build_health_report(
    repo_root: &Path,
    languages: &BTreeMap<String, usize>,
    parsed_files: usize,
    total_signatures: usize,
    total_import_edges: usize,
    max_chars: usize,
) -> Result<String, AstParserError> {
    let repo_name = repo_root
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("repo");
    let payload = serde_json::json!({
        "source": "native-rust tree-sitter-language-pack",
        "repo": repo_name,
        "parsed_files": parsed_files,
        "total_signatures": total_signatures,
        "total_import_edges": total_import_edges,
        "languages": languages,
    });
    let text = serde_json::to_string(&payload).map_err(|e| AstParserError::SerializationFailure {
        reason: e.to_string(),
    })?;
    Ok(truncate_chars(&text, max_chars))
}

fn sanitize_relative_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn directory_key(path: &str) -> String {
    let parent = Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| ".".to_string());
    if parent.is_empty() {
        ".".to_string()
    } else {
        parent
    }
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }
    content.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_expected_languages() {
        assert_eq!(detect_language(Path::new("src/lib.rs")).as_deref(), Some("rust"));
        assert_eq!(detect_language(Path::new("src/app.ts")).as_deref(), Some("typescript"));
        assert_eq!(detect_language(Path::new("main.py")).as_deref(), Some("python"));
        assert_eq!(detect_language(Path::new("Program.cs")).as_deref(), Some("c_sharp"));
        assert_eq!(detect_language(Path::new("config.yaml")).as_deref(), Some("yaml"));
        assert_eq!(detect_language(Path::new("config.yml")).as_deref(), Some("yaml"));
        assert_eq!(detect_language(Path::new("notes.md")), None);
    }

    #[test]
    fn fallback_parser_extracts_csharp_signatures() {
        let source = r#"
namespace Demo;
public class Greeter {
    public string Name { get; set; }
    public void Run() {}
}
"#;
        let (signatures, imports) =
            extract_with_tree_sitter_fallback(source, "c_sharp", "Program.cs").unwrap();
        assert_eq!(imports, 0);
        assert!(signatures.iter().any(|item| item.contains("c# class Greeter")));
        assert!(signatures.iter().any(|item| item.contains("c# method Run")));
    }

    #[test]
    fn fallback_parser_extracts_yaml_signatures() {
        let source = r#"
name: ci
jobs:
  build:
    steps:
      - run: cargo test
"#;
        let (signatures, imports) =
            extract_with_tree_sitter_fallback(source, "yaml", ".github/workflows/ci.yml").unwrap();
        assert_eq!(imports, 0);
        assert!(signatures.iter().any(|item| item.contains("yaml key name")));
        assert!(signatures.iter().any(|item| item.contains("yaml key jobs")));
    }

    #[test]
    fn derive_scannable_roots_skips_toxic_js_fixture_paths() {
        let paths = vec![
            "packages/web/src/lib/index.ts".to_string(),
            "packages/web/tests/runtime/sample.spec.ts".to_string(),
            "packages/web/__mocks__/browser.ts".to_string(),
            "playgrounds/sandbox/src/main.ts".to_string(),
            "packages/web/src/generated/client.generated.ts".to_string(),
        ];

        let roots = derive_scannable_roots_from_relative_paths(&paths, 16);

        assert_eq!(roots, vec!["packages/web/src".to_string()]);
    }

    #[test]
    fn derive_scannable_roots_splits_large_anchor_subtrees_recursively() {
        let mut paths = Vec::new();
        for idx in 0..90 {
            paths.push(format!("packages/svelte/src/compiler/phases/1-parse/file_{idx}.js"));
            paths.push(format!("packages/svelte/src/compiler/phases/2-analyze/file_{idx}.js"));
            paths.push(format!("packages/svelte/src/internal/runtime/file_{idx}.js"));
        }

        let roots = derive_scannable_roots_from_relative_paths(&paths, 16);

        assert!(roots.contains(&"packages/svelte/src/compiler/phases/1-parse".to_string()));
        assert!(roots.contains(&"packages/svelte/src/compiler/phases/2-analyze".to_string()));
        assert!(roots.contains(&"packages/svelte/src/internal/runtime".to_string()));
        assert!(!roots.contains(&"packages/svelte/src".to_string()));
    }
}
