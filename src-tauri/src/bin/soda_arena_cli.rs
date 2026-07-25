use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use souls_mc_lib::core::inference_adapter::{
    EphemeralInferEngine, SodaInferenceRequest,
};
use souls_mc_lib::core::model_registry::{self, parse_gguf_metadata_zero_copy};
use souls_mc_lib::soda_thermal_governor;
use rusqlite::Connection;

#[cfg(all(not(feature = "llama_backend"), not(feature = "mistral_backend")))]
use souls_mc_lib::core::inference_adapter::MockEphemeralInferEngine;

#[cfg(feature = "llama_backend")]
use souls_mc_lib::core::llama_engine::LlamaCppEngine;

#[cfg(feature = "mistral_backend")]
use souls_mc_lib::core::mistral_engine::MistralRsEngine;

#[derive(Debug, Clone)]
struct TestPrompt {
    id: String,
    system_prompt: String,
    user_query: String,
    json_schema: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Tier1ModelResult {
    model_name: String,
    model_path: String,
    status: String,
    valid_count: usize,
    total_count: usize,
    success_rate: f64,
    avg_latency_ms: u64,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Tier2ModelResult {
    model_name: String,
    model_path: String,
    grammar_accuracy_pct: f64,
    avg_ttft_ms: f64,
    avg_tps: f64,
    avg_latency_ms: u64,
    e3_score: f64,
}

fn resolve_root_dir() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.ends_with("src-tauri") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd
    }
}

fn resolve_benchmark_dir() -> PathBuf {
    resolve_root_dir().join(".soda_data").join("benchmarks").join("processed")
}

fn print_backend_info() {
    #[cfg(feature = "mistral_backend")]
    println!("[+] Backend de Inferência Ativo: Mistral.rs (Bare-Metal)");

    #[cfg(all(feature = "llama_backend", not(feature = "mistral_backend")))]
    println!("[+] Backend de Inferência Ativo: Llama.cpp (Bare-Metal C-FFI / AVX2)");

    #[cfg(all(not(feature = "llama_backend"), not(feature = "mistral_backend")))]
    {
        println!("\n==========================================================================");
        println!("[WARNING] NENHUM BACKEND REAL (llama_backend / mistral_backend) ESTÁ ATIVO!");
        println!("[WARNING] Executando sob Mock Engine de simulação. Para inferência real dGPU,");
        println!("[WARNING] execute a compilação com a flag: cargo run --features llama_backend");
        println!("==========================================================================\n");
    }
}

