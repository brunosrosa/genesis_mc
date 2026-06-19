use rusqlite::Connection;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TARGET_REPO_ID: &str = "bytedance/Lance";
const EXPECTED_BLOBS: [&str; 11] = [
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

struct RawBlobRow {
    artifact_type: String,
    payload_blob: Vec<u8>,
    timestamp_extracao: i64,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn now_epoch_secs() -> io::Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| io::Error::other(format!("Falha ao ler relogio do sistema: {err}")))
}

fn audit_report_path(root: &Path, repo_id: &str) -> PathBuf {
    let safe_repo = repo_id.replace(['/', '\\'], "_");
    root.join(".soda_scratchpad")
        .join("reports")
        .join(format!("_RAW_AUDIT_{safe_repo}.md"))
}

fn load_raw_blobs(conn: &Connection, repo_id: &str) -> io::Result<Vec<RawBlobRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT artifact_type, payload_blob, timestamp_extracao
             FROM artefatos_brutos
             WHERE repo_id = ?1
             ORDER BY artifact_type ASC",
        )
        .map_err(|err| io::Error::other(format!("Falha ao preparar query de auditoria: {err}")))?;

    let rows = stmt
        .query_map([repo_id], |row| {
            Ok(RawBlobRow {
                artifact_type: row.get(0)?,
                payload_blob: row.get(1)?,
                timestamp_extracao: row.get(2)?,
            })
        })
        .map_err(|err| io::Error::other(format!("Falha ao executar query de auditoria: {err}")))?;

    let mut blobs = Vec::new();
    for row in rows {
        blobs.push(row.map_err(|err| io::Error::other(format!("Falha ao ler blob bruto: {err}")))?);
    }
    Ok(blobs)
}

fn render_report(repo_id: &str, db_path: &Path, blobs: &[RawBlobRow], generated_at: u64) -> String {
    let found = blobs
        .iter()
        .map(|row| row.artifact_type.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let missing = EXPECTED_BLOBS
        .iter()
        .copied()
        .filter(|artifact| !found.contains(artifact))
        .collect::<Vec<_>>();

    let mut output = String::new();
    output.push_str("# RAW Audit\n\n");
    output.push_str(&format!("- repo_id: `{repo_id}`\n"));
    output.push_str(&format!("- db_path: `{}`\n", db_path.display()));
    output.push_str(&format!("- generated_at_epoch: `{generated_at}`\n"));
    output.push_str(&format!("- blob_count: `{}`\n", blobs.len()));
    output.push_str(&format!("- expected_blob_count: `{}`\n", EXPECTED_BLOBS.len()));
    if missing.is_empty() {
        output.push_str("- missing_blobs: `none`\n");
    } else {
        output.push_str(&format!("- missing_blobs: `{}`\n", missing.join(", ")));
    }
    output.push('\n');

    for row in blobs {
        let payload_text = String::from_utf8_lossy(&row.payload_blob);
        output.push_str(&format!("## {}\n\n", row.artifact_type));
        output.push_str(&format!("- extracted_at_epoch: `{}`\n", row.timestamp_extracao));
        output.push_str(&format!("- payload_bytes: `{}`\n\n", row.payload_blob.len()));
        output.push_str("````text\n");
        output.push_str(&payload_text);
        if !payload_text.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("````\n\n");
    }

    if !missing.is_empty() {
        output.push_str("## Missing Blobs\n\n");
        for artifact in missing {
            output.push_str(&format!("- `{artifact}`\n"));
        }
    }

    output
}

fn main() -> io::Result<()> {
    let repo_id = std::env::args()
        .nth(1)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| TARGET_REPO_ID.to_string());

    let root = workspace_root();
    let db_path = root.join(".soda_data").join("soda_heuristic_vault.db");
    let report_path = audit_report_path(&root, &repo_id);
    let generated_at = now_epoch_secs()?;

    let conn = Connection::open(&db_path)
        .map_err(|err| io::Error::other(format!("Falha ao abrir SQLite de auditoria: {err}")))?;
    let blobs = load_raw_blobs(&conn, &repo_id)?;
    let report = render_report(&repo_id, &db_path, &blobs, generated_at);

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, report)?;

    let found = blobs
        .iter()
        .map(|row| row.artifact_type.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let missing = EXPECTED_BLOBS
        .iter()
        .copied()
        .filter(|artifact| !found.contains(artifact))
        .collect::<Vec<_>>();

    println!("RAW audit report written to {}", report_path.display());
    println!("repo_id={repo_id} blob_count={}", blobs.len());

    if !missing.is_empty() {
        return Err(io::Error::other(format!(
            "Auditoria incompleta: blobs ausentes para {repo_id}: {}",
            missing.join(", ")
        )));
    }

    Ok(())
}
