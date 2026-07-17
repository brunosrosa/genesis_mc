use souls_mc_lib::harvester::detect::LanguageDetector;
use souls_mc_lib::harvester::git::BloblessCloner;
use souls_mc_lib::harvester::ramdisk::RamdiskAllocator;
use souls_mc_lib::harvester::repo_radar;
use souls_mc_lib::harvester::sandbox::{SandboxOrchestrator, SandboxPolicy};
use souls_mc_lib::harvester::sast::{PolyglotSastInput, PolyglotSastSidecar};
use rusqlite::Connection;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

const TARGET_REPO_ID: &str = "bytedance/Lance";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn output_report_path(root: &Path) -> PathBuf {
    root.join(".soda_scratchpad")
        .join("reports")
        .join("_TESTE_BLOB_08.json")
}

fn load_repo_url(conn: &Connection, repo_id: &str) -> io::Result<String> {
    conn.query_row(
        "SELECT repo_url FROM repositorios WHERE project_name = ?1",
        [repo_id],
        |row| row.get::<_, String>(0),
    )
    .map_err(|err| io::Error::other(format!("Falha ao carregar repo_url de {repo_id}: {err}")))
}

fn to_io_error(context: &str, err: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("{context}: {err}"))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let root = workspace_root();
    let db_path = root.join(".soda_data").join("soda_heuristic_vault.db");
    let report_path = output_report_path(&root);

    let conn = Connection::open(&db_path)
        .map_err(|err| to_io_error("Falha ao abrir soda_heuristic_vault.db", err))?;
    let repo_url = load_repo_url(&conn, TARGET_REPO_ID)?;
    let parsed_url = Url::parse(&repo_url)
        .map_err(|err| to_io_error("Falha ao parsear repo_url do banco", err))?;

    let mut workspace = RamdiskAllocator::allocate(256)
        .await
        .map_err(|err| to_io_error("Falha ao alocar workspace efemero", err))?;
    let repo_path = BloblessCloner::clone(&parsed_url, &mut workspace)
        .await
        .map_err(|err| to_io_error("Falha ao clonar workspace efemero", err))?;
    let profile = LanguageDetector::detect(&repo_path)
        .await
        .map_err(|err| to_io_error("Falha ao detectar stack base", err))?;
    let sandbox = Arc::new(
        SandboxOrchestrator::create(&repo_path, SandboxPolicy::ReadWrite)
        .await
        .map_err(|err| to_io_error("Falha ao criar sandbox efemero", err))?
    );

    let clean_files = Arc::new(repo_radar::build_repo_radar(&repo_path).clean_files().to_vec());
    let artifacts = PolyglotSastSidecar::extract(PolyglotSastInput {
        executor: Arc::clone(&sandbox),
        timeout_secs: 120,
        profile: &profile,
        clean_files,
    })
    .await
    .map_err(|err| to_io_error("Falha ao extrair blob_08 poliglota", err))?;

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, &artifacts.health_report_blob)?;

    println!("repo_id={TARGET_REPO_ID}");
    println!("repo_url={repo_url}");
    println!("profile={profile:?}");
    println!("report_path={}", report_path.display());
    println!("blob_08_bytes={}", artifacts.health_report_blob.len());

    drop(sandbox);
    workspace
        .cleanup()
        .await
        .map_err(|err| to_io_error("Falha ao limpar workspace efemero", err))?;

    Ok(())
}
