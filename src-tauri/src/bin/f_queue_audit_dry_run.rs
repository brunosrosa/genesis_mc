use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
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

fn normalize_header_cell(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
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
            "clientInfo": { "name": "f-queue-audit-dry-run", "version": "1.0.0" }
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
        .map_err(|e| io::Error::other(format!("Falha ao spawnar mcp-google-sheets: {e}")))?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().ok_or_else(|| io::Error::other("stdin indisponível"))?;
        writeln!(stdin, "{}", init_req)?;
        writeln!(stdin, "{}", initialized_notif)?;
        writeln!(stdin, "{}", mcp_request)?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| io::Error::other(format!("Falha ao aguardar mcp-google-sheets: {e}")))?;
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

#[derive(Debug)]
struct ColumnMap {
    status_atualizacao_idx: usize,
    project_name_idx: usize,
    repo_url_idx: usize,
}

fn resolve_column_map(header_row: &[String]) -> io::Result<ColumnMap> {
    let mut status_atualizacao_idx = None;
    let mut project_name_idx = None;
    let mut repo_url_idx = None;
    for (idx, raw) in header_row.iter().enumerate() {
        let h = normalize_header_cell(raw);
        match h.as_str() {
            "status_atualizacao" => status_atualizacao_idx = Some(idx),
            "project_name" => project_name_idx = Some(idx),
            "repo_url" => repo_url_idx = Some(idx),
            _ => {}
        }
    }
    Ok(ColumnMap {
        status_atualizacao_idx: status_atualizacao_idx
            .ok_or_else(|| io::Error::other("Cabeçalho sem status_atualizacao"))?,
        project_name_idx: project_name_idx.ok_or_else(|| io::Error::other("Cabeçalho sem project_name"))?,
        repo_url_idx: repo_url_idx.ok_or_else(|| io::Error::other("Cabeçalho sem repo_url"))?,
    })
}

fn parse_cli_args() -> Option<String> {
    let mut args = std::env::args();
    args.next();
    while let Some(arg) = args.next() {
        if arg == "--sheets-id" {
            return args.next();
        }
    }
    None
}

fn main() -> io::Result<()> {
    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let spreadsheet_id = parse_cli_args()
        .or_else(|| std::env::var("GOOGLE_SHEETS_ID").ok())
        .ok_or_else(|| io::Error::other("Missing GOOGLE_SHEETS_ID (or --sheets-id)"))?;

    let header = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": "A1:CF1",
            "include_grid_data": false
        }),
    )?;
    let header_row = extract_values_2d(&header)
        .unwrap_or_default()
        .get(0)
        .cloned()
        .unwrap_or_default();
    let cols = resolve_column_map(&header_row)?;

    let data = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": "A2:CF",
            "include_grid_data": false
        }),
    )?;
    let values = extract_values_2d(&data).unwrap_or_default();

    for row in values {
        let status = row
            .get(cols.status_atualizacao_idx)
            .map(|s| s.trim())
            .unwrap_or("");
        if status != "PENDENTE_FASE_0" && status != "PENDENTE_IA" {
            continue;
        }
        let name = row
            .get(cols.project_name_idx)
            .map(|s| s.trim())
            .unwrap_or("");
        let url = row.get(cols.repo_url_idx).map(|s| s.trim()).unwrap_or("");
        println!("{} | {} | {}", status, name, url);
    }

    Ok(())
}
