//! Handlers canônicos de persistência socrática (Marco 3.9 Fase E.2).
//!
//! **Single Source of Truth** consumido por:
//! 1. `bin/souls_mcp_server.rs` (gateway MCP, transporte NDJSON).
//! 2. `main.rs` (comandos `#[tauri::command]` para Svelte 5).
//!
//! Padrão de fronteira: `Result<Value, String>` para que Svelte 5 capture
//! erros do SQLite sem travar o renderer. A serialização JSON é feita
//! uma única vez e reusada pelos dois lados.

use crate::cognition::memory_graph::errors::CognitiveError;
use crate::cognition::thinking;
use crate::cognition::thinking::persistence::{SocraticThought, ThoughtId, ThoughtType};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Erro unificado de fronteira para os handlers socráticos.
/// Wrap de `CognitiveError` e IO/JSON errors.
#[derive(Debug)]
pub struct SocraticHandlerError(pub String);

impl std::fmt::Display for SocraticHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SocraticHandlerError {}

impl From<CognitiveError> for SocraticHandlerError {
    fn from(e: CognitiveError) -> Self {
        Self(e.to_string())
    }
}

impl From<rusqlite::Error> for SocraticHandlerError {
    fn from(e: rusqlite::Error) -> Self {
        Self(format!("SQLite error: {e}"))
    }
}

impl From<std::io::Error> for SocraticHandlerError {
    fn from(e: std::io::Error) -> Self {
        Self(format!("IO error: {e}"))
    }
}

/// Helper privado: abre `souls_state.db` em modo leitura+escrita, garantindo
/// que `.souls_data/` exista (idempotente) e que FKs estejam ON.
fn open_db(workspace_root: &Path) -> Result<Connection, SocraticHandlerError> {
    let souls_data_dir = workspace_root.join(".souls_data");
    std::fs::create_dir_all(&souls_data_dir).map_err(|e| {
        SocraticHandlerError(format!(
            "Falha ao criar diretório .souls_data/ ({}): {e}",
            souls_data_dir.display()
        ))
    })?;
    let db_path = souls_data_dir.join("souls_state.db");
    let mut conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(|e| SocraticHandlerError(format!("Falha ao abrir souls_state.db: {e}")))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(|e| SocraticHandlerError(format!("Falha ao configurar PRAGMAs: {e}")))?;
    thinking::ops::migrate_v3_to_v5(&mut conn)?;
    Ok(conn)
}

/// Helper privado: abre `souls_state.db` em modo Read-Only exclusivo com `PRAGMA query_only = ON;`.
fn open_read_db(workspace_root: &Path) -> Result<Connection, SocraticHandlerError> {
    let souls_data_dir = workspace_root.join(".souls_data");
    let db_path = souls_data_dir.join("souls_state.db");
    if !db_path.exists() {
        return open_db(workspace_root);
    }
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| SocraticHandlerError(format!("Falha ao abrir souls_state.db (read-only): {e}")))?;
    conn.execute_batch(
        "PRAGMA query_only = ON;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(|e| SocraticHandlerError(format!("Falha ao configurar PRAGMAs Read-Only: {e}")))?;
    Ok(conn)
}

/// Resolve o workspace root (1 nível acima de `src-tauri/`, ou env override).
fn default_workspace_root() -> PathBuf {
    // Tenta o env var primeiro (SODA_ENV_WORKSPACE_ROOT).
    if let Ok(p) = std::env::var("SOULS_WORKSPACE_ROOT") {
        return PathBuf::from(p);
    }
    // Fallback: CWD, e se for src-tauri, sobe um nível.
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
        cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd)
    } else {
        cwd
    }
}

