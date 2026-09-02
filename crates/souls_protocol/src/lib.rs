//! SOULS MC — Protocol Layer (ADR-001, ADR-005, ADR-014, ADR-041)
//!
//! DTOs tipados, envelopes IPC e eventos de alta frequência serializáveis
//! entre o backend Bare-Metal Rust e o frontend Svelte 5 (Zero-VDOM).

use serde::{Deserialize, Serialize};

/// Envelope de comunicação bidirecional IPC via `window.ipc.postMessage` / `evaluate_script`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEnvelope {
    pub id: String,
    pub channel: String,
    pub payload: serde_json::Value,
}

impl IpcEnvelope {
    pub fn new(id: impl Into<String>, channel: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            channel: channel.into(),
            payload,
        }
    }

    pub fn event(channel: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: String::new(),
            channel: channel.into(),
            payload,
        }
    }
}

/// Instantâneo de telemetria emitido pelo Watchdog em 5Hz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelemetrySnapshot {
    pub cpu_usage_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub gpu_temperature_c: f32,
    pub active_model: String,
    pub active_backend: String,
    pub tokens_per_sec: f32,
    pub is_kill_switch_active: bool,
    pub timestamp_epoch_ms: u64,
}

impl Default for TelemetrySnapshot {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            ram_used_mb: 0,
            ram_total_mb: 16384,
            vram_used_mb: 0,
            vram_total_mb: 6144, // 6GB VRAM da RTX 2060m (ADR-014)
            gpu_temperature_c: 42.0,
            active_model: "BitNet-b1.58-2B".to_string(),
            active_backend: "Candle/AVX2".to_string(),
            tokens_per_sec: 0.0,
            is_kill_switch_active: false,
            timestamp_epoch_ms: 0,
        }
    }
}

/// Eventos do fluxo de pensamento socrático (MCP Sequential Thinking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocraticThoughtEvent {
    pub session_id: String,
    pub thought_id: String,
    pub iteration: u32,
    pub max_iterations: u32,
    pub branch_type: String, // "Regular", "Revision", "Branching"
    pub hypothesis: String,
    pub score: f32,
    pub is_final: bool,
    pub latency_ms: u64,
}

/// Linha de saída do terminal / Engine Room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalStreamEvent {
    pub id: String,
    pub stream_type: String, // "stdout", "stderr", "system", "finops"
    pub line: String,
    pub source_tag: String,
    pub timestamp_epoch_ms: u64,
}

/// Evento de governança de blast radius e contenção FinOps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadiusEvent {
    pub incident_id: String,
    pub blast_level: String, // "Safe", "Caution", "Critical", "Locked"
    pub affected_subsystems: Vec<String>,
    pub human_in_the_loop_required: bool,
    pub is_kill_switch_active: bool,
    pub reason: String,
}

/// Comandos IPC enviados pelo Frontend para o Backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum FrontendCommand {
    Ping,
    ToggleSpotlight,
    ToggleTerminal,
    SetKillSwitch { active: bool },
    ExecuteSocraticStep { session_id: String, query: String },
    ExportSocraticSession { session_id: String },
    AnalyzeSocraticSession { session_id: String },
    MergeSocraticSessions { source_session_id: String, target_session_id: String },
    RequestTelemetrySnapshot,
    RequestHistory { limit: usize },
    Custom { command: String, payload: serde_json::Value },
}

/// Respostas emitidas pelo Backend para comandos IPC síncronos/assíncronos
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum BackendResponse {
    Ok(serde_json::Value),
    Error { code: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_snapshot_serialization() {
        let snapshot = TelemetrySnapshot::default();
        let json = serde_json::to_string(&snapshot).expect("falha ao serializar snapshot");
        assert!(json.contains("vram_total_mb\":6144"));
        let deserialized: TelemetrySnapshot = serde_json::from_str(&json).expect("falha ao desserializar");
        assert_eq!(snapshot, deserialized);
    }

    #[test]
    fn test_ipc_envelope_routing() {
        let envelope = IpcEnvelope::new("req_101", "telemetry/stream", serde_json::json!({"status": "active"}));
        assert_eq!(envelope.channel, "telemetry/stream");
        assert_eq!(envelope.id, "req_101");
    }
}
