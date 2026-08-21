//! `ipc_bridge.rs` — Emissor Telemétrico do Watchdog Binário (Tauri v2 IPC).
//!
//! **ADR-003 / ADR-014 / ADR-027 (Agnosticismo de Hardware & Higiene Zero-Copy):**
//! - Lê o `WATCHDOG_STATE` (`AtomicU64` empacotado lock-free) em taxa de 1Hz a 5Hz.
//! - Converte para 8 bytes contíguos little-endian (`u64::to_le_bytes()`).
//! - Emite o buffer cru para o canal `"hardware-telemetry"` no Tauri WebView.
//! - O decodificador `DataView` no Svelte 5 (`getBigUint64(0, true)`) consome
//!   os bytes diretamente da RAM com 0 alocações de string JSON no V8.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use crate::core::hardware_watchdog;

/// Nome canônico do evento Tauri de telemetria binária de hardware.
pub const HARDWARE_TELEMETRY_EVENT: &str = "hardware-telemetry";

/// Taxa padrão do emissor de telemetria (5Hz = 200ms para 60 FPS Canvas smoothing).
pub const DEFAULT_TELEMETRY_INTERVAL_MS: u64 = 200;

/// Trait abstrato para envio de telemetria binária (agnóstico de runtime).
pub trait TelemetrySink: Send + Sync {
    fn emit_telemetry(&self, event: &str, payload: &[u8]) -> Result<(), String>;
}

/// Sink para execução real em produção sob runtime Tauri v2.
#[cfg(feature = "tauri-app")]
pub struct TauriTelemetrySink {
    app_handle: tauri::AppHandle,
}

#[cfg(feature = "tauri-app")]
impl TauriTelemetrySink {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

#[cfg(feature = "tauri-app")]
impl TelemetrySink for TauriTelemetrySink {
    fn emit_telemetry(&self, event: &str, payload: &[u8]) -> Result<(), String> {
        use tauri::Emitter;
        self.app_handle
            .emit(event, payload.to_vec())
            .map_err(|e| format!("Falha ao emitir {event} via Tauri: {e}"))
    }
}

/// Tipo de amostra de telemetria binária (evento, 8 bytes LE).
pub type TelemetrySample = (String, [u8; 8]);

/// Sink in-memory para testes unitários e ambientes headless.
#[derive(Default, Clone)]
pub struct InMemoryTelemetrySink {
    pub emissions: Arc<Mutex<Vec<TelemetrySample>>>,
}

impl InMemoryTelemetrySink {
    pub fn new() -> Self {
        Self {
            emissions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn last_emission(&self) -> Option<[u8; 8]> {
        self.emissions
            .lock()
            .ok()?
            .last()
            .map(|(_, bytes)| *bytes)
    }

    pub fn count(&self) -> usize {
        self.emissions.lock().map(|v| v.len()).unwrap_or(0)
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.emissions.lock() {
            g.clear();
        }
    }
}

impl TelemetrySink for InMemoryTelemetrySink {
    fn emit_telemetry(&self, event: &str, payload: &[u8]) -> Result<(), String> {
        if payload.len() != 8 {
            return Err(format!("Payload de telemetria deve ter 8 bytes, recebeu {}", payload.len()));
        }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(payload);
        if let Ok(mut g) = self.emissions.lock() {
            g.push((event.to_string(), arr));
        }
        Ok(())
    }
}

static GLOBAL_TELEMETRY_SINK: OnceLock<Arc<dyn TelemetrySink>> = OnceLock::new();

/// Configura o sink global de telemetria. Idempotente.
pub fn set_telemetry_sink(sink: Arc<dyn TelemetrySink>) -> bool {
    GLOBAL_TELEMETRY_SINK.set(sink).is_ok()
}

/// Retorna o sink global de telemetria configurado ou um fallback seguro.
pub fn telemetry_sink() -> Arc<dyn TelemetrySink> {
    GLOBAL_TELEMETRY_SINK
        .get()
        .cloned()
        .unwrap_or_else(|| Arc::new(InMemoryTelemetrySink::new()))
}

/// Ponte IPC do Watchdog Binário para streaming contínuo.
#[derive(Debug, Clone)]
pub struct WatchdogIpcBridge {
    interval: Duration,
}

impl Default for WatchdogIpcBridge {
    fn default() -> Self {
        Self::new(Duration::from_millis(DEFAULT_TELEMETRY_INTERVAL_MS))
    }
}

impl WatchdogIpcBridge {
    /// Constrói a ponte com taxa configurável (1Hz = 1000ms a 5Hz = 200ms).
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }

    /// Constrói a ponte a partir de uma frequência em Hertz (1 a 5 Hz).
    pub fn from_hz(rate_hz: u32) -> Self {
        let hz = rate_hz.clamp(1, 10);
        let millis = (1000 / hz) as u64;
        Self::new(Duration::from_millis(millis))
    }

    /// Spawna a task Tokio assíncrona consumindo `WATCHDOG_STATE` e emitindo bytes LE.
    pub fn spawn(&self, sink: Arc<dyn TelemetrySink>) -> tokio::task::JoinHandle<()> {
        let interval_dur = self.interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval_dur);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Leitura lock-free O(1) do AtomicU64
                let state = hardware_watchdog::get_state()
                    .map(|arc| arc.load(Ordering::Acquire))
                    .unwrap_or(0u64);

                let bytes: [u8; 8] = state.to_le_bytes();

                if sink.emit_telemetry(HARDWARE_TELEMETRY_EVENT, &bytes).is_err() {
                    // Se o sink falhou (ex: webview fechou), mantém o loop vivo sem pânico
                }
            }
        })
    }

    /// Helper de bootstrap: inicia o loop com o sink global.
    pub fn start_global(&self) -> tokio::task::JoinHandle<()> {
        self.spawn(telemetry_sink())
    }
}

/// Helper para converter 8 bytes little-endian de volta para u64 (emulação DataView V8).
#[inline]
pub fn decode_v8_dataview_u64_le(bytes: &[u8]) -> Result<u64, String> {
    if bytes.len() != 8 {
        return Err(format!("Tamanho inválido para DataView u64: {}", bytes.len()));
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hardware_watchdog::pack_state;

    #[tokio::test]
    async fn test_watchdog_ipc_bridge_emits_binary_telemetry() {
        let sink = Arc::new(InMemoryTelemetrySink::new());
        let bridge = WatchdogIpcBridge::from_hz(5); // 200ms

        // Popula o estado global
        let state_arc = hardware_watchdog::WATCHDOG_STATE
            .get_or_init(|| Arc::new(std::sync::atomic::AtomicU64::new(0)));
        let test_packed = pack_state(4096, 16384, 55.0, 68.0, 0);
        state_arc.store(test_packed, Ordering::Release);

        let handle = bridge.spawn(sink.clone());

        // Aguarda ao menos 1 tick
        tokio::time::sleep(Duration::from_millis(250)).await;
        handle.abort();

        assert!(sink.count() >= 1, "Deve ter emitido ao menos 1 amostra de telemetria");
        let last_bytes = sink.last_emission().expect("Deve conter amostra");
        let decoded = decode_v8_dataview_u64_le(&last_bytes).expect("Decodificação DataView");
        assert_eq!(decoded, test_packed);
        assert_eq!(hardware_watchdog::decode_vram_mb(decoded), 4096);
        assert_eq!(hardware_watchdog::decode_ram_mb(decoded), 16384);
    }
}
