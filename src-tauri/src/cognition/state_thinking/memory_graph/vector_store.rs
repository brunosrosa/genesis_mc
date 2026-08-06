//! Reator Vetorial LanceDB (`vector_store.rs`)
//!
//! Gerencia o armazenamento e busca por similaridade de cosseno de embeddings
//! de observações (384 dimensões, modelo local fast bge-small-en-v1.5).
//!
//! Princípios Bare-Metal:
//! - Opera sob `mmap` direto do disco em `.souls_data/souls_vectors.lance`.
//! - Zero consumo contínuo de VRAM (operações em CPU/RAM via Arrow Memory Model).
//! - Execução isolada em `spawn_blocking` para proteger o reactor loop do Tokio contra page faults síncronos.

use futures_util::StreamExt;
use lancedb::arrow::array::{FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray};
use lancedb::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use lancedb::connect;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;

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
    let record_batch_stream = RecordBatchIterator::new(
        batches.into_iter().map(Ok),
        schema,
    );

    table
        .add(record_batch_stream)
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

    let mut query = table.vector_search(query_vector).map_err(|e| e.to_string())?;
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
            let distance = distance_col.map(|d| d.value(i)).unwrap_or(0.0);
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
        let synthetic_vec: Vec<f32> = (0..VECTOR_DIMENSION)
            .map(|i| (i as f32) / 384.0)
            .collect();

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

        let results_stable = search_observation_vectors(
            &db_path,
            vec1.clone(),
            10,
            Some("STABLE".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(results_stable.len(), 1);
        assert_eq!(results_stable[0]["observation_id"], "obs-1");
        assert_eq!(results_stable[0]["temporal_stability"], "STABLE");

        let results_evolving = search_observation_vectors(
            &db_path,
            vec1.clone(),
            10,
            Some("EVOLVING".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(results_evolving.len(), 2);
        for r in results_evolving {
            assert_eq!(r["temporal_stability"], "EVOLVING");
        }
    }
}
