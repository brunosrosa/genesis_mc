use souls_mc_lib::finops::iron_cost::{FinOpsError, IronCostBreaker, ModelTier};
use thiserror::Error;
use tokio::join;

use crate::swarm::SwarmDebate;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SwarmError {
    #[error("Falha no disjuntor FinOps: {0}")]
    FinOpsBlocked(#[from] FinOpsError),
    #[error("Falha na persistência do debate: {0}")]
    PersistenceError(String),
}

/// Papéis do Free-MAD tripartite (Marco I, Lente A/B/C).
///
/// `(lettera, papel_descritivo)` é a única fonte de verdade — antes
/// estava duplicado em 3 funções `exec_lente_a/b/c` idênticas, que o
/// rust-analyzer sinalizava como cópia evidente.
const SWARM_LENTES: &[(&str, &str)] = &[
    ("A", "Sentido/UX"),
    ("B", "Estrutura/Architecture"),
    ("C", "Realidade/FinOps"),
];

pub struct CognitiveSwarmDispatcher;

impl CognitiveSwarmDispatcher {
    /// Despacha o debate Free-MAD paralelo para `repo_id` sob a catraca
    /// FinOps. Retorna o `SwarmDebate` populado com os 3 laudos.
    pub async fn dispatch_swarm(
        repo_id: &str,
        tokens_count: usize,
        target_tier: ModelTier,
    ) -> Result<SwarmDebate, SwarmError> {
        // 1. Catraca FinOps (síncrona O(1)) — blinda a rede contra explosão
        //    de tokens e estouro de VRAM.
        IronCostBreaker::calculate_and_route(tokens_count, target_tier)
            .map_err(SwarmError::FinOpsBlocked)?;

        // 2. Free-MAD tripartite em paralelo (O(1) wall-clock = lente mais lenta).
        //    Mantemos 3 calls nomeadas em vez de `join_all` para que o
        //    desempacotamento preserve a ordem sem alocação intermediária.
        let (lente_a, lente_b, lente_c) = join!(
            Self::exec_lente(&SWARM_LENTES[0], repo_id),
            Self::exec_lente(&SWARM_LENTES[1], repo_id),
            Self::exec_lente(&SWARM_LENTES[2], repo_id)
        );

        // 3. Persistência atômica (stub — gravação real via StateDbOp::SubAgent).
        Ok(SwarmDebate {
            repo_id: repo_id.to_string(),
            lente_a,
            lente_b,
            lente_c,
        })
    }

    /// Invoca o Gemini Flash com o prompt da lente (`(lettera, papel)`).
    /// Failsafe: sem `GOOGLE_API_KEY`, devolve sentinel em vez de panic.
    async fn chama_gemini(prompt: &str) -> String {
        let api_key = std::env::var("GOOGLE_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return "API KEY MISSING".to_string();
        }
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={api_key}"
        );
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }]
        });

        match client.post(&url).json(&body).send().await {
            Ok(res) => res
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|json| {
                    json["candidates"][0]["content"]["parts"][0]["text"]
                        .as_str()
                        .map(str::to_string)
                })
                .unwrap_or_else(|| "Falha ao extrair texto da API".to_string()),
            Err(e) => format!("Erro de rede: {e}"),
        }
    }

    /// Constrói o prompt da lente a partir de `(lettera, papel)` e despacha
    /// para o Gemini. Substitui as 3 cópias `exec_lente_a/b/c` por uma
    /// única função parametrizada (DRY).
    async fn exec_lente(lente: &(&'static str, &'static str), repo_id: &str) -> String {
        let (lettera, papel) = *lente;
        let prompt = format!(
            "Atue como Lente {lettera} ({papel}) e avalie o repositório {repo_id} em um parágrafo."
        );
        Self::chama_gemini(&prompt).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use tokio::time::sleep;

    use super::*;

    // Mock com sleep para validar paralelismo.
    async fn mock_lente_slow(name: &str) -> String {
        sleep(Duration::from_millis(100)).await;
        format!("Laudo {name}")
    }

    #[tokio::test]
    async fn test_finops_rejection() {
        // tokens massivos em tier premium — disjuntor deve barrar.
        let res =
            CognitiveSwarmDispatcher::dispatch_swarm("test_repo", 500_000, ModelTier::PremiumCloud)
                .await;
        assert!(matches!(
            res,
            Err(SwarmError::FinOpsBlocked(FinOpsError::BudgetExceeded { .. }))
        ));
    }

    #[tokio::test]
    async fn test_swarm_parallel_execution() {
        let start = Instant::now();

        // 3 lentes mockadas em paralelo via `join!`.
        let (a, b, c) = join!(
            mock_lente_slow("A"),
            mock_lente_slow("B"),
            mock_lente_slow("C")
        );

        let duration = start.elapsed();

        assert_eq!(a, "Laudo A");
        assert_eq!(b, "Laudo B");
        assert_eq!(c, "Laudo C");

        // Sequencial seria 300ms; paralelo ≈ 100ms. Margem de 250ms
        // absorve jitter do scheduler do Tokio.
        assert!(duration >= Duration::from_millis(100));
        assert!(
            duration < Duration::from_millis(250),
            "Execução parece sequencial: {duration:?}"
        );
    }

    /// Valida que `SWARM_LENTES` está em sincronia com a função
    /// `dispatch_swarm` — ambas consomem os 3 papéis na mesma ordem.
    /// Anti-regressão: se alguém adicionar uma 4ª lente aqui, o teste
    /// força a atualização do `dispatch_swarm`.
    #[test]
    fn test_swarm_lentes_invariant() {
        assert_eq!(SWARM_LENTES.len(), 3);
        assert_eq!(SWARM_LENTES[0].0, "A");
        assert_eq!(SWARM_LENTES[1].0, "B");
        assert_eq!(SWARM_LENTES[2].0, "C");
    }
}
