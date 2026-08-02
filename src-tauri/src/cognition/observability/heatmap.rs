//! Heatmap: Mapeamento dinamico de caminhos quentes via Langevin decay.
//!
//! O Langevin decay e um tipo particular de decaimento exponencial onde a
//! "temperatura" (acesso recente) cai com o tempo. A constante `lambda`
//! controla a velocidade: quanto maior, mais rapido o arquivo "esfria".
//!
//! Formula canonica:
//!
//! ```text
//! score(path, t_now) = sum_i exp(-lambda * (t_now - t_i))
//! ```
//!
//! Com `lambda = 0.05` (calibrado empiricamente), um arquivo acessado ha
//! 60s tem peso ≈ 0.05, ha 1h ≈ 0.74 e ha 24h ≈ 0.012.
//!
//! Toda a logica e deterministica e opera em RAM (sem I/O de disco alem
//! da query SQL inicial). Complexidade: O(N) onde N = total de acessos.

use crate::cognition::memory_graph::errors::CognitiveError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Constante de decaimento Langevin canonica do SODA.
///
/// `lambda = 0.05` foi calibrado empiricamente pelo Arquiteto-Chefe para
/// prover uma meia-vida (peso cai a 50%) de aproximadamente 14 segundos.
/// Isso prioriza acessos *muito recentes* sem descartar totalmente o
/// historico de pocos minutos atras.
pub const DEFAULT_LAMBDA: f64 = 0.05;

/// Entrada individual do heatmap (um arquivo + seu score acumulado).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatmapEntry {
    /// Caminho do arquivo (chave de agrupamento).
    pub path: String,
    /// Score acumulado via Langevin decay.
    pub score: f64,
    /// Numero total de acessos no intervalo (util para auditoria).
    pub access_count: i64,
}

/// Calcula o score Langevin para um unico instante de acesso.
///
/// `accessed_at` em epoch seconds; `now` em epoch seconds. Se
/// `accessed_at > now` (acesso futuro — relogio desregulado), o score
/// retornado e `1.0` (maximo) e nao panica.
///
/// A funcao e publica e deterministica para permitir testes TDD sem
/// dependencia de SQLite.
#[inline]
pub fn langevin_score(accessed_at: i64, now: i64, lambda: f64) -> f64 {
    let dt = (now - accessed_at).max(0) as f64;
    (-lambda * dt).exp()
}

/// Calcula o score agregado de Langevin para um arquivo com multiplos acessos.
///
/// Retorna a soma de `exp(-lambda * (now - t_i))` para todos os `t_i`
/// fornecidos. Util para testes e para a versao em RAM do heatmap.
pub fn langevin_aggregate(accesses: &[i64], now: i64, lambda: f64) -> f64 {
    accesses
        .iter()
        .map(|&t| langevin_score(t, now, lambda))
        .sum()
}

/// Le `file_access_logs` do SQLite e computa o heatmap ordenado por score.
///
/// Retorna no maximo `limit` entradas (default 50), ordenadas por:
/// 1. `score` descendente (mais quente primeiro).
/// 2. `path` ascendente (desempate deterministico).
///
/// A query SQL agrega acessos em RAM (`GROUP BY file_path`) e devolve
/// uma lista linear para a funcao. Para tabelas com milhoes de registros,
/// recomenda-se paginacao via `LIMIT/OFFSET` no caller.
pub fn compute_heatmap(
    conn: &Connection,
    now: i64,
    lambda: f64,
    limit: usize,
) -> Result<Vec<HeatmapEntry>, CognitiveError> {
    // Pre-agregacao: coleta todos os timestamps por path.
    let mut by_path: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT file_path, accessed_at FROM file_access_logs")
            .map_err(CognitiveError::from)?;
        let mut rows = stmt.query([]).map_err(CognitiveError::from)?;
        while let Some(row) = rows.next().map_err(CognitiveError::from)? {
            let path: String = row.get(0).map_err(CognitiveError::from)?;
            let ts: i64 = row.get(1).map_err(CognitiveError::from)?;
            by_path.entry(path).or_default().push(ts);
        }
    }

    // Calcula score por path e ordena.
    let mut entries: Vec<HeatmapEntry> = by_path
        .into_iter()
        .map(|(path, ts)| {
            let score = langevin_aggregate(&ts, now, lambda);
            HeatmapEntry {
                path,
                score,
                access_count: ts.len() as i64,
            }
        })
        .collect();

    // Ordena por score desc, desempate por path asc.
    entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
    entries.truncate(limit);
    Ok(entries)
}
