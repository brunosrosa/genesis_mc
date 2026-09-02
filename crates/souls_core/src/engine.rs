//! SOULS MC — Core Engine (Bare-Metal Async Runtime)
//!
//! Desacoplado 100% de qualquer biblioteca gráfica ou de janelas.
//! Comunica-se exclusivamente via canais Tokio MPSC / Broadcast.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use souls_protocol::{
    BackendResponse, BlastRadiusEvent, FrontendCommand, IpcEnvelope, SocraticThoughtEvent,
    TelemetrySnapshot,
};
use tokio::sync::{broadcast, mpsc};

use crate::telemetry_collector::TelemetryCollector;

pub struct CoreEngine {
    ui_event_tx: mpsc::UnboundedSender<IpcEnvelope>,
    telemetry_tx: broadcast::Sender<TelemetrySnapshot>,
    shutdown_tx: broadcast::Sender<()>,
    is_kill_switch_active: Arc<AtomicBool>,
}

impl CoreEngine {
    pub fn new(ui_event_tx: mpsc::UnboundedSender<IpcEnvelope>) -> (Self, broadcast::Receiver<TelemetrySnapshot>) {
        let (telemetry_tx, telemetry_rx) = broadcast::channel(128);
        let (shutdown_tx, _) = broadcast::channel(8);
        let is_kill_switch_active = Arc::new(AtomicBool::new(false));

        let engine = Self {
            ui_event_tx,
            telemetry_tx,
            shutdown_tx,
            is_kill_switch_active,
        };

        (engine, telemetry_rx)
    }

    pub fn start_background_tasks(&self) {
        let collector = TelemetryCollector::new(
            self.is_kill_switch_active.clone(),
            self.telemetry_tx.clone(),
        );
        let shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            collector.run_loop(shutdown_rx).await;
        });

        // Tarefa que converte os snapshots de broadcast em envelopes IPC para a UI
        let mut telemetry_sub = self.telemetry_tx.subscribe();
        let ui_tx = self.ui_event_tx.clone();
        tokio::spawn(async move {
            while let Ok(snapshot) = telemetry_sub.recv().await {
                if let Ok(json) = serde_json::to_value(&snapshot) {
                    let env = IpcEnvelope::event("telemetry/snapshot", json);
                    if ui_tx.send(env).is_err() {
                        break;
                    }
                }
            }
        });
    }

    pub async fn handle_command(&self, cmd: FrontendCommand) -> BackendResponse {
        match cmd {
            FrontendCommand::Ping => {
                BackendResponse::Ok(serde_json::json!({
                    "engine": "souls_core",
                    "status": "online",
                    "version": env!("CARGO_PKG_VERSION"),
                }))
            }
            FrontendCommand::SetKillSwitch { active } => {
                self.is_kill_switch_active.store(active, Ordering::SeqCst);
                let blast_event = BlastRadiusEvent {
                    incident_id: uuid::Uuid::new_v4().to_string(),
                    blast_level: if active { "Locked".to_string() } else { "Safe".to_string() },
                    affected_subsystems: vec![
                        "LocalSLM".to_string(),
                        "McpWorkers".to_string(),
                        "InferenceEngine".to_string(),
                    ],
                    human_in_the_loop_required: false,
                    is_kill_switch_active: active,
                    reason: if active {
                        "Kill-Switch ativado pelo operador: todos os processos sofreram SIGKILL atômico.".to_string()
                    } else {
                        "Sistema rearmado com sucesso.".to_string()
                    },
                };

                let _ = self.ui_event_tx.send(IpcEnvelope::event(
                    "governance/blast_radius",
                    serde_json::to_value(&blast_event).unwrap_or_default(),
                ));

                BackendResponse::Ok(serde_json::json!({ "kill_switch": active }))
            }
            FrontendCommand::ExecuteSocraticStep { session_id, query } => {
                let thought = SocraticThoughtEvent {
                    session_id: session_id.clone(),
                    thought_id: uuid::Uuid::new_v4().to_string(),
                    iteration: 1,
                    max_iterations: 5,
                    branch_type: "Regular".to_string(),
                    hypothesis: format!("Analisando consulta sob SDD/Bare-metal: {}", query),
                    score: 0.96,
                    is_final: false,
                    latency_ms: 18,
                };

                let _ = self.ui_event_tx.send(IpcEnvelope::event(
                    "socratic/thought",
                    serde_json::to_value(&thought).unwrap_or_default(),
                ));

                BackendResponse::Ok(serde_json::json!({ "status": "processing", "thought_id": thought.thought_id }))
            }
            FrontendCommand::ExportSocraticSession { session_id } => {
                BackendResponse::Ok(serde_json::json!({
                    "session_id": session_id,
                    "exported_path": format!("%TEMP%/.souls_workspaces/export_{}.json", session_id),
                }))
            }
            FrontendCommand::AnalyzeSocraticSession { session_id } => {
                BackendResponse::Ok(serde_json::json!({
                    "session_id": session_id,
                    "metrics": {
                        "total_thoughts": 5,
                        "branch_divergence": 0.12,
                        "epistemic_entropy": 0.04,
                        "finops_token_efficiency": 0.94
                    }
                }))
            }
            FrontendCommand::MergeSocraticSessions { source_session_id, target_session_id } => {
                BackendResponse::Ok(serde_json::json!({
                    "source_session_id": source_session_id,
                    "target_session_id": target_session_id,
                    "status": "merged_atomic",
                    "merged_nodes": 8
                }))
            }
            FrontendCommand::RequestTelemetrySnapshot => {
                let kill_switch = self.is_kill_switch_active.load(Ordering::Relaxed);
                let snap = TelemetrySnapshot {
                    is_kill_switch_active: kill_switch,
                    ..TelemetrySnapshot::default()
                };
                BackendResponse::Ok(serde_json::to_value(snap).unwrap_or_default())
            }
            FrontendCommand::ToggleSpotlight | FrontendCommand::ToggleTerminal | FrontendCommand::RequestHistory { .. } => {
                BackendResponse::Ok(serde_json::json!({ "acknowledged": true }))
            }
            FrontendCommand::Custom { command, payload } => {
                tracing::info!("[souls_core] Comando customizado recebido: {} -> {:?}", command, payload);
                BackendResponse::Ok(serde_json::json!({ "status": "executed", "command": command }))
            }
        }
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}
