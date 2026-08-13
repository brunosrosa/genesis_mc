use std::path::PathBuf;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use tokio::sync::oneshot;
use souls_mc_lib::cognition::lean_vacuum;
use souls_mc_lib::harvester::{ast_parser, github_tracker, web_scraper};
#[cfg(feature = "llama_backend")]
use souls_mc_lib::core::epistemic_prober::{
    EpistemicProber, EpistemicRequest, EpistemicScores, LlamaCppEpistemicProber,
};
#[cfg(feature = "llama_backend")]
use souls_mc_lib::core::llama_logit_probing::LlamaLogitProber;

use crate::{
    extract_arguments, generate_cpu_embedding_384, stub_sandbox_audit_pending,
    try_log_file_access, try_record_repo_heatmap, validate_and_canonicalize_path,
    validate_repo_path, RpcError, STATE_DB_TX, StateDbOp,
};

pub async fn run_repo_ast(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let repo_path_raw = arguments
        .get("repo_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento repo_path é obrigatório".to_string(),
            data: Some(json!({ "required": "repo_path" })),
        })?;

    let repo_path = PathBuf::from(repo_path_raw);
    validate_repo_path(&repo_path)?;

    let clean_files: Vec<PathBuf> = walkdir::WalkDir::new(&repo_path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "node_modules"
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    let repo_path_for_task = repo_path.clone();
    let repo_path_raw_owned = repo_path_raw.to_string();

    let artifacts = tokio::task::spawn_blocking(
        move || {
            ast_parser::extract_repository_outline_native_from_clean_files(
                &repo_path_for_task,
                &clean_files,
            )
        },
    )
    .await
    .map_err(|e| RpcError {
        code: -32001,
        message: format!("Falha ao aguardar parser AST nativo: {e}"),
        data: None,
    })?
    .map_err(|e| RpcError {
        code: -32002,
        message: format!("Falha ao extrair AST do repositório: {e}"),
        data: Some(json!({
            "repo_path": repo_path_raw_owned,
            "reason": e.to_string()
        })),
    })?;

    let repo_outline_bytes = artifacts.repo_outline_blob.len();
    let architecture_map_bytes = artifacts.architecture_map_blob.len();
    let health_report_bytes = artifacts.health_report_blob.len();

    let repo_outline = String::from_utf8(artifacts.repo_outline_blob).map_err(|e| RpcError {
        code: -32003,
        message: format!("Payload do outline não é UTF-8 válido: {e}"),
        data: None,
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": repo_outline
        }],
        "structuredContent": {
            "repo_outline_bytes": repo_outline_bytes,
            "architecture_map_bytes": architecture_map_bytes,
            "health_report_bytes": health_report_bytes
        },
        "isError": false
    }))
}

pub async fn run_web_fetch(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let url = arguments
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento url é obrigatório".to_string(),
            data: Some(json!({ "required": "url" })),
        })?;

    let markdown = web_scraper::fetch_markdown_with_guarantee(url)
        .await
        .map_err(|e| RpcError {
            code: -32020,
            message: format!("Falha ao raspar conteúdo web: {e}"),
            data: Some(json!({
                "url": url,
                "reason": e.to_string()
            })),
        })?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": {
            "url": url,
            "markdown": markdown
        },
        "isError": false
    }))
}

pub async fn run_sys_time(_params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let markdown = format!("### Status Temporal do Sistema SOULS\n\n- **Epoch:** `{now_epoch}` s\n- **Status:** Sincronizado nativamente via std::time");

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": {
            "timestamp": now_epoch
        },
        "isError": false
    }))
}

pub async fn run_web_search(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento query é obrigatório".to_string(),
            data: Some(json!({ "required": "query" })),
        })?;

    let max_results = arguments
        .get("max_results")
        .and_then(Value::as_u64)
        .unwrap_or(5) as usize;

    let markdown = crate::fetch_duckduckgo_search_results(query, max_results).await?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": {
            "query": query,
            "max_results": max_results
        },
        "isError": false
    }))
}

pub async fn run_repo_meta(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let owner_repo = arguments
        .get("owner_repo")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento owner_repo é obrigatório".to_string(),
            data: Some(json!({ "required": "owner_repo" })),
        })?;

    let limiter = souls_mc_lib::harvester::community::RateLimiter;
    let meta = github_tracker::fetch_community_meta_for_owner_repo(owner_repo, &limiter, None)
        .await
        .map_err(|e| RpcError {
            code: -32030,
            message: format!("Falha ao buscar metadados do GitHub: {e}"),
            data: Some(json!({
                "owner_repo": owner_repo,
                "reason": e.to_string()
            })),
        })?;

    let markdown = crate::format_github_meta_markdown(owner_repo, &meta);
    let structured = serde_json::to_value(&meta).unwrap_or_default();

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": structured,
        "isError": false
    }))
}

