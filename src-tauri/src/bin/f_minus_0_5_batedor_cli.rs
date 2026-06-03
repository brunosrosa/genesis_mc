use std::collections::{BTreeSet, HashMap};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};

const MASTER_SOLUTIONS_SHEET: &str = "MASTER_SOLUTIONS";
const STATUS_GATILHO: &str = "INICIAR_TRIAGEM";
const STATUS_CONCLUIDO: &str = "TRIAGEM_CONCLUIDA";
const FASE_OK: &str = "FASE_-0.5_BATEDOR_OK";

const README_CHAR_LIMIT: usize = 3_000;
const MAX_UPDATES_PER_BATCH: usize = 50;
const MAX_RESUMO_CHARS: usize = 800;
const MAX_DEDUP_LINKS_IN_RESUMO: usize = 12;

const ALLOWED_CATEGORIA_ARQUITETURAL: [&str; 47] = [
        "AI_Research - Foundation_Model",
        "CanvasUI - Core_Pattern",
        "CanvasUI - Domain_App",
        "CanvasUI - Ops_Dashboard",
        "CanvasUI - Terminal_Workspace",
        "Comms_Social - Platform_Client",
        "Domain_App - Self_Hosted",
        "Infraestrutura_Core - Concurrency_OS",
        "Infraestrutura_Core - Data_Pipeline",
        "Infraestrutura_Core - Data_Serialization",
        "Infraestrutura_Core - Hardware_Ops",
        "Knowledge_Extraction - Doc_Parsing",
        "Knowledge_Extraction - Generic",
        "Knowledge_Extraction - Multimedia_Parsing",
        "Knowledge_Extraction - Semantic_Mining",
        "Knowledge_Extraction - Web_Scraping",
        "Memoria_RAG - Graph_Store",
        "Memoria_RAG - Relational_Episodic",
        "Memoria_RAG - Vector_Store",
        "Model_Serving - Edge_Deployment",
        "Model_Serving - Inference_Engine",
        "Model_Serving - Resource_Scheduler",
        "Model_Serving - Training_FineTuning",
        "Orquestracao_Agentes - Dev_Framework",
        "Orquestracao_Agentes - OS_Runtime",
        "Orquestracao_Agentes - Simulation_Environment",
        "Orquestracao_Agentes - Skill_Library",
        "Orquestracao_Agentes - Specialized_Worker",
        "Orquestracao_Agentes - Workflow_DAG",
        "Roteamento_FinOps - API_Gateway",
        "Roteamento_FinOps - Cost_Analytics",
        "Roteamento_FinOps - Network_Tunnel",
        "Roteamento_FinOps - Prompt_Caching",
        "Seguranca_Sandbox - Auth_Crypto",
        "Seguranca_Sandbox - MicroVM_Container",
        "Seguranca_Sandbox - Privacy_Governance",
        "Seguranca_Sandbox - Runtime_Isolation",
        "Tooling_Dev - CLI_Utilities",
        "Tooling_Dev - Knowledge_Curation",
        "Tooling_Dev - MCP_Bridging",
        "Tooling_Dev - Observability_Eval",
        "Tooling_Dev - Prompt_Knowledge",
        "UILibrary - Animation_Graphics",
        "UILibrary - Component_System",
        "UILibrary - Generative_UI",
        "UILibrary - Terminal_TUI",
        "Outros - Uncategorized",
];

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn col_idx_to_a1(col_idx0: usize) -> String {
    let mut n = col_idx0 + 1;
    let mut out = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        out.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    out
}

fn normalize_header_cell(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    let mut out = String::with_capacity(lowered.len());
    for ch in lowered.chars() {
        let mapped = match ch {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            ' ' | '-' => '_',
            _ => ch,
        };
        out.push(mapped);
    }
    out
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    input.chars().take(max_chars).collect()
}

fn sanitize_env_scalar(raw: &str) -> String {
    let mut v = raw.trim().trim_matches('"').trim_matches('\'').trim().to_string();
    if let Some(hash) = v.find('#') {
        v.truncate(hash);
        v = v.trim().to_string();
    }
    if let Some(first) = v.split_whitespace().next() {
        first.trim().to_string()
    } else {
        String::new()
    }
}

fn normalize_batedor_model(raw: &str) -> String {
    let mut m = sanitize_env_scalar(raw);
    if m.eq_ignore_ascii_case("deepseek/deepseek-v4") {
        m = "deepseek/deepseek-v4-flash".to_string();
    }
    m
}

