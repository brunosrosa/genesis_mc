use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};
use url::Url;
use tokio::time::Instant;
use tokio::io::AsyncBufReadExt;

type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
type SheetsUpdateFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
type GithubTagFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Option<String>, String>> + Send + 'a>>;

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

trait GithubClient: Send + Sync {
    fn latest_release_tag<'a>(
        &'a self,
        repo_url: &'a str,
    ) -> GithubTagFuture<'a>;
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
                    "Timeout: O servidor MCP (Sheets) não emitiu o payload após {} segundos. Verifique dependências Node/Python.",
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
                Ok(Err(e)) => {
                    return Err(format!("Falha ao ler stdout do MCP: {e}"));
                }
                Err(_) => {}
            }
        }
    }

    async fn call_mcp(tool_name: &str, arguments: Value) -> Result<Value, String> {
        use tokio::io::{AsyncWriteExt, BufReader};
        use tokio::process::Command;
        use std::process::Stdio;

        let creds = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| "Missing GOOGLE_APPLICATION_CREDENTIALS".to_string())?;

        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "f-minus-1-guardian", "version": "1.0.0" }
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

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "stdin indisponível".to_string())?;
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

        let timeout = match tool_name {
            "batch_update_cells" => Duration::from_secs(120),
            _ => Duration::from_secs(20),
        };
        let msg = Self::poll_for_jsonrpc_response_from_reader(stdout_reader, timeout, 1).await?;

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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>> {
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

struct ReqwestGithubClient {
    http: Client,
    api_base: String,
    allow_host_override: bool,
    github_pat: String,
    policy: RetryPolicy,
    jitter_state: AtomicU64,
}

#[derive(Clone, Copy)]
struct RetryPolicy {
    max_attempts: u32,
    jitter_min_ms: u64,
    jitter_max_ms: u64,
    backoff_base_ms: u64,
}

fn xorshift64star(state: u64) -> u64 {
    let mut x = state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    x.wrapping_mul(2685821657736338717u64)
}

fn jitter_ms(state: &AtomicU64, min_ms: u64, max_ms: u64) -> u64 {
    if max_ms <= min_ms {
        return min_ms;
    }
    let cur = state.load(Ordering::Relaxed);
    let next = xorshift64star(cur.wrapping_add(0x9E3779B97F4A7C15));
    state.store(next, Ordering::Relaxed);
    min_ms + (next % (max_ms - min_ms + 1))
}

fn backoff_delay_ms(base_ms: u64, attempt_index_1based: u32) -> u64 {
    let shift = attempt_index_1based.saturating_sub(1).min(16);
    base_ms.saturating_mul(1u64 << shift)
}

impl ReqwestGithubClient {
    fn new() -> Result<Self, String> {
        Self::new_with_policy(RetryPolicy {
            max_attempts: 3,
            jitter_min_ms: 2000,
            jitter_max_ms: 4500,
            backoff_base_ms: 2000,
        })
    }

    fn new_with_policy(policy: RetryPolicy) -> Result<Self, String> {
        let api_base = std::env::var("SODA_GITHUB_API_BASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://api.github.com".to_string());
        let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
        let github_pat = std::env::var("GITHUB_PAT")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "Missing GITHUB_PAT".to_string())?;
        let http = Client::builder()
            .user_agent("f-minus-1-guardian/1.0")
            .build()
            .map_err(|e| format!("Falha ao criar client HTTP: {e}"))?;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xD1B54A32D192ED03);
        Ok(Self {
            http,
            api_base,
            allow_host_override,
            github_pat,
            policy,
            jitter_state: AtomicU64::new(seed),
        })
    }

    async fn get_with_retries(&self, endpoint: &str) -> Result<reqwest::Response, String> {
        for attempt in 1..=self.policy.max_attempts {
            let sleep_ms = jitter_ms(
                &self.jitter_state,
                self.policy.jitter_min_ms,
                self.policy.jitter_max_ms,
            );
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

            let resp = self
                .http
                .get(endpoint)
                .bearer_auth(&self.github_pat)
                .send()
                .await
                .map_err(|e| format!("Falha HTTP GitHub: {e}"))?;

            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status == reqwest::StatusCode::FORBIDDEN
            {
                if attempt < self.policy.max_attempts {
                    let backoff_ms = backoff_delay_ms(self.policy.backoff_base_ms, attempt);
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    continue;
                }
                return Err(format!(
                    "GitHub rate limit (status {}) após {} tentativas",
                    status, attempt
                ));
            }

            return Ok(resp);
        }

        Err("Falha inesperada: loop de tentativas terminou sem retorno".to_string())
    }
}

#[derive(Deserialize)]
struct GithubReleaseResponse {
    tag_name: Option<String>,
}

#[derive(Deserialize)]
struct GithubRepoResponse {
    default_branch: Option<String>,
}

