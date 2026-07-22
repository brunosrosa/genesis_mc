use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const GOOGLE_SHEETS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";

// L14: Connection pooling para OAuth. Reutiliza conexões TCP/TLS em vez de criar
// um novo handshake a cada requisição de token.
fn oauth_http_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .pool_max_idle_per_host(3)
            .pool_idle_timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest Client deve inicializar")
    })
}

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
    let now = crate::telemetry::now_epoch_secs();
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
        let response = oauth_http_client()
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
    let response = oauth_http_client()
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
    fn default_bin_names() -> Vec<String> {
        if cfg!(windows) {
            vec![
                "mcp-google-x86_64-pc-windows-msvc.exe".to_string(),
                "mcp-google.exe".to_string(),
            ]
        } else {
            vec![
                "mcp-google".to_string(),
            ]
        }
    }

    fn exists(path: &str) -> bool {
        std::fs::metadata(path).is_ok()
    }

    let configured = env::var("MCP_GOOGLE_WORKSPACE_BIN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(ref primary) = configured {
        if primary.contains(['\\', '/', ':']) && exists(primary) {
            return primary.clone();
        }
    }

    // Tenta primeiro no diretório de empacotados bin/ do manifesto
    for name in default_bin_names() {
        let packaged = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("bin")
            .join(&name);
        if let Some(packaged_str) = packaged.to_str() {
            if exists(packaged_str) {
                return packaged_str.to_string();
            }
        }
    }

    // Tenta nos caminhos alternativos/cargo
    let primary = configured.unwrap_or_else(|| {
        if cfg!(windows) {
            "mcp-google.exe".to_string()
        } else {
            "mcp-google".to_string()
        }
    });

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

fn summarize_debug_pipe(bytes: &[u8]) -> String {
    const MAX_CHARS: usize = 1200;
    let rendered = String::from_utf8_lossy(bytes).replace('\r', "").replace('\n', " | ");
    if rendered.len() > MAX_CHARS {
        format!("{}...(truncated)", &rendered[..MAX_CHARS])
    } else {
        rendered
    }
}

fn summarize_seen_lines(lines: &[String]) -> String {
    summarize_debug_pipe(lines.join("\n").as_bytes())
}

fn spawn_line_reader<R>(reader: R) -> (mpsc::Receiver<String>, Arc<Mutex<Vec<String>>>)
where
    R: std::io::Read + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen_for_thread = Arc::clone(&seen);
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(mut guard) = seen_for_thread.lock() {
                guard.push(trimmed.clone());
            }
            let _ = tx.send(trimmed);
        }
    });
    (rx, seen)
}

fn wait_for_jsonrpc_response(
    rx: &mpsc::Receiver<String>,
    seen: &Arc<Mutex<Vec<String>>>,
    timeout: Duration,
    expected_id: i64,
    label: &str,
) -> Result<Value, String> {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > timeout {
            let snapshot = seen.lock().map(|g| g.clone()).unwrap_or_default();
            return Err(format!(
                "Timeout aguardando resposta JSON-RPC id={} em {} stdout={}",
                expected_id,
                label,
                summarize_seen_lines(&snapshot)
            ));
        }
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                let Ok(value) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let id_matches = match value.get("id") {
                    Some(Value::Number(n)) => n.as_i64() == Some(expected_id),
                    Some(Value::String(s)) => s.parse::<i64>().ok() == Some(expected_id),
                    _ => false,
                };
                if id_matches {
                    if value.get("error").is_some() {
                        return Err(format!("MCP retornou erro: {value}"));
                    }
                    return Ok(value);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let snapshot = seen.lock().map(|g| g.clone()).unwrap_or_default();
                return Err(format!(
                    "Pipe {} do mcp-google-workspace foi encerrado antes da resposta id={} stdout={}",
                    label,
                    expected_id,
                    summarize_seen_lines(&snapshot)
                ));
            }
        }
    }
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

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout indisponível".to_string())?;
    let (stdout_rx, stdout_seen) = spawn_line_reader(stdout);
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "stderr indisponível".to_string())?;
    let (_stderr_rx, _stderr_seen) = spawn_line_reader(stderr);

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "stdin indisponível".to_string())?;

    {
        use std::io::Write;
        if let Err(e) = writeln!(stdin, "{}", init_req) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Falha ao escrever init no MCP: {e}"));
        }
        if let Err(e) = stdin.flush() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Falha ao flush init no MCP: {e}"));
        }
    }
    let init_res = wait_for_jsonrpc_response(&stdout_rx, &stdout_seen, timeout, 0, "stdout-init");
    if let Err(e) = init_res {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }

    {
        use std::io::Write;
        if let Err(e) = writeln!(stdin, "{}", initialized_notif) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Falha ao escrever initialized no MCP: {e}"));
        }
        if let Err(e) = writeln!(stdin, "{}", mcp_request) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Falha ao escrever tools/call no MCP: {e}"));
        }
        if let Err(e) = stdin.flush() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Falha ao flush tools/call no MCP: {e}"));
        }
    }
    let tool_response =
        wait_for_jsonrpc_response(&stdout_rx, &stdout_seen, timeout, 1, "stdout-tools-call");
    let _ = child.kill();
    let _ = child.wait();
    let tool_response = tool_response?;
    let result = tool_response
        .get("result")
        .cloned()
        .ok_or_else(|| "Resposta MCP sem campo result".to_string())?;
    Ok(normalize_mcp_tool_result(result))
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

