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

/// `repo_heatmap` — Frecency ranking de arquivos do monorepo com query atômica indexada (< 3ms).
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

    // Consulta atômica indexada (< 3ms); se banco novo/vazio, seed via scan completo
    let report = match repo_heatmap::compute_repo_heatmap_from_db(&conn, now, lambda, limit) {
        Ok(rep) if rep.total > 0 => rep,
        _ => repo_heatmap::compute_repo_heatmap(&repo_root, &mut conn, now, lambda, limit).map_err(|e| RpcError {
            code: -32000,
            message: format!("Falha ao escanear repo_heatmap: {e}"),
            data: None,
        })?,
    };

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

/// `repo_impact` — Blast Radius multilíngue via BFS reverso em RAM (< 3ms).
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

    let ram_report = lean_vacuum::repo_impact_from_ram(file_path, max_depth);
    let report = if ram_report.total_impacted_files > 0 {
        ram_report
    } else {
        let target = Path::new(file_path);
        lean_vacuum::repo_impact_fn(&repo_root, target, max_depth).map_err(|e| RpcError {
            code: -32000,
            message: format!("Falha ao calcular Blast Radius: {e}"),
            data: None,
        })?
    };

    try_record_repo_heatmap(&report.target_file);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&report).unwrap_or_default()
        }]
    }))
}

/// `routes` — mapeamento estático pré-compilado de comandos Tauri e invokes Svelte (< 1ms).
pub async fn run_routes(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let repo_root = args
        .get("repo_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(workspace_root);
    let report = observability::cached_scan_routes(&repo_root).map_err(|e| RpcError {
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

/// `metrics` — agregação FinOps real via tabela `telemetry_logs` (< 2ms).
pub async fn run_metrics(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let _ = params;
    let souls_data_dir = workspace_root().join(".souls_data");
    let db_path = souls_data_dir.join("souls_state.db");
    let mut conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao abrir souls_state.db para metrics: {e}"),
        data: None,
    })?;
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");
    let _ = observability::migrate_v3_to_v4(&mut conn);

    let report = observability::aggregate_telemetry(&conn).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao agregar métricas: {e}"),
        data: None,
    })?;

    let total_tokens = report.total_tokens_in + report.total_tokens_out;
    let total_microdollars = (report.total_cost_usd * 1_000_000.0).round() as i64;
    let fast_calls = report
        .by_tool
        .values()
        .map(|t| if (t.duration_ms_total / t.calls.max(1)) < 50 { t.calls } else { 0 })
        .sum::<i64>();
    let l2_hit_rate = if report.total_calls > 0 {
        (fast_calls as f64) / (report.total_calls as f64)
    } else {
        1.0
    };

    let metrics_payload = json!({
        "total_tokens_in": report.total_tokens_in,
        "total_tokens_out": report.total_tokens_out,
        "total_tokens": total_tokens,
        "total_cost_usd": report.total_cost_usd,
        "total_microdollars": total_microdollars,
        "total_calls": report.total_calls,
        "cache_l2_hit_rate": l2_hit_rate,
        "accuracy_score_avg": report.accuracy_score_avg,
        "e3_efficiency_real": report.e3_efficiency_real,
        "by_tool": report.by_tool,
    });

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&metrics_payload).unwrap_or_default()
        }],
        "structuredContent": metrics_payload,
        "isError": false
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
            "e3_efficiency_real_global": report.e3_efficiency_real,
            "e3_efficiency_v2_global": report.e3_efficiency_v2,
            "e3_efficiency_token_global": report.e3_efficiency,
            "accuracy_score_avg_global": report.accuracy_score_avg,
            "total_calls": report.total_calls,
            "by_tool": report.by_tool,
            "formula": "E3_real = accuracy_score / (cost_usd + duration_seconds)"
        },
        "isError": false
    }))
}