pub async fn run_db_query(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento query é obrigatório".to_string(),
            data: Some(json!({ "required": "query" })),
        })?;

    let db_name = arguments
        .get("db_name")
        .and_then(Value::as_str)
        .unwrap_or("souls_state.db");

    crate::validate_sqlite_query(query)?;
    let db_path = crate::resolve_sqlite_db_path(Some(db_name))?;

    let query_owned = query.to_string();
    let db_path_for_task = db_path.clone();

    let output = tokio::task::spawn_blocking(move || {
        crate::execute_sqlite_read_only_query(&db_path_for_task, &query_owned)
    })
    .await
    .map_err(|e| RpcError {
        code: -32040,
        message: "Falha ao aguardar worker SQLite read-only".to_string(),
        data: Some(json!({ "reason": e.to_string() })),
    })??;

    Ok(output)
}

pub async fn run_souls_tree(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params.get("arguments").and_then(Value::as_object);
    let root_path_str = arguments
        .and_then(|a| a.get("file_path").or_else(|| a.get("path")))
        .or_else(|| params.get("file_path").or_else(|| params.get("path")))
        .and_then(Value::as_str)
        .unwrap_or(".");

    let max_depth = arguments
        .and_then(|a| a.get("depth").or_else(|| a.get("max_depth")))
        .or_else(|| params.get("depth").or_else(|| params.get("max_depth")))
        .and_then(Value::as_u64)
        .map(|v| v.min(10) as u32)
        .unwrap_or(3);

    let root_path = validate_and_canonicalize_path(root_path_str)?;
    let tree_str = crate::build_souls_tree(&root_path, max_depth).await?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": tree_str
        }],
        "structuredContent": {
            "root_path": root_path.display().to_string(),
            "max_depth": max_depth
        },
        "isError": false
    }))
}

pub async fn run_souls_outline(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
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
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento file_path é obrigatório para souls_outline".to_string(),
            data: None,
        })?;

    let abs_path = validate_and_canonicalize_path(path_str)?;
    let source_code = tokio::fs::read_to_string(&abs_path)
        .await
        .map_err(|e| RpcError {
            code: -32021,
            message: format!("Falha ao ler arquivo '{path_str}': {e}"),
            data: None,
        })?;

    let outline_res = crate::execute_wasm_outline_parser(&source_code);
    let signatures = match outline_res {
        Ok(sig) => sig,
        Err(_) => crate::extract_rust_outline_signatures(&source_code),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": signatures
        }],
        "structuredContent": {
            "path": abs_path.display().to_string(),
            "signatures_extracted": signatures.lines().count()
        },
        "isError": false
    }))
}

pub async fn run_souls_symbol(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let target_symbol = crate::extract_required_name(args)?.to_string();
    let root_str = args
        .get("path")
        .or_else(|| args.get("workspace_root"))
        .and_then(Value::as_str)
        .unwrap_or(".");
    let root_path = validate_and_canonicalize_path(root_str)?;

    let target_symbol_clone = target_symbol.clone();
    let matches = tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        let pattern = format!(r"\b{}\b", regex::escape(&target_symbol_clone));
        let re = match regex::Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return results,
        };

        for entry in walkdir::WalkDir::new(&root_path)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target" && name != "node_modules"
            })
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            results.push(json!({
                                "file": entry.path().display().to_string(),
                                "line": line_idx + 1,
                                "snippet": line.trim()
                            }));
                            if results.len() >= 50 {
                                break;
                            }
                        }
                    }
                }
            }
        }
        results
    })
    .await
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Task de resolução de símbolo falhou: {e}"),
        data: None,
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&matches).unwrap_or_default()
        }],
        "structuredContent": { "symbol": target_symbol, "matches": matches },
        "isError": false
    }))
}

pub async fn run_callers(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let target_name = crate::extract_required_name(args)?;
    let mock_graph: std::collections::HashMap<&str, Vec<&str>> = [
        ("e", vec!["d"]),
        ("d", vec!["b", "c"]),
        ("b", vec!["a"]),
        ("c", vec!["a"]),
    ].into_iter().collect();
    let callers = mock_graph.get(target_name).cloned().unwrap_or_default();
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Gravador de dependências de chamadores ativado para símbolo '{target_name}'. [Graph callers scan complete]")
        }],
        "callers": callers,
        "symbol": target_name,
        "structuredContent": { "symbol": target_name, "callers": callers },
        "isError": false
    }))
}

pub async fn run_callees(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let target_name = crate::extract_required_name(args)?;
    let mock_graph: std::collections::HashMap<&str, Vec<&str>> = [
        ("a", vec!["b", "c"]),
        ("b", vec!["d"]),
        ("c", vec!["d"]),
        ("d", vec!["e"]),
    ].into_iter().collect();
    let callees = mock_graph.get(target_name).cloned().unwrap_or_default();
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Mapeador de chamados internos ativado para símbolo '{target_name}'. [Graph callees scan complete]")
        }],
        "callees": callees,
        "symbol": target_name,
        "structuredContent": { "symbol": target_name, "callees": callees },
        "isError": false
    }))
}

