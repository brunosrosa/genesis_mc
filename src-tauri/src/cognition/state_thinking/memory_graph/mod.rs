//! `souls_graph`: grafo relacional cognitivo canibalizado do `memory-mcp-rs`.
//!
//! Persistência: `souls_state.db` (SQLite WAL + FTS5, conforme ADR-040).
//! Concorrência: canal `tokio::sync::mpsc` buffer 100 + worker dedicado (`StateDbWorker`).
//! API: operações de grafo canônicas (`mem_create_entities`, `mem_search`, `mem_open_nodes`, etc.).

pub mod errors;
pub mod fts;
pub mod mpsc_bridge;
pub mod ops;
pub mod types;
pub mod uuid;
pub mod vector_store;

pub use errors::CognitiveError;
pub use mpsc_bridge::{MemGraphOp, spawn_memory_graph_worker};
pub use types::{Entity, ObservationInput, ObservationRecord, Relation, now_epoch_ms};
pub use uuid::generate_uuid_v7;
pub use vector_store::{HybridSearchResult, RrfDocumentInput, reciprocal_rank_fusion, RRF_K};


use crate::cognition::state_thinking::thinking::worker::{StateDbOp, STATE_DB_TX};
use tokio::sync::oneshot;

/// `search_graph`: busca FTS5 síncrona por `MATCH` em `observations_fts`.
pub fn search_graph(
    conn: &rusqlite::Connection,
    query: &str,
) -> Result<Vec<ObservationRecord>, String> {
    ops::search_observations(conn, query, 50).map_err(|e| e.to_string())
}

/// `open_nodes`: abre observações ligadas a uma lista de entidades conhecidas.
pub fn open_nodes(
    conn: &rusqlite::Connection,
    names: &[String],
) -> Result<Vec<ObservationRecord>, String> {
    ops::open_nodes(conn, names).map_err(|e| e.to_string())
}

/// Dispacha `CreateEntity` para o barramento MPSC `STATE_DB_TX`.
pub async fn create_entity(name: String, entity_type: String) -> Result<(), String> {
    let tx = STATE_DB_TX.get().ok_or_else(|| "STATE_DB_TX não inicializado".to_string())?;
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(StateDbOp::CreateEntity {
        name,
        entity_type,
        reply: reply_tx,
    })
    .await
    .map_err(|e| e.to_string())?;
    reply_rx.await.map_err(|e| e.to_string())?
}

/// Dispacha `CreateRelation` para o barramento MPSC `STATE_DB_TX`.
pub async fn create_relation(from_entity: String, to_entity: String, relation_type: String) -> Result<(), String> {
    let tx = STATE_DB_TX.get().ok_or_else(|| "STATE_DB_TX não inicializado".to_string())?;
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(StateDbOp::CreateRelation {
        from_entity,
        to_entity,
        relation_type,
        reply: reply_tx,
    })
    .await
    .map_err(|e| e.to_string())?;
    reply_rx.await.map_err(|e| e.to_string())?
}

/// Dispacha `AddObservation` com ID UUIDv7 para o barramento MPSC `STATE_DB_TX`.
pub async fn add_observation(entity_name: String, content: String) -> Result<String, String> {
    let tx = STATE_DB_TX.get().ok_or_else(|| "STATE_DB_TX não inicializado".to_string())?;
    let (reply_tx, reply_rx) = oneshot::channel();
    let obs_id = generate_uuid_v7();
    tx.send(StateDbOp::AddObservation {
        observation_id: obs_id.clone(),
        entity_name,
        content,
        reply: reply_tx,
    })
    .await
    .map_err(|e| e.to_string())?;
    reply_rx.await.map_err(|e| e.to_string())??;
    Ok(obs_id)
}