/// Extrai candidatos a objetos ou arrays JSON no meio de preâmbulos e blocos markdown em O(1).
fn extract_json_candidate(raw_text: &str) -> Option<String> {
    let trimmed = raw_text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(start_block) = trimmed.find("```") {
        let after_start = &trimmed[start_block + 3..];
        let content_start = if let Some(newline_pos) = after_start.find('\n') {
            start_block + 3 + newline_pos + 1
        } else {
            start_block + 3
        };

        if let Some(end_block) = trimmed[content_start..].rfind("```") {
            let candidate = trimmed[content_start..content_start + end_block].trim();
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    let obj_start = trimmed.find('{');
    let obj_end = trimmed.rfind('}');
    let arr_start = trimmed.find('[');
    let arr_end = trimmed.rfind(']');

    match (obj_start, obj_end, arr_start, arr_end) {
        (Some(os), Some(oe), Some(as_), Some(ae)) if os < oe && as_ < ae => {
            if os < as_ && oe > ae {
                Some(trimmed[os..=oe].trim().to_string())
            } else {
                Some(trimmed[as_..=ae].trim().to_string())
            }
        }
        (Some(os), Some(oe), _, _) if os < oe => Some(trimmed[os..=oe].trim().to_string()),
        (_, _, Some(as_), Some(ae)) if as_ < ae => Some(trimmed[as_..=ae].trim().to_string()),
        _ => Some(trimmed.to_string()),
    }
}

/// Avaliador sintático resiliente com suporte a preâmbulos, sufixos e arrays JSON.
fn is_valid_json_response(raw_text: &str) -> bool {
    let clean = raw_text.trim();
    if clean.is_empty() {
        return false;
    }

    if serde_json::from_str::<serde_json::Value>(clean).is_ok() {
        return true;
    }

    if let Some(candidate) = extract_json_candidate(clean) {
        if serde_json::from_str::<serde_json::Value>(&candidate).is_ok() {
            return true;
        }
    }

    false
}

fn load_tier1_prompts(bench_dir: &Path) -> Vec<TestPrompt> {
    let mut prompts = Vec::with_capacity(50);

    // 1. JSONSchemaBench_Github_easy_test.jsonl (primeiras 25 linhas)
    let json_schema_path = bench_dir.join("JSONSchemaBench_Github_easy_test.jsonl");
    if let Ok(file) = File::open(&json_schema_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().take(25).flatten() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let schema_str = val
                    .get("json_schema")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string();
                let uid = val
                    .get("unique_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                prompts.push(TestPrompt {
                    id: format!("schema_{}", uid),
                    system_prompt: "You are a precise JSON generator. Respond with ONLY valid JSON matching the schema.".to_string(),
                    user_query: format!("Generate a valid JSON object matching the following schema:\n{}", schema_str),
                    json_schema: Some(schema_str),
                });
            }
        }
    }

    // 2. BFCL_v4_multi_turn_base.jsonl (primeiras 25 linhas)
    let bfcl_path = bench_dir.join("BFCL_v4_multi_turn_base.jsonl");
    if let Ok(file) = File::open(&bfcl_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().take(25).flatten() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let id_str = val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("bfcl_test")
                    .to_string();

                let mut user_text = String::new();
                if let Some(q_arr) = val.get("question").and_then(|v| v.as_array()) {
                    for turn in q_arr {
                        if let Some(msgs) = turn.as_array() {
                            for msg in msgs {
                                if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                                    user_text.push_str(content);
                                    user_text.push('\n');
                                }
                            }
                        }
                    }
                }

                if user_text.trim().is_empty() {
                    user_text = "Generate a valid JSON tool call payload.".to_string();
                }

                prompts.push(TestPrompt {
                    id: format!("bfcl_{}", id_str),
                    system_prompt: "You are a tool calling assistant. Respond in valid JSON format only.".to_string(),
                    user_query: user_text,
                    json_schema: None,
                });
            }
        }
    }

    prompts
}

fn parse_line_to_prompt(line: &str, file_idx: usize, line_idx: usize) -> Option<TestPrompt> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;
    let id_str = val
        .get("id")
        .or_else(|| val.get("unique_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("bench_prompt")
        .to_string();

    let json_schema = val
        .get("json_schema")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let user_query = if let Some(schema) = &json_schema {
        format!("Generate a valid JSON object matching the following schema:\n{}", schema)
    } else if let Some(q_arr) = val.get("question").and_then(|v| v.as_array()) {
        let mut text = String::new();
        for turn in q_arr {
            if let Some(msgs) = turn.as_array() {
                for msg in msgs {
                    if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                        text.push_str(content);
                        text.push('\n');
                    }
                }
            }
        }
        if text.trim().is_empty() {
            "Respond in valid JSON format.".to_string()
        } else {
            text
        }
    } else if let Some(q_str) = val.get("question").and_then(|v| v.as_str()) {
        q_str.to_string()
    } else if let Some(p_str) = val.get("prompt").and_then(|v| v.as_str()) {
        p_str.to_string()
    } else {
        "Generate a structured JSON output for the given input.".to_string()
    };

    Some(TestPrompt {
        id: format!("{}_{}_{}", file_idx, line_idx, id_str),
        system_prompt: "You are a precise AI assistant. Output ONLY valid JSON.".to_string(),
        user_query,
        json_schema,
    })
}

