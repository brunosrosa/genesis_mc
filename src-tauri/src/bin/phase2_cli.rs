use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use genesis_mc_lib::cognition::phase2_swarm::{
    ensure_phase2_schema, CognitiveSwarmDispatcher, HttpLensInvoker, SqliteDebateStore,
};
use rusqlite::{params, Connection};
use tracing::info;

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

fn ensure_phase2_cli_schema(conn: &Connection) -> io::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS repositorios (
            project_name TEXT PRIMARY KEY,
            lote_id TEXT,
            repo_url TEXT,
            repo_version TEXT,
            ultima_versao_online TEXT,
            soda_universal_uuid TEXT,
            status_processamento TEXT NOT NULL,
            timestamp_fase_1 INTEGER,
            timestamp_fase_3 INTEGER,
            retry_count INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar/verificar tabela repositorios: {}", e)))?;

    let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN repo_version TEXT", []);
    let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN ultima_versao_online TEXT", []);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pacotes_destilados (
            package_id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL,
            package_name TEXT NOT NULL,
            payload_package TEXT NOT NULL,
            timestamp_empacotamento INTEGER NOT NULL,
            UNIQUE(repo_id, package_name)
        )",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar/verificar tabela pacotes_destilados: {}", e)))?;

    ensure_phase2_schema(conn).map_err(io::Error::other)?;
    Ok(())
}

fn mark_repo_running_if_present(conn: &Connection, repo_id: &str) -> io::Result<()> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM repositorios WHERE project_name = ?1",
            params![repo_id],
            |row| row.get(0),
        )
        .map_err(|e| io::Error::other(format!("Falha ao consultar repositorio {}: {}", repo_id, e)))?;

    if exists == 0 {
        conn.execute(
            "INSERT INTO repositorios
                (project_name, lote_id, repo_url, soda_universal_uuid, status_processamento, retry_count)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![
                repo_id,
                "LOTE_02_PILOTO",
                format!("https://github.com/{}", repo_id),
                format!("UUID-{}", repo_id),
                "FASE_2_RUNNING"
            ],
        )
        .map_err(|e| io::Error::other(format!("Falha ao registrar repositorio {}: {}", repo_id, e)))?;
    } else {
        conn.execute(
            "UPDATE repositorios
             SET status_processamento = ?1
             WHERE project_name = ?2",
            params!["FASE_2_RUNNING", repo_id],
        )
        .map_err(|e| io::Error::other(format!("Falha ao marcar FASE_2_RUNNING: {}", e)))?;
    }

    Ok(())
}

fn fetch_debate_row(
    conn: &Connection,
    repo_id: &str,
) -> io::Result<(String, String, String, String, String)> {
    conn.query_row(
        "SELECT lens_a_json, lens_b_json, lens_c_json, model_used, phase_status
         FROM debates_enxame
         WHERE repo_id = ?1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    )
    .map_err(|e| io::Error::other(format!("Falha ao buscar debate persistido de {}: {}", repo_id, e)))
}

struct Phase2Report<'a> {
    repo_id: &'a str,
    elapsed_ms: u128,
    lens_a_json: &'a str,
    lens_b_json: &'a str,
    lens_c_json: &'a str,
    model_used: &'a str,
    phase_status: &'a str,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
}

fn extract_usage_totals_from_lens_json(lens_json: &str) -> (u64, u64, u64, f64) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(lens_json) else {
        return (0, 0, 0, 0.0);
    };
    let prompt_tokens = value.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let completion_tokens = value
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total_tokens = value.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let total_cost_usd = value
        .get("total_cost_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    (prompt_tokens, completion_tokens, total_tokens, total_cost_usd)
}

