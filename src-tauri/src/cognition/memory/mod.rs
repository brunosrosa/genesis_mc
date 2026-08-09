pub mod chyros_daemon;
pub mod fts_retriever;
pub mod langevin_decay;
pub mod rrf_fusion;
pub mod vector_retriever;

pub use chyros_daemon::{ActivityTracker, ChyrosDaemon, ConsolidationReport};
pub use fts_retriever::{FtsRetriever, LexicalMatch};
pub use langevin_decay::{apply_langevin_decay, proj_poincare, PoincareVector};
pub use rrf_fusion::{load_tombstones, RrfFusionEngine, UnifiedMatch, DEFAULT_RRF_K};
pub use vector_retriever::{VectorRetriever, VectorialMatch};

/// Migração idempotente do FrankenSQLite elevando o schema para suporte do status de estabilidade,
/// coordenadas na Bola de Poincaré e fila de eventos L0 em modo STRICT.
pub fn init_memory_schema(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS souls_memory_nodes (
             memory_id TEXT PRIMARY KEY,
             content TEXT NOT NULL,
             stability_status TEXT NOT NULL CHECK(stability_status IN ('STABLE', 'EVOLVING', 'SUPERSEDED')),
             relevance_score REAL NOT NULL DEFAULT 1.0,
             poincare_x REAL NOT NULL DEFAULT 0.0,
             poincare_y REAL NOT NULL DEFAULT 0.0,
             updated_at INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE IF NOT EXISTS souls_raw_events_l0 (
             event_id INTEGER PRIMARY KEY AUTOINCREMENT,
             event_type TEXT NOT NULL,
             payload TEXT NOT NULL,
             processed INTEGER NOT NULL DEFAULT 0,
             created_at INTEGER NOT NULL
         ) STRICT;"
    )?;
    Ok(())
}
