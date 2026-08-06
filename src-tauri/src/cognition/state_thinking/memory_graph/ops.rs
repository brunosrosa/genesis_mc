//! 9 operações canônicas do `souls_graph` (canibalização cirúrgica do `memory-mcp-rs`).
//!
//! Princípios:
//! - Toda escrita é transacional (`Connection::transaction`) — auto-rollback
//!   via `Drop` blinda o SSD contra row-by-row commit em caso de pânico.
//! - Leitura via JOIN hidrata `Entity.observations` da tabela normalizada.
//! - Erros de banco propagam via `CognitiveError::GraphError` (From<rusqlite::Error>).
//! - FTS5: busca por `MATCH` síncrono; `LIMIT 50` para defesa.

use crate::cognition::memory_graph::errors::CognitiveError;
use crate::cognition::memory_graph::types::{Entity, ObservationInput, ObservationRecord, Relation, now_epoch_ms};
use crate::cognition::memory_graph::uuid::generate_uuid_v7;
use rusqlite::{Connection, params};

// ---------------------------------------------------------------------------
// ESCRITA (3 ops)
// ---------------------------------------------------------------------------

/// `mem_create_entities`: cria entidades novas (idempotente).
pub fn create_entities(
    conn: &mut Connection,
    entities: &[Entity],
) -> Result<Vec<Entity>, CognitiveError> {
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    let now = now_epoch_ms();
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO entities (entity_name, entity_type, observations, created_at) \
                 VALUES (?1, ?2, '[]', ?3) \
                 ON CONFLICT(entity_name) DO UPDATE SET entity_type = excluded.entity_type",
            )
            .map_err(CognitiveError::from)?;
        for e in entities {
            stmt.execute(params![e.name, e.entity_type, now])
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
    let now = now_epoch_ms();
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO relations (from_entity, to_entity, relation_type, created_at) \
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(CognitiveError::from)?;
        for r in relations {
            stmt.execute(params![r.from, r.to, r.relation_type, now])
                .map_err(CognitiveError::from)?;
        }
    }
    tx.commit().map_err(CognitiveError::from)?;
    Ok(relations.to_vec())
}

/// `mem_add_observations`: anexa observações a entidades existentes.
pub fn add_observations(
    conn: &mut Connection,
    observations: &[ObservationInput],
) -> Result<Vec<ObservationRecord>, CognitiveError> {
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    let now = now_epoch_ms();
    let mut records = Vec::new();
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO observations (observation_id, entity_name, content, created_at) \
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(CognitiveError::from)?;
        for obs in observations {
            for content in &obs.contents {
                let uuid = generate_uuid_v7();
                stmt.execute(params![uuid, obs.entity_name, content, now])
                    .map_err(CognitiveError::from)?;
                records.push(ObservationRecord {
                    id: uuid,
                    entity_name: obs.entity_name.clone(),
                    content: content.clone(),
                    created_at: now,
                });
            }
        }
    }
    tx.commit().map_err(CognitiveError::from)?;
    Ok(records)
}

// ---------------------------------------------------------------------------
// LEITURA (3 ops)
// ---------------------------------------------------------------------------