/// `export_session` — reconstrói a árvore socrática de uma sessão e a
/// formata como JSON canônico (default) ou Markdown com indentação.
///
/// `format`: `"json"` (default), `"markdown"` ou `"md"`.
pub fn handle_export_session(
    session_id: &str,
    format: Option<&str>,
    workspace_root: Option<&Path>,
) -> Result<Value, SocraticHandlerError> {
    let fmt = format.unwrap_or("json");
    let root = workspace_root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(default_workspace_root);
    let conn = open_read_db(&root)?;
    let thoughts = thinking::ops::list_thoughts_for_session(&conn, session_id)?;
    let (roots, children) = build_socratic_tree(&thoughts);

    let payload = match fmt {
        "markdown" | "md" => {
            let body = render_socratic_markdown(&roots, &children);
            json!({
                "session_id": session_id,
                "format": "markdown",
                "total_thoughts": thoughts.len(),
                "root_count": roots.len(),
                "body": body,
            })
        }
        _ => {
            let nodes: Vec<Value> = thoughts
                .iter()
                .map(|t| {
                    json!({
                        "thought_id": t.thought_id,
                        "session_id": t.session_id,
                        "branch_id": t.branch_id,
                        "parent_thought_id": t.parent_thought_id,
                        "thought_type": t.thought_type.as_str(),
                        "content": t.content,
                        "step_number": t.step_number,
                        "duration_ms": t.duration_ms,
                        "created_at": t.created_at,
                    })
                })
                .collect();
            let adjacency: std::collections::BTreeMap<&str, Vec<&str>> = children
                .iter()
                .map(|(k, v)| (*k, v.iter().map(|t| t.thought_id.as_str()).collect()))
                .collect();
            let adjacency_json: std::collections::BTreeMap<String, Vec<String>> = adjacency
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
                .collect();
            json!({
                "session_id": session_id,
                "format": "json",
                "total_thoughts": thoughts.len(),
                "root_count": roots.len(),
                "nodes": nodes,
                "children": adjacency_json,
            })
        }
    };

    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "session_id": session_id,
        "format": fmt,
        "total_thoughts": thoughts.len(),
    }))
}

/// `analyze_session` — computa métricas FinOps cognitivas da sessão.
pub fn handle_analyze_session(
    session_id: &str,
    workspace_root: Option<&Path>,
) -> Result<Value, SocraticHandlerError> {
    let root = workspace_root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(default_workspace_root);
    let conn = open_read_db(&root)?;

    let thoughts = thinking::ops::list_thoughts_for_session(&conn, session_id)?;
    let metrics = thinking::compute_metrics(&thoughts);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&json!({
                "session_id": session_id,
                "metrics": &metrics,
            }))
            .unwrap_or_default()
        }],
        "session_id": session_id,
        "metrics": &metrics,
    }))
}

/// `merge_sessions` — fusão atômica last-write-wins.
///
/// **Modo síncrono** (default): faz o merge em uma única transação SQLite
/// (BEGIN EXCLUSIVE + COMMIT) para garantir consistência. Retorna o
/// número de pensamentos migrados + o transporte usado.
///
/// **Modo fire-and-forget** (`use_mpsc=true`): despacha cada pensamento
/// para o `SocraticWriteWorker` via canal MPSC. Não bloqueia. Caller
/// recebe `thoughts_enqueued` e `thoughts_dropped_backpressure`.
///
/// Se o caller fornecer um `socratic_tx` (SocraticWriteHandle), o modo
/// fire-and-forget é usado; caso contrário, modo síncrono.
pub fn handle_merge_sessions(
    source_session_id: &str,
    target_session_id: &str,
    workspace_root: Option<&Path>,
    socratic_handle: Option<&thinking::SocraticWriteHandle>,
) -> Result<Value, SocraticHandlerError> {
    if source_session_id == target_session_id {
        return Err(SocraticHandlerError(
            "merge_sessions: source e target devem ser distintos".to_string(),
        ));
    }
    let root = workspace_root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(default_workspace_root);
    let conn = open_db(&root)?;

    // Lista source (read-only).
    let source_thoughts = thinking::ops::list_thoughts_for_session(&conn, source_session_id)?;

    if let Some(handle) = socratic_handle {
        // HIPER-FORWARD via MPSC.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default();
        handle
            .try_send(thinking::SocraticOp::UpsertSessionFire {
                session_id: target_session_id.to_string(),
                created_at: now,
                metadata: "{}".to_string(),
            })
            .map_err(|_| {
                SocraticHandlerError(format!(
                    "SocraticWriteWorker saturado: canal MPSC bounded(512) rejeitou o upsert do target '{target_session_id}'."
                ))
            })?;
        let mut id_remap: HashMap<String, String> = HashMap::new();
        let mut inserted: usize = 0;
        let mut dropped_backpressure: usize = 0;
        for t in &source_thoughts {
            let new_id = thinking::ops::gen_simple_uuid("th_merge");
            id_remap.insert(t.thought_id.clone(), new_id.clone());
            let new_parent = t
                .parent_thought_id
                .as_ref()
                .and_then(|p| id_remap.get(p).cloned());
            let remapped = SocraticThought {
                thought_id: new_id,
                session_id: target_session_id.to_string(),
                branch_id: t.branch_id.clone(),
                parent_thought_id: new_parent,
                thought_type: t.thought_type,
                content: t.content.clone(),
                step_number: t.step_number,
                duration_ms: t.duration_ms,
                created_at: t.created_at,
            };
            if handle
                .try_send(thinking::SocraticOp::UpsertThoughtFire {
                    thought: remapped,
                })
                .is_err()
            {
                dropped_backpressure += 1;
            } else {
                inserted += 1;
            }
        }
        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "Merge HIPER-FORWARD despachado: {inserted} pensamentos enfileirados de '{source_session_id}' → '{target_session_id}' (dropped_backpressure={dropped_backpressure})."
                )
            }],
            "source_session_id": source_session_id,
            "target_session_id": target_session_id,
            "thoughts_merged": inserted,
            "thoughts_dropped_backpressure": dropped_backpressure,
            "transport": "socratic_mpsc_v1",
        }))
    } else {
        // Modo síncrono: BEGIN EXCLUSIVE + INSERT OR REPLACE.
        let mut conn = conn;
        let tx = conn.transaction()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default();
        thinking::ops::upsert_socratic_session(&tx, target_session_id, now, "{}")?;
        let source_thoughts_in_tx =
            thinking::ops::list_thoughts_for_session(&tx, source_session_id)?;
        let mut id_remap: HashMap<String, String> = HashMap::new();
        let mut inserted: usize = 0;
        for t in &source_thoughts_in_tx {
            let new_id = thinking::ops::gen_simple_uuid("th_merge");
            id_remap.insert(t.thought_id.clone(), new_id.clone());
            let new_parent = t
                .parent_thought_id
                .as_ref()
                .and_then(|p| id_remap.get(p).cloned());
            let remapped = SocraticThought {
                thought_id: new_id,
                session_id: target_session_id.to_string(),
                branch_id: t.branch_id.clone(),
                parent_thought_id: new_parent,
                thought_type: t.thought_type,
                content: t.content.clone(),
                step_number: t.step_number,
                duration_ms: t.duration_ms,
                created_at: t.created_at,
            };
            thinking::ops::upsert_socratic_thought(&tx, &remapped)?;
            inserted += 1;
        }
        tx.commit()?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "Merge atômico last-write-wins concluído: {inserted} pensamentos migrados de '{source_session_id}' → '{target_session_id}'."
                )
            }],
            "source_session_id": source_session_id,
            "target_session_id": target_session_id,
            "thoughts_merged": inserted,
            "transport": "synchronous_txn_v1",
        }))
    }
}

