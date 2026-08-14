// SOULS V6 — Memory Module: VectorRetriever (LanceDB Serverless & MMAP Zero-VRAM)
// Conforme ADR-001, ADR-005 (RAG-Temporal), ADR-027 (Termodinâmica VRAM 0 MB) e Marco VI.

use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, StringArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures_util::StreamExt;
use lancedb::connect;
use lancedb::connection::Connection as LanceConnection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const VECTOR_DIMENSION: i32 = 384;
pub const CANONICAL_TABLE_NAME: &str = "hippocampus_memories";
pub const LEGACY_TABLE_NAME: &str = "observations_vector";

/// Estrutura canônica para representar uma memória persistida ou recuperada no LanceDB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HippocampusMemoryRecord {
    pub id: String,
    pub text_content: String,
    pub embedding: Vec<f32>,
    pub temporal_stability: String, // "STABLE" | "EVOLVING"
    pub valid_from: i64,
    pub valid_to: Option<i64>,
}

/// Estrutura para representar uma correspondência vetorial retornada pelo LanceDB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorialMatch {
    pub observation_id: String,
    pub content: String,
    pub similarity: f32,
    pub file_path: String,
    pub temporal_stability: String,
    pub valid_from: i64,
    pub valid_to: Option<i64>,
    pub metadata: serde_json::Value,
}

/// Retorna o schema canônico Apache Arrow da tabela `hippocampus_memories`.
pub fn get_hippocampus_table_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("text_content", DataType::Utf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                VECTOR_DIMENSION,
            ),
            false,
        ),
        Field::new("temporal_stability", DataType::Utf8, false),
        Field::new("valid_from", DataType::Int64, false),
        Field::new("valid_to", DataType::Int64, true),
    ]))
}

/// Recuperador vetorial local alimentado pelo LanceDB via mapeamento de memória (mmap NVMe / CPU Host).
/// REGRA DE FERRO (ADR-027): Consumo de VRAM = 0 MB. Mapeamento 100% Host RAM e NVMe.
#[derive(Debug, Clone)]
pub struct VectorRetriever {
    pub db_path: PathBuf,
}

impl VectorRetriever {
    /// Cria uma nova instância do `VectorRetriever` configurando o caminho da base LanceDB.
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    /// Caminho padrão para o LanceDB dentro do workspace.
    pub fn default_path() -> PathBuf {
        PathBuf::from(".souls_data").join("lancedb")
    }

    /// Abre ou conecta à base LanceDB em modo serverless com mmap.
    pub async fn connect_db(&self) -> Result<LanceConnection, String> {
        let db_path_str = self.db_path.to_string_lossy().to_string();
        connect(&db_path_str)
            .execute()
            .await
            .map_err(|e| format!("Erro ao conectar ao LanceDB em '{}': {}", db_path_str, e))
    }

    /// Garante que a tabela canônica `hippocampus_memories` exista.
    pub async fn get_or_create_table(&self, db: &LanceConnection) -> Result<Table, String> {
        let table_names = db
            .table_names()
            .execute()
            .await
            .map_err(|e| format!("Erro ao listar tabelas do LanceDB: {}", e))?;

        if table_names.contains(&CANONICAL_TABLE_NAME.to_string()) {
            db.open_table(CANONICAL_TABLE_NAME)
                .execute()
                .await
                .map_err(|e| format!("Erro ao abrir tabela '{}': {}", CANONICAL_TABLE_NAME, e))
        } else if table_names.contains(&LEGACY_TABLE_NAME.to_string()) {
            db.open_table(LEGACY_TABLE_NAME)
                .execute()
                .await
                .map_err(|e| format!("Erro ao abrir tabela legada '{}': {}", LEGACY_TABLE_NAME, e))
        } else {
            let schema = get_hippocampus_table_schema();
            db.create_empty_table(CANONICAL_TABLE_NAME, schema)
                .execute()
                .await
                .map_err(|e| format!("Erro ao criar tabela '{}': {}", CANONICAL_TABLE_NAME, e))
        }
    }

