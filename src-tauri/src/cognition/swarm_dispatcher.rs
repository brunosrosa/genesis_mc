use crate::finops::iron_cost::{IronCostBreaker, ModelTier, FinOpsError};
use thiserror::Error;
use tokio::join;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SwarmError {
    #[error("Falha no disjuntor FinOps: {0}")]
    FinOpsBlocked(#[from] FinOpsError),
    #[error("Falha na persistência do debate: {0}")]
    PersistenceError(String),
}

pub struct CognitiveSwarmDispatcher;

use crate::cognition::sgr_synthesizer::SwarmDebate;

impl CognitiveSwarmDispatcher {
    pub async fn dispatch_swarm(
        repo_id: &str,
        tokens_count: usize,
        target_tier: ModelTier,
    ) -> Result<SwarmDebate, SwarmError> {
        // 1. Catraca FinOps (Síncrona O(1))
        // PT-SWARM: O disjuntor blinda a rede
        IronCostBreaker::calculate_and_route(tokens_count, target_tier)
            .map_err(SwarmError::FinOpsBlocked)?;

        // 2. Mecânica Free-MAD (Tripartite Paralela)
        // PT-SWARM-1: Retornos são String (texto livre)
        let (lente_a, lente_b, lente_c) = join!(
            Self::exec_lente_a(),
            Self::exec_lente_b(),
            Self::exec_lente_c()
        );

        // 3. Persistência Atômica (Simulada aqui conforme Phase C)
        Ok(SwarmDebate {
            repo_id: repo_id.to_string(),
            lente_a,
            lente_b,
            lente_c,
        })
    }

    async fn exec_lente_a() -> String {
        // Mock de processamento
        "Laudo Lente A: Sentido/UX".to_string()
    }

    async fn exec_lente_b() -> String {
        // Mock de processamento
        "Laudo Lente B: Estrutura/Architecture".to_string()
    }

    async fn exec_lente_c() -> String {
        // Mock de processamento
        "Laudo Lente C: Realidade/FinOps".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Instant, Duration};
    use tokio::time::sleep;

    // Mock functions with sleep for parallel test
    async fn mock_lente_slow(name: &str) -> String {
        sleep(Duration::from_millis(100)).await;
        format!("Laudo {}", name)
    }

    #[tokio::test]
    async fn test_finops_rejection() {
        // Simular tokens massivos que excedem budget e VRAM
        let res = CognitiveSwarmDispatcher::dispatch_swarm("test_repo", 500_000, ModelTier::PremiumCloud).await;
        assert!(matches!(res, Err(SwarmError::FinOpsBlocked(FinOpsError::BudgetExceeded))));
    }

    #[tokio::test]
    async fn test_swarm_parallel_execution() {
        let start = Instant::now();
        
        // Executando as lentes mockadas em paralelo via join!
        let (a, b, c) = join!(
            mock_lente_slow("A"),
            mock_lente_slow("B"),
            mock_lente_slow("C")
        );

        let duration = start.elapsed();
        
        assert_eq!(a, "Laudo A");
        assert_eq!(b, "Laudo B");
        assert_eq!(c, "Laudo C");

        // Se fosse sequencial levaria 300ms. Paralelo leva ~100ms.
        // Usamos uma margem de segurança para o scheduler do Tokio
        assert!(duration >= Duration::from_millis(100));
        assert!(duration < Duration::from_millis(250), "Execução parece sequencial: {:?}", duration);
    }
}
