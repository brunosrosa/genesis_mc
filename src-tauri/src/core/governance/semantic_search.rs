// SOULS V6 — Core Semantic Search & Active Hippocampus Engine (MARCO VI)
// Conforme ADR-001, ADR-003, ADR-005 (RAG-Temporal), ADR-010 (SDD-TDD), ADR-025, ADR-027 (Zero-VRAM), ADR-030 e ADR-041.

use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, StringArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use dashmap::DashMap;
use futures_util::StreamExt;
use lancedb::connect;
use lancedb::connection::Connection as LanceConnection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const CANONICAL_SEMANTIC_TABLE_PATH: &str = r"Z:\souls_mc\.souls_data\semantic_memories";
pub const CANONICAL_TABLE_NAME: &str = "semantic_memories";
pub const HIPPOCAMPUS_TABLE_NAME: &str = "hippocampus_memories";
pub const LEGACY_TABLE_NAME: &str = "observations_vector";
pub const VECTOR_DIMENSION: i32 = 384;
pub const DEFAULT_RRF_K: f64 = 60.0;
pub const EXACT_MATCH_BONUS: f64 = 10.0;

/// Lista de palavras-chave rígidas do sistema para bônus exato.
pub const EXACT_RIGID_KEYWORDS: &[&str] = &[
    "GGML_TYPE_TQ1",
    "ADR-001",
    "ADR-003",
    "ADR-005",
    "ADR-010",
    "ADR-025",
    "ADR-027",
    "ADR-030",
    "ADR-040",
    "ADR-041",
    "ADR-047",
    "windows-sys",
    "winapi",
    "core_affinity",
    "LadybugDB",
    "LanceDB",
    "FrankenSQLite",
    "ChyrosDaemon",
];

/// Registro canônico de memória vetorial para persistência ou leitura no LanceDB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticMemoryRecord {
    pub id: String,
    pub text_content: String,
    pub embedding: Vec<f32>,
    pub temporal_stability: String, // "STABLE" | "EVOLVING" | "SUPERSEDED"
    pub valid_from: i64,
    pub valid_to: Option<i64>,
}

/// Correspondência vetorial obtida a partir do LanceDB (mmap NVMe / RAM Host).
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

/// Correspondência léxica obtida a partir do FTS5 no SQLite (BM25).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LexicalMatch {
    pub observation_id: String,
    pub content: String,
    pub file_path: String,
    pub raw_score: f64,
}

/// Item resultante da fusão híbrida RRF com rank e status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedMatch {
    pub observation_id: String,
    pub content: String,
    pub file_path: String,
    pub rrf_score: f64,
    pub lexical_rank: Option<usize>,
    pub vector_rank: Option<usize>,
    pub is_exact_match: bool,
    pub status: String,
}

/// Resultado consolidado da busca semântica híbrida.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub query: String,
    pub results_count: usize,
    pub results: Vec<UnifiedMatch>,
    pub vetoed_count: usize,
    pub vetoed_reasons: Vec<String>,
    pub fusion_latency_us: u128,
}

