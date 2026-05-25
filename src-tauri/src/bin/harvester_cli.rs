use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use genesis_mc_lib::harvester::canon::CANON_GLOBAL_REPO_ID;
use genesis_mc_lib::harvester::orchestrator::HarvesterOrchestrator;
use rusqlite::{params, Connection};
use tracing::{error, info};
use url::Url;

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

fn ensure_phase1_schema(conn: &Connection) -> io::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS repositorios (
            project_name TEXT PRIMARY KEY,
            lote_id TEXT NOT NULL,
            repo_url TEXT NOT NULL UNIQUE,
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

fn write_phase1_report(
    root_dir: &Path,
    conn_arc: &Arc<Mutex<Connection>>,
    repo_id: &str,
) -> io::Result<PathBuf> {
    let reports_dir = root_dir.join(".soda_scratchpad").join("reports");
    std::fs::create_dir_all(&reports_dir)
        .map_err(|e| io::Error::other(format!("Falha ao criar reports_dir: {}", e)))?;
    let report_path = reports_dir.join(format!("_PHASE1_REPORT_{}.txt", sanitize_repo_id(repo_id)));
    let rows = {
        let conn = conn_arc.lock().map_err(|e| {
            io::Error::other(format!("Falha ao adquirir lock do banco para relatório da Fase 1: {}", e))
        })?;
        let mut stmt = conn
            .prepare(
                "SELECT artifact_type, LENGTH(payload_blob)
                 FROM artefatos_brutos
                 WHERE repo_id = ?1
                 ORDER BY artifact_type ASC",
            )
            .map_err(|e| io::Error::other(format!("Falha ao preparar query do relatório da Fase 1: {}", e)))?;
        let iter = stmt
            .query_map([repo_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| io::Error::other(format!("Falha ao executar query do relatório da Fase 1: {}", e)))?;

        let mut rows = Vec::new();
        for row in iter {
            rows.push(row.map_err(|e| io::Error::other(format!("Falha ao ler linha do relatório da Fase 1: {}", e)))?);
        }
        rows
    };

    if rows.is_empty() {
        return Err(io::Error::other("A Fase 1 terminou sem blobs persistidos para o relatório"));
    }

    let mut report = String::new();
    report.push_str(&format!("repo_id={}\n", repo_id));
    report.push_str("artifact_type\tpayload_bytes\n");
    for (artifact_type, payload_len) in rows {
        report.push_str(&format!("{}\t{}\n", artifact_type, payload_len));
    }

    std::fs::write(&report_path, report).map_err(|e| {
        io::Error::other(format!(
            "Falha ao exportar relatório local da Fase 1 em {}: {}",
            report_path.display(),
            e
        ))
    })?;

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

    info!("SODA Fase 1: Iniciando extração isolada");
    let started = Instant::now();

    let root_dir = workspace_root()?;
    let soda_data_dir = root_dir.join(".soda_data");
    tokio::fs::create_dir_all(&soda_data_dir).await?;

    let db_path = soda_data_dir.join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path)?;
    ensure_phase1_schema(&conn)?;

    let repo_id = parse_repo_id_from_args();
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

    info!(repo_id = %repo_id, "Registro base inserido/verificado. Iniciando orquestração da Fase 1");

    let conn_arc = Arc::new(Mutex::new(conn));

    match HarvesterOrchestrator::run(&repo_id, &repo_url, Arc::clone(&conn_arc)).await {
        Ok(_) => {
            info!(repo_id = %repo_id, "CLI: HarvesterOrchestrator retornou OK; atualizando status para FASE_1_OK");
            {
                let conn_lock = conn_arc.lock().map_err(|e| {
                    io::Error::other(format!("Falha ao adquirir lock do banco após Fase 1: {}", e))
                })?;
                conn_lock.execute(
                    "UPDATE repositorios
                     SET status_processamento = ?1,
                         timestamp_fase_1 = ?2
                     WHERE project_name = ?3",
                    params!["FASE_1_OK", now_epoch_secs()?, &repo_id],
                )?;
            }

            info!(repo_id = %repo_id, "CLI: Status FASE_1_OK persistido; exportando relatorio local");
            let report_path = write_phase1_report(&root_dir, &conn_arc, &repo_id)?;
            info!(
                repo_id = %repo_id,
                report = %report_path.display(),
                elapsed_ms = started.elapsed().as_millis(),
                "Fase 1 concluída; relatório local exportado"
            );
            return Ok(());
        }
        Err(e) => {
            error!(repo_id = %repo_id, error = %e, "Falha crítica na Fase 1");
            let conn_lock = conn_arc.lock().map_err(|lock_err| {
                io::Error::other(format!("Falha ao adquirir lock do banco no erro da Fase 1: {}", lock_err))
            })?;
            conn_lock.execute(
                "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                params!["ERRO_FASE_1", &repo_id],
            )?;
            return Err(e.into());
        }
    }
}
