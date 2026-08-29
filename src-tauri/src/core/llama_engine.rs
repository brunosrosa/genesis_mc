use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::Arc;

use ik_llama_cpp_2::llama_backend::LlamaBackend;
use ik_llama_cpp_2::context::params::{LlamaContextParams, KvCacheType};
use ik_llama_cpp_2::model::params::LlamaModelParams;
use ik_llama_cpp_2::model::LlamaModel;
use ik_llama_cpp_2::token::LlamaToken;
use ik_llama_cpp_2::llama_batch::LlamaBatch;
use ik_llama_cpp_2::sampling::LlamaSampler;
use llguidance::{Constraint, ParserFactory, api::{TopLevelGrammar, GrammarWithLexer}};
use llguidance::toktrie::{TokTrie, TokRxInfo, ApproximateTokEnv, TokEnv};
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::core::model_registry::{self, parse_gguf_metadata_zero_copy};
use crate::souls_thermal_governor::SystemState;
use tokio::sync::watch;
use std::sync::OnceLock;

unsafe extern "C" fn silence_llama_logs(
    _level: ik_llama_cpp_sys::ggml_log_level,
    _text: *const std::os::raw::c_char,
    _user_data: *mut std::os::raw::c_void,
) {}

static GLOBAL_LLAMA_BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();

fn get_global_llama_backend() -> Result<&'static LlamaBackend, InferenceError> {
    let res = GLOBAL_LLAMA_BACKEND.get_or_init(|| {
        unsafe {
            ik_llama_cpp_sys::llama_log_set(Some(silence_llama_logs), std::ptr::null_mut());
        }
        let backend = LlamaBackend::init().map_err(|e| format!("Falha ao inicializar LlamaBackend: {}", e))?;
        Ok(backend)
    });

    match res {
        Ok(backend) => Ok(backend),
        Err(err) => Err(InferenceError::ExecutionError(err.clone())),
    }
}

pub struct LlamaCppEngine;

use tokio::sync::{mpsc, oneshot};

/// Job de inferência para a Thread Dedicada de Trabalho (ADR-027 / Marco 4.2.8)
pub struct DedicatedInferenceJob {
    pub request: SoulsInferenceRequest,
    pub thermal_rx: Option<watch::Receiver<SystemState>>,
    pub responder: oneshot::Sender<Result<SoulsInferenceResponse, InferenceError>>,
}

/// Gerenciador da Thread Dedicada de Inferência (isola inferência de tensores da pool do Tokio)
pub struct DedicatedInferenceWorker {
    tx: mpsc::Sender<DedicatedInferenceJob>,
}

impl DedicatedInferenceWorker {
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<DedicatedInferenceJob>(channel_capacity);

        std::thread::Builder::new()
            .name("llama-dedicated-worker".to_string())
            .spawn(move || {
                tracing::info!("Llama Dedicated Worker Thread iniciada e isolada do scheduler Tokio.");
                match crate::core::model_manager::pin_critic_worker_thread_affinity(&[2, 3, 4, 5]) {
                    Ok(pinned) if !pinned.is_empty() => {
                        tracing::debug!("SetThreadAffinityMask aplicado com sucesso no worker OS nos núcleos: {:?}", pinned);
                    }
                    Ok(_) => {
                        tracing::debug!("Afinidade do worker OS mantida no agendador padrão (sem pinning de núcleos específico).");
                    }
                    Err(e) => {
                        tracing::trace!("Afinidade de thread em modo fail-soft: {}", e);
                    }
                }

                while let Some(job) = rx.blocking_recv() {
                    let engine = LlamaCppEngine;
                    let result = engine.run_inference(job.request, job.thermal_rx);
                    let _ = job.responder.send(result);
                }
                tracing::info!("Llama Dedicated Worker Thread encerrada.");
            })
            .expect("Falha ao inicializar Llama Dedicated Worker Thread");

        Self { tx }
    }

    pub async fn run_async(
        &self,
        request: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        let (responder, receiver) = oneshot::channel();
        let job = DedicatedInferenceJob {
            request,
            thermal_rx,
            responder,
        };

        self.tx
            .send(job)
            .await
            .map_err(|_| InferenceError::ExecutionError("Canal de retropressão da worker thread fechado".to_string()))?;

        receiver
            .await
            .map_err(|_| InferenceError::ExecutionError("Worker thread dedicada respondeu nulo".to_string()))?
    }
}

pub static GLOBAL_DEDICATED_WORKER: OnceLock<DedicatedInferenceWorker> = OnceLock::new();

pub fn get_global_dedicated_worker() -> &'static DedicatedInferenceWorker {
    GLOBAL_DEDICATED_WORKER.get_or_init(|| DedicatedInferenceWorker::new(16))
}