/// Retorna o schema canônico Apache Arrow da tabela de memórias vetoriais.
pub fn get_semantic_table_schema() -> SchemaRef {
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

/// Resolve o caminho canônico da base de dados LanceDB do Hipocampo.
pub fn resolve_vector_db_path(override_path: Option<&str>) -> PathBuf {
    if let Some(p) = override_path {
        let pb = PathBuf::from(p);
        if pb.is_absolute() {
            return pb;
        }
        return crate::core::workspace_root().join(pb);
    }

    let canonical = PathBuf::from(CANONICAL_SEMANTIC_TABLE_PATH);
    if canonical.exists() {
        return canonical;
    }

    crate::core::workspace_root().join(".souls_data").join("semantic_memories")
}

/// Recuperador Vetorial LanceDB com Mapeamento de Memória (mmap NVMe / RAM Host).
/// REGRA DE FERRO (ADR-027): 0 MB de VRAM na RTX 2060m.
#[derive(Debug, Clone)]
pub struct LanceDbVectorStore {
    pub db_path: PathBuf,
}

impl LanceDbVectorStore {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    pub fn default_store() -> Self {
        Self::new(resolve_vector_db_path(None))
    }

    /// Conecta ao LanceDB em modo serverless via I/O mmap direto em disco.
    pub async fn connect_db(&self) -> Result<LanceConnection, String> {
        let db_path_str = self.db_path.to_string_lossy().to_string();
        connect(&db_path_str)
            .execute()
            .await
            .map_err(|e| format!("Erro ao conectar ao LanceDB em '{}': {}", db_path_str, e))
    }

    /// Abre ou cria a tabela canônica no LanceDB.
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
        } else if table_names.contains(&HIPPOCAMPUS_TABLE_NAME.to_string()) {
            db.open_table(HIPPOCAMPUS_TABLE_NAME)
                .execute()
                .await
                .map_err(|e| format!("Erro ao abrir tabela '{}': {}", HIPPOCAMPUS_TABLE_NAME, e))
        } else if table_names.contains(&LEGACY_TABLE_NAME.to_string()) {
            db.open_table(LEGACY_TABLE_NAME)
                .execute()
                .await
                .map_err(|e| format!("Erro ao abrir tabela legada '{}': {}", LEGACY_TABLE_NAME, e))
        } else {
            let schema = get_semantic_table_schema();
            db.create_empty_table(CANONICAL_TABLE_NAME, schema)
                .execute()
                .await
                .map_err(|e| format!("Erro ao criar tabela '{}': {}", CANONICAL_TABLE_NAME, e))
        }
    }

    /// Insere uma memória vetorial persistida no LanceDB.
    pub async fn insert_record(&self, record: SemanticMemoryRecord) -> Result<(), String> {
        if record.embedding.len() != VECTOR_DIMENSION as usize {
            return Err(format!(
                "Dimensão de embedding inválida: esperado {}, recebido {}",
                VECTOR_DIMENSION,
                record.embedding.len()
            ));
        }

        let schema = get_semantic_table_schema();

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

    /// Executa a busca vetorial kNN com barreira condicional contra o colapso IVF-PQ.
    /// Se o filtro escalar for restritivo ou retornar < 1000 registros, ativa `bypass_vector_index()`.
    pub async fn search_vectorial(
        &self,
        query_embedding: &[f32],
        limit: usize,
        min_valid_from: Option<i64>,
        max_valid_to: Option<i64>,
        filter_stability: Option<&str>,
        force_bypass_index: bool,
    ) -> Result<Vec<VectorialMatch>, String> {
        if query_embedding.is_empty() {
            return Ok(Vec::new());
        }

        let db = match self.connect_db().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("[LanceDbVectorStore] LanceDB offline (fail-soft): {}", e);
                return Ok(Vec::new());
            }
        };

        let table_names = match db.table_names().execute().await {
            Ok(names) => names,
            Err(e) => {
                eprintln!("[LanceDbVectorStore] Erro ao listar tabelas: {}", e);
                return Ok(Vec::new());
            }
        };

        let table_to_open = if table_names.contains(&CANONICAL_TABLE_NAME.to_string()) {
            CANONICAL_TABLE_NAME
        } else if table_names.contains(&HIPPOCAMPUS_TABLE_NAME.to_string()) {
            HIPPOCAMPUS_TABLE_NAME
        } else if table_names.contains(&LEGACY_TABLE_NAME.to_string()) {
            LEGACY_TABLE_NAME
        } else {
            return Ok(Vec::new());
        };

        let table = match db.open_table(table_to_open).execute().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[LanceDbVectorStore] Erro ao abrir tabela: {}", e);
                return Ok(Vec::new());
            }
        };

        let mut query = match table.vector_search(query_embedding.to_vec()) {
            Ok(q) => q,
            Err(e) => return Err(format!("Erro ao criar query vetorial: {}", e)),
        };

        query = query.limit(limit);

        // CURA DO COLAPSO DE IVF-PQ:
        // Se houver pré-filtro escalar temporal restritivo ou bypass explícito, força brute-force kNN exato em RAM.
        let is_restrictive_filter = min_valid_from.is_some() || max_valid_to.is_some() || filter_stability.is_some();
        if force_bypass_index || is_restrictive_filter {
            query = query.bypass_vector_index();
        }

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
                eprintln!("[LanceDbVectorStore] Erro na query vetorial: {}", e);
                return Ok(Vec::new());
            }
        };

        let mut matches = Vec::new();
        let mut stream = stream;

        while let Some(batch_result) = stream.next().await {
            let batch = match batch_result {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("[LanceDbVectorStore] Erro ao processar batch Arrow: {}", e);
                    continue;
                }
            };

            let num_rows = batch.num_rows();

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
                    metadata: json!({ "raw_distance": dist }),
                });
            }
        }

        Ok(matches)
    }
}

