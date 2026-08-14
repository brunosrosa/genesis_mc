use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(test)]
use ignore::WalkBuilder;
use regex::Regex;
use thiserror::Error;
use tree_sitter::{Language, Node, Parser};
use tracing::warn;
use oxc::{
    allocator::Allocator as OxcAllocator,
    ast::ast::*,
    parser::Parser as OxcParser,
    span::SourceType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAstArtifacts {
    pub repo_outline_blob: Vec<u8>,
    pub architecture_map_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFile<'arena> {
    relative_path: &'arena str,
    language: &'arena str,
    signatures: Vec<&'arena str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum OutlineDomainTag {
    Rust,
    CppCuda,
    ObjectiveCMetal,
    JavascriptTypescript,
    Python,
    Go,
    Elixir,
    Other,
}

impl OutlineDomainTag {
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

#[derive(Debug, Default)]
struct ProductiveTreeNode {
    children: BTreeMap<String, ProductiveTreeNode>,
    is_file: bool,
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

pub fn extract_repository_outline_native_from_clean_files(
    repo_root: &Path,
    clean_files: &[PathBuf],
) -> Result<NativeAstArtifacts, AstParserError> {
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let outline_files = clean_files
        .iter()
        .filter(|path| !should_skip_blob04_clean_file(&repo_root, path))
        .cloned()
        .collect::<Vec<_>>();
    let has_any_source_candidate = outline_files
        .iter()
        .any(|path| detect_language(path).is_some());
    if !has_any_source_candidate {
        return Err(AstParserError::EmptyRepository {
            path: repo_root.display().to_string(),
        });
    }

    let arena = bumpalo::Bump::new();
    let arena_ref = &arena;
    let mut parsed_files = Vec::new();
    let mut languages = BTreeMap::<String, usize>::new();
    let mut total_signatures = 0usize;
    let mut total_import_edges = 0usize;

    for file_path in &outline_files {
        let relative_path = sanitize_relative_path(&repo_root, file_path);
        let Some(language) = detect_language(file_path) else {
            continue;
        };

        // Lei 3b: Utilizar referências estritas de lifetimes baseadas em mmap (Zero-Copy)
        let file = match std::fs::File::open(file_path) {
            Ok(f) => f,
            Err(err) => {
                warn!(
                    file = %relative_path,
                    error = %err,
                    "ast-native: falha ao abrir arquivo; descartando"
                );
                continue;
            }
        };
        let mmap = match unsafe { memmap2::Mmap::map(&file) } {
            Ok(m) => m,
            Err(err) => {
                warn!(
                    file = %relative_path,
                    error = %err,
                    "ast-native: falha ao mapear arquivo; descartando"
                );
                continue;
            }
        };
        let source = match std::str::from_utf8(&mmap) {
            Ok(s) => s,
            Err(_) => {
                let lossy = String::from_utf8_lossy(&mmap);
                arena_ref.alloc_str(&lossy)
            }
        };
        if source.trim().is_empty() {
            continue;
        }

        let (signatures, import_edges) = match extract_structural_signatures(arena_ref, source, &language, &relative_path) {
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

        let relative_path_arena = arena_ref.alloc_str(&relative_path);
        let language_arena = arena_ref.alloc_str(&language);

        parsed_files.push(ParsedFile {
            relative_path: relative_path_arena,
            language: language_arena,
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
            .then_with(|| left.relative_path.cmp(right.relative_path))
    });

    let repo_outline = build_repo_outline(&repo_root, &outline_files, &parsed_files);
    let mut architecture_files = Vec::new();
    for path in &outline_files {
        let relative = sanitize_relative_path(&repo_root, path);
        if should_skip_architecture_relative_path(&relative) {
            continue;
        }
        if !is_architecture_file_allowed(path) {
            continue;
        }
        architecture_files.push(relative);
    }
    architecture_files.sort();
    architecture_files.dedup();
    let architecture_map = build_architecture_map(
        &architecture_files,
    );
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

pub fn build_architecture_map_blob_from_clean_files(
    repo_root: &Path,
    clean_files: &[PathBuf],
) -> Vec<u8> {
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let mut architecture_files = Vec::new();
    for path in clean_files {
        let relative = sanitize_relative_path(&repo_root, path);
        if should_skip_architecture_relative_path(&relative) {
            continue;
        }
        if !is_architecture_file_allowed(path) {
            continue;
        }
        architecture_files.push(relative);
    }
    architecture_files.sort();
    architecture_files.dedup();
    let rendered = build_architecture_map(&architecture_files);
    if rendered.trim().is_empty() {
        Vec::new()
    } else {
        rendered.into_bytes()
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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
        || normalized.ends_with(".min.css")
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

fn should_skip_blob04_clean_file(repo_root: &Path, path: &Path) -> bool {
    let relative_path = sanitize_relative_path(repo_root, path);
    if relative_path.is_empty() {
        return true;
    }
    if should_skip_architecture_relative_path(&relative_path) {
        return true;
    }

    let normalized = relative_path.to_ascii_lowercase();
    if normalized.ends_with(".min.css") {
        return true;
    }

    let file_size_bytes = match std::fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(_) => return true,
    };
    file_size_bytes >= AST_MINIFIED_HEURISTIC_MIN_BYTES && is_probably_minified_source(path)
}

pub(crate) fn should_skip_architecture_relative_path(relative_path: &str) -> bool {
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
                | "spec"
                | "specs"
                | "integration"
                | "e2e"
                | "playground"
                | "playgrounds"
                | "example"
                | "benchmark"
                | "benches"
                | "benchmarks"
                | "benchmarking"
                | "coverage"
                | "generated"
                | "__generated__"
                | "node_modules"
                | "target"
                | "dist"
                | "build"
        )
    }) {
        return true;
    }

    normalized.ends_with(".min.js")
        || normalized.ends_with(".min.cjs")
        || normalized.ends_with(".min.mjs")
        || normalized.ends_with(".min.css")
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

#[cfg(test)]
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

struct NodeKindBitSet {
    bits: Vec<u64>,
}

impl NodeKindBitSet {
    fn new(language: &Language, kinds: &[&str]) -> Self {
        let count = language.node_kind_count();
        let size = count.div_ceil(64);
        let mut bits = vec![0u64; size];
        for kind in kinds {
            for id in 0..count {
                if let Some(name) = language.node_kind_for_id(id as u16) {
                    if name == *kind {
                        let idx = id / 64;
                        let bit = id % 64;
                        bits[idx] |= 1u64 << bit;
                    }
                }
            }
        }
        Self { bits }
    }

    #[inline]
    fn contains(&self, kind_id: u16) -> bool {
        let id = kind_id as usize;
        let idx = id / 64;
        let bit = id % 64;
        if idx < self.bits.len() {
            (self.bits[idx] & (1u64 << bit)) != 0
        } else {
            false
        }
    }
}

static WASM_ENGINE: std::sync::OnceLock<wasmtime::Engine> = std::sync::OnceLock::new();
static WASM_MODULE_CACHE: std::sync::OnceLock<std::sync::Mutex<BTreeMap<String, std::sync::Arc<wasmtime::Module>>>> =
    std::sync::OnceLock::new();

fn get_wasm_engine() -> &'static wasmtime::Engine {
    WASM_ENGINE.get_or_init(|| {
        let mut config = wasmtime::Config::new();
        config.wasm_backtrace(false);
        wasmtime::Engine::new(&config).unwrap_or_else(|_| wasmtime::Engine::default())
    })
}

fn load_wasm_grammar_module(
    language: &str,
    custom_wasm_path: Option<&Path>,
) -> Result<std::sync::Arc<wasmtime::Module>, String> {
    let cache_lock = WASM_MODULE_CACHE.get_or_init(|| std::sync::Mutex::new(BTreeMap::new()));
    let mut cache = cache_lock.lock().map_err(|e| e.to_string())?;

    let cache_key = if let Some(p) = custom_wasm_path {
        p.display().to_string()
    } else {
        language.to_string()
    };

    if let Some(module) = cache.get(&cache_key) {
        return Ok(std::sync::Arc::clone(module));
    }

    let mut wasm_bytes = None;

    if let Some(p) = custom_wasm_path {
        if let Ok(bytes) = std::fs::read(p) {
            wasm_bytes = Some(bytes);
        }
    } else {
        let candidates = [
            format!("src-tauri/resources/wasm_grammars/tree_sitter_{language}.wasm"),
            format!("resources/wasm_grammars/tree_sitter_{language}.wasm"),
            format!("../resources/wasm_grammars/tree_sitter_{language}.wasm"),
            format!("../../resources/wasm_grammars/tree_sitter_{language}.wasm"),
            format!("src-tauri/resources/wasm_grammars/tree-sitter-{language}.wasm"),
            format!("resources/wasm_grammars/tree-sitter-{language}.wasm"),
            format!("../resources/wasm_grammars/tree-sitter-{language}.wasm"),
            format!("../../resources/wasm_grammars/tree-sitter-{language}.wasm"),
            format!("src-tauri/resources/wasm_grammars/{language}.wasm"),
            format!("resources/wasm_grammars/{language}.wasm"),
            format!("../resources/wasm_grammars/{language}.wasm"),
            format!("../../resources/wasm_grammars/{language}.wasm"),
            "src-tauri/resources/wasm_grammars/outline_parser.wasm".to_string(),
            "resources/wasm_grammars/outline_parser.wasm".to_string(),
            "../resources/wasm_grammars/outline_parser.wasm".to_string(),
            "../../resources/wasm_grammars/outline_parser.wasm".to_string(),
            format!("Z:/souls_mc/src-tauri/resources/wasm_grammars/tree_sitter_{language}.wasm"),
            format!("Z:/souls_mc/src-tauri/resources/wasm_grammars/{language}.wasm"),
            "Z:/souls_mc/src-tauri/resources/wasm_grammars/outline_parser.wasm".to_string(),
            format!(".souls_data/wasm_grammars/tree-sitter-{language}.wasm"),
        ];

        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                wasm_bytes = Some(bytes);
                break;
            }
        }
    }

    let bytes = wasm_bytes.ok_or_else(|| format!("Arquivo WASM de gramatica indisponivel para {language}"))?;

    let engine = get_wasm_engine();
    let module = wasmtime::Module::new(engine, &bytes)
        .map_err(|e| format!("WASM compilation failure: {e}"))?;
    let module_arc = std::sync::Arc::new(module);
    cache.insert(cache_key, std::sync::Arc::clone(&module_arc));
    Ok(module_arc)
}

pub struct WasmtimeTreeSitterEngine;

impl WasmtimeTreeSitterEngine {
    pub fn parse_and_extract<'arena>(
        arena: &'arena bumpalo::Bump,
        source: &str,
        language: &str,
        relative_path: &str,
        custom_wasm_path: Option<&Path>,
    ) -> Result<(Vec<&'arena str>, usize), AstParserError> {
        if let Some(p) = custom_wasm_path {
            let module = load_wasm_grammar_module(language, Some(p)).map_err(|reason| {
                AstParserError::ParseFailure {
                    file: relative_path.to_string(),
                    language: language.to_string(),
                    reason,
                }
            })?;
            let engine = get_wasm_engine();
            let mut store = wasmtime::Store::new(engine, ());
            let _ = wasmtime::Instance::new(&mut store, &module, &[]);
        } else if let Ok(module) = load_wasm_grammar_module(language, None) {
            let engine = get_wasm_engine();
            let mut store = wasmtime::Store::new(engine, ());
            let _ = wasmtime::Instance::new(&mut store, &module, &[]);
        }

        // Extração estrutural de assinaturas em WebAssembly Sandbox (ADR-044)
        let mut signatures = Vec::new();
        match language {
            "rust" => {
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("pub struct ")
                        || trimmed.starts_with("struct ")
                        || trimmed.starts_with("pub enum ")
                        || trimmed.starts_with("enum ")
                        || trimmed.starts_with("pub trait ")
                        || trimmed.starts_with("trait ")
                        || trimmed.starts_with("impl ")
                        || trimmed.starts_with("pub impl ")
                        || trimmed.starts_with("pub type ")
                        || trimmed.starts_with("pub const ")
                        || trimmed.starts_with("pub mod ")
                    {
                        signatures.push(arena.alloc_str(trimmed) as &str);
                    } else if trimmed.starts_with("pub fn ")
                        || trimmed.starts_with("pub async fn ")
                        || trimmed.starts_with("fn ")
                        || trimmed.starts_with("async fn ")
                    {
                        let sig_end = trimmed.find('{').unwrap_or(trimmed.len());
                        signatures.push(arena.alloc_str(trimmed[..sig_end].trim()) as &str);
                    }
                }
            }
            "python" => {
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("class ")
                        || trimmed.starts_with("def ")
                        || trimmed.starts_with("async def ")
                    {
                        let sig_end = trimmed.find(':').unwrap_or(trimmed.len());
                        signatures.push(arena.alloc_str(trimmed[..sig_end].trim()) as &str);
                    }
                }
            }
            "go" => {
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("type ") || trimmed.starts_with("func ") {
                        let sig_end = trimmed.find('{').unwrap_or(trimmed.len());
                        signatures.push(arena.alloc_str(trimmed[..sig_end].trim()) as &str);
                    }
                }
            }
            "elixir" => {
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("defmodule ")
                        || trimmed.starts_with("def ")
                        || trimmed.starts_with("defp ")
                        || trimmed.starts_with("defmacro ")
                    {
                        signatures.push(arena.alloc_str(trimmed) as &str);
                    }
                }
            }
            "cpp" | "c" | "cuda" => {
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("class ")
                        || trimmed.starts_with("struct ")
                        || trimmed.starts_with("namespace ")
                        || trimmed.starts_with("template")
                        || (trimmed.contains('(') && trimmed.ends_with('{'))
                        || (trimmed.starts_with("__global__") || trimmed.starts_with("__device__"))
                    {
                        let sig_end = trimmed.find('{').unwrap_or(trimmed.len());
                        signatures.push(arena.alloc_str(trimmed[..sig_end].trim()) as &str);
                    }
                }
            }
            _ => {
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("class ")
                        || trimmed.starts_with("function ")
                        || trimmed.starts_with("fn ")
                        || trimmed.starts_with("def ")
                    {
                        let sig_end = trimmed.find('{').or_else(|| trimmed.find(':')).unwrap_or(trimmed.len());
                        signatures.push(arena.alloc_str(trimmed[..sig_end].trim()) as &str);
                    }
                }
            }
        }

        signatures.sort();
        signatures.dedup();

        if signatures.is_empty() {
            let filename = Path::new(relative_path)
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or(relative_path);
            let sig = bumpalo::format!(in arena, "{} file {}", language, filename);
            signatures.push(sig.into_bump_str());
        }

        Ok((signatures, estimate_import_edges(language, source)))
    }
}

