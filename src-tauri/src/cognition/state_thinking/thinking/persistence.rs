//! Tipos canônicos de persistência socrática (Marco 3.9 Fase E).
//!
//! Complementa `thinking::types` com os DTOs que o SQLite precisa
//! entender. A identidade dos pensamentos passa de `u32` (in-RAM,
//! 1-based sequencial) para `String` (UUIDv4, globalmente único para
//! suportar `merge_sessions` entre sessões distintas).

use serde::{Deserialize, Serialize};

/// Identificador único de uma sessão socrática (UUIDv4 simples).
pub type SessionId = String;

/// Identificador único de um pensamento dentro de uma sessão
/// (UUIDv4 simples). Garante colisão zero no `merge_sessions` entre
/// sessões distintas.
pub type ThoughtId = String;

/// Identificador de ramificação (string curta, e.g. `"main"`, `"branch-A"`).
pub type BranchId = String;

/// Modo cognitivo do pensamento. Reflete a Tríade do PRD-032 §3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThoughtType {
    /// Pensamento regular: decomposição / proposta.
    Regular,
    /// Pensamento de revisão: avalia retroativamente outro pensamento.
    Revision,
    /// Pensamento de ramificação: explora alternativa a partir de outro.
    Branching,
}

impl ThoughtType {
    /// Serializa para string canônica (lowercase, snake_case) para SQLite.
    pub fn as_str(self) -> &'static str {
        match self {
            ThoughtType::Regular => "regular",
            ThoughtType::Revision => "revision",
            ThoughtType::Branching => "branching",
        }
    }

    /// Parseia de string canônica. Retorna `None` se a string for
    /// desconhecida (fail-closed: nunca infere modo).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "regular" => Some(ThoughtType::Regular),
            "revision" => Some(ThoughtType::Revision),
            "branching" => Some(ThoughtType::Branching),
            _ => None,
        }
    }
}

/// Pensamento socrático persistido no SQLite (V5).
///
/// Réplica 1-pra-1 da tabela `socratic_thoughts` (ver `ADR-045`).
/// `parent_thought_id` é `None` para a Tese raiz; `Some(uuid)` para
/// filhos, netos, etc. A reconstrução da árvore em RAM é responsabilidade
/// de [`super::ops::list_thoughts_for_session`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocraticThought {
    /// UUIDv4 único global.
    pub thought_id: ThoughtId,
    /// FK para `socratic_sessions.session_id`.
    pub session_id: SessionId,
    /// Branch à qual o pensamento pertence (default `"main"`).
    pub branch_id: BranchId,
    /// FK opcional para o pensamento pai (None = raiz).
    pub parent_thought_id: Option<ThoughtId>,
    /// Tipo cognitivo (regular, revision, branching).
    pub thought_type: ThoughtType,
    /// Conteúdo textual livre.
    pub content: String,
    /// Posição sequencial dentro do branch (1-based, monotonicamente crescente).
    pub step_number: u32,
    /// Latência de geração do pensamento (ms). Usada por `analyze_session`.
    pub duration_ms: u32,
    /// Epoch seconds (Unix timestamp).
    pub created_at: i64,
}

/// Trait de escrita assíncrona socrática (Marco 3.9 / Marco VI).
/// Envia dados de pensamento e sessão de forma não-bloqueante via MPSC sem travar a interface de chat.
pub trait SocraticPersist: Send + Sync {
    /// Persiste um pensamento de forma assíncrona/não-bloqueante.
    fn persist_thought(&self, thought: SocraticThought) -> Result<(), String>;
    /// Persiste uma sessão de forma assíncrona/não-bloqueante.
    fn persist_session(&self, session_id: &str, metadata: &str) -> Result<(), String>;
}
