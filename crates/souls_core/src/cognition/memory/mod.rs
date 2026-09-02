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

/// Migração idempotente do FrankenSQLite elevando o schema para suporte da Tríade de Memória L2/L3,
/// Persistência Socrática e Log de Calor Langevin em modo STRICT (V5).
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

         CREATE TABLE IF NOT EXISTS socratic_sessions (
             session_id TEXT PRIMARY KEY,
             created_at INTEGER NOT NULL,
             metadata   TEXT NOT NULL DEFAULT '{}'
         ) STRICT;

         CREATE TABLE IF NOT EXISTS socratic_thoughts (
             thought_id        TEXT PRIMARY KEY,
             session_id        TEXT NOT NULL,
             branch_id         TEXT NOT NULL DEFAULT 'main',
             parent_thought_id TEXT,
             thought_type      TEXT NOT NULL,
             content           TEXT NOT NULL,
             step_number       INTEGER NOT NULL,
             duration_ms       INTEGER NOT NULL DEFAULT 0,
             created_at        INTEGER NOT NULL,
             FOREIGN KEY(session_id)        REFERENCES socratic_sessions(session_id) ON DELETE CASCADE,
             FOREIGN KEY(parent_thought_id) REFERENCES socratic_thoughts(thought_id) ON DELETE CASCADE
         ) STRICT;

         CREATE INDEX IF NOT EXISTS idx_thoughts_session ON socratic_thoughts(session_id);
         CREATE INDEX IF NOT EXISTS idx_thoughts_branch ON socratic_thoughts(branch_id);
         CREATE INDEX IF NOT EXISTS idx_thoughts_parent ON socratic_thoughts(parent_thought_id);
         CREATE INDEX IF NOT EXISTS idx_thoughts_session_step ON socratic_thoughts(session_id, step_number);

         CREATE TABLE IF NOT EXISTS repo_heatmap (
             file_path TEXT PRIMARY KEY,
             frecency_score REAL NOT NULL,
             last_modified_epoch INTEGER NOT NULL,
             modification_count INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX IF NOT EXISTS idx_heatmap_score ON repo_heatmap(frecency_score DESC);

         CREATE TABLE IF NOT EXISTS repo_heatmap_log (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             file_path TEXT NOT NULL,
             access_type TEXT NOT NULL,
             accessed_at INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX IF NOT EXISTS idx_heatmap_log_path ON repo_heatmap_log(file_path);
         CREATE INDEX IF NOT EXISTS idx_heatmap_log_time ON repo_heatmap_log(accessed_at);

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

    // Eleva user_version para 5 se ainda for menor que 5
    let current_version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap_or(0);
    if current_version < 5 {
        conn.pragma_update(None, "user_version", 5)?;
    }

    Ok(())
}