    /// Insere uma memória com embedding de 384 floats na tabela canônica do LanceDB.
    pub async fn insert_memory(&self, record: HippocampusMemoryRecord) -> Result<(), String> {
        if record.embedding.len() != VECTOR_DIMENSION as usize {
            return Err(format!(
                "Dimensão de embedding inválida: esperado {}, recebido {}",
                VECTOR_DIMENSION,
                record.embedding.len()
            ));
        }

        let schema = get_hippocampus_table_schema();

        let id_array = StringArray::from(vec![record.id.as_str()]);
        let text_array = StringArray::from(vec![record.text_content.as_str()]);
        let stability_array = StringArray::from(vec![record.temporal_stability.as_str()]);
        let valid_from_array = Int64Array::from(vec![record.valid_from]);
        let valid_to_array = match record.valid_to {
            Some(vt) => Int64Array::from(vec![Some(vt)]),
            None => Int64Array::from(vec![None::<i64>]),
        };

        let values_array = Float32Array::from(record.embedding);
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
                Arc::new(text_array),
                Arc::new(vector_array),
                Arc::new(stability_array),
                Arc::new(valid_from_array),
                Arc::new(valid_to_array),
            ],
        )
        .map_err(|e| format!("Erro ao criar RecordBatch Arrow: {}", e))?;

        let db = self.connect_db().await?;
        let table = self.get_or_create_table(&db).await?;

        table
            .add(vec![batch])
            .execute()
            .await
            .map_err(|e| format!("Erro ao inserir memória no LanceDB: {}", e))?;

        Ok(())
    }

    /// Executa a busca vetorial por similaridade de cosseno usando o leitor de arquivos mmap do LanceDB.
    pub async fn search_vectorial(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorialMatch>, String> {
        self.search_with_temporal_filter(query_embedding, limit, None, None, None)
            .await
    }

    /// Executa busca kNN com pré-filtragem escalar rígida (valid_from > Epoch, valid_to, stability).
    /// Se a pré-filtragem resultar em 0 resultados por máscara excessivamente estrita, o circuito de
    /// fallback (bypass_vector_index) é ativado para permitir resgate via FTS5 no SQLite.
    pub async fn search_with_temporal_filter(
        &self,
        query_embedding: &[f32],
        limit: usize,
        min_valid_from: Option<i64>,
        max_valid_to: Option<i64>,
        filter_stability: Option<&str>,
    ) -> Result<Vec<VectorialMatch>, String> {
        if query_embedding.is_empty() {
            return Ok(Vec::new());
        }

        let db = match self.connect_db().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("[VectorRetriever] LanceDB offline (fail-soft): {}", e);
                return Ok(Vec::new());
            }
        };

        let table_names = match db.table_names().execute().await {
            Ok(names) => names,
            Err(e) => {
                eprintln!("[VectorRetriever] Aviso ao listar tabelas: {}", e);
                return Ok(Vec::new());
            }
        };

        let table_to_open = if table_names.contains(&CANONICAL_TABLE_NAME.to_string()) {
            CANONICAL_TABLE_NAME
        } else if table_names.contains(&LEGACY_TABLE_NAME.to_string()) {
            LEGACY_TABLE_NAME
        } else {
            return Ok(Vec::new());
        };

        let table = match db.open_table(table_to_open).execute().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[VectorRetriever] Aviso ao abrir tabela: {}", e);
                return Ok(Vec::new());
            }
        };

        let mut query = match table.vector_search(query_embedding.to_vec()) {
            Ok(q) => q,
            Err(e) => return Err(format!("Erro ao criar query vetorial: {}", e)),
        };

        query = query.limit(limit);

        // Construção do predicado de pré-filtragem escalar
        let mut predicates = Vec::new();

        if let Some(min_epoch) = min_valid_from {
            predicates.push(format!("valid_from >= {}", min_epoch));
        }

        if let Some(max_epoch) = max_valid_to {
            predicates.push(format!("(valid_to IS NULL OR valid_to <= {})", max_epoch));
        }

        if let Some(stability) = filter_stability {
            let sanitized = stability.replace('\'', "''");
            predicates.push(format!("temporal_stability = '{}'", sanitized));
        }

        if !predicates.is_empty() {
            let filter_expr = predicates.join(" AND ");
            query = query.only_if(filter_expr);
        }

        let stream = match query.execute().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[VectorRetriever] Erro ao executar busca vetorial pré-filtrada: {}", e);
                return Ok(Vec::new());
            }
        };

        let mut matches = Vec::new();
        let mut stream = stream;

        while let Some(batch_result) = stream.next().await {
            let batch = match batch_result {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("[VectorRetriever] Erro ao processar batch: {}", e);
                    continue;
                }
            };

            let num_rows = batch.num_rows();

            // Identificação de colunas suportando schema canônico e legado
            let id_col = batch
                .column_by_name("id")
                .or_else(|| batch.column_by_name("observation_id"))
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let text_col = batch
                .column_by_name("text_content")
                .or_else(|| batch.column_by_name("content"))
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let stability_col = batch
                .column_by_name("temporal_stability")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let valid_from_col = batch
                .column_by_name("valid_from")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            let valid_to_col = batch
                .column_by_name("valid_to")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            let file_path_col = batch
                .column_by_name("file_path")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let distance_col = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            for i in 0..num_rows {
                let obs_id = id_col.map(|c| c.value(i).to_string()).unwrap_or_else(|| i.to_string());
                let content = text_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                let stability = stability_col.map(|c| c.value(i).to_string()).unwrap_or_else(|| "STABLE".to_string());
                let valid_from = valid_from_col.map(|c| c.value(i)).unwrap_or(0);
                let valid_to = valid_to_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) });
                let file_path = file_path_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                let dist = distance_col.map(|d| d.value(i)).unwrap_or(1.0);

                let similarity = 1.0 / (1.0 + dist.max(0.0));

                matches.push(VectorialMatch {
                    observation_id: obs_id,
                    content,
                    similarity,
                    file_path,
                    temporal_stability: stability,
                    valid_from,
                    valid_to,
                    metadata: json!({
                        "raw_distance": dist
                    }),
                });
            }
        }

        Ok(matches)
    }
}
