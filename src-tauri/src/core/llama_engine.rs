use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::Arc;

use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::context::params::{LlamaContextParams, KvCacheType};
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::token::LlamaToken;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::sampling::LlamaSampler;

use llguidance::{Constraint, ParserFactory, api::{TopLevelGrammar, GrammarWithLexer}};
use llguidance::toktrie::{TokTrie, TokRxInfo, ApproximateTokEnv, TokEnv};

use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::core::model_registry::{self, parse_gguf_metadata_zero_copy};
use crate::souls_thermal_governor::SystemState;
use tokio::sync::watch;

use std::sync::OnceLock;

static GLOBAL_LLAMA_BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();

fn get_global_llama_backend() -> Result<&'static LlamaBackend, InferenceError> {
    let res = GLOBAL_LLAMA_BACKEND.get_or_init(|| {
        LlamaBackend::init().map_err(|e| format!("Falha ao inicializar LlamaBackend: {}", e))
    });

    match res {
        Ok(backend) => Ok(backend),
        Err(err) => Err(InferenceError::ExecutionError(err.clone())),
    }
}

pub struct LlamaCppEngine;

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
    if n_embd_head_v > 0 && n_embd_head_v.is_multiple_of(256) {
        KvCacheType::Q4_K
    } else {
        // Fallback matemático silencioso para Q8_0 (tamanho de bloco 32) prevenindo pânico na C-FFI
        KvCacheType::Q8_0
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

    let mut params = LlamaContextParams::default()
        .with_n_ctx(std::num::NonZeroU32::new(n_ctx.max(512)))
        .with_n_batch(4096)
        .with_type_k(KvCacheType::F16)
        .with_type_v(type_v);

    let lower_fam = family.to_lowercase();
    if let Some(factor) = rope_attn_factor {
        params = params
            .with_rope_freq_scale(factor)
            .with_yarn_attn_factor(factor)
            .with_rope_scaling_type(llama_cpp_2::context::params::RopeScalingType::Linear);
    } else if lower_fam.contains("phi") || lower_fam.contains("phi3") || lower_fam.contains("phi4") {
        params = params
            .with_rope_freq_scale(1.190238)
            .with_yarn_attn_factor(1.190238)
            .with_rope_scaling_type(llama_cpp_2::context::params::RopeScalingType::Linear);
    }

    params
}

pub fn build_default_context_params() -> LlamaContextParams {
    build_context_params_with_fallback(256, 4096, "", None)
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

        // 0. Interceptação Fail-Soft de Arquiteturas Ternárias (BitNet i2_s/i1_s) ou Recorrentes (Mamba/Zamba/RWKV)
        let path_lower = req.model_path.to_lowercase();
        let is_ternary = path_lower.contains("i2_s") || path_lower.contains("i1_s") || path_lower.contains("bitnet");
        let is_recurrent = if let Some(ref meta) = gguf_meta {
            let fam = meta.family.to_lowercase();
            fam.contains("mamba") || fam.contains("zamba") || fam.contains("rwkv")
        } else {
            path_lower.contains("mamba") || path_lower.contains("zamba") || path_lower.contains("rwkv")
        };

        if is_ternary || is_recurrent {
            return Err(InferenceError::ExecutionError(
                "PENDING_ENGINE: Este modelo requer engine especializada (bitnet.cpp/mamba-ssm) ainda não integrada no runtime".to_string()
            ));
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

        // 2. Parâmetros do modelo
        let model_params = LlamaModelParams::default().with_n_gpu_layers(99);

        let model = LlamaModel::load_from_file(backend, model_path, &model_params).map_err(|e| {
            InferenceError::ExecutionError(format!("Falha ao carregar modelo GGUF '{}': {}", req.model_path, e))
        })?;

        // 3. Alocação do contexto com KV Cache Assimétrico & RoPE Scaling Params
        let (n_embd_head_v, declared_ctx, family, rope_attn_factor) = if let Some(ref meta) = gguf_meta {
            let h_kv = meta.architecture.head_count_kv.max(1);
            let head_v = if meta.architecture.embedding_length > 0 {
                meta.architecture.embedding_length / h_kv
            } else {
                128
            };
            (head_v, meta.context_length as u32, meta.family.clone(), meta.architecture.rope_scaling_attn_factor)
        } else {
            (256, 4096, String::new(), None)
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

        // 7. Cadeia de Samplers Nativos (DRY, Temp, Min-P, Dist)
        let sampler_dry = LlamaSampler::dry(&model, 0.8, 1.75, 2, 512, ["\n", ":", "\"", "{", "}"]);
        let sampler_temp = LlamaSampler::temp(req.temperature);
        let sampler_min_p = LlamaSampler::min_p(req.min_p, 1);
        let sampler_dist = LlamaSampler::dist(0);

        let samplers = vec![sampler_dry, sampler_temp, sampler_min_p, sampler_dist];
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
                        candidates.apply_sampler(&sampler);
                        let sampled = candidates.selected_token().unwrap_or_else(|| {
                            sampler.sample(&ctx, sample_batch_idx)
                        });
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
            let token_str = model.token_to_str(token, llama_cpp_2::model::Special::Tokenize).map_err(|e| {
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

pub fn disable_model_in_sqlite(model_path: &str) {
    let db_path = crate::core::model_registry::resolve_db_path();
    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        let _ = conn.execute(
            "UPDATE model_registry SET is_active = 0 WHERE file_path = ?1 OR model_id = ?1",
            [model_path],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llama_engine_context_init_asymmetric_kv_cache() {
        let params = build_default_context_params();
        let type_k = params.type_k();
        let type_v = params.type_v();
        assert_eq!(type_k, KvCacheType::F16, "Keys no KV Cache devem ser F16 para rotação RoPE");
        assert_eq!(type_v, KvCacheType::Q4_K, "Values no KV Cache devem ser Q4_K para esmagar o footprint < 1GB");
    }

    #[test]
    fn test_kv_cache_v_type_fallback_math() {
        assert_eq!(calculate_kv_cache_v_type(256), KvCacheType::Q4_K);
        assert_eq!(calculate_kv_cache_v_type(128), KvCacheType::Q8_0);
        assert_eq!(calculate_kv_cache_v_type(64), KvCacheType::Q8_0);
    }

    #[test]
    fn test_gemma_family_thermal_context_cap() {
        assert_eq!(cap_context_length_for_family("gemma4", 131072), 32768);
        assert_eq!(cap_context_length_for_family("gemma2", 65536), 32768);
        assert_eq!(cap_context_length_for_family("qwen3", 40960), 40960);
    }

    #[test]
    fn test_unsupported_architecture_rejection() {
        assert!(!model_registry::is_architecture_supported("zamba2"));
        assert!(!model_registry::is_architecture_supported("mamba"));
        assert!(!model_registry::is_architecture_supported("rwkv"));

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
}

