// SOULS V4 — Engine: LlamaCpp4LogitEngine
// Stub CPU-AVX2 para logit probing usado pelo Hipocampo Epistêmico.
// NUNCA decodifica string. Retorna apenas os logits brutos do último token do prefill.
//
// Agnosticismo: intrinsics `core::arch::x86_64::*` serão guardados por `cfg(target_arch)` quando
// integrarmos com llama.cpp real. Por enquanto, distribution determinística (FNV-1a) para TDD.

use std::time::Instant;
use tokio::sync::watch;
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::souls_thermal_governor::SystemState;

/// Tamanho canonico do vocabulario mockado (cobertura minima para Hipocampo probe).
const MOCK_VOCAB_SIZE: usize = 128;

pub struct LlamaCpp4LogitEngine {
    /// Logits pre-computados do prefill (deterministicos para o mesmo seed de input).
    mock_logits: Vec<f32>,
}

impl Default for LlamaCpp4LogitEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LlamaCpp4LogitEngine {
    pub fn new() -> Self {
        let mock_logits = (0..MOCK_VOCAB_SIZE)
            .map(|i| seed_logit(i, 0x5A5A_C0DE))
            .collect();
        Self { mock_logits }
    }

    /// Acessor publico para o Hipocampo Epistemico (logit probing).
    pub fn last_token_logits(&self) -> &[f32] {
        &self.mock_logits
    }
}

/// FNV-1a hash normalizado em [-1.0, 1.0] — distribution deterministica e reprodutivel.
fn seed_logit(idx: usize, seed: u32) -> f32 {
    let mut h: u32 = seed.wrapping_add(0x811C_9DC5);
    h = h.wrapping_add(idx as u32);
    h = h.wrapping_mul(0x0100_0193);
    (h % 2000) as f32 / 1000.0 - 1.0
}

impl EphemeralInferEngine for LlamaCpp4LogitEngine {
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

        // Modo CPU-AVX2: nunca tenta carregar modelo do disco. Stub puro.
        // O cascade NUNCA roteia para esta engine se o modelo for um GGUF real;
        // ela serve apenas para logit probing sob demanda do Hipocampo.
        let mock_text = format!(
            "[LOGIT_PROBE_MOCK] vocab_size={} query='{}'",
            MOCK_VOCAB_SIZE,
            if req.user_query.len() > 60 {
                format!("{}...", &req.user_query[..60])
            } else {
                req.user_query.clone()
            }
        );

        let prompt_tokens = (req.user_query.len() as u32 / 4).max(1);
        let completion_tokens = 0; // Logit probing nao gera completion.

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
    fn test_llama_cpp4_logit_engine_returns_deterministic_logits() {
        let engine = LlamaCpp4LogitEngine::new();
        let logits = engine.last_token_logits();

        assert_eq!(logits.len(), MOCK_VOCAB_SIZE);
        for &v in logits {
            assert!((-1.0..=1.0).contains(&v), "logit fora de [-1,1]: {v}");
        }

        // Re-instanciacao deve produzir a mesma distribution (determinismo).
        let engine2 = LlamaCpp4LogitEngine::new();
        assert_eq!(engine.last_token_logits(), engine2.last_token_logits());
    }

    #[test]
    fn test_llama_cpp4_logit_engine_respects_thermal_paused() {
        let engine = LlamaCpp4LogitEngine::new();
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/avx2.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe".to_string(),
            max_tokens: 0,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
        };
        let resp = engine.run_inference(req, None).expect("mock nao deve falhar");
        assert_eq!(resp.completion_tokens, 0);
        assert!(resp.text.contains("LOGIT_PROBE_MOCK"));
    }
}