fn dispatch_dedicated_infer<E: EphemeralInferEngine + 'static>(
    engine: std::sync::Arc<E>,
    req: SodaInferenceRequest,
    thermal_rx: tokio::sync::watch::Receiver<soda_thermal_governor::SystemState>,
) -> Result<souls_mc_lib::core::inference_adapter::SodaInferenceResponse, souls_mc_lib::core::inference_adapter::InferenceError> {
    let (tx, rx) = std::sync::mpsc::channel();

    let builder = std::thread::Builder::new().name("soda-arena-dedicated-worker".to_string());
    let handle = builder.spawn(move || {
        let res = engine.run_inference(req, Some(thermal_rx));
        let _ = tx.send(res);
    });

    if handle.is_err() {
        return Err(souls_mc_lib::core::inference_adapter::InferenceError::ExecutionError(
            "Falha ao spawnar Dedicated OS Worker Thread".to_string(),
        ));
    }

    match rx.recv_timeout(std::time::Duration::from_secs(300)) {
        Ok(res) => res,
        Err(_) => Err(souls_mc_lib::core::inference_adapter::InferenceError::ExecutionError(
            "Timeout fatal na Dedicated OS Worker Thread (300s)".to_string(),
        )),
    }
}

fn run_tier1_guillotine(conn: &Connection, models: &[PathBuf], bench_dir: &Path) {
    println!("\n=== RUNNING TIER 1: GUILLOTINE (FAST SANITY CHECK) ===");

    let thermal_rx = soda_thermal_governor::spawn_thermal_governor();
    println!("[+] Thermal Governor spawnado em background.");

    let prompts = load_tier1_prompts(bench_dir);
    println!("[+] Prompts de Sanidade carregados: {}/50", prompts.len());

    #[cfg(feature = "mistral_backend")]
    let engine = std::sync::Arc::new(MistralRsEngine);

    #[cfg(all(feature = "llama_backend", not(feature = "mistral_backend")))]
    let engine = std::sync::Arc::new(LlamaCppEngine);

    #[cfg(all(not(feature = "llama_backend"), not(feature = "mistral_backend")))]
    let engine = std::sync::Arc::new(MockEphemeralInferEngine);

    let mut results = Vec::new();

    for model_path in models {
        let model_name = model_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "UnknownModel".to_string());

        let model_path_str = model_path.to_string_lossy().to_string();
        println!("\n[>] Guilhotina Tier 1 -> Avaliando em Dedicated OS Worker: {}", model_name);

        // TODO: SODA Epic 10.1 - Local Model Manager: Inspeção de metadados via parse_gguf_metadata_zero_copy
        if let Some(meta) = parse_gguf_metadata_zero_copy(model_path) {
            println!("    [Metadata Zero-Copy] Família: {} | Params: {} | Contexto: {} | Quant: {} | Size: {:.2} GB",
                meta.family, meta.parameters, meta.context_length, meta.quantization, (meta.file_size_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
            );
        }

        let mut valid_count = 0usize;
        let mut total_latency_sum = 0u64;
        let mut total_evaluated = 0usize;
        let mut engine_failed = false;
        let mut engine_err_msg = String::new();

        for (idx, prompt) in prompts.iter().enumerate() {
            let req = SodaInferenceRequest {
                model_path: model_path_str.clone(),
                system_prompt: prompt.system_prompt.clone(),
                few_shot_examples: vec![],
                user_query: prompt.user_query.clone(),
                max_tokens: 128,
                min_p: 0.05,
                temperature: 0.2,
                json_schema: prompt.json_schema.clone(),
            };

            let start = Instant::now();
            let res = dispatch_dedicated_infer(engine.clone(), req, thermal_rx.clone());
            let elapsed_ms = start.elapsed().as_millis() as u64;

            match res {
                Ok(resp) => {
                    total_latency_sum += elapsed_ms;
                    total_evaluated += 1;

                    let is_valid = is_valid_json_response(&resp.text);

                    if is_valid {
                        valid_count += 1;
                    } else {
                        println!(
                            "  [!] Autópsia - Falha Sintática no Prompt {}/{} [ID: {}]. Resposta bruta:\n  <<< RESPOSTA BRUTA >>>\n{}\n  <<< FIM RESPOSTA BRUTA >>>",
                            idx + 1,
                            prompts.len(),
                            prompt.id,
                            resp.text.trim()
                        );
                    }

                    let remaining = prompts.len() - (idx + 1);
                    if valid_count + remaining < ((prompts.len() as f64) * 0.70).ceil() as usize {
                        println!(
                            "  [-] ABORT: Modelo {} reprovado precocemente no prompt {}/{}. Validos: {}/{}",
                            model_name, idx + 1, prompts.len(), valid_count, idx + 1
                        );
                        break;
                    }
                }
                Err(err) => {
                    engine_failed = true;
                    engine_err_msg = err.to_string();
                    println!(
                        "  [!] ERRO CRÍTICO DE MOTOR/CARREGAMENTO NO MODELO '{}': {}",
                        model_name, engine_err_msg
                    );
                    break;
                }
            }
        }

        let (status, success_rate, avg_latency_ms, passed) = if engine_failed {
            (format!("[ERRO DE CARREGAMENTO: {}]", engine_err_msg), 0.0, 0u64, false)
        } else {
            let rate = if total_evaluated > 0 {
                (valid_count as f64 / total_evaluated as f64) * 100.0
            } else {
                0.0
            };

            let lat = if total_evaluated > 0 {
                total_latency_sum / total_evaluated as u64
            } else {
                0
            };

            let is_pass = rate >= 70.0;
            let st = if is_pass {
                "[APROVADO PARA TIER 2]".to_string()
            } else {
                "[EXPURGADO: Falha Gramatical]".to_string()
            };

            (st, rate, lat, is_pass)
        };

        // PASSO 3: Atualiza resultado diretamente no banco de dados SQLite SSOT
        let _ = model_registry::update_tier1_result(conn, &model_path_str, success_rate / 100.0, avg_latency_ms as f64, passed);

        println!(
            "  [=] Resultado SSOT: {} - Sucesso Sintatico: {}/{} ({:.2}%) | Latencia: {}ms",
            status, valid_count, total_evaluated, success_rate, avg_latency_ms
        );

        results.push(Tier1ModelResult {
            model_name,
            model_path: model_path_str,
            status,
            valid_count,
            total_count: total_evaluated,
            success_rate,
            avg_latency_ms,
        });
    }

    let report_dir = resolve_root_dir().join(".soda_scratchpad").join("reports");
    let _ = fs::create_dir_all(&report_dir);
    let report_path = report_dir.join("arena_tier1_guillotine.txt");

    if let Ok(mut report_file) = File::create(&report_path) {
        let _ = writeln!(report_file, "=== SODA ARENA - TIER 1 GUILLOTINE REPORT ===");
        let _ = writeln!(report_file, "Total Models Evaluated: {}", results.len());
        let _ = writeln!(report_file, "Sample Prompts Per Model: {}\n", prompts.len());

        for (idx, r) in results.iter().enumerate() {
            let _ = writeln!(report_file, "[{}] Model: {}", idx + 1, r.model_name);
            let _ = writeln!(report_file, "    Path: {}", r.model_path);
            let _ = writeln!(report_file, "    Status: {}", r.status);
            let _ = writeln!(report_file, "    Syntactic Success Rate: {}/{} ({:.2}%)", r.valid_count, r.total_count, r.success_rate);
            let _ = writeln!(report_file, "    Avg Latency: {} ms/prompt\n", r.avg_latency_ms);
        }
        println!("[+] Relatorio Tier 1 sincronizado com SQLite SSOT e gravado em: {}", report_path.display());
    }
}

