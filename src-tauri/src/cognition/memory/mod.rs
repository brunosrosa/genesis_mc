pub mod chyros_daemon;
pub mod fts_retriever;
pub mod ladybug_firewall;
pub mod langevin_decay;
pub mod rrf_fusion;
pub mod vector_retriever;

pub use chyros_daemon::{ActivityTracker, ChyrosDaemon, ConsolidationReport};
pub use fts_retriever::{FtsRetriever, LexicalMatch};
pub use ladybug_firewall::{FirewallVerdict, OntologicalEdge, OntologicalFirewall, OntologicalNode};
pub use langevin_decay::{apply_langevin_decay, proj_poincare, PoincareVector};
pub use rrf_fusion::{
    is_exact_term_match, load_tombstones, RrfFusionEngine, UnifiedMatch, DEFAULT_RRF_K,
    EXACT_MATCH_BONUS,
};
pub use vector_retriever::{
    get_hippocampus_table_schema, HippocampusMemoryRecord, VectorRetriever, VectorialMatch,
    CANONICAL_TABLE_NAME, VECTOR_DIMENSION,
};

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
         ) STRICT;

         CREATE TABLE IF NOT EXISTS repo_heatmap (
             file_path TEXT PRIMARY KEY,
             frecency_score REAL NOT NULL,
             last_modified_epoch INTEGER NOT NULL,
             modification_count INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX IF NOT EXISTS idx_heatmap_score ON repo_heatmap(frecency_score DESC);

         CREATE TABLE IF NOT EXISTS weevolve_ratings (
             target_id TEXT PRIMARY KEY,
             rating_type TEXT NOT NULL CHECK(rating_type IN ('MODEL', 'TOOL', 'PROMPT')),
             elo_rating REAL NOT NULL DEFAULT 1200.0,
             ema_score REAL NOT NULL DEFAULT 1.0,
             total_matches INTEGER NOT NULL DEFAULT 0,
             updated_at INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE IF NOT EXISTS weevolve_feedbacks (
             feedback_id TEXT PRIMARY KEY,
             target_id TEXT NOT NULL,
             feedback_type TEXT NOT NULL CHECK(feedback_type IN ('EXPLICIT_POSITIVE', 'EXPLICIT_NEGATIVE', 'IMPLICIT_POSITIVE', 'IMPLICIT_NEGATIVE')),
             source_action TEXT NOT NULL,
             reward_value REAL NOT NULL,
             created_at INTEGER NOT NULL,
             FOREIGN KEY(target_id) REFERENCES weevolve_ratings(target_id) ON DELETE CASCADE
         ) STRICT;"
    )?;
    Ok(())
}
