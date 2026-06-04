use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::Instant;
use tracing::{error, info, warn};

use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::Connection;
use genesis_mc_lib::cognition::synthesizer::master_solutions_header_range;

type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
type SheetsUpdateFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

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
        ranges: serde_json::Value,
    ) -> SheetsUpdateFuture<'a>;
}

struct SheetsMcpClient;

impl SheetsMcpClient {
    async fn poll_for_jsonrpc_response_from_reader<R>(
        reader: R,
        timeout: Duration,
        expected_id: i64,
    ) -> Result<Value, String>
    where
        R: tokio::io::AsyncBufRead + Unpin,
    {
        let started = Instant::now();
        let mut lines = reader.lines();
        loop {
            if started.elapsed() > timeout {
                return Err(format!(
                    "Timeout: O servidor MCP (Sheets) não emitiu o payload após {} segundos.",
                    timeout.as_secs()
                ));
            }

            match tokio::time::timeout(Duration::from_millis(200), lines.next_line()).await {
                Ok(Ok(Some(line))) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&line) {
                        let id_matches = match value.get("id") {
                            Some(Value::Number(n)) => n.as_i64() == Some(expected_id),
                            Some(Value::String(s)) => s.parse::<i64>().ok() == Some(expected_id),
                            _ => false,
                        };
                        if id_matches {
                            return Ok(value);
                        }
                    }
                }
                Ok(Ok(None)) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                Ok(Err(e)) => return Err(format!("Falha ao ler stdout do MCP: {e}")),
                Err(_) => {}
            }
        }
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
                "clientInfo": { "name": "n0-daemon-watcher", "version": "1.0.0" }
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

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "stdin indisponível".to_string())?;
        stdin
            .write_all(format!("{init_req}\n").as_bytes())
            .await
            .map_err(|e| format!("Falha ao escrever init_req: {e}"))?;
        stdin
            .write_all(format!("{initialized_notif}\n").as_bytes())
            .await
            .map_err(|e| format!("Falha ao escrever initialized: {e}"))?;
        stdin
            .write_all(format!("{mcp_request}\n").as_bytes())
            .await
            .map_err(|e| format!("Falha ao escrever tools/call: {e}"))?;
        drop(stdin);

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "stdout indisponível".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "stderr indisponível".to_string())?;

        let stdout_reader = BufReader::new(stdout);
        let _stderr_reader = BufReader::new(stderr);
        let msg =
            Self::poll_for_jsonrpc_response_from_reader(stdout_reader, Duration::from_secs(20), 1)
                .await?;

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
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn try_extract_project_name_from_repo_url(repo_url: &str) -> Option<String> {
    let s = repo_url.trim();
    let s = s.trim_end_matches('/').trim_end_matches(".git");
    let marker = "github.com/";
    let idx = s.to_ascii_lowercase().find(marker)?;
    let rest = &s[(idx + marker.len())..];
    let mut parts = rest.split('/').map(|p| p.trim()).filter(|p| !p.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    Some(format!("{owner}/{repo}"))
}

fn short_circuit_cleanup_sqlite(project_name: &str) -> Result<usize, String> {
    let root = workspace_root().map_err(|e| e.to_string())?;
    let db_path = root.join(".soda_data").join("soda_heuristic_vault.db");
    let conn =
        Connection::open(&db_path).map_err(|e| format!("Falha ao abrir vault {}: {e}", db_path.display()))?;
    match conn.execute("DELETE FROM artefatos_brutos WHERE repo_id = ?1", [project_name]) {
        Ok(n) => Ok(n),
        Err(e) => {
            let msg = e.to_string();
            if msg.to_ascii_lowercase().contains("no such table") {
                return Ok(0);
            }
            Err(format!(
                "Falha ao deletar artefatos_brutos para repo_id='{project_name}': {msg}"
            ))
        }
    }
}

async fn short_circuit_mark_sheet<S: SheetsClient>(
    sheets: &S,
    spreadsheet_id: &str,
    status_fase_idx: usize,
    row_number_1based: u32,
) -> Result<(), String> {
    let col = col_idx_to_a1(status_fase_idx);
    let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
    let mut ranges = serde_json::Map::new();
    ranges.insert(range, json!([[ "SHORT-CIRCUIT" ]]));
    sheets
        .batch_update_cells(spreadsheet_id, "MASTER_SOLUTIONS", Value::Object(ranges))
        .await
        .map_err(|e| format!("Falha ao atualizar status_fase=SHORT-CIRCUIT no Sheets: {e}"))
}

impl SheetsClient for SheetsMcpClient {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a> {
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
            extract_values_2d_strict(&result)
        })
    }

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: serde_json::Value,
    ) -> SheetsUpdateFuture<'a> {
        Box::pin(async move {
            let _ = Self::call_mcp(
                "batch_update_cells",
                json!({
                    "spreadsheet_id": spreadsheet_id,
                    "sheet": sheet,
                    "ranges": ranges
                }),
            )
            .await?;
            Ok(())
        })
    }
}