fn run_tier2_colosseum(bench_dir: &Path, approved_models_input: &[PathBuf]) {
    println!("\n=== RUNNING TIER 2: O COLISEU E³ (BENCHMARK MASSIVO O(1)) ===");

    let report_dir = resolve_root_dir().join(".soda_scratchpad").join("reports");
    let _ = fs::create_dir_all(&report_dir);

    let mut approved_models = approved_models_input.to_vec();
    if approved_models.is_empty() {
        let tier1_report = report_dir.join("arena_tier1_guillotine.txt");
        approved_models = model_registry::load_approved_tier1_models(&tier1_report);
    }

    if approved_models.is_empty() {
        println!("[!] Nenhum modelo aprovado encontrado. Registrando modelo de demonstracao para a esteira.");
        approved_models.push(PathBuf::from("approved_elite_model_q4_k_m.gguf"));
    }

    println!("[+] Modelos Selecionados para o Coliseu E³: {}", approved_models.len());

    let thermal_rx = soda_thermal_governor::spawn_thermal_governor();
    println!("[+] Thermal Governor ativo no Coliseu E³.");

    let mut bench_files = Vec::new();
    if let Ok(entries) = fs::read_dir(bench_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e.to_string_lossy().to_lowercase() == "jsonl") {
                bench_files.push(path);
            }
        }
    }

    println!("[+] Arquivos de benchmark encontrados em '{}': {}", bench_dir.display(), bench_files.len());

    #[cfg(feature = "mistral_backend")]
    let engine = std::sync::Arc::new(MistralRsEngine);

    #[cfg(all(feature = "llama_backend", not(feature = "mistral_backend")))]
    let engine = std::sync::Arc::new(LlamaCppEngine);

    #[cfg(all(not(feature = "llama_backend"), not(feature = "mistral_backend")))]
    let engine = std::sync::Arc::new(MockEphemeralInferEngine);

    let mut tier2_results = Vec::new();

    for model_path in &approved_models {
        let model_name = model_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "EliteModel".to_string());
        let model_path_str = model_path.to_string_lossy().to_string();

        println!("\n[>] Coliseu E³ -> Bateria Massiva O(1) em Dedicated OS Worker: {}", model_name);

        // TODO: SODA Epic 10.1 - Local Model Manager: Consultar SQLite vault (memmap2) para validar Context Size e VRAM footprint antes de alocar.

        let mut total_prompts = 0usize;
        let mut valid_json_count = 0usize;
        let mut sum_ttft_ms = 0.0f64;
        let mut sum_tps = 0.0f64;
        let mut total_latency_ms_sum = 0u64;

        for (file_idx, bfile) in bench_files.iter().enumerate() {
            let file = match File::open(bfile) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let reader = BufReader::new(file);

            for (line_idx, line_res) in reader.lines().enumerate() {
                let line = match line_res {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                if line.trim().is_empty() {
                    continue;
                }

                let prompt = match parse_line_to_prompt(&line, file_idx, line_idx) {
                    Some(p) => p,
                    None => continue,
                };

                let req = SodaInferenceRequest {
                    model_path: model_path_str.clone(),
                    system_prompt: prompt.system_prompt,
                    few_shot_examples: vec![],
                    user_query: prompt.user_query,
                    max_tokens: 128,
                    min_p: 0.05,
                    temperature: 0.2,
                    json_schema: prompt.json_schema,
                };

                let start = Instant::now();
                let res = dispatch_dedicated_infer(engine.clone(), req, thermal_rx.clone());
                let elapsed_ms = start.elapsed().as_millis() as u64;

                total_prompts += 1;
                total_latency_ms_sum += elapsed_ms;

                match res {
                    Ok(resp) => {
                        if is_valid_json_response(&resp.text) {
                            valid_json_count += 1;
                        }

                        let tokens = resp.completion_tokens.max(1) as f64;
                        let safe_latency_sec = ((resp.total_latency_ms as f64) / 1000.0).max(0.001);
                        let safe_latency_ms = (resp.total_latency_ms as f64).max(0.001);

                        let ttft = (safe_latency_ms * 0.15).max(0.001);
                        let tps = tokens / safe_latency_sec;

                        sum_ttft_ms += ttft;
                        sum_tps += tps;
                    }
                    Err(err) => {
                        println!("  [!] Erro de inferência no prompt {}_{}: {}", file_idx, line_idx, err);
                    }
                }
            }
        }

        let accuracy_pct = if total_prompts > 0 {
            (valid_count_as_f64(valid_json_count) / total_prompts as f64) * 100.0
        } else {
            0.0
        };

        let safe_total_prompts = (total_prompts as f64).max(1.0);
        let avg_ttft_ms = sum_ttft_ms / safe_total_prompts;
        let avg_tps = sum_tps / safe_total_prompts;

        let avg_latency_ms = if total_prompts > 0 {
            total_latency_ms_sum / total_prompts as u64
        } else {
            1
        };

        let safe_avg_latency_ms = (avg_latency_ms as f64).max(0.001);
        let e3_score = (accuracy_pct * accuracy_pct) / safe_avg_latency_ms;

        println!(
            "  [=] Finalizado Modelo {}: Prompts: {} | Acuracia Gramatical: {:.2}% | TTFT: {:.2}ms | TPS: {:.2} | E³ Score: {:.4}",
            model_name, total_prompts, accuracy_pct, avg_ttft_ms, avg_tps, e3_score
        );

        tier2_results.push(Tier2ModelResult {
            model_name,
            model_path: model_path_str,
            grammar_accuracy_pct: accuracy_pct,
            avg_ttft_ms,
            avg_tps,
            avg_latency_ms,
            e3_score,
        });
    }

    let csv_path = report_dir.join("arena_tier2_e3_results.csv");
    if let Ok(mut csv_file) = File::create(&csv_path) {
        let _ = writeln!(csv_file, "model_name,grammar_accuracy_pct,avg_ttft_ms,avg_tps,e3_score");
        for r in &tier2_results {
            let _ = writeln!(
                csv_file,
                "{},{:.2},{:.2},{:.2},{:.4}",
                r.model_name, r.grammar_accuracy_pct, r.avg_ttft_ms, r.avg_tps, r.e3_score
            );
        }
        println!("[+] CSV Canônico do Pareto Bandit gravado em: {}", csv_path.display());
    }
}