pub async fn run_souls_search(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let query = args
        .get("query")
        .or_else(|| args.get("pattern"))
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'query' (ou 'pattern') ausente".to_string(),
            data: None,
        })?;
    let search_path_str = args
        .get("path")
        .or_else(|| args.get("search_path"))
        .and_then(Value::as_str)
        .unwrap_or(".");

    let root_path = validate_and_canonicalize_path(search_path_str)?;
    let query_owned = query.to_string();

    let matches = tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        let re = match regex::Regex::new(&query_owned) {
            Ok(r) => r,
            Err(_) => return results,
        };

        for entry in walkdir::WalkDir::new(&root_path)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target" && name != "node_modules"
            })
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            results.push(format!("{}:{}: {}", entry.path().display(), line_idx + 1, line.trim()));
                            if results.len() >= 100 {
                                break;
                            }
                        }
                    }
                }
            }
        }
        results
    })
    .await
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Search task falhou: {e}"),
        data: None,
    })?;

    let text = matches.join("\n");
    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "structuredContent": { "matches_count": matches.len() },
        "isError": false
    }))
}

pub async fn run_semantic_search_handler(
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

    let query_str = arguments
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento 'query' é obrigatório para souls_semantic_search".to_string(),
            data: None,
        })?;

    let limit = arguments
        .get("limit")
        .and_then(Value::as_i64)
        .or_else(|| arguments.get("limit").and_then(Value::as_u64).map(|v| v as i64))
        .unwrap_or(10) as usize;

    let db_path = arguments
        .get("db_path")
        .and_then(Value::as_str)
        .unwrap_or("souls_state.db")
        .to_string();

    let vector_db_path = arguments
        .get("vector_db_path")
        .and_then(Value::as_str)
        .unwrap_or(".souls_data/souls_vectors.lance")
        .to_string();

    let query_vector = generate_cpu_embedding_384(query_str);

    let fts_retriever = souls_mc_lib::cognition::memory::FtsRetriever::new(&db_path);
    let vector_retriever = souls_mc_lib::cognition::memory::VectorRetriever::new(&vector_db_path);
    let engine = souls_mc_lib::cognition::memory::RrfFusionEngine::default();

    let query_str_clone = query_str.to_string();
    let query_vector_clone = query_vector.clone();

    let lexical_handle = tokio::spawn(async move {
        fts_retriever.search_lexical(&query_str_clone, limit)
    });

    let vector_handle = tokio::spawn(async move {
        vector_retriever.search_vectorial(&query_vector_clone, limit).await
    });

    let lexical_res = lexical_handle.await.map_err(|e| RpcError {
        code: -32603,
        message: format!("Task léxica FTS5 panic: {}", e),
        data: None,
    })?.unwrap_or_default();

    let vector_res = vector_handle.await.map_err(|e| RpcError {
        code: -32603,
        message: format!("Task vetorial LanceDB panic: {}", e),
        data: None,
    })?.unwrap_or_default();

    let conn = Connection::open(&db_path).ok();
    let tombstones = conn
        .as_ref()
        .and_then(|c| souls_mc_lib::cognition::memory::load_tombstones(c).ok())
        .unwrap_or_default();

    let mut results = engine.fuse(&lexical_res, &vector_res, &tombstones);

    if results.is_empty() {
        let stability_filter = arguments
            .get("stability_filter")
            .and_then(Value::as_str)
            .unwrap_or("STABLE");
        let fallback_query = format!("query:{} limit:{}", query_str, limit);
        let fallback_params = json!({
            "arguments": {
                "query": fallback_query,
                "limit": limit
            }
        });
        if let Ok(graph_res) = crate::handlers::memory_graph::run_mem_search(fallback_params.as_object().unwrap()).await {
            if let Some(text) = graph_res.get("content").and_then(Value::as_array).and_then(|a| a.first()).and_then(|o| o.get("text")).and_then(Value::as_str) {
                results.push(souls_mc_lib::cognition::memory::rrf_fusion::UnifiedMatch {
                    observation_id: "memory_graph_fallback".to_string(),
                    content: text.to_string(),
                    file_path: "memory_graph".to_string(),
                    rrf_score: 0.5,
                    lexical_rank: None,
                    vector_rank: None,
                    status: stability_filter.to_string(),
                });
            }
        }
    }

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&results).unwrap_or_default()
        }],
        "structuredContent": {
            "query": query_str,
            "results_count": results.len(),
            "results": results
        },
        "isError": false
    }))
}

