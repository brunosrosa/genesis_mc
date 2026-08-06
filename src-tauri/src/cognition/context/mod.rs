pub mod ansi_filter;
pub mod atomic_once;
pub mod ccr_dedup;
pub mod ccr_types;
pub mod dedup;
pub mod extensions;
pub mod multi_read;
pub mod myers_diff;
pub mod souls_read;
pub mod souls_search;
pub mod souls_smart_read;
pub mod souls_tree;

pub mod context_compression {
    pub use super::ccr_dedup as dedup;
    pub use super::ccr_dedup::*;
    pub use super::ccr_types::*;
    pub use super::multi_read::*;
}
pub use ccr_dedup::{
    clear_dedup_cache, compress_with_dedup, dedup_marker_for, hash_block, is_blank_line,
    DEDUP_CACHE,
};
pub use ccr_types::BlockCompressionStats;
pub use multi_read::{multi_read_concurrent, FileCompaction};
pub use souls_search as search;
pub use souls_smart_read as smart_read;

pub use ansi_filter::{ansi_density, strip_ansi};
pub use atomic_once::FireOnce;
pub use dedup::{
    clear_session_cache, clear_session_dedup_cache, deduplicate_blocks, deduplicate_blocks_session,
    SESSION_DEDUP_CACHE,
};
pub use extensions::{is_excluded_dir, is_source_ext, EXCLUDE_DIRS, SOURCE_EXTENSIONS};
pub use myers_diff::myers_diff;
pub use souls_read::{aggressive_compress, compress_to_lean, lightweight_cleanup, read_to_lean, MAX_READ_BYTES};
pub use souls_search::{format_lean_notation, search_lean, SearchMatch};
pub use souls_smart_read::{count_tokens, extract_outline_signatures, smart_read_text, smart_read_text_for_lang};
pub use souls_tree::dot_flatten;
