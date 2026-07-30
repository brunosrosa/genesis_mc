pub mod hardware_profiler;
pub mod inference_adapter;
pub mod model_registry;
pub mod response_healing;
pub mod model_manager;
pub mod headroom_engine;
pub mod mcp_transport; // SODA-CANIBALIZED: trait McpTransport + LeanVacuum

#[cfg(feature = "llama_backend")]
pub mod llama_engine;

#[cfg(feature = "mistral_backend")]
pub mod mistral_engine;

pub mod bitnet_daemon;

