//! Reator Vetorial LanceDB (`vector_store.rs`)
//!
//! Gerencia o armazenamento e busca por similaridade de cosseno de embeddings
//! de observações (384 dimensões, modelo local fast bge-small-en-v1.5).
//!
//! Princípios Bare-Metal:
//! - Opera sob `mmap` direto do disco em `.souls_data/souls_vectors.lance`.
//! - Zero consumo contínuo de VRAM (operações em CPU/RAM via Arrow Memory Model).
//! - Execução isolada em `spawn_blocking` para proteger o reactor loop do Tokio contra page faults síncronos.

use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};

use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures_util::StreamExt;
use lancedb::connect;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// Constante de suavização RRF (Reciprocal Rank Fusion)
pub const RRF_K: f64 = 60.0;

/// Estrutura de saída da busca híbrida unificada RRF
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HybridSearchResult {
    pub observation_id: String,
    pub entity_name: String,
    pub content: String,
    pub temporal_stability: String,
    pub rrf_score: f64,
}

/// Estrutura de entrada de documento para fusão RRF
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RrfDocumentInput {
    pub observation_id: String,
    pub entity_name: String,
    pub content: String,
    pub temporal_stability: String,
}

/// Executa a fusão RRF (Reciprocal Rank Fusion) de forma pura em RAM ($O(N \log N)$).
pub fn reciprocal_rank_fusion(
    fts5_results: Vec<RrfDocumentInput>,
    lancedb_results: Vec<RrfDocumentInput>,
) -> Vec<HybridSearchResult> {
    use std::collections::HashMap;

    let mut map: HashMap<String, (f64, RrfDocumentInput)> = HashMap::new();

    for (idx, doc) in fts5_results.into_iter().enumerate() {
        let rank = (idx + 1) as f64;
        let score = 1.0 / (RRF_K + rank);
        map.entry(doc.observation_id.clone())
            .and_modify(|(s, _)| *s += score)
            .or_insert((score, doc));
    }

    for (idx, doc) in lancedb_results.into_iter().enumerate() {
        let rank = (idx + 1) as f64;
        let score = 1.0 / (RRF_K + rank);
        map.entry(doc.observation_id.clone())
            .and_modify(|(s, _)| *s += score)
            .or_insert((score, doc));
    }

    let mut results: Vec<HybridSearchResult> = map
        .into_iter()
        .map(|(obs_id, (score, doc))| HybridSearchResult {
            observation_id: obs_id,
            entity_name: doc.entity_name,
            content: doc.content,
            temporal_stability: doc.temporal_stability,
            rrf_score: score,
        })
        .collect();

    results.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Dimensão padrão dos vetores de observação (bge-small-en-v1.5 / local fast embedding)
pub const VECTOR_DIMENSION: i32 = 384;

/// Nome da tabela canônica de vetores
pub const TABLE_NAME: &str = "observations_vector";

/// Retorna o Schema Apache Arrow da tabela `observations_vector`.
pub fn get_vector_table_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("observation_id", DataType::Utf8, false),
        Field::new("entity_name", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("temporal_stability", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                VECTOR_DIMENSION,
            ),
            false,
        ),
    ]))
}

/// Abre ou cria a conexão com a base LanceDB em `.souls_data/souls_vectors.lance`.
pub async fn open_vector_db<P: AsRef<Path>>(path: P) -> Result<Connection, String> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    connect(&path_str)
        .execute()
        .await
        .map_err(|e| format!("Erro ao conectar ao LanceDB em '{}': {}", path_str, e))
}

/// Abre ou inicializa a tabela `observations_vector` no LanceDB.
pub async fn get_or_create_vector_table(db: &Connection) -> Result<Table, String> {
    let table_names = db
        .table_names()
        .execute()
        .await
        .map_err(|e| format!("Erro ao listar tabelas do LanceDB: {}", e))?;

    if table_names.contains(&TABLE_NAME.to_string()) {
        db.open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| format!("Erro ao abrir tabela '{}': {}", TABLE_NAME, e))
    } else {
        let schema = get_vector_table_schema();
        db.create_empty_table(TABLE_NAME, schema)
            .execute()
            .await
            .map_err(|e| format!("Erro ao criar tabela vazia '{}': {}", TABLE_NAME, e))
    }
}

