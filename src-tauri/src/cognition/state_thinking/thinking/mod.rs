//! `souls_thinking`: máquina de estados socrática com disjuntor cognitivo.
//!
//! Canibalização cirúrgica do `ultrafast-mcp-sequential-thinking`.
//! Estado in-RAM (`HashMap<BranchId, Vec<ThoughtId>>`); sem persistência
//! obrigatória (Marco 4 cuidará disso).
//! API: `core_think` (MCP tool), `ThinkingEngine::push_thought` (Rust API).
//!
//! Marco 3.9 Fase E: persistência SQLite V5 (Souls State).
//!
//! - [`persistence`]: tipos canônicos (`SocraticThought`, `ThoughtType`).
//! - [`ops`]: DDL V5 idempotente, INSERT/SELECT/DELETE com FK CASCADE.
//! - [`analytics`]: métricas FinOps cognitivas (revision rate, branching,
//!   latência) — pure functions, sem I/O.
//!
//! Marco 3.9 Fase E.2: barramento assíncrono para gravações socráticas.
//!
//! - [`socratic_bridge`]: canal MPSC bounded(512) + worker dedicado
//!   (`std::thread::spawn` + `blocking_recv`) para extirpar a dependência
//!   de mutex globais síncronos. Hiper-Forward via `try_send` no critical
//!   path do Tokio event loop.

pub mod analytics;
pub mod errors;
pub mod handlers;
pub mod ops;
pub mod persistence;
pub mod socratic_bridge;
pub mod state_machine;
pub mod types;
pub mod worker;

pub mod test_helpers;

pub use analytics::{compute_metrics, SessionMetrics};
pub use errors::{CognitiveError, ThinkingError};
pub use ops::{
    delete_socratic_session, fetch_thought, gen_simple_uuid, list_thoughts_for_session,
    migrate_v3_to_v5, upsert_socratic_session, upsert_socratic_thought, TARGET_VERSION,
    V5_SCHEMA_DDL,
};
pub use persistence::{BranchId, SessionId, SocraticThought, ThoughtId, ThoughtType};
pub use socratic_bridge::{spawn_socratic_write_worker, SocraticOp, SocraticWriteHandle};
pub use state_machine::{
    DEFAULT_HARD_LIMIT, HITL_EXTENDED_LIMIT, ThinkingEngine,
};
pub use types::{
    BranchSummary, ThoughtData, ThinkingMode, ThinkingResponse,
};
pub use worker::{init_state_db_worker, try_send_cold, StateDbOp, SubAgentDiary, STATE_DB_TX};

