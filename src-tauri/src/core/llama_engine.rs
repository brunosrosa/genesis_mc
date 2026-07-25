use std::path::Path;
use std::time::Instant;

use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::context::params::{LlamaContextParams, KvCacheType};
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::sampling::LlamaSampler;

use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SodaInferenceRequest, SodaInferenceResponse,
};

use crate::soda_thermal_governor::SystemState;
use tokio::sync::watch;

pub struct LlamaCppEngine;

fn build_chat_prompt(system_prompt: &str, few_shots: &[(String, String)], user_query: &str) -> String {
    let mut prompt = String::new();
    if !system_prompt.trim().is_empty() {
        prompt.push_str("<|im_start|>system\n");
        prompt.push_str(system_prompt.trim());
        prompt.push_str("<|im_end|>\n");
    }
    for (input, output) in few_shots {
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

impl EphemeralInferEngine for LlamaCppEngine {
    fn run_inference(
        &self,
        req: SodaInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SodaInferenceResponse, InferenceError> {
        let start_time = Instant::now();

        let model_path = Path::new(&req.model_path);
        if !model_path.exists() {
            return Err(InferenceError::ModelNotFound(req.model_path.clone()));
        }

        // 1. Inicializa backend bare-metal do llama.cpp
        let backend = LlamaBackend::init().map_err(|e| {
            InferenceError::ExecutionError(format!("Falha ao inicializar LlamaBackend: {}", e))
        })?;

        // 2. Parâmetros do modelo - ADR-027: use_mmap = true por padrão no LlamaModelParams
        let model_params = LlamaModelParams::default();

        let model = LlamaModel::load_from_file(&backend, model_path, &model_params).map_err(|e| {
            InferenceError::ExecutionError(format!("Falha ao carregar modelo GGUF '{}': {}", req.model_path, e))
        })?;

        // 3. Alocação do contexto com KV Cache comprimido em 4-bit (KvCacheType::Q4_K para economizar VRAM)
        let ctx_params = LlamaContextParams::default()
            .with_type_k(KvCacheType::Q4_K)
            .with_type_v(KvCacheType::Q4_K);

        let mut ctx = model.new_context(&backend, ctx_params).map_err(|_| {
            InferenceError::GpuOom
        })?;

        // 4. Concatenação estruturada Few-Shot & Tokenização do prompt final
        let formatted_prompt = build_chat_prompt(&req.system_prompt, &req.few_shot_examples, &req.user_query);

        let prompt_tokens_vec = model.str_to_token(&formatted_prompt, llama_cpp_2::model::AddBos::Always).map_err(|e| {
            InferenceError::ExecutionError(format!("Falha na tokenização do prompt: {}", e))
        })?;

        let prompt_tokens_count = prompt_tokens_vec.len() as u32;

        if prompt_tokens_vec.is_empty() {
            return Err(InferenceError::ExecutionError("Prompt resultou em 0 tokens".to_string()));
        }

        // 5. Alocação DINÂMICA do LlamaBatch
        let batch_capacity = (prompt_tokens_count + req.max_tokens).max(512) as usize;
        let mut batch = LlamaBatch::new(batch_capacity, 1);

        let last_idx = prompt_tokens_vec.len() - 1;
        for (i, &token) in prompt_tokens_vec.iter().enumerate() {
            let is_last = i == last_idx;
            batch.add(token, i as i32, &[0], is_last).map_err(|e| {
                InferenceError::ExecutionError(format!("Falha ao adicionar token ao batch: {}", e))
            })?;
        }

        ctx.decode(&mut batch).map_err(|e| {
            InferenceError::ExecutionError(format!("Falha ao decodificar batch inicial: {}", e))
        })?;

        // 6. Construção da Cadeia de Samplers com Algema Gramatical JSON (ADR-028) e DRY / Repetition Penalty
        let mut samplers = Vec::new();

        if let Some(ref schema) = req.json_schema {
            let gbnf_grammar = llama_cpp_2::json_schema_to_grammar(schema).map_err(|e| {
                InferenceError::GrammarMaskError(format!("Falha ao converter JSON schema para GBNF: {}", e))
            })?;
            let grammar_sampler = LlamaSampler::grammar(&model, &gbnf_grammar, "root").map_err(|e| {
                InferenceError::GrammarMaskError(format!("Falha ao inicializar sampler de gramática: {}", e))
            })?;
            samplers.push(grammar_sampler);
        }

        let sampler_dry = LlamaSampler::dry(&model, 0.8, 1.75, 2, 512, ["\n", ":", "\"", "{", "}"]);
        let sampler_temp = LlamaSampler::temp(req.temperature);
        let sampler_min_p = LlamaSampler::min_p(req.min_p, 1);
        let sampler_dist = LlamaSampler::dist(0);

        samplers.push(sampler_dry);
        samplers.push(sampler_temp);
        samplers.push(sampler_min_p);
        samplers.push(sampler_dist);

        let mut sampler = LlamaSampler::chain_simple(samplers);

        // 7. Loop de Geração Autoregressiva Efêmera
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

            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            if model.is_eog_token(token) {
                break;
            }

            #[allow(deprecated)]
            let token_str = model.token_to_str(token, llama_cpp_2::model::Special::Tokenize).map_err(|e| {
                InferenceError::ExecutionError(format!("Falha ao decodificar token para texto: {}", e))
            })?;
            generated_text.push_str(&token_str);
            completion_tokens_count += 1;

            batch.clear();
            batch.add(token, current_pos, &[0], true).map_err(|e| {
                InferenceError::ExecutionError(format!("Falha no batch de geração: {}", e))
            })?;
            current_pos += 1;

            if let Err(e) = ctx.decode(&mut batch) {
                return Err(InferenceError::ExecutionError(format!("Falha no decode autoregressivo: {}", e)));
            }
        }

        let total_latency_ms = start_time.elapsed().as_millis() as u64;

        Ok(SodaInferenceResponse {
            status: "success".to_string(),
            text: generated_text,
            prompt_tokens: prompt_tokens_count,
            completion_tokens: completion_tokens_count,
            total_latency_ms,
        })
    }
}
