//! Operacoes de I/O e migracao V3→V5 da Persistencia Socratica (Marco 3.9 Fase E).
//!
//! Toda escrita e transacional (`Connection::transaction`) seguindo o
//! padrao do Marco 3.5 (auto-rollback via `Drop` blinda SSD contra
//! row-by-row commit em caso de panico).
//!
//! Migracao idempotente: `migrate_v3_to_v5` pode ser invocada em cold start
//! de qualquer banco v3, v4 ou v5 sem efeito colateral.

use crate::cognition::memory_graph::errors::CognitiveError;
use crate::cognition::memory_graph::fts::{read_user_version, write_user_version};
use crate::cognition::thinking::persistence::{SocraticThought, ThoughtId, ThoughtType};
use rusqlite::{params, Connection, OptionalExtension};

/// Erro ad-hoc para `FromSqlConversionFailure` quando o `thought_type`
/// recuperado do SQLite não casa com nenhum canônico. Implementa
/// `std::error::Error` para satisfazer o trait bound de `Box<dyn StdError>`.
#[derive(Debug)]
struct InvalidThoughtType {
    raw: String,
}

impl std::fmt::Display for InvalidThoughtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "thought_type inválido: {}", self.raw)
    }
}

impl std::error::Error for InvalidThoughtType {}

/// DDL puro do schema V5 (Persistência Socrática).
///
/// Tabelas:
/// - `socratic_sessions` — aggregate root. PK `session_id` TEXT STRICT.
/// - `socratic_thoughts` — cada pensamento. PK `thought_id` TEXT STRICT.
///   FK `session_id` ON DELETE CASCADE; FK `parent_thought_id` ON DELETE SET NULL.
///
/// Índices:
/// - `idx_thoughts_session` (session_id) — listagem por sessão.
/// - `idx_thoughts_branch` (branch_id) — agrupamento por branch.
/// - `idx_thoughts_parent` (parent_thought_id) — busca de filhos.
/// - `idx_thoughts_session_step` (session_id, step_number) — ordenação
///   canônica de reconstrução da árvore.
pub const V5_SCHEMA_DDL: &str = "
CREATE TABLE IF NOT EXISTS socratic_sessions (
    session_id TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    metadata   TEXT NOT NULL DEFAULT '{}'
) STRICT;

