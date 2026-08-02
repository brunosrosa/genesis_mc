pub mod hardware_profiler;
pub mod file_locker;
pub mod inference_adapter;
pub mod model_registry;
pub mod response_healing;
pub mod model_manager;
pub mod headroom_engine;
pub mod mcp_transport; // SOULS-CANIBALIZED: trait McpTransport + LeanVacuum

#[cfg(feature = "llama_backend")]
pub mod llama_engine;

#[cfg(feature = "mistral_backend")]
pub mod mistral_engine;

pub mod bitnet_daemon;
pub mod engine_trait;
pub mod v3_ignition_tests;

// SOULS V4 — Topologia dos 8 motores de inferencia (stubs conformantes sob EphemeralInferEngine).
pub mod llama_cpp4_logit;
pub mod mistral_sidecar;
pub mod bitnet_engine;
pub mod pulp_lele;
pub mod burn_agnostic;
pub mod ort_scorer;