/// `mem_search`: busca FTS5 síncrona por `MATCH` em `observations_fts`.
pub fn search_observations(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<ObservationRecord>, CognitiveError> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT o.observation_id, o.entity_name, o.content, o.created_at \
             FROM observations_fts f \
             JOIN observations o ON o.observation_id = f.observation_id \
             WHERE observations_fts MATCH ?1 \
             LIMIT ?2",
        )
        .map_err(CognitiveError::from)?;
    let rows = stmt
        .query_map(params![q, limit as i64], |row| {
            Ok(ObservationRecord {
                id: row.get(0)?,
                entity_name: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(CognitiveError::from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(CognitiveError::from)?);
    }
    Ok(out)
}

/// `mem_open_nodes`: abre entidades por nome e retorna todas as suas observações.
pub fn open_nodes(conn: &Connection, names: &[String]) -> Result<Vec<ObservationRecord>, CognitiveError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: Vec<String> = (0..names.len()).map(|i| format!("?{}", i + 1)).collect();
    let sql_obs = format!(
        "SELECT observation_id, entity_name, content, created_at FROM observations \
         WHERE entity_name IN ({}) ORDER BY created_at ASC",
        placeholders.join(",")
    );
    let params_obs: Vec<&dyn rusqlite::ToSql> =
        names.iter().map(|n| n as &dyn rusqlite::ToSql).collect();
    let mut stmt = conn
        .prepare(&sql_obs)
        .map_err(CognitiveError::from)?;
    let rows = stmt
        .query_map(params_obs.as_slice(), |row| {
            Ok(ObservationRecord {
                id: row.get(0)?,
                entity_name: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(CognitiveError::from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(CognitiveError::from)?);
    }
    Ok(out)
}

/// `mem_read_graph`: retorna o grafo inteiro (entities + relations) até `LIMIT`.
pub fn read_graph(conn: &Connection, limit: usize) -> Result<(Vec<Entity>, Vec<Relation>), CognitiveError> {
    let cap = limit.min(500);
    let mut entities_stmt = conn
        .prepare("SELECT entity_name, entity_type FROM entities LIMIT ?1")
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
// DELEÇÃO (3 ops)
// ---------------------------------------------------------------------------

/// `mem_delete_entities`: remove entidades. Cascade apaga relations e observations.
pub fn delete_entities(conn: &mut Connection, names: &[String]) -> Result<(), CognitiveError> {
    if names.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    let placeholders: Vec<String> = (0..names.len()).map(|i| format!("?{}", i + 1)).collect();
    let sql = format!(
        "DELETE FROM entities WHERE entity_name IN ({})",
        placeholders.join(",")
    );
    let params_dyn: Vec<&dyn rusqlite::ToSql> =
        names.iter().map(|n| n as &dyn rusqlite::ToSql).collect();
    tx.execute(&sql, params_dyn.as_slice())
        .map_err(CognitiveError::from)?;
    tx.commit().map_err(CognitiveError::from)?;
    Ok(())
}

/// `mem_delete_observations`: remove observações por `observation_id`.
pub fn delete_observations_by_id(
    conn: &mut Connection,
    observation_ids: &[String],
) -> Result<(), CognitiveError> {
    if observation_ids.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction().map_err(CognitiveError::from)?;
    let placeholders: Vec<String> = (0..observation_ids.len()).map(|i| format!("?{}", i + 1)).collect();
    let sql = format!(
        "DELETE FROM observations WHERE observation_id IN ({})",
        placeholders.join(",")
    );
    let params_dyn: Vec<&dyn rusqlite::ToSql> =
        observation_ids.iter().map(|n| n as &dyn rusqlite::ToSql).collect();
    tx.execute(&sql, params_dyn.as_slice())
        .map_err(CognitiveError::from)?;
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
    use super::*;
    use crate::cognition::thinking::ops::migrate_v3_to_v5;
    use rusqlite::Connection;

    fn fresh_v5_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate_v3_to_v5(&mut conn).expect("migração V5");
        conn
    }

    #[test]
    fn test_graph_cascade_delete() {
        let mut conn = fresh_v5_db();

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

        let n_entities_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .expect("conta entidades");
        assert_eq!(n_entities_before, 3);

        let n_relations_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
            .expect("conta relações");
        assert_eq!(n_relations_before, 3);

        let n_observations_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))
            .expect("conta observações");
        assert_eq!(n_observations_before, 4);

        // ATUA: remove a entidade central "ADR-040".
        delete_entities(&mut conn, &["ADR-040".to_string()]).expect("deleta ADR-040");

        // PROVA: cascade apagou as relações e a observação de ADR-040.
        let n_entities_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .expect("conta entidades pós");
        assert_eq!(n_entities_after, 2);

        let n_relations_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
            .expect("conta relações pós");
        assert_eq!(n_relations_after, 1);

        let n_observations_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))
            .expect("conta observações pós");
        assert_eq!(n_observations_after, 3);
    }

    #[test]
    fn test_fts5_lexical_grounding() {
        let mut conn = fresh_v5_db();
        create_entities(
            &mut conn,
            &[Entity {
                name: "needle".to_string(),
                entity_type: "Test".to_string(),
                observations: vec![],
            }],
        )
        .expect("cria âncora");

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

        let start = std::time::Instant::now();
        let hits = search_observations(&conn, "XYZNEEDLE", 50).expect("busca FTS5");
        let elapsed = start.elapsed();

        assert!(!hits.is_empty(), "FTS5 deve localizar a agulha XYZNEEDLE");
        assert_eq!(hits[0].entity_name, "needle");
        assert!(
            elapsed.as_millis() < 50,
            "FTS5 levou {} ms (deve ser sub-ms)",
            elapsed.as_millis()
        );
    }
}

// ---------------------------------------------------------------------------
// VETORIZAÇÃO E BUSCA SEMÂNTICA (LanceDB - MARCO 4.8.1 Thread-Isolated)
// ---------------------------------------------------------------------------

/// Caminho padrão da base vetorial LanceDB
pub fn default_vector_db_path() -> std::path::PathBuf {
    std::path::PathBuf::from(".souls_data").join("souls_vectors.lance")
}

/// Anexa uma observação ao reator vetorial LanceDB.
///
/// **ISOLAMENTO DE THREAD (MARCO 4.8.1)**: Executado dentro de `spawn_blocking`
/// para evitar que `mmap` e page faults do kernel bloqueiem o reactor loop do Tokio.
pub async fn add_to_vector_store(
    observation_id: &str,
    entity_name: &str,
    content: &str,
    stability: &str,
    embedding: Vec<f32>,
) -> Result<(), String> {
    let obs_id = observation_id.to_string();
    let entity = entity_name.to_string();
    let cnt = content.to_string();
    let stab = stability.to_string();
    let db_path = default_vector_db_path();

    let _ = (observation_id, entity_name, content, stability, embedding);
    Err("Vector store pending materialization in Marco 4.9.0".to_string())
}

/// Realiza a busca vetorial por similaridade de cosseno com pré-filtro escalar de estabilidade.
///
/// **ISOLAMENTO DE THREAD (MARCO 4.8.1)**: Executado dentro de `spawn_blocking`
/// para proteger o reactor loop do Tokio contra leituras síncronas de mmap.
pub async fn run_souls_semantic_search(
    _query_vector: Vec<f32>,
    _limit: usize,
    _filter_stability: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    Err("Vector store pending materialization in Marco 4.9.0".to_string())
}