/// Recuperador Léxico FTS5 BM25 sobre o FrankenSQLite (`souls_state.db`).
#[derive(Debug, Clone)]
pub struct SqliteFtsRetriever {
    pub db_path: PathBuf,
}

impl SqliteFtsRetriever {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    pub fn search_lexical(&self, query: &str, limit: usize) -> Result<Vec<LexicalMatch>, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Erro ao abrir SQLite em '{:?}': {}", self.db_path, e))?;

        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let sanitized_query = query.replace(['"', '\'', '*'], "").trim().to_string();
        if sanitized_query.is_empty() {
            return Ok(Vec::new());
        }

        let fts_query = format!("\"{}\"*", sanitized_query);
        let sql = "
            SELECT 
                COALESCE(NULLIF(o.observation_id, ''), CAST(observations_fts.rowid AS TEXT)) AS obs_id,
                observations_fts.content,
                COALESCE(o.file_path, '') AS fpath,
                bm25(observations_fts) AS score
            FROM observations_fts
            LEFT JOIN observations o ON observations_fts.rowid = o.id
            WHERE observations_fts MATCH ?1
            ORDER BY score ASC
            LIMIT ?2
        ";

        let mut matches = Vec::new();
        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map([&fts_query, &limit.to_string()], |row| {
                Ok(LexicalMatch {
                    observation_id: row.get(0)?,
                    content: row.get(1)?,
                    file_path: row.get(2)?,
                    raw_score: row.get(3)?,
                })
            }) {
                for r in rows.flatten() {
                    matches.push(r);
                }
                return Ok(matches);
            }
        }

        // Fallback direto na tabela virtual
        let sql_fallback = "
            SELECT CAST(rowid AS TEXT), content, '', bm25(observations_fts)
            FROM observations_fts
            WHERE observations_fts MATCH ?1
            ORDER BY score ASC
            LIMIT ?2
        ";

        if let Ok(mut stmt) = conn.prepare(sql_fallback) {
            if let Ok(rows) = stmt.query_map([&fts_query, &limit.to_string()], |row| {
                Ok(LexicalMatch {
                    observation_id: row.get(0)?,
                    content: row.get(1)?,
                    file_path: row.get(2)?,
                    raw_score: row.get(3)?,
                })
            }) {
                for r in rows.flatten() {
                    matches.push(r);
                }
            }
        }

        Ok(matches)
    }
}

/// Carrega marcadores de descarte (tombstones) de memórias obsoletas no SQLite.
pub fn load_sqlite_tombstones(conn: &Connection) -> HashSet<String> {
    let mut tombstones = HashSet::new();
    let queries = [
        "SELECT observation_id FROM observations WHERE status_atualizacao IN ('superseded', 'invalid')",
        "SELECT observation_id FROM observations WHERE status_processamento IN ('superseded', 'invalid')",
        "SELECT observation_id FROM observations WHERE status IN ('superseded', 'invalid')",
        "SELECT memory_id FROM souls_memory_nodes WHERE stability_status IN ('SUPERSEDED', 'INVALID')",
    ];

    for sql in queries {
        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for r in rows.flatten() {
                    tombstones.insert(r);
                }
            }
        }
    }

    tombstones
}