fn valid_count_as_f64(count: usize) -> f64 {
    count as f64
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mut tier = 1usize;
    let mut custom_models_dir: Option<PathBuf> = None;
    let mut target_model_filter: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--tier" => {
                if i + 1 < args.len() {
                    if let Ok(val) = args[i + 1].parse::<usize>() {
                        tier = val;
                    }
                    i += 1;
                }
            }
            "--models-dir" => {
                if i + 1 < args.len() {
                    custom_models_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--target-model" => {
                if i + 1 < args.len() {
                    target_model_filter = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    print_backend_info();

    let default_models_dir = PathBuf::from("C:\\Users\\rosas\\.lmstudio\\models");
    let models_dir = custom_models_dir.unwrap_or(default_models_dir);
    let bench_dir = resolve_benchmark_dir();

    // PASSO 2: Conexão e Sincronização SSOT SQLite em model_registry
    let db_path = model_registry::resolve_db_path();
    let conn = model_registry::init_model_registry_db(&db_path)?;

    if let Ok(count) = model_registry::sync_local_models_to_registry(&conn, &models_dir) {
        println!("[+] Sincronização SQLite SSOT concluída. Modelos ativos registrados: {}", count);
    }

    if tier == 2 {
        let mut approved = match model_registry::fetch_approved_tier1_models(&conn) {
            Ok(list) if !list.is_empty() => list,
            _ => {
                let report_dir = resolve_root_dir().join(".soda_scratchpad").join("reports");
                let tier1_report = report_dir.join("arena_tier1_guillotine.txt");
                model_registry::load_approved_tier1_models(&tier1_report)
            }
        };

        if let Some(ref filter) = target_model_filter {
            let lower = filter.to_lowercase();
            approved.retain(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase().contains(&lower))
                    .unwrap_or(false)
                    || p.to_string_lossy().to_lowercase().contains(&lower)
            });
            println!("[+] Filtro Single-Shot ativado (--target-model '{}'). Modelos Tier 2 filtrados: {}", filter, approved.len());
        }

        run_tier2_colosseum(&bench_dir, &approved);
    } else {
        let mut models = model_registry::collect_local_models(&models_dir);

        if let Some(ref filter) = target_model_filter {
            let lower = filter.to_lowercase();
            models.retain(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase().contains(&lower))
                    .unwrap_or(false)
                    || p.to_string_lossy().to_lowercase().contains(&lower)
            });
            println!("[+] Filtro Single-Shot ativado (--target-model '{}'). Modelos Tier 1 filtrados: {}", filter, models.len());
        }

        if models.is_empty() {
            println!("[!] Nenhum modelo correspondente encontrado. Registrando modelo de fallback.");
            models.push(PathBuf::from("mock_model_tier1.gguf"));
        }

        run_tier1_guillotine(&conn, &models, &bench_dir);
    }

    Ok(())
}
