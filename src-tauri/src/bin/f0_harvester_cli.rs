use std::io;
use std::path::{Path, PathBuf};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chrono::{FixedOffset, Utc};
use genesis_mc_lib::harvester::canon::CANON_GLOBAL_REPO_ID;
use genesis_mc_lib::harvester::orchestrator::HarvesterOrchestrator;
use genesis_mc_lib::persist::sheets_utils::{col_idx_to_a1, extract_values_2d_strict, normalize_header_cell};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use tracing::{error, info, warn};
use url::Url;
use rand::rngs::OsRng;
use rand::RngCore;

const STATUS_GATE_HARVESTER: &str = "APROVADO_PARA_HARVESTER";
const STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO: &str = "CONCLUIDO_AGUARDANDO";
const STATUS_FASE_F0_OK: &str = "FASE_0_HARVESTER_OK";
const STATUS_ERRO_F0: &str = "ERRO_F0";
const STATUS_DEGRADADO_F0: &str = "DEGRADADO_F0";
const STATUS_FASE_F0_DEGRADADA: &str = "FASE_0_DEGRADADA";

const EXPECTED_F0_BLOBS: [&str; 11] = [
    "blob_01_promessa_readme",
    "blob_02_dependency_manifest",
    "blob_03_test_intent",
    "blob_04_repo_outline",
    "blob_05_architecture_map",
    "blob_06_unsafe_hotspots",
    "blob_07_ops_blueprint",
    "blob_08_health_report",
    "blob_09_community_meta",
    "blob_10_soda_canon_context",
    "blob_11_ux_contracts",
];

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
    direct: bool,
}

fn parse_cli_args_from<I>(args: I) -> CliArgs
where
    I: IntoIterator<Item = String>,
{
    let mut it = args.into_iter();
    it.next();
    let mut repo_id: Option<String> = None;
    let mut batch = false;
    let mut direct = false;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--repo" => repo_id = it.next(),
            "--batch" => batch = true,
            "--direct" => direct = true,
            _ => {}
        }
    }
    CliArgs {
        repo_id,
        batch,
        direct,
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepoOutcome {
    Success,
    Skipped,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepoBatchSummary {
    repo_id: String,
    row_number_1based: u32,
    outcome: RepoOutcome,
    elapsed_ms: u128,
    blobs_present: Vec<String>,
    blobs_missing: Vec<String>,
    report_path: Option<PathBuf>,
    error: Option<String>,
}

fn read_repo_blobs_present(conn: &Connection, repo_id: &str) -> io::Result<Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT artifact_type
             FROM artefatos_brutos
             WHERE repo_id = ?1 AND artifact_type LIKE 'blob_%'
             ORDER BY artifact_type",
        )
        .map_err(io::Error::other)?;
    let rows = stmt
        .query_map([repo_id], |row| row.get::<_, String>(0))
        .map_err(io::Error::other)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(io::Error::other)?);
    }
    Ok(out)
}

fn read_repo_blob_text(conn: &Connection, repo_id: &str, artifact_type: &str) -> io::Result<Option<String>> {
    conn.query_row(
        "SELECT CAST(payload_blob AS TEXT)
         FROM artefatos_brutos
         WHERE repo_id = ?1 AND artifact_type = ?2
         LIMIT 1",
        params![repo_id, artifact_type],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(io::Error::other)
}

fn is_content_repo_kind_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("kind: skilllibrary") || lower.contains("kind: contentrepo")
}

fn is_no_source_files_like_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("no source files") || lower.contains("no source file")
}

fn looks_like_knowledge_repo(conn: &Connection, repo_id: &str) -> bool {
    let readme_ok = read_repo_blob_text(conn, repo_id, "blob_01_promessa_readme")
        .ok()
        .flatten()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !readme_ok {
        return false;
    }

    let manifest = read_repo_blob_text(conn, repo_id, "blob_02_dependency_manifest")
        .ok()
        .flatten()
        .unwrap_or_default();
    let stack_unknown = manifest.contains("stack_base: UNKNOWN")
        || manifest.contains("stack_base: unknown")
        || manifest.contains("stack_base: N/A")
        || manifest.contains("stack_base: n/a");

    let ux = read_repo_blob_text(conn, repo_id, "blob_11_ux_contracts")
        .ok()
        .flatten()
        .unwrap_or_default();
    let ux_skipped = ux.contains("package.json ausente") || ux.contains("foi pulada");

    stack_unknown && ux_skipped
}

