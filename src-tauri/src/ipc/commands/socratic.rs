/// `socratic_export_session` — exporta árvore de pensamentos socráticos.
#[tauri::command]
pub async fn socratic_export_session(
    session_id: String,
    format: Option<String>,
) -> Result<serde_json::Value, String> {
    crate::cognition::thinking::handlers::handle_export_session(
        &session_id,
        format.as_deref(),
        None,
    )
    .map_err(|e| format!("export_session falhou: {e}"))
}

/// `socratic_analyze_session` — métricas FinOps cognitivas por sessão.
#[tauri::command]
pub async fn socratic_analyze_session(session_id: String) -> Result<serde_json::Value, String> {
    crate::cognition::thinking::handlers::handle_analyze_session(&session_id, None)
        .map_err(|e| format!("analyze_session falhou: {e}"))
}

/// `socratic_merge_sessions` — fusão atômica via barramento MPSC HIPER-FORWARD.
#[tauri::command]
pub async fn socratic_merge_sessions(
    source_session_id: String,
    target_session_id: String,
) -> Result<serde_json::Value, String> {
    crate::cognition::thinking::handlers::handle_merge_sessions(
        &source_session_id,
        &target_session_id,
        None,
        None, // síncrono por padrão (Tauri frontend prefere transação explícita)
    )
    .map_err(|e| format!("merge_sessions falhou: {e}"))
}
