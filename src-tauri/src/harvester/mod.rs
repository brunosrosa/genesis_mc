pub mod ramdisk;
#[cfg(target_os = "windows")]
pub mod projfs;
pub mod git;
pub mod sandbox;
pub mod detect;
pub mod router;
pub mod ast_parser;
pub mod sidecar;
pub mod web_scraper;
pub mod github_tracker;
pub mod extract;
pub mod community;
pub mod persist;
pub mod guard;
pub mod orchestrator;
pub mod canon;

pub(crate) const PHASE1_HEAVY_BLOB_MAX_CHARS: usize = 500_000;

pub(crate) fn normalize_repo_path_key(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace('\\', "/")
        .replace("::", "/")
}

pub(crate) fn normalized_path_has_any_segment(normalized: &str, segments: &[&str]) -> bool {
    normalized.split('/').any(|segment| segments.contains(&segment))
}
