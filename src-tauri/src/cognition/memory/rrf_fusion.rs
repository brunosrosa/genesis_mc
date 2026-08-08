use super::fts_retriever::LexicalMatch;
use super::vector_retriever::VectorialMatch;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Constante de suavização padrão do RRF (Reciprocal Rank Fusion)
pub const DEFAULT_RRF_K: f64 = 60.0;

/// Resultado unificado da fusão RRF contendo pontuação combinada e ranks parciais.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedMatch {
    pub observation_id: String,
    pub content: String,
    pub file_path: String,
    pub rrf_score: f64,
    pub lexical_rank: Option<usize>,
    pub vector_rank: Option<usize>,
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

/// Motor de fusão matemática RRF com filtro de invalidação JIT (Tombstone).
#[derive(Debug, Clone)]
pub struct RrfFusionEngine {
    pub k: f64,
}

impl Default for RrfFusionEngine {
    fn default() -> Self {
        Self::new(DEFAULT_RRF_K)
    }
}

type RrfAccumulatorEntry = (f64, Option<usize>, Option<usize>, String, String);

impl RrfFusionEngine {
    /// Instancia o motor RRF com parâmetro de suavização `k`.
    pub fn new(k: f64) -> Self {
        Self { k }
    }

    /// Executa a fusão das listas léxica e vetorial em O(N) e aplica a invalidação JIT.
    pub fn fuse(
        &self,
        lexical: &[LexicalMatch],
        vectorial: &[VectorialMatch],
        tombstones: &HashSet<String>,
    ) -> Vec<UnifiedMatch> {
        // Map: observation_id -> (rrf_score, lexical_rank, vector_rank, content, file_path)
        let mut map: HashMap<String, RrfAccumulatorEntry> = HashMap::new();

        // Processa candidatos léxicos
        for (idx, m) in lexical.iter().enumerate() {
            if tombstones.contains(&m.observation_id) {
                // Invalidação JIT Tombstone em O(1)
                continue;
            }

            let rank = idx + 1;
            let score = 1.0 / (self.k + rank as f64);

            map.entry(m.observation_id.clone())
                .and_modify(|(s, r_lex, _, _, _)| {
                    *s += score;
                    *r_lex = Some(rank);
                })
                .or_insert((
                    score,
                    Some(rank),
                    None,
                    m.content.clone(),
                    m.file_path.clone(),
                ));
        }

        // Processa candidatos vetoriais
        for (idx, m) in vectorial.iter().enumerate() {
            if tombstones.contains(&m.observation_id) {
                // Invalidação JIT Tombstone em O(1)
                continue;
            }

            let rank = idx + 1;
            let score = 1.0 / (self.k + rank as f64);

            map.entry(m.observation_id.clone())
                .and_modify(|(s, _, r_vec, _, _)| {
                    *s += score;
                    *r_vec = Some(rank);
                })
                .or_insert((
                    score,
                    None,
                    Some(rank),
                    m.content.clone(),
                    m.file_path.clone(),
                ));
        }

        // Converte para lista unificada
        let mut results: Vec<UnifiedMatch> = map
            .into_iter()
            .map(|(obs_id, (score, r_lex, r_vec, content, file_path))| UnifiedMatch {
                observation_id: obs_id,
                content,
                file_path,
                rrf_score: score,
                lexical_rank: r_lex,
                vector_rank: r_vec,
                status: "valid".to_string(),
            })
            .collect();

        // Ordenação decrescente por RRF_Score
        results.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }
}
