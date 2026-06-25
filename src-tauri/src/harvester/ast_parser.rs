use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::Regex;
use thiserror::Error;
use tree_sitter::{Language, Node, Parser};
use tree_sitter_language_pack::{process, ProcessConfig};
use tracing::warn;

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
    let architecture_files = collect_architecture_files(&repo_root)?;

    let mut parsed_files = Vec::new();
    let mut languages = BTreeMap::<String, usize>::new();
    let mut total_signatures = 0usize;
    let mut total_import_edges = 0usize;
    for file_path in source_files {
        let relative_path = sanitize_relative_path(&repo_root, &file_path);
        let Some(language) = detect_language(&file_path) else {
            continue;
        };

        let file_size_bytes = match std::fs::metadata(&file_path) {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                warn!(
                    file = %relative_path,
                    error = %err,
                    "ast-native: falha ao ler metadata; descartando arquivo"
                );
                continue;
            }
        };
        if file_size_bytes >= AST_MINIFIED_HEURISTIC_MIN_BYTES && is_probably_minified_source(&file_path) {
            continue;
        }

        let source_bytes = match std::fs::read(&file_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                warn!(
                    file = %relative_path,
                    error = %err,
                    "ast-native: falha ao ler bytes do arquivo; descartando arquivo"
                );
                continue;
            }
        };
        let source = String::from_utf8_lossy(&source_bytes).into_owned();
        if source.trim().is_empty() {
            continue;
        }

        let (signatures, import_edges) = match extract_structural_signatures(&source, &language, &relative_path) {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    file = %relative_path,
                    language = %language,
                    error = %err,
                    "ast-native: extracao estrutural falhou; descartando arquivo"
                );
                continue;
            }
        };

        if signatures.is_empty() {
            continue;
        }

        total_import_edges += import_edges;
        total_signatures += signatures.len();
        *languages.entry(language.clone()).or_insert(0) += 1;

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

    let repo_outline = build_repo_outline(&repo_root, &parsed_files);
    let architecture_map = build_architecture_map(&architecture_files);
    let health_report = build_health_report(
        &repo_root,
        &languages,
        parsed_files.len(),
        total_signatures,
        total_import_edges,
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

pub fn collect_direct_scannable_files(root: &Path) -> Result<Vec<PathBuf>, AstParserError> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(root).map_err(|err| AstParserError::WalkFailure {
        path: root.display().to_string(),
        reason: err.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| AstParserError::WalkFailure {
            path: root.display().to_string(),
            reason: err.to_string(),
        })?;
        let file_type = entry.file_type().map_err(|err| AstParserError::WalkFailure {
            path: root.display().to_string(),
            reason: err.to_string(),
        })?;
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        if should_skip_file(&path) {
            continue;
        }
        let relative_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        if should_skip_scannable_relative_path(&relative_name) {
            continue;
        }
        files.push(path);
    }
    files.sort();
    Ok(files)
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

    if by_child.len() == 1 {
        if let Some((child, child_tails)) = by_child.into_iter().next() {
            let child_prefix = format!("{prefix}/{child}");
            collect_scannable_roots(&child_prefix, &child_tails, depth + 1, out);
            if direct_files > 0 {
                out.insert(prefix.to_string());
            }
        }
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

fn collect_architecture_files(repo_root: &Path) -> Result<Vec<String>, AstParserError> {
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
        let relative_path = sanitize_relative_path(repo_root, entry.path());
        if should_skip_architecture_relative_path(&relative_path) {
            continue;
        }
        if !is_architecture_file_allowed(entry.path()) {
            continue;
        }
        files.push(relative_path);
    }
    files.sort();
    Ok(files)
}

fn should_skip_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
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
            | "benchmarks"
            | "benchmarking"
            | "generated"
            | "__generated__"
            | ".git"
            | ".hg"
            | ".jj"
            | ".svn"
            | ".bzr"
            | ".idea"
            | ".vscode"
            | ".vs"
            | ".fleet"
            | ".history"
            | "node_modules"
            | ".pnpm-store"
            | ".yarn"
            | ".turbo"
            | ".nx"
            | ".next"
            | ".nuxt"
            | ".svelte-kit"
            | ".parcel-cache"
            | ".cache"
            | "target"
            | "dist"
            | "build"
            | "out"
            | "coverage"
            | "vendor"
            | "deps"
            | ".native_ast_cache"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | ".tox"
            | ".nox"
            | ".venv"
            | "venv"
            | "env"
            | ".gradle"
            | ".dart_tool"
            | ".swiftpm"
            | ".build"
            | ".zig-cache"
            | "zig-out"
            | "cmakefiles"
            | "pods"
            | "deriveddata"
            | "bin"
            | "obj"
            | "documentation"
            | "docs"
            | "examples"
            | "evals"
    ) || lower.starts_with("cmake-build-")
}