fn try_extract_owner_repo_from_repo_url(repo_url: &str) -> Option<(String, String)> {
    let url = url::Url::parse(repo_url).ok()?;
    if !url.host_str()?.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let mut parts = url.path().trim_matches('/').split('/');
    let owner = parts.next()?.trim().to_string();
    let repo = parts.next()?.trim().to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
type SheetsUpdateFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

trait SheetsClient: Send + Sync {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a>;

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: HashMap<String, Vec<Vec<String>>>,
    ) -> SheetsUpdateFuture<'a>;
}

struct SheetsMcpClient;

impl SheetsMcpClient {
    async fn poll_for_jsonrpc_response_from_reader<R>(
        reader: R,
        timeout: Duration,
    ) -> Result<Value, String>
    where
        R: tokio::io::AsyncBufRead + Unpin,
    {
        use tokio::io::AsyncBufReadExt;

        let started = Instant::now();
        let mut lines = reader.lines();
        loop {
            if started.elapsed() > timeout {
                return Err(format!(
                    "Timeout: O servidor MCP (Sheets) não emitiu o payload após {} segundos",
                    timeout.as_secs()
                ));
            }

            match tokio::time::timeout(Duration::from_millis(200), lines.next_line()).await {
                Ok(Ok(Some(line))) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&line) {
                        if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                            return Ok(value);
                        }
                    }
                }
                Ok(Ok(None)) => tokio::time::sleep(Duration::from_millis(200)).await,
                Ok(Err(e)) => return Err(format!("Falha ao ler stdout do MCP: {e}")),
                Err(_) => {}
            }
        }
    }

    async fn call_mcp(tool_name: &str, arguments: Value) -> Result<Value, String> {
        use std::process::Stdio;
        use tokio::io::{AsyncWriteExt, BufReader};
        use tokio::process::Command;

        let creds = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| "Missing GOOGLE_APPLICATION_CREDENTIALS".to_string())?;

        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "f-minus-0-5-batedor", "version": "1.0.0" }
            }
        });
        let initialized_notif = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let mcp_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": tool_name, "arguments": arguments }
        });

        let mut child = Command::new("mcp-google-sheets")
            .env("GOOGLE_APPLICATION_CREDENTIALS", creds)
            .env("UV_NO_PROGRESS", "1")
            .env("UV_QUIET", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Falha ao spawnar mcp-google-sheets: {e}"))?;

        let mut stdin = child.stdin.take().ok_or_else(|| "stdin indisponível".to_string())?;
        stdin
            .write_all(format!("{}\n", init_req).as_bytes())
            .await
            .map_err(|e| format!("Falha ao escrever init_req: {e}"))?;
        stdin
            .write_all(format!("{}\n", initialized_notif).as_bytes())
            .await
            .map_err(|e| format!("Falha ao escrever initialized: {e}"))?;
        stdin
            .write_all(format!("{}\n", mcp_request).as_bytes())
            .await
            .map_err(|e| format!("Falha ao escrever tools/call: {e}"))?;
        drop(stdin);

        let stdout = child.stdout.take().ok_or_else(|| "stdout indisponível".to_string())?;
        let msg = Self::poll_for_jsonrpc_response_from_reader(BufReader::new(stdout), Duration::from_secs(20)).await?;
        let _ = child.kill().await;
        let _ = child.wait().await;

        if msg.get("error").is_some() {
            return Err(format!("MCP retornou erro: {msg}"));
        }
        let result = msg
            .get("result")
            .cloned()
            .ok_or_else(|| "Resposta MCP inválida (sem campo result)".to_string())?;
        Ok(Self::normalize_mcp_tool_result(result))
    }

    fn normalize_mcp_tool_result(result: Value) -> Value {
        let content = match result.get("content").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return result,
        };

        for item in content {
            if let Some(json_val) = item.get("json") {
                return json_val.clone();
            }
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                    return parsed;
                }
            }
        }
        result
    }

    fn extract_values_2d(json: &Value) -> Vec<Vec<String>> {
        if let Some(values) = json.get("values").and_then(|v| v.as_array()) {
            return values
                .iter()
                .map(|row| {
                    row.as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|cell| cell.as_str().unwrap_or("").to_string())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
        }
        if let Some(vrs) = json.get("valueRanges").and_then(|v| v.as_array()) {
            if let Some(first) = vrs.first() {
                if let Some(values) = first.get("values").and_then(|v| v.as_array()) {
                    return values
                        .iter()
                        .map(|row| {
                            row.as_array()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|cell| cell.as_str().unwrap_or("").to_string())
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();
                }
            }
        }
        vec![]
    }
}

impl SheetsClient for SheetsMcpClient {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a> {
        Box::pin(async move {
            let out = SheetsMcpClient::call_mcp(
                "get_sheet_data",
                json!({
                    "spreadsheet_id": spreadsheet_id,
                    "sheet": sheet,
                    "range": range,
                    "include_grid_data": false
                }),
            )
            .await?;
            Ok(SheetsMcpClient::extract_values_2d(&out))
        })
    }

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: HashMap<String, Vec<Vec<String>>>,
    ) -> SheetsUpdateFuture<'a> {
        Box::pin(async move {
            let mut payload_ranges = serde_json::Map::new();
            for (range, values) in ranges {
                payload_ranges.insert(range, json!(values));
            }
            let _ = SheetsMcpClient::call_mcp(
                "batch_update_cells",
                json!({
                    "spreadsheet_id": spreadsheet_id,
                    "sheet": sheet,
                    "ranges": Value::Object(payload_ranges)
                }),
            )
            .await?;
            Ok(())
        })
    }
}

