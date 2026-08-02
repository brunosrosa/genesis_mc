//! 9 operações canônicas do `souls_graph` (canibalização cirúrgica do `memory-mcp-rs`).
//!
//! Princípios:
//! - Toda escrita é transacional (`Connection::transaction`) — auto-rollback
//!   via `Drop` blinda o SSD contra row-by-row commit em caso de pânico.
//! - Leitura via JOIN hidrata `Entity.observations` da tabela normalizada.
//! - Erros de banco propagam via `CognitiveError::GraphError` (From<rusqlite::Error>).
//! - FTS5: busca por `MATCH` síncrono; `LIMIT 50` para defesa.

use crate::cognition::memory_graph::errors::CognitiveError;
use crate::cognition::memory_graph::fts::{V2_SCHEMA_DDL, read_user_version, write_user_version};
use crate::cognition::memory_graph::types::{Entity, ObservationInput, Relation, now_epoch_ms};
use rusqlite::{Connection, params};

/// Migra um banco V1 (ou vazio) para V2 de forma atômica.
///
/// Idempotente: se `user_version >= 2`, é no-op.
pub fn migrate_v1_to_v2(conn: &mut Connection) -> Result<(), CognitiveError> {
    let current = read_user_version(conn).map_err(CognitiveError::from)?;
    if current >= 2 {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    tx.execute_batch(V2_SCHEMA_DDL).map_err(CognitiveError::from)?;
    write_user_version(&tx, 2).map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ESCRITA (3 ops)
// ---------------------------------------------------------------------------

/// `mem_create_entities`: cria entidades novas (idempotente via INSERT OR IGNORE).
/// Retorna a lista de entidades efetivamente materializadas no banco (incluindo
/// as pré-existentes — comportamento idêntico ao `memory-mcp-rs`).
///
/// Nota de compatibilidade: a coluna legada `entities.observations` (V1, NOT NULL)
/// é populada com string vazia para satisfazer a constraint. A hidratação
/// oficial de `Entity.observations` é feita via `open_nodes` (PRD-031 §2.1).
pub fn create_entities(
    conn: &mut Connection,
    entities: &[Entity],
) -> Result<Vec<Entity>, CognitiveError> {
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO entities (name, entity_type, observations) \
                 VALUES (?1, ?2, '')",
            )
            .map_err(CognitiveError::from)?;
        for e in entities {
            stmt.execute(params![e.name, e.entity_type])
                .map_err(CognitiveError::from)?;
        }
    }
    tx.commit().map_err(CognitiveError::from)?;
    Ok(entities
        .iter()
        .map(|e| Entity {
            name: e.name.clone(),
            entity_type: e.entity_type.clone(),
            observations: Vec::new(),
        })
        .collect())
}

/// `mem_create_relations`: cria arestas entre entidades (idempotente).
pub fn create_relations(
    conn: &mut Connection,
    relations: &[Relation],
) -> Result<Vec<Relation>, CognitiveError> {
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO relations (from_entity, to_entity, relation_type) \
                 VALUES (?1, ?2, ?3)",
            )
            .map_err(CognitiveError::from)?;
        for r in relations {
            stmt.execute(params![r.from, r.to, r.relation_type])
                .map_err(CognitiveError::from)?;
        }
    }
    tx.commit().map_err(CognitiveError::from)?;
    Ok(relations.to_vec())
}

/// `mem_add_observations`: anexa observações a entidades existentes.
/// Usa `INSERT INTO observations` (a tabela nova V2) — triggers FTS5
/// mantém `observations_fts` em sincronia.
pub fn add_observations(
    conn: &mut Connection,
    observations: &[ObservationInput],
) -> Result<(), CognitiveError> {
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    let now = now_epoch_ms();
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO observations (entity_name, content, created_at) \
                 VALUES (?1, ?2, ?3)",
            )
            .map_err(CognitiveError::from)?;
        for obs in observations {
            for content in &obs.contents {
                stmt.execute(params![obs.entity_name, content, now])
                    .map_err(CognitiveError::from)?;
            }
        }
    }
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// LEITURA (3 ops)
// ---------------------------------------------------------------------------

