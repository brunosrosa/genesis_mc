// SOULS V6 — MARCO 5.5.0: Módulo Context Compression
pub mod context_stitcher;
pub mod dedup;

pub use context_stitcher::{count_tokens_gigatoken, pad_zone_to_64_tokens, ContextStitcher};
pub use dedup::{
    ccr_cache, ccr_marker_for, clear_ccr_cache, compress, hash_text, rehydrate_ccr,
    CCR_HOST_RAM_CACHE, MARKER_MID, MARKER_PREFIX, MARKER_SUFFIX,
};

// Re-exports para retrocompatibilidade com chamadas legadas de ccr_dedup, ccr_types e multi_read
pub use crate::cognition::context::ccr_dedup::{
    clear_dedup_cache, compress_with_dedup, dedup_marker_for, hash_block, is_blank_line,
    DEDUP_CACHE,
};
pub use crate::cognition::context::ccr_types::*;
pub use crate::cognition::context::multi_read::*;
