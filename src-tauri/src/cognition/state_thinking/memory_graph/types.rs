//! Tipos canônicos do `souls_graph` (canibalização do memory-mcp-rs).
//!
//! Convenções SOULS:
//! - Timestamps via `std::time::SystemTime` em milissegundos (UNIX Epoch).
//!   Banido: `chrono` (ADR-005).
//! - `Entity.observations` é SEMPRE hidratado em runtime via JOIN com a tabela
//!   `observations`. Nunca persistido como JSON direto no registro de `entities`.

use serde::{Deserialize, Serialize};

/// Timestamp UNIX Epoch em milissegundos (CPU nativa, sem chrono).
pub type EpochMs = i64;

/// Retorna o timestamp atual em milissegundos desde UNIX Epoch.
pub fn now_epoch_ms() -> EpochMs {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Entidade atômica do grafo de memória (PRD-031 §2.1.1).
///
/// `observations` é populado em runtime via JOIN com a tabela `observations`
/// — o registro persistido em `entities` NÃO contém esse vetor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<String>,
}

/// Relação direcionada entre duas entidades (PRD-031 §2.1.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
}

/// Lote de observações a anexar a uma entidade existente (PRD-031 §2.1.3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservationInput {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub contents: Vec<String>,
}

/// Registro persistido da observação (com `id` e `created_at`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservationRecord {
    pub id: i64,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub content: String,
    #[serde(rename = "createdAt")]
    pub created_at: EpochMs,
}