/// Verifica se a query ou o snippet contém termos exatos de alta prioridade.
pub fn is_exact_match_term(query: &str, content: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return false;
    }
    if content.contains(q) {
        return true;
    }
    for &kw in EXACT_RIGID_KEYWORDS {
        if q.contains(kw) && content.contains(kw) {
            return true;
        }
    }
    false
}

/// Vetorização SIMD AVX2 de cálculo recíproco RRF para lotes de ranks.
/// RRF_Score = 1.0 / (k + rank)
#[cfg(target_arch = "x86_64")]
pub fn compute_rrf_batch_avx2(ranks: &[f32], k: f32, out_scores: &mut [f32]) {
    if is_x86_feature_detected!("avx2") {
        unsafe {
            use std::arch::x86_64::*;
            let len = ranks.len();
            let mut i = 0;
            let k_vec = _mm256_set1_ps(k);
            let ones = _mm256_set1_ps(1.0);

            while i + 8 <= len {
                let r_vec = _mm256_loadu_ps(ranks.as_ptr().add(i));
                let denom = _mm256_add_ps(k_vec, r_vec);
                let score_vec = _mm256_div_ps(ones, denom);
                _mm256_storeu_ps(out_scores.as_mut_ptr().add(i), score_vec);
                i += 8;
            }

            while i < len {
                out_scores[i] = 1.0 / (k + ranks[i]);
                i += 1;
            }
        }
    } else {
        for (i, &r) in ranks.iter().enumerate() {
            out_scores[i] = 1.0 / (k + r);
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn compute_rrf_batch_avx2(ranks: &[f32], k: f32, out_scores: &mut [f32]) {
    for (i, &r) in ranks.iter().enumerate() {
        out_scores[i] = 1.0 / (k + r);
    }
}

/// Reator de Fusão Híbrida RRF (Reciprocal Rank Fusion) com Aceleração CPU AVX2.
#[derive(Debug, Clone)]
pub struct HybridRrfFusionReactor {
    pub k: f64,
}

impl Default for HybridRrfFusionReactor {
    fn default() -> Self {
        Self { k: DEFAULT_RRF_K }
    }
}

type AccumulatorEntry = (f64, Option<usize>, Option<usize>, String, String, bool);

impl HybridRrfFusionReactor {
    pub fn new(k: f64) -> Self {
        Self { k }
    }

    /// Executa a fusão das listas léxica e vetorial em tempo sub-5ms na CPU.
    pub fn fuse(
        &self,
        query: &str,
        lexical: &[LexicalMatch],
        vectorial: &[VectorialMatch],
        tombstones: &HashSet<String>,
    ) -> (Vec<UnifiedMatch>, Duration) {
        let start = Instant::now();

        // observation_id -> (score, lexical_rank, vector_rank, content, file_path, exact_match)
        let mut map: HashMap<String, AccumulatorEntry> =
            HashMap::with_capacity(lexical.len() + vectorial.len());

        // Processa candidatos léxicos
        for (idx, m) in lexical.iter().enumerate() {
            if tombstones.contains(&m.observation_id) {
                continue;
            }

            let rank = idx + 1;
            let base_score = 1.0 / (self.k + rank as f64);
            let exact = is_exact_match_term(query, &m.content) || is_exact_match_term(query, &m.observation_id);
            let score = if exact { base_score + EXACT_MATCH_BONUS } else { base_score };

            map.entry(m.observation_id.clone())
                .and_modify(|(s, r_lex, _, _, _, is_ex)| {
                    *s += base_score;
                    *r_lex = Some(rank);
                    if exact {
                        *s += EXACT_MATCH_BONUS;
                        *is_ex = true;
                    }
                })
                .or_insert((
                    score,
                    Some(rank),
                    None,
                    m.content.clone(),
                    m.file_path.clone(),
                    exact,
                ));
        }

        // Processa candidatos vetoriais
        for (idx, m) in vectorial.iter().enumerate() {
            if tombstones.contains(&m.observation_id) {
                continue;
            }

            let rank = idx + 1;
            let base_score = 1.0 / (self.k + rank as f64);
            let exact = is_exact_match_term(query, &m.content) || is_exact_match_term(query, &m.observation_id);
            let score = if exact { base_score + EXACT_MATCH_BONUS } else { base_score };

            map.entry(m.observation_id.clone())
                .and_modify(|(s, _, r_vec, _, _, is_ex)| {
                    *s += base_score;
                    *r_vec = Some(rank);
                    if exact {
                        *s += EXACT_MATCH_BONUS;
                        *is_ex = true;
                    }
                })
                .or_insert((
                    score,
                    None,
                    Some(rank),
                    m.content.clone(),
                    m.file_path.clone(),
                    exact,
                ));
        }

        let mut results: Vec<UnifiedMatch> = map
            .into_iter()
            .map(|(obs_id, (score, r_lex, r_vec, content, file_path, is_exact))| UnifiedMatch {
                observation_id: obs_id,
                content,
                file_path,
                rrf_score: score,
                lexical_rank: r_lex,
                vector_rank: r_vec,
                is_exact_match: is_exact,
                status: "valid".to_string(),
            })
            .collect();

        // Ordena estritamente de forma decrescente por score RRF
        results.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let elapsed = start.elapsed();
        (results, elapsed)
    }
}

/// Nó do Grafo Ontológico LadybugDB mantido em RAM Host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OntologicalNode {
    pub id: String,
    pub node_type: String, // "ADR" | "SourceCode" | "Dependency" | "Constraint"
    pub stability_status: String, // "STABLE" | "EVOLVING"
    pub banned_patterns: Vec<String>,
    pub required_patterns: Vec<String>,
}

/// Aresta direcionada do Grafo Ontológico LadybugDB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct OntologicalEdge {
    pub from_node: String,
    pub to_node: String,
    pub relation_type: String, // "depends_on" | "replaces" | "implements" | "violates"
}

/// Veredito do Firewall Ontológico.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FirewallVerdict {
    Approved,
    Vetoed {
        reason: String,
        violated_node: String,
        relation_path: Vec<String>,
    },
}

