//! SOULS MC — Core Business and Async Processing Library
//!
//! Arquitetura desacoplada sem dependências de janelas ou interfaces gráficas.

pub mod cognition;
pub mod core;
pub mod engine;
pub mod finops;
pub mod harvester;
pub mod persist;
pub mod process_guard;
pub mod souls_thermal_governor;
pub mod telemetry;
pub mod telemetry_collector;

pub use engine::CoreEngine;
pub use telemetry_collector::TelemetryCollector;

#[cfg(test)]
mod tests {
    use super::*;
    use souls_protocol::FrontendCommand;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_core_engine_ping() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let (engine, _telemetry_rx) = CoreEngine::new(tx);
        let resp = engine.handle_command(FrontendCommand::Ping).await;
        if let souls_protocol::BackendResponse::Ok(val) = resp {
            assert_eq!(val["status"], "online");
            assert_eq!(val["engine"], "souls_core");
        } else {
            panic!("esperado BackendResponse::Ok");
        }
    }

    #[tokio::test]
    async fn test_core_engine_kill_switch() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (engine, _telemetry_rx) = CoreEngine::new(tx);
        let resp = engine.handle_command(FrontendCommand::SetKillSwitch { active: true }).await;
        if let souls_protocol::BackendResponse::Ok(val) = resp {
            assert_eq!(val["kill_switch"], true);
        } else {
            panic!("esperado BackendResponse::Ok");
        }

        // Deve ter emitido evento de blast radius
        let env = rx.recv().await.expect("evento de blast radius esperado");
        assert_eq!(env.channel, "governance/blast_radius");
        assert_eq!(env.payload["is_kill_switch_active"], true);
    }
}