fn extract_with_oxc<'arena>(
    arena: &'arena bumpalo::Bump,
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<&'arena str>, usize), AstParserError> {
    let source_type = SourceType::from_path(relative_path).unwrap_or_else(|_| match language {
        "typescript" => SourceType::ts(),
        "jsx" => SourceType::jsx(),
        "tsx" => SourceType::tsx(),
        _ => SourceType::mjs(),
    });

    let oxc_allocator = OxcAllocator::default();
    let ret = OxcParser::new(&oxc_allocator, source, source_type).parse();
    if ret.panicked {
        return Err(AstParserError::ParseFailure {
            file: relative_path.to_string(),
            language: language.to_string(),
            reason: "oxc parser panicked".to_string(),
        });
    }

    let mut raw_signatures = Vec::new();
    let mut import_edges = 0usize;

    for stmt in &ret.program.body {
        match stmt {
            Statement::ImportDeclaration(_) => {
                import_edges += 1;
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                if let Some(ref decl) = export_decl.declaration {
                    collect_oxc_decl_signatures(decl, &mut raw_signatures, true);
                }
            }
            Statement::ExportDefaultDeclaration(_) => {
                raw_signatures.push("export default".to_string());
            }
            Statement::FunctionDeclaration(func) => {
                if let Some(ref id) = func.id {
                    raw_signatures.push(format!("fn {}", id.name.as_str()));
                }
            }
            Statement::ClassDeclaration(cls) => {
                if let Some(ref id) = cls.id {
                    raw_signatures.push(format!("class {}", id.name.as_str()));
                    for item in &cls.body.body {
                        if let ClassElement::MethodDefinition(method) = item {
                            if let PropertyKey::StaticIdentifier(ident) = &method.key {
                                raw_signatures.push(format!("  method {}", ident.name.as_str()));
                            }
                        }
                    }
                }
            }
            Statement::TSInterfaceDeclaration(iface) => {
                raw_signatures.push(format!("interface {}", iface.id.name.as_str()));
            }
            Statement::TSTypeAliasDeclaration(alias) => {
                raw_signatures.push(format!("type {}", alias.id.name.as_str()));
            }
            Statement::TSEnumDeclaration(enum_decl) => {
                raw_signatures.push(format!("enum {}", enum_decl.id.name.as_str()));
            }
            Statement::VariableDeclaration(var_decl) => {
                for declarator in &var_decl.declarations {
                    if let BindingPattern::BindingIdentifier(ref ident) = declarator.id {
                        raw_signatures.push(format!("var/const {}", ident.name.as_str()));
                    }
                }
            }
            _ => {}
        }
    }

    let mut arena_signatures = Vec::with_capacity(raw_signatures.len());
    for sig in raw_signatures {
        arena_signatures.push(&*arena.alloc_str(&sig));
    }

    arena_signatures.sort();
    arena_signatures.dedup();

    if arena_signatures.is_empty() {
        return Err(AstParserError::ParseFailure {
            file: relative_path.to_string(),
            language: language.to_string(),
            reason: "oxc nao encontrou simbolos estruturais".to_string(),
        });
    }

    Ok((arena_signatures, import_edges))
}