fn write_phase2_report(root_dir: &Path, report_data: &Phase2Report<'_>) -> io::Result<PathBuf> {
    let reports_dir = root_dir.join(".soda_scratchpad").join("reports");
    std::fs::create_dir_all(&reports_dir)
        .map_err(|e| io::Error::other(format!("Falha ao criar reports_dir: {}", e)))?;
    let report_path = reports_dir.join(format!(
        "_PHASE2_REPORT_{}_V6.txt",
        sanitize_repo_id(report_data.repo_id)
    ));
    let mut report = String::new();
    report.push_str(&format!("repo_id={}\n", report_data.repo_id));
    report.push_str(&format!("elapsed_ms={}\n", report_data.elapsed_ms));
    report.push_str(&format!("phase_status={}\n", report_data.phase_status));
    report.push_str("persisted_in=debates_enxame\n");
    report.push_str(&format!("model_used={}\n", report_data.model_used));
    report.push_str(&format!("prompt_tokens={}\n", report_data.prompt_tokens));
    report.push_str(&format!(
        "completion_tokens={}\n",
        report_data.completion_tokens
    ));
    report.push_str(&format!("total_tokens={}\n", report_data.total_tokens));
    report.push_str(&format!("total_cost_usd={:.6}\n", report_data.total_cost_usd));
    report.push('\n');
    report.push_str("\n== LENS A ==\n");
    report.push_str(report_data.lens_a_json);
    report.push_str("\n\n== LENS B ==\n");
    report.push_str(report_data.lens_b_json);
    report.push_str("\n\n== LENS C ==\n");
    report.push_str(report_data.lens_c_json);
    report.push('\n');

    std::fs::write(&report_path, report).map_err(|e| {
        io::Error::other(format!(
            "Falha ao exportar relatório da Fase 2 em {}: {}",
            report_path.display(),
            e
        ))
    })?;

    Ok(report_path)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let level = match rust_log.to_ascii_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    let started = Instant::now();
    let repo_id = parse_repo_id_from_args();

    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();
    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path).map_err(|e| {
        io::Error::other(format!("Falha ao abrir vault em {}: {}", db_path.display(), e))
    })?;

    ensure_phase2_cli_schema(&conn)?;
    mark_repo_running_if_present(&conn, &repo_id)?;
    info!(repo_id = %repo_id, "Fase 2: schema verificado e repositório preparado");

    let store = SqliteDebateStore::new(Arc::new(Mutex::new(conn)));
    let invoker = HttpLensInvoker::from_openrouter_env().map_err(io::Error::other)?;
    let dispatcher = CognitiveSwarmDispatcher::new(store, invoker);

    dispatcher
        .dispatch_swarm(&repo_id)
        .await
        .map_err(|e| io::Error::other(format!("Falha ao executar enxame cognitivo: {}", e)))?;

    let conn = Connection::open(&db_path).map_err(|e| {
        io::Error::other(format!("Falha ao reabrir vault em {}: {}", db_path.display(), e))
    })?;
    let (lens_a_json, lens_b_json, lens_c_json, model_used, phase_status) =
        fetch_debate_row(&conn, &repo_id)?;

    let (p_a, c_a, t_a, cost_a) = extract_usage_totals_from_lens_json(&lens_a_json);
    let (p_b, c_b, t_b, cost_b) = extract_usage_totals_from_lens_json(&lens_b_json);
    let (p_c, c_c, t_c, cost_c) = extract_usage_totals_from_lens_json(&lens_c_json);
    let prompt_tokens = p_a.saturating_add(p_b).saturating_add(p_c);
    let completion_tokens = c_a.saturating_add(c_b).saturating_add(c_c);
    let total_tokens = t_a.saturating_add(t_b).saturating_add(t_c);
    let total_cost_usd = cost_a + cost_b + cost_c;

    let report_path = write_phase2_report(
        &root_dir,
        &Phase2Report {
            repo_id: &repo_id,
            elapsed_ms: started.elapsed().as_millis(),
            lens_a_json: &lens_a_json,
            lens_b_json: &lens_b_json,
            lens_c_json: &lens_c_json,
            model_used: &model_used,
            phase_status: &phase_status,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            total_cost_usd,
        },
    )?;

    info!(
        repo_id = %repo_id,
        report = %report_path.display(),
        elapsed_ms = started.elapsed().as_millis(),
        now_epoch = now_epoch_secs()?,
        "Fase 2 concluída com persistência atômica"
    );

    println!("PHASE_2_OK repo_id={} report={}", repo_id, report_path.display());
    println!("--- LENS A ---\n{}", lens_a_json);
    println!("--- LENS B ---\n{}", lens_b_json);
    println!("--- LENS C ---\n{}", lens_c_json);
    println!("--- MODEL USED ---\n{}", model_used);
    println!("Persistido em debates_enxame com status={}", phase_status);

    Ok(())
}