/// Reconstrução iterativa da árvore socrática (movida do bin para lib).
fn build_socratic_tree(
    thoughts: &[thinking::SocraticThought],
) -> (
    Vec<&thinking::SocraticThought>,
    HashMap<&str, Vec<&thinking::SocraticThought>>,
) {
    let mut roots: Vec<&thinking::SocraticThought> = Vec::new();
    let mut children: HashMap<&str, Vec<&thinking::SocraticThought>> = HashMap::new();
    for t in thoughts {
        match &t.parent_thought_id {
            None => roots.push(t),
            Some(parent_id) => {
                children
                    .entry(parent_id.as_str())
                    .or_default()
                    .push(t);
            }
        }
    }
    for v in children.values_mut() {
        v.sort_by(|a, b| {
            a.step_number
                .cmp(&b.step_number)
                .then_with(|| a.branch_id.cmp(&b.branch_id))
        });
    }
    roots.sort_by(|a, b| {
        a.branch_id
            .cmp(&b.branch_id)
            .then_with(|| a.step_number.cmp(&b.step_number))
    });
    (roots, children)
}

/// Renderiza a árvore em Markdown com indentação por profundidade.
fn render_socratic_markdown(
    roots: &[&thinking::SocraticThought],
    children: &HashMap<&str, Vec<&thinking::SocraticThought>>,
) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str("# Socratic Session Tree\n\n");
    let mut stack: Vec<(&thinking::SocraticThought, usize)> = roots
        .iter()
        .rev()
        .map(|t| (*t, 0_usize))
        .collect();
    while let Some((node, depth)) = stack.pop() {
        let indent = "  ".repeat(depth);
        out.push_str(&format!(
            "{indent}- **{}** [{}] step={} dur={}ms\n",
            node.thought_type.as_str(),
            node.thought_id,
            node.step_number,
            node.duration_ms
        ));
        if !node.content.trim().is_empty() {
            for line in node.content.lines() {
                out.push_str(&format!("{indent}  > {line}\n"));
            }
        }
        if let Some(kids) = children.get(node.thought_id.as_str()) {
            for k in kids.iter().rev() {
                stack.push((*k, depth + 1));
            }
        }
    }
    out
}

// Re-exports para evitar warning de unused imports.
#[allow(dead_code)]
pub(crate) type _ThoughtIdMarker = ThoughtId;
#[allow(dead_code)]
pub(crate) type _ThoughtTypeMarker = ThoughtType;
