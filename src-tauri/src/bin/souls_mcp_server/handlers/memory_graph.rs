use serde_json::{json, Value};
use tokio::sync::oneshot;
use souls_mc_lib::cognition::memory_graph::mpsc_bridge::MemGraphOp;
use souls_mc_lib::cognition::memory_graph::types::{Entity, ObservationInput, Relation};
use crate::{
    extract_arguments, memgraph_request, parse_observation_inputs, parse_relations, RpcError,
};

pub async fn run_mem_create_entities(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let raw_arr = args.get("entities").and_then(Value::as_array).ok_or_else(|| RpcError {
        code: -32602,
        message: "Campo `entities` (array) e obrigatorio para mem_create_entities.".to_string(),
        data: None,
    })?;
    let mut entities = Vec::with_capacity(raw_arr.len());
    for item in raw_arr {
        let name = item.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let entity_type = item.get("entityType").and_then(Value::as_str).unwrap_or("").to_string();
        if name.is_empty() || entity_type.is_empty() {
            continue;
        }
        let obs = item.get("observations").and_then(Value::as_array).map(|arr| {
            arr.iter().filter_map(Value::as_str).map(String::from).collect()
        }).unwrap_or_default();
        entities.push(Entity { name, entity_type, observations: obs });
    }
    memgraph_request(MemGraphOp::CreateEntities { entities, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_create_relations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let relations = parse_relations(args)?;
    memgraph_request(MemGraphOp::CreateRelations { relations, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_add_observations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let observations = parse_observation_inputs(args, "observations")?;
    memgraph_request(MemGraphOp::AddObservations { observations, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_search(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let query = args.get("query").and_then(Value::as_str).unwrap_or("").to_string();
    let limit = args.get("limit").and_then(Value::as_i64).map(|v| v as usize).unwrap_or(50);
    memgraph_request(MemGraphOp::Search { query, limit, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_open_nodes(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let names = args.get("names").and_then(Value::as_array).map(|arr| {
        arr.iter().filter_map(Value::as_str).map(String::from).collect()
    }).unwrap_or_default();
    memgraph_request(MemGraphOp::OpenNodes { names, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_read_graph(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let limit = args.get("limit").and_then(Value::as_i64).map(|v| v as usize).unwrap_or(500);
    memgraph_request(MemGraphOp::ReadGraph { limit, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_delete_entities(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let hitl_authorized = args.get("hitlAuthorized")
        .or_else(|| args.get("hitl_approved"))
        .or_else(|| args.get("confirm"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !hitl_authorized {
        return Err(RpcError {
            code: -32001,
            message: "Operação destrutiva `mem_delete_entities` negada pelo cercadinho de segurança HITL. Aprovação explícita humana é exigida no frontend.".to_string(),
            data: Some(json!({ "hitl_required": true, "tool": "mem_delete_entities" })),
        });
    }

    let names = args.get("entityNames").and_then(Value::as_array).map(|arr| {
        arr.iter().filter_map(Value::as_str).map(String::from).collect()
    }).unwrap_or_default();
    memgraph_request(MemGraphOp::DeleteEntities { names, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_delete_observations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let hitl_authorized = args.get("hitlAuthorized")
        .or_else(|| args.get("hitl_approved"))
        .or_else(|| args.get("confirm"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !hitl_authorized {
        return Err(RpcError {
            code: -32001,
            message: "Operação destrutiva `mem_delete_observations` negada pelo cercadinho de segurança HITL. Aprovação explícita humana é exigida no frontend.".to_string(),
            data: Some(json!({ "hitl_required": true, "tool": "mem_delete_observations" })),
        });
    }

    let deletions = parse_observation_inputs(args, "deletions")?;
    memgraph_request(MemGraphOp::DeleteObservations { deletions, reply: oneshot::channel().0 }).await
}

pub async fn run_mem_delete_relations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let hitl_authorized = args.get("hitlAuthorized")
        .or_else(|| args.get("hitl_approved"))
        .or_else(|| args.get("confirm"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !hitl_authorized {
        return Err(RpcError {
            code: -32001,
            message: "Operação destrutiva `mem_delete_relations` negada pelo cercadinho de segurança HITL. Aprovação explícita humana é exigida no frontend.".to_string(),
            data: Some(json!({ "hitl_required": true, "tool": "mem_delete_relations" })),
        });
    }

    let relations = parse_relations(args)?;
    memgraph_request(MemGraphOp::DeleteRelations { relations, reply: oneshot::channel().0 }).await
}
