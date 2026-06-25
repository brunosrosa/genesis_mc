use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ignore::WalkBuilder;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RepoRadar {
    repo_root: PathBuf,
    all_files: Vec<PathBuf>,
    clean_files: Vec<PathBuf>,
    root_files: BTreeMap<String, PathBuf>,
}

impl RepoRadar {
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub fn all_files(&self) -> &[PathBuf] {
        &self.all_files
    }

    pub fn clean_files(&self) -> &[PathBuf] {
        &self.clean_files
    }

    pub fn find_root_file_case_insensitive(&self, candidate: &str) -> Option<PathBuf> {
        self.root_files
            .get(&candidate.to_ascii_lowercase())
            .cloned()
    }

    pub fn clean_files_under(&self, execution_root: &Path) -> Vec<String> {
        let normalized_root = execution_root
            .canonicalize()
            .unwrap_or_else(|_| execution_root.to_path_buf());
        let mut out = self
            .clean_files
            .iter()
            .filter(|path| path.starts_with(&normalized_root))
            .filter_map(|path| {
                path.strip_prefix(&normalized_root)
                    .ok()
                    .map(|rel| rel.to_string_lossy().replace('\\', "/"))
                    .filter(|rel| !rel.is_empty())
            })
            .collect::<Vec<_>>();
        out.sort();
        out
    }
}

pub fn build_repo_radar(repo_root: &Path) -> Arc<RepoRadar> {
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    Arc::new(scan_repo_radar(&repo_root))
}

fn scan_repo_radar(repo_root: &Path) -> RepoRadar {
    let mut builder = WalkBuilder::new(repo_root);
    builder.hidden(false);
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);
    builder.require_git(false);
    builder.filter_entry(|entry| {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let name = entry
                .path()
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default();
            return !should_skip_universal_dir(name);
        }
        true
    });

    let mut all_files = Vec::new();
    let mut clean_files = Vec::new();
    let mut root_files = BTreeMap::new();

    for item in builder.build() {
        let entry = match item {
            Ok(entry) => entry,
            Err(err) => {
                warn!(
                    repo_root = %repo_root.display(),
                    error = %err,
                    "radar-global: falha ao caminhar entrada; ignorando"
                );
                continue;
            }
        };

        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }

        let path = entry.into_path();
        if should_skip_universal_file(&path) {
            continue;
        }

        if path.parent() == Some(repo_root) {
            if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                root_files.insert(name.to_ascii_lowercase(), path.clone());
            }
        }

        let relative = sanitize_relative_path(repo_root, &path);
        all_files.push(path.clone());
        if should_skip_clean_relative_path(&relative) {
            continue;
        }
        clean_files.push(path);
    }

    all_files.sort();
    clean_files.sort();
    clean_files.dedup();

    RepoRadar {
        repo_root: repo_root.to_path_buf(),
        all_files,
        clean_files,
        root_files,
    }
}

fn sanitize_relative_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn should_skip_universal_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        ".git"
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
            | "venv"
            | ".venv"
    )
}

fn should_skip_universal_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file_name.ends_with(".min.js")
        || file_name.ends_with(".min.cjs")
        || file_name.ends_with(".min.mjs")
    {
        return true;
    }
    has_binary_or_image_extension(path)
}

fn should_skip_clean_relative_path(relative_path: &str) -> bool {
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
                | "generated"
                | "__generated__"
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

fn has_binary_or_image_extension(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "bmp"
            | "tiff"
            | "pdf"
            | "zip"
            | "gz"
            | "tar"
            | "7z"
            | "rar"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "bin"
            | "dat"
            | "db"
            | "sqlite"
            | "mp3"
            | "mp4"
            | "mov"
            | "avi"
            | "wav"
            | "flac"
    )
}