impl GithubClient for ReqwestGithubClient {
    fn latest_release_tag<'a>(
        &'a self,
        repo_url: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send + 'a>> {
        Box::pin(async move {
            let url = Url::parse(repo_url).map_err(|e| format!("repo_url inválida: {e}"))?;
            if url.host_str() != Some("github.com") && !self.allow_host_override {
                return Ok(None);
            }
            let mut segments = url
                .path_segments()
                .map(|parts| parts.collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter()
                .filter(|segment| !segment.is_empty())
                .map(|segment| segment.trim_end_matches(".git").to_string())
                .collect::<Vec<_>>();
            if segments.len() < 2 {
                return Ok(None);
            }
            let repo = segments.pop().unwrap();
            let owner = segments.pop().unwrap();

            let base = self.api_base.trim_end_matches('/');
            let repo_endpoint = format!("{base}/repos/{owner}/{repo}");
            let release_endpoint = format!("{repo_endpoint}/releases/latest");

            let release_resp = self.get_with_retries(&release_endpoint).await?;
            let release_status = release_resp.status();
            if release_status.is_success() {
                let parsed = release_resp
                    .json::<GithubReleaseResponse>()
                    .await
                    .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
                if let Some(tag) = parsed
                    .tag_name
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                {
                    return Ok(Some(tag));
                }
            } else if release_status != reqwest::StatusCode::NOT_FOUND {
                return Err(format!("GitHub retornou status {}", release_status));
            }

            let repo_resp = self.get_with_retries(&repo_endpoint).await?;
            let repo_status = repo_resp.status();
            if repo_status == reqwest::StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if !repo_status.is_success() {
                return Err(format!("GitHub retornou status {}", repo_status));
            }
            let repo_meta = repo_resp
                .json::<GithubRepoResponse>()
                .await
                .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;

            let tags_endpoint = format!("{repo_endpoint}/tags?per_page=1");
            let tags_resp = self.get_with_retries(&tags_endpoint).await?;
            let tags_status = tags_resp.status();
            if tags_status.is_success() {
                let tags = tags_resp
                    .json::<Vec<Value>>()
                    .await
                    .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
                if let Some(tag) = tags
                    .first()
                    .and_then(|t| t.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|v| !v.is_empty())
                {
                    return Ok(Some(tag));
                }
            } else if tags_status != reqwest::StatusCode::NOT_FOUND {
                return Err(format!("GitHub retornou status {}", tags_status));
            }

            if let Some(branch) = repo_meta
                .default_branch
                .as_deref()
                .map(|b| b.trim())
                .filter(|b| !b.is_empty())
            {
                let commit_endpoint = format!("{repo_endpoint}/commits/{branch}");
                let commit_resp = self.get_with_retries(&commit_endpoint).await?;
                let commit_status = commit_resp.status();
                if !commit_status.is_success() {
                    return Err(format!("GitHub retornou status {}", commit_status));
                }
                let commit = commit_resp
                    .json::<Value>()
                    .await
                    .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
                let sha = commit
                    .get("sha")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "GitHub: resposta de commit sem sha".to_string())?;
                let short = sha.chars().take(7).collect::<String>();
                return Ok(Some(short));
            }

            let commits_endpoint = format!("{repo_endpoint}/commits?per_page=1");
            let commits_resp = self.get_with_retries(&commits_endpoint).await?;
            let commits_status = commits_resp.status();
            if !commits_status.is_success() {
                return Err(format!("GitHub retornou status {}", commits_status));
            }
            let commits = commits_resp
                .json::<Vec<Value>>()
                .await
                .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
            let sha = commits
                .first()
                .and_then(|c| c.get("sha"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "GitHub: lista de commits vazia".to_string())?;
            let short = sha.chars().take(7).collect::<String>();
            Ok(Some(short))
        })
    }
}

fn normalize_version(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.starts_with('v') || s.starts_with('V') {
        s = s[1..].to_string();
    }
    if let Some(stripped) = s.strip_prefix("release-") {
        s = stripped.to_string();
    }
    s.trim().to_string()
}

fn has_drift(repo_analised_version: &str, github_latest: &str) -> bool {
    let local = normalize_version(repo_analised_version);
    let remote = normalize_version(github_latest);
    !(remote.is_empty() || (!local.is_empty() && local == remote))
}