fn workspace_meta(spreadsheet_id: &str, access_token: &str) -> Value {
    json!({
        "spreadsheet_id": spreadsheet_id,
        "access_token": access_token
    })
}

pub fn read_values_blocking(
    spreadsheet_id: &str,
    sheet: &str,
    range: &str,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let access_token = google_workspace_access_token_blocking()?;
    call_google_workspace_tool_blocking(
        "read_values",
        json!({
            "sheet": sheet,
            "range": range,
            "major_dimension": "ROWS"
        }),
        workspace_meta(spreadsheet_id, &access_token),
        client_name,
        timeout,
    )
}

pub async fn read_values_async(
    spreadsheet_id: &str,
    sheet: &str,
    range: &str,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let access_token = google_workspace_access_token_async().await?;
    call_google_workspace_tool_async(
        "read_values",
        json!({
            "sheet": sheet,
            "range": range,
            "major_dimension": "ROWS"
        }),
        workspace_meta(spreadsheet_id, &access_token),
        client_name,
        timeout,
    )
    .await
}

pub fn write_values_blocking(
    spreadsheet_id: &str,
    sheet: &str,
    range: &str,
    values: Value,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let access_token = google_workspace_access_token_blocking()?;
    call_google_workspace_tool_blocking(
        "write_values",
        json!({
            "sheet": sheet,
            "range": range,
            "values": values,
            "major_dimension": "ROWS"
        }),
        workspace_meta(spreadsheet_id, &access_token),
        client_name,
        timeout,
    )
}

pub async fn write_values_async(
    spreadsheet_id: &str,
    sheet: &str,
    range: &str,
    values: Value,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let access_token = google_workspace_access_token_async().await?;
    call_google_workspace_tool_async(
        "write_values",
        json!({
            "sheet": sheet,
            "range": range,
            "values": values,
            "major_dimension": "ROWS"
        }),
        workspace_meta(spreadsheet_id, &access_token),
        client_name,
        timeout,
    )
    .await
}

pub fn write_ranges_blocking(
    spreadsheet_id: &str,
    sheet: &str,
    ranges: &serde_json::Map<String, Value>,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let mut updated_ranges = Vec::with_capacity(ranges.len());
    for (range, values) in ranges {
        write_values_blocking(
            spreadsheet_id,
            sheet,
            range,
            values.clone(),
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

pub async fn write_ranges_async(
    spreadsheet_id: &str,
    sheet: &str,
    ranges: &serde_json::Map<String, Value>,
    client_name: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let mut updated_ranges = Vec::with_capacity(ranges.len());
    for (range, values) in ranges {
        write_values_async(
            spreadsheet_id,
            sheet,
            range,
            values.clone(),
            client_name,
            timeout,
        )
        .await?;
        updated_ranges.push(range.clone());
    }
    Ok(json!({
        "ok": true,
        "updated_ranges": updated_ranges
    }))
}