#[derive(Debug, Clone)]
struct Columns {
    status_atualizacao_idx: usize,
    status_fase_idx: usize,
    repo_url_idx: usize,
    proposta_original_resumo_idx: usize,
    categoria_arquitetural_idx: usize,
}

fn resolve_columns(header_row: &[String]) -> Result<Columns, String> {
    let mut status_atualizacao_idx = None;
    let mut status_fase_idx = None;
    let mut repo_url_idx = None;
    let mut proposta_original_resumo_idx = None;
    let mut categoria_arquitetural_idx = None;

    let normalized = header_row
        .iter()
        .map(|raw| normalize_header_cell(raw))
        .collect::<Vec<_>>();

    for (idx, h) in normalized.iter().enumerate() {
        match h.as_str() {
            "status_atualizacao" => status_atualizacao_idx = Some(idx),
            "status_fase" => status_fase_idx = Some(idx),
            "repo_url" => repo_url_idx = Some(idx),
            "proposta_original_resumo" => proposta_original_resumo_idx = Some(idx),
            "categoria_arquitetural" => categoria_arquitetural_idx = Some(idx),
            _ => {}
        }
    }

    let normalized_join = normalized.join(", ");
    Ok(Columns {
        status_atualizacao_idx: status_atualizacao_idx
            .ok_or_else(|| format!("Cabeçalho não contém 'status_atualizacao'. headers_normalizados=[{normalized_join}]"))?,
        status_fase_idx: status_fase_idx.ok_or_else(|| format!("Cabeçalho não contém 'status_fase'. headers_normalizados=[{normalized_join}]"))?,
        repo_url_idx: repo_url_idx.ok_or_else(|| format!("Cabeçalho não contém 'repo_url'. headers_normalizados=[{normalized_join}]"))?,
        proposta_original_resumo_idx: proposta_original_resumo_idx
            .ok_or_else(|| format!("Cabeçalho não contém 'proposta_original_resumo'. headers_normalizados=[{normalized_join}]"))?,
        categoria_arquitetural_idx: categoria_arquitetural_idx
            .ok_or_else(|| format!("Cabeçalho não contém 'categoria_arquitetural'. headers_normalizados=[{normalized_join}]"))?,
    })
}

fn parse_cli_args() -> bool {
    let mut args = std::env::args();
    args.next();
    let mut dry_run = false;
    for arg in args {
        if arg == "--dry-run" {
            dry_run = true;
        }
    }
    dry_run
}

#[derive(Debug, Clone)]
struct PendingRow {
    row_number_1based: u32,
    repo_url: String,
}

#[derive(Debug, Clone, Copy)]
struct GatilhoScan {
    pending: usize,
    skipped_non_trigger: usize,
    skipped_missing_repo_url: usize,
}

fn find_gatilho_rows(values: &[Vec<String>], cols: &Columns) -> (Vec<PendingRow>, GatilhoScan) {
    let mut out = Vec::new();
    let mut skipped_non_trigger = 0usize;
    let mut skipped_missing_repo_url = 0usize;
    for (idx, row) in values.iter().enumerate() {
        let status_atualizacao = row
            .get(cols.status_atualizacao_idx)
            .map(|s| s.trim())
            .unwrap_or("");
        if !status_atualizacao.eq_ignore_ascii_case(STATUS_GATILHO) {
            skipped_non_trigger += 1;
            continue;
        }
        let repo_url = row.get(cols.repo_url_idx).map(|s| s.trim()).unwrap_or("");
        if repo_url.is_empty() {
            skipped_missing_repo_url += 1;
            continue;
        }
        out.push(PendingRow {
            row_number_1based: (idx as u32) + 2,
            repo_url: repo_url.to_string(),
        });
    }
    let pending = out.len();
    (
        out,
        GatilhoScan {
            pending,
            skipped_non_trigger,
            skipped_missing_repo_url,
        },
    )
}

#[derive(Debug, Clone, Deserialize)]
struct BatedorOut {
    proposta_original_resumo: String,
    categoria_arquitetural: String,
}

