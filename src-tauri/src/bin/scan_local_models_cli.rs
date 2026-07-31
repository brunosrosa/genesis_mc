use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};
use walkdir::WalkDir;
use souls_mc_lib::core::model_registry::{
    parse_gguf_metadata_zero_copy, parse_safetensors_metadata_zero_copy, resolve_db_path,
    ModelMetadata,
};

fn collect_model_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(dir).max_depth(5).into_iter().flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "gguf" || ext_str == "safetensors" {
                    files.push(path.to_path_buf());
                }
            }
        }
    }
    files
}

fn init_sqlite_vault(db_path: &Path) -> Connection {
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let conn = Connection::open(db_path).expect("Falha ao abrir souls_heuristic_vault.db");
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS local_models (
            file_path TEXT PRIMARY KEY,
            model_name TEXT NOT NULL DEFAULT '',
            family TEXT NOT NULL,
            parameters TEXT NOT NULL,
            context_length INTEGER NOT NULL,
            quantization TEXT NOT NULL,
            capabilities TEXT NOT NULL DEFAULT '[]',
            file_size_bytes INTEGER NOT NULL,
            last_seen TEXT NOT NULL
        );",
        [],
    )
    .expect("Falha ao criar tabela local_models");

    let _ = conn.execute("ALTER TABLE local_models ADD COLUMN model_name TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE local_models ADD COLUMN capabilities TEXT NOT NULL DEFAULT '[]';", []);

    conn.execute_batch(
        "DROP VIEW IF EXISTS vw_finops_routing;
         CREATE VIEW vw_finops_routing AS
         SELECT 
            file_path,
            model_name,
            family,
            parameters,
            context_length,
            quantization,
            capabilities,
            file_size_bytes,
            last_seen
         FROM local_models
         ORDER BY 
            family ASC,
            CASE 
                WHEN capabilities LIKE '%MTP%' THEN 0
                WHEN capabilities LIKE '%THINKING%' THEN 1
                WHEN capabilities LIKE '%TOOL_CALLING%' THEN 2
                ELSE 3
            END ASC,
            CAST(
                REPLACE(
                    REPLACE(UPPER(parameters), 'B', ''),
                    'UNKNOWN', '999999'
                ) AS REAL
            ) ASC,
            file_size_bytes DESC;"
    )
    .expect("Falha ao criar VIEW vw_finops_routing");

    conn
}

fn sync_state_to_vault(conn: &mut Connection, scanned_models: &[ModelMetadata]) {
    let tx = conn.transaction().expect("Falha ao iniciar transacao SQLite");

    for m in scanned_models {
        let caps_json = serde_json::to_string(&m.capabilities).unwrap_or_else(|_| "[]".to_string());
        tx.execute(
            "INSERT INTO local_models (file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, DATETIME('now'))
             ON CONFLICT(file_path) DO UPDATE SET
                model_name=excluded.model_name,
                family=excluded.family,
                parameters=excluded.parameters,
                context_length=excluded.context_length,
                quantization=excluded.quantization,
                capabilities=excluded.capabilities,
                file_size_bytes=excluded.file_size_bytes,
                last_seen=excluded.last_seen;",
            params![
                m.file_path,
                m.model_name,
                m.family,
                m.parameters,
                m.context_length as i64,
                m.quantization,
                caps_json,
                m.file_size_bytes as i64,
            ],
        )
        .expect("Falha ao realizar UPSERT em local_models");
    }

    let mut stmt = tx.prepare("SELECT file_path FROM local_models").unwrap();
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap();

    let mut orphaned = Vec::new();
    for row in rows.flatten() {
        if !Path::new(&row).exists() {
            orphaned.push(row);
        }
    }
    drop(stmt);

    for orphan in &orphaned {
        tx.execute("DELETE FROM local_models WHERE file_path = ?1", params![orphan])
            .expect("Falha ao deletar modelo orfao");
    }

    tx.commit().expect("Falha ao efetuar commit no SQLite Vault");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let default_target = "C:\\Users\\rosas\\.lmstudio\\models".to_string();
    let target_path_str = args.get(1).unwrap_or(&default_target);
    let target = Path::new(target_path_str);

    if !target.exists() {
        eprintln!("ERR: Caminho alvo nao encontrado: {}", target.display());
        std::process::exit(1);
    }

    let files = collect_model_files(target);
    let mut scanned_models = Vec::new();

    for file_path in &files {
        if let Some(m) = parse_gguf_metadata_zero_copy(file_path) {
            let size_mb = (m.file_size_bytes / (1024 * 1024)) as u64;
            let is_ssm = m.family.to_lowercase().contains("mamba") || m.family.to_lowercase().contains("zamba");
            let profile = souls_mc_lib::core::model_manager::profile_gguf_vram(
                &m.model_name,
                size_mb,
                m.context_length as u32,
                is_ssm,
            );
            match profile {
                Ok(p) => {
                    println!("[PROFILER-OK] {} -> VRAM Estimada: {} MB", m.model_name, p.total_vram_projected_mb);
                }
                Err(e) => {
                    eprintln!("[PROFILER-WARN] {} -> Rejeitado por VRAM Overbudget: {:?}", m.model_name, e);
                }
            }
            scanned_models.push(m);
        } else if let Some(m) = parse_safetensors_metadata_zero_copy(file_path) {
            scanned_models.push(m);
        }
    }

    let db_path = resolve_db_path();
    let mut conn = init_sqlite_vault(&db_path);
    sync_state_to_vault(&mut conn, &scanned_models);

    println!(
        "SUCCESS: Scanner finalizado com sucesso. Modelos escaneados e sincronizados: {}",
        scanned_models.len()
    );

    let mut stmt = conn
        .prepare("SELECT file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes FROM vw_finops_routing LIMIT 5")
        .unwrap();

    let model_rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })
        .unwrap();

    println!("\n=== TOP MODELOS ROTEADOS PELA VIEW FINOPS (vw_finops_routing) ===");
    for (idx, r) in model_rows.flatten().enumerate() {
        println!(
            "[{}] Nome: {:<35} | Família: {:<10} | Params: {:<6} | Contexto: {:<6} | Quant: {:<8} | Caps: {:<25} | Size: {:.2} GB",
            idx + 1,
            r.1,
            r.2,
            r.3,
            r.4,
            r.5,
            r.6,
            (r.7 as f64) / (1024.0 * 1024.0 * 1024.0)
        );
    }

    std::process::exit(0);
}
