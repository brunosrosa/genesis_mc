//! Feedback: Telemetria FinOps e eficiencia E3.
//!
//! A **eficiencia E3** mede a fracao de tokens que NAO foram consumidos
//! em output (resposta crua) versus o total (input + output). Ferramentas
//! como `compress` e `dedup` devem ter E3 alto (poupam tokens); tools de
//! output cru como `read` tem E3 baixo.
//!
//! Formula:
//!
//! ```text
//! E3 = 1 - (tokens_out / max(1, tokens_in + tokens_out))
//! ```
//!
//! E3 ∈ [0, 1] onde 1.0 = maximo de economia (nenhum token desperdiçado).
//!
//! Toda a leitura e uma unica query SQL agregada (GROUP BY tool). O
//! relatorio final contem o total, o E3 global e a decomposicao por tool.

use crate::cognition::memory_graph::errors::CognitiveError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Calcula a eficiencia E3 a partir de tokens brutos.
///
/// Retorna um valor em [0.0, 1.0]. E3 = 1.0 quando `tokens_out == 0`
/// (toda a operacao foi em input). E3 = 0.0 quando o output consome
/// 100% do total.
///
/// Robusto contra divisao por zero: `max(1, in + out)` no denominador.
#[inline]
pub fn e3_efficiency(tokens_in: i64, tokens_out: i64) -> f64 {
    let total = (tokens_in.max(0) + tokens_out.max(0)) as f64;
    if total <= 0.0 {
        return 1.0;
    }
    let out = tokens_out.max(0) as f64;
    1.0 - (out / total)
}

/// Marco 3.8 Fase C.1: eficiencia E3 constitucional.
///
/// Formula canonica do ADR-043 v1.1 (SODA canon matematico):
///
/// ```text
/// E3 = (accuracy_score * accuracy_score) / max(1.0, duration_ms)
/// ```
///
/// Penaliza duplamente a degradacao sintatica (quadrado da acuracia) e
/// recompensa a velocidade (divisao pela duracao em ms, com piso de 1.0
/// para evitar explosao quando `duration_ms == 0`).
///
/// Para agregacao, callers devem aplicar `e3_efficiency_v2(avg_acc, sum_dur)`
/// sobre valores pre-agregados (accuracy media, duracao somada).
#[inline]
pub fn e3_efficiency_v2(accuracy_score: f64, duration_ms: f64) -> f64 {
    let acc = accuracy_score.clamp(0.0, 1.0);
    let dur = duration_ms.max(0.0);
    (acc * acc) / dur.max(1.0)
}

/// Entrada por tool no relatorio FinOps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ToolTelemetry {
    pub tool: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub duration_ms_total: i64,
    pub calls: i64,
    /// Marco 3.7 Fase B: Eficiencia E3 token-based (1 - out/total).
    pub e3_efficiency: f64,
    /// Marco 3.8 Fase C.1: Acuracia sintatica media (0.0-1.0).
    pub accuracy_score_avg: f64,
    /// Marco 3.8 Fase C.1: Eficiencia E3 constitucional
    /// = `(acc^2) / max(1.0, duration_ms_total)`.
    pub e3_efficiency_v2: f64,
}

/// Relatorio FinOps agregado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TelemetryReport {
    pub total_tokens_in: i64,
    pub total_tokens_out: i64,
    pub total_cost_usd: f64,
    pub total_duration_ms: i64,
    pub total_calls: i64,
    /// Eficiencia E3 global (agregada sobre o somatorio).
    pub e3_efficiency: f64,
    /// Marco 3.8 Fase C.1: acuracia sintatica media global (0.0-1.0).
    pub accuracy_score_avg: f64,
    /// Marco 3.8 Fase C.1: Eficiencia E3 constitucional global.
    pub e3_efficiency_v2: f64,
    /// Decomposicao por tool (ordenada por nome).
    pub by_tool: BTreeMap<String, ToolTelemetry>,
}

/// Le `telemetry_logs` e produz o relatorio agregado.
///
/// Query SQL unica (`GROUP BY tool`); computacao de E3 por tool feita em
/// RAM apos a hidratacao. Para milhoes de logs, o caller deve paginar
/// via `LIMIT` antes de invocar esta funcao.
pub fn aggregate_telemetry(conn: &Connection) -> Result<TelemetryReport, CognitiveError> {
    let mut stmt = conn
        .prepare(
            "SELECT tool, \
                    COALESCE(SUM(tokens_in), 0), \
                    COALESCE(SUM(tokens_out), 0), \
                    COALESCE(SUM(cost_usd), 0.0), \
                    COALESCE(SUM(duration_ms), 0), \
                    COALESCE(AVG(accuracy_score), 1.0), \
                    COUNT(*) \
             FROM telemetry_logs \
             GROUP BY tool \
             ORDER BY tool ASC",
        )
        .map_err(CognitiveError::from)?;
    let mut rows = stmt.query([]).map_err(CognitiveError::from)?;

    let mut by_tool: BTreeMap<String, ToolTelemetry> = BTreeMap::new();
    let mut total_in: i64 = 0;
    let mut total_out: i64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut total_dur: i64 = 0;
    let mut total_calls: i64 = 0;
    // Acumulador de acuracia media ponderada: soma(acc_i * calls_i) / soma(calls_i).
    let mut total_acc_weighted_sum: f64 = 0.0;

    while let Some(row) = rows.next().map_err(CognitiveError::from)? {
        let tool: String = row.get(0).map_err(CognitiveError::from)?;
        let tin: i64 = row.get(1).map_err(CognitiveError::from)?;
        let tout: i64 = row.get(2).map_err(CognitiveError::from)?;
        let cost: f64 = row.get(3).map_err(CognitiveError::from)?;
        let dur: i64 = row.get(4).map_err(CognitiveError::from)?;
        let acc_avg: f64 = row.get(5).map_err(CognitiveError::from)?;
        let calls: i64 = row.get(6).map_err(CognitiveError::from)?;

        let e3 = e3_efficiency(tin, tout);
        let e3_v2 = e3_efficiency_v2(acc_avg, dur as f64);
        by_tool.insert(
            tool.clone(),
            ToolTelemetry {
                tool,
                tokens_in: tin,
                tokens_out: tout,
                cost_usd: cost,
                duration_ms_total: dur,
                calls,
                e3_efficiency: e3,
                accuracy_score_avg: acc_avg,
                e3_efficiency_v2: e3_v2,
            },
        );
        total_in += tin;
        total_out += tout;
        total_cost += cost;
        total_dur += dur;
        total_calls += calls;
        total_acc_weighted_sum += acc_avg * (calls as f64);
    }

    let e3_global = e3_efficiency(total_in, total_out);
    // Acuracia media global: media ponderada por numero de chamadas.
    let acc_global = if total_calls > 0 {
        total_acc_weighted_sum / (total_calls as f64)
    } else {
        1.0
    };
    let e3_v2_global = e3_efficiency_v2(acc_global, total_dur as f64);

    Ok(TelemetryReport {
        total_tokens_in: total_in,
        total_tokens_out: total_out,
        total_cost_usd: total_cost,
        total_duration_ms: total_dur,
        total_calls,
        e3_efficiency: e3_global,
        accuracy_score_avg: acc_global,
        e3_efficiency_v2: e3_v2_global,
        by_tool,
    })
}
