use std::io;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use genesis_mc_lib::cognition::phase3_4::{run_phase3_sgr, Block0Context, Phase3Config, Phase3Error, OFFICIAL_FORMATTER_MODEL};
use genesis_mc_lib::persist::ssot_injector::SsotInjector;
use reqwest::Client;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tracing::{error, info};

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn now_epoch_secs() -> io::Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("Falha ao calcular timestamp atual: {}", e)))?
        .as_secs() as i64)
}

fn sanitize_repo_id(repo_id: &str) -> String {
    repo_id
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn parse_repo_id_from_args() -> String {
    let mut args = std::env::args();
    args.next();
    let mut repo_id = String::from("aaif-goose/goose");
    while let Some(arg) = args.next() {
        if arg == "--repo" {
            if let Some(value) = args.next() {
                repo_id = value;
            }
        }
    }
    repo_id
}

fn get_first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
    })
}

#[derive(Debug, Default, Clone)]
struct UsageTotals {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
}

struct OpenRouterFormatterClient {
    client: Client,
    base_url: String,
    api_key: String,
    usage: std::sync::Arc<std::sync::Mutex<UsageTotals>>,
}

impl OpenRouterFormatterClient {
    fn from_env() -> Result<Self, String> {
        let api_key = get_first_env(&[
            "OPENROUTER_API_HEAVY_KEY",
            "OPENROUTER_API_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
        ])
        .ok_or_else(|| "OPENROUTER_API_HEAVY_KEY/OPENROUTER_API_KEY/OPENROUTER_API_FAST_KEY/OPENROUTER_API_FREE_KEY ausente".to_string())?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .map(|base| format!("{}/chat/completions", base.trim_end_matches('/')))
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1/chat/completions".to_string());

        Ok(Self {
            client: Client::new(),
            base_url,
            api_key,
            usage: std::sync::Arc::new(std::sync::Mutex::new(UsageTotals::default())),
        })
    }

    fn usage_totals(&self) -> UsageTotals {
        match self.usage.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => UsageTotals::default(),
        }
    }

    fn extract_openrouter_content(json: &Value) -> Option<String> {
        let content = &json["choices"][0]["message"]["content"];
        if let Some(text) = content.as_str() {
            return Some(text.to_string());
        }
        let parts = content.as_array()?;
        let joined = parts
            .iter()
            .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        if joined.trim().is_empty() {
            None
        } else {
            Some(joined)
        }
    }

    fn harvest_usage(&self, json: &Value) {
        let usage = &json["usage"];
        let prompt = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let completion = usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cost = usage
            .get("total_cost")
            .or_else(|| usage.get("cost"))
            .or_else(|| usage.get("estimated_cost"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if let Ok(mut guard) = self.usage.lock() {
            guard.prompt_tokens = guard.prompt_tokens.saturating_add(prompt);
            guard.completion_tokens = guard.completion_tokens.saturating_add(completion);
            guard.total_tokens = guard.total_tokens.saturating_add(total);
            guard.total_cost_usd += cost;
        }
    }
}

impl genesis_mc_lib::cognition::phase3_4::FormatterClient for OpenRouterFormatterClient {
    fn format<'a>(
        &'a self,
        model: &'a str,
        prompt: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let body = json!({
                "model": model,
                "messages": [
                    {
                        "role": "system",
                        "content": "Responda somente com um bloco Markdown ```json ... ``` contendo JSON válido. Sem texto fora do code-fence."
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "temperature": 0.0,
                "max_tokens": 4096
            });

            let response = self
                .client
                .post(&self.base_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Erro de rede: {}", e))?;

            let status = response.status();
            let raw = response.text().await.map_err(|e| e.to_string())?;
            if !status.is_success() {
                return Err(format!("HTTP {}: {}", status.as_u16(), raw));
            }

            let envelope: Value = serde_json::from_str(&raw)
                .map_err(|e| format!("Envelope JSON inválido do OpenRouter: {}", e))?;
            self.harvest_usage(&envelope);
            Self::extract_openrouter_content(&envelope).ok_or_else(|| "Resposta vazia do OpenRouter".to_string())
        })
    }
}

fn fetch_debates(conn: &Connection, repo_id: &str) -> io::Result<(String, String, String)> {
    conn.query_row(
        "SELECT lens_a_json, lens_b_json, lens_c_json
         FROM debates_enxame
         WHERE repo_id = ?1
         LIMIT 1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(|e| io::Error::other(format!("Debates da Fase 2 ausentes em debates_enxame para {}: {}", repo_id, e)))
}

fn fetch_repo_core(conn: &Connection, repo_id: &str) -> io::Result<(String, String)> {
    conn.query_row(
        "SELECT lote_id, repo_url
         FROM repositorios
         WHERE project_name = ?1
         LIMIT 1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|e| io::Error::other(format!("Metadados base ausentes em repositorios para {}: {}", repo_id, e)))
}

fn try_fetch_repo_heuristics_seed(conn: &Connection, repo_id: &str) -> Option<(String, String, String, String, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT repo_version, ultima_versao_online, licenca, stack_base, declared_description
             FROM repo_heuristics
             WHERE project_name = ?1
             LIMIT 1",
        )
        .ok()?;
    stmt.query_row(params![repo_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })
    .ok()
}

fn call_mcp(tool_name: &str, arguments: Value) -> io::Result<Value> {
    use std::process::{Command, Stdio};

    let creds = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .map_err(|_| io::Error::other("Missing GOOGLE_APPLICATION_CREDENTIALS"))?;

    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "phase3-4-cli", "version": "1.0.0" }
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
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io::Error::other(format!("Falha ao spawnar mcp-google-sheets: {}", e)))?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().ok_or_else(|| io::Error::other("stdin indisponível"))?;
        writeln!(stdin, "{}", init_req)?;
        writeln!(stdin, "{}", initialized_notif)?;
        writeln!(stdin, "{}", mcp_request)?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| io::Error::other(format!("Falha ao aguardar mcp-google-sheets: {}", e)))?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "mcp-google-sheets falhou. Exit {}. STDERR: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    for line in stdout_str.lines().rev() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                if value.get("error").is_some() {
                    return Err(io::Error::other(format!("MCP retornou erro: {}", value)));
                }
                if let Some(result) = value.get("result") {
                    return Ok(normalize_mcp_tool_result(result.clone()));
                }
            }
        }
    }

    Err(io::Error::other("Resposta MCP não encontrada no stdout"))
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