fn detect_degraded_blobs(conn: &Connection, repo_id: &str) -> io::Result<Vec<String>> {
    let mut degraded = Vec::new();

    let outline = read_repo_blob_text(conn, repo_id, "blob_04_repo_outline")?;
    let arch_map = read_repo_blob_text(conn, repo_id, "blob_05_architecture_map")?;
    let is_content_repo = outline
        .as_deref()
        .map(is_content_repo_kind_text)
        .unwrap_or(false)
        || arch_map
            .as_deref()
            .map(is_content_repo_kind_text)
            .unwrap_or(false);
    let is_knowledge_repo = is_content_repo || looks_like_knowledge_repo(conn, repo_id);

    if let Some(text) = outline {
        if text.contains("Fallback:")
            && !(is_knowledge_repo || is_no_source_files_like_text(&text))
        {
            degraded.push("blob_04_repo_outline".to_string());
        }
    }
    if let Some(text) = arch_map {
        let is_empty_topology = text.contains("index nao gerou relacoes topologicas internas");
        if text.contains("Fallback:")
            && !(is_knowledge_repo || is_no_source_files_like_text(&text) || is_empty_topology)
        {
            degraded.push("blob_05_architecture_map".to_string());
        }
    }
    if let Some(text) = read_repo_blob_text(conn, repo_id, "blob_08_health_report")? {
        let has_fallback = text.contains("\"fallback\":true") || text.contains("\"fallback\": true");
        let is_skipped = text.to_ascii_lowercase().contains("foi pulado")
            || text.to_ascii_lowercase().contains("pulado pelo roteamento")
            || text.to_ascii_lowercase().contains("static analysis (semgrep) foi pulado");
        if !is_knowledge_repo && has_fallback && !is_skipped
        {
            degraded.push("blob_08_health_report".to_string());
        }
    }

    Ok(degraded)
}

fn compute_missing_blobs(present: &[String]) -> Vec<String> {
    let set = present.iter().cloned().collect::<BTreeSet<_>>();
    EXPECTED_F0_BLOBS
        .iter()
        .filter(|t| !set.contains(**t))
        .map(|t| t.to_string())
        .collect()
}

