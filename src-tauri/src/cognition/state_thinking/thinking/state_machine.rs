//! Re-exportação do `ThinkingEngine` (MARCO 4.7.0).
//!
//! A implementação foi transplantada para `engine.rs` para apoiar a
//! integridade socrática, o UUIDv7 e o Roteador Metacognitivo (`ThinkingParadigm`).

pub use crate::cognition::thinking::engine::{
    ThinkingEngine, ThinkingParadigm, DEFAULT_HARD_LIMIT, HITL_EXTENDED_LIMIT,
};
