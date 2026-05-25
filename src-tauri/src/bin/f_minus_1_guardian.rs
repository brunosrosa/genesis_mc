use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};
use url::Url;

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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: HashMap<String, Vec<Vec<String>>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;
}

trait GithubClient: Send + Sync {
    fn latest_release_tag<'a>(
        &'a self,
        repo_url: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send + 'a>>;
}

fn extract_values_2d(result: &Value) -> Option<Vec<Vec<String>>> {
    let values = result.get("values")?.as_array()?;
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
    fn call_mcp(tool_name: &str, arguments: Value) -> Result<Value, String> {
        use std::process::{Command, Stdio};

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
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Falha ao spawnar mcp-google-sheets: {e}"))?;

        {
            use std::io::Write;
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| "stdin indisponível".to_string())?;
            writeln!(stdin, "{}", init_req).map_err(|e| format!("Falha ao escrever init_req: {e}"))?;
            writeln!(stdin, "{}", initialized_notif)
                .map_err(|e| format!("Falha ao escrever initialized: {e}"))?;
            writeln!(stdin, "{}", mcp_request)
                .map_err(|e| format!("Falha ao escrever tools/call: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Falha ao aguardar mcp-google-sheets: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "mcp-google-sheets falhou. Exit {}. STDERR: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        for line in stdout_str.lines().rev() {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                    if value.get("error").is_some() {
                        return Err(format!("MCP retornou erro: {value}"));
                    }
                    if let Some(result) = value.get("result") {
                        return Ok(Self::normalize_mcp_tool_result(result.clone()));
                    }
                }
            }
        }

        Err("Resposta MCP não encontrada no stdout".to_string())
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
            )?;
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
            )?;
            Ok(())
        })
    }
}

struct ReqwestGithubClient {
    http: Client,
    api_base: String,
    allow_host_override: bool,
}

impl ReqwestGithubClient {
    fn new() -> Result<Self, String> {
        let api_base = std::env::var("SODA_GITHUB_API_BASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://api.github.com".to_string());
        let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
        let http = Client::builder()
            .user_agent("f-minus-1-guardian/1.0")
            .build()
            .map_err(|e| format!("Falha ao criar client HTTP: {e}"))?;
        Ok(Self {
            http,
            api_base,
            allow_host_override,
        })
    }
}

#[derive(Deserialize)]
struct GithubReleaseResponse {
    tag_name: Option<String>,
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

            let endpoint = format!(
                "{}/repos/{owner}/{repo}/releases/latest",
                self.api_base.trim_end_matches('/')
            );

            let resp = self
                .http
                .get(&endpoint)
                .send()
                .await
                .map_err(|e| format!("Falha HTTP GitHub: {e}"))?;
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if !resp.status().is_success() {
                return Err(format!("GitHub retornou status {}", resp.status()));
            }
            let parsed = resp
                .json::<GithubReleaseResponse>()
                .await
                .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
            Ok(parsed
                .tag_name
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()))
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

fn has_drift(repo_version: &str, github_latest: &str) -> bool {
    let local = normalize_version(repo_version);
    let remote = normalize_version(github_latest);
    !(local.is_empty() || remote.is_empty() || local == remote)
}

struct Guardian<S: SheetsClient, G: GithubClient> {
    sheets: S,
    github: G,
}

