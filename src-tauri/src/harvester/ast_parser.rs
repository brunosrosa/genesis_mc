use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use thiserror::Error;
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

        let mut config = ProcessConfig::new(&language);
        config.structure = true;
        config.imports = true;
        config.exports = false;
        config.comments = false;
        config.docstrings = false;
        config.symbols = false;
        config.diagnostics = true;

        let processed = process(&source, &config).map_err(|e| AstParserError::ParseFailure {
            file: relative_path.clone(),
            language: language.clone(),
            reason: e.to_string(),
        })?;

        let signatures = processed
            .structure
            .iter()
            .flat_map(flatten_structure_signatures)
            .collect::<Vec<_>>();

        if signatures.is_empty() {
            continue;
        }

        total_import_edges += processed.imports.len();
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
            | ".jcodemunch_index"
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
        || normalized.contains("/.jcodemunch_index/")
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
        assert_eq!(detect_language(Path::new("notes.md")), None);
    }
}
