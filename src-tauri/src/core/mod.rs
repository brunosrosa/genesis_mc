pub mod hardware_profiler;
pub mod file_locker;
pub mod inference_adapter;
pub mod model_registry;
pub mod response_healing;
pub mod model_manager;
pub mod headroom_engine;
pub mod mcp_transport; // SOULS-CANIBALIZED: trait McpTransport + LeanVacuum

// Marco I · v6.1 — Agnostic L7 Gateway (componentes canibalizados)
pub mod gateway_config; // SOULS-CANIBALIZED Marco I: JSONC parser + GatewayConfig SSOT
pub mod peak_ewma; // SOULS-CANIBALIZED Marco I: PeakEWMA α=0.3 + lock-free ring buffer
pub mod sticky_router; // SOULS-CANIBALIZED Marco I: Sticky routing por session_id (Prefix Cache)
pub mod pii_redactor; // SOULS-CANIBALIZED Marco I: Aho-Corasick PII redaction (default disabled)
pub mod telemetry_dispatcher; // SOULS-CANIBALIZED Marco I: MPSC → SQLite WAL V5 (worker thread dedicada)
pub mod subprocess_guard; // SOULS-CANIBALIZED Marco I: SubprocessGuard RAII com kill_on_drop

#[cfg(feature = "llama_backend")]
pub mod llama_engine;

#[cfg(feature = "mistral_backend")]
pub mod mistral_engine;

pub mod bitnet_daemon;
pub mod engine_trait;
pub mod v3_ignition_tests;

// SOULS V4 — Topologia dos 8 motores de inferencia (stubs conformantes sob EphemeralInferEngine).
pub mod llama_logit_probing;
pub mod epistemic_prober; // SOULS-CANIBALIZED Marco 4.9.3: Avaliador Epistêmico (Hipocampo) - trait síncrono CPU/AVX2
pub mod cohomology; // SOULS-CANIBALIZED Marco 4.10.0: Cohomologia de Feixes Socráticos (H¹ ≠ 0 → boost conflito_memoria)
pub mod socratic_event_bus; // SOULS-CANIBALIZED Marco 4.10.0: Disjuntor Socrático via IPC Zero-Copy (Tauri event 'socratic_interrupt')
pub mod socratic_interrupt; // SOULS V6 MARCO 5.11.0: Canal de Interrupção Socrática CLI Híbrido
pub mod l7_shield; // SOULS-CANIBALIZED Marco 4.10.0: L7 Shield (MPSC + oneshot para prober síncrono em thread dedicada)
pub mod mistral_sidecar;
pub mod bitnet_engine;
pub mod pulp_matrix_engine;
pub mod burn_engine;
pub mod ort_scorer;
pub mod gliclass_engine; // SOULS V6 MARCO 5.3.0: Sentinela de Borda Bare-Metal OrtScorerEngine (GLiClass Zero-Shot Triage)
pub mod gigatoken_encoder; // SOULS V6 MARCO 5.4.0: GigaTokenEncoder Auto-Curativo & Prefill Bypass
pub mod vram_scheduler; // SOULS V6 MARCO 5.12.0: VRAM Scheduler Dinâmico e Gerenciador de Evicção LRU
pub mod hardware_watchdog; // SOULS V6 MARCO 5.12.0/IV: Watchdog Térmico + WATCHDOG_STATE lock-free
pub mod sandbox; // SOULS V6 MARCO 5.13.0: Isolamento LPAC Nativo e Bypass Gracioso (Windows 11)
pub mod sdd; // SOULS V6 MARCO 5.16.0: Orquestrador de Cascata Documental SDD (SddValidationEngine + State V6)
pub mod semantic_search; // SOULS V6 MARCO VI: Hipocampo Ativo, LanceDB Zero-VRAM, RRF AVX2 e LadybugDB
pub mod chyros_daemon; // SOULS V6 MARCO 5.7.0: Chyros Daemon (AutoDream & Metabolismo Estocástico)
pub mod socratic_cli; // SOULS V6 MARCO 5.11.0: Socratic CLI & CPU Logit Probing Controller

#[cfg(feature = "lora_adapter")]
pub mod llama_lora_adapter; // SOULS V6 MARCO IV: Hot-swap de adaptadores LoRA (ik_llama.cpp FFI)

// Aliases de compatibilidade retroativa
pub use burn_engine as burn_agnostic;
pub use llama_logit_probing as llama_cpp4_logit;
pub use pulp_matrix_engine as pulp_lele;

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