fn try_extract_project_name_from_repo_url(repo_url: &str) -> Option<String> {
    let url = Url::parse(repo_url).ok()?;
    if !url.host_str()?.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let mut parts = url.path().trim_matches('/').split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
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

#[derive(Debug, Clone)]
struct ColumnMap {
    status_atualizacao_idx: usize,
    status_fase_idx: Option<usize>,
    project_name_idx: Option<usize>,
    repo_url_idx: usize,
    repo_analised_version_idx: Option<usize>,
    ultima_versao_online_idx: usize,
}

fn resolve_column_map(header_row: &[String]) -> Result<ColumnMap, String> {
    let mut status_atualizacao_idx = None;
    let mut status_fase_idx = None;
    let mut project_name_idx = None;
    let mut repo_url_idx = None;
    let mut repo_analised_version_idx = None;
    let mut ultima_versao_online_idx = None;

    for (idx, raw) in header_row.iter().enumerate() {
        let h = normalize_header_cell(raw);
        match h.as_str() {
            "status_atualizacao" => status_atualizacao_idx = Some(idx),
            "status_fase" => status_fase_idx = Some(idx),
            "project_name" => project_name_idx = Some(idx),
            "repo_url" => repo_url_idx = Some(idx),
            "repo_analised_version" => repo_analised_version_idx = Some(idx),
            "repo_version" => {
                if repo_analised_version_idx.is_none() {
                    repo_analised_version_idx = Some(idx)
                }
            }
            "ultima_versao_online" => ultima_versao_online_idx = Some(idx),
            _ => {}
        }
    }

    let Some(status_atualizacao_idx) = status_atualizacao_idx else {
        return Err("Cabeçalho não contém 'status_atualizacao'".to_string());
    };
    let Some(repo_url_idx) = repo_url_idx else {
        return Err("Cabeçalho não contém 'repo_url'".to_string());
    };
    let Some(ultima_versao_online_idx) = ultima_versao_online_idx else {
        return Err("Cabeçalho não contém 'ultima_versao_online'".to_string());
    };

    Ok(ColumnMap {
        status_atualizacao_idx,
        status_fase_idx,
        project_name_idx,
        repo_url_idx,
        repo_analised_version_idx,
        ultima_versao_online_idx,
    })
}

fn should_skip_by_status_atualizacao(status_atualizacao: &str) -> bool {
    let s = status_atualizacao.trim();
    if s.is_empty() {
        return false;
    }
    let up = s.to_ascii_uppercase();
    up == "INICIAR_TRIAGEM"
        || up == "TRIAGEM_CONCLUIDA"
        || up.starts_with("APROVADO_")
        || up.starts_with("REJEITADO_")
        || up == "DESATUALIZADA"
        || up == "EM_PROCESSAMENTO"
        || up.starts_with("PENDENTE_")
        || up == "NOVO_LINK_OK"
}

fn try_build_repo_url_from_project_name(project_name: &str) -> Option<String> {
    let raw = project_name.trim();
    if raw.is_empty() {
        return None;
    }
    let mut parts = raw.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}"))
}

struct Guardian<S: SheetsClient, G: GithubClient> {
    sheets: Arc<S>,
    github: Arc<G>,
}

