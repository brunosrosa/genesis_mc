// ab_test_silicon_shock.rs — Silicon Shock Laboratory (A/B Route Comparison)
// Route A: FastCloudCascade (OpenRouter FAST_KEY) → `_ab_test_cloud_essence.md`
// Route B: LocalDistiller + LmStudio → `_ab_test_local_essence.md`
//
// Pré-requisitos:
//   - LM Studio rodando em localhost:1234 com Qwen 3.5 4B (ou similar)
//   - .env com OPENROUTER_API_FAST_KEY (para Route A)
//
// Executar: cargo run --example ab_test_silicon_shock

use genesis_mc_lib::finops::phase1_5::cloud_cascade::CascadeError;
use genesis_mc_lib::finops::phase1_5::local_distiller::{DistillationError, InferenceEngine, LocalDistiller};
use reqwest::Client as HttpClient;
use reqwest::blocking::Client as BlockingClient;
use serde::{Deserialize, Serialize};
use std::time::Instant;

const LM_STUDIO_URL: &str = "http://127.0.0.1:1234/api/v1/chat";
const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DISTILL_SYSTEM_PROMPT: &str =
    "Resuma os fatos crus, extraia a alma matemática, \
    não emita opiniões, limite-se a ~3000 tokens.";

struct LmStudioEngine {
    client: BlockingClient,
    model_name: String,
}

impl LmStudioEngine {
    fn new(model_name: &str) -> Self {
        Self {
            client: BlockingClient::new(),
            model_name: model_name.to_string(),
        }
    }
}

impl Default for LmStudioEngine {
    fn default() -> Self {
        Self::new("qwen3.5-4b")
    }
}

impl InferenceEngine for LmStudioEngine {
    fn infer(&self, prompt: &str, _max_tokens: usize) -> Result<String, DistillationError> {
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            input: String,
        }

        #[derive(Deserialize)]
        struct LmStudioResponse {
            output: Vec<OutputItem>,
        }

        #[derive(Deserialize)]
        struct OutputItem {
            #[serde(rename = "type")]
            item_type: String,
            content: String,
        }

        let request_body = ChatRequest {
            model: self.model_name.clone(),
            input: prompt.to_string(),
        };

        let response = self
            .client
            .post(LM_STUDIO_URL)
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(600))
            .send()
            .map_err(|e| DistillationError::InferenceError(format!("LM Studio request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(DistillationError::InferenceError(format!(
                "LM Studio HTTP {status}: {body}"
            )));
        }

        let parsed: LmStudioResponse = response
            .json()
            .map_err(|e| DistillationError::InferenceError(format!("Failed to parse LM Studio response: {e}")))?;

        parsed
            .output
            .iter()
            .find(|item| item.item_type == "message")
            .map(|item| item.content.clone())
            .ok_or_else(|| DistillationError::InferenceError("No message in LM Studio response".to_string()))
    }

    fn is_loaded(&self) -> bool {
        true
    }

    fn clear_cache(&mut self) {}
}

struct FastCloudCascade {
    client: HttpClient,
    api_key: String,
    model: String,
}

impl FastCloudCascade {
    fn new() -> Result<Self, CascadeError> {
        let api_key = std::env::var("OPENROUTER_API_FAST_KEY")
            .map_err(|_| CascadeError::NetworkError("OPENROUTER_API_FAST_KEY not set".to_string()))?;
        Ok(Self {
            client: HttpClient::new(),
            api_key,
            model: "deepseek/deepseek-chat".to_string(),
        })
    }

    async fn cascade_distill(&self, payload: &str, system_prompt: &str) -> Result<String, CascadeError> {
        #[derive(Serialize)]
        struct RequestBody<'a> {
            model: &'a str,
            messages: Vec<Message<'a>>,
            max_tokens: usize,
        }

        #[derive(Serialize)]
        struct Message<'a> {
            role: &'a str,
            content: &'a str,
        }

        let body = RequestBody {
            model: &self.model,
            messages: vec![
                Message { role: "system", content: system_prompt },
                Message { role: "user", content: payload },
            ],
            max_tokens: 3000,
        };

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://genesis_mc.local")
            .header("X-Title", "SiliconShockLab")
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| CascadeError::NetworkError(format!("Request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(CascadeError::PaidFallbackFailed {
                status: status.as_u16(),
                message: body_text,
            });
        }

        #[derive(Deserialize)]
        struct OpenRouterResponse {
            choices: Vec<Choice>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: ResponseMessage,
        }

        #[derive(Deserialize)]
        struct ResponseMessage {
            content: String,
        }

        let parsed: OpenRouterResponse = response
            .json()
            .await
            .map_err(|e| CascadeError::NetworkError(format!("JSON parse error: {e}")))?;

        parsed
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| CascadeError::InvalidInput)
    }
}