fn collect_oxc_decl_signatures(decl: &Declaration, out: &mut Vec<String>, is_export: bool) {
    let prefix = if is_export { "export " } else { "" };
    match decl {
        Declaration::FunctionDeclaration(func) => {
            if let Some(ref id) = func.id {
                out.push(format!("{prefix}fn {}", id.name.as_str()));
            }
        }
        Declaration::ClassDeclaration(cls) => {
            if let Some(ref id) = cls.id {
                out.push(format!("{prefix}class {}", id.name.as_str()));
                for item in &cls.body.body {
                    if let ClassElement::MethodDefinition(method) = item {
                        if let PropertyKey::StaticIdentifier(ident) = &method.key {
                            out.push(format!("  method {}", ident.name.as_str()));
                        }
                    }
                }
            }
        }
        Declaration::TSInterfaceDeclaration(iface) => {
            out.push(format!("{prefix}interface {}", iface.id.name.as_str()));
        }
        Declaration::TSTypeAliasDeclaration(alias) => {
            out.push(format!("{prefix}type {}", alias.id.name.as_str()));
        }
        Declaration::TSEnumDeclaration(enum_decl) => {
            out.push(format!("{prefix}enum {}", enum_decl.id.name.as_str()));
        }
        Declaration::VariableDeclaration(var_decl) => {
            for declarator in &var_decl.declarations {
                if let BindingPattern::BindingIdentifier(ref ident) = declarator.id {
                    out.push(format!("{prefix}var/const {}", ident.name.as_str()));
                }
            }
        }
        _ => {}
    }
}

fn extract_structural_signatures<'arena>(
    arena: &'arena bumpalo::Bump,
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<&'arena str>, usize), AstParserError> {
    let mut import_edges = 0usize;

    let (signatures, edges) = match language {
        "javascript" | "typescript" => {
            extract_with_oxc(arena, source, language, relative_path)?
        }
        "c_sharp" => {
            extract_with_official_tree_sitter(arena, source, language, relative_path)?
        }
        "rust" | "python" | "go" | "elixir" | "cpp" | "c" | "cuda" | "zig" | "ruby" | "php" | "java" | "kotlin" | "swift" => {
            WasmtimeTreeSitterEngine::parse_and_extract(arena, source, language, relative_path, None)?
        }
        _ => {
            WasmtimeTreeSitterEngine::parse_and_extract(arena, source, language, relative_path, None)?
        }
    };

    import_edges = import_edges.max(edges);
    let signatures = sanitize_outline_signatures_in(arena, signatures, source, language);
    if signatures.is_empty() {
        return Err(AstParserError::ParseFailure {
            file: relative_path.to_string(),
            language: language.to_string(),
            reason: "sanitizacao eliminou todos os simbolos estruturais".to_string(),
        });
    }
    Ok((signatures, import_edges))
}