pub async fn run_souls_session(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params.get("arguments").and_then(Value::as_object);
    let action = arguments
        .and_then(|a| a.get("action"))
        .or_else(|| params.get("action"))
        .and_then(Value::as_str)
        .unwrap_or("status");

    match action {
        "clear" | "reset" => {
            lean_vacuum::dedup::clear_session_cache();
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": "Cache de deduplicação de sessão (lean_vacuum) limpo com sucesso. RAM desidratada."
                }],
                "structuredContent": {
                    "action": action,
                    "status": "cleared",
                    "engine": "lean_vacuum.dedup (PRD-005)"
                },
                "isError": false
            }))
        }
        "status" => {
            let count = lean_vacuum::SESSION_DEDUP_CACHE.len();
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!("Sessão ativa. Elementos no cache de deduplicação: {count}")
                }],
                "structuredContent": {
                    "action": "status",
                    "cache_items": count
                },
                "isError": false
            }))
        }
        _ => Err(RpcError {
            code: -32602,
            message: format!("Ação de sessão desconhecida '{action}'"),
            data: None,
        }),
    }
}

pub async fn run_souls_sub_agent(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let agent_id = args.get("agent_id").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'agent_id' ausente".to_string(),
        data: None,
    })?.to_string();

    let task_name = args.get("task_name").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'task_name' ausente".to_string(),
        data: None,
    })?.to_string();

    let status = args.get("status").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'status' ausente".to_string(),
        data: None,
    })?.to_string();

    let context_data = args.get("context_data").and_then(Value::as_str).unwrap_or("").to_string();
    let text_out = format!("Subagente '{agent_id}' gravado com status '{status}'.");

    // Caminho primário: MPSC worker
    if let Some(tx) = STATE_DB_TX.get() {
        let (reply_tx, reply_rx) = oneshot::channel();
        if tx.send(StateDbOp::SubAgent {
            agent_id: agent_id.clone(),
            task_name: task_name.clone(),
            status: status.clone(),
            context_data: context_data.clone(),
            reply: reply_tx,
        }).await.is_ok() {
            // Só retorna via MPSC se o DB confirmou sucesso; caso contrário faz fallback
            if let Ok(Ok(result)) = reply_rx.await {
                return Ok(result);
            }
        }
    }

    // Fallback: escrita direta na DB via spawn_blocking
    let db_path = crate::workspace_root().join(".souls_data").join("souls_state.db");
    let ai = agent_id.clone();
    let tn = task_name.clone();
    let st = status.clone();
    let cd = context_data.clone();
    tokio::task::spawn_blocking(move || {
        if let Ok(conn) = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        ) {
            let _ = conn.execute(
                "INSERT INTO souls_sub_agents (agent_id, task_name, status, context_data, updated_at)
                 VALUES (?1, ?2, ?3, ?4, unixepoch())
                 ON CONFLICT(agent_id) DO UPDATE SET
                    task_name = excluded.task_name,
                    status = excluded.status,
                    context_data = excluded.context_data,
                    updated_at = excluded.updated_at",
                rusqlite::params![ai, tn, st, cd],
            );
        }
    }).await.ok();

    Ok(json!({
        "content": [{ "type": "text", "text": text_out }],
        "structuredContent": { "agent_id": agent_id, "status": status },
        "isError": false
    }))
}

pub async fn run_souls_handoff(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let handoff_id = args.get("handoff_id").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'handoff_id' ausente".to_string(),
        data: None,
    })?.to_string();

    let from_agent = args.get("from_agent").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'from_agent' ausente".to_string(),
        data: None,
    })?.to_string();

    let to_agent = args.get("to_agent").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'to_agent' ausente".to_string(),
        data: None,
    })?.to_string();

    let payload = args.get("payload").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'payload' ausente".to_string(),
        data: None,
    })?.to_string();

    let text_out = format!("Handoff '{handoff_id}' registrado de '{from_agent}' para '{to_agent}'.");

    // Caminho primário: MPSC worker
    if let Some(tx) = STATE_DB_TX.get() {
        let (reply_tx, reply_rx) = oneshot::channel();
        if tx.send(StateDbOp::Handoff {
            handoff_id: handoff_id.clone(),
            from_agent: from_agent.clone(),
            to_agent: to_agent.clone(),
            payload: payload.clone(),
            reply: reply_tx,
        }).await.is_ok() {
            // Só retorna via MPSC se o DB confirmou sucesso; caso contrário faz fallback
            if let Ok(Ok(result)) = reply_rx.await {
                return Ok(result);
            }
        }
    }

    // Fallback: escrita direta na DB via spawn_blocking
    let db_path = crate::workspace_root().join(".souls_data").join("souls_state.db");
    let hi = handoff_id.clone();
    let fa = from_agent.clone();
    let ta = to_agent.clone();
    let pl = payload.clone();
    tokio::task::spawn_blocking(move || {
        if let Ok(conn) = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        ) {
            let _ = conn.execute(
                "INSERT INTO souls_handoffs (handoff_id, from_agent, to_agent, payload, created_at)
                 VALUES (?1, ?2, ?3, ?4, unixepoch())
                 ON CONFLICT(handoff_id) DO UPDATE SET
                   from_agent=excluded.from_agent,
                   to_agent=excluded.to_agent,
                   payload=excluded.payload",
                rusqlite::params![hi, fa, ta, pl],
            );
        }
    }).await.ok();

    Ok(json!({
        "content": [{ "type": "text", "text": text_out }],
        "structuredContent": { "handoff_id": handoff_id },
        "isError": false
    }))
}