fn extract_values_2d_strict(value: &Value) -> Result<Vec<Vec<String>>, String> {
    if let Some(err) = value.get("error") {
        let code = err.get("code").and_then(|v| v.as_i64());
        let message = err.get("message").and_then(|v| v.as_str());
        return Err(match (code, message) {
            (Some(c), Some(m)) => format!("Google Sheets API error: code={c} message={m}"),
            (Some(c), None) => format!("Google Sheets API error: code={c}"),
            (None, Some(m)) => format!("Google Sheets API error: message={m}"),
            _ => format!("Google Sheets API error: {err}"),
        });
    }

    let values = if let Some(arr) = value.get("values").and_then(|v| v.as_array()) {
        arr
    } else {
        let vr = value
            .get("valueRanges")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| "Sheets payload inválido: sem 'values' ou 'valueRanges'".to_string())?;
        vr.get("values")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Sheets payload inválido: 'valueRanges[0].values' ausente".to_string())?
    };

    let mut out = Vec::new();
    for row in values {
        let row_arr = row
            .as_array()
            .ok_or_else(|| "Sheets payload inválido: linha não é array".to_string())?;
        out.push(
            row_arr
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect(),
        );
    }
    Ok(out)
}

#[derive(Clone, Copy)]
struct JitterPolicy {
    base_ms: u64,
    jitter_max_ms: u64,
}

impl JitterPolicy {
    fn compute_sleep<R: RngCore>(&self, rng: &mut R) -> Duration {
        let jitter = if self.jitter_max_ms == 0 {
            0
        } else {
            rng.next_u64() % (self.jitter_max_ms.saturating_add(1))
        };
        Duration::from_millis(self.base_ms.saturating_add(jitter))
    }
}

#[derive(Clone, Copy)]
struct RetryPolicy {
    backoff_base_ms: u64,
    jitter_max_ms: u64,
}

fn backoff_delay_ms(base_ms: u64, attempt_index_1based: u32) -> u64 {
    let shift = attempt_index_1based.saturating_sub(1).min(16);
    base_ms.saturating_mul(1u64 << shift)
}

trait Sleeper: Send + Sync {
    fn sleep<'a>(&'a self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

struct TokioSleeper;

impl Sleeper for TokioSleeper {
    fn sleep<'a>(&'a self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            tokio::time::sleep(duration).await;
        })
    }
}

struct BackoffGuard<S: Sleeper> {
    policy: RetryPolicy,
    rng: Arc<Mutex<OsRng>>,
    sleeper: Arc<S>,
}

impl<S: Sleeper> BackoffGuard<S> {
    async fn sleep_before_retry(&self, attempt_index_1based: u32) {
        let base = backoff_delay_ms(self.policy.backoff_base_ms, attempt_index_1based);
        let jitter = if self.policy.jitter_max_ms == 0 {
            0
        } else {
            let mut rng = self.rng.lock().await;
            rng.next_u64() % (self.policy.jitter_max_ms.saturating_add(1))
        };
        let delay = Duration::from_millis(base.saturating_add(jitter));
        self.sleeper.sleep(delay).await;
    }
}

