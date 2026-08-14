use std::path::{Path, PathBuf};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use souls_mc_lib::cognition::{lean_vacuum, observability};
use crate::{
    extract_arguments, try_record_repo_heatmap, workspace_root, RpcError,
};

/// `heatmap` — mapeia arquivos quentes via Langevin decay (lambda=0.05).
pub async fn run_heatmap(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let limit: usize = args
        .get("limit")
        .and_then(Value::as_i64)
        .map(|v| v.max(1) as usize)
        .unwrap_or(50);
    let lambda: f64 = args
        .get("lambda")
        .and_then(Value::as_f64)
        .unwrap_or(observability::heatmap::DEFAULT_LAMBDA);

    let souls_data_dir = workspace_root().join(".souls_data");
    let db_path = souls_data_dir.join("souls_state.db");
    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao abrir souls_state.db: {e}"),
        data: None,
    })?;
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let entries = observability::compute_heatmap(&conn, now, lambda, limit)
        .map_err(|e| RpcError {
            code: -32000,
            message: format!("Heatmap falhou: {e}"),
            data: None,
        })?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&json!({
                "lambda": lambda,
                "entries": entries,
            }))
            .unwrap_or_default()
        }],
        "structuredContent": {
            "lambda": lambda,
            "count": entries.len(),
            "entries": entries,
        },
        "isError": false
    }))
}

/// `repo_heatmap` — Frecency ranking de arquivos do monorepo.
pub async fn run_repo_heatmap(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let limit: usize = args
        .get("limit")
        .and_then(Value::as_i64)
        .map(|v| (v as usize).clamp(1, 500))
        .unwrap_or(50);
    use souls_mc_lib::cognition::lean_vacuum::repo_heatmap;

    let lambda: f64 = args
        .get("lambda")
        .and_then(Value::as_f64)
        .unwrap_or(repo_heatmap::DEFAULT_LAMBDA);
    let repo_root = args
        .get("repo_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(workspace_root);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let db_path = workspace_root().join(".souls_data").join("souls_state.db");
    let mut conn = rusqlite::Connection::open(&db_path).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao abrir SQLite para repo_heatmap: {e}"),
        data: None,
    })?;

    let report = repo_heatmap::compute_repo_heatmap(&repo_root, &mut conn, now, lambda, limit).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao escanear repo_heatmap: {e}"),
        data: None,
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&report).unwrap_or_default()
        }],
        "structuredContent": {
            "lambda": report.lambda,
            "now": report.now,
            "total": report.total,
            "entries_count": report.entries.len(),
        },
        "isError": false
    }))
}

/// `repo_impact` — Blast Radius multilíngue via BFS reverso no grafo de imports.
pub async fn run_repo_impact(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let file_path = args
        .get("file_path")
        .or_else(|| args.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento 'file_path' e obrigatorio".to_string(),
            data: None,
        })?;
    let max_depth = args
        .get("max_depth")
        .and_then(Value::as_u64)
        .map(|n| n.clamp(1, lean_vacuum::MAX_DEPTH_CEILING as u64) as u8)
        .unwrap_or(lean_vacuum::DEFAULT_MAX_DEPTH);
    let repo_root = args
        .get("repo_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(workspace_root);

    let target = Path::new(file_path);
    let report = lean_vacuum::repo_impact_fn(&repo_root, target, max_depth).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao calcular Blast Radius: {e}"),
        data: None,
    })?;

    try_record_repo_heatmap(&report.target_file);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&report).unwrap_or_default()
        }]
    }))
}

/// `routes` — varredura regex de comandos Tauri e invokes Svelte.
pub async fn run_routes(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let repo_root = args
        .get("repo_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(workspace_root);
    let report = observability::scan_routes(&repo_root).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao escanear rotas: {e}"),
        data: None,
    })?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&report).unwrap_or_default()
        }]
    }))
}

/// `feedback` — dump FinOps agregado com E3.
pub async fn run_feedback(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let _ = params;
    let souls_data_dir = workspace_root().join(".souls_data");
    let db_path = souls_data_dir.join("souls_state.db");
    let mut conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao abrir souls_state.db: {e}"),
        data: None,
    })?;
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");
    if let Err(e) = observability::migrate_v3_to_v4(&mut conn) {
        eprintln!("[feedback] ALERTA: migrate_v3_to_v4 falhou: {e}");
    }
    let report = observability::aggregate_telemetry(&conn).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao agregar telemetria: {e}"),
        data: None,
    })?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&report).unwrap_or_default()
        }],
        "structuredContent": {
            "e3_efficiency_v2_global": report.e3_efficiency_v2,
            "e3_efficiency_token_global": report.e3_efficiency,
            "accuracy_score_avg_global": report.accuracy_score_avg,
            "total_calls": report.total_calls,
            "by_tool": report.by_tool,
            "formula": "E3_v2 = (accuracy_score^2) / max(1.0, duration_ms)"
        },
        "isError": false
    }))
}
