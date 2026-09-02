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

    // SOULS V6 MARCO 5.16.0: SDD Cascade Orchestrator (sdd.rs)
    // Emite quando a divergência de hash SHA-256 em REQUIREMENTS.md
    // dispara a invalidação atômica dos documentos downstream.
    #[error("Violação da cascata documental SDD: {0} documento(s) downstream rebaixado(s)")]
    SddCascadeViolation(usize),
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

// SOULS V6 MARCO 5.16.0: canibalização dos mapeamentos de I/O do FS
// e de regex para o catálogo canônico de CognitiveError. Permite que o
// `?` operator funcione transparente em todo o módulo sdd.rs.
impl From<std::io::Error> for CognitiveError {
    fn from(e: std::io::Error) -> Self {
        CognitiveError::GraphError(format!("I/O do FS: {e}"))
    }
}

impl From<regex::Error> for CognitiveError {
    fn from(e: regex::Error) -> Self {
        CognitiveError::InvalidPayload(format!("regex interna: {e}"))
    }
}