impl<S: Sleeper> Clone for BackoffGuard<S> {
    fn clone(&self) -> Self {
        Self {
            policy: self.policy,
            rng: self.rng.clone(),
            sleeper: self.sleeper.clone(),
        }
    }
}

#[derive(Debug, Clone)]
enum DispatchError {
    RateLimited,
    #[allow(dead_code)]
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RouteDecision {
    N1,
    N2,
    N3,
    N4,
    #[allow(dead_code)]
    N5,
    N6,
    ShortCircuit,
    Skip,
}

fn route_for_status_atualizacao(raw: &str) -> RouteDecision {
    let s = raw.trim();
    if s.is_empty() {
        return RouteDecision::N1;
    }
    if s.starts_with("REJEITADO_") {
        return RouteDecision::ShortCircuit;
    }
    match s {
        "INICIAR_TRIAGEM" => RouteDecision::N2,
        "APROVADO_PARA_HARVESTER" => RouteDecision::N3,
        "APROVADO_PARA_ENXAME" => RouteDecision::N4,
        "APROVADO_DEEP_COMPONENTS_ANALYSIS" => RouteDecision::N6,
        _ => RouteDecision::Skip,
    }
}

trait Dispatcher: Send + Sync {
    fn dispatch<'a>(
        &'a self,
        route: RouteDecision,
        ctx: RowCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), DispatchError>> + Send + 'a>>;
}

#[derive(Clone, Debug)]
struct RowCtx {
    row_number_1based: u32,
    repo_url: String,
    project_name: String,
    status_atualizacao: String,
    status_fase: String,
}

#[derive(Default, Debug)]
struct Telemetry {
    linhas_inspecionadas: usize,
    roteadas_n1: usize,
    roteadas_n2: usize,
    roteadas_n3: usize,
    roteadas_n4: usize,
    roteadas_n5: usize,
    roteadas_n6: usize,
    roteadas_short_circuit: usize,
    erros_sheets: usize,
    erros_dispatch: usize,
}

fn should_colorize_telemetry() -> bool {
    std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn ansi_wrap(style: &str, text: &str) -> String {
    format!("\x1b[{style}m{text}\x1b[0m")
}

fn colored_kv(key: &str, value: usize, style: &str, enabled: bool) -> String {
    if enabled {
        ansi_wrap(style, &format!("{key}={value}"))
    } else {
        format!("{key}={value}")
    }
}

fn format_telemetry_line(tel: &Telemetry) -> String {
    let color = should_colorize_telemetry();
    let n1 = colored_kv("n1", tel.roteadas_n1, "97;44", color);
    let n2 = colored_kv("n2", tel.roteadas_n2, "97;45", color);
    let n3 = colored_kv("n3", tel.roteadas_n3, "30;42", color);
    let n4 = colored_kv("n4", tel.roteadas_n4, "30;43", color);
    let n5 = colored_kv("n5", tel.roteadas_n5, "30;46", color);
    let n6 = colored_kv("n6", tel.roteadas_n6, "97;41", color);
    format!(
        "N0: rodada concluída | linhas={} | {} {} {} {} {} {} | short_circuit={} | erros_sheets={} | erros_dispatch={}",
        tel.linhas_inspecionadas,
        n1,
        n2,
        n3,
        n4,
        n5,
        n6,
        tel.roteadas_short_circuit,
        tel.erros_sheets,
        tel.erros_dispatch
    )
}

struct DaemonConfig {
    scan_sleep: JitterPolicy,
    max_parallel: usize,
    max_attempts_per_line: u32,
}

struct DaemonWatcher<S: SheetsClient + 'static, D: Dispatcher + 'static, Sl: Sleeper + 'static> {
    sheets: Arc<S>,
    dispatcher: Arc<D>,
    guard: BackoffGuard<Sl>,
    config: DaemonConfig,
}

