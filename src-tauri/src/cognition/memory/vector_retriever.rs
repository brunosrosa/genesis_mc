use arrow_array::{Float32Array, StringArray};
use futures_util::StreamExt;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Estrutura para representar uma correspondência vetorial retornada pelo LanceDB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorialMatch {
    pub observation_id: String,
    pub content: String,
    pub similarity: f32,
    pub file_path: String,
    pub metadata: serde_json::Value,
}

/// Recuperador vetorial local alimentado pelo LanceDB via mapeamento de memória (mmap NVMe).
/// REGRA HARDWARE (RTX 2060m Protection): Consumo de VRAM = 0 MB.
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

    /// Executa a busca vetorial por similaridade de cosseno usando o leitor de arquivos mmap do LanceDB.
    pub async fn search_vectorial(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorialMatch>, String> {
        eprintln!(
            "[VectorRetriever] Conectando ao LanceDB via NVMe MMAP em '{:?}'. VRAM footprint: 0 MB",
            self.db_path
        );

        if query_embedding.is_empty() {
            return Ok(Vec::new());
        }

        let db_path_str = self.db_path.to_string_lossy().to_string();

        let db = match connect(&db_path_str).execute().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("[VectorRetriever] Aviso ao abrir LanceDB (fail-soft): {}", e);
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

        const TABLE_NAME: &str = "observations_vector";
        if !table_names.contains(&TABLE_NAME.to_string()) {
            return Ok(Vec::new());
        }

        let table = match db.open_table(TABLE_NAME).execute().await {
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

        let stream = match query.execute().await {
            Ok(s) => s,
            Err(e) => return Err(format!("Erro ao executar busca vetorial LanceDB: {}", e)),
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
            let id_col = batch
                .column_by_name("observation_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let content_col = batch
                .column_by_name("content")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let entity_col = batch
                .column_by_name("entity_name")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let file_path_col = batch
                .column_by_name("file_path")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            let distance_col = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            for i in 0..num_rows {
                let obs_id = id_col.map(|c| c.value(i).to_string()).unwrap_or_else(|| i.to_string());
                let content = content_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                let entity_name = entity_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                let file_path = file_path_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                let dist = distance_col.map(|d| d.value(i)).unwrap_or(1.0);
                
                // Converte distância L2/Cosseno em score de similaridade [0.0, 1.0]
                let similarity = 1.0 / (1.0 + dist.max(0.0));

                matches.push(VectorialMatch {
                    observation_id: obs_id,
                    content,
                    similarity,
                    file_path,
                    metadata: json!({
                        "entity_name": entity_name,
                        "raw_distance": dist
                    }),
                });
            }
        }

        Ok(matches)
    }
}
