use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use souls_mc_lib::harvester::community::{CommunityMetaFetcher, RateLimiter};
use souls_mc_lib::harvester::extract::render_community_meta_dossier;
use rusqlite::{params, Connection, OptionalExtension};
use url::Url;

const TARGET_ARTIFACT: &str = "blob_09_community_meta";

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
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
    .map_err(|e| io::Error::other(format!("Falha ao criar tabela repositorios: {e}")))?;

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
    .map_err(|e| io::Error::other(format!("Falha ao criar tabela artefatos_brutos: {e}")))?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_artefatos_repo_tipo
         ON artefatos_brutos(repo_id, artifact_type)",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar índice único de artefatos: {e}")))?;

    Ok(())
}

fn parse_repo_id() -> io::Result<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--repo" {
            return args
                .next()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| io::Error::other("Uso: cargo run --bin refresh_blob09_cli -- --repo owner/repo"));
        }
    }
    Err(io::Error::other(
        "Parametro obrigatorio ausente. Uso: cargo run --bin refresh_blob09_cli -- --repo owner/repo",
    ))
}

fn repo_url_for_refresh(conn: &Connection, repo_id: &str) -> io::Result<Url> {
    let maybe_repo_url: Option<String> = conn
        .query_row(
            "SELECT repo_url FROM repositorios WHERE project_name = ?1 LIMIT 1",
            [repo_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| io::Error::other(format!("Falha ao consultar repo_url para {repo_id}: {e}")))?;

    let fallback = format!("https://github.com/{repo_id}");
    Url::parse(maybe_repo_url.as_deref().unwrap_or(&fallback))
        .map_err(|e| io::Error::other(format!("repo_url invalida para {repo_id}: {e}")))
}

fn now_epoch_secs() -> io::Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("Falha ao calcular timestamp atual: {e}")))?
        .as_secs() as i64)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let repo_id = parse_repo_id()?;
    let root_dir = workspace_root()?;
    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    std::fs::create_dir_all(db_path.parent().ok_or("db parent")?)?;

    let conn = Connection::open(&db_path)?;
    ensure_phase1_schema(&conn)?;
    let repo_url = repo_url_for_refresh(&conn, &repo_id)?;

    let limiter = RateLimiter;
    let payload = CommunityMetaFetcher::fetch(&repo_url, &limiter)
        .await
        .map_err(|e| io::Error::other(format!("Falha ao recapturar {TARGET_ARTIFACT} para {repo_id}: {e}")))?;
    let payload_blob = render_community_meta_dossier(&payload, None);
    let payload_bytes = payload_blob.len();
    let timestamp = now_epoch_secs()?;

    conn.execute(
        "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(repo_id, artifact_type) DO UPDATE SET
            payload_blob = excluded.payload_blob,
            timestamp_extracao = excluded.timestamp_extracao",
        params![repo_id, TARGET_ARTIFACT, payload_blob, timestamp],
    )?;

    println!("repo_id={repo_id}");
    println!("repo_url={repo_url}");
    println!("artifact_type={TARGET_ARTIFACT}");
    println!("payload_bytes={payload_bytes}");
    println!("full_name={}", payload.full_name.as_deref().unwrap_or("N/A"));
    println!("stars_count={}", payload.stars_count);
    println!("open_prs_count={}", payload.open_prs_count);
    println!("extracted_at={}", payload.extracted_at.to_rfc3339());

    Ok(())
}
