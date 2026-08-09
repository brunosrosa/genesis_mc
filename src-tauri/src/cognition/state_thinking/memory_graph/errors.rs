//! Erros tipados do Core Cognitivo (souls_graph + souls_thinking).
//! Lei de Ferro: Fail-Closed L7 — todos os erros abortam a propagação
//! e reportam ao Arquiteto, sem alucinações compensatórias.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CognitiveError {
    #[error("Disjuntor de Overthinking acionado: {actual} pensamentos excedem o teto de {max}")]
    OverthinkingThresholdBreached { actual: u32, max: u32 },

    #[error("Pensamento de revisão sem revises_thought: o campo é obrigatório quando is_revision=true")]
    RevisionWithoutTarget,

    #[error("Branch órfão: branch_id={0} referencia pensamento inexistente na sessão")]
    OrphanBranch(String),

    #[error("Falha no grafo de memória (SQLite): {0}")]
    GraphError(String),

    #[error("Falha no engine socrático: {0}")]
    ThinkingError(String),

    #[error("Payload MCP inválido: {0}")]
    InvalidPayload(String),

    #[error("Operação negada pelo HITL: {0}")]
    HitlDenied(String),

    #[error("Execução não confiável bloqueada: {0}")]
    UntrustedExecutionBlocked(String),
}

impl From<rusqlite::Error> for CognitiveError {
    fn from(e: rusqlite::Error) -> Self {
        CognitiveError::GraphError(e.to_string())
    }
}

impl From<serde_json::Error> for CognitiveError {
    fn from(e: serde_json::Error) -> Self {
        CognitiveError::InvalidPayload(e.to_string())
    }
}