fn should_skip_file(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("/documentation/")
        || normalized.contains("/docs/")
        || normalized.contains("/examples/")
        || normalized.contains("/evals/")
        || normalized.contains("/mock/")
        || normalized.contains("/mocks/")
        || normalized.contains("/__mocks__/")
        || normalized.contains("/fixture/")
        || normalized.contains("/fixtures/")
        || normalized.contains("/__fixtures__/")
        || normalized.contains("/snapshot/")
        || normalized.contains("/snapshots/")
        || normalized.contains("/__snapshots__/")
        || normalized.contains("/playground/")
        || normalized.contains("/playgrounds/")
        || normalized.contains("/benchmark/")
        || normalized.contains("/benchmarks/")
        || normalized.contains("/benchmarking/")
        || normalized.contains("/generated/")
        || normalized.contains("/__generated__/")
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

    if normalized.ends_with("/output.json")
        || normalized.ends_with(".min.js")
        || normalized.ends_with(".min.cjs")
        || normalized.ends_with(".min.mjs")
        || normalized.ends_with(".bundle.js")
        || normalized.ends_with(".bundle.cjs")
        || normalized.ends_with(".bundle.mjs")
        || normalized.contains(".generated.")
    {
        return true;
    }

    !is_ast_source_file_allowed(path)
}

const AST_MINIFIED_HEURISTIC_MIN_BYTES: u64 = 32_000;
const AST_MINIFIED_PREFIX_BYTES: usize = 8192;
const AST_MINIFIED_WHITESPACE_RATIO_X100: usize = 7;
const AST_MINIFIED_MAX_LINE_LEN: usize = 1200;

fn is_probably_minified_source(path: &Path) -> bool {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let mut buf = vec![0u8; AST_MINIFIED_PREFIX_BYTES];
    let read_len = match file.read(&mut buf) {
        Ok(len) => len,
        Err(_) => return false,
    };
    buf.truncate(read_len);
    if buf.is_empty() {
        return false;
    }

    let sample = String::from_utf8_lossy(&buf);
    let sample = sample.as_ref();
    let mut total = 0usize;
    let mut whitespace = 0usize;
    let mut current_line = 0usize;
    let mut max_line = 0usize;
    for ch in sample.chars() {
        total += 1;
        if ch.is_whitespace() {
            whitespace += 1;
        }
        if ch == '\n' {
            if current_line > max_line {
                max_line = current_line;
            }
            current_line = 0;
        } else {
            current_line += 1;
        }
    }
    if current_line > max_line {
        max_line = current_line;
    }

    if total < 1024 {
        return false;
    }

    let looks_like_single_line_blob = max_line >= AST_MINIFIED_MAX_LINE_LEN;
    let looks_like_low_whitespace = whitespace * 100 < total * AST_MINIFIED_WHITESPACE_RATIO_X100;

    looks_like_single_line_blob && looks_like_low_whitespace
}

fn should_skip_architecture_relative_path(relative_path: &str) -> bool {
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
                | "benchmarks"
                | "benchmarking"
                | "coverage"
                | "generated"
                | "__generated__"
        )
    }) {
        return true;
    }

    normalized.ends_with(".min.js")
        || normalized.ends_with(".min.cjs")
        || normalized.ends_with(".min.mjs")
        || normalized.ends_with(".bundle.js")
        || normalized.ends_with(".bundle.cjs")
        || normalized.ends_with(".bundle.mjs")
        || normalized.ends_with(".d.ts")
        || normalized.ends_with(".generated.rs")
        || normalized.contains(".generated.")
}

fn detect_language(path: &Path) -> Option<String> {
    let ext = normalized_source_extension(path)?;
    let language = match ext.as_str() {
        "rs" => "rust",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
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
        "dart" => "dart",
        "lua" => "lua",
        "ex" | "exs" => "elixir",
        "svelte" => "svelte",
        "zig" => "zig",
        "sol" => "solidity",
        _ => return None,
    };
    Some(language.to_string())
}

fn is_ast_source_file_allowed(path: &Path) -> bool {
    normalized_source_extension(path).is_some()
}

fn normalized_source_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs"
        | "js"
        | "jsx"
        | "mjs"
        | "cjs"
        | "ts"
        | "tsx"
        | "mts"
        | "cts"
        | "py"
        | "go"
        | "java"
        | "kt"
        | "kts"
        | "c"
        | "h"
        | "cc"
        | "cpp"
        | "cxx"
        | "hpp"
        | "hh"
        | "hxx"
        | "swift"
        | "cs"
        | "rb"
        | "php"
        | "scala"
        | "dart"
        | "lua"
        | "ex"
        | "exs"
        | "svelte"
        | "zig"
        | "sol" => Some(ext),
        _ => None,
    }
}

fn is_architecture_file_allowed(path: &Path) -> bool {
    architecture_file_kind(path).is_some()
}

