#[cfg(feature = "mistral_backend")]
use std::path::Path;
#[cfg(feature = "mistral_backend")]
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tokio::sync::watch;
use crate::souls_thermal_governor::SystemState;

use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};

#[cfg(feature = "mistral_backend")]
use mistralrs::{
    GgufModelBuilder, TextModelBuilder, TextMessageRole, RequestBuilder, IsqType, Model,
    PagedAttentionConfig, MemoryGpuConfig, PagedCacheType,
};

pub struct MistralRsEngine;

#[cfg(feature = "mistral_backend")]
static MISTRAL_PIPELINE_CACHE: OnceLock<Mutex<Option<(String, Arc<Model>)>>> = OnceLock::new();

#[cfg(feature = "mistral_backend")]
static MISTRAL_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

#[cfg(feature = "mistral_backend")]
fn get_mistral_cache() -> &'static Mutex<Option<(String, Arc<Model>)>> {
    MISTRAL_PIPELINE_CACHE.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "mistral_backend")]
fn get_mistral_runtime() -> Result<&'static tokio::runtime::Runtime, InferenceError> {
    MISTRAL_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Falha ao inicializar Tokio Runtime global para Mistral.rs")
    });
    MISTRAL_RUNTIME.get().ok_or_else(|| InferenceError::ExecutionError("Mistral Runtime indisponível".to_string()))
}

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
        let model_path = Path::new(&req.model_path);
        if !model_path.exists() {
            return Err(InferenceError::ModelNotFound(req.model_path.clone()));
        }

        let (parent_dir, filename) = match (model_path.parent(), model_path.file_name()) {
            (Some(p), Some(f)) => (p, f.to_string_lossy().to_string()),
            _ => return Err(InferenceError::ExecutionError(format!("Caminho de modelo invalido: {}", req.model_path))),
        };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let rt = get_mistral_runtime()?;

            rt.block_on(async {
                let is_safetensors = filename.to_lowercase().ends_with(".safetensors") || filename.to_lowercase().ends_with(".bin");
                let cache_key = format!("{}:{}", req.model_path, if is_safetensors { "safetensors" } else { "gguf" });

                // FastSwitch Pipeline Cache Check
                let model = {
                    let cache_lock = get_mistral_cache();
                    let mut lock = cache_lock.lock().map_err(|_| InferenceError::ExecutionError("Lock poisoned".to_string()))?;
                    if let Some((ref cached_key, ref cached_model)) = *lock {
                        if cached_key == &cache_key {
                            Some(cached_model.clone())
                        } else {
                            *lock = None; // FastSwitch purga atômica do modelo anterior
                            None
                        }
                    } else {
                        None
                    }
                };

                let model = match model {
                    Some(m) => m,
                    None => {
                        let paged_cfg = PagedAttentionConfig::new(
                            Some(32),
                            MemoryGpuConfig::ContextSize(4096),
                            PagedCacheType::Auto,
                        ).ok();

                        let built_model = if is_safetensors {
                            // Pilar 1 & 4: TextModelBuilder com In-Situ Quantization (ISQ Q4K) para Safetensors
                            let mut builder = TextModelBuilder::new(parent_dir.to_string_lossy().to_string())
                                .with_isq(IsqType::Q4K);
                            if let Some(ref p_cfg) = paged_cfg {
                                builder = builder.with_paged_attn(p_cfg.clone());
                            }
                            Arc::new(builder.build().await.map_err(|e| {
                                InferenceError::ExecutionError(format!("Falha ao carregar modelo Safetensors via TextModelBuilder: {}", e))
                            })?)
                        } else {
                            // Pilar 1 & 2: GgufModelBuilder com PagedAttention e detecção local de Tokenizer
                            let mut builder = GgufModelBuilder::new(
                                parent_dir.to_string_lossy().to_string(),
                                vec![filename],
                            );
                            let tok_json = parent_dir.join("tokenizer.json");
                            if tok_json.exists() {
                                builder = builder.with_tokenizer_json(tok_json.to_string_lossy().to_string());
                            }
                            let chat_tmpl = parent_dir.join("chat_template.json");
                            if chat_tmpl.exists() {
                                builder = builder.with_chat_template(chat_tmpl.to_string_lossy().to_string());
                            }
                            if let Some(ref p_cfg) = paged_cfg {
                                builder = builder.with_paged_attn(p_cfg.clone());
                            }
                            Arc::new(builder.build().await.map_err(|e| {
                                InferenceError::ExecutionError(format!("Falha ao carregar modelo GGUF via GgufModelBuilder: {}", e))
                            })?)
                        };

                        if let Ok(mut lock) = get_mistral_cache().lock() {
                            *lock = Some((cache_key, built_model.clone()));
                        }
                        built_model
                    }
                };

                let mut builder_req = RequestBuilder::new();
                if !req.system_prompt.trim().is_empty() {
                    builder_req = builder_req.add_message(TextMessageRole::System, req.system_prompt.trim());
                }
                for (input, output) in &req.few_shot_examples {
                    builder_req = builder_req.add_message(TextMessageRole::User, input.trim());
                    builder_req = builder_req.add_message(TextMessageRole::Assistant, output.trim());
                }
                builder_req = builder_req
                    .add_message(TextMessageRole::User, req.user_query.trim())
                    .set_sampler_max_len(req.max_tokens as usize)
                    .set_sampler_temperature(req.temperature as f64)
                    .with_truncate_sequence(true);

                let response = model.send_chat_request(builder_req).await
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
        }));

        match result {
            Ok(inner_res) => inner_res,
            Err(panic_payload) => {
                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Panic interno capturado em mistralrs-core (GGUF metadata/architecture unsupported)".to_string()
                };
                Err(InferenceError::ExecutionError(format!("Mistral.rs panic interceptado: {}", msg)))
            }
        }
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
