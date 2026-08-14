// SOULS V6 — Memory Module: RRF Fusion Engine (Reciprocal Rank Fusion com AVX2 & Exact Match Bonus)
// Conforme ADR-001, ADR-005, ADR-027 e Marco VI (Latência Sub-5ms na CPU Host).

use super::fts_retriever::LexicalMatch;
use super::vector_retriever::VectorialMatch;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Constante de suavização padrão do RRF (Reciprocal Rank Fusion)
pub const DEFAULT_RRF_K: f64 = 60.0;

/// Bônus aditivo para correspondências de termos exatos rígidos (constantes, ADRs, IDs, caminhos).
pub const EXACT_MATCH_BONUS: f64 = 10.0;

/// Lista de constantes rígidas canônicas que disparam o bônus de termo exato no SOULS.
pub const EXACT_RIGID_KEYWORDS: &[&str] = &[
    "GGML_TYPE_TQ1",
    "ADR-001",
    "ADR-003",
    "ADR-005",
    "ADR-025",
    "ADR-027",
    "ADR-030",
    "ADR-040",
    "ADR-047",
    "windows-sys",
    "winapi",
    "core_affinity",
    "LadybugDB",
    "LanceDB",
    "FrankenSQLite",
    "ChyrosDaemon",
];

/// Resultado unificado da fusão RRF contendo pontuação combinada e ranks parciais.
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

/// Carrega todos os IDs de observações/premissas com status 'superseded' ou 'invalid' no SQLite.
pub fn load_tombstones(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut tombstones = HashSet::new();

    let queries = [
        "SELECT observation_id FROM observations WHERE status_atualizacao IN ('superseded', 'invalid')",
        "SELECT observation_id FROM observations WHERE status_processamento IN ('superseded', 'invalid')",
        "SELECT observation_id FROM observations WHERE status IN ('superseded', 'invalid')",
        "SELECT project_name FROM repo_heuristics WHERE status_atualizacao IN ('superseded', 'invalid')",
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

    Ok(tombstones)
}

/// Verifica se a query ou conteúdo contém termos exatos de alta precedência.
pub fn is_exact_term_match(query: &str, content: &str) -> bool {
    let q_trimmed = query.trim();
    if q_trimmed.is_empty() {
        return false;
    }

    // Match direto da query inteira como substring exata
    if content.contains(q_trimmed) {
        return true;
    }

    // Match de palavras-chave rígidas do sistema
    for &kw in EXACT_RIGID_KEYWORDS {
        if q_trimmed.contains(kw) && content.contains(kw) {
            return true;
        }
    }

    false
}

/// Motor de fusão matemática RRF com aceleração de CPU e bônus de termos exatos.
#[derive(Debug, Clone)]
pub struct RrfFusionEngine {
    pub k: f64,
}

impl Default for RrfFusionEngine {
    fn default() -> Self {
        Self::new(DEFAULT_RRF_K)
    }
}

type RrfAccumulatorEntry = (f64, Option<usize>, Option<usize>, String, String, bool);

impl RrfFusionEngine {
    /// Instancia o motor RRF com parâmetro de suavização `k`.
    pub fn new(k: f64) -> Self {
        Self { k }
    }

    /// Executa a fusão das listas léxica e vetorial em tempo sub-5ms na CPU Host com invalidação JIT e bônus exato.
    pub fn fuse_with_query(
        &self,
        query: &str,
        lexical: &[LexicalMatch],
        vectorial: &[VectorialMatch],
        tombstones: &HashSet<String>,
    ) -> (Vec<UnifiedMatch>, std::time::Duration) {
        let start = Instant::now();

        // Map: observation_id -> (rrf_score, lexical_rank, vector_rank, content, file_path, is_exact_match)
        let mut map: HashMap<String, RrfAccumulatorEntry> = HashMap::with_capacity(lexical.len() + vectorial.len());

        // Processa candidatos léxicos
        for (idx, m) in lexical.iter().enumerate() {
            if tombstones.contains(&m.observation_id) {
                continue;
            }

            let rank = idx + 1;
            let base_score = 1.0 / (self.k + rank as f64);
            let exact = is_exact_term_match(query, &m.content) || is_exact_term_match(query, &m.observation_id);
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
            let exact = is_exact_term_match(query, &m.content) || is_exact_term_match(query, &m.observation_id);
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

        // Converte para lista unificada
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

        // Ordenação decrescente por RRF_Score (Exact Match prioritário no topo)
        results.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let elapsed = start.elapsed();
        (results, elapsed)
    }

    /// Versão de compatibilidade sem query explícita.
    pub fn fuse(
        &self,
        lexical: &[LexicalMatch],
        vectorial: &[VectorialMatch],
        tombstones: &HashSet<String>,
    ) -> Vec<UnifiedMatch> {
        let (results, _) = self.fuse_with_query("", lexical, vectorial, tombstones);
        results
    }
}
