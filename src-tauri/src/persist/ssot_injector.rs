use crate::cognition::synthesizer::{
    apply_phase4_block5, master_solutions_header_range, sheet_range_for_row, MasterSolutionsRow,
    MASTER_SOLUTIONS_CANONICAL_COLUMNS,
};
use thiserror::Error;
use serde_json::{json, Value};
use rusqlite::{Connection, ErrorCode};
use std::collections::{HashMap, HashSet};
use std::env;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use url::Url;
use tracing::info;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SsotError {
    #[error("Falha na validacao do payload SSOT: {0}")]
    ValidationFailure(String),
    #[error("Falha na persistência L2 (SQLite): {0}")]
    L2Failure(String),
    #[error("Falha no despacho para a nuvem (Sheets): {0}")]
    CloudFailure(String),
    #[error("Config ausente: {0}")]
    ConfigMissing(&'static str),
    #[error("Falha de rede/MCP: {0}")]
    NetworkFailure(String),
}

pub struct SsotInjector;

const SSOT_EXPECTED_COLUMNS: usize = 85;
const MASTER_SOLUTIONS_SHEET: &str = "MASTER_SOLUTIONS";

#[cfg(not(test))]
const MCP_TIMEOUT: Duration = Duration::from_secs(180);
#[cfg(test)]
const MCP_TIMEOUT: Duration = Duration::from_millis(250);

pub type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
pub type SheetsFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SqliteRetryPolicy {
    max_attempts: usize,
    base_delay_ms: u64,
    max_delay_ms: u64,
    jitter_ms: u64,
}

pub trait SheetsClient: Send + Sync {
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
        ranges: Value,
    ) -> SheetsFuture<'a>;
}

pub struct McpGoogleSheetsClient;

impl SheetsClient for McpGoogleSheetsClient {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a> {
        Box::pin(async move {
            use std::process::Stdio;
            use tokio::io::AsyncWriteExt;
            use tokio::process::Command;

            let creds = env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .map_err(|_| "Missing GOOGLE_APPLICATION_CREDENTIALS".to_string())?;

            let init_req = json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "soda-injector", "version": "1.0.0" }
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
                    "name": "get_sheet_data",
                    "arguments": {
                        "spreadsheet_id": spreadsheet_id,
                        "sheet": sheet,
                        "range": range,
                        "include_grid_data": false
                    }
                }
            });

            let mut child = Command::new("mcp-google-sheets")
                .env("GOOGLE_APPLICATION_CREDENTIALS", &creds)
                .env("UV_NO_PROGRESS", "1")
                .env("UV_QUIET", "1")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Falha ao spawnar mcp-google-sheets: {}", e))?;

            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(format!("{}\n", init_req).as_bytes()).await;
                let _ = stdin
                    .write_all(format!("{}\n", initialized_notif).as_bytes())
                    .await;
                let _ = stdin.write_all(format!("{}\n", mcp_request).as_bytes()).await;
            }

            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| "stdout indisponível".to_string())?;
            let mut stderr = child
                .stderr
                .take()
                .ok_or_else(|| "stderr indisponível".to_string())?;
            let (status, stdout_buf, stderr_buf) =
                match tokio::time::timeout(MCP_TIMEOUT, async {
                    use tokio::io::AsyncReadExt;
                    let mut out_buf = Vec::new();
                    let mut err_buf = Vec::new();
                    let (out_res, err_res, status_res) = tokio::join!(
                        stdout.read_to_end(&mut out_buf),
                        stderr.read_to_end(&mut err_buf),
                        child.wait()
                    );
                    out_res.map_err(|e| format!("Falha ao ler stdout MCP: {}", e))?;
                    err_res.map_err(|e| format!("Falha ao ler stderr MCP: {}", e))?;
                    let status = status_res.map_err(|e| format!("Falha ao aguardar processo MCP: {}", e))?;
                    Ok::<_, String>((status, out_buf, err_buf))
                })
                .await
                {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        let _ = child.kill().await;
                        return Err(format!(
                            "Timeout aguardando mcp-google-sheets (get_sheet_data) timeout_s={}",
                            MCP_TIMEOUT.as_secs()
                        ));
                    }
                };
            let stdout_str = String::from_utf8_lossy(&stdout_buf);
            let stderr_str = String::from_utf8_lossy(&stderr_buf);
            if !status.success() {
                return Err(format!(
                    "MCP get_sheet_data falhou: status={} stderr={}",
                    status, stderr_str
                ));
            }

            let mut last_json: Option<Value> = None;
            for line in stdout_str.lines() {
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    last_json = Some(v);
                }
            }
            let Some(msg) = last_json else {
                return Err("Resposta MCP inválida (stdout vazio)".to_string());
            };
            let values = SsotInjector::extract_values_2d(&msg).unwrap_or_default();
            Ok(values)
        })
    }

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a> {
        Box::pin(async move {
            use std::process::Stdio;
            use tokio::io::AsyncWriteExt;
            use tokio::process::Command;

            let creds = env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .map_err(|_| "Missing GOOGLE_APPLICATION_CREDENTIALS".to_string())?;

            let init_req = json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "soda-injector", "version": "1.0.0" }
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
                    "name": "batch_update_cells",
                    "arguments": {
                        "spreadsheet_id": spreadsheet_id,
                        "sheet": sheet,
                        "ranges": ranges
                    }
                }
            });

            let mut child = Command::new("mcp-google-sheets")
                .env("GOOGLE_APPLICATION_CREDENTIALS", &creds)
                .env("UV_NO_PROGRESS", "1")
                .env("UV_QUIET", "1")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Falha ao spawnar mcp-google-sheets: {}", e))?;

            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(format!("{}\n", init_req).as_bytes()).await;
                let _ = stdin
                    .write_all(format!("{}\n", initialized_notif).as_bytes())
                    .await;
                let _ = stdin.write_all(format!("{}\n", mcp_request).as_bytes()).await;
            }

            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| "stdout indisponível".to_string())?;
            let mut stderr = child
                .stderr
                .take()
                .ok_or_else(|| "stderr indisponível".to_string())?;
            let (status, stdout_buf, stderr_buf) =
                match tokio::time::timeout(MCP_TIMEOUT, async {
                    use tokio::io::AsyncReadExt;
                    let mut out_buf = Vec::new();
                    let mut err_buf = Vec::new();
                    let (out_res, err_res, status_res) = tokio::join!(
                        stdout.read_to_end(&mut out_buf),
                        stderr.read_to_end(&mut err_buf),
                        child.wait()
                    );
                    out_res.map_err(|e| format!("Falha ao ler stdout MCP: {}", e))?;
                    err_res.map_err(|e| format!("Falha ao ler stderr MCP: {}", e))?;
                    let status = status_res.map_err(|e| format!("Falha ao aguardar processo MCP: {}", e))?;
                    Ok::<_, String>((status, out_buf, err_buf))
                })
                .await
                {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        let _ = child.kill().await;
                        return Err(format!(
                            "Timeout aguardando mcp-google-sheets (batch_update_cells) timeout_s={}",
                            MCP_TIMEOUT.as_secs()
                        ));
                    }
                };
            let stdout_str = String::from_utf8_lossy(&stdout_buf);
            let stderr_str = String::from_utf8_lossy(&stderr_buf);

            if stdout_str.contains("\"isError\":true") || stdout_str.contains("\"error\":") {
                return Err(format!("MCP Retornou Erro: {}", stdout_str));
            }

            if !status.success() {
                return Err(format!(
                    "Falha no processo MCP. Exit {}. STDERR: {}",
                    status, stderr_str
                ));
            }

            Ok(())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedSsotFields {
    project_name: String,
    repo_url: String,
    repo_analised_version: String,
    ultima_versao_online: String,
    lote_id: String,
    data_ultima_analise: i64,
    analise_origem: String,
    declared_description: String,
    proposta_original_resumo: String,
    stack_base: String,
    licenca: String,
}