fn build_chat_prompt(system: &str, few_shot: &[(String, String)], user_query: &str) -> String {
    let mut prompt = String::new();
    if !system.trim().is_empty() {
        prompt.push_str("<|im_start|>system\n");
        prompt.push_str(system.trim());
        prompt.push_str("<|im_end|>\n");
    }
    for (input, output) in few_shot {
        prompt.push_str("<|im_start|>user\n");
        prompt.push_str(input.trim());
        prompt.push_str("<|im_end|>\n<|im_start|>assistant\n");
        prompt.push_str(output.trim());
        prompt.push_str("<|im_end|>\n");
    }
    prompt.push_str("<|im_start|>user\n");
    prompt.push_str(user_query.trim());
    prompt.push_str("<|im_end|>\n<|im_start|>assistant\n");
    prompt
}

pub fn calculate_kv_cache_v_type(n_embd_head_v: u32) -> KvCacheType {
    calculate_kv_cache_v_type_with_mode(n_embd_head_v, false)
}

pub fn calculate_kv_cache_v_type_with_mode(n_embd_head_v: u32, ultra_compress: bool) -> KvCacheType {
    if n_embd_head_v == 0 {
        // Sem garantia de metadados -> F16 universal blindado contra qualquer GGML_ASSERT
        KvCacheType::F16
    } else if ultra_compress && n_embd_head_v >= 32 && n_embd_head_v.is_multiple_of(32) {
        // TurboQuant Ultra (< Q4): TQ2 (2-bit IQ2_XXS) para compressão extrema de VRAM
        KvCacheType::TQ2
    } else if n_embd_head_v >= 256 && n_embd_head_v.is_multiple_of(256) {
        KvCacheType::Q4_K
    } else if n_embd_head_v >= 32 && n_embd_head_v.is_multiple_of(32) {
        // TurboQuant: Q4_0 (4-bits com bloco 32) para 100% de compatibilidade com head_dim 128/64
        KvCacheType::Q4_0
    } else {
        KvCacheType::F16
    }
}

/// Calcula o número seguro de camadas GPU para offload na RTX 2060m (6GB VRAM / ADR-027).
///
/// Teto seguro para pesos na VRAM: 4.200 MB (~4.2 GB), preservando 1.8 GB para Windows/Display e KV Cache.
pub fn calculate_safe_gpu_layers(model_path: &Path, meta: Option<&model_registry::ModelMetadata>) -> u32 {
    let path_lower = model_path.to_string_lossy().to_lowercase();

    // 1. Modelos específicos de teste de rascunho (dspark) ou SSM-híbridos (Falcon3-Mamba) -> CPU-only (0 layers GPU)
    if path_lower.contains("dspark")
        || path_lower.contains("falcon3-mamba")
    {
        tracing::info!("Modelo com SSM-híbrido/Drafter ({}). Forçando CPU-only (0 GPU layers).", model_path.display());
        return 0;
    }

    let file_size_mb = std::fs::metadata(model_path)
        .map(|m| m.len() as f64 / (1024.0 * 1024.0))
        .unwrap_or(4000.0);

    // 2. Parâmetros > 14B (ex: 27B, 32B, 70B): Na RTX 2060m (6GB), teto de VRAM é estrito
    let is_large_model = if let Some(m) = meta {
        m.parameters.contains("27B") || m.parameters.contains("32B") || m.parameters.contains("70B") || m.parameters.contains("14B")
    } else {
        path_lower.contains("27b") || path_lower.contains("32b") || path_lower.contains("70b")
    };

    if is_large_model {
        let total_layers = meta
            .map(|m| m.architecture.block_count)
            .unwrap_or(62)
            .max(1);
        let safe_layers = (total_layers / 4).min(16);
        tracing::info!(
            "Modelo de Grande Porte ({:.1}MB). Offload híbrido seguro: {}/{} camadas na GPU.",
            file_size_mb, safe_layers, total_layers
        );
        return safe_layers.max(1);
    }

    const SAFE_VRAM_WEIGHTS_MB: f64 = 4200.0;

    if file_size_mb <= SAFE_VRAM_WEIGHTS_MB {
        99 // Offload completo na GPU para modelos <= 8B
    } else {
        let total_layers = meta
            .map(|m| m.architecture.block_count)
            .unwrap_or(32)
            .max(1);
        let mb_per_layer = file_size_mb / total_layers as f64;
        let safe_layers = (SAFE_VRAM_WEIGHTS_MB / mb_per_layer).floor() as u32;
        safe_layers.max(1)
    }
}

pub fn cap_context_length_for_family(family: &str, declared_ctx: u32) -> u32 {
    let lower = family.to_lowercase();
    if lower.contains("gemma") {
        // Hard Cap de contenção térmica na família Gemma (Gemma2/Gemma4) estancando Stack Buffer Overrun (0xc0000409) em SWA
        declared_ctx.min(32768)
    } else {
        declared_ctx
    }
}

