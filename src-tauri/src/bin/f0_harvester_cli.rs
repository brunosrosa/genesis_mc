use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chrono::{FixedOffset, Utc};
use genesis_mc_lib::harvester::canon::CANON_GLOBAL_REPO_ID;
use genesis_mc_lib::harvester::orchestrator::HarvesterOrchestrator;
use genesis_mc_lib::persist::sheets_utils::{col_idx_to_a1, extract_values_2d_strict, normalize_header_cell};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tokio::io::AsyncBufReadExt;
use tracing::{error, info};
use url::Url;

const STATUS_GATE_HARVESTER: &str = "APROVADO_PARA_HARVESTER";
const STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO: &str = "CONCLUIDO_AGUARDANDO";
const STATUS_FASE_F0_OK: &str = "FASE_0_HARVESTER_OK";

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

fn now_brt_rfc3339() -> String {
    Utc::now()
        .with_timezone(&FixedOffset::west_opt(3 * 3600).unwrap())
        .to_rfc3339()
}

fn etl_report_path(root_dir: &Path, repo_id: &str) -> io::Result<PathBuf> {
    let reports_dir = root_dir.join(".soda_scratchpad").join("reports");
    std::fs::create_dir_all(&reports_dir)
        .map_err(|e| io::Error::other(format!("Falha ao criar reports_dir: {}", e)))?;

    let trimmed = repo_id.trim();
    let mut parts = trimmed.split('/').map(|s| s.trim()).filter(|s| !s.is_empty());
    let owner = parts.next().unwrap_or(trimmed);
    let repo = parts.next().unwrap_or(trimmed);
    Ok(reports_dir.join(format!(
        "_ETL_REPORT_{}_{}.txt",
        sanitize_repo_id(owner),
        sanitize_repo_id(repo)
    )))
}

fn ensure_phase1_schema(conn: &Connection) -> io::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS repositorios (
            project_name TEXT PRIMARY KEY,
            lote_id TEXT NOT NULL,
            repo_url TEXT NOT NULL UNIQUE,
            repo_analised_version TEXT,
            repo_version TEXT,
            ultima_versao_online TEXT,
            soda_universal_uuid TEXT NOT NULL UNIQUE,
            status_processamento TEXT NOT NULL,
            timestamp_fase_1 INTEGER,
            timestamp_fase_3 INTEGER,
            retry_count INTEGER NOT NULL
        )",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar tabela repositorios: {}", e)))?;

    let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN repo_analised_version TEXT", []);
    let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN repo_version TEXT", []);
    let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN ultima_versao_online TEXT", []);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS artefatos_brutos (
            artifact_id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL REFERENCES repositorios(project_name),
            payload_blob BLOB NOT NULL,
            timestamp_extracao INTEGER NOT NULL,
            artifact_type TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar tabela artefatos_brutos: {}", e)))?;

    conn.execute(
        "DELETE FROM artefatos_brutos
         WHERE artifact_id NOT IN (
             SELECT MAX(artifact_id)
             FROM artefatos_brutos
             GROUP BY repo_id, artifact_type
         )",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao deduplicar artefatos existentes: {}", e)))?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_artefatos_repo_tipo
         ON artefatos_brutos(repo_id, artifact_type)",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar índice único de artefatos: {}", e)))?;

    conn.execute(
        "INSERT OR IGNORE INTO repositorios
         (project_name, lote_id, repo_url, soda_universal_uuid, status_processamento, timestamp_fase_1, retry_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            CANON_GLOBAL_REPO_ID,
            "CANON_CACHE",
            "https://notebooklm.google.com/",
            "UUID-SODA-CANON-GLOBAL",
            "CACHE_GLOBAL",
            0_i64,
            0_i64
        ],
    )
    .map_err(|e| io::Error::other(format!("Falha ao registrar linha sintética do cache canônico global: {}", e)))?;

    Ok(())
}