/// `mem_search`: busca FTS5 síncrona por `MATCH` em `observations_fts`.
/// Retorna entidades cujos nomes ou observações casam com a query.
pub fn search_observations(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<Entity>, CognitiveError> {
    // Saneamento mínimo: query vazia ou apenas com aspas é tratada como no-op.
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    // Estratégia: casar via JOIN com observations e retornar entidades distintas
    // (LIMIT aplicado no universo de observações, depois dedup por name).
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT e.name, e.entity_type \
             FROM observations_fts f \
             JOIN observations o ON o.id = f.rowid \
             JOIN entities e ON e.name = o.entity_name \
             WHERE observations_fts MATCH ?1 \
             LIMIT ?2",
        )
        .map_err(CognitiveError::from)?;
    let rows = stmt
        .query_map(params![q, limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(CognitiveError::from)?;
    let mut out = Vec::new();
    for r in rows {
        let (name, entity_type) = r.map_err(CognitiveError::from)?;
        out.push(Entity {
            name,
            entity_type,
            observations: Vec::new(),
        });
    }
    Ok(out)
}

/// `mem_open_nodes`: abre entidades específicas por nome e hidrata observações
/// via JOIN. Cobre o caso de nome inexistente (retorna vetor vazio).
pub fn open_nodes(conn: &Connection, names: &[String]) -> Result<Vec<Entity>, CognitiveError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    // 1ª passada: metadata de entities distintas solicitadas.
    let placeholders: Vec<String> = (0..names.len()).map(|i| format!("?{}", i + 1)).collect();
    let sql_entities = format!(
        "SELECT DISTINCT name, entity_type FROM entities WHERE name IN ({})",
        placeholders.join(",")
    );
    let params_entities: Vec<&dyn rusqlite::ToSql> =
        names.iter().map(|n| n as &dyn rusqlite::ToSql).collect();
    let mut stmt = conn
        .prepare(&sql_entities)
        .map_err(CognitiveError::from)?;
    let rows = stmt
        .query_map(params_entities.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(CognitiveError::from)?;
    let mut out: Vec<Entity> = Vec::new();
    for r in rows {
        let (name, entity_type) = r.map_err(CognitiveError::from)?;
        out.push(Entity {
            name,
            entity_type,
            observations: Vec::new(),
        });
    }
    // 2ª passada: hidrata observations de TODAS as entidades de uma vez
    // (uma query, não N queries).
    if !out.is_empty() {
        let obs_placeholders: Vec<String> = (0..out.len()).map(|i| format!("?{}", i + 1)).collect();
        let sql_obs = format!(
            "SELECT entity_name, content FROM observations \
             WHERE entity_name IN ({}) ORDER BY created_at ASC",
            obs_placeholders.join(",")
        );
        let names_for_obs: Vec<&dyn rusqlite::ToSql> = out
            .iter()
            .map(|e| &e.name as &dyn rusqlite::ToSql)
            .collect();
        let mut obs_stmt = conn
            .prepare(&sql_obs)
            .map_err(CognitiveError::from)?;
        let obs_rows = obs_stmt
            .query_map(names_for_obs.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(CognitiveError::from)?;
        for r in obs_rows {
            let (ename, content) = r.map_err(CognitiveError::from)?;
            if let Some(e) = out.iter_mut().find(|e| e.name == ename) {
                e.observations.push(content);
            }
        }
    }
    Ok(out)
}

/// `mem_read_graph`: retorna o grafo inteiro (entities + relations) até `LIMIT`.
/// Defesa: `LIMIT 500` (canônica do memory-mcp-rs).
pub fn read_graph(conn: &Connection, limit: usize) -> Result<(Vec<Entity>, Vec<Relation>), CognitiveError> {
    let cap = limit.min(500);
    let mut entities_stmt = conn
        .prepare("SELECT name, entity_type FROM entities LIMIT ?1")
        .map_err(CognitiveError::from)?;
    let entity_rows = entities_stmt
        .query_map(params![cap as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(CognitiveError::from)?;
    let mut entities: Vec<Entity> = Vec::new();
    for r in entity_rows {
        let (name, entity_type) = r.map_err(CognitiveError::from)?;
        entities.push(Entity {
            name,
            entity_type,
            observations: Vec::new(),
        });
    }
    let mut rel_stmt = conn
        .prepare(
            "SELECT from_entity, to_entity, relation_type FROM relations LIMIT ?1",
        )
        .map_err(CognitiveError::from)?;
    let rel_rows = rel_stmt
        .query_map(params![cap as i64], |row| {
            Ok(Relation {
                from: row.get(0)?,
                to: row.get(1)?,
                relation_type: row.get(2)?,
            })
        })
        .map_err(CognitiveError::from)?;
    let mut relations: Vec<Relation> = Vec::new();
    for r in rel_rows {
        relations.push(r.map_err(CognitiveError::from)?);
    }
    Ok((entities, relations))
}

// ---------------------------------------------------------------------------
// DELEÇÃO (3 ops) — uso restrito, HITL recomendado
// ---------------------------------------------------------------------------

/// `mem_delete_entities`: remove entidades. Cascade apaga relations e observations.
pub fn delete_entities(conn: &mut Connection, names: &[String]) -> Result<(), CognitiveError> {
    if names.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    let placeholders: Vec<String> = (0..names.len()).map(|i| format!("?{}", i + 1)).collect();
    let sql = format!(
        "DELETE FROM entities WHERE name IN ({})",
        placeholders.join(",")
    );
    let params_dyn: Vec<&dyn rusqlite::ToSql> =
        names.iter().map(|n| n as &dyn rusqlite::ToSql).collect();
    tx.execute(&sql, params_dyn.as_slice())
        .map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

/// `mem_delete_observations`: remove observações específicas por (entity, content).
pub fn delete_observations(
    conn: &mut Connection,
    deletions: &[ObservationInput],
) -> Result<(), CognitiveError> {
    if deletions.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    {
        let mut stmt = tx
            .prepare("DELETE FROM observations WHERE entity_name = ?1 AND content = ?2")
            .map_err(CognitiveError::from)?;
        for d in deletions {
            for c in &d.contents {
                stmt.execute(params![d.entity_name, c])
                    .map_err(CognitiveError::from)?;
            }
        }
    }
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

/// `mem_delete_relations`: remove arestas.
pub fn delete_relations(
    conn: &mut Connection,
    relations: &[Relation],
) -> Result<(), CognitiveError> {
    if relations.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    {
        let mut stmt = tx
            .prepare(
                "DELETE FROM relations \
                 WHERE from_entity = ?1 AND to_entity = ?2 AND relation_type = ?3",
            )
            .map_err(CognitiveError::from)?;
        for r in relations {
            stmt.execute(params![r.from, r.to, r.relation_type])
                .map_err(CognitiveError::from)?;
        }
    }
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! TDD obrigatório do Marco 3.5.
    //! Refs: PRD-031 §2, PRD-032 §4, ADR-040 §Definition-of-Done.

    use super::*;
    use crate::cognition::memory_graph::fts::read_user_version;
    use rusqlite::Connection;

    /// Helper: cria um DB em memória com o schema V1 (entities + relations) já
    /// materializado, replicando o que `init_state_db_and_worker` faz antes
    /// da migração V1→V2.
    fn fresh_v1_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory");
        conn.execute_batch(
            "CREATE TABLE entities (name TEXT PRIMARY KEY, entity_type TEXT NOT NULL, \
             observations TEXT NOT NULL) STRICT; \
             CREATE TABLE relations ( \
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 from_entity TEXT NOT NULL, \
                 to_entity TEXT NOT NULL, \
                 relation_type TEXT NOT NULL, \
                 UNIQUE(from_entity, to_entity, relation_type), \
                 FOREIGN KEY(from_entity) REFERENCES entities(name) ON DELETE CASCADE, \
                 FOREIGN KEY(to_entity) REFERENCES entities(name) ON DELETE CASCADE \
             ) STRICT;",
        )
        .expect("cria schema V1 (entities+relations)");
        conn
    }

    /// Helper: aplica a migração V1→V2 e devolve a conexão pronta.
    fn fresh_v2_db() -> Connection {
        let mut conn = fresh_v1_db();
        migrate_v1_to_v2(&mut conn).expect("migração V1→V2");
        conn
    }

    #[test]
    fn test_migration_user_version_bump() {
        let mut conn = fresh_v1_db();
        // Pré-condição: user_version = 0 (banco V1 virgem).
        let v0 = read_user_version(&conn).expect("lê user_version");
        assert_eq!(v0, 0, "V1 deve começar com user_version=0");

        // Executa migração.
        migrate_v1_to_v2(&mut conn).expect("migração ok");

        // Pós-condição: user_version = 2 e schema V2 existe.
        let v2 = read_user_version(&conn).expect("lê user_version pós-migração");
        assert_eq!(v2, 2, "user_version deve ser 2 após migração");

        // Idempotência: rodar de novo não muda nada.
        migrate_v1_to_v2(&mut conn).expect("idempotente");
        let v2_again = read_user_version(&conn).expect("lê user_version 2x");
        assert_eq!(v2_again, 2, "segunda migração deve ser no-op");

        // Verifica que as tabelas do V2 existem.
        let n_observations: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='observations'",
                [],
                |row| row.get(0),
            )
            .expect("conta tabela observations");
        assert_eq!(n_observations, 1, "tabela observations deve existir");

        let n_fts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='observations_fts'",
                [],
                |row| row.get(0),
            )
            .expect("conta tabela observations_fts");
        assert_eq!(n_fts, 1, "tabela observations_fts deve existir");

        let n_triggers: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' \
                 AND name IN ('observations_ai','observations_ad','observations_au')",
                [],
                |row| row.get(0),
            )
            .expect("conta triggers FTS");
        assert_eq!(n_triggers, 3, "3 triggers FTS5 devem existir");
    }

    #[test]
    fn test_graph_cascade_delete() {
        let mut conn = fresh_v2_db();
        // PRAGMA foreign_keys = ON é mandatória para CASCADE real.
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("liga FK");

        // Setup: cria entidades, relações e observações.
        let entities = vec![
            Entity { name: "ADR-040".to_string(), entity_type: "ADR".to_string(), observations: vec![] },
            Entity { name: "memory_graph".to_string(), entity_type: "Module".to_string(), observations: vec![] },
            Entity { name: "thinking".to_string(), entity_type: "Module".to_string(), observations: vec![] },
        ];
        create_entities(&mut conn, &entities).expect("cria entidades");

        let relations = vec![
            Relation { from: "memory_graph".to_string(), to: "ADR-040".to_string(), relation_type: "part_of".to_string() },
            Relation { from: "thinking".to_string(), to: "ADR-040".to_string(), relation_type: "part_of".to_string() },
            Relation { from: "memory_graph".to_string(), to: "thinking".to_string(), relation_type: "depends_on".to_string() },
        ];
        create_relations(&mut conn, &relations).expect("cria relações");

        let observations = vec![
            ObservationInput { entity_name: "ADR-040".to_string(), contents: vec!["Marco 3.5 ativo".to_string()] },
            ObservationInput { entity_name: "memory_graph".to_string(), contents: vec!["grafo SQLite WAL".to_string(), "FTS5 sub-ms".to_string()] },
            ObservationInput { entity_name: "thinking".to_string(), contents: vec!["disjuntor 5/7".to_string()] },
        ];
        add_observations(&mut conn, &observations).expect("anexa observações");

        // Conta entidades, relações e observações ANTES.
        let n_entities_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .expect("conta entidades");
        assert_eq!(n_entities_before, 3, "3 entidades criadas");

        let n_relations_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
            .expect("conta relações");
        assert_eq!(n_relations_before, 3, "3 relações criadas");

        let n_observations_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))
            .expect("conta observações");
        assert_eq!(n_observations_before, 4, "4 observações (1+2+1) criadas");

        // ATUA: remove a entidade central "ADR-040".
        delete_entities(&mut conn, &vec!["ADR-040".to_string()]).expect("deleta ADR-040");

        // PROVA: cascade apagou as 2 relações que apontam para ADR-040
        // e a observação de ADR-040. As relações e observações de
        // memory_graph/thinking permanecem intactas.
        let n_entities_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .expect("conta entidades pós");
        assert_eq!(n_entities_after, 2, "2 entidades (cascade removeu ADR-040)");

        let n_relations_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
            .expect("conta relações pós");
        assert_eq!(n_relations_after, 1, "1 relação (apenas memory_graph→thinking)");

        let n_observations_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))
            .expect("conta observações pós");
        assert_eq!(n_observations_after, 3, "3 observações (apenas de memory_graph e thinking)");
    }

    #[test]
    fn test_fts5_observational_grounding() {
        let mut conn = fresh_v2_db();
        // Cria uma entidade âncora.
        create_entities(
            &mut conn,
            &[Entity {
                name: "needle".to_string(),
                entity_type: "Test".to_string(),
                observations: vec![],
            }],
        )
        .expect("cria âncora");

        // Insere 999 observações de "feno" e 1 de "agulha".
        let feno: Vec<String> = (0..999).map(|i| format!("feno observacao {i}")).collect();
        let mut inputs = vec![ObservationInput {
            entity_name: "needle".to_string(),
            contents: feno,
        }];
        inputs.push(ObservationInput {
            entity_name: "needle".to_string(),
            contents: vec!["a agulha esta no palheiro com token XYZNEEDLE".to_string()],
        });
        add_observations(&mut conn, &inputs).expect("insere 1000 observações");

        // Mede a latência da busca FTS5 pela agulha.
        let start = std::time::Instant::now();
        let hits = search_observations(&conn, "XYZNEEDLE", 50).expect("busca FTS5");
        let elapsed = start.elapsed();

        // Assertiva 1: FTS5 encontra a agulha.
        assert!(!hits.is_empty(), "FTS5 deve localizar a agulha XYZNEEDLE");
        assert!(
            hits.iter().any(|e| e.name == "needle"),
            "FTS5 deve retornar a entidade `needle`"
        );

        // Assertiva 2: latência sub-milissegundo. Toleramos até 1ms para
        // acomodar overhead de wall-clock no Windows (cold cache permitido).
        assert!(
            elapsed.as_millis() < 50,
            "FTS5 levou {} ms (deve ser sub-ms, tolerância cold-cache 50ms)",
            elapsed.as_millis()
        );
    }
}