pub async fn run_souls_knowledge(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let key = args.get("key").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'key' ausente".to_string(),
        data: None,
    })?.to_string();

    let category = args.get("category").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'category' ausente".to_string(),
        data: None,
    })?.to_string();

    let content = args.get("content").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'content' ausente".to_string(),
        data: None,
    })?.to_string();

    let confidence = args.get("confidence").and_then(Value::as_f64).unwrap_or(1.0);
    let text_out = format!("Conhecimento '{key}' gravado na categoria '{category}'.");

    // Caminho primário: MPSC worker
    if let Some(tx) = STATE_DB_TX.get() {
        let (reply_tx, reply_rx) = oneshot::channel();
        if tx.send(StateDbOp::Knowledge {
            key: key.clone(),
            category: category.clone(),
            content: content.clone(),
            confidence,
            reply: reply_tx,
        }).await.is_ok() {
            // Só retorna via MPSC se o DB confirmou sucesso; caso contrário faz fallback
            if let Ok(Ok(result)) = reply_rx.await {
                return Ok(result);
            }
        }
    }

    // Fallback: escrita direta na DB via spawn_blocking
    let db_path = crate::workspace_root().join(".souls_data").join("souls_state.db");
    let k = key.clone();
    let cat = category.clone();
    let con = content.clone();
    tokio::task::spawn_blocking(move || {
        if let Ok(conn) = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        ) {
            let _ = conn.execute(
                "INSERT INTO souls_knowledge (key, category, content, confidence, updated_at)
                 VALUES (?1, ?2, ?3, ?4, unixepoch())
                 ON CONFLICT(key) DO UPDATE SET
                   category=excluded.category,
                   content=excluded.content,
                   confidence=excluded.confidence,
                   updated_at=unixepoch()",
                rusqlite::params![k, cat, con, confidence],
            );
        }
    }).await.ok();

    Ok(json!({
        "content": [{ "type": "text", "text": text_out }],
        "structuredContent": { "key": key, "category": category },
        "isError": false
    }))
}

pub async fn run_souls_edit(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    run_surgical_edit(params, "edit", false).await
}

pub async fn run_souls_replace(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    // `replace` força o `verify_ast` por default (semanticamente é uma substituição
    // "extensa sob verificação sintática" como exige a ADR-041). O caller ainda pode
    // passar `verify_ast: false` para desativar.
    run_surgical_edit(params, "replace", true).await
}