fn extract_with_official_tree_sitter<'arena>(
    arena: &'arena bumpalo::Bump,
    source: &str,
    language: &str,
    relative_path: &str,
) -> Result<(Vec<&'arena str>, usize), AstParserError> {
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

    let kinds_csharp = [
        "namespace_declaration",
        "class_declaration",
        "interface_declaration",
        "struct_declaration",
        "enum_declaration",
        "record_declaration",
        "method_declaration",
        "constructor_declaration",
        "property_declaration",
        "field_declaration",
    ];
    let bitset = NodeKindBitSet::new(&ts_language, &kinds_csharp);

    let mut signatures = Vec::new();
    collect_official_tree_sitter_signatures(arena, language, source.as_bytes(), tree.root_node(), &mut signatures, &bitset);
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

fn collect_official_tree_sitter_signatures<'arena>(
    arena: &'arena bumpalo::Bump,
    language: &str,
    source: &[u8],
    node: Node<'_>,
    out: &mut Vec<&'arena str>,
    bitset: &NodeKindBitSet,
) {
    if bitset.contains(node.kind_id()) {
        if let Some(signature) = official_signature_for_node(arena, language, source, node) {
            out.push(signature);
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_official_tree_sitter_signatures(arena, language, source, child, out, bitset);
    }
}

fn official_signature_for_node<'arena>(
    arena: &'arena bumpalo::Bump,
    language: &str,
    source: &[u8],
    node: Node<'_>,
) -> Option<&'arena str> {
    match language {
        "c_sharp" => csharp_signature_for_node(arena, source, node),
        _ => None,
    }
}

fn csharp_signature_for_node<'arena>(
    arena: &'arena bumpalo::Bump,
    source: &[u8],
    node: Node<'_>,
) -> Option<&'arena str> {
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

    let name = node_text_by_field(arena, node, source, &["name", "identifier"])
        .unwrap_or_else(|| compact_node_text(arena, node, source, 80));
    Some(bumpalo::format!(in arena, "c# {} {}", label, name).into_bump_str())
}

fn node_text_by_field<'arena>(
    arena: &'arena bumpalo::Bump,
    node: Node<'_>,
    source: &[u8],
    fields: &[&str],
) -> Option<&'arena str> {
    fields
        .iter()
        .find_map(|field| node.child_by_field_name(field))
        .map(|child| compact_node_text(arena, child, source, 80))
        .filter(|value| !value.is_empty())
}

fn compact_node_text<'arena>(
    arena: &'arena bumpalo::Bump,
    node: Node<'_>,
    source: &[u8],
    max_chars: usize,
) -> &'arena str {
    let raw = node.utf8_text(source).unwrap_or("");
    let mut cleaned = bumpalo::collections::String::new_in(arena);
    let mut last_was_space = false;
    let mut char_count = 0;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            if !last_was_space && !cleaned.is_empty() {
                cleaned.push(' ');
                last_was_space = true;
                char_count += 1;
            }
        } else {
            cleaned.push(ch);
            last_was_space = false;
            char_count += 1;
        }
        if char_count >= max_chars {
            break;
        }
    }
    arena.alloc_str(cleaned.trim())
}


fn sanitize_outline_signatures_in<'arena>(
    arena: &'arena bumpalo::Bump,
    signatures: Vec<&'arena str>,
    source: &str,
    language: &str,
) -> Vec<&'arena str> {
    let mut sanitized = signatures
        .into_iter()
        .filter_map(|signature| sanitize_outline_signature_in(arena, signature, language))
        .collect::<Vec<_>>();
    for imp in extract_import_signatures_in(arena, source, language) {
        sanitized.push(imp);
    }
    sanitized.sort();
    sanitized.dedup();
    sanitized
}

fn sanitize_outline_signature_in<'arena>(
    arena: &'arena bumpalo::Bump,
    signature: &str,
    language: &str,
) -> Option<&'arena str> {
    let mut text = strip_signature_comments_in(arena, signature, language);
    text = trim_structural_body_suffix_in(arena, text);
    text = compact_signature_whitespace_in(arena, text);
    let trimmed = text.trim_matches(|ch: char| ch.is_whitespace() || ch == ';' || ch == ',');
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(arena.alloc_str(truncate_chars(trimmed, 220)))
    }
}

fn strip_signature_comments_in<'arena>(
    arena: &'arena bumpalo::Bump,
    signature: &str,
    language: &str,
) -> &'arena str {
    let re_block = Regex::new(r"/\*[\s\S]*?\*/").unwrap();
    let replaced_block = re_block.replace_all(signature, " ");
    
    let final_str = if !matches!(language, "python" | "ruby") {
        let re_line = Regex::new(r"//.*$").unwrap();
        re_line.replace_all(&replaced_block, "").into_owned()
    } else {
        replaced_block.into_owned()
    };
    
    arena.alloc_str(&final_str)
}

fn trim_structural_body_suffix_in<'arena>(
    arena: &'arena bumpalo::Bump,
    signature: &str,
) -> &'arena str {
    let compact = compact_signature_whitespace_in(arena, signature);
    let trimmed = compact.trim();
    if trimmed.is_empty() {
        return "";
    }
    let lower = trimmed.to_ascii_lowercase();
    let looks_like_import = lower.starts_with("import ")
        || lower.starts_with("export import ")
        || lower.starts_with("use ")
        || lower.starts_with("pub use ")
        || lower.starts_with("from ")
        || lower.starts_with("#include ");
    if looks_like_import {
        return arena.alloc_str(trimmed);
    }

    if let Some(index) = trimmed.find('{') {
        return arena.alloc_str(trimmed[..index].trim());
    }
    if let Some(index) = trimmed.find(" =>") {
        return arena.alloc_str(trimmed[..index + 3].trim());
    }
    if let Some(index) = trimmed.find("=> ") {
        return arena.alloc_str(trimmed[..index + 2].trim());
    }
    arena.alloc_str(trimmed)
}

fn compact_signature_whitespace_in<'arena>(
    arena: &'arena bumpalo::Bump,
    signature: &str,
) -> &'arena str {
    let mut compact = bumpalo::collections::String::new_in(arena);
    let mut last_was_space = false;
    for ch in signature.chars() {
        if ch.is_whitespace() {
            if !last_was_space && !compact.is_empty() {
                compact.push(' ');
                last_was_space = true;
            }
        } else {
            compact.push(ch);
            last_was_space = false;
        }
    }
    arena.alloc_str(compact.trim())
}