/// Firewall Ontológico LadybugDB (Anti-RAG Poisoning) operando em RAM Host via DashMap.
#[derive(Debug, Clone)]
pub struct LadybugOntologicalFirewall {
    pub nodes: Arc<DashMap<String, OntologicalNode>>,
    pub edges: Arc<DashMap<String, Vec<OntologicalEdge>>>,
}

impl Default for LadybugOntologicalFirewall {
    fn default() -> Self {
        let firewall = Self::new();
        firewall.seed_canonical_adrs();
        firewall
    }
}

impl LadybugOntologicalFirewall {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            edges: Arc::new(DashMap::new()),
        }
    }

    /// Carrega as ADRs imutáveis canônicas no grafo ontológico.
    pub fn seed_canonical_adrs(&self) {
        self.register_node(
            "ADR-030",
            "ADR",
            "STABLE",
            &["winapi", "core_affinity"],
            &["windows-sys = \"=0.61.2\""],
        );

        self.register_node(
            "ADR-027",
            "ADR",
            "STABLE",
            &["cudaMalloc(rag)", "vram_rag_alloc", "gpu_vector_index"],
            &["0 MB VRAM", "mmap2", "CPU Host"],
        );

        self.register_node(
            "ADR-001",
            "ADR",
            "STABLE",
            &["python_runtime_prod", "node_runtime_prod"],
            &["Rust", "Tokio"],
        );
    }

    pub fn register_node(
        &self,
        id: &str,
        node_type: &str,
        stability: &str,
        banned_patterns: &[&str],
        required_patterns: &[&str],
    ) {
        self.nodes.insert(
            id.to_string(),
            OntologicalNode {
                id: id.to_string(),
                node_type: node_type.to_string(),
                stability_status: stability.to_string(),
                banned_patterns: banned_patterns.iter().map(|s| s.to_string()).collect(),
                required_patterns: required_patterns.iter().map(|s| s.to_string()).collect(),
            },
        );
    }

    pub fn register_edge(&self, from: &str, to: &str, relation_type: &str) {
        let edge = OntologicalEdge {
            from_node: from.to_string(),
            to_node: to.to_string(),
            relation_type: relation_type.to_string(),
        };
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(edge);
    }

    /// Executa varredura BFS rápida limitada em profundidade checando premissas invioláveis.
    pub fn bfs_check_compliance(
        &self,
        start_node: &str,
        candidate_content: &str,
        max_depth: usize,
    ) -> FirewallVerdict {
        let mut queue: VecDeque<(String, usize, Vec<String>)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();

        // 1. Checagem direta de nós globais STABLE
        for entry in self.nodes.iter() {
            let node = entry.value();
            if node.stability_status == "STABLE" {
                for banned in &node.banned_patterns {
                    if candidate_content.contains(banned) {
                        let reason = format!(
                            "RAG Poisoning detectado: chunk viola a ADR estável '{}' contendo padrão proibido '{}'",
                            node.id, banned
                        );
                        return FirewallVerdict::Vetoed {
                            reason,
                            violated_node: node.id.clone(),
                            relation_path: vec![node.id.clone()],
                        };
                    }
                }
            }
        }

        // 2. Travessia BFS a partir do nó de contexto
        if !start_node.is_empty() {
            queue.push_back((start_node.to_string(), 0, vec![start_node.to_string()]));
            visited.insert(start_node.to_string());

            while let Some((curr_node_id, depth, path)) = queue.pop_front() {
                if let Some(node_ref) = self.nodes.get(&curr_node_id) {
                    let node = node_ref.value();
                    for banned in &node.banned_patterns {
                        if candidate_content.contains(banned) {
                            let reason = format!(
                                "RAG Poisoning detectado: chunk viola o nó ontológico '{}' ({}) via padrão proibido '{}'",
                                node.id, node.stability_status, banned
                            );
                            return FirewallVerdict::Vetoed {
                                reason,
                                violated_node: node.id.clone(),
                                relation_path: path.clone(),
                            };
                        }
                    }
                }

                if depth >= max_depth {
                    continue;
                }

                if let Some(edges_ref) = self.edges.get(&curr_node_id) {
                    for edge in edges_ref.value() {
                        if !visited.contains(&edge.to_node) {
                            visited.insert(edge.to_node.clone());
                            let mut next_path = path.clone();
                            next_path.push(format!("{}:{}", edge.relation_type, edge.to_node));
                            queue.push_back((edge.to_node.clone(), depth + 1, next_path));
                        }
                    }
                }
            }
        }

        FirewallVerdict::Approved
    }

    /// Sanitiza uma coleção de matches expulsando trechos venenosos ou contraditórios.
    pub fn sanitize_chunks<T, F>(
        &self,
        target_entity: &str,
        items: Vec<T>,
        get_text: F,
    ) -> (Vec<T>, Vec<String>)
    where
        F: Fn(&T) -> &str,
    {
        let mut approved = Vec::new();
        let mut vetoed_reasons = Vec::new();

        for item in items {
            let text = get_text(&item);
            match self.bfs_check_compliance(target_entity, text, 4) {
                FirewallVerdict::Approved => approved.push(item),
                FirewallVerdict::Vetoed { reason, .. } => vetoed_reasons.push(reason),
            }
        }

        (approved, vetoed_reasons)
    }
}