async fn fetch_harvester_batch_candidates(
    spreadsheet_id: &str,
) -> Result<(Vec<BatchCandidate>, MasterCols), String> {
    let header_range = genesis_mc_lib::cognition::synthesizer::master_solutions_header_range();
    let header = get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", header_range.clone()).await?;
    let header_row = header.first().cloned().unwrap_or_default();
    if header_row.is_empty() {
        return Err(format!("Header vazio em MASTER_SOLUTIONS!{}", header_range));
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
) -> RepoBatchSummary {
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
        .unwrap_or_default();
    if status_atualizacao.trim() != STATUS_GATE_HARVESTER {
        info!(
            repo_id = %repo_id,
            row_number = row_number_1based,
            status_atualizacao = %status_atualizacao,
            expected = STATUS_GATE_HARVESTER,
            "F0: skip (fora do gatilho rígido)"
        );
        return RepoBatchSummary {
            repo_id: repo_id.to_string(),
            row_number_1based,
            outcome: RepoOutcome::Skipped,
            elapsed_ms: started.elapsed().as_millis(),
            blobs_present: Vec::new(),
            blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
            report_path: None,
            error: None,
        };
    }

    let conn = match Connection::open(db_path).map_err(io::Error::other) {
        Ok(conn) => conn,
        Err(e) => {
            return RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based,
                outcome: RepoOutcome::Error,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: Vec::new(),
                blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
                report_path: None,
                error: Some(e.to_string()),
            };
        }
    };
    if let Err(e) = ensure_phase1_schema(&conn) {
        return RepoBatchSummary {
            repo_id: repo_id.to_string(),
            row_number_1based,
            outcome: RepoOutcome::Error,
            elapsed_ms: started.elapsed().as_millis(),
            blobs_present: Vec::new(),
            blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
            report_path: None,
            error: Some(e.to_string()),
        };
    }

    let repo_url_str = format!("https://github.com/{}", repo_id);
    let repo_url = match Url::parse(&repo_url_str).map_err(io::Error::other) {
        Ok(url) => url,
        Err(e) => {
            return RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based,
                outcome: RepoOutcome::Error,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: Vec::new(),
                blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
                report_path: None,
                error: Some(e.to_string()),
            };
        }
    };
    let now = match now_epoch_secs() {
        Ok(now) => now,
        Err(e) => {
            return RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based,
                outcome: RepoOutcome::Error,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: Vec::new(),
                blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
                report_path: None,
                error: Some(e.to_string()),
            };
        }
    };

    if let Err(e) = conn.execute(
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
    ) {
        let msg = format!("Falha ao inserir/atualizar repositorios: {e}");
        error!(
            repo_id = %repo_id,
            row_number = row_number_1based,
            error = %msg,
            "F0: falha ao registrar repo no banco"
        );
        let _ = update_status_atualizacao_e_fase(
            spreadsheet_id,
            row_number_1based,
            cols,
            STATUS_ERRO_F0,
            STATUS_ERRO_F0,
        )
        .await;
        return RepoBatchSummary {
            repo_id: repo_id.to_string(),
            row_number_1based,
            outcome: RepoOutcome::Error,
            elapsed_ms: started.elapsed().as_millis(),
            blobs_present: Vec::new(),
            blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
            report_path: None,
            error: Some(msg),
        };
    }

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

    let (mut blobs_present, mut blobs_missing) = {
        let present = match conn_arc.lock() {
            Ok(conn_lock) => read_repo_blobs_present(&conn_lock, repo_id).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let missing = compute_missing_blobs(&present);
        (present, missing)
    };

    match res {
        Ok(_) => {
            let mut post_error: Option<String> = None;
            if let Ok(conn_lock) = conn_arc.lock() {
                match now_epoch_secs() {
                    Ok(now) => {
                        if let Err(e) = conn_lock.execute(
                            "UPDATE repositorios
                             SET status_processamento = ?1,
                                 timestamp_fase_1 = ?2
                             WHERE project_name = ?3",
                            params!["F0_OK", now, repo_id],
                        ) {
                            post_error = Some(e.to_string());
                        }
                    }
                    Err(e) => post_error = Some(e.to_string()),
                }
            } else {
                post_error = Some("Falha ao adquirir lock do banco após F0".to_string());
            }

            let report_path = match write_f0_report(root_dir, &conn_arc, repo_id) {
                Ok(p) => Some(p),
                Err(e) => {
                    post_error = Some(e.to_string());
                    None
                }
            };
            let degraded_blobs = match conn_arc.lock() {
                Ok(conn_lock) => detect_degraded_blobs(&conn_lock, repo_id).unwrap_or_default(),
                Err(_) => Vec::new(),
            };
            if !degraded_blobs.is_empty() {
                let msg = format!(
                    "F0 degradada: blobs com fallback detectado: {}",
                    degraded_blobs.join(", ")
                );
                error!(
                    repo_id = %repo_id,
                    row_number = row_number_1based,
                    degraded = ?degraded_blobs,
                    "F0: degradação detectada; bloqueando avanço"
                );
                if let Ok(conn_lock) = conn_arc.lock() {
                    let _ = conn_lock.execute(
                        "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                        params![STATUS_DEGRADADO_F0, repo_id],
                    );
                }
                let _ = update_status_atualizacao_e_fase(
                    spreadsheet_id,
                    row_number_1based,
                    cols,
                    STATUS_DEGRADADO_F0,
                    STATUS_FASE_F0_DEGRADADA,
                )
                .await;
                return RepoBatchSummary {
                    repo_id: repo_id.to_string(),
                    row_number_1based,
                    outcome: RepoOutcome::Error,
                    elapsed_ms: started.elapsed().as_millis(),
                    blobs_present: std::mem::take(&mut blobs_present),
                    blobs_missing: std::mem::take(&mut blobs_missing),
                    report_path,
                    error: Some(msg),
                };
            } else {
                update_status_atualizacao_e_fase(
                    spreadsheet_id,
                    row_number_1based,
                    cols,
                    STATUS_ATUALIZACAO_CONCLUIDO_AGUARDANDO,
                    STATUS_FASE_F0_OK,
                )
                .await
                .ok();
            }
            if let Some(ref path) = report_path {
                info!(
                    repo_id = %repo_id,
                    row_number = row_number_1based,
                    report = %path.display(),
                    elapsed_ms = started.elapsed().as_millis(),
                    "F0: concluído"
                );
            } else {
                info!(
                    repo_id = %repo_id,
                    row_number = row_number_1based,
                    elapsed_ms = started.elapsed().as_millis(),
                    "F0: concluído (sem relatório)"
                );
            }
            RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based,
                outcome: RepoOutcome::Success,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: std::mem::take(&mut blobs_present),
                blobs_missing: std::mem::take(&mut blobs_missing),
                report_path,
                error: post_error,
            }
        }
        Err(e) => {
            error!(
                repo_id = %repo_id,
                row_number = row_number_1based,
                error = %e,
                "F0: falha fatal (fail-soft por repo)"
            );
            if let Ok(conn_lock) = conn_arc.lock() {
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
            RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based,
                outcome: RepoOutcome::Error,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: std::mem::take(&mut blobs_present),
                blobs_missing: std::mem::take(&mut blobs_missing),
                report_path: None,
                error: Some(e.to_string()),
            }
        }
    }
}

