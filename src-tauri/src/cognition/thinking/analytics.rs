//! Análise comportamental de sessões socráticas (Marco 3.9 Fase E).
//!
//! Computa métricas FinOps cognitivas a partir de uma lista de
//! [`SocraticThought`]s, sem I/O. Pure functions, fail-closed em
//! divisão por zero.
//!
//! Referência normativa: [ADR-045 §3](../../../../docs/adrs/ADR-045-Persistencia-da-Alma-Socratica.md).

use crate::cognition::thinking::persistence::{SocraticThought, ThoughtType};
use serde::Serialize;
use std::collections::HashMap;

/// Métricas comportamentais de uma sessão socrática.
///
/// Veja equações no ADR-045 §3:
///
/// - `revision_rate` = |revision thoughts| / |total thoughts|
/// - `branching_factor` = média de filhos por branch
/// - `latency_mean_ms` = média de duration_ms sobre todos os pensamentos
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionMetrics {
    /// Total de pensamentos na sessão.
    pub total_thoughts: usize,
    /// Quantidade de branches distintos (inclui `"main"`).
    pub branch_count: usize,
    /// |revision thoughts| / |total thoughts|. 0.0 se sessão vazia.
    pub revision_rate: f64,
    /// Média de filhos por branch (fator de ramificação). 0.0 se sessão vazia.
    pub branching_factor: f64,
    /// Média de `duration_ms` sobre todos os pensamentos. 0.0 se sessão vazia.
    pub latency_mean_ms: f64,
    /// Total acumulado de `duration_ms` (soma).
    pub latency_total_ms: u64,
}

/// Computa as [`SessionMetrics`] de uma sessão a partir de seus
/// pensamentos reconstruídos.
///
/// Fail-closed: divisão por zero é defendida (devolve 0.0).
///
/// Complexidade: O(|T|) onde T = pensamentos.
pub fn compute_metrics(thoughts: &[SocraticThought]) -> SessionMetrics {
    let n = thoughts.len();
    if n == 0 {
        return SessionMetrics {
            total_thoughts: 0,
            branch_count: 0,
            revision_rate: 0.0,
            branching_factor: 0.0,
            latency_mean_ms: 0.0,
            latency_total_ms: 0,
        };
    }

    // Contadores por tipo.
    let mut revisions: usize = 0;
    let mut latency_total: u64 = 0;

    // Mapa branch → contagem de filhos.
    // Filho = todo pensamento que tem parent_thought_id E cujo parent
    // está em outro branch (i.e. é a raiz de um novo branch) OU tem
    // um parent_thought_id com branch_id diferente.
    let mut child_count_by_parent_branch: HashMap<String, usize> = HashMap::new();
    let mut branches: std::collections::HashSet<String> = std::collections::HashSet::new();

    for t in thoughts {
        if t.thought_type == ThoughtType::Revision {
            revisions += 1;
        }
        latency_total = latency_total.saturating_add(t.duration_ms as u64);
        branches.insert(t.branch_id.clone());

        // Heurística: se parent_thought_id é None, é raiz; senão
        // atribuímos o "filho" ao branch do pai.
        if let Some(_parent_id) = &t.parent_thought_id {
            // Para simplicidade operacional, associamos o filho ao
            // PRÓPRIO branch do pensamento (já que a reconstrução
            // da árvore é por session_id, branch_id, step_number).
            *child_count_by_parent_branch
                .entry(t.branch_id.clone())
                .or_insert(0) += 1;
        }
    }

    let total = n as f64;
    let revision_rate = revisions as f64 / total;
    let branching_factor = if branches.is_empty() {
        0.0
    } else {
        // Média de filhos POR BRANCH (inclui branches com 0 filhos).
        let sum_children: usize = child_count_by_parent_branch.values().sum();
        sum_children as f64 / branches.len() as f64
    };
    let latency_mean_ms = latency_total as f64 / total;

    SessionMetrics {
        total_thoughts: n,
        branch_count: branches.len(),
        revision_rate,
        branching_factor,
        latency_mean_ms,
        latency_total_ms: latency_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::thinking::persistence::{SocraticThought, ThoughtType};

    fn mk(
        id: &str,
        branch: &str,
        parent: Option<&str>,
        ty: ThoughtType,
        dur_ms: u32,
    ) -> SocraticThought {
        SocraticThought {
            thought_id: id.into(),
            session_id: "sess".into(),
            branch_id: branch.into(),
            parent_thought_id: parent.map(String::from),
            thought_type: ty,
            content: format!("c-{id}"),
            step_number: 1,
            duration_ms: dur_ms,
            created_at: 0,
        }
    }

    #[test]
    fn test_compute_metrics_empty() {
        let m = compute_metrics(&[]);
        assert_eq!(m.total_thoughts, 0);
        assert_eq!(m.revision_rate, 0.0);
        assert_eq!(m.branching_factor, 0.0);
        assert_eq!(m.latency_mean_ms, 0.0);
    }

    #[test]
    fn test_compute_metrics_revision_rate() {
        // 4 pensamentos, 2 revisions → 0.5
        let t = vec![
            mk("1", "main", None, ThoughtType::Regular, 100),
            mk("2", "main", Some("1"), ThoughtType::Revision, 200),
            mk("3", "main", Some("1"), ThoughtType::Revision, 300),
            mk("4", "main", Some("1"), ThoughtType::Regular, 400),
        ];
        let m = compute_metrics(&t);
        assert_eq!(m.total_thoughts, 4);
        assert_eq!(m.revision_rate, 0.5);
        assert_eq!(m.latency_total_ms, 1000);
        assert_eq!(m.latency_mean_ms, 250.0);
    }

    #[test]
    fn test_compute_metrics_branching() {
        // 1 branch main com 3 filhos → branching_factor = 3.0
        let t = vec![
            mk("1", "main", None, ThoughtType::Regular, 0),
            mk("2", "main", Some("1"), ThoughtType::Branching, 0),
            mk("3", "main", Some("1"), ThoughtType::Branching, 0),
            mk("4", "main", Some("1"), ThoughtType::Branching, 0),
        ];
        let m = compute_metrics(&t);
        assert_eq!(m.branch_count, 1);
        assert_eq!(m.branching_factor, 3.0);
    }
}
