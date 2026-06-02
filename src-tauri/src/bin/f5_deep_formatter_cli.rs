use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use reqwest::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncBufReadExt;
use tokio::time::Instant;
use tracing::{info, warn};

const MASTER_SOLUTIONS_SHEET: &str = "MASTER_SOLUTIONS";
const DEEP_COMPONENTS_SHEET: &str = "DEEP_COMPONENTS";

const STATUS_PENDING_F5: &str = "APROVADO_DEEP_COMPONENTS_ANALYSIS";
const STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO: &str = "CONCLUIDO_AGUARDANDO";
const STATUS_FASE_F5_OK: &str = "FASE_5_DEEP_COMPONENTS_OK";

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn normalize_header_cell(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
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

fn extract_values_2d(result: &Value) -> Option<Vec<Vec<String>>> {
    let values = if let Some(values) = result.get("values").and_then(|v| v.as_array()) {
        values
    } else {
        let vr = result.get("valueRanges")?.as_array()?;
        let first = vr.first()?;
        first.get("values")?.as_array()?
    };
    let mut out = Vec::with_capacity(values.len());
    for row in values {
        let Some(cells) = row.as_array() else {
            out.push(Vec::new());
            continue;
        };
        out.push(
            cells
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect(),
        );
    }
    Some(out)
}

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

type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
type SheetsUpdateFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
type LlmComponentsFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(DeepComponentsEnvelope, f64), String>> + Send + 'a>>;

trait LlmClient: Send + Sync {
    fn run_components<'a>(&'a self, prompt: &'a str) -> LlmComponentsFuture<'a>;
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
        let started = Instant::now();
        let mut lines = reader.lines();
        loop {
            if started.elapsed() > timeout {
                return Err(format!(
                    "Timeout: O servidor MCP (Sheets) não emitiu o payload após {} segundos. Verifique dependências Node/Python.",
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
                "clientInfo": { "name": "f5-deep-formatter", "version": "1.0.0" }
            }
        });
        let initialized_notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let mcp_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
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
        let timeout = Duration::from_secs(20);
        let msg =
            Self::poll_for_jsonrpc_response_from_reader(BufReader::new(stdout), timeout).await?;

        let _ = child.kill().await;
        let _ = child.wait().await;

        if msg.get("error").is_some() {
            return Err(format!("MCP retornou erro: {msg}"));
        }
        if let Some(result) = msg.get("result") {
            return Ok(Self::normalize_mcp_tool_result(result.clone()));
        }

        Err("Resposta MCP inválida (sem campo result)".to_string())
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
}

impl SheetsClient for SheetsMcpClient {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>
    {
        Box::pin(async move {
            let result = Self::call_mcp(
                "get_sheet_data",
                json!({
                    "spreadsheet_id": spreadsheet_id,
                    "sheet": sheet,
                    "range": range,
                    "include_grid_data": false
                }),
            )
            .await?;
            Ok(extract_values_2d(&result).unwrap_or_default())
        })
    }

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: HashMap<String, Vec<Vec<String>>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let mut payload_ranges = serde_json::Map::new();
            for (range, values) in ranges {
                payload_ranges.insert(range, json!(values));
            }
            let _ = Self::call_mcp(
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
struct MasterColumns {
    status_atualizacao_idx: usize,
    status_fase_idx: usize,
    project_name_idx: usize,
    repo_url_idx: usize,
    repo_version_idx: Option<usize>,
    lote_id_idx: usize,
}

fn resolve_master_columns(header_row: &[String]) -> Result<MasterColumns, String> {
    let mut status_atualizacao_idx = None;
    let mut status_fase_idx = None;
    let mut project_name_idx = None;
    let mut repo_url_idx = None;
    let mut repo_version_idx = None;
    let mut lote_id_idx = None;

    for (idx, raw) in header_row.iter().enumerate() {
        let h = normalize_header_cell(raw);
        match h.as_str() {
            "status_atualizacao" => status_atualizacao_idx = Some(idx),
            "status_fase" => status_fase_idx = Some(idx),
            "project_name" => project_name_idx = Some(idx),
            "repo_url" => repo_url_idx = Some(idx),
            "repo_analised_version" => repo_version_idx = Some(idx),
            "repo_version" => {
                if repo_version_idx.is_none() {
                    repo_version_idx = Some(idx)
                }
            }
            "lote_id" => lote_id_idx = Some(idx),
            _ => {}
        }
    }

    Ok(MasterColumns {
        status_atualizacao_idx: status_atualizacao_idx
            .ok_or_else(|| "Cabeçalho sem status_atualizacao".to_string())?,
        status_fase_idx: status_fase_idx.ok_or_else(|| "Cabeçalho sem status_fase".to_string())?,
        project_name_idx: project_name_idx.ok_or_else(|| "Cabeçalho sem project_name".to_string())?,
        repo_url_idx: repo_url_idx.ok_or_else(|| "Cabeçalho sem repo_url".to_string())?,
        repo_version_idx,
        lote_id_idx: lote_id_idx.ok_or_else(|| "Cabeçalho sem lote_id".to_string())?,
    })
}

#[derive(Debug, Clone)]
struct ParentRow {
    row_number_1based: u32,
    project_name: String,
    repo_url: String,
    repo_analised_version: String,
    lote_id: String,
}

fn find_pending_phase5_rows(
    values: &[Vec<String>],
    cols: &MasterColumns,
) -> Vec<ParentRow> {
    let mut out = Vec::new();
    for (idx, row) in values.iter().enumerate() {
        let status = row
            .get(cols.status_atualizacao_idx)
            .map(|s| s.trim())
            .unwrap_or("");
        if status != STATUS_PENDING_F5 {
            continue;
        }
        let project_name = row
            .get(cols.project_name_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let repo_url = row
            .get(cols.repo_url_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let lote_id = row
            .get(cols.lote_id_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if project_name.is_empty() || repo_url.is_empty() || lote_id.is_empty() {
            continue;
        }
        let repo_analised_version = cols
            .repo_version_idx
            .and_then(|i| row.get(i))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        out.push(ParentRow {
            row_number_1based: (idx as u32) + 2,
            project_name,
            repo_url,
            repo_analised_version,
            lote_id,
        });
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DeepComponent {
    repo_url: String,
    lote_id: String,
    comp_id: String,
    comp_name: String,
    comp_purpose: String,
    comp_inputs: String,
    comp_outputs: String,
    comp_public_api: String,
    comp_internal_modules: String,
    comp_dependencies: String,
    comp_risks: String,
    comp_tests_hint: String,
    comp_integration_steps: String,
    model_used: String,
    cost_usd: f64,
    created_at_epoch: i64,
}

impl DeepComponent {
    fn to_sheet_row(&self) -> Vec<String> {
        vec![
            self.repo_url.clone(),
            self.lote_id.clone(),
            self.comp_id.clone(),
            self.comp_name.clone(),
            self.comp_purpose.clone(),
            self.comp_inputs.clone(),
            self.comp_outputs.clone(),
            self.comp_public_api.clone(),
            self.comp_internal_modules.clone(),
            self.comp_dependencies.clone(),
            self.comp_risks.clone(),
            self.comp_tests_hint.clone(),
            self.comp_integration_steps.clone(),
            self.model_used.clone(),
            format!("{:.6}", self.cost_usd),
            self.created_at_epoch.to_string(),
        ]
    }
}

#[derive(Debug, Clone, Deserialize)]
struct DeepComponentsEnvelope {
    components: Vec<DeepComponent>,
}

fn response_format_for_phase5() -> Value {
    fn strict_object(properties: serde_json::Map<String, Value>, required: Vec<&'static str>) -> Value {
        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": required,
            "additionalProperties": false
        })
    }

    fn bounded_string(max_len: u32) -> Value {
        json!({ "type": "string", "maxLength": max_len })
    }

    let mut comp_props = serde_json::Map::new();
    comp_props.insert("repo_url".to_string(), bounded_string(300));
    comp_props.insert("lote_id".to_string(), bounded_string(80));
    comp_props.insert(
        "comp_id".to_string(),
        json!({ "type": "string", "pattern": "^COMP_\\d{4}$", "maxLength": 16 }),
    );
    comp_props.insert("comp_name".to_string(), bounded_string(120));
    comp_props.insert("comp_purpose".to_string(), bounded_string(600));
    comp_props.insert("comp_inputs".to_string(), bounded_string(800));
    comp_props.insert("comp_outputs".to_string(), bounded_string(800));
    comp_props.insert("comp_public_api".to_string(), bounded_string(800));
    comp_props.insert("comp_internal_modules".to_string(), bounded_string(1200));
    comp_props.insert("comp_dependencies".to_string(), bounded_string(1200));
    comp_props.insert("comp_risks".to_string(), bounded_string(800));
    comp_props.insert("comp_tests_hint".to_string(), bounded_string(800));
    comp_props.insert("comp_integration_steps".to_string(), bounded_string(800));
    comp_props.insert("model_used".to_string(), bounded_string(120));
    comp_props.insert("cost_usd".to_string(), json!({ "type": "number", "minimum": 0 }));
    comp_props.insert(
        "created_at_epoch".to_string(),
        json!({ "type": "integer", "minimum": 0 }),
    );

    let comp_schema = strict_object(
        comp_props,
        vec![
            "repo_url",
            "lote_id",
            "comp_id",
            "comp_name",
            "comp_purpose",
            "comp_inputs",
            "comp_outputs",
            "comp_public_api",
            "comp_internal_modules",
            "comp_dependencies",
            "comp_risks",
            "comp_tests_hint",
            "comp_integration_steps",
            "model_used",
            "cost_usd",
            "created_at_epoch",
        ],
    );

    let mut env_props = serde_json::Map::new();
    env_props.insert(
        "components".to_string(),
        json!({
            "type": "array",
            "minItems": 1,
            "items": comp_schema
        }),
    );

    let schema = strict_object(env_props, vec!["components"]);
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "soda_f5_deep_components",
            "strict": true,
            "schema": schema
        }
    })
}

fn openrouter_body_for_phase5(prompt: &str) -> Value {
    let model = OpenRouterClient::model();
    json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "Responda SOMENTE com JSON válido (sem markdown, sem texto extra)."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.0,
        "max_tokens": 4096,
        "response_format": response_format_for_phase5()
    })
}

struct OpenRouterClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl OpenRouterClient {
    fn new() -> Result<Self, String> {
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
        let api_key = std::env::var("OPENROUTER_API_HEAVY_KEY")
            .ok()
            .map(|v| v.trim().trim_matches('"').to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "Missing OPENROUTER_API_HEAVY_KEY".to_string())?;
        Ok(Self {
            client: Client::new(),
            base_url,
            api_key,
        })
    }

    fn model() -> String {
        std::env::var("OPENROUTER_HEAVY_DEFAULT_MODEL")
            .ok()
            .map(|v| v.trim().trim_matches('"').to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "deepseek/deepseek-v4-pro".to_string())
    }

    fn extract_openrouter_content(json: &Value) -> Option<String> {
        fn flatten(value: &Value) -> Option<String> {
            match value {
                Value::String(text) => Some(text.trim()).filter(|t| !t.is_empty()).map(|t| t.to_string()),
                Value::Array(parts) => {
                    let mut out = String::new();
                    for part in parts {
                        if let Some(t) = flatten(part)
                            .or_else(|| part.get("text").and_then(flatten))
                            .or_else(|| part.get("content").and_then(flatten))
                            .or_else(|| part.get("value").and_then(flatten))
                        {
                            if !out.is_empty() {
                                out.push('\n');
                            }
                            out.push_str(&t);
                        }
                    }
                    Some(out).filter(|s| !s.trim().is_empty())
                }
                Value::Object(obj) => obj
                    .get("text")
                    .and_then(flatten)
                    .or_else(|| obj.get("content").and_then(flatten))
                    .or_else(|| obj.get("value").and_then(flatten)),
                _ => None,
            }
        }

        let choices = json.get("choices")?.as_array()?;
        let first = choices.first()?;
        if let Some(message) = first.get("message") {
            if let Some(content) = message.get("content") {
                if let Some(text) = flatten(content) {
                    return Some(text);
                }
            }
        }
        first.get("text").and_then(flatten)
    }

    fn harvest_cost_usd(json: &Value) -> f64 {
        let usage = &json["usage"];
        usage
            .get("total_cost")
            .or_else(|| usage.get("cost"))
            .or_else(|| usage.get("estimated_cost"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    async fn run_components(&self, prompt: &str) -> Result<(DeepComponentsEnvelope, f64), String> {
        let body = openrouter_body_for_phase5(prompt);

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Erro de rede: {e}"))?;

        let status = resp.status();
        let raw = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {}: {}", status.as_u16(), raw));
        }
        let envelope: Value =
            serde_json::from_str(&raw).map_err(|e| format!("Envelope JSON inválido do OpenRouter: {e}"))?;
        let cost = Self::harvest_cost_usd(&envelope);
        let content =
            Self::extract_openrouter_content(&envelope).ok_or_else(|| "Resposta vazia do OpenRouter".to_string())?;

        let parsed: DeepComponentsEnvelope =
            serde_json::from_str(&content).map_err(|e| format!("JSON inválido do modelo: {e}"))?;
        Ok((parsed, cost))
    }
}

impl LlmClient for OpenRouterClient {
    fn run_components<'a>(
        &'a self,
        prompt: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(DeepComponentsEnvelope, f64), String>> + Send + 'a>,
    > {
        Box::pin(async move { OpenRouterClient::run_components(self, prompt).await })
    }
}

fn ensure_deep_components_schema(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS deep_components (
            repo_url TEXT NOT NULL,
            lote_id TEXT NOT NULL,
            comp_id TEXT NOT NULL,
            comp_name TEXT NOT NULL,
            comp_purpose TEXT NOT NULL,
            comp_inputs TEXT NOT NULL,
            comp_outputs TEXT NOT NULL,
            comp_public_api TEXT NOT NULL,
            comp_internal_modules TEXT NOT NULL,
            comp_dependencies TEXT NOT NULL,
            comp_risks TEXT NOT NULL,
            comp_tests_hint TEXT NOT NULL,
            comp_integration_steps TEXT NOT NULL,
            model_used TEXT NOT NULL,
            cost_usd REAL NOT NULL,
            created_at_epoch INTEGER NOT NULL,
            PRIMARY KEY (repo_url, lote_id, comp_id)
        )",
        [],
    )
    .map_err(|e| format!("Falha ao criar tabela deep_components: {e}"))?;
    Ok(())
}

fn upsert_deep_component(conn: &Connection, c: &DeepComponent) -> Result<(), String> {
    conn.execute(
        "INSERT INTO deep_components (
            repo_url, lote_id, comp_id, comp_name, comp_purpose, comp_inputs, comp_outputs,
            comp_public_api, comp_internal_modules, comp_dependencies, comp_risks, comp_tests_hint,
            comp_integration_steps, model_used, cost_usd, created_at_epoch
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(repo_url, lote_id, comp_id) DO UPDATE SET
            comp_name=excluded.comp_name,
            comp_purpose=excluded.comp_purpose,
            comp_inputs=excluded.comp_inputs,
            comp_outputs=excluded.comp_outputs,
            comp_public_api=excluded.comp_public_api,
            comp_internal_modules=excluded.comp_internal_modules,
            comp_dependencies=excluded.comp_dependencies,
            comp_risks=excluded.comp_risks,
            comp_tests_hint=excluded.comp_tests_hint,
            comp_integration_steps=excluded.comp_integration_steps,
            model_used=excluded.model_used,
            cost_usd=excluded.cost_usd,
            created_at_epoch=excluded.created_at_epoch",
        params![
            c.repo_url,
            c.lote_id,
            c.comp_id,
            c.comp_name,
            c.comp_purpose,
            c.comp_inputs,
            c.comp_outputs,
            c.comp_public_api,
            c.comp_internal_modules,
            c.comp_dependencies,
            c.comp_risks,
            c.comp_tests_hint,
            c.comp_integration_steps,
            c.model_used,
            c.cost_usd,
            c.created_at_epoch
        ],
    )
    .map_err(|e| format!("Falha no UPSERT deep_components: {e}"))?;
    Ok(())
}

fn update_parent_status_local(conn: &Connection, project_name: &str) -> Result<(), String> {
    let _ = conn.execute(
        "UPDATE repo_heuristics
         SET status_atualizacao = ?2,
             status_fase = ?3
         WHERE project_name = ?1",
        params![project_name, STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO, STATUS_FASE_F5_OK],
    );
    Ok(())
}

fn compute_first_empty_row_1based(col_a_values: &[Vec<String>]) -> u32 {
    for (idx, row) in col_a_values.iter().enumerate() {
        let cell = row.first().map(|s| s.trim()).unwrap_or("");
        if cell.is_empty() {
            return (idx as u32) + 2;
        }
    }
    ((col_a_values.len() as u32) + 2).max(2)
}

fn build_prompt(parent: &ParentRow, seed: &str) -> String {
    format!(
        "SODA_PHASE=5\nrepo_url={}\nrepo_analised_version={}\nlote_id={}\n\nContexto_SGR_Minimo:\n{}\n\nTarefa: gere componentes COMP_0001..COMP_9999. Saída deve ser JSON estrito no shape do schema.\n",
        parent.repo_url,
        parent.repo_analised_version,
        parent.lote_id,
        seed
    )
}

fn fetch_seed_from_repo_heuristics(conn: &Connection, project_name: &str) -> String {
    let row = conn.query_row(
        "SELECT
            lente_a_sentido_prod_ux,
            lente_b_estrutura_arq,
            lente_c_realidade_ops,
            ouro_a_extrair,
            deep_pattern,
            transplantable_core,
            must_components_arq,
            must_components_ops
         FROM repo_heuristics
         WHERE project_name = ?1
         LIMIT 1",
        params![project_name],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
            ))
        },
    );
    match row {
        Ok((a, b, c, ouro, pat, core, arq, ops)) => format!(
            "lente_a={}\n\nlente_b={}\n\nlente_c={}\n\nouro_a_extrair={}\n\ndeep_pattern={}\n\ntransplantable_core={}\n\nmust_components_arq={}\n\nmust_components_ops={}",
            a, b, c, ouro, pat, core, arq, ops
        ),
        Err(_) => String::new(),
    }
}