async fn process_one_repo_f0_direct(
    root_dir: &Path,
    db_path: &Path,
    repo_id: &str,
) -> RepoBatchSummary {
    let started = Instant::now();
    info!(repo_id = %repo_id, "F0(direct): iniciando (sem Sheets)");

    let conn = match Connection::open(db_path).map_err(io::Error::other) {
        Ok(conn) => conn,
        Err(e) => {
            return RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based: 0,
                outcome: RepoOutcome::Error,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: Vec::new(),
                blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
                report_path: None,
                error: Some(e.to_string()),
            };
        }
    };
    if let Err(e) = ensure_phase1_schema(&conn) {
        return RepoBatchSummary {
            repo_id: repo_id.to_string(),
            row_number_1based: 0,
            outcome: RepoOutcome::Error,
            elapsed_ms: started.elapsed().as_millis(),
            blobs_present: Vec::new(),
            blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
            report_path: None,
            error: Some(e.to_string()),
        };
    }

    let repo_url_str = format!("https://github.com/{}", repo_id);
    let repo_url = match Url::parse(&repo_url_str).map_err(io::Error::other) {
        Ok(url) => url,
        Err(e) => {
            return RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based: 0,
                outcome: RepoOutcome::Error,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: Vec::new(),
                blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
                report_path: None,
                error: Some(e.to_string()),
            };
        }
    };
    let now = match now_epoch_secs() {
        Ok(now) => now,
        Err(e) => {
            return RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based: 0,
                outcome: RepoOutcome::Error,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: Vec::new(),
                blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
                report_path: None,
                error: Some(e.to_string()),
            };
        }
    };

    if let Err(e) = conn.execute(
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
    ) {
        let msg = format!("Falha ao inserir/atualizar repositorios: {e}");
        return RepoBatchSummary {
            repo_id: repo_id.to_string(),
            row_number_1based: 0,
            outcome: RepoOutcome::Error,
            elapsed_ms: started.elapsed().as_millis(),
            blobs_present: Vec::new(),
            blobs_missing: EXPECTED_F0_BLOBS.iter().map(|s| s.to_string()).collect(),
            report_path: None,
            error: Some(msg),
        };
    }

    let conn_arc = Arc::new(Mutex::new(conn));
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
                        "F0(direct): rate limit detectado; aplicando backoff e retry"
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

    let (mut blobs_present, mut blobs_missing) = {
        let present = match conn_arc.lock() {
            Ok(conn_lock) => read_repo_blobs_present(&conn_lock, repo_id).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let missing = compute_missing_blobs(&present);
        (present, missing)
    };

    match res {
        Ok(_) => {
            if let Ok(conn_lock) = conn_arc.lock() {
                if let Ok(now) = now_epoch_secs() {
                    let _ = conn_lock.execute(
                        "UPDATE repositorios
                         SET status_processamento = ?1,
                             timestamp_fase_1 = ?2
                         WHERE project_name = ?3",
                        params!["F0_OK", now, repo_id],
                    );
                }
            }

            let report_path = write_f0_report(root_dir, &conn_arc, repo_id).ok();

            let degraded_blobs = match conn_arc.lock() {
                Ok(conn_lock) => detect_degraded_blobs(&conn_lock, repo_id).unwrap_or_default(),
                Err(_) => Vec::new(),
            };
            if !degraded_blobs.is_empty() {
                let msg = format!(
                    "F0 degradada: blobs com fallback detectado: {}",
                    degraded_blobs.join(", ")
                );
                if let Ok(conn_lock) = conn_arc.lock() {
                    let _ = conn_lock.execute(
                        "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                        params![STATUS_DEGRADADO_F0, repo_id],
                    );
                }
                return RepoBatchSummary {
                    repo_id: repo_id.to_string(),
                    row_number_1based: 0,
                    outcome: RepoOutcome::Error,
                    elapsed_ms: started.elapsed().as_millis(),
                    blobs_present: std::mem::take(&mut blobs_present),
                    blobs_missing: std::mem::take(&mut blobs_missing),
                    report_path,
                    error: Some(msg),
                };
            }

            info!(
                repo_id = %repo_id,
                elapsed_ms = started.elapsed().as_millis(),
                blobs = blobs_present.len(),
                missing = blobs_missing.len(),
                missing_list = ?blobs_missing,
                report = ?report_path.as_ref().map(|p| p.display().to_string()),
                "F0(direct): concluído"
            );
            RepoBatchSummary {
                repo_id: repo_id.to_string(),
                row_number_1based: 0,
                outcome: RepoOutcome::Success,
                elapsed_ms: started.elapsed().as_millis(),
                blobs_present: std::mem::take(&mut blobs_present),
                blobs_missing: std::mem::take(&mut blobs_missing),
                report_path,
                error: None,
            }
        }
        Err(e) => RepoBatchSummary {
            repo_id: repo_id.to_string(),
            row_number_1based: 0,
            outcome: RepoOutcome::Error,
            elapsed_ms: started.elapsed().as_millis(),
            blobs_present: std::mem::take(&mut blobs_present),
            blobs_missing: std::mem::take(&mut blobs_missing),
            report_path: None,
            error: Some(e.to_string()),
        },
    }
}