impl<S: SheetsClient + 'static, D: Dispatcher + 'static, Sl: Sleeper + 'static> DaemonWatcher<S, D, Sl> {
    async fn run_once(&self, spreadsheet_id: &str) -> Telemetry {
        let mut tel = Telemetry::default();

        let header_range = master_solutions_header_range();
        let header = match self
            .sheets
            .get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", header_range.clone())
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tel.erros_sheets += 1;
                error!(error = %e, "N0: falha ao ler header");
                return tel;
            }
        };
        let header_row = header.first().cloned().unwrap_or_default();
        if header_row.is_empty() {
            tel.erros_sheets += 1;
            error!(range = %header_range, "N0: header vazio em MASTER_SOLUTIONS (falha de leitura ou payload inesperado)");
            return tel;
        }
        let cols = match resolve_column_map(&header_row) {
            Ok(v) => v,
            Err(e) => {
                tel.erros_sheets += 1;
                error!(error = %e, "N0: header inválido");
                return tel;
            }
        };

        let min_idx = cols
            .iter_required()
            .into_iter()
            .min()
            .unwrap_or(0);
        let max_idx = cols
            .iter_required()
            .into_iter()
            .max()
            .unwrap_or(0);
        let start_col = col_idx_to_a1(min_idx);
        let end_col = col_idx_to_a1(max_idx);
        let range = format!("{start_col}2:{end_col}");
        let values = match self
            .sheets
            .get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", range)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tel.erros_sheets += 1;
                error!(error = %e, "N0: falha ao ler values");
                return tel;
            }
        };

        let sem = Arc::new(Semaphore::new(self.config.max_parallel.max(1)));
        let mut tasks = Vec::new();
        let mut prioritized = Vec::new();
        let mut others = Vec::new();
        for (idx, row) in values.into_iter().enumerate() {
            tel.linhas_inspecionadas += 1;
            let row_number_1based = (idx as u32) + 2;
            let ctx = cols.extract_row_ctx(row_number_1based, min_idx, &row);
            let Some(ctx) = ctx else { continue };
            let route = route_for_status_atualizacao(&ctx.status_atualizacao);
            match route {
                RouteDecision::N1 => tel.roteadas_n1 += 1,
                RouteDecision::N2 => tel.roteadas_n2 += 1,
                RouteDecision::N3 => tel.roteadas_n3 += 1,
                RouteDecision::N4 => tel.roteadas_n4 += 1,
                RouteDecision::N5 => tel.roteadas_n5 += 1,
                RouteDecision::N6 => tel.roteadas_n6 += 1,
                RouteDecision::ShortCircuit => tel.roteadas_short_circuit += 1,
                RouteDecision::Skip => {}
            }
            if route == RouteDecision::Skip {
                continue;
            }
            if route == RouteDecision::ShortCircuit {
                let project_name = if !ctx.project_name.trim().is_empty() {
                    ctx.project_name.trim().to_string()
                } else if let Some(extracted) = try_extract_project_name_from_repo_url(&ctx.repo_url) {
                    extracted
                } else {
                    tel.erros_dispatch += 1;
                    error!(
                        row_number_1based = ctx.row_number_1based,
                        repo_url = %ctx.repo_url,
                        "N0: SHORT-CIRCUIT falhou (sem project_name e repo_url não parseável)"
                    );
                    continue;
                };

                match short_circuit_cleanup_sqlite(&project_name) {
                    Ok(deleted) => {
                        info!(
                            row_number_1based = ctx.row_number_1based,
                            project_name = %project_name,
                            deleted,
                            "N0: SHORT-CIRCUIT cleanup concluído (artefatos_brutos)"
                        );
                    }
                    Err(e) => {
                        tel.erros_dispatch += 1;
                        error!(
                            row_number_1based = ctx.row_number_1based,
                            project_name = %project_name,
                            error = %e,
                            "N0: SHORT-CIRCUIT cleanup falhou"
                        );
                        continue;
                    }
                }

                if let Err(e) = short_circuit_mark_sheet(
                    self.sheets.as_ref(),
                    spreadsheet_id,
                    cols.status_fase_idx,
                    ctx.row_number_1based,
                )
                .await
                {
                    tel.erros_sheets += 1;
                    error!(
                        row_number_1based = ctx.row_number_1based,
                        project_name = %project_name,
                        error = %e,
                        "N0: SHORT-CIRCUIT falhou ao marcar status_fase no Sheets"
                    );
                }
                continue;
            }
            let item = (ctx, route);
            if route == RouteDecision::N1 {
                prioritized.push(item);
            } else {
                others.push(item);
            }
        }

        prioritized.extend(others);
        for (ctx, route) in prioritized {
            let permit = sem.clone().acquire_owned().await.unwrap();
            let dispatcher = self.dispatcher.clone();
            let guard = self.guard.clone();
            let max_attempts = self.config.max_attempts_per_line.max(1);
            tasks.push(tokio::spawn(async move {
                let _permit = permit;
                for attempt in 1..=max_attempts {
                    let res = dispatcher.dispatch(route, ctx.clone()).await;
                    match res {
                        Ok(()) => return Ok::<(), DispatchError>(()),
                        Err(DispatchError::RateLimited) => {
                            warn!(
                                row_number_1based = ctx.row_number_1based,
                                repo_url = %ctx.repo_url,
                                project_name = %ctx.project_name,
                                status_fase = %ctx.status_fase,
                                attempt,
                                "N0: rate limit (429) em dispatch; aplicando backoff"
                            );
                            guard.sleep_before_retry(attempt).await;
                            continue;
                        }
                        Err(e) => {
                            warn!(
                                row_number_1based = ctx.row_number_1based,
                                repo_url = %ctx.repo_url,
                                project_name = %ctx.project_name,
                                status_fase = %ctx.status_fase,
                                error = ?e,
                                "N0: falha em dispatch (fail-soft por linha)"
                            );
                            return Err::<(), DispatchError>(e);
                        }
                    }
                }
                warn!(
                    row_number_1based = ctx.row_number_1based,
                    repo_url = %ctx.repo_url,
                    project_name = %ctx.project_name,
                    status_fase = %ctx.status_fase,
                    "N0: exaustão de tentativas por rate limit (fail-soft por linha)"
                );
                Err(DispatchError::RateLimited)
            }));
        }

        for t in tasks {
            match t.await {
                Ok(Ok(())) => {}
                Ok(Err(_e)) => tel.erros_dispatch += 1,
                Err(_join) => tel.erros_dispatch += 1,
            }
        }

        tel
    }

    async fn run_daemon(&self, spreadsheet_id: &str) {
        let mut rng = OsRng;
        loop {
            let tel = self.run_once(spreadsheet_id).await;
            info!("{}", format_telemetry_line(&tel));
            let sleep = self.config.scan_sleep.compute_sleep(&mut rng);
            tokio::time::sleep(sleep).await;
        }
    }
}