fn response_format_for_batedor() -> Value {
    fn strict_object(properties: serde_json::Map<String, Value>, required: Vec<&'static str>) -> Value {
        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": required,
            "additionalProperties": false
        })
    }

    let mut props = serde_json::Map::new();
    props.insert(
        "proposta_original_resumo".to_string(),
        json!({ "type": "string", "minLength": 10, "maxLength": MAX_RESUMO_CHARS }),
    );
    props.insert(
        "categoria_arquitetural".to_string(),
        json!({
            "type": "string",
            "enum": ALLOWED_CATEGORIA_ARQUITETURAL.iter().copied().collect::<Vec<&str>>()
        }),
    );

    let schema = strict_object(props, vec!["proposta_original_resumo", "categoria_arquitetural"]);
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "soda_batedor_triage_v1",
            "strict": true,
            "schema": schema
        }
    })
}

fn openrouter_body_for_batedor(model: &str, prompt: &str) -> Value {
    json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "Responda SOMENTE com JSON válido (sem markdown, sem texto extra)."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.0,
        "max_tokens": 4000,
        "response_format": response_format_for_batedor()
    })
}

struct OpenRouterClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenRouterClient {
    fn new() -> Result<Self, String> {
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());

        let api_key = [
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
            "OPENROUTER_API_KEY",
        ]
        .into_iter()
        .find_map(|k| std::env::var(k).ok().map(|v| v.trim().trim_matches('"').to_string()))
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "Missing OPENROUTER_API_FAST_KEY/OPENROUTER_API_FREE_KEY/OPENROUTER_API_KEY".to_string())?;

        let mut model = std::env::var("OPENROUTER_BATEDOR_MODEL")
            .ok()
            .map(|v| normalize_batedor_model(&v))
            .filter(|v| !v.is_empty())
            .or_else(|| {
                std::env::var("OPENROUTER_DEFAULT_MODEL")
                    .ok()
                    .map(|v| normalize_batedor_model(&v))
                    .filter(|v| !v.is_empty())
            })
            .unwrap_or_else(|| "deepseek/deepseek-v4-flash".to_string());
        if is_expensive_model(&model) {
            model = "deepseek/deepseek-v4-flash".to_string();
        }

        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            model,
        })
    }

    async fn triage(&self, prompt: &str) -> Result<BatedorOut, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = openrouter_body_for_batedor(&self.model, prompt);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(Duration::from_secs(35))
            .send()
            .await
            .map_err(|e| format!("Falha HTTP OpenRouter: {e}"))?;

        let status = response.status();
        let json = response
            .json::<Value>()
            .await
            .map_err(|e| format!("Falha ao parsear JSON OpenRouter: {e}"))?;
        if !status.is_success() {
            return Err(format!("OpenRouter HTTP {}: {}", status.as_u16(), json));
        }
        let content = extract_openrouter_content(&json)
            .ok_or_else(|| format!("OpenRouter: resposta vazia/inesperada: {json}"))?;
        let parsed: BatedorOut = serde_json::from_str(&content)
            .map_err(|e| format!("OpenRouter: JSON inválido para Batedor: {e}. content={content}"))?;
        validate_batedor_out(&parsed)?;
        Ok(parsed)
    }

    async fn dedup_links_default_model(&self, repo_urls: &[String]) -> Result<Vec<String>, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let model = std::env::var("OPENROUTER_DEFAULT_MODEL")
            .ok()
            .map(|v| normalize_batedor_model(&v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| self.model.clone());
        let prompt = build_link_dedup_prompt(repo_urls);
        let body = openrouter_body_for_link_dedup(&model, &prompt);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(Duration::from_secs(35))
            .send()
            .await
            .map_err(|e| format!("Falha HTTP OpenRouter (dedup): {e}"))?;

        let status = response.status();
        let json = response
            .json::<Value>()
            .await
            .map_err(|e| format!("Falha ao parsear JSON OpenRouter (dedup): {e}"))?;
        if !status.is_success() {
            return Err(format!("OpenRouter HTTP {} (dedup): {}", status.as_u16(), json));
        }
        let content = extract_openrouter_content(&json)
            .ok_or_else(|| format!("OpenRouter: resposta vazia/inesperada (dedup): {json}"))?;
        let parsed: LinkDedupOut = serde_json::from_str(&content)
            .map_err(|e| format!("OpenRouter: JSON inválido para dedup: {e}. content={content}"))?;
        let mut out = Vec::new();
        for item in parsed.repos {
            let trimmed = item.trim();
            if trimmed.is_empty() || !trimmed.contains('/') {
                continue;
            }
            out.push(trimmed.to_string());
            if out.len() >= 80 {
                break;
            }
        }
        Ok(out)
    }
}

