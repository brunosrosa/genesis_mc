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
use tracing::{error, info, warn};
use url::Url;
use rand::rngs::OsRng;
use rand::RngCore;

const STATUS_GATE_HARVESTER: &str = "APROVADO_PARA_HARVESTER";
const STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO: &str = "CONCLUIDO_AGUARDANDO";
const STATUS_FASE_F0_OK: &str = "FASE_0_HARVESTER_OK";
const STATUS_ERRO_F0: &str = "ERRO_F0";

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    repo_id: Option<String>,
    batch: bool,
}

fn parse_cli_args_from<I>(args: I) -> CliArgs
where
    I: IntoIterator<Item = String>,
{
    let mut it = args.into_iter();
    it.next();
    let mut repo_id: Option<String> = None;
    let mut batch = false;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--repo" => repo_id = it.next(),
            "--batch" => batch = true,
            _ => {}
        }
    }
    CliArgs { repo_id, batch }
}

fn normalize_repo_url_for_match(raw: &str) -> String {
    raw.trim()
        .trim_matches('`')
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string()
}

fn try_extract_repo_id_from_repo_url(repo_url: &str) -> Option<String> {
    let s = normalize_repo_url_for_match(repo_url);
    let marker = "github.com/";
    let idx = s.to_ascii_lowercase().find(marker)?;
    let rest = &s[(idx + marker.len())..];
    let mut parts = rest.split('/').map(|p| p.trim()).filter(|p| !p.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    Some(format!("{owner}/{repo}"))
}

fn is_rate_limit_error_text(raw: &str) -> bool {
    let s = raw.to_ascii_lowercase();
    s.contains("rate limit")
        || s.contains("api limit exceeded")
        || s.contains("403")
        || s.contains("http status client error (403")
}

fn jitter_ms_3_to_7_from_u32(v: u32) -> u64 {
    3_000 + (v as u64 % 4_001)
}

fn backoff_ms_from_attempt(attempt: u32, jitter_seed: u32) -> u64 {
    let base_ms = 3_000_u64;
    let exp = 1_u64
        .checked_shl(attempt.min(6))
        .unwrap_or(64);
    let jitter = jitter_seed as u64 % 1_000;
    (base_ms.saturating_mul(exp)).saturating_add(jitter).min(60_000)
}

async fn sleep_between_repos_jitter() {
    let mut rng = OsRng;
    let ms = jitter_ms_3_to_7_from_u32(rng.next_u32());
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BatchCandidate {
    repo_id: String,
    row_number_1based: u32,
}

async fn fetch_harvester_batch_candidates(
    spreadsheet_id: &str,
) -> Result<(Vec<BatchCandidate>, MasterCols), String> {
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
    let out = select_batch_candidates_from_values(&values, cols, min_idx);
    Ok((out, cols))
}

fn select_batch_candidates_from_values(
    values: &[Vec<String>],
    cols: MasterCols,
    min_idx: usize,
) -> Vec<BatchCandidate> {
    let mut out = Vec::new();
    for (i, row) in values.iter().enumerate() {
        let get = |abs_idx: usize| -> String {
            let rel = abs_idx.saturating_sub(min_idx);
            row.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        let status = get(cols.status_atualizacao_idx);
        if status.trim() != STATUS_GATE_HARVESTER {
            continue;
        }
        let repo_url = get(cols.repo_url_idx);
        if repo_url.trim().is_empty() {
            continue;
        }
        let Some(repo_id) = try_extract_repo_id_from_repo_url(&repo_url) else {
            continue;
        };
        out.push(BatchCandidate {
            repo_id,
            row_number_1based: (i as u32) + 2,
        });
    }
    out
}

async fn process_one_repo_f0(
    root_dir: &Path,
    db_path: &Path,
    spreadsheet_id: &str,
    cols: MasterCols,
    repo_id: &str,
    row_number_1based: u32,
    batch_index: Option<(usize, usize)>,
) -> io::Result<()> {
    let started = Instant::now();
    if let Some((idx, total)) = batch_index {
        info!(
            repo_id = %repo_id,
            row_number = row_number_1based,
            idx,
            total,
            "F0(batch): iniciando"
        );
    } else {
        info!(repo_id = %repo_id, row_number = row_number_1based, "F0: iniciando");
    }

    let status_atualizacao = read_status_atualizacao_at_row(spreadsheet_id, row_number_1based, cols)
        .await
        .map_err(io::Error::other)?;
    if status_atualizacao.trim() != STATUS_GATE_HARVESTER {
        info!(
            repo_id = %repo_id,
            row_number = row_number_1based,
            status_atualizacao = %status_atualizacao,
            expected = STATUS_GATE_HARVESTER,
            "F0: skip (fora do gatilho rígido)"
        );
        return Ok(());
    }

    let conn = Connection::open(db_path).map_err(io::Error::other)?;
    ensure_phase1_schema(&conn)?;

    let repo_url_str = format!("https://github.com/{}", repo_id);
    let repo_url = Url::parse(&repo_url_str).map_err(io::Error::other)?;
    let now = now_epoch_secs()?;

    conn.execute(
        "INSERT INTO repositorios (project_name, lote_id, repo_url, soda_universal_uuid, status_processamento, timestamp_fase_1, retry_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(project_name) DO UPDATE SET
            repo_url = excluded.repo_url,
            status_processamento = excluded.status_processamento,
            timestamp_fase_1 = excluded.timestamp_fase_1",
        params![
            repo_id,
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
    )
    .map_err(io::Error::other)?;

    let conn_arc = Arc::new(Mutex::new(conn));

    let heartbeat_repo = repo_id.to_string();
    let hb_started = Instant::now();
    let hb = tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
        tick.tick().await;
        loop {
            tick.tick().await;
            info!(
                repo_id = %heartbeat_repo,
                elapsed_s = hb_started.elapsed().as_secs(),
                "F0: heartbeat"
            );
        }
    });

    let max_attempts: u32 = 4;
    let mut attempt: u32 = 0;
    let mut res: Result<(), genesis_mc_lib::harvester::orchestrator::OrchestratorError> = Ok(());
    while attempt < max_attempts {
        match HarvesterOrchestrator::run(repo_id, &repo_url, Arc::clone(&conn_arc)).await {
            Ok(()) => {
                res = Ok(());
                break;
            }
            Err(e) => {
                let msg = e.to_string();
                if is_rate_limit_error_text(&msg) && attempt + 1 < max_attempts {
                    let mut rng = OsRng;
                    let backoff_ms = backoff_ms_from_attempt(attempt, rng.next_u32());
                    warn!(
                        repo_id = %repo_id,
                        attempt = attempt + 1,
                        backoff_ms,
                        error = %msg,
                        "F0: rate limit detectado; aplicando backoff e retry"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    attempt += 1;
                    continue;
                }
                res = Err(e);
                break;
            }
        }
    }
    hb.abort();

    match res {
        Ok(_) => {
            {
                let conn_lock = conn_arc.lock().map_err(|e| {
                    io::Error::other(format!("Falha ao adquirir lock do banco após F0: {}", e))
                })?;
                conn_lock
                    .execute(
                    "UPDATE repositorios
                     SET status_processamento = ?1,
                         timestamp_fase_1 = ?2
                     WHERE project_name = ?3",
                    params!["F0_OK", now_epoch_secs()?, repo_id],
                    )
                    .map_err(io::Error::other)?;
            }

            let report_path = write_f0_report(root_dir, &conn_arc, repo_id)?;
            update_status_atualizacao_e_fase(
                spreadsheet_id,
                row_number_1based,
                cols,
                STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO,
                STATUS_FASE_F0_OK,
            )
            .await
            .map_err(io::Error::other)?;
            info!(
                repo_id = %repo_id,
                row_number = row_number_1based,
                report = %report_path.display(),
                elapsed_ms = started.elapsed().as_millis(),
                "F0: concluído"
            );
        }
        Err(e) => {
            error!(
                repo_id = %repo_id,
                row_number = row_number_1based,
                error = %e,
                "F0: falha fatal (fail-soft por repo)"
            );
            {
                let conn_lock = conn_arc.lock().map_err(|lock_err| {
                    io::Error::other(format!("Falha ao adquirir lock do banco no erro da F0: {}", lock_err))
                })?;
                let _ = conn_lock.execute(
                    "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                    params![STATUS_ERRO_F0, repo_id],
                );
            }
            let _ = update_status_atualizacao_e_fase(
                spreadsheet_id,
                row_number_1based,
                cols,
                STATUS_ERRO_F0,
                STATUS_ERRO_F0,
            )
            .await;
        }
    };

    tokio::task::yield_now().await;
    Ok(())
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

    let root_dir = workspace_root()?;
    let soda_data_dir = root_dir.join(".soda_data");
    tokio::fs::create_dir_all(&soda_data_dir).await?;

    let db_path = soda_data_dir.join("soda_heuristic_vault.db");
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let args = parse_cli_args_from(std::env::args());

    if args.batch {
        info!(
            gate = STATUS_GATE_HARVESTER,
            "SODA F0 (Harvester/Zero-IA): modo batch sequencial"
        );
        let (candidates, cols) =
            fetch_harvester_batch_candidates(&spreadsheet_id).await.map_err(io::Error::other)?;
        info!(count = candidates.len(), "F0(batch): fila carregada");
        let total = candidates.len();
        for (idx, item) in candidates.into_iter().enumerate() {
            let _ = process_one_repo_f0(
                &root_dir,
                &db_path,
                &spreadsheet_id,
                cols,
                &item.repo_id,
                item.row_number_1based,
                Some((idx + 1, total)),
            )
            .await;
            sleep_between_repos_jitter().await;
        }
        info!("F0(batch): concluído");
        return Ok(());
    }

    let repo_id = args
        .repo_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "aaif-goose/goose".to_string());
    info!("SODA F0 (Harvester/Zero-IA): execução isolada (1 repo)");
    let (row_number, cols, _min_idx) =
        gate_harvester_by_sheet(&spreadsheet_id, &repo_id).await.map_err(io::Error::other)?;
    process_one_repo_f0(
        &root_dir,
        &db_path,
        &spreadsheet_id,
        cols,
        &repo_id,
        row_number,
        None,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_args_reads_repo_and_batch() {
        let args = vec![
            "bin".to_string(),
            "--repo".to_string(),
            "acme/widgets".to_string(),
        ];
        assert_eq!(
            parse_cli_args_from(args),
            CliArgs {
                repo_id: Some("acme/widgets".to_string()),
                batch: false
            }
        );

        let args = vec!["bin".to_string(), "--batch".to_string()];
        assert_eq!(
            parse_cli_args_from(args),
            CliArgs {
                repo_id: None,
                batch: true
            }
        );
    }

    #[test]
    fn extracts_repo_id_from_github_url_with_git_suffix() {
        assert_eq!(
            try_extract_repo_id_from_repo_url("https://github.com/aaif-goose/goose.git"),
            Some("aaif-goose/goose".to_string())
        );
        assert_eq!(
            try_extract_repo_id_from_repo_url("https://github.com/aaif-goose/goose/"),
            Some("aaif-goose/goose".to_string())
        );
        assert_eq!(try_extract_repo_id_from_repo_url(""), None);
        assert_eq!(try_extract_repo_id_from_repo_url("https://example.com/x/y"), None);
    }

    #[test]
    fn batch_selection_respects_column_indices_and_status_gate() {
        let cols = MasterCols {
            repo_url_idx: 10,
            status_atualizacao_idx: 2,
            status_fase_idx: 7,
        };
        let min_idx = 2;
        let row_ok = {
            let mut row = vec![String::new(); 9];
            row[0] = STATUS_GATE_HARVESTER.to_string();
            row[5] = "X".to_string();
            row[8] = "https://github.com/acme/ok".to_string();
            row
        };
        let row_skip_status = {
            let mut row = vec![String::new(); 9];
            row[0] = "OUTRO".to_string();
            row[8] = "https://github.com/acme/nope".to_string();
            row
        };
        let row_skip_bad_url = {
            let mut row = vec![String::new(); 9];
            row[0] = STATUS_GATE_HARVESTER.to_string();
            row[8] = "notaurl".to_string();
            row
        };
        let values = vec![row_ok, row_skip_status, row_skip_bad_url];
        let out = select_batch_candidates_from_values(&values, cols, min_idx);
        assert_eq!(
            out,
            vec![BatchCandidate {
                repo_id: "acme/ok".to_string(),
                row_number_1based: 2
            }]
        );
    }
}