fn architecture_file_kind(path: &Path) -> Option<&'static str> {
    let file_name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if is_named_architecture_file(&file_name) || has_named_architecture_suffix(&file_name) {
        return Some("project");
    }

    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs"
        | "js"
        | "jsx"
        | "mjs"
        | "cjs"
        | "ts"
        | "tsx"
        | "mts"
        | "cts"
        | "py"
        | "go"
        | "java"
        | "kt"
        | "kts"
        | "c"
        | "h"
        | "cc"
        | "cpp"
        | "cxx"
        | "hpp"
        | "hh"
        | "hxx"
        | "swift"
        | "cs"
        | "fs"
        | "fsi"
        | "fsx"
        | "rb"
        | "php"
        | "scala"
        | "sc"
        | "dart"
        | "lua"
        | "ex"
        | "exs"
        | "erl"
        | "hrl"
        | "zig"
        | "zon"
        | "sol"
        | "svelte"
        | "vue"
        | "astro"
        | "groovy"
        | "clj"
        | "cljs"
        | "cljc"
        | "m"
        | "mm" => Some("source"),
        _ => None,
    }
}

fn is_named_architecture_file(file_name: &str) -> bool {
    matches!(
        file_name,
        "cargo.toml"
            | "package.json"
            | "tsconfig.json"
            | "jsconfig.json"
            | "deno.json"
            | "deno.jsonc"
            | "svelte.config.js"
            | "svelte.config.ts"
            | "svelte.config.mjs"
            | "svelte.config.cjs"
            | "vite.config.js"
            | "vite.config.ts"
            | "vite.config.mjs"
            | "vite.config.cjs"
            | "vite.config.mts"
            | "vite.config.cts"
            | "astro.config.js"
            | "astro.config.ts"
            | "astro.config.mjs"
            | "astro.config.cjs"
            | "next.config.js"
            | "next.config.ts"
            | "next.config.mjs"
            | "next.config.cjs"
            | "nuxt.config.js"
            | "nuxt.config.ts"
            | "nuxt.config.mjs"
            | "nuxt.config.cjs"
            | "rollup.config.js"
            | "rollup.config.ts"
            | "rollup.config.mjs"
            | "rollup.config.cjs"
            | "webpack.config.js"
            | "webpack.config.ts"
            | "webpack.config.mjs"
            | "webpack.config.cjs"
            | "jest.config.js"
            | "jest.config.ts"
            | "jest.config.mjs"
            | "jest.config.cjs"
            | "vitest.config.js"
            | "vitest.config.ts"
            | "vitest.config.mjs"
            | "vitest.config.cjs"
            | "tailwind.config.js"
            | "tailwind.config.ts"
            | "tailwind.config.mjs"
            | "tailwind.config.cjs"
            | "postcss.config.js"
            | "postcss.config.ts"
            | "postcss.config.mjs"
            | "postcss.config.cjs"
            | "babel.config.js"
            | "babel.config.ts"
            | "babel.config.mjs"
            | "babel.config.cjs"
            | "go.mod"
            | "go.work"
            | "pyproject.toml"
            | "requirements.txt"
            | "pipfile"
            | "mix.exs"
            | "rebar.config"
            | "gemfile"
            | "rakefile"
            | "composer.json"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "settings.gradle"
            | "settings.gradle.kts"
            | "cmakelists.txt"
            | "conanfile.txt"
            | "meson.build"
            | "package.swift"
            | "build.zig"
            | "build.zig.zon"
            | "podfile"
            | "project.clj"
    )
}

fn has_named_architecture_suffix(file_name: &str) -> bool {
    [
        ".csproj",
        ".fsproj",
        ".vbproj",
        ".vcxproj",
        ".sln",
        ".xcodeproj",
    ]
    .iter()
    .any(|suffix| file_name.ends_with(suffix))
}