async fn call_mcp(tool_name: &str, arguments: Value) -> Result<Value, String> {
    genesis_mc_lib::persist::google_workspace_mcp::call_legacy_sheets_tool_async(
        tool_name,
        arguments,
        "f0-harvester-cli",
        std::time::Duration::from_secs(20),
    )
    .await
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
    let header_range = genesis_mc_lib::cognition::synthesizer::master_solutions_header_range();
    let header = get_sheet_data(spreadsheet_id, "MASTER_SOLUTIONS", header_range.clone()).await?;
    let header_row = header.first().cloned().unwrap_or_default();
    if header_row.is_empty() {
        return Err(format!("Header vazio em MASTER_SOLUTIONS!{}", header_range));
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
            let _ = status;
            return Ok(((i as u32) + 2, cols, min_idx));
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
    let args = parse_cli_args_from(std::env::args());

    if args.batch {
        let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
        info!(
            gate = STATUS_GATE_HARVESTER,
            "SODA F0 (Harvester/Zero-IA): modo batch sequencial"
        );
        let batch_started = Instant::now();
        let (candidates, cols) =
            fetch_harvester_batch_candidates(&spreadsheet_id).await.map_err(io::Error::other)?;
        info!(count = candidates.len(), "F0(batch): fila carregada");
        let total = candidates.len();
        let mut results: Vec<RepoBatchSummary> = Vec::new();
        for (idx, item) in candidates.into_iter().enumerate() {
            let summary = process_one_repo_f0(
                &root_dir,
                &db_path,
                &spreadsheet_id,
                cols,
                &item.repo_id,
                item.row_number_1based,
                Some((idx + 1, total)),
            )
            .await;
            results.push(summary);
            sleep_between_repos_jitter().await;
        }
        let total_elapsed_ms = batch_started.elapsed().as_millis();
        let mut ok_count = 0usize;
        let mut error_count = 0usize;
        let mut skipped_count = 0usize;
        let mut processed_elapsed_ms: u128 = 0;
        let mut processed_count: u128 = 0;
        for r in &results {
            match r.outcome {
                RepoOutcome::Success => {
                    ok_count += 1;
                    processed_count += 1;
                    processed_elapsed_ms = processed_elapsed_ms.saturating_add(r.elapsed_ms);
                }
                RepoOutcome::Error => {
                    error_count += 1;
                    processed_count += 1;
                    processed_elapsed_ms = processed_elapsed_ms.saturating_add(r.elapsed_ms);
                }
                RepoOutcome::Skipped => skipped_count += 1,
            }
        }
        let avg_ms = if processed_count == 0 {
            0
        } else {
            processed_elapsed_ms / processed_count
        };

        info!(
            total_candidates = total,
            ok = ok_count,
            error = error_count,
            skipped = skipped_count,
            total_elapsed_ms,
            avg_ms,
            "F0(batch): resumo final"
        );
        for r in &results {
            match r.outcome {
                RepoOutcome::Success => {
                    info!(
                        repo_id = %r.repo_id,
                        row_number = r.row_number_1based,
                        elapsed_ms = r.elapsed_ms,
                        blobs = r.blobs_present.len(),
                        missing = r.blobs_missing.len(),
                        missing_list = ?r.blobs_missing,
                        report = ?r.report_path.as_ref().map(|p| p.display().to_string()),
                        "F0(batch): OK"
                    );
                }
                RepoOutcome::Skipped => {
                    info!(
                        repo_id = %r.repo_id,
                        row_number = r.row_number_1based,
                        elapsed_ms = r.elapsed_ms,
                        "F0(batch): SKIP"
                    );
                }
                RepoOutcome::Error => {
                    warn!(
                        repo_id = %r.repo_id,
                        row_number = r.row_number_1based,
                        elapsed_ms = r.elapsed_ms,
                        blobs = r.blobs_present.len(),
                        missing = r.blobs_missing.len(),
                        missing_list = ?r.blobs_missing,
                        error = ?r.error,
                        "F0(batch): ERRO"
                    );
                }
            }
        }
        info!("F0(batch): concluído");
        return Ok(());
    }

    let repo_id = args
        .repo_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "aaif-goose/goose".to_string());
    if args.direct {
        info!("SODA F0 (Harvester/Zero-IA): execução direta (sem Sheets)");
        let summary = process_one_repo_f0_direct(&root_dir, &db_path, &repo_id).await;
        if summary.outcome == RepoOutcome::Error {
            let detail = summary
                .error
                .unwrap_or_else(|| "Erro não especificado".to_string());
            return Err(io::Error::other(format!("F0(direct) falhou para {}: {}", summary.repo_id, detail)).into());
        }
        return Ok(());
    }

    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    info!("SODA F0 (Harvester/Zero-IA): execução isolada (1 repo)");
    let (row_number, cols, _min_idx) =
        gate_harvester_by_sheet(&spreadsheet_id, &repo_id).await.map_err(io::Error::other)?;
    let summary =
        process_one_repo_f0(&root_dir, &db_path, &spreadsheet_id, cols, &repo_id, row_number, None)
            .await;
    if summary.outcome == RepoOutcome::Error {
        let detail = summary
            .error
            .unwrap_or_else(|| "Erro não especificado".to_string());
        return Err(io::Error::other(format!("F0 falhou para {}: {}", summary.repo_id, detail)).into());
    }
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
                batch: false,
                direct: false
            }
        );

        let args = vec!["bin".to_string(), "--batch".to_string()];
        assert_eq!(
            parse_cli_args_from(args),
            CliArgs {
                repo_id: None,
                batch: true,
                direct: false
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
