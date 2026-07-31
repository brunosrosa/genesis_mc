use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use souls_mc_lib::harvester::canon::{SodaCanonExtractor, CANON_GLOBAL_REPO_ID};

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
            "file://local/docs/SODA_CANON_MANIFEST.md",
            "UUID-SODA-CANON-GLOBAL",
            "CACHE_GLOBAL",
            0_i64,
            0_i64
        ],
    )
    .map_err(|e| io::Error::other(format!("Falha ao registrar linha sintética do cache canônico global: {}", e)))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root_dir = workspace_root()?;
    let db_path = root_dir.join(".souls_data").join("souls_heuristic_vault.db");
    std::fs::create_dir_all(db_path.parent().ok_or("db parent")?)?;

    let conn = Connection::open(&db_path)?;
    ensure_phase1_schema(&conn)?;

    let repo_id = "LOCAL_CANON_REFRESH";
    conn.execute(
        "INSERT OR IGNORE INTO repositorios
         (project_name, lote_id, repo_url, soda_universal_uuid, status_processamento, timestamp_fase_1, retry_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            repo_id,
            "LOCAL",
            "file://local/souls_mc",
            "UUID-LOCAL-CANON-REFRESH",
            "LOCAL",
            0_i64,
            0_i64
        ],
    )?;

    conn.execute(
        "DELETE FROM artefatos_brutos
         WHERE artifact_type = 'blob_10_soda_canon_context'
           AND repo_id IN (?1, ?2)",
        params![repo_id, CANON_GLOBAL_REPO_ID],
    )?;

    let conn = Arc::new(Mutex::new(conn));
    SodaCanonExtractor::extract(repo_id, Arc::clone(&conn)).await?;

    let payload: String = {
        let conn_guard = conn.lock().map_err(|_| "VaultDb lock poisoned")?;
        conn_guard.query_row(
            "SELECT CAST(payload_blob AS TEXT)
             FROM artefatos_brutos
             WHERE repo_id = ?1 AND artifact_type = 'blob_10_soda_canon_context'
             LIMIT 1",
            params![repo_id],
            |row| row.get(0),
        )?
    };

    let char_count = payload.chars().count();
    println!("\n=== BLOB_10 UPDATED (Canon V5) ===");
    println!("DB: {}", db_path.display());
    println!("repo_id: {}", repo_id);
    println!("chars: {}", char_count);
    println!("bytes: {}", payload.len());

    println!("\n--- FIRST 10 LINES ---");
    for line in payload.lines().take(10) {
        println!("{}", line);
    }
    println!("----------------------\n");

    Ok(())
}