fn is_expensive_model(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    m.contains("opus")
        || m.contains("claude")
        || m.contains("gpt-5")
        || m.contains("gpt-4")
        || m.contains("o1")
        || m.contains("o3")
        || m.contains("deepseek-v4-pro")
}

fn extract_openrouter_content(json: &Value) -> Option<String> {
    let content = json
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))?;

    match content {
        Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        Value::Array(parts) => {
            let mut out = String::new();
            for part in parts {
                if let Some(t) = part
                    .as_str()
                    .or_else(|| part.get("text").and_then(|v| v.as_str()))
                    .or_else(|| part.get("content").and_then(|v| v.as_str()))
                {
                    let t = t.trim();
                    if t.is_empty() {
                        continue;
                    }
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(t);
                }
            }
            Some(out).filter(|s| !s.trim().is_empty())
        }
        _ => None,
    }
}

fn validate_batedor_out(out: &BatedorOut) -> Result<(), String> {
    let resumo = out.proposta_original_resumo.trim();
    if resumo.is_empty() {
        return Err("proposta_original_resumo vazio".to_string());
    }
    if resumo.chars().count() > MAX_RESUMO_CHARS {
        return Err(format!(
            "proposta_original_resumo excede limite de {} chars",
            MAX_RESUMO_CHARS
        ));
    }
    let cat = out.categoria_arquitetural.trim();
    if cat.is_empty() {
        return Err("categoria_arquitetural inválida (vazia)".to_string());
    }
    if !ALLOWED_CATEGORIA_ARQUITETURAL.iter().any(|v| v == &cat) {
        return Err("categoria_arquitetural inválida (fora do ENUM)".to_string());
    }
    Ok(())
}

async fn fetch_readme_truncated(repo_url: &str) -> Result<String, String> {
    let (owner, repo) = try_extract_owner_repo_from_repo_url(repo_url)
        .ok_or_else(|| "repo_url não é GitHub (esperado https://github.com/<owner>/<repo>)".to_string())?;

    let api_base = std::env::var("SODA_GITHUB_API_BASE_URL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://api.github.com".to_string());
    let url = format!("{}/repos/{}/{}/readme", api_base.trim_end_matches('/'), owner, repo);

    let client = reqwest::Client::new();
    let mut req = client
        .get(&url)
        .header("User-Agent", "soda-batedor")
        .header("Accept", "application/vnd.github.raw")
        .timeout(Duration::from_secs(25));
    if let Ok(token) = std::env::var("GITHUB_PAT") {
        let token = token.trim().trim_matches('"').to_string();
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("GitHub README HTTP falhou: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub README HTTP {} (repo={owner}/{repo})", resp.status().as_u16()));
    }
    let text = resp.text().await.map_err(|e| format!("GitHub README body falhou: {e}"))?;
    Ok(truncate_chars(&text, README_CHAR_LIMIT))
}

fn build_prompt(readme_trunc: &str) -> String {
    let allowed = ALLOWED_CATEGORIA_ARQUITETURAL
        .iter()
        .copied()
        .collect::<Vec<&str>>()
        .join(", ");
    let mut out = String::new();
    out.push_str("Tarefa: Resuma o repositório em 1 frase técnica, neutra e desidratada e classifique-o.\n");
    out.push_str("Responda SOMENTE com JSON válido, seguindo o schema fornecido.\n");
    out.push_str("Regras:\n");
    out.push_str("- proposta_original_resumo: 1 frase técnica, neutra, desidratada (até 800 chars).\n");
    out.push_str("- categoria_arquitetural: escolha EXATA dentre: ");
    out.push_str(&allowed);
    out.push_str(".\n");
    out.push_str("\n\nREADME (primeiros 3000 chars):\n");
    out.push_str(readme_trunc);
    out
}

fn extract_urls_from_text(text: &str, max_urls: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    let bytes = text.as_bytes();
    while idx < bytes.len() && out.len() < max_urls {
        let rest = &text[idx..];
        let Some(rel_pos) = rest.find("http") else {
            break;
        };
        idx = idx.saturating_add(rel_pos);
        let candidate = &text[idx..];
        let end = candidate
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '"' | '\'' | '<' | '>'))
            .unwrap_or(candidate.len());
        let mut url = candidate[..end].trim().trim_end_matches(['.', ',', ';', ':']).to_string();
        if url.starts_with("http://") || url.starts_with("https://") {
            if url.len() > 2048 {
                url.truncate(2048);
            }
            out.push(url);
        }
        idx = idx.saturating_add(end.max(1));
    }
    out
}

