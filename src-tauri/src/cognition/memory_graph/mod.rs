//! `souls_graph`: grafo relacional cognitivo canibalizado do `memory-mcp-rs`.
//!
//! Persistência: `souls_state.db` (SQLite WAL + FTS5, conforme ADR-040).
//! Concorrência: canal `tokio::sync::mpsc` buffer 100 + worker dedicado
//! (`std::thread::spawn` + `blocking_recv`).
//! API: 9 operações MCP canônicas (`mem_create_entities`, `mem_search`, etc.).

pub mod errors;
pub mod fts;
pub mod mpsc_bridge;
pub mod ops;
pub mod types;

pub use errors::CognitiveError;
pub use mpsc_bridge::{MemGraphOp, spawn_memory_graph_worker};
pub use types::{Entity, ObservationInput, ObservationRecord, Relation, now_epoch_ms};