impl SsotInjector {
    fn should_short_circuit(status_atualizacao: &str) -> bool {
        status_atualizacao.trim().starts_with("REJEITADO_")
    }

    fn status_fase_to_persist<'a>(status_atualizacao: &str, status_fase: &'a str) -> &'a str {
        if Self::should_short_circuit(status_atualizacao) {
            "SHORT-CIRCUIT"
        } else {
            status_fase
        }
    }

    fn retry_sqlite_busy<T, F>(
        policy: SqliteRetryPolicy,
        mut op: F,
    ) -> Result<T, rusqlite::Error>
    where
        F: FnMut() -> Result<T, rusqlite::Error>,
    {
        let attempts = policy.max_attempts.max(1);
        let mut delay_ms = policy.base_delay_ms.max(1);
        let max_delay_ms = policy.max_delay_ms.max(delay_ms);

        for attempt in 1..=attempts {
            match op() {
                Ok(value) => return Ok(value),
                Err(err) => {
                    let is_busy = match &err {
                        rusqlite::Error::SqliteFailure(ffi_err, _) => matches!(
                            ffi_err.code,
                            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                        ),
                        _ => false,
                    };
                    if !is_busy || attempt == attempts {
                        return Err(err);
                    }

                    let jitter = if policy.jitter_ms > 0 {
                        rand::random::<u64>() % (policy.jitter_ms + 1)
                    } else {
                        0
                    };
                    let sleep_ms = (delay_ms.saturating_add(jitter)).min(max_delay_ms);
                    std::thread::sleep(Duration::from_millis(sleep_ms));
                    delay_ms = (delay_ms.saturating_mul(2)).min(max_delay_ms);
                }
            }
        }

        Err(rusqlite::Error::InvalidQuery)
    }

    fn cleanup_artefatos_brutos_for_repo_id(
        conn: &Connection,
        repo_id: &str,
        policy: SqliteRetryPolicy,
    ) -> Result<usize, String> {
        let out = Self::retry_sqlite_busy(policy, || {
            conn.execute("DELETE FROM artefatos_brutos WHERE repo_id = ?1", [repo_id])
        });
        match out {
            Ok(deleted) => Ok(deleted),
            Err(e) => Err(e.to_string()),
        }
    }
    /// Injeta os dados no SSOT (SQLite + Google Sheets Batch)
    pub async fn inject_ssot(
        repo_id: &str,
        mut row: MasterSolutionsRow,
        block3_justifications: HashMap<String, String>,
        now_epoch: i64,
    ) -> Result<u32, SsotError> {
        let validated = Self::validate_payload(repo_id, &row)?;

        apply_phase4_block5(now_epoch, &mut row);

        let spreadsheet_id =
            env::var("GOOGLE_SHEETS_ID").map_err(|_| SsotError::ConfigMissing("GOOGLE_SHEETS_ID"))?;
        let sheet = "MASTER_SOLUTIONS";
        let row_number_1based =
            Self::resolve_row_number_by_repo_url(
                &spreadsheet_id,
                sheet,
                &validated.repo_url,
            )
            .await?;

        let client = McpGoogleSheetsClient;
        let header_row = Self::load_master_solutions_header(&client, &spreadsheet_id).await?;
        let batch_payload =
            Self::prepare_batch_payload_dynamic(row_number_1based, &header_row, &row)?;

        match Self::dispatch_to_cloud(batch_payload).await {
            Ok(()) => {
                row.status_fase = "FASE_4_SHEETS_UPDATED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Ok(row_number_1based)
            }
            Err(err) => {
                row.status_fase = "FASE_4_CLOUD_FAILED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Err(err)
            }
        }
    }

    pub async fn inject_ssot_with_skip_columns(
        repo_id: &str,
        mut row: MasterSolutionsRow,
        block3_justifications: HashMap<String, String>,
        now_epoch: i64,
        skip_columns: &[&'static str],
    ) -> Result<u32, SsotError> {
        let validated = Self::validate_payload(repo_id, &row)?;

        apply_phase4_block5(now_epoch, &mut row);

        let spreadsheet_id =
            env::var("GOOGLE_SHEETS_ID").map_err(|_| SsotError::ConfigMissing("GOOGLE_SHEETS_ID"))?;
        let sheet = "MASTER_SOLUTIONS";
        let row_number_1based =
            Self::resolve_row_number_by_repo_url(&spreadsheet_id, sheet, &validated.repo_url).await?;

        let client = McpGoogleSheetsClient;
        let header_row = Self::load_master_solutions_header(&client, &spreadsheet_id).await?;
        let batch_payload = Self::prepare_batch_payload_dynamic_with_skip(
            row_number_1based,
            &header_row,
            &row,
            skip_columns,
        )?;

        match Self::dispatch_to_cloud(batch_payload).await {
            Ok(()) => {
                row.status_fase = "FASE_4_SHEETS_UPDATED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Ok(row_number_1based)
            }
            Err(err) => {
                row.status_fase = "FASE_4_CLOUD_FAILED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Err(err)
            }
        }
    }

    async fn resolve_row_number_by_repo_url(
        spreadsheet_id: &str,
        sheet: &str,
        repo_url: &str,
    ) -> Result<u32, SsotError> {
        let result = Self::call_mcp_google_sheets_tool(
            "get_sheet_data",
            json!({
                "spreadsheet_id": spreadsheet_id,
                "sheet": sheet,
                "range": "D2:D",
                "include_grid_data": false
            }),
        )
        .await?;

        let values = Self::extract_values_2d(&result).unwrap_or_default();
        let needle = repo_url.trim_end_matches('/').to_ascii_lowercase();
        if let Some(found) = Self::resolve_row_number_from_repo_url_column(&values, &needle) {
            return Ok(found);
        }

        Err(SsotError::CloudFailure(format!(
            "Linha SSOT não encontrada para repo_url='{}'. Append é proibido; abortando.",
            repo_url
        )))
    }

    fn resolve_row_number_from_repo_url_column(
        values: &[Vec<String>],
        repo_url_needle: &str,
    ) -> Option<u32> {
        for (idx, row) in values.iter().enumerate() {
            let repo_cell = row.first().map(|s| s.trim()).unwrap_or("");
            let repo_hay = repo_cell.trim_end_matches('/').to_ascii_lowercase();
            if !repo_hay.is_empty() && repo_hay == repo_url_needle {
                return Some((idx as u32) + 2);
            }
        }
        None
    }

    async fn call_mcp_google_sheets_tool(
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, SsotError> {
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        let creds = env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| SsotError::ConfigMissing("GOOGLE_APPLICATION_CREDENTIALS"))?;

        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": { "name": "genesis-mc", "version": "1.0" },
                "capabilities": {}
            }
        });
        let initialized_notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let mcp_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": tool_name, "arguments": arguments }
        });

        let mut child = Command::new("mcp-google-sheets")
            .env("GOOGLE_APPLICATION_CREDENTIALS", &creds)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SsotError::NetworkFailure(format!("Falha ao spawnar mcp-google-sheets: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(format!("{}\n", init_req).as_bytes())
                .await
                .map_err(|e| SsotError::NetworkFailure(format!("Falha ao escrever init no MCP: {}", e)))?;
            stdin
                .write_all(format!("{}\n", initialized_notif).as_bytes())
                .await
                .map_err(|e| SsotError::NetworkFailure(format!("Falha ao escrever initialized no MCP: {}", e)))?;
            stdin
                .write_all(format!("{}\n", mcp_request).as_bytes())
                .await
                .map_err(|e| SsotError::NetworkFailure(format!("Falha ao escrever tools/call no MCP: {}", e)))?;
        }

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| SsotError::NetworkFailure("stdout indisponível".to_string()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| SsotError::NetworkFailure("stderr indisponível".to_string()))?;
        let (status, stdout_buf, stderr_buf) =
            match tokio::time::timeout(MCP_TIMEOUT, async {
                use tokio::io::AsyncReadExt;
                let mut out_buf = Vec::new();
                let mut err_buf = Vec::new();
                let (out_res, err_res, status_res) = tokio::join!(
                    stdout.read_to_end(&mut out_buf),
                    stderr.read_to_end(&mut err_buf),
                    child.wait()
                );
                out_res.map_err(|e| SsotError::NetworkFailure(format!("Falha ao ler stdout MCP: {}", e)))?;
                err_res.map_err(|e| SsotError::NetworkFailure(format!("Falha ao ler stderr MCP: {}", e)))?;
                let status =
                    status_res.map_err(|e| SsotError::NetworkFailure(format!("Falha ao aguardar processo MCP: {}", e)))?;
                Ok::<_, SsotError>((status, out_buf, err_buf))
            })
            .await
            {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    let _ = child.kill().await;
                    return Err(SsotError::NetworkFailure(format!(
                        "Timeout aguardando mcp-google-sheets tool={} timeout_s={}",
                        tool_name,
                        MCP_TIMEOUT.as_secs()
                    )));
                }
            };
        if !status.success() {
            return Err(SsotError::NetworkFailure(format!(
                "Falha no processo MCP. Exit {}. STDERR: {}",
                status,
                String::from_utf8_lossy(&stderr_buf)
            )));
        }

        let stdout_str = String::from_utf8_lossy(&stdout_buf);
        Self::parse_mcp_tool_stdout(&stdout_str, 2).map_err(SsotError::NetworkFailure)
    }

    fn parse_mcp_tool_stdout(stdout: &str, expected_id: i64) -> Result<Value, String> {
        for line in stdout.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let id = value.get("id").and_then(|v| v.as_i64());
            if id != Some(expected_id) {
                continue;
            }
            if let Some(err) = value.get("error") {
                return Err(format!("MCP error: {}", err));
            }
            let Some(result) = value.get("result") else {
                return Err("MCP missing result".to_string());
            };
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                let mut combined = String::new();
                for part in content {
                    if part.get("type").and_then(|t| t.as_str()) != Some("text") {
                        continue;
                    }
                    let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    if !text.trim().is_empty() {
                        if !combined.is_empty() {
                            combined.push('\n');
                        }
                        combined.push_str(text);
                    }
                }
                if !combined.trim().is_empty() {
                    if let Ok(json_val) = serde_json::from_str::<Value>(&combined) {
                        return Ok(json_val);
                    }
                    return Ok(json!({ "text": combined }));
                }
            }
            return Ok(result.clone());
        }
        Err("MCP tool response not found in stdout".to_string())
    }

    fn extract_values_2d(value: &Value) -> Option<Vec<Vec<String>>> {
        if let Some(values) = value.get("values").and_then(|v| v.as_array()) {
            return Some(Self::parse_values_array(values));
        }
        if let Some(vrs) = value.get("valueRanges").and_then(|v| v.as_array()) {
            if let Some(first) = vrs.first() {
                if let Some(values) = first.get("values").and_then(|v| v.as_array()) {
                    return Some(Self::parse_values_array(values));
                }
            }
        }
        if let Some(result) = value.get("result") {
            return Self::extract_values_2d(result);
        }
        if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                return Self::extract_values_2d(&parsed);
            }
        }
        None
    }

    fn parse_values_array(values: &[Value]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|row| {
                row.as_array()
                    .map(|r| {
                        r.iter()
                            .map(|cell| cell.as_str().unwrap_or("").to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
    }

    fn validate_payload(
        repo_id: &str,
        payload: &MasterSolutionsRow,
    ) -> Result<ValidatedSsotFields, SsotError> {
        if payload.categoria_arquitetural == crate::cognition::synthesizer::ArchitecturalCategory::Unknown {
            return Err(SsotError::ValidationFailure(
                "categoria_arquitetural invalida (fora do catálogo de 10 ENUMs)".to_string(),
            ));
        }
        let _ = crate::cognition::synthesizer::ArchitecturalCategory::parse_strict(
            payload.categoria_arquitetural.as_str(),
        )
        .map_err(SsotError::ValidationFailure)?;

        let project_name = Self::require_non_empty("project_name", &payload.project_name)?;
        if project_name != repo_id {
            return Err(SsotError::ValidationFailure(format!(
                "project_name divergente do repo_id: esperado '{}', recebido '{}'",
                repo_id, project_name
            )));
        }

        let repo_url = Self::require_non_empty("repo_url", &payload.repo_url)?;
        let parsed_repo_url = Url::parse(&repo_url)
            .map_err(|e| SsotError::ValidationFailure(format!("repo_url invalido: {}", e)))?;
        let expected_repo_url = format!("https://github.com/{}", repo_id);
        if parsed_repo_url.as_str().trim_end_matches('/') != expected_repo_url {
            return Err(SsotError::ValidationFailure(format!(
                "repo_url divergente do repo_id: esperado '{}', recebido '{}'",
                expected_repo_url, parsed_repo_url
            )));
        }

        let repo_analised_version = Self::require_non_empty(
            "repo_analised_version",
            &payload.repo_analised_version,
        )?;
        let repo_analised_version_lower = repo_analised_version.to_ascii_lowercase();
        if repo_analised_version_lower == "main" || repo_analised_version_lower == "master" {
            return Err(SsotError::ValidationFailure(format!(
                "repo_analised_version invalido (branch): '{}'",
                repo_analised_version
            )));
        }
        let ultima_versao_online =
            Self::require_non_empty("ultima_versao_online", &payload.ultima_versao_online)?;
        let lote_id = Self::require_non_empty("lote_id", &payload.lote_id)?;
        let data_ultima_analise = if payload.data_ultima_analise > 0 {
            payload.data_ultima_analise
        } else {
            return Err(SsotError::ValidationFailure(
                "data_ultima_analise ausente ou invalida".to_string(),
            ));
        };
        let analise_origem = Self::require_non_empty("analise_origem", &payload.analise_origem)?;
        let declared_description =
            Self::require_non_empty("declared_description", &payload.declared_description)?;
        let proposta_original_resumo = Self::require_non_empty(
            "proposta_original_resumo",
            &payload.proposta_original_resumo,
        )?;
        let stack_base = Self::require_non_empty("stack_base", &payload.stack_base)?;
        let licenca = Self::require_non_empty("licenca", &payload.licenca)?;

        Ok(ValidatedSsotFields {
            project_name,
            repo_url,
            repo_analised_version,
            ultima_versao_online,
            lote_id,
            data_ultima_analise,
            analise_origem,
            declared_description,
            proposta_original_resumo,
            stack_base,
            licenca,
        })
    }

    fn require_non_empty(field: &str, value: &str) -> Result<String, SsotError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(SsotError::ValidationFailure(format!(
                "campo obrigatorio '{}' ausente ou vazio",
                field
            )));
        }

        Ok(trimmed.to_string())
    }

    fn update_local_status(
        repo_id: &str,
        status_value: &str,
        payload: &MasterSolutionsRow,
        validated: &ValidatedSsotFields,
        block3_justifications: &HashMap<String, String>,
        now_epoch: i64,
    ) -> Result<(), String> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let root_dir = std::path::Path::new(manifest_dir).parent().unwrap_or_else(|| std::path::Path::new("."));
        let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
        
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Falha ao conectar no SQLite: {}", e))?;

        Self::ensure_repo_heuristics_schema(&conn)?;
        Self::ensure_repo_heuristics_justifications_schema(&conn)?;

        let status_fase_to_persist =
            Self::status_fase_to_persist(&payload.status_atualizacao, &payload.status_fase)
                .to_string();

        let repo_version_to_persist = {
            let primary = payload.repo_analised_version.trim();
            if !primary.is_empty() {
                primary.to_string()
            } else {
                let fallback = payload.ultima_versao_online.trim();
                if !fallback.is_empty() {
                    fallback.to_string()
                } else {
                    "unknown".to_string()
                }
            }
        };

        // I/O L2 Real: Mapeando SgrPayload para as colunas reais da tabela
        conn.execute(
            "INSERT OR REPLACE INTO repo_heuristics (
                project_name, status_atualizacao, status_fase, repo_url, repo_analised_version, repo_version, ultima_versao_online, lote_id, data_ultima_analise, analise_origem, declared_description, proposta_original_resumo, stack_base, licenca, lente_a_sentido_prod_ux, lente_b_estrutura_arq, lente_c_realidade_ops, visao_do_enxame, justificativa_decisao, executive_verdict, classificacao_terminal, acao_de_canibalizacao, categoria_arquitetural, horizonte_extracao, tipo_integracao, categoria_nuance_tecnica, integracao_papel_exato, ouro_a_extrair, deep_pattern, transplantable_core, logic_math_heuristic, real_structural_problem, must_components_prod_ux, must_components_arq, must_components_ops, detected_toxic_deps, do_not_absorb, where_ai_should_not_enter, bare_metal_fit, extractability_level, operability_level, entropy_risk, design_misuse_risk, intrinsic_ethics_risk, discipline_dependency, risco_principal, risco_linha_vermelha, observacoes, score_final, score_fit_geral_soda, score_philosophical_fit, score_bare_metal_fit, score_architectural_extractability, score_operability, score_creep_risk, score_runtime_sovereignty, score_model_logic_value, score_ethics_safety, score_intrinsic_risk, capability_nature_primary, architectural_topology, runtime_sovereignty_fit, local_first_fit, temporal_stability, adoptability_level, longitudinal_sustainability, abandonment_risk, maintenance_burden, onboarding_friction, observability_operational, recoverability_level, degradation_behavior, curation_burden, time_to_first_clear_value, imperfection_tolerance, evolution_cost, regulatory_risk, score_architectural_priority, score_human_product_priority, score_absorption_readiness, score_operational_priority, score_sustainability_adjusted_fit, valid_from, valid_to, embargo_status, indicacao_otimista_canibalizacao
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, ?81, ?82, ?83, ?84, ?85, ?86
            )",
            rusqlite::params![
                &validated.project_name,
                &payload.status_atualizacao,
                &status_fase_to_persist,
                &validated.repo_url,
                &validated.repo_analised_version,
                &repo_version_to_persist,
                &payload.ultima_versao_online,
                &payload.lote_id,
                payload.data_ultima_analise,
                &payload.analise_origem,
                &payload.declared_description,
                &payload.proposta_original_resumo,
                &payload.stack_base,
                &payload.licenca,
                &payload.lente_a_sentido_prod_ux,
                &payload.lente_b_estrutura_arq,
                &payload.lente_c_realidade_ops,
                &payload.visao_do_enxame,
                &payload.justificativa_decisao,
                &payload.executive_verdict,
                payload.classificacao_terminal.as_str(),
                payload.acao_de_canibalizacao.as_str(),
                payload.categoria_arquitetural.as_str(),
                payload.horizonte_extracao.as_str(),
                payload.tipo_integracao.as_str(),
                &payload.categoria_nuance_tecnica,
                &payload.integracao_papel_exato,
                &payload.ouro_a_extrair,
                &payload.deep_pattern,
                &payload.transplantable_core,
                &payload.logic_math_heuristic,
                &payload.real_structural_problem,
                &payload.must_components_prod_ux,
                &payload.must_components_arq,
                &payload.must_components_ops,
                &payload.detected_toxic_deps,
                &payload.do_not_absorb,
                &payload.where_ai_should_not_enter,
                payload.bare_metal_fit.as_str(),
                payload.extractability_level.as_str(),
                payload.operability_level.as_str(),
                payload.entropy_risk.as_str(),
                payload.design_misuse_risk.as_str(),
                payload.intrinsic_ethics_risk.as_str(),
                payload.discipline_dependency.as_str(),
                &payload.risco_principal,
                &payload.risco_linha_vermelha,
                &payload.observacoes,
                payload.score_final,
                payload.score_fit_geral_soda,
                payload.score_philosophical_fit,
                payload.score_bare_metal_fit,
                payload.score_architectural_extractability,
                payload.score_operability,
                payload.score_creep_risk,
                payload.score_runtime_sovereignty,
                payload.score_model_logic_value,
                payload.score_ethics_safety,
                payload.score_intrinsic_risk,
                payload.capability_nature_primary.as_str(),
                payload.architectural_topology.as_str(),
                payload.runtime_sovereignty_fit.as_str(),
                payload.local_first_fit.as_str(),
                payload.temporal_stability.as_str(),
                payload.adoptability_level.as_str(),
                payload.longitudinal_sustainability.as_str(),
                payload.abandonment_risk.as_str(),
                payload.maintenance_burden.as_str(),
                payload.onboarding_friction.as_str(),
                payload.observability_operational.as_str(),
                payload.recoverability_level.as_str(),
                payload.degradation_behavior.as_str(),
                payload.curation_burden.as_str(),
                payload.time_to_first_clear_value.as_str(),
                payload.imperfection_tolerance.as_str(),
                payload.evolution_cost.as_str(),
                payload.regulatory_risk.as_str(),
                payload.score_architectural_priority,
                payload.score_human_product_priority,
                payload.score_absorption_readiness,
                payload.score_operational_priority,
                payload.score_sustainability_adjusted_fit,
                payload.valid_from,
                payload.valid_to,
                payload.embargo_status,
                &payload.indicacao_otimista_canibalizacao,
            ],
        ).map_err(|e| format!("Falha ao executar INSERT repo_heuristics: {}", e))?;

        if status_fase_to_persist == "SHORT-CIRCUIT" {
            let policy = SqliteRetryPolicy {
                max_attempts: 5,
                base_delay_ms: 25,
                max_delay_ms: 400,
                jitter_ms: 50,
            };
            if let Err(err) = Self::cleanup_artefatos_brutos_for_repo_id(&conn, repo_id, policy) {
                info!(repo_id = %repo_id, error = %err, "SHORT-CIRCUIT: cleanup de blobs falhou (fail-soft)");
            }
        }

        if !block3_justifications.is_empty() {
            let json_text =
                serde_json::to_string(block3_justifications).unwrap_or_else(|_| "{}".to_string());
            conn.execute(
                "INSERT INTO repo_heuristics_justifications (project_name, block, justifications_json, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(project_name, block) DO UPDATE SET
                    justifications_json = excluded.justifications_json,
                    created_at = excluded.created_at",
                rusqlite::params![repo_id, 3_i64, json_text, now_epoch],
            )
            .map_err(|e| format!("Falha ao persistir justifications do Bloco 3 em SQLite: {}", e))?;
        }

        // Atualizando o status em repositorios
        let _ = conn.execute(
            "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
            rusqlite::params![status_value, repo_id],
        ).map_err(|e| format!("Falha ao executar UPDATE repositorios: {}", e))?;
        
        Ok(())
    }

    fn ensure_repo_heuristics_schema(conn: &Connection) -> Result<(), String> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repo_heuristics (
                project_name TEXT PRIMARY KEY,
                status_atualizacao TEXT NOT NULL,
                status_fase TEXT NOT NULL,
                repo_url TEXT NOT NULL,
                repo_analised_version TEXT NOT NULL,
                repo_version TEXT,
                ultima_versao_online TEXT,
                indicacao_otimista_canibalizacao TEXT NOT NULL DEFAULT '',
                lote_id TEXT NOT NULL,
                data_ultima_analise INTEGER NOT NULL,
                analise_origem TEXT NOT NULL,
                declared_description TEXT NOT NULL,
                proposta_original_resumo TEXT NOT NULL,
                stack_base TEXT NOT NULL,
                licenca TEXT,
                lente_a_sentido_prod_ux TEXT,
                lente_b_estrutura_arq TEXT,
                lente_c_realidade_ops TEXT,
                visao_do_enxame TEXT NOT NULL,
                justificativa_decisao TEXT NOT NULL,
                executive_verdict TEXT NOT NULL,
                classificacao_terminal TEXT NOT NULL,
                acao_de_canibalizacao TEXT NOT NULL,
                categoria_arquitetural TEXT NOT NULL,
                horizonte_extracao TEXT NOT NULL,
                tipo_integracao TEXT NOT NULL,
                categoria_nuance_tecnica TEXT NOT NULL,
                integracao_papel_exato TEXT NOT NULL,
                ouro_a_extrair TEXT NOT NULL,
                deep_pattern TEXT NOT NULL,
                transplantable_core TEXT NOT NULL,
                logic_math_heuristic TEXT NOT NULL,
                real_structural_problem TEXT NOT NULL,
                must_components_prod_ux TEXT NOT NULL,
                must_components_arq TEXT NOT NULL,
                must_components_ops TEXT NOT NULL,
                detected_toxic_deps TEXT NOT NULL,
                do_not_absorb TEXT NOT NULL,
                where_ai_should_not_enter TEXT NOT NULL,
                bare_metal_fit TEXT NOT NULL,
                extractability_level TEXT NOT NULL,
                operability_level TEXT NOT NULL,
                entropy_risk TEXT NOT NULL,
                design_misuse_risk TEXT NOT NULL,
                intrinsic_ethics_risk TEXT NOT NULL,
                discipline_dependency TEXT NOT NULL,
                risco_principal TEXT NOT NULL,
                risco_linha_vermelha TEXT NOT NULL,
                observacoes TEXT NOT NULL,
                score_final REAL NOT NULL,
                score_fit_geral_soda REAL NOT NULL,
                score_philosophical_fit INTEGER NOT NULL,
                score_bare_metal_fit INTEGER NOT NULL,
                score_architectural_extractability INTEGER NOT NULL,
                score_operability INTEGER NOT NULL,
                score_creep_risk INTEGER NOT NULL,
                score_runtime_sovereignty INTEGER NOT NULL,
                score_model_logic_value INTEGER NOT NULL,
                score_ethics_safety INTEGER NOT NULL,
                score_intrinsic_risk INTEGER NOT NULL,
                capability_nature_primary TEXT NOT NULL,
                architectural_topology TEXT NOT NULL,
                runtime_sovereignty_fit TEXT NOT NULL,
                local_first_fit TEXT NOT NULL,
                temporal_stability TEXT NOT NULL,
                adoptability_level TEXT NOT NULL,
                longitudinal_sustainability TEXT NOT NULL,
                abandonment_risk TEXT NOT NULL,
                maintenance_burden TEXT NOT NULL,
                onboarding_friction TEXT NOT NULL,
                observability_operational TEXT NOT NULL,
                recoverability_level TEXT NOT NULL,
                degradation_behavior TEXT NOT NULL,
                curation_burden TEXT NOT NULL,
                time_to_first_clear_value TEXT NOT NULL,
                imperfection_tolerance TEXT NOT NULL,
                evolution_cost TEXT NOT NULL,
                regulatory_risk TEXT NOT NULL,
                score_architectural_priority REAL NOT NULL,
                score_human_product_priority REAL NOT NULL,
                score_absorption_readiness REAL NOT NULL,
                score_operational_priority REAL NOT NULL,
                score_sustainability_adjusted_fit REAL NOT NULL,
                valid_from INTEGER NOT NULL,
                valid_to INTEGER,
                embargo_status INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Falha ao criar tabela repo_heuristics: {}", e))?;

        let mut stmt = conn
            .prepare("PRAGMA table_info('repo_heuristics')")
            .map_err(|e| format!("Falha ao preparar PRAGMA table_info(repo_heuristics): {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("Falha ao executar PRAGMA table_info(repo_heuristics): {e}"))?;

        let mut has_status_atualizacao = false;
        let mut has_status_fase = false;
        let mut has_repo_analised_version = false;
        let mut has_repo_version = false;
        let mut has_indicacao_otimista = false;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Falha ao iterar PRAGMA table_info(repo_heuristics): {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("Falha ao ler coluna name do PRAGMA table_info: {e}"))?;
            match name.as_str() {
                "status_atualizacao" => has_status_atualizacao = true,
                "status_fase" => has_status_fase = true,
                "repo_analised_version" => has_repo_analised_version = true,
                "repo_version" => has_repo_version = true,
                "indicacao_otimista_canibalizacao" => has_indicacao_otimista = true,
                _ => {}
            }
        }

        if !has_status_atualizacao {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN status_atualizacao TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna status_atualizacao: {e}"))?;
        }
        if !has_status_fase {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN status_fase TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna status_fase: {e}"))?;
        }
        if !has_repo_analised_version {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN repo_analised_version TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna repo_analised_version: {e}"))?;
        }
        if !has_repo_version {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN repo_version TEXT",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna repo_version (legado): {e}"))?;
        }
        if !has_indicacao_otimista {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN indicacao_otimista_canibalizacao TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna indicacao_otimista_canibalizacao: {e}"))?;
        }

        let _ = conn.execute(
            "UPDATE repo_heuristics
             SET repo_analised_version = repo_version
             WHERE (repo_analised_version IS NULL OR repo_analised_version = '')
               AND repo_version IS NOT NULL
               AND repo_version != ''",
            [],
        );

        Ok(())
    }

    fn ensure_repo_heuristics_justifications_schema(conn: &Connection) -> Result<(), String> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repo_heuristics_justifications (
                project_name TEXT NOT NULL,
                block INTEGER NOT NULL,
                justifications_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_name, block)
            )",
            [],
        )
        .map_err(|e| format!("Falha ao criar tabela repo_heuristics_justifications: {}", e))?;
        Ok(())
    }

    fn normalize_header_cell(raw: &str) -> String {
        raw.trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "_")
    }

    fn col_idx_to_a1(idx_0based: usize) -> String {
        let mut n = (idx_0based + 1) as u32;
        let mut s = String::new();
        while n > 0 {
            let rem = ((n - 1) % 26) as u8;
            s.insert(0, (b'A' + rem) as char);
            n = (n - 1) / 26;
        }
        s
    }

    async fn load_master_solutions_header(
        sheets: &dyn SheetsClient,
        spreadsheet_id: &str,
    ) -> Result<Vec<String>, SsotError> {
        let raw = sheets
            .get_sheet_data(
                spreadsheet_id,
                MASTER_SOLUTIONS_SHEET,
                master_solutions_header_range(),
            )
            .await
            .map_err(SsotError::CloudFailure)?;
        Ok(raw.first().cloned().unwrap_or_default())
    }

    fn validate_sheet_row_values(row_values_by_name: &HashMap<&'static str, Value>) -> Result<(), SsotError> {
        let valid_from_ok = row_values_by_name
            .get("valid_from")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !valid_from_ok {
            return Err(SsotError::ValidationFailure(
                "valid_from invalido no payload do Sheets".to_string(),
            ));
        }
        let valid_to_ok = row_values_by_name
            .get("valid_to")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().is_empty() || s.contains('-'))
            .unwrap_or(false);
        if !valid_to_ok {
            return Err(SsotError::ValidationFailure(
                "valid_to invalido no payload do Sheets".to_string(),
            ));
        }
        let embargo_ok = row_values_by_name
            .get("embargo_status")
            .and_then(|v| v.as_str())
            .map(|s| s == "LIVRE" || s == "EMBARGADO")
            .unwrap_or(false);
        if !embargo_ok {
            return Err(SsotError::ValidationFailure(
                "embargo_status invalido no payload do Sheets".to_string(),
            ));
        }
        Ok(())
    }

    fn prepare_batch_payload_dynamic(
        row_number_1based: u32,
        header_row: &[String],
        payload: &MasterSolutionsRow,
    ) -> Result<Value, SsotError> {
        Self::prepare_batch_payload_dynamic_with_skip(row_number_1based, header_row, payload, &[])
    }

    fn prepare_batch_payload_dynamic_with_skip(
        row_number_1based: u32,
        header_row: &[String],
        payload: &MasterSolutionsRow,
        skip_columns: &[&'static str],
    ) -> Result<Value, SsotError> {
        let row_values = payload.to_sheet_row();
        if row_values.len() != SSOT_EXPECTED_COLUMNS {
            return Err(SsotError::ValidationFailure(format!(
                "Payload interno desalinhado: esperado {} colunas, recebeu {}",
                SSOT_EXPECTED_COLUMNS,
                row_values.len()
            )));
        }

        let mut by_name: HashMap<&'static str, Value> = HashMap::with_capacity(SSOT_EXPECTED_COLUMNS);
        for (idx, name) in MASTER_SOLUTIONS_CANONICAL_COLUMNS.iter().enumerate() {
            by_name.insert(*name, row_values[idx].clone());
        }
        if let Some(v) = by_name.get("repo_analised_version").cloned() {
            by_name.insert("repo_version", v);
        }
        Self::validate_sheet_row_values(&by_name)?;

        let mut canonical_set: HashSet<&'static str> =
            MASTER_SOLUTIONS_CANONICAL_COLUMNS.iter().copied().collect();
        canonical_set.insert("repo_version");
        let mut skip_set: HashSet<&'static str> = HashSet::new();
        for s in skip_columns {
            skip_set.insert(*s);
        }
        let mut map = serde_json::Map::new();
        for (idx, raw) in header_row.iter().enumerate() {
            let key = Self::normalize_header_cell(raw);
            if key.is_empty() {
                continue;
            }
            if !canonical_set.contains(key.as_str()) {
                continue;
            }
            if skip_set.contains(key.as_str()) {
                continue;
            }
            let Some(value) = by_name.get(key.as_str()) else { continue };
            let col = Self::col_idx_to_a1(idx);
            let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
            map.insert(range, json!(vec![vec![value]]));
        }

        if map.is_empty() {
            return Err(SsotError::ValidationFailure(
                "HeaderResolver não encontrou colunas conhecidas para escrita".to_string(),
            ));
        }

        info!(
            ranges = map.len(),
            sheet_range = %sheet_range_for_row(row_number_1based),
            "Payload dinâmico do Google Sheets montado (late-binding por header)"
        );
        Ok(Value::Object(map))
    }

    pub async fn dispatch_master_solutions_row(
        sheets: &dyn SheetsClient,
        spreadsheet_id: &str,
        row_number_1based: u32,
        row: &MasterSolutionsRow,
    ) -> Result<(), SsotError> {
        let header_row = Self::load_master_solutions_header(sheets, spreadsheet_id).await?;
        let payload = Self::prepare_batch_payload_dynamic(row_number_1based, &header_row, row)?;
        sheets
            .batch_update_cells(spreadsheet_id, MASTER_SOLUTIONS_SHEET, payload)
            .await
            .map_err(SsotError::CloudFailure)
    }

    async fn dispatch_to_cloud(payload: Value) -> Result<(), SsotError> {
        let sheets_id = env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| SsotError::CloudFailure("Missing GOOGLE_SHEETS_ID".to_string()))?;
        let client = McpGoogleSheetsClient;
        client
            .batch_update_cells(&sheets_id, MASTER_SOLUTIONS_SHEET, payload)
            .await
            .map_err(SsotError::CloudFailure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::ffi::Error as FfiError;
    use rusqlite::{params, Error as RusqliteError, ErrorCode};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn resolve_row_number_never_returns_row_1() {
        let values = vec![vec!["https://github.com/acme/widget".to_string()]];
        let needle = "https://github.com/acme/widget".to_string();
        let row =
            SsotInjector::resolve_row_number_from_repo_url_column(&values, &needle).unwrap();
        assert!(row >= 2);
        assert_eq!(row, 2);
    }

    #[test]
    fn resolve_row_number_returns_none_when_repo_url_not_found() {
        let values = vec![
            vec!["https://github.com/acme/a".to_string()],
            vec!["".to_string()],
        ];
        let needle = "https://github.com/acme/unknown".to_string();
        assert!(SsotInjector::resolve_row_number_from_repo_url_column(&values, &needle).is_none());
    }

    #[test]
    fn test_prepare_batch_payload_is_85_columns_a_to_cg() {
        let row = MasterSolutionsRow {
            status_atualizacao: "CONCLUIDO".to_string(),
            status_fase: "F4".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SGR".to_string(),
            declared_description: "Descricao".to_string(),
            proposta_original_resumo: "Resumo".to_string(),
            stack_base: "Rust".to_string(),
            licenca: "MIT".to_string(),
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            ..Default::default()
        };

        let validated = SsotInjector::validate_payload("owner/repo", &row).unwrap();
        let header_row: Vec<String> = MASTER_SOLUTIONS_CANONICAL_COLUMNS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let batch = SsotInjector::prepare_batch_payload_dynamic(2, &header_row, &row).unwrap();
        let obj = batch.as_object().unwrap();

        assert_eq!(
            obj.get("C2:C2").unwrap(),
            &json!(vec![vec![json!("owner / repo")]])
        );
        assert_eq!(
            obj.get("D2:D2").unwrap(),
            &json!(vec![vec![json!("https://github.com/owner/repo")]])
        );
        assert!(obj
            .get("CE2:CE2")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty() && s.contains('-'))
            .unwrap_or(false));
        assert_eq!(obj.get("CF2:CF2").unwrap(), &json!(vec![vec![json!("")]]));
        assert_eq!(
            obj.get("CG2:CG2").unwrap(),
            &json!(vec![vec![json!("LIVRE")]])
        );
        assert_eq!(validated.project_name, "owner/repo");
    }

    #[test]
    fn test_payload_validation_rejects_missing_required_fields() {
        let row = MasterSolutionsRow::default();
        let result = SsotInjector::validate_payload("owner/repo", &row);
        assert!(matches!(result, Err(SsotError::ValidationFailure(_))));
    }

    #[test]
    fn categoria_arquitetural_rejects_value_outside_10_enums_fail_closed() {
        let row = MasterSolutionsRow {
            status_atualizacao: "INICIAR_TRIAGEM".to_string(),
            status_fase: "FASE_-0.5_BATEDOR_OK".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SGR".to_string(),
            declared_description: "Descricao".to_string(),
            proposta_original_resumo: "Resumo".to_string(),
            stack_base: "Rust".to_string(),
            licenca: "MIT".to_string(),
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            categoria_arquitetural: crate::cognition::synthesizer::ArchitecturalCategory::Unknown,
            ..Default::default()
        };

        let out = SsotInjector::validate_payload("owner/repo", &row);
        assert!(matches!(out, Err(SsotError::ValidationFailure(_))));
    }

    #[test]
    fn rejected_lixo_toxico_triggers_systemic_short_circuit_status_fase() {
        let out = SsotInjector::status_fase_to_persist(
            "REJEITADO_LIXO_TOXICO",
            "FASE_0_HARVESTER_OK",
        );
        assert_eq!(out, "SHORT-CIRCUIT");
    }

    fn busy_err() -> RusqliteError {
        RusqliteError::SqliteFailure(
            FfiError {
                code: ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            Some("SQLITE_BUSY".to_string()),
        )
    }

    fn setup_artefatos_brutos_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE artefatos_brutos (
                artifact_id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                payload_blob BLOB NOT NULL,
                timestamp_extracao INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn short_circuit_cleanup_deletes_only_target_repo_blobs() {
        let conn = setup_artefatos_brutos_db();
        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
            params!["a/b", "blob_01", vec![1_u8], 1_i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
            params!["a/b", "blob_02", vec![2_u8], 2_i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
            params!["c/d", "blob_01", vec![3_u8], 3_i64],
        )
        .unwrap();

        let policy = SqliteRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 10,
            max_delay_ms: 50,
            jitter_ms: 10,
        };
        let out = SsotInjector::cleanup_artefatos_brutos_for_repo_id(&conn, "a/b", policy);
        assert!(out.is_ok());

        let remaining_a: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM artefatos_brutos WHERE repo_id = ?1",
                params!["a/b"],
                |row| row.get(0),
            )
            .unwrap();
        let remaining_c: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM artefatos_brutos WHERE repo_id = ?1",
                params!["c/d"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_a, 0);
        assert_eq!(remaining_c, 1);
    }

    #[test]
    fn sqlite_busy_retry_applies_backoff_and_finishes_successfully() {
        let policy = SqliteRetryPolicy {
            max_attempts: 5,
            base_delay_ms: 1,
            max_delay_ms: 5,
            jitter_ms: 1,
        };
        let mut calls = 0usize;
        let res = SsotInjector::retry_sqlite_busy(policy, || {
            calls += 1;
            if calls < 3 {
                Err(busy_err())
            } else {
                Ok(42usize)
            }
        });
        assert!(res.is_ok());
        assert_eq!(calls, 3);
    }

    #[test]
    fn sqlite_busy_retry_aborts_without_infinite_loop() {
        let policy = SqliteRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 1,
            max_delay_ms: 5,
            jitter_ms: 1,
        };
        let mut calls = 0usize;
        let res: Result<usize, _> = SsotInjector::retry_sqlite_busy(policy, || {
            calls += 1;
            Err(busy_err())
        });
        assert!(res.is_err());
        assert_eq!(calls, 3);
    }

    struct MockSheetsClient {
        calls: Arc<Mutex<Vec<(String, String, Value)>>>,
        header: Vec<String>,
    }

    impl MockSheetsClient {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                header: MASTER_SOLUTIONS_CANONICAL_COLUMNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            }
        }
    }

    impl SheetsClient for MockSheetsClient {
        fn get_sheet_data<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            range: String,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>> {
            Box::pin(async move {
                if range.contains("1:") {
                    Ok(vec![self.header.clone()])
                } else {
                    Ok(Vec::new())
                }
            })
        }

        fn batch_update_cells<'a>(
            &'a self,
            spreadsheet_id: &'a str,
            sheet: &'a str,
            ranges: Value,
        ) -> SheetsFuture<'a> {
            Box::pin(async move {
                self.calls.lock().await.push((
                    spreadsheet_id.to_string(),
                    sheet.to_string(),
                    ranges,
                ));
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn dispatch_uses_mock_sheets_client() {
        let row = MasterSolutionsRow {
            status_atualizacao: "CONCLUIDO".to_string(),
            status_fase: "F4".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SGR".to_string(),
            declared_description: "Descricao".to_string(),
            proposta_original_resumo: "Resumo".to_string(),
            stack_base: "Rust".to_string(),
            licenca: "MIT".to_string(),
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            ..Default::default()
        };

        let sheets = MockSheetsClient::new();
        SsotInjector::dispatch_master_solutions_row(&sheets, "SHEET_ID", 2, &row)
            .await
            .unwrap();

        let calls = sheets.calls.lock().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "SHEET_ID");
        assert_eq!(calls[0].1, "MASTER_SOLUTIONS");
        assert!(calls[0].2.get("D2:D2").is_some());
    }
}
