use souls_mc_lib::harvester::extract::LocalStaticExtractor;
use souls_mc_lib::harvester::git::BloblessCloner;
use souls_mc_lib::harvester::ramdisk::RamdiskAllocator;
use rusqlite::Connection;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use url::Url;

const TARGET_REPO_ID: &str = "bytedance/Lance";
const TARGET_BLOB_TYPE: &str = "blob_01_promessa_readme";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn output_report_path(root: &Path) -> PathBuf {
    root.join(".soda_scratchpad")
        .join("reports")
        .join("_TESTE_BLOB_01.md")
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

    let blob = LocalStaticExtractor::extract_all(repo_path.as_ref())
        .await
        .map_err(|err| to_io_error("Falha ao extrair blob_01_promessa_readme", err))?
        .into_iter()
        .find(|blob| blob.artifact_type == TARGET_BLOB_TYPE)
        .ok_or_else(|| io::Error::other("blob_01_promessa_readme nao foi produzido"))?;

    let content = String::from_utf8(blob.payload_blob)
        .map_err(|err| to_io_error("Falha ao decodificar blob_01 como UTF-8", err))?;

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, &content)?;

    println!("repo_id={TARGET_REPO_ID}");
    println!("repo_url={repo_url}");
    println!("report_path={}", report_path.display());
    println!("blob_chars={}", content.chars().count());

    Ok(())
}