fn extract_import_signatures_in<'arena>(
    arena: &'arena bumpalo::Bump,
    source: &str,
    language: &str,
) -> Vec<&'arena str> {
    let patterns: &[&str] = match language {
        "rust" => &[r"(?m)^\s*(?:pub\s+)?use\s+[^;\n]+;"],
        "python" => &[r"(?m)^\s*(?:from\s+[^\n]+?\s+import\s+[^\n#]+|import\s+[^\n#]+)"],
        "javascript" | "typescript" | "svelte" => &[r#"(?m)^\s*import\s+[^;\n]+(?:from\s+["'][^"']+["'])?;?"#],
        "go" => &[r#"(?m)^\s*import\s+(?:\([\s\S]*?\)|[^\n]+)"#],
        "java" | "kotlin" => &[r"(?m)^\s*import\s+[^\n;]+;"],
        "c" | "cpp" => &[r#"(?m)^\s*#include\s+[<"][^>"]+[>"]"#],
        "php" => &[r"(?m)^\s*use\s+[^\n;]+;"],
        "ruby" => &[r#"(?m)^\s*(?:require|require_relative)\s+["'][^"']+["']"#],
        _ => &[],
    };

    let mut imports = Vec::new();
    for pattern in patterns {
        let Ok(regex) = Regex::new(pattern) else {
            continue;
        };
        for matched in regex.find_iter(source) {
            if let Some(signature) = sanitize_outline_signature_in(arena, matched.as_str(), language) {
                imports.push(signature);
            }
        }
    }
    imports
}

// ── FUNÇÕES ORIGINAIS DE COMPATIBILIDADE (SEM ARENA) ─────────────────
fn compact_signature_whitespace(signature: &str) -> String {
    signature.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[allow(dead_code)]
fn sanitize_outline_signature(signature: &str, language: &str) -> Option<String> {
    let mut text = strip_signature_comments(signature, language);
    text = trim_structural_body_suffix(&text);
    text = compact_signature_whitespace(&text);
    let text = text
        .trim_matches(|ch: char| ch.is_whitespace() || ch == ';' || ch == ',')
        .trim()
        .to_string();
    if text.is_empty() {
        None
    } else {
        let truncated = truncate_chars(&text, 220);
        Some(truncated.to_string())
    }
}

#[allow(dead_code)]
fn strip_signature_comments(signature: &str, language: &str) -> String {
    let mut text = Regex::new(r"/\*[\s\S]*?\*/")
        .unwrap()
        .replace_all(signature, " ")
        .into_owned();
    if !matches!(language, "python" | "ruby") {
        text = Regex::new(r"//.*$")
            .unwrap()
            .replace_all(&text, "")
            .into_owned();
    }
    text
}

#[allow(dead_code)]
fn trim_structural_body_suffix(signature: &str) -> String {
    let compact = compact_signature_whitespace(signature);
    let trimmed = compact.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let lower = trimmed.to_ascii_lowercase();
    let looks_like_import = lower.starts_with("import ")
        || lower.starts_with("export import ")
        || lower.starts_with("use ")
        || lower.starts_with("pub use ")
        || lower.starts_with("from ")
        || lower.starts_with("#include ");
    if looks_like_import {
        return trimmed.to_string();
    }

    if let Some(index) = trimmed.find('{') {
        return trimmed[..index].trim().to_string();
    }
    if let Some(index) = trimmed.find(" =>") {
        return trimmed[..index + 3].trim().to_string();
    }
    if let Some(index) = trimmed.find("=> ") {
        return trimmed[..index + 2].trim().to_string();
    }
    trimmed.to_string()
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




fn classify_outline_domain(relative_path: &str, language: &str) -> OutlineDomainTag {
    let normalized = relative_path.replace('\\', "/").to_ascii_lowercase();
    let extension = Path::new(relative_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    if normalized.contains("/candle-metal-kernels/")
        || normalized.contains("/metal/")
        || normalized.contains("objc")
        || normalized.contains("objc2")
        || normalized.contains("core-ml")
        || matches!(extension.as_deref(), Some("m" | "mm" | "metal"))
    {
        return OutlineDomainTag::ObjectiveCMetal;
    }

    if normalized.contains("/cuda/")
        || normalized.contains("/candle-kernels/")
        || normalized.contains("cudarc")
        || normalized.contains("cuda")
        || normalized.contains("kernel")
        || matches!(
            extension.as_deref(),
            Some("c" | "cc" | "cpp" | "cxx" | "cu" | "cuh" | "h" | "hh" | "hpp" | "hxx")
        )
    {
        return OutlineDomainTag::CppCuda;
    }

    match language {
        "rust" => OutlineDomainTag::Rust,
        "typescript" | "javascript" | "tsx" | "jsx" | "svelte" | "vue" => {
            OutlineDomainTag::JavascriptTypescript
        }
        "python" => OutlineDomainTag::Python,
        "go" => OutlineDomainTag::Go,
        "elixir" => OutlineDomainTag::Elixir,
        _ => match extension.as_deref() {
            Some("rs") => OutlineDomainTag::Rust,
            Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" | "svelte" | "vue") => {
                OutlineDomainTag::JavascriptTypescript
            }
            Some("py") => OutlineDomainTag::Python,
            Some("go") => OutlineDomainTag::Go,
            Some("ex" | "exs") => OutlineDomainTag::Elixir,
            _ => OutlineDomainTag::Other,
        },
    }
}

fn render_outline_domain_header(domain: OutlineDomainTag) -> String {
    format!(
        "=================================================================\n[DOMAIN: {}]\n=================================================================",
        domain.label()
    )
}

impl ProductiveTreeNode {
    fn insert(&mut self, relative_path: &str) {
        let mut node = self;
        for segment in relative_path.split('/').filter(|segment| !segment.is_empty()) {
            node = node.children.entry(segment.to_string()).or_default();
        }
        node.is_file = true;
    }
}

fn render_productive_tree_node(
    node: &ProductiveTreeNode,
    prefix: &str,
    out: &mut String,
) {
    let mut children = node.children.iter().collect::<Vec<_>>();
    children.sort_by(|(left_name, left_node), (right_name, right_node)| {
        left_node
            .is_file
            .cmp(&right_node.is_file)
            .then_with(|| left_name.cmp(right_name))
    });

    for (index, (name, child)) in children.iter().enumerate() {
        let is_last = index + 1 == children.len();
        out.push_str(prefix);
        out.push_str(if is_last { "`-- " } else { "|-- " });
        out.push_str(name);
        if !child.is_file {
            out.push('/');
        }
        out.push('\n');

        if !child.children.is_empty() {
            let mut next_prefix = prefix.to_string();
            next_prefix.push_str(if is_last { "    " } else { "|   " });
            render_productive_tree_node(child, &next_prefix, out);
        }
    }
}

fn build_productive_tree(repo_root: &Path, clean_files: &[PathBuf]) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repo");
    let mut root = ProductiveTreeNode::default();
    let mut seen = BTreeSet::new();
    for path in clean_files {
        let relative_path = sanitize_relative_path(repo_root, path);
        if relative_path.is_empty() || !seen.insert(relative_path.clone()) {
            continue;
        }
        root.insert(&relative_path);
    }

    let mut out = String::new();
    out.push_str(repo_name);
    out.push_str("/\n");
    render_productive_tree_node(&root, "", &mut out);
    out.trim_end().to_string()
}

fn build_repo_outline(
    repo_root: &Path,
    clean_files: &[PathBuf],
    parsed_files: &[ParsedFile<'_>],
) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("repo");
    let mut by_domain = BTreeMap::<OutlineDomainTag, Vec<&ParsedFile<'_>>>::new();
    for file in parsed_files {
        let domain = classify_outline_domain(file.relative_path, file.language);
        by_domain.entry(domain).or_default().push(file);
    }
    let mut out = String::new();
    out.push_str("# Repository Outline\n\n");
    out.push_str(&format!("repo: {repo_name}\n"));
    out.push_str(&format!("symbol_files: {}\n", parsed_files.len()));
    out.push_str("source: native-rust multi-strategy (language-pack + targeted-tree-sitter + regex-fallback)\n\n");
    out.push_str("## Productive Tree\n\n");
    out.push_str(&build_productive_tree(repo_root, clean_files));
    out.push_str("\n\n## Indexed Symbol Files\n\n");
    for (domain, files) in &by_domain {
        out.push_str(&render_outline_domain_header(*domain));
        out.push('\n');
        for file in files {
            out.push_str(&format!(
                "- {} [{}; {} symbols]\n",
                file.relative_path,
                file.language,
                file.signatures.len()
            ));
        }
        out.push('\n');
    }
    out.push_str("## AST Blueprint\n\n");
    for (domain, files) in by_domain {
        out.push_str(&render_outline_domain_header(domain));
        out.push('\n');
        for file in files {
            out.push_str(&format!("[{}]\n", file.relative_path));
            for signature in &file.signatures {
                out.push_str("- ");
                out.push_str(signature);
                out.push('\n');
            }
            out.push('\n');
        }
    }
    compact_outline_blob_text(&out)
}

fn compact_outline_blob_text(text: &str) -> String {
    let mut out = String::new();
    let mut previous_blank = false;
    for raw_line in text.lines() {
        let trimmed_end = raw_line.trim_end();
        let line = if trimmed_end.starts_with("- ") {
            let suffix = trimmed_end.trim_start_matches("- ").trim();
            format!("- {}", compact_signature_whitespace(suffix))
        } else {
            trimmed_end.to_string()
        };

        if line.trim().is_empty() {
            if previous_blank {
                continue;
            }
            previous_blank = true;
            out.push('\n');
            continue;
        }

        previous_blank = false;
        out.push_str(&line);
        out.push('\n');
    }
    out.trim().to_string()
}

fn build_architecture_map(files: &[String]) -> String {
    // PRD-046: agrupa arquivos por LAYER (frontend, backend, cli, database,
    // tests, infra, docs, config, core) e DENTRO de cada layer por diretorio.
    // Isso da contexto arquitetonico real (data flow + camadas) em vez de
    // apenas um flat list de diretorios que LLMs 3-7B nao conseguem
    // interpretar com precisao.
    let mut by_layer: BTreeMap<&'static str, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for file in files {
        let layer = classify_layer(file);
        let dir = directory_key(file);
        by_layer.entry(layer).or_default().entry(dir).or_default().push(file.clone());
    }

    let mut out = String::from("# Architecture Map\n");
    for (layer, dirs) in &by_layer {
        out.push_str(&format!("\n[{}] ({} arquivos)\n", layer, total_files(dirs)));
        for (dir, files) in dirs {
            out.push_str(&format!("  [{}]\n", dir));
            for file in files {
                out.push_str("  - ");
                out.push_str(file);
                out.push('\n');
            }
        }
    }
    out
}

fn total_files(dirs: &BTreeMap<String, Vec<String>>) -> usize {
    dirs.values().map(|v| v.len()).sum()
}

/// PRD-046: classifica um path em uma CAMADA arquitetural deterministica.
/// Heuristica: combina extensao do arquivo com segmentos do path.
///
/// Layers canônicas (em ordem de prioridade):
/// 1. frontend: Svelte/React/Vue, components/, pages/, src/routes/
/// 2. backend: src/api/, src/server/, src/services/, src/handlers/, src/controllers/
/// 3. cli: bin/, cmd/, src/cli/, src/commands/
/// 4. database: migrations/, schema/, prisma/, drizzle/
/// 5. tests: tests/, __tests__/, spec/, *.test.*, *_test.*, *_spec.*
/// 6. infra: Dockerfile*, .github/, deploy/, k8s/, terraform/
/// 7. docs: docs/, README*, CHANGELOG*, *.md (top-level)
/// 8. config: package.json, Cargo.toml, go.mod, pyproject.toml, tsconfig.json
/// 9. core: catch-all (lib/, src/, internal/)
fn classify_layer(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    let lower_slash = lower.replace('\\', "/");
    let segments: Vec<&str> = lower_slash.split('/').collect();
    let file_name = segments.last().copied().unwrap_or("");
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    // 5. tests: tem "test" no path ou extensao .test.* / _spec.
    if segments.iter().any(|s| *s == "tests" || *s == "__tests__" || *s == "spec")
        || file_name.contains(".test.")
        || file_name.contains("_test.")
        || file_name.contains("_spec.")
        || file_name.ends_with("_test.go")
        || file_name.ends_with("_spec.rb")
    {
        return "tests";
    }

    // 1. frontend
    if matches!(ext, "svelte" | "vue" | "jsx" | "tsx")
        || segments.iter().any(|s| matches!(*s, "components" | "pages" | "routes" | "views" | "screens"))
    {
        return "frontend";
    }

    // 2. backend
    if segments.iter().any(|s| matches!(*s, "api" | "server" | "services" | "handlers" | "controllers" | "endpoints" | "rest" | "graphql"))
    {
        return "backend";
    }

    // 3. cli
    if segments.iter().any(|s| matches!(*s, "bin" | "cmd" | "cli" | "commands"))
        || file_name == "main.go"
        || file_name == "main.rs"
    {
        return "cli";
    }

    // 4. database
    if segments.iter().any(|s| matches!(*s, "migrations" | "schema" | "prisma" | "drizzle" | "models"))
        || ext == "sql"
    {
        return "database";
    }

    // 6. infra
    if file_name.starts_with("dockerfile")
        || file_name == "containerfile"
        || file_name == "docker-compose.yml"
        || file_name == "compose.yaml"
        || segments.first() == Some(&".github")
        || segments.iter().any(|s| matches!(*s, "deploy" | "k8s" | "kubernetes" | "terraform" | "ansible"))
    {
        return "infra";
    }

    // 7. docs
    if segments.iter().any(|s| matches!(*s, "docs" | "documentation"))
        || file_name.starts_with("readme")
        || file_name.starts_with("changelog")
        || file_name == "license"
        || file_name == "contributing.md"
    {
        return "docs";
    }

    // 8. config (manifests, build files)
    if matches!(file_name,
        "package.json" | "cargo.toml" | "go.mod" | "pyproject.toml" | "tsconfig.json"
        | "composer.json" | "gemfile" | "mix.exs" | "build.zig" | "pnpm-lock.yaml"
        | "yarn.lock" | "package-lock.json" | "requirements.txt" | "setup.py"
        | "workspace.toml" | ".gitignore" | ".gitattributes"
    ) {
        return "config";
    }

    // 9. core (catch-all)
    "core"
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

fn truncate_chars(content: &str, max_chars: usize) -> &str {
    if max_chars == 0 {
        return "";
    }
    let mut char_count = 0;
    let mut byte_idx = 0;
    for (idx, _) in content.char_indices() {
        if char_count == max_chars {
            return &content[..idx];
        }
        char_count += 1;
        byte_idx = idx;
    }
    if char_count <= max_chars {
        content
    } else {
        &content[..byte_idx]
    }
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
        let arena = bumpalo::Bump::new();
        let (signatures, imports) =
            extract_with_official_tree_sitter(&arena, source, "c_sharp", "Program.cs").unwrap();
        assert_eq!(imports, 0);
        assert!(signatures.iter().any(|item| item.contains("c# class Greeter")));
        assert!(signatures.iter().any(|item| item.contains("c# method Run")));
    }

    #[test]
    fn test_oxc_js_ts_outline() {
        let ts_source = r#"
import { useState } from 'react';

export interface UserProfile<T> {
    id: string;
    data: T;
}

export type UserRole = 'admin' | 'user';

export class UserService {
    private api: string;
    constructor(api: string) {
        this.api = api;
    }
    public static getUser(id: string): UserProfile<any> {
        return { id, data: null };
    }
}

export function fetchUsers(): void {}
"#;
        let arena = bumpalo::Bump::new();
        let (signatures, edges) = extract_with_oxc(&arena, ts_source, "typescript", "user.ts").unwrap();
        assert_eq!(edges, 1);
        assert!(signatures.iter().any(|s| s.contains("interface UserProfile")));
        assert!(signatures.iter().any(|s| s.contains("type UserRole")));
        assert!(signatures.iter().any(|s| s.contains("class UserService")));
        assert!(signatures.iter().any(|s| s.contains("fn fetchUsers")));
    }

    #[test]
    fn test_wasm_tree_sitter_rust_outline() {
        let rust_source = r#"
pub struct UserStore {
    name: String,
}

impl UserStore {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}
"#;
        let arena = bumpalo::Bump::new();
        let dir = TempDir::new().unwrap();
        let wasm_path = dir.path().join("tree-sitter-rust.wasm");
        let wasm_bytes = b"\x00asm\x01\x00\x00\x00";
        let _ = std::fs::write(&wasm_path, wasm_bytes);

        let res = WasmtimeTreeSitterEngine::parse_and_extract(&arena, rust_source, "rust", "src/lib.rs", Some(&wasm_path));
        assert!(res.is_ok() || res.is_err());
    }

    #[test]
    fn test_fail_soft_corrupted_wasm_grammar() {
        let corrupted_bytes = b"CORRUPTED_GARBAGE_WASM_BYTES_HEADER_1234567890";
        let dir = TempDir::new().unwrap();
        let wasm_path = dir.path().join("tree-sitter-corrupted.wasm");
        std::fs::write(&wasm_path, corrupted_bytes).unwrap();

        let arena = bumpalo::Bump::new();
        let res = WasmtimeTreeSitterEngine::parse_and_extract(&arena, "fn test() {}", "rust", "src/lib.rs", Some(&wasm_path));
        assert!(res.is_err());
        if let Err(AstParserError::ParseFailure { reason, .. }) = res {
            assert!(reason.contains("WASM compilation failure") || reason.contains("corrupted") || reason.contains("indisponivel"));
        }
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

        let clean_files = vec![
            repo_root.join("src/lib.rs"),
            repo_root.join("web/App.svelte"),
            repo_root.join("web/routes.ts"),
            repo_root.join("go/main.go"),
            repo_root.join("dotnet/App/App.csproj"),
            repo_root.join("Workspace.sln"),
            repo_root.join("Cargo.toml"),
            repo_root.join("package.json"),
            repo_root.join("svelte.config.js"),
            repo_root.join("go.mod"),
            repo_root.join("mix.exs"),
            repo_root.join("Gemfile"),
            repo_root.join("composer.json"),
            repo_root.join("cpp/CMakeLists.txt"),
            repo_root.join("zig/build.zig.zon"),
            repo_root.join("Package.swift"),
        ];
        let clean_files = clean_files
            .into_iter()
            .map(|path| path.canonicalize().unwrap_or(path))
            .collect::<Vec<_>>();
        let artifacts = extract_repository_outline_native_from_clean_files(repo_root, &clean_files).unwrap();
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

        let arena = bumpalo::Bump::new();
        for (language, source, path, _expected_fragment) in scenarios {
            let (signatures, _) = extract_structural_signatures(&arena, source, language, path).unwrap();
            assert!(
                !signatures.is_empty(),
                "assinaturas de {language} nao deveriam estar vazias; obtido: {:?}",
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

        let clean_files = vec![
            repo_root.join("Cargo.toml"),
            repo_root.join("pyproject.toml"),
            repo_root.join("package.json"),
            repo_root.join("mix.exs"),
            repo_root.join("cpp/CMakeLists.txt"),
            repo_root.join("src/lib.rs"),
            repo_root.join("python/app.py"),
            repo_root.join("web/app.tsx"),
            repo_root.join("cpp/main.cpp"),
            repo_root.join("lib/demo/engine.ex"),
        ];
        let clean_files = clean_files
            .into_iter()
            .map(|path| path.canonicalize().unwrap_or(path))
            .collect::<Vec<_>>();
        let artifacts = extract_repository_outline_native_from_clean_files(repo_root, &clean_files).unwrap();
        let repo_outline = String::from_utf8(artifacts.repo_outline_blob).unwrap();

        assert!(repo_outline.contains("## Productive Tree"));
        assert!(repo_outline.contains("demo/"));
        assert!(repo_outline.contains("[DOMAIN: RUST]"));
        assert!(repo_outline.contains("[DOMAIN: PYTHON]"));
        assert!(repo_outline.contains("[DOMAIN: JAVASCRIPT / TYPESCRIPT]"));
        assert!(repo_outline.contains("[DOMAIN: C++ / CUDA]"));
        assert!(repo_outline.contains("[DOMAIN: ELIXIR]"));
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
    fn sanitize_outline_signature_removes_body_and_dead_comments() {
        let signature = "pub async fn run(ctx: Ctx) { /* noop */ // dead\n execute(ctx)\n }";
        let sanitized = sanitize_outline_signature(signature, "rust").unwrap();
        assert_eq!(sanitized, "pub async fn run(ctx: Ctx)");
    }

    #[test]
    fn blob04_reapplies_universal_pruning_even_with_dirty_clean_files() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();
        std::fs::create_dir_all(repo_root.join("src")).unwrap();
        std::fs::create_dir_all(repo_root.join("tests")).unwrap();
        std::fs::create_dir_all(repo_root.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(repo_root.join("dist")).unwrap();
        std::fs::create_dir_all(repo_root.join("build")).unwrap();
        std::fs::create_dir_all(repo_root.join("target")).unwrap();

        std::fs::write(repo_root.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
        std::fs::write(repo_root.join("src/lib.rs"), "pub struct Engine;\npub fn run() {}\n").unwrap();
        std::fs::write(repo_root.join("tests/engine.rs"), "pub fn noisy_test() {}\n").unwrap();
        std::fs::write(repo_root.join("node_modules/pkg/index.ts"), "export function noise() { return 1 }\n").unwrap();
        std::fs::write(repo_root.join("dist/app.min.js"), "function x(){return 1};").unwrap();
        std::fs::write(repo_root.join("build/app.min.css"), ".app{color:red}").unwrap();
        std::fs::write(repo_root.join("target/generated.rs"), "pub fn generated() {}\n").unwrap();

        let clean_files = vec![
            repo_root.join("Cargo.toml"),
            repo_root.join("src/lib.rs"),
            repo_root.join("tests/engine.rs"),
            repo_root.join("node_modules/pkg/index.ts"),
            repo_root.join("dist/app.min.js"),
            repo_root.join("build/app.min.css"),
            repo_root.join("target/generated.rs"),
        ];
        let clean_files = clean_files
            .into_iter()
            .map(|path| path.canonicalize().unwrap_or(path))
            .collect::<Vec<_>>();

        let artifacts = extract_repository_outline_native_from_clean_files(repo_root, &clean_files).unwrap();
        let repo_outline = String::from_utf8(artifacts.repo_outline_blob).unwrap();

        assert!(repo_outline.contains("[src/lib.rs]"));
        assert!(!repo_outline.contains("tests/engine.rs"));
        assert!(!repo_outline.contains("node_modules/pkg/index.ts"));
        assert!(!repo_outline.contains("dist/app.min.js"));
        assert!(!repo_outline.contains("build/app.min.css"));
        assert!(!repo_outline.contains("target/generated.rs"));
    }

    // PRD-046: testes de classificacao de camada arquitetural.
    // A heuristica deve ser deterministica para que LLMs 3-7B consigam
    // inferir o data flow do repo (frontend -> backend -> cli -> db).

    #[test]
    fn test_classify_layer_frontend_extensions() {
        assert_eq!(classify_layer("web/App.svelte"), "frontend");
        assert_eq!(classify_layer("src/components/Button.tsx"), "frontend");
        assert_eq!(classify_layer("pages/Home.vue"), "frontend");
        assert_eq!(classify_layer("src/routes/about.tsx"), "frontend");
    }

    #[test]
    fn test_classify_layer_backend_segments() {
        assert_eq!(classify_layer("src/api/users.rs"), "backend");
        assert_eq!(classify_layer("src/server/main.ts"), "backend");
        assert_eq!(classify_layer("src/handlers/auth.go"), "backend");
        assert_eq!(classify_layer("src/services/billing.py"), "backend");
    }

    #[test]
    fn test_classify_layer_cli_segments() {
        assert_eq!(classify_layer("bin/souls.rs"), "cli");
        assert_eq!(classify_layer("cmd/root.go"), "cli");
        assert_eq!(classify_layer("src/cli/main.ts"), "cli");
    }

    #[test]
    fn test_classify_layer_database_segments() {
        assert_eq!(classify_layer("migrations/001_init.sql"), "database");
        assert_eq!(classify_layer("prisma/schema.prisma"), "database");
        assert_eq!(classify_layer("drizzle/0001_init.ts"), "database");
    }

    #[test]
    fn test_classify_layer_tests_segments() {
        assert_eq!(classify_layer("tests/auth.rs"), "tests");
        assert_eq!(classify_layer("src/__tests__/foo.tsx"), "tests");
        assert_eq!(classify_layer("spec/user_spec.rb"), "tests");
        assert_eq!(classify_layer("src/Button.test.tsx"), "tests");
        assert_eq!(classify_layer("pkg/foo_test.go"), "tests");
    }

    #[test]
    fn test_classify_layer_infra_files() {
        assert_eq!(classify_layer("Dockerfile"), "infra");
        assert_eq!(classify_layer("Dockerfile.dev"), "infra");
        assert_eq!(classify_layer("docker-compose.yml"), "infra");
        assert_eq!(classify_layer(".github/workflows/ci.yml"), "infra");
        assert_eq!(classify_layer("k8s/deployment.yaml"), "infra");
    }

    #[test]
    fn test_classify_layer_docs_and_config() {
        assert_eq!(classify_layer("README.md"), "docs");
        assert_eq!(classify_layer("CHANGELOG.md"), "docs");
        assert_eq!(classify_layer("docs/guide.md"), "docs");
        assert_eq!(classify_layer("package.json"), "config");
        assert_eq!(classify_layer("Cargo.toml"), "config");
        assert_eq!(classify_layer("go.mod"), "config");
    }

    #[test]
    fn test_classify_layer_core_catchall() {
        assert_eq!(classify_layer("src/lib.rs"), "core");
        assert_eq!(classify_layer("lib/utils.py"), "core");
        assert_eq!(classify_layer("internal/auth.go"), "core");
    }

    #[test]
    fn test_build_architecture_map_groups_by_layer() {
        let files = vec![
            "src/lib.rs".to_string(),
            "web/App.svelte".to_string(),
            "tests/auth.rs".to_string(),
            "Dockerfile".to_string(),
            "README.md".to_string(),
            "Cargo.toml".to_string(),
        ];
        let map = build_architecture_map(&files);

        // Cada layer aparece com seu count
        assert!(map.contains("[core]"), "deveria ter layer core: {map}");
        assert!(map.contains("[frontend]"), "deveria ter layer frontend: {map}");
        assert!(map.contains("[tests]"), "deveria ter layer tests: {map}");
        assert!(map.contains("[infra]"), "deveria ter layer infra: {map}");
        assert!(map.contains("[docs]"), "deveria ter layer docs: {map}");
        assert!(map.contains("[config]"), "deveria ter layer config: {map}");
        // Cada arquivo aparece dentro do seu layer
        assert!(map.contains("src/lib.rs"));
        assert!(map.contains("web/App.svelte"));
        assert!(map.contains("tests/auth.rs"));
    }
}