/// Insere uma observação com seu embedding de 384 floats na tabela LanceDB.
pub async fn insert_observation_vector<P: AsRef<Path>>(
    db_path: P,
    observation_id: &str,
    entity_name: &str,
    content: &str,
    stability: &str,
    embedding: Vec<f32>,
) -> Result<(), String> {
    if embedding.len() != VECTOR_DIMENSION as usize {
        return Err(format!(
            "Dimensão de embedding inválida: esperado {}, recebido {}",
            VECTOR_DIMENSION,
            embedding.len()
        ));
    }

    let schema = get_vector_table_schema();

    // Constrói os arrays Apache Arrow
    let id_array = StringArray::from(vec![observation_id]);
    let entity_array = StringArray::from(vec![entity_name]);
    let content_array = StringArray::from(vec![content]);
    let stability_array = StringArray::from(vec![stability]);

    let values_array = Float32Array::from(embedding);
    let vector_array = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        VECTOR_DIMENSION,
        Arc::new(values_array),
        None,
    );

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(id_array),
            Arc::new(entity_array),
            Arc::new(content_array),
            Arc::new(stability_array),
            Arc::new(vector_array),
        ],
    )
    .map_err(|e| format!("Erro ao criar RecordBatch Arrow: {}", e))?;

    let db = open_vector_db(db_path).await?;
    let table = get_or_create_vector_table(&db).await?;

    let batches = vec![batch];

    table
        .add(batches)
        .execute()
        .await
        .map_err(|e| format!("Erro ao inserir vetor no LanceDB: {}", e))?;

    Ok(())
}

