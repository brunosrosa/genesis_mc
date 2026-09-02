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

pub use inference::*;
pub use vram_hardware::*;
pub use security::*;
pub use socratic::*;
pub use governance::*;

// Aliases de compatibilidade retroativa
pub use inference::burn_engine as burn_agnostic;
pub use inference::llama_logit_probing as llama_cpp4_logit;
pub use inference::pulp_matrix_engine as pulp_lele;

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
