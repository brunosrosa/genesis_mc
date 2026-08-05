//! Erros do engine socrático `souls_thinking`.
//!
//! Re-exporta `CognitiveError` (já cobre o disjuntor e validação de branching)
//! e adiciona `ThinkingError` para erros de payload/orquestração MCP.

use thiserror::Error;

pub use crate::cognition::memory_graph::errors::CognitiveError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ThinkingError {
    #[error("Falha de serialização do payload: {0}")]
    PayloadSerialization(String),
    #[error("Operação socrática negada: {0}")]
    OperationDenied(String),
}