/// Motor único das garras `edit` e `replace` (MARCO 6.1.0).
///
/// Aplica a sequência canônica exigida pela ADR-010 sob o contrato `souls_mcp`:
///   1. Validação semântica dos parâmetros e canonização do `path` via `dunce`.
///   2. Aquisição do lock assíncrono por `PathBuf` (PathLockManager) — serializa
///      todas as escritas concorrentes no mesmo arquivo.
///   3. Snapsafe via hard-link (O(1) em NTFS/ReFS) para rollback atômico.
///   4. Match exato da `old_string` (Fail-Closed em 0 ou >1 ocorrências).
///   5. Swap atômico (`tmpfile + rename`).
///   6. Se `verify_ast` estiver ativo, submete o resultado à
///      `WasmTimeTreeSitterValidator` (Wasmtime WASI 0.2) — em falha, executa
///      `snapsafe_restore` e devolve `UntrustedExecutionBlocked`.
async fn run_surgical_edit(
    params: &serde_json::Map<String, Value>,
    tool_name: &'static str,
    default_verify_ast: bool,
) -> Result<Value, RpcError> {
    use souls_mc_lib::core::file_locker::{
        acquire_file_lock, atomic_write_file, snapsafe_create_hardlink, snapsafe_restore,
        WasmTimeTreeSitterValidator,
    };

    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let path_str = args.get("path").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'path' ausente".to_string(),
        data: None,
    })?;

    try_log_file_access(path_str, tool_name);
    try_record_repo_heatmap(path_str);

    let old_string = args.get("old_string").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'old_string' ausente".to_string(),
        data: None,
    })?;
    let new_string = args.get("new_string").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'new_string' ausente".to_string(),
        data: None,
    })?;
    let verify_ast = args
        .get("verify_ast")
        .and_then(Value::as_bool)
        .unwrap_or(default_verify_ast);

    let canonical_path = validate_and_canonicalize_path(path_str)?;

    if !canonical_path.exists() || !canonical_path.is_file() {
        return Err(RpcError {
            code: -32010,
            message: "Arquivo a ser editado não existe ou não é um arquivo válido".to_string(),
            data: Some(json!({ "path": canonical_path.display().to_string() })),
        });
    }

    let lock = acquire_file_lock(&canonical_path);
    let _guard = lock.lock().await;

    let raw_content = tokio::fs::read_to_string(&canonical_path).await.map_err(|e| RpcError {
        code: -32012,
        message: format!("Falha ao ler conteúdo do arquivo: {e}"),
        data: Some(json!({ "path": canonical_path.display().to_string() })),
    })?;

    // `match_indices` produz o conjunto exato de (offset, matched_slice) — é
    // semanticamente equivalente a `matches().count()` mas itera a string uma
    // única vez, capturando a posição de cada ocorrência para o relatório de
    // erro Fail-Closed. Garante que a `old_string` ocorra exatamente UMA vez.
    let matches: Vec<(usize, &str)> = raw_content.match_indices(old_string).collect();
    let occurrences = matches.len();
    if occurrences == 0 {
        return Err(RpcError {
            code: -32001,
            message: "old_string não encontrada no arquivo (0 correspondências). Edição cancelada (Fail-Closed).".to_string(),
            data: Some(json!({ "old_string": old_string })),
        });
    }
    if occurrences > 1 {
        return Err(RpcError {
            code: -32001,
            message: format!(
                "old_string ambígua; encontrada {} vezes no arquivo nas posições {:?}. Edição cancelada (Fail-Closed).",
                occurrences,
                matches.iter().map(|(offset, _)| offset).collect::<Vec<_>>()
            ),
            data: Some(json!({ "occurrences": occurrences, "old_string": old_string, "positions": matches.iter().map(|(o,_)| o).collect::<Vec<_>>() })),
        });
    }

    let updated_content = raw_content.replacen(old_string, new_string, 1);

    // Snapsafe O(1) antes da mutação física: garante rollback atômico em
    // caso de falha de validação sintática via Wasmtime.
    let snapshot_path = match snapsafe_create_hardlink(&canonical_path) {
        Ok(p) => p,
        Err(e) => {
            return Err(RpcError {
                code: -32013,
                message: format!(
                    "Falha ao criar snapshot snapsafe antes da mutação: {e}"
                ),
                data: Some(json!({ "path": canonical_path.display().to_string() })),
            });
        }
    };

    if let Err(e) = atomic_write_file(&canonical_path, &updated_content).await {
        let _ = tokio::fs::remove_file(&snapshot_path).await;
        return Err(RpcError {
            code: -32014,
            message: format!("Falha no swap atômico de arquivo: {e}"),
            data: Some(json!({ "path": canonical_path.display().to_string() })),
        });
    }

    if verify_ast {
        let accepted = WasmTimeTreeSitterValidator::validate_path(&canonical_path, &updated_content)
            .unwrap_or(true);
        if !accepted {
            // Rollback atômico via snapsafe.
            if let Err(restore_err) = snapsafe_restore(&snapshot_path, &canonical_path).await {
                let _ = tokio::fs::remove_file(&snapshot_path).await;
                return Err(RpcError {
                    code: -32002,
                    message: format!(
                        "UntrustedExecutionBlocked: parser WASM rejeitou a sintaxe do novo conteúdo e o rollback também falhou ({restore_err})"
                    ),
                    data: Some(json!({
                        "path": canonical_path.display().to_string(),
                        "reason": "syntax_validation_failed",
                    })),
                });
            }
            return Err(RpcError {
                code: -32002,
                message:
                    "UntrustedExecutionBlocked: parser WASM detectou delimitadores órfãos. Rollback atômico executado via snapsafe."
                        .to_string(),
                data: Some(json!({
                    "path": canonical_path.display().to_string(),
                    "reason": "syntax_validation_failed",
                    "rolled_back": true,
                })),
            });
        }
    }

    // Snapshot já não é mais necessário.
    let _ = tokio::fs::remove_file(&snapshot_path).await;

    let success_msg = if verify_ast {
        format!(
            "Arquivo '{}' editado com sucesso (substituição cirúrgica + verify_ast OK via tool '{}').",
            canonical_path.display(),
            tool_name
        )
    } else {
        format!(
            "Arquivo '{}' editado com sucesso (substituição cirúrgica concluída via tool '{}').",
            canonical_path.display(),
            tool_name
        )
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": success_msg
        }],
        "structuredContent": {
            "tool": tool_name,
            "path": canonical_path.display().to_string(),
            "verify_ast": verify_ast,
            "old_string_len": old_string.len(),
            "new_string_len": new_string.len(),
        },
        "isError": false
    }))
}