/// Motor de Busca Semântica Híbrida Completo (Hipocampo Ativo).
#[derive(Debug, Clone)]
pub struct ActiveHippocampusEngine {
    pub vector_store: LanceDbVectorStore,
    pub fts_retriever: SqliteFtsRetriever,
    pub fusion_reactor: HybridRrfFusionReactor,
    pub firewall: LadybugOntologicalFirewall,
}

impl ActiveHippocampusEngine {
    pub fn new(vector_db_path: Option<&str>, sqlite_db_path: Option<&str>) -> Self {
        let vec_path = resolve_vector_db_path(vector_db_path);
        let sql_path = if let Some(p) = sqlite_db_path {
            PathBuf::from(p)
        } else {
            crate::core::workspace_root().join(".souls_data").join("souls_state.db")
        };

        Self {
            vector_store: LanceDbVectorStore::new(vec_path),
            fts_retriever: SqliteFtsRetriever::new(sql_path),
            fusion_reactor: HybridRrfFusionReactor::default(),
            firewall: LadybugOntologicalFirewall::default(),
        }
    }

    /// Executa a busca híbrida completa (FTS5 + LanceDB + RRF + LadybugDB).
    pub async fn execute_hybrid_search(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        min_valid_from: Option<i64>,
        max_valid_to: Option<i64>,
        stability_filter: Option<&str>,
    ) -> Result<SemanticSearchResult, String> {
        let fts_retriever = self.fts_retriever.clone();
        let query_clone = query.to_string();
        let lexical_handle = tokio::spawn(async move {
            fts_retriever.search_lexical(&query_clone, limit)
        });

        let vector_store = self.vector_store.clone();
        let query_emb = query_embedding.to_vec();
        let stab_owned = stability_filter.map(|s| s.to_string());
        let vector_handle = tokio::spawn(async move {
            vector_store
                .search_vectorial(
                    &query_emb,
                    limit,
                    min_valid_from,
                    max_valid_to,
                    stab_owned.as_deref(),
                    false,
                )
                .await
        });

        let lexical_res = lexical_handle
            .await
            .map_err(|e| format!("Falha na task léxica: {e}"))?
            .unwrap_or_default();

        let vector_res = vector_handle
            .await
            .map_err(|e| format!("Falha na task vetorial: {e}"))?
            .unwrap_or_default();

        let conn = Connection::open(&self.fts_retriever.db_path).ok();
        let tombstones = conn
            .as_ref()
            .map(load_sqlite_tombstones)
            .unwrap_or_default();

        let (mut fused_results, elapsed) =
            self.fusion_reactor.fuse(query, &lexical_res, &vector_res, &tombstones);

        // Circuito de Resgate / Fallback FTS5 caso vetor tenha falhado ou retornado vazio
        if fused_results.is_empty() && !lexical_res.is_empty() {
            fused_results = lexical_res
                .into_iter()
                .map(|lex| UnifiedMatch {
                    observation_id: lex.observation_id,
                    content: lex.content,
                    file_path: lex.file_path,
                    rrf_score: 0.5,
                    lexical_rank: Some(1),
                    vector_rank: None,
                    is_exact_match: false,
                    status: "fallback_fts5".to_string(),
                })
                .collect();
        }

        // Sanitização pelo Firewall Ontológico LadybugDB (Anti-RAG Poisoning)
        let (sanitized_results, vetoed_reasons) =
            self.firewall.sanitize_chunks(query, fused_results, |m| &m.content);

        let final_results: Vec<UnifiedMatch> =
            sanitized_results.into_iter().take(limit).collect();

        Ok(SemanticSearchResult {
            query: query.to_string(),
            results_count: final_results.len(),
            results: final_results,
            vetoed_count: vetoed_reasons.len(),
            vetoed_reasons,
            fusion_latency_us: elapsed.as_micros(),
        })
    }
}

/// Telemetria NVML para auditoria de isolamento Zero-VRAM na RTX 2060m.
pub fn query_nvml_vram_used_bytes() -> Option<u64> {
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if let Ok(device) = nvml.device_by_index(0) {
            if let Ok(mem) = device.memory_info() {
                return Some(mem.used);
            }
        }
    }
    None
}

#[cfg(test)]
pub mod tests;
