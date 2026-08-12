use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use souls_mc_lib::cognition::{context, lean_vacuum};
#[cfg(feature = "gateway_ccr")]
use souls_mc_lib::cognition::context_compression;
use crate::{
    try_log_file_access, try_log_telemetry, try_record_repo_heatmap,
    validate_and_canonicalize_path, RpcError,
};

/// `souls_read` — Lê arquivo + Saco a Vácuo nativo.
pub async fn run_souls_read(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let path_str = arguments
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento path é obrigatório".to_string(),
            data: Some(json!({ "required": "path" })),
        })?;

    try_log_file_access(path_str, "read");
    try_record_repo_heatmap(path_str);

    let path = validate_and_canonicalize_path(path_str)?;
    if !path.exists() {
        return Err(RpcError {
            code: -32010,
            message: "Arquivo não existe".to_string(),
            data: Some(json!({ "path": path.display().to_string() })),
        });
    }
    if !path.is_file() {
        return Err(RpcError {
            code: -32011,
            message: "path não aponta para um arquivo regular".to_string(),
            data: Some(json!({ "path": path.display().to_string() })),
        });
    }

    let raw = std::fs::read_to_string(&path).map_err(|e| RpcError {
        code: -32012,
        message: "Falha ao ler arquivo (pode ser > 5MB ou binário)".to_string(),
        data: Some(json!({
            "path": path.display().to_string(),
            "reason": e.to_string(),
        })),
    })?;

    let original_chars = raw.chars().count();
    let ext = path.extension().and_then(|e| e.to_str());
    let compressed = lean_vacuum::compress_to_lean(&raw, ext);
    let compressed_chars = compressed.chars().count();

    let ratio = if original_chars == 0 {
        1.0
    } else {
        compressed_chars as f64 / original_chars as f64
    };
    let saved_pct = ((1.0 - ratio) * 100.0).round() as i64;

    let header = format!(
        "# {path} ({original}→{compressed} chars, {saved}% saved)\n\n",
        path = path.display(),
        original = original_chars,
        compressed = compressed_chars,
        saved = saved_pct,
    );
    let body = format!("```\n{compressed}\n```");

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("{header}{body}")
        }],
        "structuredContent": {
            "path": path.display().to_string(),
            "original_chars": original_chars,
            "compressed_chars": compressed_chars,
            "compression_ratio": ratio,
            "saved_percent": saved_pct,
            "ext": ext,
            "engine": "lean_vacuum.native (Fase 3)",
        },
        "isError": false
    }))
}

/// `souls_delta_diff` — Myers Diff estrutural via crate `similar` 2.7.0.
pub async fn run_souls_delta_diff(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let before = arguments
        .get("before")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento before é obrigatório (string)".to_string(),
            data: Some(json!({ "required": "before" })),
        })?;
    let after = arguments
        .get("after")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento after é obrigatório (string)".to_string(),
            data: Some(json!({ "required": "after" })),
        })?;

    let (text, stats) = lean_vacuum::myers_diff::myers_diff_with_stats(before, after);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "structuredContent": {
            "before_chars": before.chars().count(),
            "after_chars": after.chars().count(),
            "additions": stats.additions,
            "deletions": stats.deletions,
            "unchanged": stats.unchanged,
            "engine": "similar 2.7.0 (Myers)",
        },
        "isError": false
    }))
}

/// `souls_compress` — Aplica o compressor LEAN nativo ao texto.
pub async fn run_souls_compress(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let text = arguments
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento text é obrigatório".to_string(),
            data: Some(json!({ "required": "text" })),
        })?;
    let ext = arguments.get("ext").and_then(Value::as_str);

    let t_start = std::time::Instant::now();
    let tokens_in = lean_vacuum::count_tokens(text) as i64;
    let compressed = lean_vacuum::compress_to_lean(text, ext);
    let tokens_out = lean_vacuum::count_tokens(&compressed) as i64;
    let duration_ms = t_start.elapsed().as_millis() as i64;
    try_log_telemetry("souls_compress", tokens_in, tokens_out, 0.0, duration_ms, 1.0);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": compressed
        }],
        "structuredContent": {
            "compressed_text": compressed
        },
        "isError": false
    }))
}

/// `souls_dedup` — Deduplicação de blocos de 5+ linhas consecutivas.
pub async fn run_souls_dedup(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let text = arguments
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento text é obrigatório".to_string(),
            data: Some(json!({ "required": "text" })),
        })?;

    let path_opt = arguments
        .get("path")
        .or_else(|| arguments.get("file_path"))
        .and_then(Value::as_str)
        .map(Path::new);

    let t_start = std::time::Instant::now();
    let tokens_in = lean_vacuum::count_tokens(text) as i64;
    let deduplicated = lean_vacuum::deduplicate_blocks_session(text, path_opt);
    let tokens_out = lean_vacuum::count_tokens(&deduplicated) as i64;
    let duration_ms = t_start.elapsed().as_millis() as i64;
    try_log_telemetry("souls_dedup", tokens_in, tokens_out, 0.0, duration_ms, 1.0);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": deduplicated
        }],
        "structuredContent": {
            "deduplicated_text": deduplicated
        },
        "isError": false
    }))
}

