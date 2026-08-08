// SOULS V4 — Engine: BurnAgnosticEngine
// Stub agnóstico de hardware para inferencia via `burn` (https://burn-rs.github.io)
// + `cubecl` (megakernels transpilaveis).
//
// Agnosticismo: Burn abstrai WGPU/CUDA/Metal/Vulkan/NPU. O cascade NUNCA deve
// acoplar este motor a uma dependencia CUDA-only. Quando a integracao for feita,
// sera via `burn::backend::wgpu::Wgpu` ou `burn::backend::autodiff::Autodiff`,
// nunca via `burn::backend::cuda::*` diretamente.

use std::time::Instant;
use tokio::sync::watch;
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::souls_thermal_governor::SystemState;

/// Marcador de agnosticismo: a string DEVE estar presente para garantir
/// que o cascade detecta corretamente a propriedade de transmutabilidade.
pub const BURN_AGNOSTIC_MARKER: &str = "burn::backend::wgpu::Wgpu|cubecl::cuda::Cuda|cubecl::metal::Metal|cubecl::vulkan::Vulkan|cubecl::cpu::Cpu";

pub struct BurnAgnosticEngine {
    /// Habilita o stub que retorna PENDING_ENGINE. Util para o cascade discriminar
    /// entre "engine disponivel" e "engine conhecida mas nao integrada".
    pub pending: bool,
}

impl Default for BurnAgnosticEngine {
    fn default() -> Self {
        Self { pending: true }
    }
}

impl BurnAgnosticEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ready() -> Self {
        Self { pending: false }
    }
}

impl EphemeralInferEngine for BurnAgnosticEngine {
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

        if self.pending {
            return Err(InferenceError::ExecutionError(format!(
                "PENDING_ENGINE: BurnAgnosticEngine ainda nao integrado. Agnosticismo: {BURN_AGNOSTIC_MARKER}"
            )));
        }

        // Stub "ready" (nao deve ser atingido em producao ate a integracao).
        let start = Instant::now();
        let mock_text = format!(
            "[BURN_AGNOSTIC_MOCK] Megakernel CubeCL ativo. query='{}'",
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
            total_latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_burn_agnostic_engine_pending_error() {
        let engine = BurnAgnosticEngine::new(); // pending=true por default
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/burn.safetensors".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe".to_string(),
            max_tokens: 0,
            min_p: 0.0,
            temperature: 0.0,
            json_schema: None,
            input: None,
        };

        match engine.run_inference(req, None) {
            Err(InferenceError::ExecutionError(msg)) => {
                assert!(msg.contains("PENDING_ENGINE"), "esperava PENDING_ENGINE em: {msg}");
                assert!(
                    msg.contains("Agnosticismo"),
                    "esperava mencao a agnosticismo em: {msg}"
                );
            }
            other => panic!("Esperava ExecutionError com PENDING_ENGINE, recebido: {other:?}"),
        }
    }

    #[test]
    fn test_burn_agnostic_marker_covers_all_target_backends() {
        // HARDEN: garante que o marker do agnosticismo nao foi removido por refactor.
        assert!(BURN_AGNOSTIC_MARKER.contains("wgpu"));
        assert!(BURN_AGNOSTIC_MARKER.contains("metal"));
        assert!(BURN_AGNOSTIC_MARKER.contains("vulkan"));
        assert!(BURN_AGNOSTIC_MARKER.contains("cpu"));
        // NUNCA deve aparecer "cuda-only" sem alternativa.
        assert!(!BURN_AGNOSTIC_MARKER.contains("cuda-only"));
    }
}