/// Executa busca vetorial por similaridade de cosseno com pré-filtro escalar opcional.
pub async fn search_observation_vectors<P: AsRef<Path>>(
    db_path: P,
    query_vector: Vec<f32>,
    limit: usize,
    filter_stability: Option<String>,
) -> Result<Vec<Value>, String> {
    if query_vector.len() != VECTOR_DIMENSION as usize {
        return Err(format!(
            "Dimensão de query vector inválida: esperado {}, recebido {}",
            VECTOR_DIMENSION,
            query_vector.len()
        ));
    }

    let db = open_vector_db(db_path).await?;
    let table_names = db
        .table_names()
        .execute()
        .await
        .map_err(|e| format!("Erro ao listar tabelas: {}", e))?;

    if !table_names.contains(&TABLE_NAME.to_string()) {
        return Ok(Vec::new());
    }

    let table = db
        .open_table(TABLE_NAME)
        .execute()
        .await
        .map_err(|e| format!("Erro ao abrir tabela '{}': {}", TABLE_NAME, e))?;

    let mut query = table
        .vector_search(query_vector)
        .map_err(|e| e.to_string())?;
    query = query.limit(limit);

    if let Some(stability) = filter_stability {
        let filter_expr = format!("temporal_stability = '{}'", stability.replace('\'', "''"));
        query = query.only_if(filter_expr);
    }

    let stream = query
        .execute()
        .await
        .map_err(|e| format!("Erro ao executar busca vetorial LanceDB: {}", e))?;

    let mut results = Vec::new();
    let mut stream = stream;

    while let Some(batch_result) = stream.next().await {
        let batch = batch_result.map_err(|e| format!("Erro ao ler stream de resultados: {}", e))?;
        let num_rows = batch.num_rows();

        let id_col = batch
            .column_by_name("observation_id")
            .ok_or_else(|| "Coluna 'observation_id' ausente no batch".to_string())?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| "Tipo inválido para 'observation_id'".to_string())?;

        let entity_col = batch
            .column_by_name("entity_name")
            .ok_or_else(|| "Coluna 'entity_name' ausente no batch".to_string())?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| "Tipo inválido para 'entity_name'".to_string())?;

        let content_col = batch
            .column_by_name("content")
            .ok_or_else(|| "Coluna 'content' ausente no batch".to_string())?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| "Tipo inválido para 'content'".to_string())?;

        let stability_col = batch
            .column_by_name("temporal_stability")
            .ok_or_else(|| "Coluna 'temporal_stability' ausente no batch".to_string())?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| "Tipo inválido para 'temporal_stability'".to_string())?;

        let distance_col = batch
            .column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

        for i in 0..num_rows {
            let distance = distance_col
                .map(|d: &Float32Array| d.value(i))
                .unwrap_or(0.0);

            results.push(json!({
                "observation_id": id_col.value(i),
                "entity_name": entity_col.value(i),
                "content": content_col.value(i),
                "temporal_stability": stability_col.value(i),
                "_distance": distance,
            }));
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_vector_store_crud() {
        let dir = tempdir().expect("Falha ao criar tempdir");
        let db_path = dir.path().join("test_vectors.lance");

        let obs_id = "01912345-6789-7abc-def0-123456789abc";
        let entity = "STABLE_CORE";
        let content = "Teste de observação persistida no LanceDB com mmap.";
        let stability = "STABLE";
        let synthetic_vec: Vec<f32> = (0..VECTOR_DIMENSION).map(|i| (i as f32) / 384.0).collect();

        insert_observation_vector(
            &db_path,
            obs_id,
            entity,
            content,
            stability,
            synthetic_vec.clone(),
        )
        .await
        .expect("Falha na inserção no LanceDB");

        let results = search_observation_vectors(&db_path, synthetic_vec, 5, None)
            .await
            .expect("Falha na busca vetorial");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["observation_id"], obs_id);
        assert_eq!(results[0]["entity_name"], entity);
        assert_eq!(results[0]["temporal_stability"], stability);
    }

    #[tokio::test]
    async fn test_semantic_search_with_scalar_filter() {
        let dir = tempdir().expect("Falha ao criar tempdir");
        let db_path = dir.path().join("test_vectors_filter.lance");

        let vec1: Vec<f32> = vec![1.0; VECTOR_DIMENSION as usize];
        let vec2: Vec<f32> = vec![1.0; VECTOR_DIMENSION as usize];
        let vec3: Vec<f32> = vec![1.0; VECTOR_DIMENSION as usize];

        insert_observation_vector(
            &db_path,
            "obs-1",
            "Entity1",
            "Obs 1 STABLE",
            "STABLE",
            vec1.clone(),
        )
        .await
        .unwrap();

        insert_observation_vector(
            &db_path,
            "obs-2",
            "Entity2",
            "Obs 2 EVOLVING",
            "EVOLVING",
            vec2.clone(),
        )
        .await
        .unwrap();

        insert_observation_vector(
            &db_path,
            "obs-3",
            "Entity3",
            "Obs 3 EVOLVING",
            "EVOLVING",
            vec3.clone(),
        )
        .await
        .unwrap();

        let results_stable =
            search_observation_vectors(&db_path, vec1.clone(), 10, Some("STABLE".to_string()))
                .await
                .unwrap();

        assert_eq!(results_stable.len(), 1);
        assert_eq!(results_stable[0]["observation_id"], "obs-1");
        assert_eq!(results_stable[0]["temporal_stability"], "STABLE");

        let results_evolving =
            search_observation_vectors(&db_path, vec1.clone(), 10, Some("EVOLVING".to_string()))
                .await
                .unwrap();

        assert_eq!(results_evolving.len(), 2);
        for r in results_evolving {
            assert_eq!(r["temporal_stability"], "EVOLVING");
        }
    }

    #[test]
    fn test_pure_reciprocal_rank_fusion() {
        let fts_docs = vec![
            RrfDocumentInput {
                observation_id: "doc-1".to_string(),
                entity_name: "E1".to_string(),
                content: "Content 1".to_string(),
                temporal_stability: "STABLE".to_string(),
            },
            RrfDocumentInput {
                observation_id: "doc-2".to_string(),
                entity_name: "E2".to_string(),
                content: "Content 2".to_string(),
                temporal_stability: "STABLE".to_string(),
            },
        ];

        let vector_docs = vec![
            RrfDocumentInput {
                observation_id: "doc-2".to_string(),
                entity_name: "E2".to_string(),
                content: "Content 2".to_string(),
                temporal_stability: "STABLE".to_string(),
            },
            RrfDocumentInput {
                observation_id: "doc-3".to_string(),
                entity_name: "E3".to_string(),
                content: "Content 3".to_string(),
                temporal_stability: "EVOLVING".to_string(),
            },
        ];

        let fused = reciprocal_rank_fusion(fts_docs, vector_docs);

        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].observation_id, "doc-2");
        assert_eq!(fused[1].observation_id, "doc-1");
        assert_eq!(fused[2].observation_id, "doc-3");
    }

    #[tokio::test]
    async fn test_hybrid_search_rrf_synthesis() {
        use crate::cognition::memory_graph::ops::{
            create_entities, run_souls_hybrid_search_with_paths,
        };
        use crate::cognition::memory_graph::types::Entity;
        use crate::cognition::thinking::ops::migrate_v3_to_v5;
        use rusqlite::Connection;

        let dir = tempdir().expect("Falha ao criar tempdir");
        let sqlite_path = dir.path().join("test_state.db");
        let lance_path = dir.path().join("test_vectors.lance");

        let mut conn = Connection::open(&sqlite_path).expect("open test_state.db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate_v3_to_v5(&mut conn).expect("migração V5");

        let obs_a_id = "obs-a-compiler-err";
        let obs_b_id = "obs-b-conceptual-synonym";

        let entity = Entity {
            name: "RRF_TEST".to_string(),
            entity_type: "Test".to_string(),
            observations: vec![],
        };
        create_entities(&mut conn, &[entity]).expect("cria entidade");

        conn.execute(
            "INSERT INTO observations (observation_id, entity_name, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![obs_a_id, "RRF_TEST", "COMPILER_ERR_E0597: lifetime borrowing conflict in Rust async block", 1000],
        ).unwrap();

        conn.execute(
            "INSERT INTO observations (observation_id, entity_name, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![obs_b_id, "RRF_TEST", "lifetime parameter outlives reference scope in asynchronous context", 1001],
        ).unwrap();
        drop(conn);

        let generic_vec = vec![0.0f32; VECTOR_DIMENSION as usize];
        let target_vec = vec![1.0f32; VECTOR_DIMENSION as usize];

        insert_observation_vector(
            &lance_path,
            obs_a_id,
            "RRF_TEST",
            "COMPILER_ERR_E0597: lifetime borrowing conflict in Rust async block",
            "STABLE",
            generic_vec,
        )
        .await
        .unwrap();

        insert_observation_vector(
            &lance_path,
            obs_b_id,
            "RRF_TEST",
            "lifetime parameter outlives reference scope in asynchronous context",
            "STABLE",
            target_vec.clone(),
        )
        .await
        .unwrap();

        let results = run_souls_hybrid_search_with_paths(
            sqlite_path.clone(),
            lance_path.clone(),
            "COMPILER_ERR_E0597".to_string(),
            target_vec,
            10,
            None,
        )

        .await
        .expect("busca híbrida deve suceder");

        assert_eq!(results.len(), 2, "Ambos os documentos devem ser retornados");
        assert_eq!(
            results[0].observation_id, obs_a_id,
            "Documento A (hit léxico + vetorial parcial) deve estar no topo"
        );
        assert_eq!(
            results[1].observation_id, obs_b_id,
            "Documento B (hit vetorial) deve estar em segundo lugar no ranking híbrido"
        );
        assert!(results[0].rrf_score > results[1].rrf_score);
        assert!(results[1].rrf_score > 0.0);
    }
}