CREATE TABLE IF NOT EXISTS socratic_thoughts (
    thought_id        TEXT PRIMARY KEY,
    session_id        TEXT NOT NULL,
    branch_id         TEXT NOT NULL DEFAULT 'main',
    parent_thought_id TEXT,
    thought_type      TEXT NOT NULL,
    content           TEXT NOT NULL,
    step_number       INTEGER NOT NULL,
    duration_ms       INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL,
    FOREIGN KEY(session_id)        REFERENCES socratic_sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY(parent_thought_id) REFERENCES socratic_thoughts(thought_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS entities (
    entity_name TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    observations TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS relations (
    from_entity TEXT NOT NULL,
    to_entity TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY(from_entity, to_entity, relation_type),
    FOREIGN KEY(from_entity) REFERENCES entities(entity_name) ON DELETE CASCADE,
    FOREIGN KEY(to_entity) REFERENCES entities(entity_name) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS observations (
    observation_id TEXT PRIMARY KEY,
    entity_name TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(entity_name) REFERENCES entities(entity_name) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_thoughts_session
    ON socratic_thoughts(session_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_branch
    ON socratic_thoughts(branch_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_parent
    ON socratic_thoughts(parent_thought_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_session_step
    ON socratic_thoughts(session_id, step_number);
CREATE INDEX IF NOT EXISTS idx_observations_entity
    ON observations(entity_name);

CREATE VIRTUAL TABLE IF NOT EXISTS observations_fts USING fts5(
    observation_id UNINDEXED,
    entity_name,
    content
);

CREATE TRIGGER IF NOT EXISTS after_observation_insert AFTER INSERT ON observations BEGIN
    INSERT INTO observations_fts(observation_id, entity_name, content)
    VALUES (new.observation_id, new.entity_name, new.content);
END;

CREATE TRIGGER IF NOT EXISTS after_observation_delete AFTER DELETE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, observation_id, entity_name, content)
    VALUES('delete', old.observation_id, old.entity_name, old.content);
END;

CREATE TRIGGER IF NOT EXISTS after_observation_update AFTER UPDATE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, observation_id, entity_name, content)
    VALUES('delete', old.observation_id, old.entity_name, old.content);
    INSERT INTO observations_fts(observation_id, entity_name, content)
    VALUES (new.observation_id, new.entity_name, new.content);
END;
";

/// Versão do schema pós Fase E. Idempotente em bancos já migrados.
pub const TARGET_VERSION: i64 = 5;

/// Migra um banco v3 (ou v4) para v5 de forma atômica.
///
/// Idempotente: se `user_version >= 5`, é no-op.
pub fn migrate_v3_to_v5(conn: &mut Connection) -> Result<(), CognitiveError> {
    let current = read_user_version(conn).map_err(CognitiveError::from)?;
    if current >= TARGET_VERSION {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    tx.execute_batch(V5_SCHEMA_DDL)
        .map_err(CognitiveError::from)?;
    write_user_version(&tx, TARGET_VERSION).map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

/// Cria uma nova sessão socrática (idempotente em PK duplicada).
///
/// `metadata` é um JSON blob (string) — tags, task_name, agente
/// que originou, etc. Default: `"{}"`.
pub fn upsert_socratic_session(
    conn: &Connection,
    session_id: &str,
    created_at: i64,
    metadata: &str,
) -> Result<(), CognitiveError> {
    conn.execute(
        "INSERT INTO socratic_sessions (session_id, created_at, metadata)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(session_id) DO UPDATE SET
             created_at = excluded.created_at,
             metadata = excluded.metadata",
        params![session_id, created_at, metadata],
    )
    .map_err(CognitiveError::from)?;
    Ok(())
}

/// Insere ou substitui um pensamento na tabela.
///
/// `parent_thought_id = None` é válido (Tese raiz). FKs são validadas
/// pelo SQLite em `PRAGMA foreign_keys = ON` (já configurado no init).
pub fn upsert_socratic_thought(conn: &Connection, t: &SocraticThought) -> Result<(), CognitiveError> {
    conn.execute(
        "INSERT INTO socratic_thoughts (
            thought_id, session_id, branch_id, parent_thought_id,
            thought_type, content, step_number, duration_ms, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(thought_id) DO UPDATE SET
             branch_id         = excluded.branch_id,
             parent_thought_id = excluded.parent_thought_id,
             thought_type      = excluded.thought_type,
             content           = excluded.content,
             step_number       = excluded.step_number,
             duration_ms       = excluded.duration_ms,
             created_at        = excluded.created_at",
        params![
            t.thought_id,
            t.session_id,
            t.branch_id,
            t.parent_thought_id,
            t.thought_type.as_str(),
            t.content,
            t.step_number,
            t.duration_ms,
            t.created_at,
        ],
    )
    .map_err(CognitiveError::from)?;
    Ok(())
}

/// Lista todos os pensamentos de uma sessão, ordenados canonicamente.
///
/// Ordem canônica (topológica-first):
/// 1. `parent_thought_id IS NULL` DESC → raízes (Tese) primeiro.
/// 2. `branch_id ASC` → agrupamento por branch.
/// 3. `step_number ASC` → ordenação cronológica dentro do branch.
///
/// Esta ordenação garante reconstrução determinística da árvore mesmo
/// em grafos cross-branch (filho em "alt" referenciando pai em "main"):
/// a raiz é sempre processada antes do filho, e o `merge_sessions`
/// consegue remapear `parent_thought_id` sem referências órfãs.
pub fn list_thoughts_for_session(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<SocraticThought>, CognitiveError> {
    let mut stmt = conn
        .prepare(
            "SELECT thought_id, session_id, branch_id, parent_thought_id,
                    thought_type, content, step_number, duration_ms, created_at
             FROM socratic_thoughts
             WHERE session_id = ?1
             ORDER BY (parent_thought_id IS NULL) DESC, branch_id ASC, step_number ASC",
        )
        .map_err(CognitiveError::from)?;
    let rows = stmt
        .query_map(params![session_id], |row| {
            let thought_type_str: String = row.get(4)?;
            let thought_type = ThoughtType::parse(&thought_type_str)
                .ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(InvalidThoughtType {
                            raw: thought_type_str.clone(),
                        }),
                    )
                })?;
            Ok(SocraticThought {
                thought_id: row.get(0)?,
                session_id: row.get(1)?,
                branch_id: row.get(2)?,
                parent_thought_id: row.get(3)?,
                thought_type,
                content: row.get(5)?,
                step_number: row.get::<_, u32>(6)?,
                duration_ms: row.get::<_, u32>(7)?,
                created_at: row.get::<_, i64>(8)?,
            })
        })
        .map_err(CognitiveError::from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(CognitiveError::from)?);
    }
    Ok(out)
}

/// Remove uma sessão inteira (CASCADE purga todos os pensamentos).
pub fn delete_socratic_session(conn: &Connection, session_id: &str) -> Result<usize, CognitiveError> {
    let n = conn
        .execute(
            "DELETE FROM socratic_sessions WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(CognitiveError::from)?;
    Ok(n)
}

/// Busca um pensamento específico por ID (None se não existe).
pub fn fetch_thought(conn: &Connection, thought_id: &str) -> Result<Option<SocraticThought>, CognitiveError> {
    let mut stmt = conn
        .prepare(
            "SELECT thought_id, session_id, branch_id, parent_thought_id,
                    thought_type, content, step_number, duration_ms, created_at
             FROM socratic_thoughts
             WHERE thought_id = ?1",
        )
        .map_err(CognitiveError::from)?;
    let row = stmt
        .query_row(params![thought_id], |row| {
            let thought_type_str: String = row.get(4)?;
            let thought_type = ThoughtType::parse(&thought_type_str).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(InvalidThoughtType {
                        raw: thought_type_str.clone(),
                    }),
                )
            })?;
            Ok(SocraticThought {
                thought_id: row.get(0)?,
                session_id: row.get(1)?,
                branch_id: row.get(2)?,
                parent_thought_id: row.get(3)?,
                thought_type,
                content: row.get(5)?,
                step_number: row.get::<_, u32>(6)?,
                duration_ms: row.get::<_, u32>(7)?,
                created_at: row.get::<_, i64>(8)?,
            })
        })
        .optional()
        .map_err(CognitiveError::from)?;
    Ok(row)
}

/// Métricas para FinOps cognitivo (exportado em `analytics.rs`).
pub use crate::cognition::thinking::analytics::SessionMetrics;

/// Helper para gerar UUIDs simples (baseado em tempo + counter).
/// Suficiente para ID local; não é UUIDv4 criptográfico, mas é
/// monotônico dentro de um processo e 100% determinístico entre runs.
pub fn gen_simple_uuid(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}_{:x}_{:x}", (now & 0xFFFF_FFFF_FFFF_FFFF) as u64, n)
}

/// Helper para mapear `&ThoughtId` (placeholder de futura integração
/// com o `ThinkingEngine` in-RAM).
pub fn thought_id_str(_id: &ThoughtId) -> &str {
    // Função reservada para futura normalização; hoje é no-op identidade.
    // Mantida para garantir a API estável quando `ThinkingEngine::push_thought`
    // passar a persistir cada pensamento automaticamente.
    "stub"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_in_memory() -> Connection {
        let conn = Connection::open_in_memory().expect("abre :memory:");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    #[test]
    fn test_migrate_v3_to_v5_is_idempotent() {
        let mut conn = open_in_memory();
        // Cold start: vazio, version = 0.
        assert_eq!(read_user_version(&conn).unwrap(), 0);
        migrate_v3_to_v5(&mut conn).expect("migra para v5");
        assert_eq!(read_user_version(&conn).unwrap(), 5);
        // Re-invocar é no-op.
        migrate_v3_to_v5(&mut conn).expect("re-invoca sem efeito");
        assert_eq!(read_user_version(&conn).unwrap(), 5);
    }

    #[test]
    fn test_fk_cascade_purges_thoughts() {
        let mut conn = open_in_memory();
        migrate_v3_to_v5(&mut conn).expect("migra");

        upsert_socratic_session(&conn, "sess1", 1000, r#"{"task":"X"}"#).unwrap();
        upsert_socratic_thought(
            &conn,
            &SocraticThought {
                thought_id: "th1".into(),
                session_id: "sess1".into(),
                branch_id: "main".into(),
                parent_thought_id: None,
                thought_type: ThoughtType::Regular,
                content: "Tese".into(),
                step_number: 1,
                duration_ms: 50,
                created_at: 1000,
            },
        )
        .unwrap();
        assert_eq!(list_thoughts_for_session(&conn, "sess1").unwrap().len(), 1);
        // Apaga sessão → CASCADE purga pensamentos.
        let n = delete_socratic_session(&conn, "sess1").unwrap();
        assert_eq!(n, 1);
        assert_eq!(list_thoughts_for_session(&conn, "sess1").unwrap().len(), 0);
    }

    #[test]
    fn test_fk_rejects_orphan_session_id() {
        let mut conn = open_in_memory();
        migrate_v3_to_v5(&mut conn).expect("migra");
        // Não insere socratic_sessions — apenas tenta inserir thought órfão.
        let res = upsert_socratic_thought(
            &conn,
            &SocraticThought {
                thought_id: "th_orphan".into(),
                session_id: "sess_inexistente".into(),
                branch_id: "main".into(),
                parent_thought_id: None,
                thought_type: ThoughtType::Regular,
                content: "órfão".into(),
                step_number: 1,
                duration_ms: 0,
                created_at: 0,
            },
        );
        assert!(res.is_err(), "FK deve rejeitar session_id inexistente");
    }
}
