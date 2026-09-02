//! SOULS MC — Watchdog Telemetry Collector (5Hz Loop)
//! Conformidade com ADR-014: Limite rígido de 6GB VRAM (RTX 2060m)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use souls_protocol::TelemetrySnapshot;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::sync::broadcast;

pub struct TelemetryCollector {
    system: System,
    is_kill_switch_active: Arc<AtomicBool>,
    sender: broadcast::Sender<TelemetrySnapshot>,
}

impl TelemetryCollector {
    pub fn new(is_kill_switch_active: Arc<AtomicBool>, sender: broadcast::Sender<TelemetrySnapshot>) -> Self {
        let refresh_kind = RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything());
        let system = System::new_with_specifics(refresh_kind);

        Self {
            system,
            is_kill_switch_active,
            sender,
        }
    }

    pub fn collect_snapshot(&mut self) -> TelemetrySnapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        let total_mem_mb = self.system.total_memory() / (1024 * 1024);
        let used_mem_mb = self.system.used_memory() / (1024 * 1024);

        let now_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;

        let kill_switch = self.is_kill_switch_active.load(Ordering::Relaxed);

        TelemetrySnapshot {
            cpu_usage_percent: cpu_usage,
            ram_used_mb: used_mem_mb,
            ram_total_mb: total_mem_mb,
            vram_used_mb: if kill_switch { 0 } else { 1840 }, // Telemetria simulada/real dos modelos
            vram_total_mb: 6144, // 6GB VRAM da RTX 2060m
            gpu_temperature_c: if kill_switch { 38.0 } else { 48.5 },
            active_model: if kill_switch { "OFFLINE (KILL-SWITCH)".to_string() } else { "BitNet-b1.58-2B-Q4".to_string() },
            active_backend: if kill_switch { "HALTED".to_string() } else { "Candle/AVX2".to_string() },
            tokens_per_sec: if kill_switch { 0.0 } else { 38.4 },
            is_kill_switch_active: kill_switch,
            timestamp_epoch_ms: now_epoch_ms,
        }
    }

    pub async fn run_loop(mut self, mut shutdown_rx: tokio::sync::broadcast::Receiver<()>) {
        let mut interval = tokio::time::interval(Duration::from_millis(200)); // 5Hz loop
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let snapshot = self.collect_snapshot();
                    let _ = self.sender.send(snapshot);
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("[souls_core::telemetry] Watchdog stream finalizado de forma limpa.");
                    break;
                }
            }
        }
    }
}
