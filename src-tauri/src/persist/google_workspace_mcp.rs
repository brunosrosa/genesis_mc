use std::env;
use std::process::{Command, Stdio};
use std::time::Duration;

use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const GOOGLE_SHEETS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    #[serde(default = "default_google_token_uri")]
    token_uri: String,
}

#[derive(Debug, Serialize)]
struct ServiceAccountClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

fn default_google_token_uri() -> String {
    "https://oauth2.googleapis.com/token".to_string()
}

fn token_from_env() -> Option<String> {
    for key in ["GOOGLE_ACCESS_TOKEN", "ACCESS_TOKEN"] {
        if let Ok(value) = env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn read_service_account_key() -> Result<ServiceAccountKey, String> {
    let creds_path = env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .map_err(|_| "Missing GOOGLE_APPLICATION_CREDENTIALS".to_string())?;
    let raw = std::fs::read_to_string(&creds_path)
        .map_err(|e| format!("Falha ao ler GOOGLE_APPLICATION_CREDENTIALS: {e}"))?;
    serde_json::from_str::<ServiceAccountKey>(&raw)
        .map_err(|e| format!("Falha ao parsear service account JSON: {e}"))
}

fn build_service_account_assertion(key: &ServiceAccountKey) -> Result<String, String> {
    let now = Utc::now().timestamp();
    let claims = ServiceAccountClaims {
        iss: &key.client_email,
        scope: GOOGLE_SHEETS_SCOPE,
        aud: &key.token_uri,
        iat: now,
        exp: now + 3600,
    };
    let encoding_key = EncodingKey::from_rsa_pem(key.private_key.as_bytes())
        .map_err(|e| format!("Falha ao carregar private_key RSA do service account: {e}"))?;
    encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)
        .map_err(|e| format!("Falha ao assinar JWT do service account: {e}"))
}

fn oauth_refresh_credentials() -> Option<(String, String, String)> {
    let client_id = env::var("GOOGLE_CLIENT_ID").ok()?.trim().to_string();
    let client_secret = env::var("GOOGLE_CLIENT_SECRET").ok()?.trim().to_string();
    let refresh_token = env::var("GOOGLE_REFRESH_TOKEN").ok()?.trim().to_string();
    if client_id.is_empty() || client_secret.is_empty() || refresh_token.is_empty() {
        return None;
    }
    Some((client_id, client_secret, refresh_token))
}

pub fn google_workspace_access_token_blocking() -> Result<String, String> {
    if let Some(token) = token_from_env() {
        return Ok(token);
    }

    if let Some((client_id, client_secret, refresh_token)) = oauth_refresh_credentials() {
        let response = reqwest::blocking::Client::new()
            .post("https://oauth2.googleapis.com/token")
            .json(&json!({
                "client_id": client_id,
                "client_secret": client_secret,
                "refresh_token": refresh_token,
                "grant_type": "refresh_token"
            }))
            .send()
            .map_err(|e| format!("Falha ao renovar Google OAuth token: {e}"))?;

        if !response.status().is_success() {
            let body = response.text().unwrap_or_else(|_| "unknown".to_string());
            return Err(format!("Falha ao renovar Google OAuth token: {body}"));
        }

        let payload: Value = response
            .json()
            .map_err(|e| format!("Falha ao parsear refresh token Google: {e}"))?;
        return payload
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| "OAuth refresh retornou access_token vazio".to_string());
    }

    let key = read_service_account_key()?;
    let assertion = build_service_account_assertion(&key)?;
    let response = reqwest::blocking::Client::new()
        .post(&key.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", assertion.as_str()),
        ])
        .send()
        .map_err(|e| format!("Falha ao trocar JWT do service account por access_token: {e}"))?;

    if !response.status().is_success() {
        let body = response.text().unwrap_or_else(|_| "unknown".to_string());
        return Err(format!(
            "Falha ao trocar JWT do service account por access_token: {body}"
        ));
    }

    let payload: Value = response
        .json()
        .map_err(|e| format!("Falha ao parsear resposta do service account: {e}"))?;
    payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Service account token exchange retornou access_token vazio".to_string())
}

pub async fn google_workspace_access_token_async() -> Result<String, String> {
    tokio::task::spawn_blocking(google_workspace_access_token_blocking)
        .await
        .map_err(|e| format!("Falha ao aguardar geração de token Google: {e}"))?
}

fn mcp_google_workspace_command() -> String {
    fn default_bin_name() -> &'static str {
        if cfg!(windows) {
            "mcp-google.exe"
        } else {
            "mcp-google"
        }
    }

    fn exists(path: &str) -> bool {
        std::fs::metadata(path).is_ok()
    }

    let configured = env::var("MCP_GOOGLE_WORKSPACE_BIN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let primary = configured.unwrap_or_else(|| default_bin_name().to_string());

    if primary.contains(['\\', '/', ':']) && exists(&primary) {
        return primary;
    }

    let packaged = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bin")
        .join(default_bin_name());
    if let Some(packaged_str) = packaged.to_str() {
        if exists(packaged_str) {
            return packaged_str.to_string();
        }
    }

    if let Ok(cargo_path) = env::var("SODA_CARGO_PATH") {
        let cargo_path = cargo_path.trim();
        if !cargo_path.is_empty() {
            let candidate = std::path::Path::new(cargo_path)
                .join(&primary)
                .to_string_lossy()
                .to_string();
            if exists(&candidate) {
                return candidate;
            }
        }
    }

    if let Ok(profile) = env::var("USERPROFILE") {
        let candidate = std::path::Path::new(profile.trim())
            .join(".cargo")
            .join("bin")
            .join(&primary)
            .to_string_lossy()
            .to_string();
        if exists(&candidate) {
            return candidate;
        }
    }
    if let Ok(home) = env::var("HOME") {
        let candidate = std::path::Path::new(home.trim())
            .join(".cargo")
            .join("bin")
            .join(&primary)
            .to_string_lossy()
            .to_string();
        if exists(&candidate) {
            return candidate;
        }
    }

    primary
}

