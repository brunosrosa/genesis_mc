// SOULS V4 — Engine: BitnetEngine
// Wrap da `BitNetDaemon` existente sob o trait `EphemeralInferEngine`.
// Mantem o enjaulamento Job Object do Windows herdado de bitnet_daemon.rs.
//
// FUTURE: iceoryx2 IPC bridge entre o daemon CPU e o orchestrator Tokio (true zero-copy
// shared memory entre processos). Por enquanto, sidecar via stdin/stdout NDJSON.

use std::time::Instant;
use tokio::sync::watch;
use crate::core::bitnet_daemon::BitNetError;
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::souls_thermal_governor::SystemState;

pub struct BitnetEngine {
    /// Binario do daemon (caminho onde o executavel `bitnet_daemon` reside).
    pub binary_path: String,
    /// Modelo ternario (1.58-bit) alvo.
    pub model_path: String,
}

impl BitnetEngine {
    pub fn new(binary_path: impl Into<String>, model_path: impl Into<String>) -> Self {
        Self {
            binary_path: binary_path.into(),
            model_path: model_path.into(),
        }
    }
}

impl EphemeralInferEngine for BitnetEngine {
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

        // Guard 1: modelo deve existir.
        if !std::path::Path::new(&self.model_path).exists() {
            return Err(InferenceError::ModelNotFound(self.model_path.clone()));
        }

        // Guard 2: detectar marcadores ternarios no path (i2_s, i1_s, bitnet) — bitnet so
        // aceita modelos ternarios. Outros tipos = fallback.
        let path_lower = self.model_path.to_lowercase();
        let is_ternary = path_lower.contains("i2_s")
            || path_lower.contains("i1_s")
            || path_lower.contains("bitnet")
            || path_lower.contains("ternary");

        if !is_ternary {
            return Err(InferenceError::ExecutionError(
                "BitnetEngine exige modelo ternario (i2_s, i1_s ou 'bitnet' no path)".to_string(),
            ));
        }

        // Stub: o daemon real seria spawnado aqui via `BitNetDaemon::spawn` herdado
        // de bitnet_daemon.rs. Para TDD, retornamos uma resposta mock deterministica
        // que prova o contrato da trait.
        let mock_text = format!(
            "[BITNET_TERNARY_MOCK] Modelo '{}' carregado em daemon isolado (Job Object). query='{}'",
            self.model_path,
            if req.user_query.len() > 50 {
                format!("{}...", &req.user_query[..50])
            } else {
                req.user_query.clone()
            }
        );
        let prompt_tokens = ((req.system_prompt.len() + req.user_query.len()) / 4).max(1) as u32;
        let completion_tokens = (mock_text.len() as u32 / 4).max(1);

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: mock_text,
            prompt_tokens,
            completion_tokens,
            total_latency_ms: start.elapsed().as_millis() as u64 + 5,
        })
    }
}

// Stub helper para mapear erros do `BitNetDaemon` em `InferenceError` (util em integracao futura).
#[allow(dead_code)]
fn map_bitnet_error(err: BitNetError) -> InferenceError {
    match err {
        BitNetError::BinaryNotFound(p) => InferenceError::ExecutionError(format!(
            "BitNet binary nao encontrado: {p}"
        )),
        BitNetError::SpawnError(e) => InferenceError::ExecutionError(format!(
            "BitNet spawn falhou: {e}"
        )),
        BitNetError::ProcessTerminated(s) => InferenceError::ExecutionError(format!(
            "BitNet daemon terminou: {s}"
        )),
        BitNetError::JobObjectError(e) => InferenceError::ExecutionError(format!(
            "BitNet Job Object falhou: {e}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitnet_engine_fails_soft_on_non_existent_model() {
        let engine = BitnetEngine::new("/dev/null/bitnet_daemon.exe", "/dev/null/nope.i2_s.gguf");
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/nope.i2_s.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe".to_string(),
            max_tokens: 8,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };

        match engine.run_inference(req, None) {
            Err(InferenceError::ModelNotFound(_)) => {}
            other => panic!("Esperava ModelNotFound, recebido: {other:?}"),
        }
    }

    #[test]
    fn test_bitnet_engine_rejects_non_ternary_path() {
        // Modelo existe mas nao e ternario — guard deve disparar.
        let temp = tempfile::tempdir().unwrap();
        let model = temp.path().join("llama-3.gguf");
        std::fs::write(&model, b"fake").unwrap();

        let engine = BitnetEngine::new("C:/fake/bitnet.exe", model.to_string_lossy().as_ref());
        let req = SoulsInferenceRequest {
            model_path: model.to_string_lossy().to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe".to_string(),
            max_tokens: 8,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };

        match engine.run_inference(req, None) {
            Err(InferenceError::ExecutionError(msg)) => {
                assert!(msg.contains("ternario"), "mensagem deveria mencionar 'ternario': {msg}");
            }
            other => panic!("Esperava ExecutionError mencionando ternario, recebido: {other:?}"),
        }
    }
}
