use serde_json::{json, Value};
use souls_mc_lib::cognition::thinking;
use souls_mc_lib::cognition::thinking::types::{ThoughtData, ThinkingResponse};
use souls_mc_lib::cognition::thinking::ThinkingEngine;
use crate::{
    extract_arguments, socratic_handle, thinking_sessions_registry, workspace_root, RpcError,
};

pub async fn run_thinking(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let session_id = args
        .get("sessionId")
        .and_then(Value::as_str)
        .unwrap_or("default")
        .to_string();
    let mut thought_obj = serde_json::Map::new();
    if let Some(v) = args.get("thought") {
        thought_obj.insert("thought".to_string(), v.clone());
    } else {
        thought_obj.insert("thought".to_string(), Value::String(String::new()));
    }
    let thought_number = args
        .get("thought_number")
        .or_else(|| args.get("thoughtNumber"))
        .cloned()
        .unwrap_or(Value::Number(1.into()));
    thought_obj.insert("thought_number".to_string(), thought_number);

    let total_thoughts = args
        .get("total_thoughts")
        .or_else(|| args.get("totalThoughts"))
        .cloned()
        .unwrap_or(Value::Number(1.into()));
    thought_obj.insert("total_thoughts".to_string(), total_thoughts);

    let next_thought_needed = args
        .get("next_thought_needed")
        .or_else(|| args.get("nextThoughtNeeded"))
        .cloned()
        .unwrap_or(Value::Bool(false));
    thought_obj.insert("next_thought_needed".to_string(), next_thought_needed);

    if let Some(v) = args.get("is_revision").or_else(|| args.get("isRevision")) {
        thought_obj.insert("is_revision".to_string(), v.clone());
    }
    if let Some(v) = args.get("revises_thought").or_else(|| args.get("revisesThought")) {
        thought_obj.insert("revises_thought".to_string(), v.clone());
    }
    if let Some(v) = args.get("branch_from_thought").or_else(|| args.get("branchFromThought")) {
        thought_obj.insert("branch_from_thought".to_string(), v.clone());
    }
    if let Some(v) = args.get("branch_id").or_else(|| args.get("branchId")) {
        thought_obj.insert("branch_id".to_string(), v.clone());
    }
    if let Some(v) = args.get("needs_more_thoughts").or_else(|| args.get("needsMoreThoughts")) {
        thought_obj.insert("needs_more_thoughts".to_string(), v.clone());
    }
    if let Some(v) = args.get("hitl_authorized").or_else(|| args.get("hitlAuthorized")) {
        thought_obj.insert("hitl_authorized".to_string(), v.clone());
    }
    let thought_value = Value::Object(thought_obj);
    let thought: ThoughtData = serde_json::from_value(thought_value).map_err(|e| RpcError {
        code: -32602,
        message: format!("Payload de thinking inválido: {e}"),
        data: None,
    })?;

    let registry = thinking_sessions_registry();
    let mut map = registry.lock().map_err(|e| RpcError {
        code: -32000,
        message: format!("Mutex THINKING_SESSIONS envenenado: {e}"),
        data: None,
    })?;
    let engine_lock = map
        .entry(session_id.clone())
        .or_insert_with(|| std::sync::Mutex::new(ThinkingEngine::new()));
    let engine = engine_lock.get_mut().map_err(|e| RpcError {
        code: -32000,
        message: format!("Mutex ThinkingEngine envenenado: {e}"),
        data: None,
    })?;
    let response: ThinkingResponse = engine.push_thought(thought.clone()).map_err(|e| RpcError {
        code: -32000,
        message: e.to_string(),
        data: None,
    })?;

    if let Some(handle) = socratic_handle() {
        let socratic = souls_mc_lib::cognition::thinking::persistence::SocraticThought {
            thought_id: souls_mc_lib::cognition::memory_graph::uuid::generate_uuid_v7(),
            session_id: session_id.clone(),
            branch_id: thought.branch_id.clone().unwrap_or_else(|| "main".to_string()),
            parent_thought_id: thought
                .revises_thought
                .map(|n| n.to_string())
                .or_else(|| thought.branch_from_thought.map(|n| n.to_string())),
            thought_type: match response.mode {
                souls_mc_lib::cognition::thinking::types::ThinkingMode::Regular => {
                    souls_mc_lib::cognition::thinking::persistence::ThoughtType::Regular
                }
                souls_mc_lib::cognition::thinking::types::ThinkingMode::Revision => {
                    souls_mc_lib::cognition::thinking::persistence::ThoughtType::Revision
                }
                souls_mc_lib::cognition::thinking::types::ThinkingMode::Branching => {
                    souls_mc_lib::cognition::thinking::persistence::ThoughtType::Branching
                }
            },
            content: thought.thought.clone(),
            step_number: thought.thought_number,
            duration_ms: 0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or_default(),
        };
        let _ = handle.try_send(souls_mc_lib::cognition::thinking::socratic_bridge::SocraticOp::UpsertThoughtFire {
            thought: socratic,
        });
    }

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&response).unwrap_or_default()
        }]
    }))
}

pub async fn run_souls_export_session(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let session_id = args
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "export_session requer arguments.session_id (string)".to_string(),
            data: None,
        })?;
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("json");

    thinking::handlers::handle_export_session(
        session_id,
        Some(format),
        Some(&workspace_root()),
    )
    .map_err(|e| RpcError {
        code: -32000,
        message: e.to_string(),
        data: None,
    })
}

pub async fn run_souls_analyze_session(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let session_id = args
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "analyze_session requer arguments.session_id (string)".to_string(),
            data: None,
        })?;

    thinking::handlers::handle_analyze_session(
        session_id,
        Some(&workspace_root()),
    )
    .map_err(|e| RpcError {
        code: -32000,
        message: e.to_string(),
        data: None,
    })
}

pub async fn run_souls_merge_sessions(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let source_session_id = args
        .get("source_session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "merge_sessions requer arguments.source_session_id (string)".to_string(),
            data: None,
        })?;
    let target_session_id = args
        .get("target_session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "merge_sessions requer arguments.target_session_id (string)".to_string(),
            data: None,
        })?;

    thinking::handlers::handle_merge_sessions(
        source_session_id,
        target_session_id,
        None,
        socratic_handle(),
    )
    .map_err(|e| RpcError {
        code: -32000,
        message: e.to_string(),
        data: None,
    })
}