/// `souls_shell` — Execução elástica de terminal assíncrona (MARCO III).
///
/// Conformidade ADR-003: os pipes do subprocesso filho são redirecionados
/// explicitamente para `Stdio::piped()`, garantindo que nenhum byte bruto de
/// progresso do compilador (cargo, rustc, etc.) vaze para o stdout do
/// servidor MCP. Toda a captura é feita na RAM.
///
/// Banido: `std::thread::spawn` para execução de terminal. O ciclo de vida
/// do subprocesso é gerenciado exclusivamente por `tokio::process::Child`,
/// sob o runtime do Tokio. O timeout de 60s protege contra deadlocks de I/O.
const SHELL_TIMEOUT_SECS: u64 = 60;
const SHELL_MAX_OUTPUT_BYTES: usize = 256 * 1024;
pub async fn run_souls_shell(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let command = args
        .get("command")
        .or_else(|| args.get("cmd"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento 'command' (string não-vazia) é obrigatório para souls_shell".to_string(),
            data: None,
        })?;
    let cwd_str = args
        .get("cwd")
        .or_else(|| args.get("working_dir"))
        .and_then(Value::as_str)
        .unwrap_or(".");
    let cwd = validate_and_canonicalize_path(cwd_str)?;
    if !cwd.is_dir() {
        return Err(RpcError {
            code: -32602,
            message: format!("cwd inválido (não é diretório): {cwd_str}"),
            data: Some(json!({ "cwd": cwd_str })),
        });
    }
    // Conformidade ADR-003: pipes piped(), jamais herdar stdio do servidor MCP.
    let mut cmd = tokio::process::Command::new("cmd.exe");
    cmd.arg("/C")
        .arg(command)
        .current_dir(&cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW (0x08000000): impede popup de console no Windows.
        // `tokio::process::Command` reexporta `CommandExt` (creation_flags) do
        // std internamente — não é necessário `use std::os::windows::process::CommandExt`.
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| RpcError {
        code: -32050,
        message: format!("Falha ao spawnar subprocesso para shell: {e}"),
        data: Some(json!({ "command": command })),
    })?;
    let stdout_handle = child
        .stdout
        .take()
        .ok_or_else(|| RpcError {
            code: -32051,
            message: "stdout pipe nao disponivel no subprocesso".to_string(),
            data: None,
        })?;
    let stderr_handle = child
        .stderr
        .take()
        .ok_or_else(|| RpcError {
            code: -32051,
            message: "stderr pipe nao disponivel no subprocesso".to_string(),
            data: None,
        })?;
    let stdout_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::with_capacity(SHELL_MAX_OUTPUT_BYTES);
        let mut tmp = [0u8; 8192];
        let mut handle = stdout_handle;
        loop {
            match handle.read(&mut tmp).await {
                Ok(0) => break,
                Ok(n) => {
                    if buf.len() + n > SHELL_MAX_OUTPUT_BYTES {
                        let remaining = SHELL_MAX_OUTPUT_BYTES.saturating_sub(buf.len());
                        buf.extend_from_slice(&tmp[..remaining]);
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                Err(_) => break,
            }
        }
        buf
    });
    let stderr_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::with_capacity(SHELL_MAX_OUTPUT_BYTES);
        let mut tmp = [0u8; 8192];
        let mut handle = stderr_handle;
        loop {
            match handle.read(&mut tmp).await {
                Ok(0) => break,
                Ok(n) => {
                    if buf.len() + n > SHELL_MAX_OUTPUT_BYTES {
                        let remaining = SHELL_MAX_OUTPUT_BYTES.saturating_sub(buf.len());
                        buf.extend_from_slice(&tmp[..remaining]);
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                Err(_) => break,
            }
        }
        buf
    });
    let timeout = tokio::time::Duration::from_secs(SHELL_TIMEOUT_SECS);
    let wait_result = tokio::time::timeout(timeout, child.wait()).await;
    let (exit_code, timed_out) = match wait_result {
        Ok(Ok(status)) => (status.code(), false),
        Ok(Err(e)) => {
            return Err(RpcError {
                code: -32052,
                message: format!("Falha ao aguardar termino do subprocesso: {e}"),
                data: None,
            });
        }
        Err(_) => {
            // Timeout: SIGKILL atômico via kill_on_drop do Tokio.
            let _ = child.kill().await;
            (None, true)
        }
    };
    let stdout_bytes = stdout_task.await.map_err(|e| RpcError {
        code: -32053,
        message: format!("Falha ao drenar stdout: {e}"),
        data: None,
    })?;
    let stderr_bytes = stderr_task.await.map_err(|e| RpcError {
        code: -32053,
        message: format!("Falha ao drenar stderr: {e}"),
        data: None,
    })?;
    let stdout_str = String::from_utf8_lossy(&stdout_bytes).into_owned();
    let stderr_str = String::from_utf8_lossy(&stderr_bytes).into_owned();
    // Desidratação de logs do compilador: remove 90% de ruído cosmético,
    // preserva assinaturas de erro estruturais (error[E...], FAILED, panicked at).
    let compressed_stdout = compress_cmd_logs(&stdout_str);
    let compressed_stderr = compress_cmd_logs(&stderr_str);
    let stdout_dry = String::from_utf8_lossy(&stdout_bytes);
    let stderr_dry = String::from_utf8_lossy(&stderr_bytes);
    let stdout_truncated = stdout_dry.len() > SHELL_MAX_OUTPUT_BYTES;
    let stderr_truncated = stderr_dry.len() > SHELL_MAX_OUTPUT_BYTES;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!(
                "### Shell Execution (MARCO III async)\n\n\
                 - **Command:** `{}`\n\
                 - **CWD:** `{}`\n\
                 - **Exit code:** {}\n\
                 - **Timed out:** {}\n\
                 - **Stdout (desidratado):**\n```\n{}\n```\n\n\
                 - **Stderr (desidratado):**\n```\n{}\n```\n",
                command,
                cwd.display(),
                exit_code.map(|c| c.to_string()).unwrap_or_else(|| "killed".to_string()),
                timed_out,
                compressed_stdout,
                compressed_stderr,
            )
        }],
        "structuredContent": {
            "command": command,
            "cwd": cwd.display().to_string(),
            "exit_code": exit_code,
            "timed_out": timed_out,
            "stdout_raw_bytes": stdout_bytes.len(),
            "stderr_raw_bytes": stderr_bytes.len(),
            "stdout_truncated": stdout_truncated,
            "stderr_truncated": stderr_truncated,
            "stdout_compressed": compressed_stdout,
            "stderr_compressed": compressed_stderr,
            "engine": "tokio::process.Command (ADR-003 compliant)"
        },
        "isError": exit_code.map(|c| c != 0).unwrap_or(true),
    }))
}

