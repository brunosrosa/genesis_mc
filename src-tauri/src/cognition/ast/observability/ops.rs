//! Operacoes de I/O e migracao da Observabilidade Cognitiva (Marco 3.7).
//!
//! Toda escrita e transacional (`Connection::transaction`) seguindo o
//! padrao do Marco 3.5 (auto-rollback via `Drop` blinda SSD contra
//! row-by-row commit em caso de panico).
//!
//! Migracao idempotente: `migrate_v2_to_v3` pode ser invocada em cold start
//! de qualquer banco v2 ou v3 sem efeito colateral.

use crate::cognition::memory_graph::errors::CognitiveError;
use crate::cognition::memory_graph::fts::{read_user_version, write_user_version};
use crate::cognition::observability::types::{FileAccessLog, TelemetryLog};
use rusqlite::{Connection, params};

/// DDL puro do schema v3 (Observabilidade Sensorial).
///
/// Nao abre conexao — retorna o SQL para o caller executar dentro de uma
/// transacao atomica.
///
/// Tabelas:
/// - `file_access_logs` — append-only. Cada tool que toca filesystem gera
///   um registro. Indice composto `(file_path, accessed_at)` otimiza
///   o scan do heatmap.
/// - `telemetry_logs` — append-only. Cada tool que consome tokens gera
///   um registro. Indice composto `(tool, created_at)` otimiza o
///   agregado FinOps.
pub const V3_SCHEMA_DDL: &str = "
CREATE TABLE IF NOT EXISTS file_access_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    tool TEXT NOT NULL,
    accessed_at INTEGER NOT NULL
) STRICT;

CREATE INDEX IF NOT EXISTS idx_file_access_path_time
    ON file_access_logs(file_path, accessed_at);

CREATE INDEX IF NOT EXISTS idx_file_access_time
    ON file_access_logs(accessed_at);

CREATE TABLE IF NOT EXISTS telemetry_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool TEXT NOT NULL,
    tokens_in INTEGER NOT NULL DEFAULT 0,
    tokens_out INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX IF NOT EXISTS idx_telemetry_tool_time
    ON telemetry_logs(tool, created_at);

CREATE INDEX IF NOT EXISTS idx_telemetry_time
    ON telemetry_logs(created_at);
";

/// Versao do schema apos a Fase B. Idempotente em bancos ja migrados.
pub const TARGET_VERSION: i64 = 3;

/// Marco 3.8 Fase C.1: versao alvo apos Telemetria Real (SOULS State v4).
pub const TARGET_VERSION_V4: i64 = 4;

/// Marco 3.8 Fase C.1: DDL da evolucao v3 -> v4 (Telemetria Real).
///
/// Adiciona a coluna `accuracy_score REAL DEFAULT 1.0` na tabela
/// `telemetry_logs` para alimentar a formula constitucional
/// `E3 = (acc^2) / max(1.0, duration_ms)`. Idempotente via
/// `migrate_v3_to_v4` (que swallow-a o erro "duplicate column" caso
/// um hot-patch parcial anterior tenha materializado a coluna).
pub const V4_SCHEMA_DDL: &str = "
ALTER TABLE telemetry_logs ADD COLUMN accuracy_score REAL NOT NULL DEFAULT 1.0;
";

/// Migra um banco v2 (ou vazio) para v3 de forma atomica.
///
/// Idempotente: se `user_version >= 3`, e no-op.
pub fn migrate_v2_to_v3(conn: &mut Connection) -> Result<(), CognitiveError> {
    let current = read_user_version(conn).map_err(CognitiveError::from)?;
    if current >= TARGET_VERSION {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    tx.execute_batch(V3_SCHEMA_DDL)
        .map_err(CognitiveError::from)?;
    write_user_version(&tx, TARGET_VERSION).map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

/// Marco 3.8 Fase C.1: migra um banco v3 (ou ja v4) para v4 de forma atomica.
///
/// Idempotente:
/// 1. Se `user_version >= 4`, no-op puro.
/// 2. Tenta `ALTER TABLE ADD COLUMN accuracy_score`; swallow do erro
///    "duplicate column" para tolerar bancos onde a coluna foi
///    adicionada por hot-patch parcial anterior (fail-soft).
pub fn migrate_v3_to_v4(conn: &mut Connection) -> Result<(), CognitiveError> {
    let current = read_user_version(conn).map_err(CognitiveError::from)?;
    if current >= TARGET_VERSION_V4 {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    // ALTER TABLE ADD COLUMN nao suporta IF NOT EXISTS; swallow defensivo
    // para hot-patches parciais (a coluna pode ja existir sem o bump de versao).
    if let Err(e) = tx.execute_batch(V4_SCHEMA_DDL) {
        let msg = e.to_string();
        if !msg.contains("duplicate column") && !msg.contains("already exists") {
            return Err(CognitiveError::from(e));
        }
    }
    write_user_version(&tx, TARGET_VERSION_V4).map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

/// Insere um registro de acesso a arquivo via transacao atomica.
///
/// Equivalente a um `INSERT INTO file_access_logs VALUES (...)`.
pub fn insert_file_access(
    conn: &mut Connection,
    log: &FileAccessLog,
) -> Result<(), CognitiveError> {
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    tx.execute(
        "INSERT INTO file_access_logs (file_path, tool, accessed_at) \
         VALUES (?1, ?2, ?3)",
        params![log.file_path, log.tool, log.accessed_at],
    )
    .map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

/// Insere um registro de telemetria FinOps via transacao atomica.
pub fn insert_telemetry(
    conn: &mut Connection,
    log: &TelemetryLog,
) -> Result<(), CognitiveError> {
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    tx.execute(
        "INSERT INTO telemetry_logs \
            (tool, tokens_in, tokens_out, cost_usd, duration_ms, accuracy_score, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            log.tool,
            log.tokens_in,
            log.tokens_out,
            log.cost_usd,
            log.duration_ms,
            log.accuracy_score,
            log.created_at,
        ],
    )
    .map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}