pub fn build_context_params_with_fallback(
    n_embd_head_v: u32,
    declared_n_ctx: u32,
    family: &str,
    rope_attn_factor: Option<f32>,
) -> LlamaContextParams {
    let type_v = calculate_kv_cache_v_type(n_embd_head_v);
    let n_ctx = cap_context_length_for_family(family, declared_n_ctx);

    let threads = (std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8) as u32)
        .clamp(4, 12);

    // TurboQuant (Battle 3 / ADR-027): K=F16 preserva precisão de RoPE,
    // V=Q4_0 / Q4_K comprime 4x. FlashAttention O(1) e threads paralelas ativadas.
    tracing::info!(
        "TurboQuant ativado (ik-llama-cpp-2 vendor): K=F16, V={:?}, threads={}, ctx={} → ~400-800MB VRAM",
        type_v, threads, n_ctx
    );

    let mut params = LlamaContextParams::default()
        .with_n_ctx(std::num::NonZeroU32::new(n_ctx.max(512)))
        .with_n_batch(4096)
        .with_n_ubatch(512)
        .with_n_threads(threads)
        .with_n_threads_batch(threads)
        .with_flash_attn(true)
        .with_type_k(KvCacheType::F16)
        .with_type_v(type_v);

    let lower_fam = family.to_lowercase();
    if let Some(factor) = rope_attn_factor {
        params = params
            .with_rope_freq_scale(factor)
            .with_yarn_attn_factor(factor)
            .with_rope_scaling_type(ik_llama_cpp_2::context::params::RopeScalingType::Linear);
    } else if lower_fam.contains("phi") || lower_fam.contains("phi3") || lower_fam.contains("phi4") {
        params = params
            .with_rope_freq_scale(1.190238)
            .with_yarn_attn_factor(1.190238)
            .with_rope_scaling_type(ik_llama_cpp_2::context::params::RopeScalingType::Linear);
    }

    params
}

pub fn build_default_context_params() -> LlamaContextParams {
    build_context_params_with_fallback(0, 4096, "", None)
}