fn write_f0_report(
    root_dir: &Path,
    conn_arc: &Arc<Mutex<Connection>>,
    repo_id: &str,
) -> io::Result<PathBuf> {
    let report_path = etl_report_path(root_dir, repo_id)?;
    let rows = {
        let conn = conn_arc.lock().map_err(|e| {
            io::Error::other(format!("Falha ao adquirir lock do banco para relatório da F0: {}", e))
        })?;
        let mut stmt = conn
            .prepare(
                "SELECT artifact_type, LENGTH(payload_blob)
                 FROM artefatos_brutos
                 WHERE repo_id = ?1
                 ORDER BY artifact_type ASC",
            )
            .map_err(|e| io::Error::other(format!("Falha ao preparar query do relatório da F0: {}", e)))?;
        let iter = stmt
            .query_map([repo_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| io::Error::other(format!("Falha ao executar query do relatório da F0: {}", e)))?;

        let mut rows = Vec::new();
        for row in iter {
            rows.push(row.map_err(|e| io::Error::other(format!("Falha ao ler linha do relatório da F0: {}", e)))?);
        }
        rows
    };

    if rows.is_empty() {
        return Err(io::Error::other("A F0 terminou sem blobs persistidos para o relatório"));
    }

    let mut report = String::new();
    report.push_str(&format!("\n\n=== FASE 0: HARVESTER @ {} ===\n\n", now_brt_rfc3339()));
    report.push_str(&format!("repo_id={}\n", repo_id));
    report.push_str("artifact_type\tpayload_bytes\n");
    for (artifact_type, payload_len) in rows {
        report.push_str(&format!("{}\t{}\n", artifact_type, payload_len));
    }

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&report_path)
        .map_err(|e| io::Error::other(format!("Falha ao abrir relatório ETL {}: {}", report_path.display(), e)))?;
    file.write_all(report.as_bytes())
        .map_err(|e| io::Error::other(format!("Falha ao anexar relatório ETL: {}", e)))?;

    Ok(report_path)
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

fn normalize_repo_url_for_match(raw: &str) -> String {
    raw.trim()
        .trim_matches('`')
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string()
}

async fn poll_for_jsonrpc_response_from_reader<R>(
    reader: R,
    timeout: std::time::Duration,
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
        match tokio::time::timeout(std::time::Duration::from_millis(200), lines.next_line()).await {
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
            Ok(Ok(None)) => tokio::time::sleep(std::time::Duration::from_millis(200)).await,
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
    use tokio::io::AsyncWriteExt;
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
            "clientInfo": { "name": "f0-harvester-cli", "version": "1.0.0" }
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

    let msg_res: Result<Value, String> = async {
        let mut stdin = child.stdin.take().ok_or_else(|| "stdin indisponível".to_string())?;
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

        let stdout = child.stdout.take().ok_or_else(|| "stdout indisponível".to_string())?;
        poll_for_jsonrpc_response_from_reader(
            tokio::io::BufReader::new(stdout),
            std::time::Duration::from_secs(20),
            1,
        )
        .await
    }
    .await;

    let _ = child.kill().await;
    let _ = child.wait().await;

    let msg = msg_res?;

    if msg.get("error").is_some() {
        return Err(format!("MCP retornou erro: {msg}"));
    }
    if let Some(result) = msg.get("result") {
        return Ok(normalize_mcp_tool_result(result.clone()));
    }
    Err("Resposta MCP inválida (sem campo result)".to_string())
}

async fn get_sheet_data(spreadsheet_id: &str, sheet: &str, range: String) -> Result<Vec<Vec<String>>, String> {
    let result = call_mcp(
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
}

#[derive(Clone, Copy)]
struct MasterCols {
    repo_url_idx: usize,
    status_atualizacao_idx: usize,
    status_fase_idx: usize,
}

fn resolve_master_cols(header_row: &[String]) -> Result<MasterCols, String> {
    let mut map = std::collections::HashMap::new();
    for (idx, cell) in header_row.iter().enumerate() {
        let k = normalize_header_cell(cell);
        if !k.is_empty() {
            map.insert(k, idx);
        }
    }
    let repo_url_idx = *map
        .get("repo_url")
        .ok_or_else(|| "Header missing repo_url".to_string())?;
    let status_atualizacao_idx = *map
        .get("status_atualizacao")
        .ok_or_else(|| "Header missing status_atualizacao".to_string())?;
    let status_fase_idx = *map
        .get("status_fase")
        .ok_or_else(|| "Header missing status_fase".to_string())?;
    Ok(MasterCols {
        repo_url_idx,
        status_atualizacao_idx,
        status_fase_idx,
    })
}

async fn gate_harvester_by_sheet(spreadsheet_id: &str, repo_id: &str) -> Result<(u32, MasterCols, usize), String> {
    let header = get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", "A1:CF1".to_string()).await?;
    let header_row = header.first().cloned().unwrap_or_default();
    if header_row.is_empty() {
        return Err("Header vazio em MASTER_SOLUTIONS!A1:CF1".to_string());
    }
    let cols = resolve_master_cols(&header_row)?;

    let required = [cols.repo_url_idx, cols.status_atualizacao_idx, cols.status_fase_idx];
    let min_idx = *required.iter().min().unwrap_or(&0);
    let max_idx = *required.iter().max().unwrap_or(&0);
    let start_col = col_idx_to_a1(min_idx);
    let end_col = col_idx_to_a1(max_idx);
    let range = format!("{start_col}2:{end_col}");
    let values = get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", range).await?;

    let expected = normalize_repo_url_for_match(&format!("https://github.com/{repo_id}"));
    for (i, row) in values.iter().enumerate() {
        let get = |abs_idx: usize| -> String {
            let rel = abs_idx.saturating_sub(min_idx);
            row.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        let repo_url = normalize_repo_url_for_match(&get(cols.repo_url_idx));
        if repo_url.is_empty() {
            continue;
        }
        if repo_url == expected {
            let status = get(cols.status_atualizacao_idx);
            return Ok(((i as u32) + 2, cols, min_idx))
                .and_then(|x| {
                    let _ = status;
                    Ok(x)
                });
        }
    }
    Err(format!(
        "Harvester gate falhou: repo_url não encontrado no Sheets (expected={})",
        expected
    ))
}

async fn read_status_atualizacao_at_row(
    spreadsheet_id: &str,
    row_number_1based: u32,
    cols: MasterCols,
) -> Result<String, String> {
    let status_col = col_idx_to_a1(cols.status_atualizacao_idx);
    let range = format!("{status_col}{row_number_1based}:{status_col}{row_number_1based}");
    let values = get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", range).await?;
    Ok(values
        .first()
        .and_then(|r| r.first())
        .map(|s| s.trim().to_string())
        .unwrap_or_default())
}

async fn update_status_atualizacao_e_fase(
    spreadsheet_id: &str,
    row_number_1based: u32,
    cols: MasterCols,
    status_atualizacao: &str,
    status_fase: &str,
) -> Result<(), String> {
    let status_col = col_idx_to_a1(cols.status_atualizacao_idx);
    let fase_col = col_idx_to_a1(cols.status_fase_idx);
    let status_range = format!("{status_col}{row_number_1based}:{status_col}{row_number_1based}");
    let fase_range = format!("{fase_col}{row_number_1based}:{fase_col}{row_number_1based}");
    let _ = call_mcp(
        "batch_update_cells",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "ranges": {
                status_range: [[status_atualizacao]],
                fase_range: [[status_fase]]
            }
        }),
    )
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let level = match rust_log.to_ascii_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    info!("SODA F0 (Harvester/Zero-IA): iniciando extração isolada");
    let started = Instant::now();

    let root_dir = workspace_root()?;
    let soda_data_dir = root_dir.join(".soda_data");
    tokio::fs::create_dir_all(&soda_data_dir).await?;

    let db_path = soda_data_dir.join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path)?;
    ensure_phase1_schema(&conn)?;

    let repo_id = parse_repo_id_from_args();
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let (row_number, cols, _min_idx) =
        gate_harvester_by_sheet(&spreadsheet_id, &repo_id).await.map_err(io::Error::other)?;
    let status_atualizacao =
        read_status_atualizacao_at_row(&spreadsheet_id, row_number, cols).await.map_err(io::Error::other)?;
    if status_atualizacao.trim() != STATUS_GATE_HARVESTER {
        info!(
            repo_id = %repo_id,
            row_number,
            status_atualizacao = %status_atualizacao,
            expected = STATUS_GATE_HARVESTER,
            "F0: skip (fora do gatilho rígido)"
        );
        return Ok(());
    }
    info!(
        repo_id = %repo_id,
        row_number,
        "F0: gatilho rígido validado (APROVADO_PARA_HARVESTER)"
    );
    let repo_url_str = format!("https://github.com/{}", repo_id);
    let repo_url = Url::parse(&repo_url_str)?;
    let now = now_epoch_secs()?;

    conn.execute(
        "INSERT INTO repositorios (project_name, lote_id, repo_url, soda_universal_uuid, status_processamento, timestamp_fase_1, retry_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(project_name) DO UPDATE SET
            repo_url = excluded.repo_url,
            status_processamento = excluded.status_processamento,
            timestamp_fase_1 = excluded.timestamp_fase_1",
        params![
            &repo_id,
            std::env::var("SODA_LOTE_ID_OVERRIDE")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "LOTE_01_ALPHA".to_string()),
            &repo_url_str,
            format!("UUID-{}", repo_id),
            "PENDENTE",
            now,
            0
        ],
    )?;

    info!(repo_id = %repo_id, "Registro base inserido/verificado. Iniciando orquestração da F0");

    let conn_arc = Arc::new(Mutex::new(conn));

    match HarvesterOrchestrator::run(&repo_id, &repo_url, Arc::clone(&conn_arc)).await {
        Ok(_) => {
            info!(repo_id = %repo_id, "CLI: HarvesterOrchestrator retornou OK; atualizando status para F0_OK");
            {
                let conn_lock = conn_arc.lock().map_err(|e| {
                    io::Error::other(format!("Falha ao adquirir lock do banco após F0: {}", e))
                })?;
                conn_lock.execute(
                    "UPDATE repositorios
                     SET status_processamento = ?1,
                         timestamp_fase_1 = ?2
                     WHERE project_name = ?3",
                    params!["F0_OK", now_epoch_secs()?, &repo_id],
                )?;
            }

            info!(repo_id = %repo_id, "CLI: Status F0_OK persistido; exportando relatório local");
            let report_path = write_f0_report(&root_dir, &conn_arc, &repo_id)?;
            update_status_atualizacao_e_fase(
                &spreadsheet_id,
                row_number,
                cols,
                STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO,
                STATUS_FASE_F0_OK,
            )
                .await
                .map_err(io::Error::other)?;
            info!(
                repo_id = %repo_id,
                report = %report_path.display(),
                elapsed_ms = started.elapsed().as_millis(),
                "F0 concluída; relatório local exportado"
            );
            return Ok(());
        }
        Err(e) => {
            error!(repo_id = %repo_id, error = %e, "Falha crítica na F0");
            let conn_lock = conn_arc.lock().map_err(|lock_err| {
                io::Error::other(format!("Falha ao adquirir lock do banco no erro da F0: {}", lock_err))
            })?;
            conn_lock.execute(
                "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                params!["ERRO_F0", &repo_id],
            )?;
            return Err(e.into());
        }
    }
}
