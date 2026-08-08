// SOULS V4 — Engine: OrtScorerEngine
// Stub para scorers pequenos baseados em ONNX Runtime (CPU EP).
// Visa uso com GLiClass (zero-shot classification) e BGE-reranker (re-ranking).
//
// Agnosticismo: a crate `ort` abstrai CPU EP (cross-platform). Para CoreML/DirectML,
// sera usada a mesma crate com feature flags — nunca dependencia CUDA-only.
//
// Por enquanto, retorna um score mock deterministico baseado no tamanho da query,
// provando o contrato da trait e fornecendo um sinal util para o Hipocampo.

use std::time::Instant;
use tokio::sync::watch;
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::souls_thermal_governor::SystemState;

/// Constante que normaliza o score mock no intervalo [0.0, 1.0].
const SCORE_NORMALIZER: f32 = 1024.0;

#[derive(Debug, Clone, Default)]
pub struct OrtScorerEngine {
    /// Modelo ONNX a ser carregado pelo EP CPU. Quando `None`, o engine cai em mock.
    pub onnx_model_path: Option<String>,
}

impl OrtScorerEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(path: impl Into<String>) -> Self {
        Self {
            onnx_model_path: Some(path.into()),
        }
    }

    /// Score mock deterministico: inverso do log(len(query)) normalizado.
    /// Quanto menor a query, maior o score (proxy de "specificidade").
    pub fn mock_score(&self, query: &str) -> f32 {
        let len = query.len().max(1) as f32;
        (1.0 / (1.0 + (len.ln() / SCORE_NORMALIZER))).clamp(0.0, 1.0)
    }
}

impl EphemeralInferEngine for OrtScorerEngine {
    fn run_inference(
        &self,
        req: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        if let Some(ref rx) = thermal_rx {
            while *rx.borrow() == SystemState::Paused {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        let start = Instant::now();

        if let Some(ref model_path) = self.onnx_model_path {
            if !std::path::Path::new(model_path).exists() {
                return Err(InferenceError::ModelNotFound(model_path.clone()));
            }
        }

        let score = self.mock_score(&req.user_query);
        let mock_text = format!(
            "[ORT_SCORER_MOCK] score={:.4} model={:?} query='{}'",
            score,
            self.onnx_model_path.as_deref().unwrap_or("<inline>"),
            if req.user_query.len() > 50 {
                format!("{}...", &req.user_query[..50])
            } else {
                req.user_query.clone()
            }
        );
        let prompt_tokens = (req.user_query.len() as u32 / 4).max(1);
        let completion_tokens = (mock_text.len() as u32 / 4).max(1);

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: mock_text,
            prompt_tokens,
            completion_tokens,
            total_latency_ms: start.elapsed().as_millis() as u64 + 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ort_scorer_engine_mock_score_is_deterministic_and_normalized() {
        let engine = OrtScorerEngine::new();

        let score_a = engine.mock_score("hello world");
        let score_b = engine.mock_score("hello world");
        assert_eq!(score_a, score_b, "score deve ser deterministico");

        assert!(score_a > 0.0 && score_a <= 1.0, "score fora de [0,1]: {score_a}");

        // Score de query curta deve ser maior que de query longa.
        let score_short = engine.mock_score("hi");
        let score_long = engine.mock_score(&"a".repeat(2000));
        assert!(
            score_short > score_long,
            "curta={score_short} deveria > longa={score_long}"
        );
    }

    #[test]
    fn test_ort_scorer_engine_returns_mock_text() {
        let engine = OrtScorerEngine::new();
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/gliclass.onnx".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "scoring probe".to_string(),
            max_tokens: 0,
            min_p: 0.0,
            temperature: 0.0,
            json_schema: None,
            input: None,
        };
        let resp = engine.run_inference(req, None).expect("mock nao deve falhar");
        assert!(resp.text.contains("ORT_SCORER_MOCK"));
        assert!(resp.text.contains("score="));
    }

    #[test]
    fn test_ort_scorer_engine_fails_on_missing_onnx_model() {
        let engine = OrtScorerEngine::with_model("/dev/null/nope.onnx");
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/nope.onnx".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "x".to_string(),
            max_tokens: 0,
            min_p: 0.0,
            temperature: 0.0,
            json_schema: None,
            input: None,
        };
        match engine.run_inference(req, None) {
            Err(InferenceError::ModelNotFound(_)) => {}
            other => panic!("Esperava ModelNotFound, recebido: {other:?}"),
        }
    }
}
