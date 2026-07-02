use std::path::{Path, PathBuf};

use crate::harvester::router::StaticAnalysisBlade;
use super::{SastExecutionTarget, derive_repo_relative_clean_targets, blade_file_batch_scope, PYTHON_LINT_FILE_LIST_CHUNK_SIZE};

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

pub fn is_python_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("py"))
        .unwrap_or(false)
}

pub fn ruff_args(scan_targets: &[String]) -> Vec<String> {
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

pub fn derive_python_execution_targets(
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