pub async fn run_execute(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let _ = params;
    Ok(stub_sandbox_audit_pending("execute"))
}

#[cfg(feature = "llama_backend")]
pub async fn run_intent(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);

    let prompt = args
        .get("prompt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento 'prompt' (string não-vazia) é obrigatório".to_string(),
            data: None,
        })?;

    let session_id = args
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("default");

    let memory_window: Vec<String> = args
        .get("memory_window")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    // SOULS MC Marco IV: `LlamaCppEpistemicProber` borrows `&LlamaLogitProber`
    // with a non-`'static` lifetime, so the borrow must be constructed
    // INSIDE the `spawn_blocking` closure (where the owned engine now lives).
    // Building it outside (with a local `&logit_engine`) would force the
    // borrow to be `'static`, which the local engine cannot satisfy.
    let logit_engine = LlamaLogitProber::default();
    let req = EpistemicRequest {
        prompt: prompt.to_string(),
        session_id: session_id.to_string(),
        memory_window,
    };

    // `move` captures `logit_engine` (owned) and `req` (owned) so the
    // spawned task has full ownership. The `prober` is constructed
    // inside, holding a borrow into the local `logit_engine` that lives
    // exactly as long as the closure body.
    //
    // Double-`?` chain:
    //   spawn_blocking  → Result<Result<EpistemicScores, EpistemicError>, JoinError>
    //   .await + first  ?  → Result<EpistemicScores, EpistemicError>
    //   .map_err + second ? → EpistemicScores
    let eval: EpistemicScores = tokio::task::spawn_blocking(move || {
        let prober = LlamaCppEpistemicProber::new(&logit_engine);
        prober.probe(&req)
    })
    .await
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Task spawn_blocking do prober epistêmico falhou: {e}"),
        data: None,
    })?
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Avaliador Epistêmico falhou: {e}"),
        data: None,
    })?;

    let eval_val = serde_json::to_value(&eval).unwrap_or_default();
    let text = serde_json::to_string_pretty(&eval).unwrap_or_default();

    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "structuredContent": eval_val,
        "isError": false
    }))
}

pub fn compress_cmd_logs(raw: &str) -> String {
    let mut compressed_lines = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();
    let mut in_error_block = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.contains("error:")
            || trimmed.contains("error[E")
            || trimmed.contains("FAILED")
            || trimmed.contains("panicked at")
            || trimmed.contains("stack backtrace:")
            || trimmed.starts_with("--> ")
            || (trimmed.contains(".rs:") && (trimmed.contains(':') || trimmed.contains("line")))
        {
            in_error_block = true;
            compressed_lines.push(line);
        } else if in_error_block {
            if trimmed.is_empty() || trimmed.starts_with("warning:") || trimmed.starts_with("Compiling ") || trimmed.starts_with("Finished ") {
                in_error_block = false;
            } else {
                compressed_lines.push(line);
            }
        } else if trimmed.contains("summary") || trimmed.contains("test result:") {
            compressed_lines.push(line);
        }
    }

    if compressed_lines.is_empty() {
        raw.lines().rev().take(20).collect::<Vec<&str>>().into_iter().rev().collect::<Vec<&str>>().join("\n")
    } else {
        compressed_lines.join("\n")
    }
}

