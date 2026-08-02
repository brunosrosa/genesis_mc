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
            (tool, tokens_in, tokens_out, cost_usd, duration_ms, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            log.tool,
            log.tokens_in,
            log.tokens_out,
            log.cost_usd,
            log.duration_ms,
            log.created_at,
        ],
    )
    .map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}