struct ProcessCtx<'a> {
    spreadsheet_id: &'a str,
    cols: &'a MasterColumns,
    dry_run: bool,
    now_epoch: i64,
}

async fn process_parent_row<S: SheetsClient, L: LlmClient>(
    sheets: &S,
    llm: &L,
    conn: &mut Connection,
    ctx: &ProcessCtx<'_>,
    parent: &ParentRow,
) -> Result<(), String> {
    let seed = fetch_seed_from_repo_heuristics(conn, &parent.project_name);
    let mut prompt = build_prompt(parent, &seed);

    let mut last_err = None;
    let mut envelope = None;
    let mut cost_total = 0.0;
    for attempt in 1..=3u8 {
        match llm.run_components(&prompt).await {
            Ok((env, cost)) => {
                envelope = Some(env);
                cost_total = cost;
                break;
            }
            Err(e) => {
                last_err = Some(e.clone());
                warn!(
                    project_name = %parent.project_name,
                    attempt,
                    error = %e,
                    "F5: falha no formatador; retry"
                );
                prompt.push_str(&format!(
                    "\nERRO_ANTERIOR: {e}\nResponda novamente com JSON estrito.\n"
                ));
            }
        }
    }

    let env = envelope.ok_or_else(|| {
        format!(
            "F5: abortando repo (falha após retries): {}",
            last_err.unwrap_or_else(|| "unknown".to_string())
        )
    })?;

    if env.components.is_empty() {
        return Err("F5: modelo retornou components vazio".to_string());
    }

    let per_cost = if cost_total > 0.0 {
        cost_total / (env.components.len() as f64)
    } else {
        0.0
    };
    let model_used = OpenRouterClient::model();

    let mut normalized = Vec::new();
    for mut c in env.components {
        c.repo_url = parent.repo_url.clone();
        c.lote_id = parent.lote_id.clone();
        c.model_used = model_used.clone();
        c.cost_usd = per_cost;
        c.created_at_epoch = ctx.now_epoch;
        normalized.push(c);
    }

    if ctx.dry_run {
        info!(
            project_name = %parent.project_name,
            components = normalized.len(),
            "F5: dry-run (sem persistência)"
        );
        return Ok(());
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao abrir transação: {e}"))?;
    for c in &normalized {
        upsert_deep_component(&tx, c)?;
    }
    update_parent_status_local(&tx, &parent.project_name)?;
    tx.commit()
        .map_err(|e| format!("Falha ao commit transação: {e}"))?;

    let col_a_values = sheets
        .get_sheet_data(ctx.spreadsheet_id, DEEP_COMPONENTS_SHEET, "A2:A".to_string())
        .await?;
    let start_row = compute_first_empty_row_1based(&col_a_values);
    let col_count = normalized[0].to_sheet_row().len();
    let end_col = col_idx_to_a1(col_count - 1);
    let end_row = start_row + (normalized.len() as u32) - 1;
    let range = format!("A{start_row}:{end_col}{end_row}");
    let values_2d = normalized
        .iter()
        .map(|c| c.to_sheet_row())
        .collect::<Vec<_>>();
    let mut ranges = HashMap::new();
    ranges.insert(range, values_2d);
    sheets
        .batch_update_cells(ctx.spreadsheet_id, DEEP_COMPONENTS_SHEET, ranges)
        .await?;

    let status_col = col_idx_to_a1(ctx.cols.status_atualizacao_idx);
    let fase_col = col_idx_to_a1(ctx.cols.status_fase_idx);
    let mut master_ranges = HashMap::new();
    master_ranges.insert(
        format!(
            "{status_col}{}:{status_col}{}",
            parent.row_number_1based, parent.row_number_1based
        ),
        vec![vec![STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO.to_string()]],
    );
    master_ranges.insert(
        format!(
            "{fase_col}{}:{fase_col}{}",
            parent.row_number_1based, parent.row_number_1based
        ),
        vec![vec![STATUS_FASE_F5_OK.to_string()]],
    );
    sheets
        .batch_update_cells(ctx.spreadsheet_id, MASTER_SOLUTIONS_SHEET, master_ranges)
        .await?;

    info!(
        project_name = %parent.project_name,
        row_number = parent.row_number_1based,
        deep_rows_written = normalized.len(),
        "F5: concluído"
    );

    Ok(())
}

fn parse_cli_args() -> (Option<String>, bool, Option<usize>) {
    let mut args = std::env::args();
    args.next();
    let mut repo = None;
    let mut dry_run = false;
    let mut max = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo" => repo = args.next(),
            "--dry-run" => dry_run = true,
            "--max" => max = args.next().and_then(|v| v.parse::<usize>().ok()),
            _ => {}
        }
    }
    (repo, dry_run, max)
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

    let (repo_filter, dry_run, max_rows) = parse_cli_args();

    let vault_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    let mut conn = Connection::open(&vault_path)
        .map_err(|e| io::Error::other(format!("Falha ao abrir vault em {}: {e}", vault_path.display())))?;
    ensure_deep_components_schema(&conn).map_err(io::Error::other)?;

    let sheets = SheetsMcpClient;
    let header = sheets
        .get_sheet_data(&spreadsheet_id, MASTER_SOLUTIONS_SHEET, "A1:CF1".to_string())
        .await
        .map_err(io::Error::other)?;
    let header_row = header.first().cloned().unwrap_or_default();
    let cols = resolve_master_columns(&header_row).map_err(io::Error::other)?;

    let values = sheets
        .get_sheet_data(&spreadsheet_id, MASTER_SOLUTIONS_SHEET, "A2:CF".to_string())
        .await
        .map_err(io::Error::other)?;
    let mut pending = find_pending_phase5_rows(&values, &cols);

    if let Some(repo) = repo_filter {
        pending.retain(|r| r.project_name == repo);
    }
    if let Some(max) = max_rows {
        pending.truncate(max);
    }

    info!(
        count = pending.len(),
        dry_run,
        gate = STATUS_PENDING_F5,
        "F5: candidatos no gatilho rígido"
    );

    if pending.is_empty() {
        return Ok(());
    }

    let llm = OpenRouterClient::new().map_err(io::Error::other)?;
    let now_epoch = chrono::Utc::now().timestamp();
    let ctx = ProcessCtx {
        spreadsheet_id: &spreadsheet_id,
        cols: &cols,
        dry_run,
        now_epoch,
    };
    for parent in pending {
        if let Err(e) = process_parent_row(&sheets, &llm, &mut conn, &ctx, &parent).await {
            warn!(project_name = %parent.project_name, error = %e, "F5: falha ao processar repo");
            continue;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn schema_is_strict_and_requires_components() {
        let rf = response_format_for_phase5();
        assert_eq!(rf.get("type").and_then(|v| v.as_str()), Some("json_schema"));
        assert_eq!(
            rf.get("json_schema")
                .and_then(|v| v.get("strict"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        let schema = rf
            .get("json_schema")
            .and_then(|v| v.get("schema"))
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(
            schema
                .get("additionalProperties")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
        let comps = schema
            .get("properties")
            .and_then(|v| v.get("components"))
            .unwrap();
        assert_eq!(comps.get("type").and_then(|v| v.as_str()), Some("array"));
        let item = comps.get("items").unwrap();
        let props = item.get("properties").and_then(|v| v.as_object()).unwrap();
        let comp_id = props.get("comp_id").unwrap();
        assert_eq!(
            comp_id.get("pattern").and_then(|v| v.as_str()),
            Some("^COMP_\\d{4}$")
        );
    }

    #[test]
    fn gating_filters_only_aprovado_deep_components_analysis() {
        let cols = MasterColumns {
            status_atualizacao_idx: 0,
            status_fase_idx: 1,
            project_name_idx: 2,
            repo_url_idx: 3,
            repo_version_idx: Some(4),
            lote_id_idx: 6,
        };
        let values = vec![
            vec![
                "CONCLUIDO".to_string(),
                "F4".to_string(),
                "a".to_string(),
                "https://github.com/a/a".to_string(),
                "v1".to_string(),
                "".to_string(),
                "L1".to_string(),
            ],
            vec![
                "APROVADO_DEEP_COMPONENTS_ANALYSIS".to_string(),
                "F4".to_string(),
                "b".to_string(),
                "https://github.com/b/b".to_string(),
                "".to_string(),
                "".to_string(),
                "L2".to_string(),
            ],
        ];
        let out = find_pending_phase5_rows(&values, &cols);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].project_name, "b");
    }

    #[test]
    fn compute_first_empty_row_is_never_header() {
        let values = vec![vec!["x".to_string()], vec!["".to_string()]];
        assert_eq!(compute_first_empty_row_1based(&values), 3);
        let values2: Vec<Vec<String>> = Vec::new();
        assert_eq!(compute_first_empty_row_1based(&values2), 2);
    }

    #[test]
    fn sqlite_upsert_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_deep_components_schema(&conn).unwrap();
        let c1 = DeepComponent {
            repo_url: "u".to_string(),
            lote_id: "l".to_string(),
            comp_id: "COMP_0001".to_string(),
            comp_name: "n1".to_string(),
            comp_purpose: "p".to_string(),
            comp_inputs: "i".to_string(),
            comp_outputs: "o".to_string(),
            comp_public_api: "api".to_string(),
            comp_internal_modules: "m".to_string(),
            comp_dependencies: "d".to_string(),
            comp_risks: "r".to_string(),
            comp_tests_hint: "t".to_string(),
            comp_integration_steps: "s".to_string(),
            model_used: "model".to_string(),
            cost_usd: 0.1,
            created_at_epoch: 1,
        };
        upsert_deep_component(&conn, &c1).unwrap();
        let mut c2 = c1.clone();
        c2.comp_name = "n2".to_string();
        upsert_deep_component(&conn, &c2).unwrap();
        let got: String = conn
            .query_row(
                "SELECT comp_name FROM deep_components WHERE repo_url=?1 AND lote_id=?2 AND comp_id=?3",
                params!["u", "l", "COMP_0001"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(got, "n2");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deep_components", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn openrouter_posts_with_json_schema_strict() {
        let mut server = Server::new_async().await;

        let comp = serde_json::json!({
            "components": [{
                "repo_url":"u",
                "lote_id":"l",
                "comp_id":"COMP_0001",
                "comp_name":"n",
                "comp_purpose":"p",
                "comp_inputs":"i",
                "comp_outputs":"o",
                "comp_public_api":"api",
                "comp_internal_modules":"m",
                "comp_dependencies":"d",
                "comp_risks":"r",
                "comp_tests_hint":"t",
                "comp_integration_steps":"s",
                "model_used":"x",
                "cost_usd":0.0,
                "created_at_epoch":1
            }]
        })
        .to_string();

        let response = serde_json::json!({
            "usage": { "total_cost": 1.23 },
            "choices": [{ "message": { "content": comp } }]
        })
        .to_string();

        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response)
            .create_async()
            .await;

        let client = OpenRouterClient {
            client: Client::new(),
            base_url: server.url(),
            api_key: "test-key".to_string(),
        };
        let (env, cost) = client.run_components("PROMPT").await.unwrap();
        mock.assert_async().await;
        assert_eq!(cost, 1.23);
        assert_eq!(env.components.len(), 1);
        assert_eq!(env.components[0].comp_id, "COMP_0001");
    }

    #[test]
    fn openrouter_body_includes_response_format_json_schema_strict() {
        let body = openrouter_body_for_phase5("PROMPT");
        assert_eq!(body.get("temperature").and_then(|v| v.as_f64()), Some(0.0));
        assert_eq!(body.get("max_tokens").and_then(|v| v.as_i64()), Some(4096));
        assert_eq!(
            body.pointer("/response_format/type")
                .and_then(|v| v.as_str()),
            Some("json_schema")
        );
        assert_eq!(
            body.pointer("/response_format/json_schema/strict")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            body.pointer("/response_format/json_schema/schema/additionalProperties")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    struct MockLlm {
        calls: Arc<Mutex<Vec<String>>>,
        response: DeepComponentsEnvelope,
        cost: f64,
    }

    impl MockLlm {
        fn new(response: DeepComponentsEnvelope, cost: f64) -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                response,
                cost,
            }
        }
    }

    impl LlmClient for MockLlm {
        fn run_components<'a>(
            &'a self,
            prompt: &'a str,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(DeepComponentsEnvelope, f64), String>> + Send + 'a>,
        > {
            Box::pin(async move {
                self.calls.lock().await.push(prompt.to_string());
                Ok((self.response.clone(), self.cost))
            })
        }
    }

    struct MockSheets {
        updates: Mutex<Vec<(String, HashMap<String, Vec<Vec<String>>>)>>,
        deep_col_a: Vec<Vec<String>>,
    }

    impl MockSheets {
        fn new(deep_col_a: Vec<Vec<String>>) -> Self {
            Self {
                updates: Mutex::new(Vec::new()),
                deep_col_a,
            }
        }
    }

    impl SheetsClient for MockSheets {
        fn get_sheet_data<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            range: String,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>
        {
            Box::pin(async move {
                if range == "A2:A" {
                    return Ok(self.deep_col_a.clone());
                }
                Ok(Vec::new())
            })
        }

        fn batch_update_cells<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            sheet: &'a str,
            ranges: HashMap<String, Vec<Vec<String>>>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async move {
                self.updates
                    .lock()
                    .await
                    .push((sheet.to_string(), ranges));
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn orchestrator_persists_sqlite_and_updates_sheets() {
        let mut conn = Connection::open_in_memory().unwrap();
        ensure_deep_components_schema(&conn).unwrap();

        let env = DeepComponentsEnvelope {
            components: vec![
                DeepComponent {
                    repo_url: "".to_string(),
                    lote_id: "".to_string(),
                    comp_id: "COMP_0001".to_string(),
                    comp_name: "n1".to_string(),
                    comp_purpose: "p".to_string(),
                    comp_inputs: "i".to_string(),
                    comp_outputs: "o".to_string(),
                    comp_public_api: "api".to_string(),
                    comp_internal_modules: "m".to_string(),
                    comp_dependencies: "d".to_string(),
                    comp_risks: "r".to_string(),
                    comp_tests_hint: "t".to_string(),
                    comp_integration_steps: "s".to_string(),
                    model_used: "".to_string(),
                    cost_usd: 0.0,
                    created_at_epoch: 0,
                },
                DeepComponent {
                    repo_url: "".to_string(),
                    lote_id: "".to_string(),
                    comp_id: "COMP_0002".to_string(),
                    comp_name: "n2".to_string(),
                    comp_purpose: "p".to_string(),
                    comp_inputs: "i".to_string(),
                    comp_outputs: "o".to_string(),
                    comp_public_api: "api".to_string(),
                    comp_internal_modules: "m".to_string(),
                    comp_dependencies: "d".to_string(),
                    comp_risks: "r".to_string(),
                    comp_tests_hint: "t".to_string(),
                    comp_integration_steps: "s".to_string(),
                    model_used: "".to_string(),
                    cost_usd: 0.0,
                    created_at_epoch: 0,
                },
            ],
        };

        let llm = MockLlm::new(env, 1.0);
        let sheets = MockSheets::new(vec![vec!["x".to_string()]]);

        let cols = MasterColumns {
            status_atualizacao_idx: 0,
            status_fase_idx: 1,
            project_name_idx: 2,
            repo_url_idx: 3,
            repo_version_idx: Some(4),
            lote_id_idx: 6,
        };
        let parent = ParentRow {
            row_number_1based: 42,
            project_name: "proj".to_string(),
            repo_url: "https://github.com/acme/proj".to_string(),
            repo_analised_version: "v1".to_string(),
            lote_id: "LOTE".to_string(),
        };

        let ctx = ProcessCtx {
            spreadsheet_id: "SHEET_ID",
            cols: &cols,
            dry_run: false,
            now_epoch: 999,
        };

        process_parent_row(&sheets, &llm, &mut conn, &ctx, &parent)
            .await
            .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deep_components", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
        let got: (String, String, f64, i64) = conn
            .query_row(
                "SELECT repo_url, lote_id, cost_usd, created_at_epoch FROM deep_components WHERE comp_id=?1",
                params!["COMP_0001"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(got.0, "https://github.com/acme/proj");
        assert_eq!(got.1, "LOTE");
        assert_eq!(got.2, 0.5);
        assert_eq!(got.3, 999);

        let updates = sheets.updates.lock().await;
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].0, DEEP_COMPONENTS_SHEET);
        assert!(updates[0].1.contains_key("A3:P4"));
        assert_eq!(updates[1].0, MASTER_SOLUTIONS_SHEET);
        assert!(updates[1].1.contains_key("A42:A42"));
        assert!(updates[1].1.contains_key("B42:B42"));
    }
}