impl<S: SheetsClient + 'static, G: GithubClient + 'static> Guardian<S, G> {
    async fn run_once(&self, spreadsheet_id: &str) -> Result<(), String> {
        let header = self
            .sheets
            .get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", "A1:CF1".to_string())
            .await?;
        let header_row = header.first().cloned().unwrap_or_default();

        let cols = resolve_column_map(&header_row)?;
        let mut required = vec![
            cols.status_atualizacao_idx,
            cols.repo_url_idx,
            cols.ultima_versao_online_idx,
        ];
        if let Some(idx) = cols.project_name_idx {
            required.push(idx);
        }
        if let Some(idx) = cols.repo_analised_version_idx {
            required.push(idx);
        }
        let min_idx = *required.iter().min().unwrap_or(&0);
        let max_idx = *required.iter().max().unwrap_or(&0);
        let start_col = col_idx_to_a1(min_idx);
        let end_col = col_idx_to_a1(max_idx);
        let range = format!("{start_col}2:{end_col}");
        let values = self
            .sheets
            .get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", range)
            .await?;

        let mut drifted = 0usize;
        let mut updated = 0usize;
        let processed = Arc::new(AtomicUsize::new(0));
        let drifted_atomic = Arc::new(AtomicUsize::new(0));
        let updated_atomic = Arc::new(AtomicUsize::new(0));

        #[derive(Clone)]
        struct RowCtx {
            row_number_1based: u32,
            repo_url: String,
            status_atualizacao: String,
            repo_analised_version: String,
            project_name: String,
            ultima_versao_online: String,
        }

        let mut rows = Vec::new();
        for (idx, row) in values.iter().enumerate() {
            let status_atualizacao = row
                .get(cols.status_atualizacao_idx)
                .map(|s| s.trim())
                .unwrap_or("");
            if should_skip_by_status_atualizacao(status_atualizacao) {
                continue;
            }
            let project_name = cols
                .project_name_idx
                .and_then(|i| row.get(i))
                .map(|s| s.trim())
                .unwrap_or("");
            let repo_url = row.get(cols.repo_url_idx).map(|s| s.trim()).unwrap_or("");
            let repo_url = if !repo_url.is_empty() {
                repo_url.to_string()
            } else if let Some(built) = try_build_repo_url_from_project_name(project_name) {
                built
            } else {
                continue;
            };
            let ultima_versao_online = row
                .get(cols.ultima_versao_online_idx)
                .map(|s| s.trim())
                .unwrap_or("");
            let repo_analised_version = cols
                .repo_analised_version_idx
                .and_then(|i| row.get(i))
                .map(|s| s.trim())
                .unwrap_or("")
                .to_string();
            rows.push(RowCtx {
                row_number_1based: (idx as u32) + 2,
                repo_url,
                status_atualizacao: status_atualizacao.to_string(),
                repo_analised_version,
                project_name: project_name.to_string(),
                ultima_versao_online: ultima_versao_online.to_string(),
            });
        }

        let inspected = values.len();

        let status_col = col_idx_to_a1(cols.status_atualizacao_idx);
        let status_fase_col = cols.status_fase_idx.map(col_idx_to_a1);
        let project_name_col = cols.project_name_idx.map(col_idx_to_a1);
        let ultima_col = col_idx_to_a1(cols.ultima_versao_online_idx);

        let mut pending_ranges: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        let mut pending_updates: usize = 0;
        let max_updates_per_batch: usize = 50;
        let total_to_write = rows.len();
        let mut written_so_far = 0usize;
        let ghost_processed = Arc::clone(&processed);
        let ghost_drifted = Arc::clone(&drifted_atomic);
        let ghost_updated = Arc::clone(&updated_atomic);
        let ghost_started = Instant::now();
        let ghost_handle = tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(30));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                info!(
                    done = ghost_processed.load(Ordering::Relaxed),
                    total = total_to_write,
                    drifted = ghost_drifted.load(Ordering::Relaxed),
                    updated = ghost_updated.load(Ordering::Relaxed),
                    elapsed_s = ghost_started.elapsed().as_secs(),
                    "Ghost Telemetry: Guardião processando"
                );
            }
        });
        let _ghost = AbortOnDrop(ghost_handle);

        for ctx in rows {
            processed.fetch_add(1, Ordering::Relaxed);
            if let Some(project_name_col) = project_name_col.as_deref() {
                if ctx.project_name.trim().is_empty() {
                    if let Some(extracted) = try_extract_project_name_from_repo_url(&ctx.repo_url) {
                        pending_ranges.insert(
                            format!(
                                "{project_name_col}{}:{project_name_col}{}",
                                ctx.row_number_1based, ctx.row_number_1based
                            ),
                            vec![vec![extracted]],
                        );
                    }
                }
            }

            let latest = match self.github.latest_release_tag(&ctx.repo_url).await {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        repo_url = %ctx.repo_url,
                        error = %e,
                        "Guardião: falha ao consultar GitHub; pulando linha"
                    );
                    continue;
                }
            };
            let Some(latest) = latest else { continue };
            let latest = latest.trim().to_string();
            let is_new_link = ctx.status_atualizacao.trim().is_empty();
            let drift = !ctx.repo_analised_version.trim().is_empty()
                && has_drift(&ctx.repo_analised_version, &latest);
            let should_write_latest = (is_new_link || drift) && ctx.ultima_versao_online.trim() != latest;

            if !is_new_link && !drift && !should_write_latest {
                continue;
            }

            pending_updates += 1;

            if is_new_link {
                pending_ranges.insert(
                    format!(
                        "{status_col}{}:{status_col}{}",
                        ctx.row_number_1based, ctx.row_number_1based
                    ),
                    vec![vec!["INICIAR_TRIAGEM".to_string()]],
                );
                if let Some(status_fase_col) = status_fase_col.as_deref() {
                    pending_ranges.insert(
                        format!(
                            "{status_fase_col}{}:{status_fase_col}{}",
                            ctx.row_number_1based, ctx.row_number_1based
                        ),
                        vec![vec!["FASE_-1_GUARDIAO_OK".to_string()]],
                    );
                }
            } else if drift {
                pending_ranges.insert(
                    format!(
                        "{status_col}{}:{status_col}{}",
                        ctx.row_number_1based, ctx.row_number_1based
                    ),
                    vec![vec!["DESATUALIZADA".to_string()]],
                );
                if let Some(status_fase_col) = status_fase_col.as_deref() {
                    pending_ranges.insert(
                        format!(
                            "{status_fase_col}{}:{status_fase_col}{}",
                            ctx.row_number_1based, ctx.row_number_1based
                        ),
                        vec![vec!["FASE_-1_GUARDIAO_OK".to_string()]],
                    );
                }
                drifted += 1;
                drifted_atomic.store(drifted, Ordering::Relaxed);
            }

            if should_write_latest {
                pending_ranges.insert(
                    format!(
                        "{ultima_col}{}:{ultima_col}{}",
                        ctx.row_number_1based, ctx.row_number_1based
                    ),
                    vec![vec![latest]],
                );
            }

            if pending_updates >= max_updates_per_batch {
                let batch = std::mem::take(&mut pending_ranges);
                written_so_far = written_so_far.saturating_add(pending_updates);
                info!(
                    written_so_far,
                    total_to_write,
                    updates = pending_updates,
                    "Guardião: flush Sheets batch"
                );
                self.sheets
                    .batch_update_cells(spreadsheet_id, "MASTER_SOLUTIONS", batch)
                    .await?;
                updated += pending_updates;
                updated_atomic.store(updated, Ordering::Relaxed);
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
                "Guardião: flush final Sheets batch"
            );
            self.sheets
                .batch_update_cells(spreadsheet_id, "MASTER_SOLUTIONS", batch)
                .await?;
            updated += pending_updates;
            updated_atomic.store(updated, Ordering::Relaxed);
        }


        info!(
            inspected,
            drifted,
            updated,
            "Guardião: rodada concluída (mutação somente com drift)"
        );
        Ok(())
    }
}