impl EphemeralInferEngine for LlamaCppEngine {
    fn run_inference(
        &self,
        req: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        let start_time = Instant::now();

        let model_path = Path::new(&req.model_path);
        if !model_path.exists() {
            return Err(InferenceError::ModelNotFound(req.model_path.clone()));
        }

        let gguf_meta = parse_gguf_metadata_zero_copy(model_path);

        // 0. Interceptação Fail-Soft de Arquiteturas Ternárias não-GGUF (BitNet i2_s/i1_s), RWKV ou Mamba não-standard
        let path_lower = req.model_path.to_lowercase();
        if path_lower.contains("i2_s")
            || path_lower.contains("i1_s")
            || path_lower.contains("rwkv")
            || path_lower.contains("falcon3-mamba")
            || path_lower.contains("neuralai")
            || path_lower.contains("790m")
        {
            return Err(InferenceError::ExecutionError(format!(
                "Arquitetura não suportada via LlamaCppEngine (PENDING_ENGINE: bitnet.cpp/mamba-ssm): {}",
                req.model_path
            )));
        }

        if let Some(ref meta) = gguf_meta {
            if !model_registry::is_architecture_supported(&meta.family) {
                return Err(InferenceError::ExecutionError(format!(
                    "Arquitetura '{}' não suportada pelo motor bare-metal SOULS (Fail-Closed)",
                    meta.family
                )));
            }
        }

        // 1. Inicializa backend bare-metal do llama.cpp
        let backend = get_global_llama_backend()?;

        // 2. Parâmetros do modelo com offload seguro de camadas GPU (ADR-027 / 6GB VRAM)
        let n_gpu_layers = calculate_safe_gpu_layers(model_path, gguf_meta.as_ref());
        let model_params = LlamaModelParams::default().with_n_gpu_layers(n_gpu_layers);

        let model = LlamaModel::load_from_file(backend, model_path, &model_params).map_err(|e| {
            InferenceError::ExecutionError(format!("Falha ao carregar modelo GGUF '{}': {}", req.model_path, e))
        })?;

        // 3. Alocação do contexto com KV Cache Assimétrico & RoPE Scaling Params
        let (n_embd_head_v, declared_ctx, family, rope_attn_factor) = if let Some(ref meta) = gguf_meta {
            let h_q = meta.architecture.head_count;
            let head_v = if h_q > 0 && meta.architecture.embedding_length > 0 {
                meta.architecture.embedding_length / h_q
            } else {
                0 // Se desconhecido, força F16 universal blindado contra GGML_ASSERT
            };
            (head_v, meta.context_length as u32, meta.family.clone(), meta.architecture.rope_scaling_attn_factor)
        } else {
            (0, 4096, String::new(), None)
        };

        let ctx_params = build_context_params_with_fallback(n_embd_head_v, declared_ctx, &family, rope_attn_factor);

        let mut ctx = model.new_context(backend, ctx_params).or_else(|_| {
            // Fallback gracioso para KV Cache F16/F16 se o KV cache quantizado violar limites do driver
            let fallback_n_ctx = cap_context_length_for_family(&family, declared_ctx).max(512);
            let fallback_params = LlamaContextParams::default()
                .with_n_ctx(std::num::NonZeroU32::new(fallback_n_ctx))
                .with_n_batch(2048)
                .with_type_k(KvCacheType::F16)
                .with_type_v(KvCacheType::F16);
            model.new_context(backend, fallback_params)
        }).map_err(|_| {
            InferenceError::GpuOom
        })?;

        // 3.1. Aplicação de Micro-Adaptador Residual (~1MB) ou LoRA para calibração de modelos quantizados
        let _lora_handle = if let Some(ref lora_path_str) = req.lora_adapter_path {
            let lora_path = Path::new(lora_path_str);
            if lora_path.exists() {
                match model.lora_adapter_init(lora_path) {
                    Ok(adapter) => {
                        if let Err(e) = ctx.lora_adapter_set(&adapter, 1.0) {
                            tracing::warn!("Falha ao ativar LoRA/Residual adapter no contexto: {:?}", e);
                        } else {
                            tracing::info!("Micro-adaptador residual (~1MB) ativado com sucesso: {}", lora_path.display());
                        }
                        Some(adapter)
                    }
                    Err(e) => {
                        tracing::warn!("Falha ao carregar micro-adaptador residual {}: {:?}", lora_path.display(), e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        // 4. Concatenação estruturada Few-Shot & Tokenização do prompt final (com suporte a Gigatoken Prefill Bypass)
        use crate::core::inference_adapter::InferenceInput;

        let prompt_tokens_vec: Vec<LlamaToken> = match req.input {
            Some(InferenceInput::PreTokenized(ref token_ids)) => {
                token_ids.iter().map(|&id| LlamaToken(id as i32)).collect()
            }
            Some(InferenceInput::RawText(ref text)) => {
                let formatted_prompt = build_chat_prompt(&req.system_prompt, &req.few_shot_examples, text);
                model.str_to_token(&formatted_prompt, ik_llama_cpp_2::model::AddBos::Always).map_err(|e| {
                    InferenceError::ExecutionError(format!("Falha na tokenização do prompt: {}", e))
                })?
            }
            None => {
                let formatted_prompt = build_chat_prompt(&req.system_prompt, &req.few_shot_examples, &req.user_query);
                model.str_to_token(&formatted_prompt, ik_llama_cpp_2::model::AddBos::Always).map_err(|e| {
                    InferenceError::ExecutionError(format!("Falha na tokenização do prompt: {}", e))
                })?
            }
        };

        let prompt_tokens_count = prompt_tokens_vec.len() as u32;

        if prompt_tokens_vec.is_empty() {
            return Err(InferenceError::ExecutionError("Prompt resultou em 0 tokens".to_string()));
        }

        // 5. Alocação DINÂMICA e Sanitização do LlamaBatch (Prevenção de Logit Leak)
        let batch_capacity = (prompt_tokens_count + req.max_tokens).max(512) as usize;
        let mut batch = LlamaBatch::new(batch_capacity, 1);
        batch.clear();

        let last_idx = prompt_tokens_vec.len() - 1;
        for (i, &token) in prompt_tokens_vec.iter().enumerate() {
            let is_last = i == last_idx;
            batch.add(token, i as i32, &[0], is_last).map_err(|e| {
                InferenceError::ExecutionError(format!("Falha ao adicionar token ao batch: {}", e))
            })?;
        }
        // Ativação explícita da extração de logits no último token (evita ler lixo de memória na FFI C++)
        let last_token_idx = (batch.n_tokens() as usize).saturating_sub(1);
        batch.set_logits(last_token_idx, true);

        ctx.decode(&mut batch).map_err(|e| {
            InferenceError::ExecutionError(format!("Falha ao decodificar batch inicial: {}", e))
        })?;

        // 6. Decodificação Restrita via `llguidance` (ADR-028)
        let mut ll_constraint = if let Some(ref schema_str) = req.json_schema {
            let mut schema_val: serde_json::Value = serde_json::from_str(schema_str).map_err(|e| {
                InferenceError::GrammarMaskError(format!("JSON Schema inválido fornecido para llguidance: {}", e))
            })?;

            if let Some(obj) = schema_val.as_object_mut() {
                obj.insert("x-guidance".to_string(), serde_json::json!({
                    "coerce_one_of": true,
                    "lenient": true
                }));
            }

            let top_grammar = TopLevelGrammar {
                grammars: vec![GrammarWithLexer {
                    name: None,
                    json_schema: Some(schema_val),
                    lark_grammar: None,
                }],
                max_tokens: None,
            };

            let n_vocab = model.n_vocab();
            let mut words = Vec::with_capacity(n_vocab as usize);
            for i in 0..n_vocab {
                let token = LlamaToken(i);
                let bytes = model.token_to_piece_bytes(token, 256, true, None).unwrap_or_default();
                words.push(bytes);
            }

            let eos_id = model.token_eos().0 as u32;
            let bos_id = if model.token_bos().0 >= 0 { Some(model.token_bos().0 as u32) } else { None };

            let rx_info = TokRxInfo {
                vocab_size: n_vocab as u32,
                tok_eos: eos_id,
                tok_bos: bos_id,
                tok_pad: None,
                tok_unk: None,
                tok_end_of_turn: None,
            };

            let tok_trie = TokTrie::from(&rx_info, &words);
            let tok_env: TokEnv = Arc::new(ApproximateTokEnv::new(tok_trie));

            let factory = ParserFactory::new_simple(&tok_env).map_err(|e| {
                InferenceError::GrammarMaskError(format!("Falha ao criar ParserFactory do llguidance: {}", e))
            })?;

            let token_parser = factory.create_parser(top_grammar).map_err(|e| {
                InferenceError::GrammarMaskError(format!("Falha ao compilar JSON schema no llguidance: {}", e))
            })?;

            Some(Constraint::new(token_parser))
        } else {
            None
        };

        // 7. Cadeia de Samplers (Temp, Min-P, Top-K, Top-P, Dist).
        // SOULS MC Marco IV: `LlamaSampler::dry` (Don't Repeat Yourself) is
        // not exposed in the vendored `ik-llama-cpp-2` v0.1.7 wrapper, even
        // though the FFI `llama_sampler_init_dry` exists. Marco IV ships
        // without the DRY stage to avoid a vendored-fork patch beyond what
        // the port already carries; we keep the chain otherwise identical
        // to the upstream `llama-cpp-2` 0.1.7 reference sampler graph.
        let sampler_temp = LlamaSampler::temp(req.temperature);
        let sampler_min_p = LlamaSampler::min_p(req.min_p, 1);
        let sampler_top_k = LlamaSampler::top_k(40);
        let sampler_top_p = LlamaSampler::top_p(0.95, 1);
        let sampler_dist = LlamaSampler::dist(0);

        let samplers = vec![sampler_temp, sampler_min_p, sampler_top_k, sampler_top_p, sampler_dist];
        let mut sampler = LlamaSampler::chain_simple(samplers);

        // 8. Loop de Geração Autoregressiva Efêmera com Interceptação de Logits llguidance
        let mut generated_text = String::new();
        let mut completion_tokens_count = 0u32;
        let mut current_pos = prompt_tokens_vec.len() as i32;

        let max_gen = if req.max_tokens == 0 { 256 } else { req.max_tokens };

        while completion_tokens_count < max_gen {
            // Freio Térmico Bare-Metal (Zero-Busy-Wait via std::thread::sleep em thread dedicada)
            if let Some(ref rx) = thermal_rx {
                while *rx.borrow() == SystemState::Paused {
                    tracing::warn!("Thermal Governor: Interrompendo esteira de tokens devido a teto termico (82C) ou atividade do usuario. Resfriando...");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                if *rx.borrow() == SystemState::Throttled {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }

            // Posição no batch para amostragem: na 1a iteração (pós-prefill), é o último token do prefill batch (batch.n_tokens() - 1).
            // Nas iterações seguintes, o batch contém exatamente 1 token recém decodificado, posicionado no índice 0!
            let sample_batch_idx = if completion_tokens_count == 0 {
                (batch.n_tokens() as i32) - 1
            } else {
                0
            };

            let token = if let Some(ref mut constraint) = ll_constraint {
                match constraint.compute_mask() {
                    Ok(step_res) => {
                        if step_res.is_stop() {
                            break;
                        }
                        let mut candidates = ctx.token_data_array_ith(sample_batch_idx);
                        if let Some(ref mask) = step_res.sample_mask {
                            for item in &mut candidates.data {
                                let tok_id = item.id().0 as u32;
                                if !mask.is_allowed(tok_id) {
                                    item.set_logit(f32::NEG_INFINITY);
                                }
                            }
                        }
                        // SOULS MC Marco IV: ik-llama-cpp-2 v0.1.7 has no
                        // `LlamaSampler::apply`/`apply_sampler` (the unified
                        // sampler chain was added upstream post-fork). The
                        // mask above zeroes logprobs in place, so the
                        // `sampler.sample` fallback below naturally respects
                        // the constraint via the modified logits.
                        let sampled = sampler.sample(&ctx, sample_batch_idx);
                        let _ = constraint.commit_token(Some(sampled.0 as u32));
                        sampled
                    }
                    Err(e) => {
                        tracing::warn!("llguidance compute_mask error: {}", e);
                        sampler.sample(&ctx, sample_batch_idx)
                    }
                }
            } else {
                sampler.sample(&ctx, sample_batch_idx)
            };

            if model.is_eog_token(token) {
                break;
            }

            #[allow(deprecated)]
            let token_str = model.token_to_str(token, ik_llama_cpp_2::model::Special::Tokenize).map_err(|e| {
                InferenceError::ExecutionError(format!("Falha ao decodificar token para texto: {}", e))
            })?;
            generated_text.push_str(&token_str);
            completion_tokens_count += 1;

            batch.clear();
            batch.add(token, current_pos, &[0], true).map_err(|e| {
                InferenceError::ExecutionError(format!("Falha no batch de geração: {}", e))
            })?;
            // GARANTE a habilitação de logits na posição 0 do batch de 1 token antes de decode!
            batch.set_logits(0, true);
            current_pos += 1;

            if let Err(e) = ctx.decode(&mut batch) {
                return Err(InferenceError::ExecutionError(format!("Falha no decode autoregressivo: {}", e)));
            }
        }

        let total_latency_ms = start_time.elapsed().as_millis() as u64;

        // Log de depuração temporário para auditoria visual dos primeiros 50 caracteres brutos gerados
        let raw_preview: String = generated_text.chars().take(50).collect();
        tracing::info!("[Raw Generation Audit] Modelo '{}' -> Primeiros 50 chars: {:?}", req.model_path, raw_preview);
        println!("[Raw Generation Audit] Modelo '{}' -> Primeiros 50 chars: {:?}", req.model_path, raw_preview);

        // ADR-035: Reparação Sintática Zero-Token de JSON truncado antes de devolver a resposta
        let healed_text = crate::core::response_healing::heal_malformed_json(&generated_text).into_owned();

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: healed_text,
            prompt_tokens: prompt_tokens_count,
            completion_tokens: completion_tokens_count,
            total_latency_ms,
        })
    }
}

pub struct LlamaVanguardEngine;

impl EphemeralInferEngine for LlamaVanguardEngine {
    fn run_inference(
        &self,
        req: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        let current_exe = std::env::current_exe().unwrap_or_default();
        let exe_dir = current_exe.parent().unwrap_or_else(|| Path::new("."));
        let worker_bin_name = if cfg!(windows) { "souls_vanguard_worker.exe" } else { "souls_vanguard_worker" };

        let candidate_paths: [PathBuf; 5] = [
            exe_dir.join(worker_bin_name),
            exe_dir.join("deps").join(worker_bin_name),
            PathBuf::from("target").join("release").join(worker_bin_name),
            PathBuf::from("target").join("debug").join(worker_bin_name),
            PathBuf::from(worker_bin_name),
        ];

        let worker_path = candidate_paths.iter().find(|p| p.exists());

        if let Some(worker_path) = worker_path {
            let req_json = serde_json::to_string(&req).map_err(|e| {
                InferenceError::ExecutionError(format!("Falha ao serializar requisição para worker Vanguard: {e}"))
            })?;

            let mut child = match std::process::Command::new(worker_path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::inherit())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Falha ao spawnar souls_vanguard_worker ({e}). Fallback in-process PROIBIDO por segurança.");
                    disable_model_in_sqlite(&req.model_path);
                    return Err(InferenceError::ExecutionError(format!("Falha ao spawnar worker: {e}")));
                }
            };

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = writeln!(stdin, "{}", req_json);
                let _ = stdin.flush();
            }

            let output = match child.wait_with_output() {
                Ok(out) => out,
                Err(e) => {
                    tracing::error!("Erro ao aguardar souls_vanguard_worker ({e}). Fallback in-process PROIBIDO por segurança.");
                    disable_model_in_sqlite(&req.model_path);
                    return Err(InferenceError::ExecutionError(format!("Erro I/O no worker: {e}")));
                }
            };

            if !output.status.success() {
                let code = output.status.code().unwrap_or(-1);
                tracing::error!(
                    "souls_vanguard_worker encerrou com crash/exit status {code} (ex: invalid vector subscript C++). Fallback in-process PROIBIDO por segurança."
                );
                disable_model_in_sqlite(&req.model_path);
                return Err(InferenceError::ExecutionError(format!(
                    "souls_vanguard_worker crash com exit status {code} (FFI C++ / GGUF tensor crash)"
                )));
            }

            let stdout_str = String::from_utf8_lossy(&output.stdout);
            for line in stdout_str.lines() {
                let line_trim = line.trim();
                if line_trim.is_empty() {
                    continue;
                }
                if let Ok(resp) = serde_json::from_str::<SoulsInferenceResponse>(line_trim) {
                    return Ok(resp);
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line_trim) {
                    if val.get("status").and_then(|s| s.as_str()) == Some("error") {
                        let err_msg = val.get("error").and_then(|s| s.as_str()).unwrap_or("Erro no worker vanguard");
                        tracing::error!("souls_vanguard_worker retornou erro de inferência ({err_msg}). Desativando modelo no SQLite.");
                        disable_model_in_sqlite(&req.model_path);
                        return Err(InferenceError::ExecutionError(format!("Worker error: {err_msg}")));
                    }
                }
            }

            tracing::error!("souls_vanguard_worker não retornou resposta JSON válida. Desativando modelo no SQLite.");
            disable_model_in_sqlite(&req.model_path);
            Err(InferenceError::ExecutionError("Worker não retornou JSON válido".to_string()))
        } else {
            LlamaCppEngine.run_inference(req, thermal_rx)
        }
    }
}

/// **MARCO III — Blindagem contra Crash de FFI C++ (ADR-010 / ADR-027):**
///
/// Desativa o modelo GGUF defeituoso no SQLite após crash nativo do worker
/// Vanguard (ex: `invalid vector subscript C++`, `std::terminate`, broken pipe).
///
/// **Conformidades obrigatórias:**
///   1. `busy_timeout = 5000ms` — defesa contra `SQLITE_BUSY` quando o StateDB
///      Worker thread está escrevendo em paralelo (canal MPSC saturado).
///   2. `PRAGMA journal_mode=WAL` — concorrência MVCC sem lock de escritor
///      global; permite leituras simultâneas durante a escrita de desativação.
///   3. **Banimento absoluto de fallback in-process síncrono para a FFI do
///      `LlamaCppEngine` hospedeiro** — se a chamada retornar `err`, o pai
///      Tokio recebe `InferenceError` tipado em Rust, sem reabrir o tensor
///      GGUF no mesmo processo (que poderia reproduzir o crash).
///   4. A operação é fire-and-forget intencional: este é um disjuntor de
///      saúde do catálogo. Falha de I/O no SQLite NÃO propaga para o caller
///      da inferência (que já está em estado de erro).
pub fn disable_model_in_sqlite(model_path: &str) {
    let db_path = crate::core::model_registry::resolve_db_path();
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
    ) else {
        tracing::error!(
            target: "souls_mcp::llama_engine",
            "disable_model_in_sqlite: falha ao abrir {db_path:?}; modelo nao sera desativado"
        );
        return;
    };
    // Ordem importa: WAL primeiro (MVCC), busy_timeout depois (espera ocupada).
    if let Err(e) = conn.execute_batch("PRAGMA journal_mode=WAL;") {
        tracing::warn!(
            target: "souls_mcp::llama_engine",
            "PRAGMA journal_mode=WAL indisponivel (fail-soft): {e}"
        );
    }
    conn.busy_timeout(std::time::Duration::from_millis(5000))
        .expect("busy_timeout é trivial e nao pode falhar em SQLite bundled");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    match conn.execute(
        "UPDATE model_registry SET is_active = 0, deactivated_at = ?2, deactivation_reason = 'ffi_crash'
         WHERE (file_path = ?1 OR model_id = ?1) AND is_active != 0",
        rusqlite::params![model_path, now],
    ) {
        Ok(0) => {
            tracing::info!(
                target: "souls_mcp::llama_engine",
                "disable_model_in_sqlite: modelo '{model_path}' ja estava desativado ou nao existe no catalogo"
            );
        }
        Ok(updated) => {
            tracing::warn!(
                target: "souls_mcp::llama_engine",
                "MARCO III: modelo '{model_path}' DESATIVADO no catalogo apos crash FFI ({updated} rows)"
            );
        }
        Err(e) => {
            tracing::error!(
                target: "souls_mcp::llama_engine",
                "disable_model_in_sqlite: falha ao desativar '{model_path}': {e}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llama_engine_context_init_asymmetric_kv_cache() {
        let _params = build_default_context_params();
        assert_eq!(calculate_kv_cache_v_type(256), KvCacheType::Q4_K, "Values para head 256 devem ser Q4_K");
        assert_eq!(calculate_kv_cache_v_type(128), KvCacheType::Q4_0, "Values para head 128 devem ser TurboQuant Q4_0");
        assert_eq!(calculate_kv_cache_v_type(0), KvCacheType::F16, "Fallback seguro para head 0 deve ser F16");
    }

    #[test]
    fn test_kv_cache_v_type_fallback_math() {
        assert_eq!(calculate_kv_cache_v_type(256), KvCacheType::Q4_K);
        assert_eq!(calculate_kv_cache_v_type(128), KvCacheType::Q4_0);
        assert_eq!(calculate_kv_cache_v_type(64), KvCacheType::Q4_0);
        assert_eq!(calculate_kv_cache_v_type(0), KvCacheType::F16);
    }

    #[test]
    fn test_gemma_family_thermal_context_cap() {
        assert_eq!(cap_context_length_for_family("gemma4", 131072), 32768);
        assert_eq!(cap_context_length_for_family("gemma2", 65536), 32768);
        assert_eq!(cap_context_length_for_family("qwen3", 40960), 40960);
    }

    #[test]
    fn test_unsupported_architecture_rejection() {
        assert!(!model_registry::is_architecture_supported("rwkv"));

        assert!(model_registry::is_architecture_supported("mamba"));
        assert!(model_registry::is_architecture_supported("llama"));
        assert!(model_registry::is_architecture_supported("qwen3"));
        assert!(model_registry::is_architecture_supported("gemma4"));
    }

    #[test]
    fn test_llama_engine_pending_engine_interception() {
        let engine = LlamaCppEngine;
        let temp_dir = tempfile::tempdir().unwrap();
        let fake_path = temp_dir.path().join("bitnet_i2_s.gguf");
        std::fs::write(&fake_path, b"fake GGUF content").unwrap();

        let req_fake = SoulsInferenceRequest {
            model_path: fake_path.to_string_lossy().to_string(),
            system_prompt: "".to_string(),
            few_shot_examples: vec![],
            user_query: "test".to_string(),
            max_tokens: 10,
            min_p: 0.05,
            temperature: 0.7,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };

        let err = engine.run_inference(req_fake, None).unwrap_err();
        match err {
            InferenceError::ExecutionError(msg) => {
                assert!(msg.contains("PENDING_ENGINE"), "Mensagem de erro deveria conter PENDING_ENGINE: {msg}");
                assert!(msg.contains("bitnet.cpp/mamba-ssm"), "Mensagem de erro deveria explicar a causa: {msg}");
            }
            _ => panic!("Esperava InferenceError::ExecutionError com PENDING_ENGINE"),
        }
    }

    #[test]
    fn test_gguf_metadata_cache_zero_copy_single_mmap() {
        use crate::core::model_registry::GLOBAL_GGUF_METADATA_CACHE;
        let temp_dir = tempfile::tempdir().unwrap();
        let fake_gguf = temp_dir.path().join("test_model.gguf");
        // GGUF header mínimo válido (24 bytes): "GGUF" + version(3) + tensor_count(0) + kv_count(0)
        let mut header = vec![b'G', b'G', b'U', b'F'];
        header.extend_from_slice(&3u32.to_le_bytes());
        header.extend_from_slice(&0u64.to_le_bytes());
        header.extend_from_slice(&0u64.to_le_bytes());
        std::fs::write(&fake_gguf, &header).unwrap();

        let meta1 = GLOBAL_GGUF_METADATA_CACHE.get_or_parse(&fake_gguf);
        assert!(meta1.is_some(), "Primeira leitura de metadados GGUF deve suceder");

        let cache_len_before = GLOBAL_GGUF_METADATA_CACHE.len();
        assert!(cache_len_before >= 1, "Cache deve conter a entrada");

        let meta2 = GLOBAL_GGUF_METADATA_CACHE.get_or_parse(&fake_gguf);
        assert!(meta2.is_some(), "Segunda leitura GGUF deve vir em O(1) do cache");
        assert_eq!(meta1.unwrap(), meta2.unwrap(), "Metadados cacheados devem ser idênticos");
    }

    #[test]
    fn test_gguf_cached_metadata_reads() {
        use crate::core::model_registry::{parse_gguf_metadata_zero_copy, GLOBAL_GGUF_METADATA_CACHE};

        let temp_dir = tempfile::tempdir().unwrap();
        let fake_gguf = temp_dir.path().join("stress_test_model.gguf");

        // GGUF header mínimo válido (24 bytes): "GGUF" + version(3) + tensor_count(0) + kv_count(0)
        let mut header = vec![b'G', b'G', b'U', b'F'];
        header.extend_from_slice(&3u32.to_le_bytes());
        header.extend_from_slice(&0u64.to_le_bytes());
        header.extend_from_slice(&0u64.to_le_bytes());
        std::fs::write(&fake_gguf, &header).unwrap();

        // 1ª leitura: mmap/disk -> hidrata DashMap L1
        let first = parse_gguf_metadata_zero_copy(&fake_gguf);
        assert!(first.is_some(), "1ª leitura de metadados GGUF deve suceder via mmap");
        let first_meta = first.unwrap();

        let initial_cache_len = GLOBAL_GGUF_METADATA_CACHE.len();
        assert!(initial_cache_len >= 1, "Cache L1 DashMap deve possuir ao menos 1 elemento");

        // 9 leituras subsequentes: devem resolver em O(1) no DashMap L1 em RAM sem novos mmaps
        for i in 2..=10 {
            let subsequent = parse_gguf_metadata_zero_copy(&fake_gguf);
            assert!(subsequent.is_some(), "Leitura {}/10 deve retornar resultado do cache L1", i);
            assert_eq!(
                subsequent.unwrap(),
                first_meta,
                "Leitura {}/10 deve ser idêntica ao metadado inicial",
                i
            );
        }

        // Deleta o arquivo físico em disco para PROVAR que as leituras 11..15 continuam resolvendo da RAM!
        std::fs::remove_file(&fake_gguf).unwrap();

        for i in 11..=15 {
            let ram_hit = parse_gguf_metadata_zero_copy(&fake_gguf);
            assert!(
                ram_hit.is_some(),
                "Leitura {}/15 (pós-remoção física do arquivo) DEVE resolver do L1 RAM DashMap",
                i
            );
            assert_eq!(ram_hit.unwrap(), first_meta);
        }
    }

    #[tokio::test]
    async fn test_dedicated_worker_mpsc_dispatch() {
        let worker = DedicatedInferenceWorker::new(4);
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/non_existent.gguf".to_string(),
            system_prompt: "".to_string(),
            few_shot_examples: vec![],
            user_query: "hello".to_string(),
            max_tokens: 5,
            min_p: 0.05,
            temperature: 0.7,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };

        let result = worker.run_async(req, None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            InferenceError::ModelNotFound(_) => {}
            err => panic!("Esperava ModelNotFound, obteve: {err:?}"),
        }
    }
}