pub fn normalize_mcp_tool_result(result: Value) -> Value {
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

pub fn extract_values_2d(result: &Value) -> Option<Vec<Vec<String>>> {
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

fn parse_mcp_response(stdout: &[u8], response_id: i64) -> Result<Value, String> {
    let stdout_str = String::from_utf8_lossy(stdout);
    for line in stdout_str.lines().rev() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if value.get("id").and_then(|v| v.as_i64()) == Some(response_id) {
                if value.get("error").is_some() {
                    return Err(format!("MCP retornou erro: {value}"));
                }
                if let Some(result) = value.get("result") {
                    return Ok(normalize_mcp_tool_result(result.clone()));
                }
            }
        }
    }
    Err("Resposta MCP não encontrada no stdout".to_string())
}

pub fn call_google_workspace_tool_blocking(
    tool_name: &str,
    arguments: Value,
    meta: Value,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": client_name, "version": "1.0.0" }
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
            "arguments": arguments,
            "_meta": meta
        }
    });

    let mut child = Command::new(mcp_google_workspace_command())
        .arg("sheets")
        .env("RUST_LOG", "off")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Falha ao spawnar mcp-google-workspace: {e}"))?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().ok_or_else(|| "stdin indisponível".to_string())?;
        writeln!(stdin, "{}", init_req).map_err(|e| format!("Falha ao escrever init no MCP: {e}"))?;
        writeln!(stdin, "{}", initialized_notif)
            .map_err(|e| format!("Falha ao escrever initialized no MCP: {e}"))?;
        writeln!(stdin, "{}", mcp_request)
            .map_err(|e| format!("Falha ao escrever tools/call no MCP: {e}"))?;
    }

    let started = std::time::Instant::now();
    loop {
        if let Some(_status) = child
            .try_wait()
            .map_err(|e| format!("Falha ao aguardar mcp-google-workspace: {e}"))?
        {
            break;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            return Err(format!(
                "Timeout aguardando mcp-google-workspace tool={} timeout_s={}",
                tool_name,
                timeout.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Falha ao aguardar mcp-google-workspace: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "mcp-google-workspace falhou. Exit {}. STDERR: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    parse_mcp_response(&output.stdout, 1)
}

pub async fn call_google_workspace_tool_async(
    tool_name: &str,
    arguments: Value,
    meta: Value,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let tool_name = tool_name.to_string();
    let client_name = client_name.to_string();
    tokio::task::spawn_blocking(move || {
        call_google_workspace_tool_blocking(&tool_name, arguments, meta, &client_name, timeout)
    })
    .await
    .map_err(|e| format!("Falha ao aguardar thread MCP Google Workspace: {e}"))?
}

pub fn call_legacy_sheets_tool_blocking(
    tool_name: &str,
    arguments: Value,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let spreadsheet_id = arguments
        .get("spreadsheet_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Missing spreadsheet_id".to_string())?
        .to_string();
    let access_token = google_workspace_access_token_blocking()?;
    let meta = json!({
        "spreadsheet_id": spreadsheet_id,
        "access_token": access_token
    });

    match tool_name {
        "get_sheet_data" => {
            let sheet = arguments
                .get("sheet")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing sheet".to_string())?;
            let range = arguments
                .get("range")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing range".to_string())?;
            call_google_workspace_tool_blocking(
                "read_values",
                json!({
                    "sheet": sheet,
                    "range": range,
                    "major_dimension": "ROWS"
                }),
                meta,
                client_name,
                timeout,
            )
        }
        "batch_update_cells" => {
            let sheet = arguments
                .get("sheet")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing sheet".to_string())?;
            let ranges = arguments
                .get("ranges")
                .and_then(|v| v.as_object())
                .ok_or_else(|| "Missing ranges".to_string())?;
            let mut updated_ranges = Vec::with_capacity(ranges.len());
            for (range, values) in ranges {
                call_google_workspace_tool_blocking(
                    "write_values",
                    json!({
                        "sheet": sheet,
                        "range": range,
                        "values": values,
                        "major_dimension": "ROWS"
                    }),
                    meta.clone(),
                    client_name,
                    timeout,
                )?;
                updated_ranges.push(range.clone());
            }
            Ok(json!({
                "ok": true,
                "updated_ranges": updated_ranges
            }))
        }
        other => Err(format!("Tool legado não suportado no bridge Rust: {other}")),
    }
}

pub async fn call_legacy_sheets_tool_async(
    tool_name: &str,
    arguments: Value,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let tool_name = tool_name.to_string();
    let client_name = client_name.to_string();
    tokio::task::spawn_blocking(move || {
        call_legacy_sheets_tool_blocking(&tool_name, arguments, &client_name, timeout)
    })
    .await
    .map_err(|e| format!("Falha ao aguardar thread bridge Google Workspace: {e}"))?
}