fn extract_github_repo_ids(urls: &[String], max_repos: usize) -> Vec<String> {
    let mut out = BTreeSet::<String>::new();
    for url in urls {
        if out.len() >= max_repos {
            break;
        }
        let lower = url.to_ascii_lowercase();
        let marker = "github.com/";
        let Some(pos) = lower.find(marker) else {
            continue;
        };
        let mut rest = url[(pos + marker.len())..].to_string();
        if let Some(hash) = rest.find('#') {
            rest.truncate(hash);
        }
        if let Some(q) = rest.find('?') {
            rest.truncate(q);
        }
        rest = rest.trim_end_matches('/').trim_end_matches(".git").to_string();
        let mut parts = rest.split('/').map(|p| p.trim()).filter(|p| !p.is_empty());
        let Some(owner) = parts.next() else { continue };
        let Some(repo) = parts.next() else { continue };
        if owner.eq_ignore_ascii_case("topics")
            || owner.eq_ignore_ascii_case("search")
            || owner.eq_ignore_ascii_case("orgs")
            || owner.eq_ignore_ascii_case("users")
        {
            continue;
        }
        out.insert(format!("{owner}/{repo}"));
    }
    out.into_iter().take(max_repos).collect()
}

fn looks_like_content_repo(readme_trunc: &str, github_repo_links: usize) -> bool {
    let s = readme_trunc.to_ascii_lowercase();
    if s.contains("awesome") {
        return true;
    }
    if github_repo_links >= 25 {
        return true;
    }
    s.contains("curated list") || s.contains("resources") || s.contains("collection of")
}

fn response_format_for_link_dedup() -> Value {
    let schema = json!({
        "type": "object",
        "properties": {
            "repos": {
                "type": "array",
                "items": { "type": "string", "minLength": 3, "maxLength": 200 },
                "minItems": 0,
                "maxItems": 120
            }
        },
        "required": ["repos"],
        "additionalProperties": false
    });
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "soda_link_dedup_v1",
            "strict": true,
            "schema": schema
        }
    })
}

fn openrouter_body_for_link_dedup(model: &str, prompt: &str) -> Value {
    json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "Responda SOMENTE com JSON válido (sem markdown, sem texto extra)."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.0,
        "max_tokens": 2000,
        "response_format": response_format_for_link_dedup()
    })
}

#[derive(Debug, Deserialize)]
struct LinkDedupOut {
    repos: Vec<String>,
}

fn build_link_dedup_prompt(repo_urls: &[String]) -> String {
    let mut out = String::new();
    out.push_str("Tarefa: deduplicar e normalizar uma lista de repositórios GitHub.\n");
    out.push_str("Entrada: lista de strings (owner/repo).\n");
    out.push_str("Saída: JSON com campo repos: array deduplicado, ordenado por relevância.\n");
    out.push_str("Regras:\n");
    out.push_str("- Remova duplicatas e variações (maiúsculas/minúsculas).\n");
    out.push_str("- Mantenha apenas entradas no formato owner/repo.\n");
    out.push_str("- Priorize projetos centrais (não forks óbvios) quando houver redundância.\n");
    out.push_str("- Limite a 80 itens.\n\n");
    out.push_str("Lista:\n");
    for item in repo_urls.iter().take(400) {
        out.push_str("- ");
        out.push_str(item);
        out.push('\n');
    }
    out
}

fn build_success_ranges(cols: &Columns, row_number_1based: u32, out: &BatedorOut) -> HashMap<String, Vec<Vec<String>>> {
    let status_col = col_idx_to_a1(cols.status_atualizacao_idx);
    let fase_col = col_idx_to_a1(cols.status_fase_idx);
    let proposta_col = col_idx_to_a1(cols.proposta_original_resumo_idx);
    let cat_col = col_idx_to_a1(cols.categoria_arquitetural_idx);

    let r = row_number_1based;
    let mut ranges = HashMap::new();
    ranges.insert(format!("{status_col}{r}:{status_col}{r}"), vec![vec![STATUS_CONCLUIDO.to_string()]]);
    ranges.insert(format!("{fase_col}{r}:{fase_col}{r}"), vec![vec![FASE_OK.to_string()]]);
    ranges.insert(
        format!("{proposta_col}{r}:{proposta_col}{r}"),
        vec![vec![out.proposta_original_resumo.trim().to_string()]],
    );
    ranges.insert(
        format!("{cat_col}{r}:{cat_col}{r}"),
        vec![vec![out.categoria_arquitetural.trim().to_string()]],
    );
    ranges
}