#[derive(Clone, Copy)]
struct MasterColumns {
    repo_url_idx: usize,
    project_name_idx: usize,
    status_atualizacao_idx: usize,
    status_fase_idx: usize,
}

impl MasterColumns {
    fn iter_required(&self) -> [usize; 4] {
        [
            self.repo_url_idx,
            self.project_name_idx,
            self.status_atualizacao_idx,
            self.status_fase_idx,
        ]
    }

    fn extract_row_ctx(&self, row_number_1based: u32, min_idx: usize, row: &[String]) -> Option<RowCtx> {
        let get = |abs_idx: usize| -> String {
            let rel = abs_idx.saturating_sub(min_idx);
            row.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        let repo_url = get(self.repo_url_idx);
        if repo_url.is_empty() {
            return None;
        }
        Some(RowCtx {
            row_number_1based,
            repo_url,
            project_name: get(self.project_name_idx),
            status_atualizacao: get(self.status_atualizacao_idx),
            status_fase: get(self.status_fase_idx),
        })
    }
}

fn normalize_header_cell(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

fn resolve_column_map(header_row: &[String]) -> Result<MasterColumns, String> {
    let mut map = HashMap::new();
    for (idx, cell) in header_row.iter().enumerate() {
        let k = normalize_header_cell(cell);
        if !k.is_empty() {
            map.insert(k, idx);
        }
    }
    let repo_url_idx = *map
        .get("repo_url")
        .ok_or_else(|| "Header missing repo_url".to_string())?;
    let project_name_idx = *map
        .get("project_name")
        .ok_or_else(|| "Header missing project_name".to_string())?;
    let status_atualizacao_idx = *map
        .get("status_atualizacao")
        .ok_or_else(|| "Header missing status_atualizacao".to_string())?;
    let status_fase_idx = *map
        .get("status_fase")
        .ok_or_else(|| "Header missing status_fase".to_string())?;
    Ok(MasterColumns {
        repo_url_idx,
        project_name_idx,
        status_atualizacao_idx,
        status_fase_idx,
    })
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
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(std::io::stderr().is_terminal())
        .init();

    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let run_once = std::env::args().any(|arg| arg == "--once" || arg == "--dry-run");

    let watcher = DaemonWatcher {
        sheets: Arc::new(SheetsMcpClient),
        dispatcher: Arc::new(NoopDispatcher),
        guard: BackoffGuard {
            policy: RetryPolicy {
                backoff_base_ms: 1000,
                jitter_max_ms: 1000,
            },
            rng: Arc::new(Mutex::new(OsRng)),
            sleeper: Arc::new(TokioSleeper),
        },
        config: DaemonConfig {
            scan_sleep: JitterPolicy {
                base_ms: 5_000,
                jitter_max_ms: 3_000,
            },
            max_parallel: 4,
            max_attempts_per_line: 3,
        },
    };

    if run_once {
        let tel = watcher.run_once(&spreadsheet_id).await;
        info!(
            linhas_inspecionadas = tel.linhas_inspecionadas,
            n1 = tel.roteadas_n1,
            n2 = tel.roteadas_n2,
            n3 = tel.roteadas_n3,
            n4 = tel.roteadas_n4,
            n5 = tel.roteadas_n5,
            short_circuit = tel.roteadas_short_circuit,
            erros_sheets = tel.erros_sheets,
            erros_dispatch = tel.erros_dispatch,
            "N0: dry-run concluído (1 rodada)"
        );
        return Ok(());
    }

    watcher.run_daemon(&spreadsheet_id).await;
    Ok(())
}

struct NoopDispatcher;

impl Dispatcher for NoopDispatcher {
    fn dispatch<'a>(
        &'a self,
        _route: RouteDecision,
        _ctx: RowCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), DispatchError>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::mock::StepRng;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Mutex as TokioMutex;

    struct RecordingSleeper {
        sleeps: TokioMutex<Vec<Duration>>,
    }

    impl RecordingSleeper {
        fn new() -> Self {
            Self {
                sleeps: TokioMutex::new(Vec::new()),
            }
        }
    }

    impl Sleeper for RecordingSleeper {
        fn sleep<'a>(&'a self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(async move {
                self.sleeps.lock().await.push(duration);
            })
        }
    }