fn extract_structural_signatures(
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<String>, usize), AstParserError> {
    let mut import_edges = 0usize;

    if language != "svelte" {
        if let Ok((signatures, edges)) = extract_with_language_pack(source, language, relative_path) {
            import_edges = edges;
            if !signatures.is_empty() {
                return Ok((signatures, edges));
            }
        }
    }

    if let Ok((signatures, edges)) = extract_with_official_tree_sitter(source, language, relative_path) {
        return Ok((signatures, import_edges.max(edges)));
    }

    let (signatures, fallback_edges) = extract_with_regex_fallback(source, language, relative_path)?;
    Ok((signatures, import_edges.max(fallback_edges)))
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

fn extract_with_official_tree_sitter(
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<String>, usize), AstParserError> {
    let ts_language = official_tree_sitter_language(language, relative_path).ok_or_else(|| AstParserError::ParseFailure {
        file: relative_path.to_string(),
        language: language.to_string(),
        reason: "grammar tree-sitter oficial indisponivel".to_string(),
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
    collect_official_tree_sitter_signatures(language, source.as_bytes(), tree.root_node(), &mut signatures);
    signatures.sort();
    signatures.dedup();
    if signatures.is_empty() {
        return Err(AstParserError::ParseFailure {
            file: relative_path.to_string(),
            language: language.to_string(),
            reason: "grammar oficial nao encontrou simbolos estruturais".to_string(),
        });
    }

    Ok((signatures, estimate_import_edges(language, source)))
}

fn official_tree_sitter_language(language: &str, _relative_path: &str) -> Option<Language> {
    match language {
        "c_sharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        _ => None,
    }
}

fn collect_official_tree_sitter_signatures(
    language: &str,
    source: &[u8],
    node: Node<'_>,
    out: &mut Vec<String>,
) {
    if let Some(signature) = official_signature_for_node(language, source, node) {
        out.push(signature);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_official_tree_sitter_signatures(language, source, child, out);
    }
}

fn official_signature_for_node(language: &str, source: &[u8], node: Node<'_>) -> Option<String> {
    match language {
        "c_sharp" => csharp_signature_for_node(source, node),
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

fn extract_with_regex_fallback(
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<String>, usize), AstParserError> {
    let mut signatures = Vec::new();
    for (label, pattern) in regex_fallback_patterns(language) {
        let Ok(regex) = Regex::new(pattern) else {
            continue;
        };
        for captures in regex.captures_iter(source) {
            let Some(matched) = captures.get(1) else {
                continue;
            };
            let name = truncate_chars(matched.as_str().trim(), 96);
            if name.is_empty() {
                continue;
            }
            signatures.push(format!("{language} {label} {name}"));
        }
    }
    signatures.sort();
    signatures.dedup();

    if signatures.is_empty() && looks_like_legible_source(source) {
        signatures.push(format!(
            "{language} file {}",
            Path::new(relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(relative_path)
        ));
    }

    if signatures.is_empty() {
        return Err(AstParserError::ParseFailure {
            file: relative_path.to_string(),
            language: language.to_string(),
            reason: "fallback regex nao encontrou simbolos estruturais".to_string(),
        });
    }

    Ok((signatures, estimate_import_edges(language, source)))
}

fn regex_fallback_patterns(language: &str) -> &'static [(&'static str, &'static str)] {
    match language {
        "rust" => &[
            ("fn", r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("struct", r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("enum", r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("trait", r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?trait\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("impl", r"(?m)^\s*impl(?:<[^>\n]+>)?\s+([A-Za-z_][A-Za-z0-9_:<>]*)"),
            ("mod", r"(?m)^\s*(?:pub\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)"),
        ],
        "python" => &[
            ("class", r"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("def", r"(?m)^\s*(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)"),
        ],
        "javascript" | "svelte" => &[
            ("class", r"(?m)^\s*(?:export\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)"),
            ("function", r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)"),
            ("const", r"(?m)^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s*)?(?:\([^=\n]*\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*=>"),
        ],
        "typescript" => &[
            ("class", r"(?m)^\s*(?:export\s+)?(?:abstract\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)"),
            ("interface", r"(?m)^\s*(?:export\s+)?interface\s+([A-Za-z_$][A-Za-z0-9_$]*)"),
            ("type", r"(?m)^\s*(?:export\s+)?type\s+([A-Za-z_$][A-Za-z0-9_$]*)"),
            ("enum", r"(?m)^\s*(?:export\s+)?enum\s+([A-Za-z_$][A-Za-z0-9_$]*)"),
            ("function", r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)"),
            ("const", r"(?m)^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s*)?(?:\([^=\n]*\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*=>"),
        ],
        "c" | "cpp" => &[
            ("namespace", r"(?m)^\s*namespace\s+([A-Za-z_][A-Za-z0-9_:]*)"),
            ("class", r"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("struct", r"(?m)^\s*struct\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("enum", r"(?m)^\s*enum(?:\s+class)?\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("fn", r"(?m)^\s*(?:template\s*<[^>\n]+>\s*)?(?:inline\s+)?[A-Za-z_][A-Za-z0-9_:<>\s\*&~]*\s+([A-Za-z_~][A-Za-z0-9_:~]*)\s*\([^;{}]*\)\s*(?:const\s*)?(?:\{|$)"),
        ],
        "elixir" => &[
            ("module", r"(?m)^\s*defmodule\s+([A-Za-z_][A-Za-z0-9_\.!]*)"),
            ("protocol", r"(?m)^\s*defprotocol\s+([A-Za-z_][A-Za-z0-9_\.!]*)"),
            ("impl", r"(?m)^\s*defimpl\s+([A-Za-z_][A-Za-z0-9_\.!]*)"),
            ("macro", r"(?m)^\s*defmacrop?\s+([A-Za-z_][A-Za-z0-9_!?]*)"),
            ("guard", r"(?m)^\s*defguardp?\s+([A-Za-z_][A-Za-z0-9_!?]*)"),
            ("def", r"(?m)^\s*defp?\s+([A-Za-z_][A-Za-z0-9_!?]*)"),
        ],
        "c_sharp" => &[
            ("namespace", r"(?m)^\s*namespace\s+([A-Za-z_][A-Za-z0-9_\.]*)"),
            ("class", r"(?m)^\s*(?:public|private|protected|internal|sealed|abstract|static|\s)+class\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("interface", r"(?m)^\s*(?:public|private|protected|internal|\s)+interface\s+([A-Za-z_][A-Za-z0-9_]*)"),
            ("method", r"(?m)^\s*(?:public|private|protected|internal|static|virtual|override|async|\s)+[A-Za-z_<>\[\],\s]+\s+([A-Za-z_][A-Za-z0-9_]*)\s*\("),
        ],
        _ => &[],
    }
}

fn estimate_import_edges(language: &str, source: &str) -> usize {
    let pattern = match language {
        "rust" => r"(?m)^\s*use\s+[A-Za-z_]",
        "python" => r"(?m)^\s*(?:from\s+\S+\s+import|import\s+\S+)",
        "javascript" | "typescript" | "svelte" => {
            r#"(?m)^\s*(?:import\s+.+from\s+['"]|import\s+['"]|(?:const|let|var)\s+\w+\s*=\s*require\s*\()"#
        }
        "c" | "cpp" => r#"(?m)^\s*#\s*include\s+[<"]"#,
        "elixir" => r"(?m)^\s*(?:alias|import|require|use)\s+[A-Za-z_]",
        "c_sharp" => r"(?m)^\s*using\s+[A-Za-z_]",
        _ => return 0,
    };
    Regex::new(pattern)
        .ok()
        .map(|regex| regex.find_iter(source).count())
        .unwrap_or(0)
}

fn looks_like_legible_source(source: &str) -> bool {
    if source.trim().is_empty() {
        return false;
    }
    let non_whitespace = source.chars().filter(|ch| !ch.is_whitespace()).take(64).count();
    non_whitespace >= 12
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
) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("repo");
    let mut out = String::new();
    out.push_str("# Repository Outline\n\n");
    out.push_str(&format!("repo: {repo_name}\n"));
    out.push_str(&format!("symbol_files: {}\n", parsed_files.len()));
    out.push_str("source: native-rust multi-strategy (language-pack + targeted-tree-sitter + regex-fallback)\n\n");
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
    }
    out
}

fn build_architecture_map(files: &[String]) -> String {
    let directories = architecture_directories(files);
    let mut out = String::from("# Architecture Map\n\n");
    for (directory, files) in directories {
        out.push_str(&format!("[{}]\n", directory));
        for file in files {
            out.push_str("- ");
            out.push_str(&file);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn architecture_directories(files: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut directories = BTreeMap::<String, Vec<String>>::new();
    for relative_path in files {
        directories
            .entry(directory_key(relative_path))
            .or_default()
            .push(relative_path.clone());
    }
    directories
}

fn build_health_report(
    repo_root: &Path,
    languages: &BTreeMap<String, usize>,
    parsed_files: usize,
    total_signatures: usize,
    total_import_edges: usize,
) -> Result<String, AstParserError> {
    let repo_name = repo_root
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("repo");
    let mut text = String::from("# Health Report\n");
    text.push_str("\nsummary: findings=0");
    text.push_str("\nsource: native-rust multi-strategy (language-pack + targeted-tree-sitter + regex-fallback)");
    text.push_str("\nrepo: ");
    text.push_str(repo_name);
    text.push_str("\nparsed_files: ");
    text.push_str(&parsed_files.to_string());
    text.push_str("\ntotal_signatures: ");
    text.push_str(&total_signatures.to_string());
    text.push_str("\ntotal_import_edges: ");
    text.push_str(&total_import_edges.to_string());
    text.push_str("\nlanguages:");
    for (language, count) in languages {
        text.push_str("\n- ");
        text.push_str(language);
        text.push_str(": ");
        text.push_str(&count.to_string());
    }
    Ok(text)
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
    use tempfile::TempDir;

    #[test]
    fn detects_expected_languages() {
        assert_eq!(detect_language(Path::new("src/lib.rs")).as_deref(), Some("rust"));
        assert_eq!(detect_language(Path::new("src/app.ts")).as_deref(), Some("typescript"));
        assert_eq!(detect_language(Path::new("src/app.mts")).as_deref(), Some("typescript"));
        assert_eq!(detect_language(Path::new("main.py")).as_deref(), Some("python"));
        assert_eq!(detect_language(Path::new("web/App.svelte")).as_deref(), Some("svelte"));
        assert_eq!(detect_language(Path::new("Program.cs")).as_deref(), Some("c_sharp"));
        assert_eq!(detect_language(Path::new("include/runtime.h")).as_deref(), Some("c"));
        assert_eq!(detect_language(Path::new("notes.md")), None);
        assert_eq!(detect_language(Path::new("ci.yml")), None);
        assert_eq!(detect_language(Path::new("build.sh")), None);
        assert_eq!(detect_language(Path::new("Dockerfile")), None);
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
            extract_with_official_tree_sitter(source, "c_sharp", "Program.cs").unwrap();
        assert_eq!(imports, 0);
        assert!(signatures.iter().any(|item| item.contains("c# class Greeter")));
        assert!(signatures.iter().any(|item| item.contains("c# method Run")));
    }

    #[test]
    fn allowlist_rejects_non_source_extensions() {
        assert!(is_ast_source_file_allowed(Path::new("src/lib.rs")));
        assert!(is_ast_source_file_allowed(Path::new("include/runtime.hpp")));
        assert!(!is_ast_source_file_allowed(Path::new("pnpm-lock.yaml")));
        assert!(!is_ast_source_file_allowed(Path::new(".github/workflows/ci.yml")));
        assert!(!is_ast_source_file_allowed(Path::new("package-lock.json")));
        assert!(!is_ast_source_file_allowed(Path::new("scripts/bootstrap.sh")));
        assert!(!is_ast_source_file_allowed(Path::new("README.md")));
        assert!(!is_ast_source_file_allowed(Path::new("LICENSE")));
    }

    #[test]
    fn collect_source_files_ignores_workflows_lockfiles_and_shell_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();
        std::fs::create_dir_all(repo_root.join(".github/workflows")).unwrap();
        std::fs::create_dir_all(repo_root.join("src")).unwrap();
        std::fs::create_dir_all(repo_root.join("scripts")).unwrap();
        std::fs::write(repo_root.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();
        std::fs::write(repo_root.join(".github/workflows/ci.yml"), "name: ci").unwrap();
        std::fs::write(repo_root.join("scripts/bootstrap.sh"), "echo hi").unwrap();
        std::fs::write(repo_root.join("README.md"), "# demo").unwrap();
        std::fs::write(repo_root.join("src/lib.rs"), "pub fn ok() {}").unwrap();
        std::fs::write(repo_root.join("src/app.ts"), "export const ok = true;").unwrap();

        let files = collect_source_files(repo_root).unwrap();
        let relative = files
            .iter()
            .map(|path| sanitize_relative_path(repo_root, path))
            .collect::<Vec<_>>();

        assert_eq!(relative, vec!["src/app.ts".to_string(), "src/lib.rs".to_string()]);
    }


    #[test]
    fn architecture_allowlist_accepts_polyglot_source_and_project_files() {
        assert!(is_architecture_file_allowed(Path::new("src/lib.rs")));
        assert!(is_architecture_file_allowed(Path::new("web/App.svelte")));
        assert!(is_architecture_file_allowed(Path::new("frontend/routes/page.vue")));
        assert!(is_architecture_file_allowed(Path::new("Cargo.toml")));
        assert!(is_architecture_file_allowed(Path::new("package.json")));
        assert!(is_architecture_file_allowed(Path::new("svelte.config.js")));
        assert!(is_architecture_file_allowed(Path::new("go.mod")));
        assert!(is_architecture_file_allowed(Path::new("mix.exs")));
        assert!(is_architecture_file_allowed(Path::new("Gemfile")));
        assert!(is_architecture_file_allowed(Path::new("composer.json")));
        assert!(is_architecture_file_allowed(Path::new("pom.xml")));
        assert!(is_architecture_file_allowed(Path::new("settings.gradle.kts")));
        assert!(is_architecture_file_allowed(Path::new("App/App.csproj")));
        assert!(is_architecture_file_allowed(Path::new("Workspace.sln")));
        assert!(is_architecture_file_allowed(Path::new("cpp/CMakeLists.txt")));
        assert!(is_architecture_file_allowed(Path::new("zig/build.zig.zon")));
        assert!(is_architecture_file_allowed(Path::new("Package.swift")));
        assert!(!is_architecture_file_allowed(Path::new("package-lock.json")));
        assert!(!is_architecture_file_allowed(Path::new("README.md")));
    }

    #[test]
    fn collect_architecture_files_prunes_build_cache_and_dependency_noise() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();
        std::fs::create_dir_all(repo_root.join("src")).unwrap();
        std::fs::create_dir_all(repo_root.join("web")).unwrap();
        std::fs::create_dir_all(repo_root.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(repo_root.join("target/debug")).unwrap();
        std::fs::create_dir_all(repo_root.join("vendor/lib")).unwrap();
        std::fs::create_dir_all(repo_root.join("venv/bin")).unwrap();
        std::fs::create_dir_all(repo_root.join(".vscode")).unwrap();
        std::fs::create_dir_all(repo_root.join("dist/assets")).unwrap();
        std::fs::create_dir_all(repo_root.join(".git")).unwrap();
        std::fs::create_dir_all(repo_root.join("tests")).unwrap();

        std::fs::write(repo_root.join("src/lib.rs"), "pub fn ok() {}").unwrap();
        std::fs::write(repo_root.join("web/App.svelte"), "<script>let ok = true;</script>").unwrap();
        std::fs::write(repo_root.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
        std::fs::write(repo_root.join("package.json"), "{\"name\":\"demo\"}").unwrap();
        std::fs::write(repo_root.join("svelte.config.js"), "export default {};").unwrap();
        std::fs::write(repo_root.join("node_modules/pkg/index.js"), "export {};").unwrap();
        std::fs::write(repo_root.join("target/debug/app"), "binary").unwrap();
        std::fs::write(repo_root.join("vendor/lib/code.php"), "<?php echo 1;").unwrap();
        std::fs::write(repo_root.join("venv/bin/python"), "python").unwrap();
        std::fs::write(repo_root.join(".vscode/settings.json"), "{}").unwrap();
        std::fs::write(repo_root.join("dist/assets/app.js"), "console.log(1);").unwrap();
        std::fs::write(repo_root.join(".git/config"), "[core]").unwrap();
        std::fs::write(repo_root.join("tests/app_test.go"), "package demo").unwrap();

        let files = collect_architecture_files(repo_root).unwrap();

        assert!(files.contains(&"src/lib.rs".to_string()));
        assert!(files.contains(&"web/App.svelte".to_string()));
        assert!(files.contains(&"Cargo.toml".to_string()));
        assert!(files.contains(&"package.json".to_string()));
        assert!(files.contains(&"svelte.config.js".to_string()));
        assert!(!files.iter().any(|path| path.contains("node_modules")));
        assert!(!files.iter().any(|path| path.contains("target/")));
        assert!(!files.iter().any(|path| path.contains("vendor/")));
        assert!(!files.iter().any(|path| path.contains("venv/")));
        assert!(!files.iter().any(|path| path.contains(".vscode/")));
        assert!(!files.iter().any(|path| path.contains("dist/")));
        assert!(!files.iter().any(|path| path.contains(".git/")));
        assert!(!files.iter().any(|path| path.contains("tests/")));
    }

    #[test]
    fn architecture_map_keeps_full_polyglot_topology_without_char_truncation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();
        std::fs::create_dir_all(repo_root.join("src")).unwrap();
        std::fs::create_dir_all(repo_root.join("web")).unwrap();
        std::fs::create_dir_all(repo_root.join("go")).unwrap();
        std::fs::create_dir_all(repo_root.join("dotnet/App")).unwrap();
        std::fs::create_dir_all(repo_root.join("zig")).unwrap();
        std::fs::create_dir_all(repo_root.join("cpp")).unwrap();

        std::fs::write(repo_root.join("src/lib.rs"), "pub fn ok() {}").unwrap();
        std::fs::write(repo_root.join("web/App.svelte"), "<script>let ok = true;</script>").unwrap();
        std::fs::write(repo_root.join("web/routes.ts"), "export const route = '/';").unwrap();
        std::fs::write(repo_root.join("go/main.go"), "package main\nfunc main() {}\n").unwrap();
        std::fs::write(repo_root.join("dotnet/App/App.csproj"), "<Project />").unwrap();
        std::fs::write(repo_root.join("Workspace.sln"), "Microsoft Visual Studio Solution File").unwrap();
        std::fs::write(repo_root.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
        std::fs::write(repo_root.join("package.json"), "{\"name\":\"demo\"}").unwrap();
        std::fs::write(repo_root.join("svelte.config.js"), "export default {};").unwrap();
        std::fs::write(repo_root.join("go.mod"), "module demo\n").unwrap();
        std::fs::write(repo_root.join("mix.exs"), "defmodule Demo.MixProject do\nend\n").unwrap();
        std::fs::write(repo_root.join("Gemfile"), "gem 'rails'\n").unwrap();
        std::fs::write(repo_root.join("composer.json"), "{\"require\":{}}\n").unwrap();
        std::fs::write(repo_root.join("cpp/CMakeLists.txt"), "project(demo)\n").unwrap();
        std::fs::write(repo_root.join("zig/build.zig.zon"), ".{ .name = \"demo\" }\n").unwrap();
        std::fs::write(repo_root.join("Package.swift"), "import PackageDescription\n").unwrap();

        let artifacts = extract_repository_outline_native(repo_root).unwrap();
        let architecture_map = String::from_utf8(artifacts.architecture_map_blob).unwrap();

        assert!(architecture_map.len() > 20, "Blob 05 nao deve mais truncar por teto fixo");
        assert!(architecture_map.contains("[.]"));
        assert!(architecture_map.contains("Cargo.toml"));
        assert!(architecture_map.contains("package.json"));
        assert!(architecture_map.contains("svelte.config.js"));
        assert!(architecture_map.contains("go.mod"));
        assert!(architecture_map.contains("mix.exs"));
        assert!(architecture_map.contains("Gemfile"));
        assert!(architecture_map.contains("composer.json"));
        assert!(architecture_map.contains("Workspace.sln"));
        assert!(architecture_map.contains("[web]"));
        assert!(architecture_map.contains("web/App.svelte"));
        assert!(architecture_map.contains("[go]"));
        assert!(architecture_map.contains("go/main.go"));
        assert!(architecture_map.contains("[dotnet/App]"));
        assert!(architecture_map.contains("dotnet/App/App.csproj"));
        assert!(architecture_map.contains("[cpp]"));
        assert!(architecture_map.contains("cpp/CMakeLists.txt"));
        assert!(architecture_map.contains("[zig]"));
        assert!(architecture_map.contains("zig/build.zig.zon"));
    }

    #[test]
    fn regex_fallback_covers_target_polyglot_ecosystems() {
        let scenarios = [
            ("rust", "pub struct Engine;\npub async fn run() {}\n", "src/lib.rs", "rust fn run"),
            ("python", "class Engine:\n    pass\n\ndef run():\n    pass\n", "app.py", "python def run"),
            (
                "typescript",
                "export interface Engine {}\nexport const boot = async () => {}\n",
                "src/app.ts",
                "typescript const boot",
            ),
            ("cpp", "namespace demo {}\nclass Engine {};\nint run() { return 0; }\n", "src/main.cpp", "cpp fn run"),
            ("elixir", "defmodule Demo.Engine do\n  def run, do: :ok\nend\n", "lib/demo/engine.ex", "elixir module Demo.Engine"),
        ];

        for (language, source, path, expected_fragment) in scenarios {
            let (signatures, _) = extract_with_regex_fallback(source, language, path).unwrap();
            assert!(
                signatures.iter().any(|item| item.contains(expected_fragment)),
                "assinaturas de {language} deveriam conter `{expected_fragment}`; obtido: {:?}",
                signatures
            );
        }
    }

    #[test]
    fn extract_repository_outline_native_keeps_blob04_alive_for_target_ecosystems() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();
        std::fs::create_dir_all(repo_root.join("src")).unwrap();
        std::fs::create_dir_all(repo_root.join("python")).unwrap();
        std::fs::create_dir_all(repo_root.join("web")).unwrap();
        std::fs::create_dir_all(repo_root.join("cpp")).unwrap();
        std::fs::create_dir_all(repo_root.join("lib/demo")).unwrap();

        std::fs::write(repo_root.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
        std::fs::write(repo_root.join("pyproject.toml"), "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
        std::fs::write(repo_root.join("package.json"), "{\"name\":\"demo\"}").unwrap();
        std::fs::write(repo_root.join("mix.exs"), "defmodule Demo.MixProject do\nend\n").unwrap();
        std::fs::write(repo_root.join("cpp/CMakeLists.txt"), "project(demo)\n").unwrap();

        std::fs::write(repo_root.join("src/lib.rs"), "pub struct Engine;\npub fn run() {}\n").unwrap();
        std::fs::write(repo_root.join("python/app.py"), "class Engine:\n    pass\n\ndef run():\n    pass\n").unwrap();
        std::fs::write(
            repo_root.join("web/app.tsx"),
            "export interface EngineProps {}\nexport function App() { return null; }\n",
        )
        .unwrap();
        std::fs::write(
            repo_root.join("cpp/main.cpp"),
            "namespace demo {}\nclass Engine {};\nint run() { return 0; }\n",
        )
        .unwrap();
        std::fs::write(
            repo_root.join("lib/demo/engine.ex"),
            "defmodule Demo.Engine do\n  def run, do: :ok\nend\n",
        )
        .unwrap();

        let artifacts = extract_repository_outline_native(repo_root).unwrap();
        let repo_outline = String::from_utf8(artifacts.repo_outline_blob).unwrap();

        assert!(repo_outline.contains("[src/lib.rs]"));
        assert!(repo_outline.contains("[python/app.py]"));
        assert!(repo_outline.contains("[web/app.tsx]"));
        assert!(repo_outline.contains("[cpp/main.cpp]"));
        assert!(repo_outline.contains("[lib/demo/engine.ex]"));
        assert!(repo_outline.contains("rust"));
        assert!(repo_outline.contains("python"));
        assert!(repo_outline.contains("typescript"));
        assert!(repo_outline.contains("cpp"));
        assert!(repo_outline.contains("elixir"));
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
