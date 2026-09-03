// =============================================================================
// SOULS MC — Core Domain Modularization Architecture (v6.1)
//
// 1. inference/     -> 18 motores de inferência, adaptadores e registradores
// 2. vram_hardware/ -> VRAM Scheduler, HW Watchdog, Profiler e Headroom
// 3. security/      -> Sandbox LPAC, SubprocessGuard, FileLocker, PII e L7 Shield
// 4. socratic/      -> Barramento Socrático, Thoughts Stream, CLI e Cohomologia
// 5. governance/    -> GatewayConfig, SDD Cascade, Semantic Search e Telemetry
// =============================================================================

pub mod inference;
pub mod vram_hardware;
pub mod security;
pub mod socratic;
pub mod governance;

#[allow(ambiguous_glob_reexports)]
pub use inference::*;
#[allow(ambiguous_glob_reexports)]
pub use vram_hardware::*;
#[allow(ambiguous_glob_reexports)]
pub use security::*;
#[allow(ambiguous_glob_reexports)]
pub use socratic::*;
#[allow(ambiguous_glob_reexports)]
pub use governance::*;

// Aliases de compatibilidade retroativa
pub use inference::burn_engine as burn_agnostic;
pub use inference::llama_logit_probing as llama_cpp4_logit;
pub use inference::pulp_matrix_engine as pulp_lele;

pub mod llama_engine {
    pub use crate::core::inference::llama_upstream_engine::LlamaUpstreamEngine as LlamaCppEngine;

    pub fn disable_model_in_sqlite(model_path: &str) {
        let root = crate::core::workspace_root();
        let db_path = root.join(".souls_data").join("models.sqlite");
        if !db_path.exists() {
            tracing::warn!(
                target: "souls_mcp::llama_engine",
                "disable_model_in_sqlite: banco não encontrado em '{}'",
                db_path.display()
            );
            return;
        }
        let conn = match rusqlite::Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    target: "souls_mcp::llama_engine",
                    "disable_model_in_sqlite: falha ao abrir SQLite: {e}"
                );
                return;
            }
        };
        let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        match conn.execute(
            "UPDATE model_registry SET is_active = 0, deactivated_at = ?2, deactivation_reason = 'ffi_crash'
             WHERE (file_path = ?1 OR model_id = ?1) AND is_active != 0",
            rusqlite::params![model_path, now],
        ) {
            Ok(0) => {
                tracing::info!(
                    target: "souls_mcp::llama_engine",
                    "disable_model_in_sqlite: modelo '{model_path}' ja estava desativado ou nao existe no catalogo"
                );
            }
            Ok(updated) => {
                tracing::warn!(
                    target: "souls_mcp::llama_engine",
                    "MARCO III: modelo '{model_path}' DESATIVADO no catalogo apos crash FFI ({updated} rows)"
                );
            }
            Err(e) => {
                tracing::error!(
                    target: "souls_mcp::llama_engine",
                    "disable_model_in_sqlite: falha ao desativar '{model_path}': {e}"
                );
            }
        }
    }
}

use std::path::PathBuf;

/// Resolve o diretório raiz canônico do workspace SOULS MC (`z:\souls_mc`).
pub fn workspace_root() -> PathBuf {
    if let Ok(val) = std::env::var("SOULS_WORKSPACE_ROOT") {
        let p = PathBuf::from(val);
        if p.exists() {
            return p;
        }
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        if manifest_path.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
            if let Some(parent) = manifest_path.parent() {
                return parent.to_path_buf();
            }
        }
        if manifest_path.join("src-tauri").exists() {
            return manifest_path;
        }
        if let Some(parent) = manifest_path.parent() {
            if parent.join("src-tauri").exists() {
                return parent.to_path_buf();
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("Cargo.toml").exists() && cwd.join("src-tauri").exists() {
            return cwd;
        }
        if cwd.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
            if let Some(parent) = cwd.parent() {
                return parent.to_path_buf();
            }
        }
        let mut curr = cwd.as_path();
        for _ in 0..6 {
            if curr.join("src-tauri").exists() && (curr.join("package.json").exists() || curr.join(".agents").exists()) {
                return curr.to_path_buf();
            }
            if let Some(parent) = curr.parent() {
                curr = parent;
            } else {
                break;
            }
        }
    }
    PathBuf::from(".")
}
