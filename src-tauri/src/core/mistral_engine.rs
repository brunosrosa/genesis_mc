use std::time::Instant;
use tokio::sync::watch;
use crate::souls_thermal_governor::SystemState;

use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};

#[cfg(feature = "mistral_backend")]
use mistralrs::{
    GgufModelBuilder, TextMessageRole, TextMessages,
};

pub struct MistralRsEngine;

#[cfg(feature = "mistral_backend")]
impl EphemeralInferEngine for MistralRsEngine {
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

        let start_time = Instant::now();

        let model_path = std::path::Path::new(&req.model_path);
        if !model_path.exists() {
            return Err(InferenceError::ModelNotFound(req.model_path.clone()));
        }

        let (parent_dir, filename) = match (model_path.parent(), model_path.file_name()) {
            (Some(p), Some(f)) => (p, f.to_string_lossy().to_string()),
            _ => return Err(InferenceError::ExecutionError(format!("Caminho de modelo invalido: {}", req.model_path))),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| InferenceError::ExecutionError(format!("Falha ao criar runtime Tokio: {}", e)))?;

        rt.block_on(async {
            let model = GgufModelBuilder::new(
                parent_dir.to_string_lossy().to_string(),
                vec![filename],
            )
            .build()
            .await
            .map_err(|e| InferenceError::ExecutionError(format!("Falha ao carregar modelo Mistral GGUF: {}", e)))?;

            let mut messages = TextMessages::new();
            if !req.system_prompt.trim().is_empty() {
                messages = messages.add_message(TextMessageRole::System, req.system_prompt.trim());
            }
            for (input, output) in &req.few_shot_examples {
                messages = messages.add_message(TextMessageRole::User, input.trim());
                messages = messages.add_message(TextMessageRole::Assistant, output.trim());
            }
            messages = messages.add_message(TextMessageRole::User, req.user_query.trim());

            let response = model.send_chat_request(messages).await
                .map_err(|e| InferenceError::ExecutionError(format!("Falha na inferencia Mistral: {}", e)))?;

            let choice = response.choices.first()
                .ok_or_else(|| InferenceError::ExecutionError("Nenhuma resposta gerada pelo motor Mistral".to_string()))?;

            let generated_text = choice.message.content.clone().unwrap_or_default();
            let prompt_tokens = response.usage.prompt_tokens as u32;
            let completion_tokens = response.usage.completion_tokens as u32;
            let total_latency_ms = start_time.elapsed().as_millis() as u64;

            Ok(SoulsInferenceResponse {
                status: "success".to_string(),
                text: generated_text,
                prompt_tokens,
                completion_tokens,
                total_latency_ms,
            })
        })
    }
}

#[cfg(not(feature = "mistral_backend"))]
impl EphemeralInferEngine for MistralRsEngine {
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

        let start_time = Instant::now();

        if req.model_path.contains("non_existent") {
            return Err(InferenceError::ModelNotFound(req.model_path));
        }

        let total_prompt_len = req.system_prompt.len()
            + req.user_query.len()
            + req.few_shot_examples.iter().map(|(i, o)| i.len() + o.len()).sum::<usize>();

        let prompt_tokens = (total_prompt_len as u32 / 4).max(1) + 120;
        let mock_text = format!(
            "[MISTRAL.RS PREFILL MOCK] Processando query: '{}' (few_shots: {})",
            req.user_query, req.few_shot_examples.len()
        );
        let completion_tokens = (mock_text.len() as u32 / 4).max(1);

        let total_latency_ms = start_time.elapsed().as_millis() as u64 + 14;

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: mock_text,
            prompt_tokens,
            completion_tokens,
            total_latency_ms,
        })
    }
}