    struct MockSheets {
        header: Vec<Vec<String>>,
        values: Vec<Vec<String>>,
    }

    impl MockSheets {
        fn new(header: Vec<Vec<String>>, values: Vec<Vec<String>>) -> Self {
            Self { header, values }
        }
    }

    impl SheetsClient for MockSheets {
        fn get_sheet_data<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            range: String,
        ) -> SheetsDataFuture<'a> {
            Box::pin(async move {
                if range.contains("1:") {
                    Ok(self.header.clone())
                } else {
                    Ok(self.values.clone())
                }
            })
        }

        fn batch_update_cells<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            _ranges: Value,
        ) -> SheetsUpdateFuture<'a> {
            Box::pin(async move { Ok(()) })
        }
    }

    #[test]
    fn jitter_respects_base_plus_random_fluctuation() {
        let p = JitterPolicy {
            base_ms: 100,
            jitter_max_ms: 50,
        };
        let mut rng = StepRng::new(0, 1);
        let a = p.compute_sleep(&mut rng);
        let b = p.compute_sleep(&mut rng);
        assert!(a >= Duration::from_millis(100) && a <= Duration::from_millis(150));
        assert!(b >= Duration::from_millis(100) && b <= Duration::from_millis(150));
        assert_ne!(a, b);
    }

    #[test]
    fn routing_catalog_is_deterministic_and_strict() {
        assert_eq!(route_for_status_atualizacao(""), RouteDecision::N1);
        assert_eq!(
            route_for_status_atualizacao("INICIAR_TRIAGEM"),
            RouteDecision::N2
        );
        assert_eq!(
            route_for_status_atualizacao("APROVADO_PARA_HARVESTER"),
            RouteDecision::N3
        );
        assert_eq!(
            route_for_status_atualizacao("APROVADO_PARA_ENXAME"),
            RouteDecision::N4
        );
        assert_eq!(
            route_for_status_atualizacao("APROVADO_DEEP_COMPONENTS_ANALYSIS"),
            RouteDecision::N6
        );
        assert_eq!(
            route_for_status_atualizacao("REJEITADO_LIXO_TOXICO"),
            RouteDecision::ShortCircuit
        );
        assert_eq!(
            route_for_status_atualizacao("TRIAGEM_CONCLUIDA"),
            RouteDecision::Skip
        );
        let _ = DispatchError::Other("x".to_string());
    }

    struct MockDispatcher {
        calls: AtomicUsize,
        rate_limit_first_n: usize,
    }

    impl Dispatcher for MockDispatcher {
        fn dispatch<'a>(
            &'a self,
            route: RouteDecision,
            ctx: RowCtx,
        ) -> Pin<Box<dyn Future<Output = Result<(), DispatchError>> + Send + 'a>> {
            Box::pin(async move {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if route == RouteDecision::N1 && ctx.repo_url.contains("rate") && n < self.rate_limit_first_n
                {
                    return Err(DispatchError::RateLimited);
                }
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn rate_limit_429_triggers_exponential_backoff_and_fail_soft_per_row() {
        let header = vec![vec![
            "repo_url".to_string(),
            "project_name".to_string(),
            "status_atualizacao".to_string(),
            "status_fase".to_string(),
        ]];
        let values = vec![
            vec![
                "https://github.com/acme/rate".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
            vec![
                "https://github.com/acme/ok".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
        ];
        let sheets = Arc::new(MockSheets::new(header, values));
        let dispatcher = Arc::new(MockDispatcher {
            calls: AtomicUsize::new(0),
            rate_limit_first_n: 3,
        });
        let sleeper = Arc::new(RecordingSleeper::new());
        let watcher = DaemonWatcher {
            sheets,
            dispatcher,
            guard: BackoffGuard {
                policy: RetryPolicy {
                    backoff_base_ms: 100,
                    jitter_max_ms: 0,
                },
                rng: Arc::new(Mutex::new(OsRng)),
                sleeper: sleeper.clone(),
            },
            config: DaemonConfig {
                scan_sleep: JitterPolicy {
                    base_ms: 1,
                    jitter_max_ms: 0,
                },
                max_parallel: 1,
                max_attempts_per_line: 3,
            },
        };

        let tel = watcher.run_once("SHEET").await;
        assert_eq!(tel.roteadas_n1, 2);
        assert_eq!(tel.erros_dispatch, 1);

        let sleeps = sleeper.sleeps.lock().await.clone();
        assert_eq!(sleeps, vec![Duration::from_millis(100), Duration::from_millis(200), Duration::from_millis(400)]);
    }
}