async fn process_one_row(
    llm: &OpenRouterClient,
    cols: &Columns,
    row: &PendingRow,
) -> Result<Option<HashMap<String, Vec<Vec<String>>>>, String> {
    let readme = fetch_readme_truncated(&row.repo_url).await?;
    let prompt = build_prompt(&readme);
    let mut out = llm.triage(&prompt).await?;

    let urls = extract_urls_from_text(&readme, 500);
    let repo_ids = extract_github_repo_ids(&urls, 200);
    if !repo_ids.is_empty() && looks_like_content_repo(&readme, repo_ids.len()) {
        match llm.dedup_links_default_model(&repo_ids).await {
            Ok(deduped) => {
                if !deduped.is_empty() {
                    let mut take_n = MAX_DEDUP_LINKS_IN_RESUMO.min(deduped.len());
                    loop {
                        let suffix = format!(" | Links-chave: {}", deduped[..take_n].join(", "));
                        let candidate = format!("{}{}", out.proposta_original_resumo.trim(), suffix);
                        if candidate.chars().count() <= MAX_RESUMO_CHARS || take_n <= 1 {
                            if candidate.chars().count() <= MAX_RESUMO_CHARS {
                                out.proposta_original_resumo = candidate;
                            }
                            break;
                        }
                        take_n = take_n.saturating_sub(1);
                    }
                }
            }
            Err(e) => {
                warn!(row = row.row_number_1based, repo_url = %row.repo_url, error = %e, "Batedor: dedup de links falhou; seguindo sem links");
            }
        }
    }

    Ok(Some(build_success_ranges(cols, row.row_number_1based, &out)))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let level = match rust_log.to_ascii_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .ok()
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;

    let dry_run = parse_cli_args();

    let sheets = SheetsMcpClient;
    let header = sheets
        .get_sheet_data(&spreadsheet_id, MASTER_SOLUTIONS_SHEET, "A1:CF1".to_string())
        .await
        .map_err(io::Error::other)?;
    let header_row = header.first().cloned().unwrap_or_default();
    if header_row.is_empty() {
        return Err(io::Error::other(
            "Header vazio em MASTER_SOLUTIONS!A1:CF1. Verifique GOOGLE_SHEETS_ID, nome da aba e se o header está na linha 1.",
        ));
    }
    let cols = resolve_columns(&header_row).map_err(io::Error::other)?;

    let required = [
        cols.status_atualizacao_idx,
        cols.status_fase_idx,
        cols.repo_url_idx,
        cols.proposta_original_resumo_idx,
        cols.categoria_arquitetural_idx,
    ];
    let min_idx = *required.iter().min().unwrap_or(&0);
    let max_idx = *required.iter().max().unwrap_or(&0);
    let start_col = col_idx_to_a1(min_idx);
    let end_col = col_idx_to_a1(max_idx);
    let values = sheets
        .get_sheet_data(&spreadsheet_id, MASTER_SOLUTIONS_SHEET, format!("{start_col}2:{end_col}"))
        .await
        .map_err(io::Error::other)?;

    let (pending, scan) = find_gatilho_rows(&values, &cols);
    info!(
        total_rows = values.len(),
        pending = scan.pending,
        skipped_non_trigger = scan.skipped_non_trigger,
        skipped_missing_repo_url = scan.skipped_missing_repo_url,
        gatilho = STATUS_GATILHO,
        "Batedor: scan de gatilho concluído"
    );
    if pending.is_empty() {
        return Ok(());
    }
    if dry_run {
        for row in pending.iter().take(10) {
            info!(row = row.row_number_1based, repo_url = %row.repo_url, "Batedor: dry-run candidato");
        }
        info!(dry_run, "Batedor: dry-run (sem GitHub/LLM e sem writes)");
        return Ok(());
    }

    let llm = OpenRouterClient::new().map_err(io::Error::other)?;

    let mut pending_ranges: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut pending_updates = 0usize;
    let total_to_write = pending.len();
    let mut written_so_far = 0usize;
    let started = Instant::now();
    let processed = Arc::new(AtomicUsize::new(0));
    let processed_for_ghost = Arc::clone(&processed);
    let ghost_started = started;
    let ghost_handle = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let done = processed_for_ghost.load(Ordering::Relaxed);
            info!(
                done,
                total = total_to_write,
                elapsed_s = ghost_started.elapsed().as_secs(),
                "Ghost Telemetry: Batedor processando"
            );
        }
    });
    let _ghost = AbortOnDrop(ghost_handle);

    for row in pending {
        let plan = process_one_row(&llm, &cols, &row).await;
        let plan = match plan {
            Ok(Some(p)) => p,
            Ok(None) => continue,
            Err(e) => {
                warn!(row = row.row_number_1based, repo_url = %row.repo_url, error = %e, "Batedor: falha por repositório (sem write)");
                processed.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };
        for (k, v) in plan {
            pending_ranges.insert(k, v);
        }
        pending_updates += 1;
        processed.fetch_add(1, Ordering::Relaxed);

        if pending_updates >= MAX_UPDATES_PER_BATCH {
            let batch = std::mem::take(&mut pending_ranges);
            written_so_far = written_so_far.saturating_add(pending_updates);
            info!(
                written_so_far,
                total_to_write,
                updates = pending_updates,
                "Batedor: flush Sheets batch"
            );
            sheets
                .batch_update_cells(&spreadsheet_id, MASTER_SOLUTIONS_SHEET, batch)
                .await
                .map_err(io::Error::other)?;
            pending_updates = 0;
        }
    }

    if pending_updates > 0 {
        let batch = std::mem::take(&mut pending_ranges);
        written_so_far = written_so_far.saturating_add(pending_updates);
        info!(
            written_so_far,
            total_to_write,
            updates = pending_updates,
            "Batedor: flush Sheets final"
        );
        sheets
            .batch_update_cells(&spreadsheet_id, MASTER_SOLUTIONS_SHEET, batch)
            .await
            .map_err(io::Error::other)?;
    }

    info!(elapsed_ms = started.elapsed().as_millis(), "Batedor: concluído");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_is_deterministic() {
        let big = "a".repeat(10_000);
        let out = truncate_chars(&big, 3_000);
        assert_eq!(out.len(), 3_000);
        let small = "abc";
        assert_eq!(truncate_chars(small, 3_000), "abc");
    }

    #[test]
    fn header_normalization_accepts_portuguese_diacritics() {
        assert_eq!(normalize_header_cell("status_atualização"), "status_atualizacao");
        assert_eq!(normalize_header_cell("CATEGORIA_ARQUITETURAL"), "categoria_arquitetural");
        assert_eq!(normalize_header_cell("proposta-original-resumo"), "proposta_original_resumo");
    }

    #[test]
    fn model_normalization_maps_deepseek_v4_to_flash() {
        assert_eq!(
            normalize_batedor_model("deepseek/deepseek-v4"),
            "deepseek/deepseek-v4-flash"
        );
        assert_eq!(
            normalize_batedor_model("deepseek/deepseek-v4  #comment"),
            "deepseek/deepseek-v4-flash"
        );
    }

    #[test]
    fn enum_validation_rejects_outside_catalog() {
        let out = BatedorOut {
            proposta_original_resumo: "Ferramenta que faz X.".to_string(),
            categoria_arquitetural: "QualquerCoisa".to_string(),
        };
        assert!(validate_batedor_out(&out).is_err());
    }

    #[test]
    fn json_validation_requires_fields_and_types() {
        let ok = r#"{"proposta_original_resumo":"Ferramenta CLI para triagem.","categoria_arquitetural":"Tooling_Dev - CLI_Utilities"}"#;
        let parsed: BatedorOut = serde_json::from_str(ok).unwrap();
        assert!(validate_batedor_out(&parsed).is_ok());

        let missing = r#"{"categoria_arquitetural":"Tooling_Dev - CLI_Utilities"}"#;
        assert!(serde_json::from_str::<BatedorOut>(missing).is_err());
    }

    #[test]
    fn response_schema_is_strict_and_has_two_keys() {
        let rf = response_format_for_batedor();
        assert_eq!(rf.get("type").and_then(|v| v.as_str()), Some("json_schema"));
        let schema = rf
            .get("json_schema")
            .and_then(|v| v.get("schema"))
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(
            schema.get("additionalProperties").and_then(|v| v.as_bool()),
            Some(false)
        );
        let props = schema.get("properties").and_then(|v| v.as_object()).unwrap();
        assert!(props.get("proposta_original_resumo").is_some());
        assert!(props.get("categoria_arquitetural").is_some());
        assert_eq!(props.len(), 2);
    }

    #[test]
    fn status_update_happens_only_on_success() {
        let cols = Columns {
            status_atualizacao_idx: 0,
            status_fase_idx: 1,
            repo_url_idx: 3,
            proposta_original_resumo_idx: 10,
            categoria_arquitetural_idx: 11,
        };
        let out = BatedorOut {
            proposta_original_resumo: "Ferramenta CLI para triagem barata.".to_string(),
            categoria_arquitetural: "Tooling_Dev - CLI_Utilities".to_string(),
        };
        assert!(validate_batedor_out(&out).is_ok());
        let ranges = build_success_ranges(&cols, 2, &out);
        assert!(ranges.get("A2:A2").is_some());
        assert!(ranges.get("B2:B2").is_some());
        assert!(ranges.get("K2:K2").is_some());
        assert!(ranges.get("L2:L2").is_some());

        let bad = BatedorOut {
            proposta_original_resumo: "".to_string(),
            categoria_arquitetural: "Tooling_Dev - CLI_Utilities".to_string(),
        };
        assert!(validate_batedor_out(&bad).is_err());
    }
}