fn generate_dense_payload() -> String {
    let mut lines = Vec::with_capacity(600);

    lines.push(r#"# Silicon Shock Validation Report — 25k Token Dense Payload"#.to_string());
    lines.push(String::new());

    for i in 0..50 {
        lines.push(format!("## Finding Block {i:03}"));
        lines.push(format!("  severity: HIGH | tool: semgrep | id: SEC-{i:04}-RUST-001"));
        lines.push(format!(
            "  file: src/services/user_service_{:02}.rs | line: {} | col: 23",
            i % 20,
            (i * 17 + 100)
        ));
        lines.push("  pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)".to_string());
        lines.push(format!(
            "  message: In branch at line {}, `ctx` is matched against `Some` and `None` \
            without subsequent null-check on `x` within the same arm. This may cause \
            a null pointer dereference if `x` is accessed before the match completes.",
            (i * 17 + 100)
        ));
        lines.push(format!(
            "  confidence: {}% | CWE-476 | OWASP-A1 | EPSS: {:.4}",
            70 + (i % 30),
            0.1 + (i as f64) * 0.007
        ));
        lines.push(String::new());

        lines.push(format!(
            "  code_snippet: | let user = ctx.users_{:02}.get(id)?; let profile = user.profile.as_ref()?;",
            i % 20
        ));
        lines.push(String::new());

        lines.push("  remediation:".to_string());
        lines.push("    - Implement comprehensive null checks before accessing optional fields".to_string());
        lines.push("    - Use if-let chains for cleaner Optional handling".to_string());
        lines.push("    - Add integration tests covering None paths".to_string());
        lines.push(String::new());

        lines.push("  参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2".to_string());
        lines.push(String::new());
    }

    for i in 0..30 {
        lines.push(format!(
            "## Lint Info {i:03}: cargo_clippy | src/harvester/mod.rs @ {}:{}",
            50 + (i * 7),
            10 + (i % 40)
        ));
        lines.push(
            "  warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.".to_string(),
        );
        lines.push(
            "  suggestion: Use `.unwrap_or_else(|| ...)`, `.expect(\"context\")` with message, \
            or restructure with `if let Some(x) = ...`."
                .to_string(),
        );
        lines.push(String::new());
    }

    for i in 0..20 {
        lines.push(format!(
            "## Dependency Advisory {i:03}: GHSA-{:04}-{:04}-{:04} | severity: CRITICAL",
            1000 + i,
            2000 + (i * 3) % 9000,
            3000 + (i * 7) % 9000
        ));
        lines.push(format!(
            "  package: serde@1.{}:{}.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52",
            i % 9,
            (i * 13) % 10
        ));
        lines.push(format!(
            "  introduced via: package_a -> transitive_dep_{:02} -> serde",
            i % 15
        ));
        lines.push(format!("  fixed_version: 1.0.52 | EPSS: {:.4}", 0.3 + (i as f64) * 0.02));
        lines.push(String::new());
    }

    lines.join("\n")
}

fn save_essence(path: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, content)?;
    Ok(())
}

async fn run_route_a(payload: &str) -> Result<String, CascadeError> {
    let cascade = FastCloudCascade::new()?;
    cascade.cascade_distill(payload, DISTILL_SYSTEM_PROMPT).await
}

async fn run_route_b(payload: &str) -> Result<String, DistillationError> {
    let payload = payload.to_string();
    let prompt = DISTILL_SYSTEM_PROMPT.to_string();
    tokio::task::spawn_blocking(move || {
        let distiller: LocalDistiller<LmStudioEngine> = LocalDistiller::new("")
            .map_err(|e| DistillationError::InferenceError(e.to_string()))?;
        distiller.distill(&payload, &prompt)
    })
    .await
    .map_err(|e| DistillationError::InferenceError(format!("Task join error: {e}")))?
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    println!("=== Silicon Shock Laboratory — Phase 1.5 A/B Route Validation ===");
    println!();

    let payload = generate_dense_payload();
    let token_estimate = payload.split_whitespace().count() * 2;
    println!("[PAYLOAD]  ~{} tokens gerados in-memory", token_estimate);
    println!();

    println!("[ROUTE B]  LocalDistiller + LmStudio (Qwen 3.5 4B @ localhost:1234)");
    let t0 = Instant::now();
    match run_route_b(&payload).await {
        Ok(essence) => {
            let elapsed = t0.elapsed();
            println!("[ROUTE B]  OK — {}ms — LmStudio respondeu", elapsed.as_millis());
            if let Err(e) = save_essence("_ab_test_local_essence.md", &essence) {
                eprintln!("[ROUTE B]  ERRO ao salvar essência: {e}");
            } else {
                println!("[ROUTE B]  Salvo → _ab_test_local_essence.md ({} chars)", essence.len());
            }
        }
        Err(e) => {
            let elapsed = t0.elapsed();
            eprintln!(
                "[ROUTE B]  FALHA {}ms — LmStudio não disponível ou erro: {e}",
                elapsed.as_millis()
            );
            eprintln!("[ROUTE B]  Verifique se LM Studio está rodando em {LM_STUDIO_URL}",);
        }
    }
    println!();

    println!("[ROUTE A]  FastCloudCascade (OpenRouter — deepseek/deepseek-chat)");
    let t0 = Instant::now();
    match run_route_a(&payload).await {
        Ok(essence) => {
            let elapsed = t0.elapsed();
            println!("[ROUTE A]  OK — {}ms — OpenRouter respondeu", elapsed.as_millis());
            if let Err(e) = save_essence("_ab_test_cloud_essence.md", &essence) {
                eprintln!("[ROUTE A]  ERRO ao salvar essência: {e}");
            } else {
                println!("[ROUTE A]  Salvo → _ab_test_cloud_essence.md ({} chars)", essence.len());
            }
        }
        Err(CascadeError::NetworkError(msg)) if msg.contains("OPENROUTER_API_FAST_KEY") => {
            eprintln!("[ROUTE A]  PULADO — OPENROUTER_API_FAST_KEY não definida no .env");
        }
        Err(e) => {
            let elapsed = t0.elapsed();
            eprintln!("[ROUTE A]  FALHA {}ms — {e}", elapsed.as_millis());
        }
    }
    println!();
    println!("=== Silicon Shock Laboratory — Concluído ===");
}
