//! Tipos canônicos do `souls_thinking` (canibalização do `ultrafast-mcp-sequential-thinking`).
//!
//! Estrutura JSON canônica do `ThoughtData` conforme PRD-032 §3.1.

use serde::{Deserialize, Serialize};

/// Identificador único de uma thread de raciocínio dentro da sessão.
pub type BranchId = String;

/// Identificador de um pensamento (1-based, imutável dentro da sessão).
pub type ThoughtId = u32;

/// Modo cognitivo do pensamento (Tríade: Regular | Revision | Branching).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingMode {
    /// Pensamento regular: decomposição / proposta.
    Regular,
    /// Pensamento de revisão: avalia retroativamente `revises_thought`.
    Revision,
    /// Pensamento de ramificação: explora alternativa a partir de `branch_from_thought`.
    Branching,
}

/// Payload canônico do `core_think` (PRD-032 §3.1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThoughtData {
    pub thought: String,
    pub thought_number: ThoughtId,
    pub total_thoughts: u32,
    pub next_thought_needed: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_revision: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revises_thought: Option<ThoughtId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_from_thought: Option<ThoughtId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<BranchId>,
    #[serde(default, skip_serializing_if = "is_false_opt")]
    pub needs_more_thoughts: Option<bool>,
    /// Flag HITL server-side: libera o teto de 5 → 7 pensamentos.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hitl_authorized: Option<bool>,
}

fn is_false(b: &Option<bool>) -> bool {
    matches!(b, Some(false) | None)
}

fn is_false_opt(b: &Option<bool>) -> bool {
    matches!(b, Some(false) | None)
}

impl ThoughtData {
    /// Resolve o modo cognitivo a partir dos flags.
    pub fn mode(&self) -> ThinkingMode {
        if self.is_revision.unwrap_or(false) {
            ThinkingMode::Revision
        } else if self.branch_id.is_some() || self.branch_from_thought.is_some() {
            ThinkingMode::Branching
        } else {
            ThinkingMode::Regular
        }
    }
}

/// Resposta canônica do `core_think` para o cliente MCP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThinkingResponse {
    #[serde(rename = "thoughtNumber")]
    pub thought_number: ThoughtId,
    #[serde(rename = "totalThoughts")]
    pub total_thoughts: u32,
    #[serde(rename = "nextThoughtNeeded")]
    pub next_thought_needed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<BranchSummary>,
    pub mode: ThinkingMode,
}

/// Resumo de uma ramificação ativa (debug/telemetria).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchSummary {
    #[serde(rename = "branchId")]
    pub branch_id: BranchId,
    #[serde(rename = "thoughtCount")]
    pub thought_count: usize,
}
