//! `souls_thinking`: máquina de estados socrática com disjuntor cognitivo.
//!
//! Canibalização cirúrgica do `ultrafast-mcp-sequential-thinking`.
//! Estado in-RAM (`HashMap<BranchId, Vec<ThoughtId>>`); sem persistência
//! obrigatória (Marco 4 cuidará disso).
//! API: `core_think` (MCP tool), `ThinkingEngine::push_thought` (Rust API).

pub mod errors;
pub mod state_machine;
pub mod types;

pub use errors::{CognitiveError, ThinkingError};
pub use state_machine::{
    DEFAULT_HARD_LIMIT, HITL_EXTENDED_LIMIT, ThinkingEngine,
};
pub use types::{
    BranchId, BranchSummary, ThoughtData, ThoughtId, ThinkingMode, ThinkingResponse,
};