fn extract_values_2d(result: &Value) -> Option<Vec<Vec<String>>> {
    if let Some(values) = result.get("values").and_then(|v| v.as_array()) {
        let mut out = Vec::new();
        for row in values {
            let arr = row.as_array()?;
            out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
        }
        return Some(out);
    }

    if let Some(ranges) = result.get("valueRanges").and_then(|v| v.as_array()) {
        let first = ranges.first()?;
        let values = first.get("values").and_then(|v| v.as_array())?;
        let mut out = Vec::new();
        for row in values {
            let arr = row.as_array()?;
            out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
        }
        return Some(out);
    }

    if let Some(grid) = result.get("data") {
        if let Some(values) = grid.get("values").and_then(|v| v.as_array()) {
            let mut out = Vec::new();
            for row in values {
                let arr = row.as_array()?;
                out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
            }
            return Some(out);
        }
    }

    None
}

fn resolve_master_solutions_row_number(repo_id: &str) -> io::Result<u32> {
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let range = "A2:A5000";
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )?;

    let values = extract_values_2d(&result)
        .ok_or_else(|| io::Error::other("Formato inesperado do retorno get_sheet_data"))?;

    for (idx, row) in values.iter().enumerate() {
        let cell = row.get(0).map(|s| s.trim()).unwrap_or("");
        if cell.eq_ignore_ascii_case(repo_id) {
            return Ok((idx as u32) + 2);
        }
    }

    for (idx, row) in values.iter().enumerate() {
        let cell = row.get(0).map(|s| s.trim()).unwrap_or("");
        if cell.is_empty() {
            return Ok((idx as u32) + 2);
        }
    }

    info!("MASTER_SOLUTIONS sem espaço no range A2:A5000; adicionando 1 linha");
    let add_rows_res = call_mcp(
        "add_rows",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "count": 1
        }),
    )?;
    let _ = add_rows_res;

    Ok(5001)
}

