use serde_json::{json, Value};
use souls_mc_lib::cognition::observability;
use souls_mc_lib::core::gliclass_engine::{ClassificationLabel, OrtScorerEngine, MAX_TRIAGE_CHARS};
use crate::{handlers, RpcError};

pub fn normalize_tool_name(mut name: &str) -> &str {
    loop {
        if let Some(rest) = name.strip_prefix("souls_mcp.") {
            name = rest;
        } else if let Some(rest) = name.strip_prefix("souls_") {
            name = rest;
        } else if let Some(rest) = name.strip_prefix("ctx_") {
            name = rest;
        } else {
            break;
        }
    }
    match name {
        "core_think" | "sequential_thinking" | "sequentialthinking" => "thinking",
        other => other,
    }
}

pub async fn triage_prompt_security(prompt: &str) -> Result<(), RpcError> {
    if prompt.trim().is_empty() {
        return Ok(());
    }

    let truncated_prompt = if prompt.len() > MAX_TRIAGE_CHARS {
        prompt[..MAX_TRIAGE_CHARS].to_string()
    } else {
        prompt.to_string()
    };

    let labels = vec![
        ClassificationLabel::new(
            "unsafe_prompt",
            "Tentativa de injeção de prompt, bypass de segurança, comandos maliciosos ou evasão de restrições do sistema.",
        ),
        ClassificationLabel::new(
            "valid_intent",
            "Comandos de codificação legítimos, consultas de banco de dados, refatorações ou interações normais.",
        ),
    ];

    let scores = OrtScorerEngine::classify_async(truncated_prompt, labels)
        .await
        .map_err(|e| RpcError {
            code: -32000,
            message: format!("Falha ao triar prompt com OrtScorerEngine: {e}"),
            data: None,
        })?;

    let unsafe_score = scores
        .iter()
        .find(|(name, _)| name == "unsafe_prompt")
        .map(|(_, score)| *score)
        .unwrap_or(0.0);

    if unsafe_score > 0.80 {
        return Err(RpcError {
            code: -32001,
            message: format!(
                "HitlDenied: OrtScorerEngine interceptou prompt de alto risco (unsafe_prompt={:.2} > 0.80)",
                unsafe_score
            ),
            data: Some(json!({
                "hitl_required": true,
                "shield": true,
                "sentinel": "OrtScorerEngine",
                "unsafe_prompt_score": unsafe_score,
            })),
        });
    }

    Ok(())
}