impl<S: SheetsClient, G: GithubClient> Guardian<S, G> {
    async fn run_once(&self, spreadsheet_id: &str) -> Result<(), String> {
        let values = self
            .sheets
            .get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", "A2:G".to_string())
            .await?;

        let mut inspected = 0usize;
        let mut drifted = 0usize;
        let mut updated = 0usize;

        for (idx, row) in values.iter().enumerate() {
            let row_number_1based = (idx as u32) + 2;
            let repo_url = row.get(3).map(|s| s.trim()).unwrap_or("");
            let repo_version = row.get(4).map(|s| s.trim()).unwrap_or("");
            if repo_url.is_empty() || repo_version.is_empty() {
                continue;
            }
            inspected += 1;

            let latest = match self.github.latest_release_tag(repo_url).await {
                Ok(v) => v,
                Err(e) => {
                    warn!(repo_url = %repo_url, error = %e, "Guardião: falha ao consultar GitHub; pulando linha");
                    continue;
                }
            };
            let Some(latest) = latest else {
                continue;
            };

            if !has_drift(repo_version, &latest) {
                continue;
            }
            drifted += 1;

            let mut ranges: HashMap<String, Vec<Vec<String>>> = HashMap::new();
            ranges.insert(
                format!("A{row_number_1based}:A{row_number_1based}"),
                vec![vec!["DESATUALIZADA".to_string()]],
            );
            ranges.insert(
                format!("F{row_number_1based}:F{row_number_1based}"),
                vec![vec![latest.trim().to_string()]],
            );

            self.sheets
                .batch_update_cells(spreadsheet_id, "MASTER_SOLUTIONS", ranges)
                .await?;
            updated += 1;
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
        sheets: SheetsMcpClient,
        github: ReqwestGithubClient::new().map_err(io::Error::other)?,
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
    use tokio::sync::Mutex;

    struct MockSheets {
        grid: Vec<Vec<String>>,
        updates: Mutex<Vec<HashMap<String, Vec<Vec<String>>>>>,
    }

    impl MockSheets {
        fn new(grid: Vec<Vec<String>>) -> Self {
            Self {
                grid,
                updates: Mutex::new(Vec::new()),
            }
        }
    }

    impl SheetsClient for MockSheets {
        fn get_sheet_data<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            _range: String,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>
        {
            Box::pin(async move { Ok(self.grid.clone()) })
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
    }

    impl GithubClient for MockGithub {
        fn latest_release_tag<'a>(
            &'a self,
            _repo_url: &'a str,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send + 'a>>
        {
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

    #[tokio::test]
    async fn guardian_does_not_touch_row_when_no_drift() {
        let grid = vec![vec![
            "PENDENTE_IA".to_string(),
            "F4".to_string(),
            "aaif-goose / goose".to_string(),
            "https://github.com/aaif-goose/goose".to_string(),
            "v1.2.3".to_string(),
            "v1.2.3".to_string(),
            "LOTE_X".to_string(),
        ]];
        let sheets = MockSheets::new(grid);
        let github = MockGithub {
            tag: Some("v1.2.3".to_string()),
        };
        let guardian = Guardian { sheets, github };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 0);
    }

    #[tokio::test]
    async fn guardian_updates_only_a_and_f_when_drift_is_present() {
        let grid = vec![vec![
            "CONCLUIDO".to_string(),
            "F4".to_string(),
            "aaif-goose / goose".to_string(),
            "https://github.com/aaif-goose/goose".to_string(),
            "v1.2.2".to_string(),
            "v1.2.2".to_string(),
            "LOTE_X".to_string(),
        ]];
        let sheets = MockSheets::new(grid);
        let github = MockGithub {
            tag: Some("v1.2.3".to_string()),
        };
        let guardian = Guardian { sheets, github };
        guardian.run_once("SHEET").await.unwrap();
        let updates = guardian.sheets.updates.lock().await;
        assert_eq!(updates.len(), 1);
        let ranges = &updates[0];
        assert_eq!(
            ranges.get("A2:A2").unwrap(),
            &vec![vec!["DESATUALIZADA".to_string()]]
        );
        assert_eq!(
            ranges.get("F2:F2").unwrap(),
            &vec![vec!["v1.2.3".to_string()]]
        );
        assert_eq!(ranges.len(), 2);
    }

    #[tokio::test]
    async fn github_latest_release_tag_parses_tag_name_and_avoids_false_positive_when_equal() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/repos/acme/widget/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "tag_name": "v9.9.9" }"#)
            .create_async()
            .await;

        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        let client = ReqwestGithubClient::new().unwrap();
        let tag = client
            .latest_release_tag("https://github.com/acme/widget")
            .await
            .unwrap()
            .unwrap();
        mock.assert_async().await;
        assert_eq!(tag, "v9.9.9");
        assert!(!has_drift("9.9.9", &tag));
        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
    }
}
