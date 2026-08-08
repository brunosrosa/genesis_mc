use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::watch;
use crate::souls_thermal_governor::SystemState;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum InferenceError {
    #[error("Modelo não encontrado: {0}")]
    ModelNotFound(String),

    #[error("Estouro de VRAM (OOM)")]
    GpuOom,

    #[error("Falha na máscara gramatical JSON: {0}")]
    GrammarMaskError(String),

    #[error("Falha de execução do motor: {0}")]
    ExecutionError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InferenceInput {
    RawText(String),
    PreTokenized(Vec<u32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulsInferenceRequest {
    pub model_path: String,
    pub system_prompt: String,
    pub few_shot_examples: Vec<(String, String)>,
    pub user_query: String,
    pub max_tokens: u32,
    pub min_p: f32,
    pub temperature: f32,
    pub json_schema: Option<String>,
    pub input: Option<InferenceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulsInferenceResponse {
    pub status: String,
    pub text: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_latency_ms: u64,
}

pub trait EphemeralInferEngine: Send + Sync {
    fn run_inference(
        &self,
        req: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError>;
}

/// Implementação Mock do Motor de Inferência Efêmero (Fase 4.3 - Estrutura Few-Shot e Telemetria E³)
pub struct MockEphemeralInferEngine;

impl EphemeralInferEngine for MockEphemeralInferEngine {
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

        if req.model_path.contains("non_existent") {
            return Err(InferenceError::ModelNotFound(req.model_path));
        }

        let mock_text = format!(
            "[MOCK] Simulando inferência restrita para query '{}' com modelo '{}' (min_p: {}, temp: {}, few_shots: {})",
            req.user_query, req.model_path, req.min_p, req.temperature, req.few_shot_examples.len()
        );
        let total_prompt_len = req.system_prompt.len() + req.user_query.len();
        let prompt_tokens = (total_prompt_len as u32 / 4).max(1);
        let completion_tokens = (mock_text.len() as u32 / 4).max(1);

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: mock_text,
            prompt_tokens,
            completion_tokens,
            total_latency_ms: 42,
        })
    }
}