/// `souls_headroom_retrieve` — Recupera um stub comprimido via `SoulsCcrStore::intercept_loopback`.
#[cfg(feature = "gateway_ccr")]
pub async fn run_souls_headroom_retrieve(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);

    let hash = args
        .get("hash")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'hash' ausente".to_string(),
            data: None,
        })?;

    let clean_hex = hash.trim().trim_start_matches("0x");
    if let Ok(hash_u64) = u64::from_str_radix(clean_hex, 16) {
        if let Some(entry) = context_compression::ccr_cache().get(&hash_u64) {
            return Ok(json!({
                "content": [{
                    "type": "text",
                    "text": entry.value().clone()
                }],
                "structuredContent": { "retrieved": true, "engine": "CCR_HOST_RAM_CACHE" },
                "isError": false
            }));
        }
    }

    let tool_call_json = json!({
        "headroom_retrieve": true,
        "hash": hash,
    })
    .to_string();

    let store = souls_mc_lib::core::headroom_engine::SoulsCcrStore::from_env();
    let retrieved = store.intercept_loopback(&tool_call_json);

    match retrieved {
        Some(payload) => Ok(json!({
            "content": [{
                "type": "text",
                "text": payload
            }],
            "structuredContent": { "retrieved": true },
            "isError": false
        })),
        None => Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("Hash '{hash}' nao encontrado no CCR store (loopback miss).")
            }],
            "structuredContent": { "retrieved": false },
            "isError": false
        })),
    }
}

/// `souls_smart_read` — Leitura Token-Aware com Auto-Shrink e Fail-Closed.
pub async fn run_souls_smart_read(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let path_str = arguments
        .get("file_path")
        .or_else(|| arguments.get("path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento file_path (ou path) é obrigatório para souls_smart_read".to_string(),
            data: None,
        })?;

    let budget = arguments
        .get("max_tokens_budget")
        .and_then(Value::as_u64)
        .unwrap_or(8000) as usize;

    let path: PathBuf = validate_and_canonicalize_path(path_str)?;
    let content = tokio::fs::read_to_string(path.as_path())
        .await
        .map_err(|e| RpcError {
            code: -32021,
            message: format!("Falha ao ler arquivo '{path_str}': {e}"),
            data: None,
        })?;

    let result_text = context::souls_smart_read::smart_read_text_for_lang(&content, budget, Some(path_str)).map_err(|(code, msg)| RpcError {
        code,
        message: msg,
        data: None,
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": result_text
        }],
        "structuredContent": {
            "path": path.display().to_string(),
            "max_tokens_budget": budget,
            "resulting_tokens": lean_vacuum::count_tokens(&result_text),
        },
        "isError": false
    }))
}

/// `multi_read` — Lê múltiplos arquivos em lote na RAM.
pub async fn run_souls_multi_read(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let paths = arguments
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'paths' (array de strings) ausente".to_string(),
            data: None,
        })?;

    let mut results = Vec::new();
    let mut files_map = serde_json::Map::new();
    for p_val in paths {
        if let Some(p_str) = p_val.as_str() {
            try_log_file_access(p_str, "multi_read");
            try_record_repo_heatmap(p_str);
            if let Ok(abs) = validate_and_canonicalize_path(p_str) {
                if let Ok(content) = tokio::fs::read_to_string(&abs).await {
                    let ext = abs.extension().and_then(|e| e.to_str());
                    let compressed = lean_vacuum::compress_to_lean(&content, ext);
                    files_map.insert(p_str.to_string(), Value::String(compressed.clone()));
                    results.push(json!({
                        "path": p_str,
                        "content": compressed
                    }));
                }
            }
        }
    }

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&results).unwrap_or_default()
        }],
        "structuredContent": {
            "read_count": results.len(),
            "files": files_map,
            "stats": {
                "ok_count": results.len(),
                "error_count": 0
            }
        },
        "isError": false
    }))
}

/// `fill` / `ccr_fill` — Reidrata marcadores de compressão CCR.
pub async fn run_souls_fill(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let text = args.get("text").and_then(Value::as_str).unwrap_or("");
    let expanded = souls_mc_lib::cognition::context_compression::dedup::rehydrate_ccr(text);
    Ok(json!({
        "content": [{
            "type": "text",
            "text": expanded
        }],
        "structuredContent": {
            "expanded": expanded,
            "original_tokens": lean_vacuum::count_tokens(text),
            "expanded_tokens": lean_vacuum::count_tokens(&expanded)
        },
        "isError": false
    }))
}

static STUB_FILL_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// `stub_fill` — Preenche stubs de código demarcados em arquivos locais.
pub async fn run_souls_stub_fill(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let _guard = STUB_FILL_MUTEX.lock().await;
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let path_str = args
        .get("file_path")
        .or_else(|| args.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'file_path' ou 'path' ausente".to_string(),
            data: None,
        })?;
    let stub_marker = args.get("stub_marker").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'stub_marker' ausente".to_string(),
        data: None,
    })?;
    let code_payload = args.get("code_payload").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'code_payload' ausente".to_string(),
        data: None,
    })?;

    let abs_path = validate_and_canonicalize_path(path_str)?;
    let content = tokio::fs::read_to_string(&abs_path).await.map_err(|e| RpcError {
        code: -32012,
        message: format!("Falha ao ler arquivo: {e}"),
        data: None,
    })?;

    if !content.contains(stub_marker) {
        return Err(RpcError {
            code: -32001,
            message: format!("Marcador '{stub_marker}' não encontrado em {path_str}"),
            data: None,
        });
    }

    let updated = content.replace(stub_marker, code_payload);
    tokio::fs::write(&abs_path, updated).await.map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao gravar alterações: {e}"),
        data: None,
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Stub '{stub_marker}' preenchido com sucesso em '{path_str}'.")
        }],
        "structuredContent": { "path": path_str, "status": "filled" },
        "isError": false
    }))
}