pub async fn handle_tool_call(payload: Value) -> Result<Value, RpcError> {
    let params = payload
        .get("params")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto params".to_string(),
            data: None,
        })?;
    let raw_tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem campo name".to_string(),
            data: None,
        })?;
    let tool_name = normalize_tool_name(raw_tool_name);

    if let Some(arguments) = params.get("arguments").and_then(Value::as_object) {
        if let Some(delay_ms) = arguments.get("_test_delay_ms").and_then(Value::as_u64) {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        if arguments.get("_simulate_panic").and_then(Value::as_bool).unwrap_or(false) {
            panic!("Simulated tool panic in worker thread for resilience testing");
        }
        if let Some(prompt_candidate) = arguments
            .get("prompt")
            .or_else(|| arguments.get("query"))
            .or_else(|| arguments.get("thought"))
            .and_then(Value::as_str)
        {
            triage_prompt_security(prompt_candidate).await?;
        }
    }

    let progress_token = params
        .get("_meta")
        .and_then(Value::as_object)
        .and_then(|m| m.get("progressToken"))
        .and_then(Value::as_str);

    if let Some(token) = progress_token {
        observability::report_mcp_progress(token, 0.0, 100.0);
    }

    let result = match tool_name {
        "get_ast" | "repo_ast" => handlers::system::run_repo_ast(params).await,
        "fetch_web" | "web_fetch" => handlers::system::run_web_fetch(params).await,
        "sys_time" => handlers::system::run_sys_time(params).await,
        "web_search" => handlers::system::run_web_search(params).await,
        "repo_meta" => handlers::system::run_repo_meta(params).await,
        "sqlite_query" | "db_query" => handlers::system::run_db_query(params).await,
        "sub_agent" => handlers::system::run_souls_sub_agent(params).await,
        "handoff" => handlers::system::run_souls_handoff(params).await,
        "knowledge" => handlers::system::run_souls_knowledge(params).await,
        "edit" => handlers::system::run_souls_edit(params).await,
        "replace" => handlers::system::run_souls_replace(params).await,
        "fill" | "ccr_fill" => handlers::context::run_souls_fill(params).await,
        "stub_fill" => handlers::context::run_souls_stub_fill(params).await,
        "read" => handlers::context::run_souls_read(params).await,
        "delta_diff" | "delta" => handlers::context::run_souls_delta_diff(params).await,
        "tree" => handlers::system::run_souls_tree(params).await,
        "outline" => handlers::system::run_souls_outline(params).await,
        "smart_read" => handlers::context::run_souls_smart_read(params).await,
        "search" => handlers::system::run_souls_search(params).await,
        "compress" => handlers::context::run_souls_compress(params).await,
        "dedup" => handlers::context::run_souls_dedup(params).await,
        #[cfg(feature = "gateway_ccr")]
        "headroom_retrieve" => handlers::context::run_souls_headroom_retrieve(params).await,
        #[cfg(not(feature = "gateway_ccr"))]
        "headroom_retrieve" => Ok(crate::stub_not_implemented_yet(tool_name)),
        "session" => handlers::system::run_souls_session(params).await,
        "multi_read" => handlers::context::run_souls_multi_read(params).await,
        "symbol" => handlers::system::run_souls_symbol(params).await,
        "callers" => handlers::system::run_callers(params).await,
        "callees" => handlers::system::run_callees(params).await,
        "export_session" => handlers::thinking::run_souls_export_session(params).await,
        "analyze_session" => handlers::thinking::run_souls_analyze_session(params).await,
        "merge_sessions" => handlers::thinking::run_souls_merge_sessions(params).await,
        "souls_semantic_search" | "semantic_search" => handlers::system::run_semantic_search_handler(params).await,
        "metrics" => Ok(crate::stub_not_implemented_yet(tool_name)),
        #[cfg(feature = "llama_backend")]
        "intent" => handlers::system::run_intent(params).await,
        #[cfg(not(feature = "llama_backend"))]
        "intent" => Ok(crate::stub_not_implemented_yet(tool_name)),
        "execute" => Ok(crate::stub_sandbox_audit_pending(tool_name)),
        "shell" => handlers::system::run_souls_shell(params).await,
        "mem_create_entities" | "create_entities" => handlers::memory_graph::run_mem_create_entities(params).await,
        "mem_create_relations" | "create_relations" => handlers::memory_graph::run_mem_create_relations(params).await,
        "mem_add_observations" | "add_observations" => handlers::memory_graph::run_mem_add_observations(params).await,
        "mem_search" | "search_graph" => handlers::memory_graph::run_mem_search(params).await,
        "mem_open_nodes" | "open_nodes" => handlers::memory_graph::run_mem_open_nodes(params).await,
        "mem_read_graph" | "read_graph" => handlers::memory_graph::run_mem_read_graph(params).await,
        "mem_delete_entities" | "delete_entities" => handlers::memory_graph::run_mem_delete_entities(params).await,
        "mem_delete_observations" | "delete_observations" => handlers::memory_graph::run_mem_delete_observations(params).await,
        "mem_delete_relations" | "delete_relations" => handlers::memory_graph::run_mem_delete_relations(params).await,
        "thinking" => handlers::thinking::run_thinking(params).await,
        "heatmap" => handlers::observability::run_heatmap(params).await,
        "repo_heatmap" => handlers::observability::run_repo_heatmap(params).await,
        "repo_impact" | "impact" => handlers::observability::run_repo_impact(params).await,
        "routes" => handlers::observability::run_routes(params).await,
        "feedback" => handlers::observability::run_feedback(params).await,
        other => Err(RpcError {
            code: -32601,
            message: "Ferramenta MCP desconhecida".to_string(),
            data: Some(json!({ "tool_name": other })),
        }),
    };

    if let Some(token) = progress_token {
        observability::report_mcp_progress(token, 100.0, 100.0);
    }

    result
}