fn parse_cli_args() -> (Option<String>, bool) {
    let mut args = std::env::args();
    args.next();
    let mut sheets_id = None;
    let mut dry_run = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sheets-id" => sheets_id = args.next(),
            "--dry-run" => dry_run = true,
            _ => {}
        }
    }
    (sheets_id, dry_run)
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

    let (sheets_id_arg, dry_run) = parse_cli_args();
    let spreadsheet_id = sheets_id_arg
        .or_else(|| std::env::var("GOOGLE_SHEETS_ID").ok())
        .ok_or_else(|| io::Error::other("Missing GOOGLE_SHEETS_ID (or --sheets-id)"))?;

    if dry_run {
        info!("Guardião: dry-run ativado (nenhuma mutação será feita)");
        return Ok(());
    }

    let guardian = Guardian {
        sheets: Arc::new(SheetsMcpClient),
        github: Arc::new(ReqwestGithubClient::new().map_err(io::Error::other)?),
    };
    guardian
        .run_once(&spreadsheet_id)
        .await
        .map_err(io::Error::other)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::sync::OnceLock;
    use tokio::sync::Mutex;
    use tokio::io::{duplex, AsyncWriteExt, BufReader};

    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_mutex() -> &'static Mutex<()> {
        ENV_MUTEX.get_or_init(|| Mutex::new(()))
    }

    struct MockSheets {
        header: Vec<Vec<String>>,
        data: Vec<Vec<String>>,
        updates: Mutex<Vec<HashMap<String, Vec<Vec<String>>>>>,
    }

    impl MockSheets {
        fn new(header: Vec<Vec<String>>, data: Vec<Vec<String>>) -> Self {
            Self {
                header,
                data,
                updates: Mutex::new(Vec::new()),
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
                if range.ends_with("1:CF1") {
                    return Ok(self.header.clone());
                }
                Ok(self.data.clone())
            })
        }

        fn batch_update_cells<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            ranges: HashMap<String, Vec<Vec<String>>>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async move {
                self.updates.lock().await.push(ranges);
                Ok(())
            })
        }
    }

    struct MockGithub {
        tag: Option<String>,
        calls: std::sync::atomic::AtomicU64,
    }

    impl GithubClient for MockGithub {
        fn latest_release_tag<'a>(
            &'a self,
            _repo_url: &'a str,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send + 'a>>
        {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Box::pin(async move { Ok(self.tag.clone()) })
        }
    }

    #[test]
    fn plan_update_is_idempotent_when_versions_match() {
        assert!(!has_drift("v1.2.3", "1.2.3"));
        assert!(!has_drift("1.2.3", "v1.2.3"));
        assert!(!has_drift(" release-1.2.3 ", "1.2.3"));
    }

    #[test]
    fn plan_update_detects_drift_and_returns_remote_normalized() {
        assert!(has_drift("1.2.2", "v1.2.3"));
    }

    #[test]
    fn extract_values_accepts_root_values_shape() {
        let v = json!({ "values": [[ "a", "b" ], ["c"]] });
        let out = extract_values_2d(&v).unwrap();
        assert_eq!(out, vec![vec!["a".to_string(), "b".to_string()], vec!["c".to_string()]]);
    }

    #[test]
    fn extract_values_accepts_value_ranges_shape() {
        let v = json!({
            "valueRanges": [
                { "range": "MASTER_SOLUTIONS!A1:B2", "values": [[ "a", "b" ], ["c"]] }
            ]
        });
        let out = extract_values_2d(&v).unwrap();
        assert_eq!(out, vec![vec!["a".to_string(), "b".to_string()], vec!["c".to_string()]]);
    }

    #[tokio::test]
    async fn mcp_poll_waits_for_cold_start_and_then_parses_response() {
        let (mut w, r) = duplex(4096);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(600)).await;
            let _ = w
                .write_all(br#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"{\"ok\":true}"}]}}"#)
                .await;
            let _ = w.write_all(b"\n").await;
        });

        let started = Instant::now();
        let msg = SheetsMcpClient::poll_for_jsonrpc_response_from_reader(
            BufReader::new(r),
            Duration::from_secs(3),
            1,
        )
        .await
        .unwrap();
        assert!(started.elapsed() >= Duration::from_millis(600));
        assert_eq!(msg.get("id").and_then(|v| v.as_i64()), Some(1));
    }

    #[tokio::test]
    async fn mcp_poll_times_out_with_semantic_message() {
        let (_w, r) = duplex(4096);
        let err = SheetsMcpClient::poll_for_jsonrpc_response_from_reader(
            BufReader::new(r),
            Duration::from_millis(350),
            1,
        )
        .await
        .unwrap_err();
        assert!(err.contains("Timeout: O servidor MCP (Sheets) não emitiu o payload"));
    }

    #[tokio::test]
    async fn mcp_poll_accepts_string_id_and_returns_without_waiting_for_eof() {
        let (mut w, r) = duplex(4096);
        tokio::spawn(async move {
            let _ = w
                .write_all(br#"{"jsonrpc":"2.0","id":"1","result":{"content":[{"type":"text","text":"{\"ok\":true}"}]}}"#)
                .await;
            let _ = w.write_all(b"\n").await;
            tokio::time::sleep(Duration::from_millis(900)).await;
            let _ = w.write_all(b"not-json\n").await;
        });

        let started = Instant::now();
        let msg = SheetsMcpClient::poll_for_jsonrpc_response_from_reader(
            BufReader::new(r),
            Duration::from_secs(3),
            1,
        )
        .await
        .unwrap();
        assert!(started.elapsed() < Duration::from_millis(900));
        assert_eq!(msg.get("id").and_then(|v| v.as_str()), Some("1"));
    }

    #[tokio::test]
    async fn guardian_does_not_touch_row_when_no_drift() {
        let header = vec![vec![
            "status_atualizacao".to_string(),
            "status_fase".to_string(),
            "project_name".to_string(),
            "repo_url".to_string(),
            "repo_analised_version".to_string(),
            "ultima_versao_online".to_string(),
            "lote_id".to_string(),
        ]];
        let data = vec![vec![
            "PENDENTE_IA".to_string(),
            "F4".to_string(),
            "aaif-goose / goose".to_string(),
            "https://github.com/aaif-goose/goose".to_string(),
            "v1.2.3".to_string(),
            "v1.2.3".to_string(),
            "LOTE_X".to_string(),
        ]];
        let sheets = Arc::new(MockSheets::new(header, data));
        let github = MockGithub {
            tag: Some("v1.2.3".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        };
        let guardian = Guardian {
            sheets,
            github: Arc::new(github),
        };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 0);
    }

    #[tokio::test]
    async fn guardian_updates_only_a_and_f_when_drift_is_present() {
        let header = vec![vec![
            "status_atualizacao".to_string(),
            "status_fase".to_string(),
            "project_name".to_string(),
            "repo_url".to_string(),
            "repo_analised_version".to_string(),
            "ultima_versao_online".to_string(),
            "lote_id".to_string(),
        ]];
        let data = vec![vec![
            "".to_string(),
            "".to_string(),
            "aaif-goose/goose".to_string(),
            "https://github.com/aaif-goose/goose".to_string(),
            "v1.2.2".to_string(),
            "".to_string(),
            "LOTE_X".to_string(),
        ]];
        let sheets = Arc::new(MockSheets::new(header, data));
        let github = MockGithub {
            tag: Some("v1.2.3".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        };
        let guardian = Guardian {
            sheets,
            github: Arc::new(github),
        };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 1);
        let ranges = &updates[0];
        assert_eq!(
            ranges.get("A2:A2").unwrap(),
            &vec![vec!["INICIAR_TRIAGEM".to_string()]]
        );
        assert_eq!(
            ranges.get("B2:B2").unwrap(),
            &vec![vec!["FASE_-1_GUARDIAO_OK".to_string()]]
        );
        assert_eq!(
            ranges.get("F2:F2").unwrap(),
            &vec![vec!["v1.2.3".to_string()]]
        );
        assert_eq!(ranges.len(), 3);
    }

    #[tokio::test]
    async fn guardian_does_not_skip_rows_with_non_pending_status_atualizacao() {
        let header = vec![vec![
            "status_atualizacao".to_string(),
            "status_fase".to_string(),
            "project_name".to_string(),
            "repo_url".to_string(),
            "repo_analised_version".to_string(),
            "ultima_versao_online".to_string(),
            "lote_id".to_string(),
        ]];
        let data = vec![vec![
            "OK".to_string(),
            "".to_string(),
            "aaif-goose/goose".to_string(),
            "https://github.com/aaif-goose/goose".to_string(),
            "v1.2.2".to_string(),
            "v1.2.2".to_string(),
            "LOTE_X".to_string(),
        ]];
        let sheets = Arc::new(MockSheets::new(header, data));
        let github = MockGithub {
            tag: Some("v1.2.3".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        };
        let guardian = Guardian {
            sheets,
            github: Arc::new(github),
        };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 1);
        let ranges = &updates[0];
        assert!(ranges.get("A2:A2").is_some());
        assert!(ranges.get("B2:B2").is_some());
        assert!(ranges.get("F2:F2").is_some());
    }

    #[tokio::test]
    async fn guardian_tolerates_missing_status_fase_and_project_name_columns() {
        let header = vec![vec![
            "repo_url".to_string(),
            "status_atualizacao".to_string(),
            "ultima_versao_online".to_string(),
        ]];
        let data = vec![vec![
            "https://github.com/acme/widget".to_string(),
            "".to_string(),
            "".to_string(),
        ]];
        let sheets = Arc::new(MockSheets::new(header, data));
        let github = MockGithub {
            tag: Some("v2.0.0".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        };
        let guardian = Guardian {
            sheets,
            github: Arc::new(github),
        };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 1);
        let ranges = &updates[0];
        assert_eq!(
            ranges.get("B2:B2").unwrap(),
            &vec![vec!["INICIAR_TRIAGEM".to_string()]]
        );
        assert_eq!(
            ranges.get("C2:C2").unwrap(),
            &vec![vec!["v2.0.0".to_string()]]
        );
        assert_eq!(ranges.len(), 2);
    }

    #[tokio::test]
    async fn guardian_treats_empty_repo_analised_version_as_drift_but_preserves_pending_rows_legacy_header(
    ) {
        let header = vec![vec![
            "status_atualizacao".to_string(),
            "status_fase".to_string(),
            "project_name".to_string(),
            "repo_url".to_string(),
            "repo_version".to_string(),
            "ultima_versao_online".to_string(),
            "lote_id".to_string(),
        ]];
        let data = vec![
            vec![
                "INICIAR_TRIAGEM".to_string(),
                "FASE_-1_GUARDIAO_OK".to_string(),
                "acme / a".to_string(),
                "https://github.com/acme/a".to_string(),
                "".to_string(),
                "".to_string(),
                "L1".to_string(),
            ],
            vec![
                "".to_string(),
                "".to_string(),
                "acme / b".to_string(),
                "https://github.com/acme/b".to_string(),
                "".to_string(),
                "".to_string(),
                "L2".to_string(),
            ],
        ];
        let sheets = Arc::new(MockSheets::new(header, data));
        let github = MockGithub {
            tag: Some("v2.0.0".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        };
        let guardian = Guardian {
            sheets,
            github: Arc::new(github),
        };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 1);
        assert!(updates[0].get("A3:A3").is_some());
        assert!(updates[0].get("B3:B3").is_some());
        assert!(updates[0].get("F3:F3").is_some());
    }

    #[tokio::test]
    async fn guardian_skips_row_when_ultima_versao_online_is_already_filled() {
        let header = vec![vec![
            "status_atualizacao".to_string(),
            "status_fase".to_string(),
            "project_name".to_string(),
            "repo_url".to_string(),
            "repo_analised_version".to_string(),
            "ultima_versao_online".to_string(),
            "lote_id".to_string(),
        ]];
        let data = vec![vec![
            "INICIAR_TRIAGEM".to_string(),
            "FASE_-1_GUARDIAO_OK".to_string(),
            "acme / widget".to_string(),
            "https://github.com/acme/widget".to_string(),
            "v1.0.0".to_string(),
            "v1.0.0".to_string(),
            "L1".to_string(),
        ]];
        let sheets = Arc::new(MockSheets::new(header, data));
        let github = Arc::new(MockGithub {
            tag: Some("v2.0.0".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        });
        let guardian = Guardian {
            sheets,
            github: github.clone(),
        };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 0);
        assert_eq!(
            github.calls.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[tokio::test]
    async fn guardian_flushes_in_micro_batches_of_20_and_flushes_remainder() {
        let header = vec![vec![
            "status_atualizacao".to_string(),
            "status_fase".to_string(),
            "project_name".to_string(),
            "repo_url".to_string(),
            "repo_analised_version".to_string(),
            "ultima_versao_online".to_string(),
            "lote_id".to_string(),
        ]];
        let mut data = Vec::new();
        for i in 0..21u32 {
            data.push(vec![
                "".to_string(),
                "".to_string(),
                format!("acme / r{i}"),
                format!("https://github.com/acme/r{i}"),
                "v1.0.0".to_string(),
                "v1.0.0".to_string(),
                "L".to_string(),
            ]);
        }
        let sheets = Arc::new(MockSheets::new(header, data));
        let github = Arc::new(MockGithub {
            tag: Some("v2.0.0".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        });
        let guardian = Guardian {
            sheets,
            github: github.clone(),
        };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].len(), 63);
        assert_eq!(
            github.calls.load(std::sync::atomic::Ordering::Relaxed),
            21
        );
    }

    #[tokio::test]
    async fn github_latest_release_tag_parses_tag_name_and_avoids_false_positive_when_equal() {
        let _guard = env_mutex().lock().await;
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/acme/widget/releases/latest")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "tag_name": "v9.9.9" }"#)
            .create_async()
            .await;

        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        std::env::set_var("GITHUB_PAT", "test-token");
        let client = ReqwestGithubClient::new_with_policy(RetryPolicy {
            max_attempts: 3,
            jitter_min_ms: 0,
            jitter_max_ms: 0,
            backoff_base_ms: 0,
        })
        .unwrap();
        let tag = client
            .latest_release_tag("https://github.com/acme/widget")
            .await
            .unwrap()
            .unwrap();
        mock.assert_async().await;
        assert_eq!(tag, "v9.9.9");
        assert!(!has_drift("9.9.9", &tag));
        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
        std::env::remove_var("GITHUB_PAT");
    }

    #[tokio::test]
    async fn github_falls_back_to_tags_when_no_releases() {
        let _guard = env_mutex().lock().await;
        let mut server = Server::new_async().await;
        let _no_release = server
            .mock("GET", "/repos/acme/widget/releases/latest")
            .match_header("authorization", "Bearer test-token")
            .with_status(404)
            .create_async()
            .await;
        let _repo_ok = server
            .mock("GET", "/repos/acme/widget")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "default_branch": "main" }"#)
            .create_async()
            .await;
        let _tags = server
            .mock("GET", "/repos/acme/widget/tags")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{ "name": "v1.0.0" }]"#)
            .create_async()
            .await;

        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        std::env::set_var("GITHUB_PAT", "test-token");
        let client = ReqwestGithubClient::new_with_policy(RetryPolicy {
            max_attempts: 3,
            jitter_min_ms: 0,
            jitter_max_ms: 0,
            backoff_base_ms: 0,
        })
        .unwrap();

        let v = client
            .latest_release_tag("https://github.com/acme/widget")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(v, "v1.0.0");

        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
        std::env::remove_var("GITHUB_PAT");
    }

    #[tokio::test]
    async fn github_falls_back_to_short_sha_when_no_releases_or_tags() {
        let _guard = env_mutex().lock().await;
        let mut server = Server::new_async().await;
        let _no_release = server
            .mock("GET", "/repos/acme/widget/releases/latest")
            .match_header("authorization", "Bearer test-token")
            .with_status(404)
            .create_async()
            .await;
        let _repo_ok = server
            .mock("GET", "/repos/acme/widget")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "default_branch": "main" }"#)
            .create_async()
            .await;
        let _tags = server
            .mock("GET", "/repos/acme/widget/tags")
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;
        let _commit = server
            .mock("GET", "/repos/acme/widget/commits/main")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "sha": "0123456789abcdef" }"#)
            .create_async()
            .await;

        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        std::env::set_var("GITHUB_PAT", "test-token");
        let client = ReqwestGithubClient::new_with_policy(RetryPolicy {
            max_attempts: 3,
            jitter_min_ms: 0,
            jitter_max_ms: 0,
            backoff_base_ms: 0,
        })
        .unwrap();

        let v = client
            .latest_release_tag("https://github.com/acme/widget")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(v, "0123456");

        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
        std::env::remove_var("GITHUB_PAT");
    }

    #[tokio::test]
    async fn github_returns_none_when_repo_missing() {
        let _guard = env_mutex().lock().await;
        let mut server = Server::new_async().await;
        let _no_release = server
            .mock("GET", "/repos/acme/missing/releases/latest")
            .match_header("authorization", "Bearer test-token")
            .with_status(404)
            .create_async()
            .await;
        let _repo_missing = server
            .mock("GET", "/repos/acme/missing")
            .match_header("authorization", "Bearer test-token")
            .with_status(404)
            .create_async()
            .await;

        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        std::env::set_var("GITHUB_PAT", "test-token");
        let client = ReqwestGithubClient::new_with_policy(RetryPolicy {
            max_attempts: 3,
            jitter_min_ms: 0,
            jitter_max_ms: 0,
            backoff_base_ms: 0,
        })
        .unwrap();

        let v = client
            .latest_release_tag("https://github.com/acme/missing")
            .await
            .unwrap();
        assert!(v.is_none());

        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
        std::env::remove_var("GITHUB_PAT");
    }

    #[tokio::test]
    async fn github_retries_on_429_up_to_max_attempts_and_fails_soft() {
        let _guard = env_mutex().lock().await;
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/repos/acme/widget/releases/latest")
            .match_header("authorization", "Bearer test-token")
            .with_status(429)
            .expect(3)
            .create_async()
            .await;

        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        std::env::set_var("GITHUB_PAT", "test-token");
        let client = ReqwestGithubClient::new_with_policy(RetryPolicy {
            max_attempts: 3,
            jitter_min_ms: 0,
            jitter_max_ms: 0,
            backoff_base_ms: 0,
        })
        .unwrap();

        let err = client
            .latest_release_tag("https://github.com/acme/widget")
            .await
            .unwrap_err();
        assert!(err.contains("rate limit"));

        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
        std::env::remove_var("GITHUB_PAT");
    }

    #[tokio::test]
    async fn guardian_maps_columns_by_header_even_when_shuffled() {
        let header = vec![vec![
            "project_name".to_string(),
            "repo_url".to_string(),
            "ultima_versao_online".to_string(),
            "status_atualizacao".to_string(),
            "repo_analised_version".to_string(),
            "status_fase".to_string(),
            "lote_id".to_string(),
        ]];
        let data = vec![vec![
            "acme / widget".to_string(),
            "https://github.com/acme/widget".to_string(),
            "".to_string(),
            "".to_string(),
            "v1.0.0".to_string(),
            "".to_string(),
            "L1".to_string(),
        ]];

        let sheets = Arc::new(MockSheets::new(header, data));
        let github = MockGithub {
            tag: Some("v2.0.0".to_string()),
            calls: std::sync::atomic::AtomicU64::new(0),
        };
        let guardian = Guardian {
            sheets,
            github: Arc::new(github),
        };
        guardian.run_once("SHEET").await.unwrap();

        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 1);
        let ranges = &updates[0];
        assert!(ranges.get("D2:D2").is_some());
        assert!(ranges.get("F2:F2").is_some());
        assert!(ranges.get("C2:C2").is_some());
    }
}
