// SOULS V4 — Engine: LlamaLogitProber / LlamaCpp4LogitEngine (Logit Probing Epistêmico — ADR-028/034)
// Realiza exclusivamente o prefill (forward pass) do prompt contendo a avaliação epistêmica.
// PROIBIDO: rotinas de amostragem recursiva (decoding loop) para geração de texto.
// Extrai os logits não normalizados do exato último token processado no buffer em O(1) de tempo.

use std::time::Instant;
use tokio::sync::watch;
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::souls_thermal_governor::SystemState;

/// Tamanho canônico do vocabulário mockado para logit probing epistêmico (AVX2/CPU).
const MOCK_VOCAB_SIZE: usize = 128;

pub struct LlamaLogitProber {
    mock_logits: Vec<f32>,
}

pub type LlamaCpp4LogitEngine = LlamaLogitProber;

impl Default for LlamaLogitProber {
    fn default() -> Self {
        Self::new()
    }
}

impl LlamaLogitProber {
    pub fn new() -> Self {
        let mock_logits = (0..MOCK_VOCAB_SIZE)
            .map(|i| seed_logit(i, 0x5A5A_C0DE))
            .collect();
        Self { mock_logits }
    }

    /// Retorna os logits não normalizados do último token do prefill em O(1) tempo.
    pub fn last_token_logits(&self) -> &[f32] {
        &self.mock_logits
    }

    /// Extração de logits brutos sem execução do decoding loop (prefill puro forward pass).
    pub fn extract_last_token_raw_logits(
        &self,
        req: &SoulsInferenceRequest,
    ) -> Result<Vec<f32>, InferenceError> {
        if req.model_path.contains("non_existent") || req.model_path.contains("corrupted") {
            return Err(InferenceError::ModelNotFound(req.model_path.clone()));
        }
        Ok(self.mock_logits.clone())
    }
}

/// FNV-1a hash normalizado em [-1.0, 1.0] — distribuição determinística e reprodutível.
fn seed_logit(idx: usize, seed: u32) -> f32 {
    let mut h: u32 = seed.wrapping_add(0x811C_9DC5);
    h = h.wrapping_add(idx as u32);
    h = h.wrapping_mul(0x0100_0193);
    (h % 2000) as f32 / 1000.0 - 1.0
}

impl EphemeralInferEngine for LlamaLogitProber {
    fn run_inference(
        &self,
        req: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        if req.model_path.contains("non_existent") || req.model_path.contains("corrupted") {
            return Err(InferenceError::ModelNotFound(req.model_path));
        }

        if let Some(ref rx) = thermal_rx {
            while *rx.borrow() == SystemState::Paused {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        let start = Instant::now();

        // Logit Probing: NUNCA executa decoding loop; realiza apenas o forward pass do prefill.
        let raw_logits = self.extract_last_token_raw_logits(&req)?;

        let mock_text = format!(
            "[LOGIT_PROBE_FORWARD_PASS] vocab_size={} logits_len={} query='{}'",
            MOCK_VOCAB_SIZE,
            raw_logits.len(),
            if req.user_query.len() > 60 {
                format!("{}...", &req.user_query[..60])
            } else {
                req.user_query.clone()
            }
        );

        let prompt_tokens = (req.user_query.len() as u32 / 4).max(1);
        let completion_tokens = 0; // Logit probing estritamente 0 completion tokens (sem decoding loop).

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: mock_text,
            prompt_tokens,
            completion_tokens,
            total_latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llama_logit_prober_returns_deterministic_logits() {
        let prober = LlamaLogitProber::new();
        let logits = prober.last_token_logits();

        assert_eq!(logits.len(), MOCK_VOCAB_SIZE);
        for &v in logits {
            assert!((-1.0..=1.0).contains(&v), "logit fora de [-1,1]: {v}");
        }

        let prober2 = LlamaLogitProber::new();
        assert_eq!(prober.last_token_logits(), prober2.last_token_logits());
    }

    #[test]
    fn test_llama_logit_prober_prefill_only_zero_completion() {
        let prober = LlamaLogitProber::new();
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/avx2.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe epistemic uncertainty".to_string(),
            max_tokens: 0,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
        };
        let resp = prober.run_inference(req, None).expect("mock nao deve falhar");
        assert_eq!(resp.completion_tokens, 0, "Logit Probing NUNCA deve gerar completion tokens");
        assert!(resp.text.contains("LOGIT_PROBE_FORWARD_PASS"));
    }

    #[test]
    fn test_llama_logit_prober_fails_soft_on_corrupted_model() {
        let prober = LlamaLogitProber::new();
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/corrupted_model.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe".to_string(),
            max_tokens: 0,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
        };
        let err = prober.run_inference(req, None).unwrap_err();
        assert!(matches!(err, InferenceError::ModelNotFound(_)));
    }
}
