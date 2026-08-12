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
pub mod sandbox; // SOULS V6 MARCO 5.13.0: Isolamento LPAC Nativo e Bypass Gracioso (Windows 11)
pub mod sdd; // SOULS V6 MARCO 5.16.0: Orquestrador de Cascata Documental SDD (SddValidationEngine + State V6)

// Aliases de compatibilidade retroativa
pub use burn_engine as burn_agnostic;
pub use llama_logit_probing as llama_cpp4_logit;
pub use pulp_matrix_engine as pulp_lele;