fn confirm_sheet_write(row_number_1based: u32, expected_repo_id: &str) -> io::Result<bool> {
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let range = format!("A{}:A{}", row_number_1based, row_number_1based);
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )?;
    let values = extract_values_2d(&result).unwrap_or_default();
    let cell = values
        .get(0)
        .and_then(|r| r.get(0))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    Ok(cell == expected_repo_id)
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

    let started = Instant::now();
    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let repo_id = parse_repo_id_from_args();
    info!(repo_id = %repo_id, "E2E: iniciando Fases 3 e 4 (munição real)");

    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path).map_err(|e| {
        io::Error::other(format!("Falha ao abrir vault em {}: {}", db_path.display(), e))
    })?;

    let (lens_a, lens_b, lens_c) = fetch_debates(&conn, &repo_id)?;
    let (lote_id, repo_url) = fetch_repo_core(&conn, &repo_id).unwrap_or_else(|_| {
        (
            "LOTE_E2E".to_string(),
            format!("https://github.com/{}", repo_id),
        )
    });

    let seed = try_fetch_repo_heuristics_seed(&conn, &repo_id);
    let (repo_version, ultima_versao_online, licenca, stack_base, declared_description) = seed.unwrap_or_else(|| {
        (
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
        )
    });

    let now = now_epoch_secs()?;
    let block0 = Block0Context {
        project_name: repo_id.clone(),
        repo_url,
        repo_version,
        ultima_versao_online,
        lote_id,
        data_ultima_analise: now,
        analise_origem: "SODA_E2E_PHASE3_4".to_string(),
        licenca,
        stack_base,
        declared_description,
        lente_a_sentido_prod_ux: lens_a,
        lente_b_estrutura_arq: lens_b,
        lente_c_realidade_ops: lens_c,
    };

    let formatter = OpenRouterFormatterClient::from_env().map_err(io::Error::other)?;
    let cfg = Phase3Config {
        model: OFFICIAL_FORMATTER_MODEL.to_string(),
        max_attempts_per_block: 3,
    };

    let row_number = resolve_master_solutions_row_number(&repo_id)?;
    info!(row_number, "E2E: row_number resolvido na MASTER_SOLUTIONS");

    let phase3_out = match run_phase3_sgr(&formatter, &cfg, block0).await {
        Ok(out) => out,
        Err(Phase3Error::RetryExhausted { block, attempts, message }) => {
            error!(block, attempts, message = %message, "E2E: falha terminal no SGR após retries");
            return Err(io::Error::other("Falha terminal no SGR"));
        }
        Err(e) => {
            error!(error = %e, "E2E: falha no SGR");
            return Err(io::Error::other(format!("Falha SGR: {}", e)));
        }
    };

    info!("E2E: Fase 3 concluída. Iniciando Fase 4 (carga atômica Sheets)");
    SsotInjector::inject_ssot(&repo_id, phase3_out.row, row_number, now)
        .await
        .map_err(|e| io::Error::other(format!("Falha na Fase 4 (SSOT Injector): {}", e)))?;

    let confirmed = confirm_sheet_write(row_number, &repo_id)?;
    if !confirmed {
        return Err(io::Error::other(
            "E2E: atualização enviada, mas confirmação via leitura não bateu",
        ));
    }

    let usage = formatter.usage_totals();
    let elapsed_ms = started.elapsed().as_millis();
    info!(
        elapsed_ms,
        prompt_tokens = usage.prompt_tokens,
        completion_tokens = usage.completion_tokens,
        total_tokens = usage.total_tokens,
        total_cost_usd = usage.total_cost_usd,
        "E2E: concluído com confirmação de escrita no Sheets"
    );

    let report_name = format!("_E2E_REPORT_{}_PHASE4.txt", sanitize_repo_id(&repo_id));
    let report_path = root_dir.join(report_name);
    let report = format!(
        "repo_id={}\nrow_number={}\nmodel_used={}\nlatency_total_ms={}\nprompt_tokens={}\ncompletion_tokens={}\ntotal_tokens={}\ntotal_cost_usd={:.6}\nsheets_write_confirmed={}\n",
        repo_id,
        row_number,
        OFFICIAL_FORMATTER_MODEL,
        elapsed_ms,
        usage.prompt_tokens,
        usage.completion_tokens,
        usage.total_tokens,
        usage.total_cost_usd,
        confirmed
    );
    std::fs::write(&report_path, report).map_err(|e| {
        io::Error::other(format!(
            "Falha ao gravar relatório E2E em {}: {}",
            report_path.display(),
            e
        ))
    })?;

    info!(report = %report_path.display(), "E2E: relatório gravado");
    Ok(())
}
