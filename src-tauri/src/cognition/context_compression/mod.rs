// SOULS-CANIBALIZED Marco 3.6: Conveyor Belt de Contexto (CCR Lossless).
//
// Compressão reversível por janela deslizante de 5 linhas (hash DefaultHasher u64)
// e rehidratação determinística via DashMap<u64, String> em RAM Host.
// Zero dependência nova (dashmap 6.1.0, tokio 1.51.1 já presentes).
// Emenda construtiva da ADR-037 §3 (Paradigma CCR) e compatibiliza com
// ADR-041 (Servername Soberano `souls_mcp` + tetos 32/120).
//
// Coexistência com `lean_vacuum::dedup` (Marco 3): a canibalização é
// complementar. O `lean_vacuum::dedup` permanece para testes de snapshot e
// semântica cross-file destrutiva; este módulo adiciona a semântica
// lossless reversível exigida pelo CCR genuíno (souls_fill).

pub mod dedup;
pub mod multi_read;
pub mod types;

pub use dedup::{
    clear_dedup_cache, compress_with_dedup, dedup_marker_for, hash_block, is_blank_line,
    DEDUP_CACHE,
};
pub use multi_read::{multi_read_concurrent, FileCompaction};
pub use types::BlockCompressionStats;
