// SOULS MC — Marco V: SODA Canvas v0.1 & Tauri IPC Zero-Copy Telemetry Bridge.
//
// Envia os 8 bytes packed do `WATCHDOG_STATE` a 1Hz diretamente para a
// Webview, sem passar por `serde_json` (ADR-003 §37-38).
//
// ## Contrato binário
//
// | Offset | Bytes | Campo              | Decode (DataView)            |
// |--------|-------|--------------------|------------------------------|
// | 0      | 8     | `state_u64_packed` | `getBigUint64(0, true /*LE*/)` |
//
// Layout interno do `u64` é o mesmo de `core/hardware_watchdog::pack_state`:
// - bits  0..19  vram_used_mb
// - bits 20..39  ram_used_mb
// - bits 40..49  cpu_temp_celsius_x2
// - bits 50..59  gpu_temp_celsius_x2
// - bits 60..63  flags (bit 60 = thermal_throttle)
//
// ## Concorrência
//
// - `Ordering::Relaxed` no load: o AtomicU64 é publicado pela thread
//   `souls-hardware-watchdog` com `Ordering::Release` no store. Apenas
//   um único consumer (este stream) lê; não há sincronização adicional
//   necessária entre leituras consecutivas (FIFO causal pelo próprio
//   `Arc<AtomicU64>`).
// - `Ordering::Acquire` foi considerado e rejeitado: o Webview não tem
//   dependência de ordem entre ticks (cada tick é idempotente), e o
//   canal Tauri já impõe happens-before na sua fronteira IPC.
//
// ## Cancelamento
//
// Quando a Webview desmonta (reload / window close), o `Channel` do lado
// Rust é fechado pelo runtime Tauri. A próxima chamada `channel.send()`
// retorna `Err`, encerrando o loop Tokio de forma cooperativa.

use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::ipc::Channel;

use crate::core::hardware_watchdog;

/// Intervalo de tick (1Hz). Igual ao `WATCHDOG_POLL_INTERVAL_MS` do core.
const TICK_MS: u64 = 1_000;

/// Spawna uma task Tokio que envia o estado do watchdog como 8 bytes
/// little-endian (sem JSON) pelo canal binário Tauri a cada 1 segundo.
///
/// Esta função NÃO bloqueia o caller — o `tokio::spawn` retorna
/// imediatamente e a task roda em background. O caller deve passar
/// o `Channel<Vec<u8>>` recebido do comando Tauri.
pub fn spawn_watchdog_channel(_app: tauri::AppHandle, channel: Channel<Vec<u8>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));
        // Skip caso o runtime esteja sobrecarregado — preferimos pular um
        // tick a empilhar ticks atrasados (evita avalanches de send).
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            // Lê o estado atômico (lock-free, sem contenção com a thread
            // watchdog). Se ainda não foi inicializado, envia zeros
            // (a primeira amostra é sempre trivial).
            let state = hardware_watchdog::get_state()
                .map(|arc| arc.load(Ordering::Relaxed))
                .unwrap_or(0u64);

            let bytes: [u8; 8] = state.to_le_bytes();

            // Se o canal foi fechado pelo lado JS (WebView desmontou),
            // `send` retorna Err → encerra a task cooperativamente.
            if channel.send(bytes.to_vec()).is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hardware_watchdog::pack_state;

    /// TDD: `pack_state` deve produzir exatamente 8 bytes little-endian
    /// quando convertido via `u64::to_le_bytes()`. O frontend depende
    /// deste layout exato no decoder `DataView.getBigUint64(0, true)`.
    #[test]
    fn pack_state_little_endian_is_eight_bytes() {
        let state = pack_state(2048, 16384, 65.0, 72.0, 0);
        let bytes = state.to_le_bytes();
        assert_eq!(bytes.len(), 8, "u64 LE deve ter exatamente 8 bytes");
    }

    /// TDD: bits 0..19 codificam vram_used_mb. O LE encoding coloca
    /// esses bits nos 4 bytes menos significativos (bytes 0..3).
    #[test]
    fn pack_state_le_byte0_through_3_hold_vram() {
        let state = pack_state(0x12345, 0, 0.0, 0.0, 0);
        let bytes = state.to_le_bytes();
        let vram_back = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        // vram é truncado para 20 bits (mask 0xFFFFF).
        assert_eq!(vram_back, 0x12345 & 0xFFFFF);
    }

    /// TDD: bits 20..39 codificam ram_used_mb. O LE encoding coloca
    /// esses bits nos bytes 2..5 (overlap com vram nos 2 primeiros bytes
    /// do bloco ram).
    #[test]
    fn pack_state_le_byte2_through_5_hold_ram() {
        let state = pack_state(0, 0xABCDE, 0.0, 0.0, 0);
        let bytes = state.to_le_bytes();
        // Replica exatamente a decodificação do JS (DataView big-endian-agnostic):
        // state >> 20 & ((1<<20)-1)
        let ram_back = (state >> 20) & 0xFFFFF;
        assert_eq!(ram_back, 0xABCDE & 0xFFFFF);
        // Sanidade: o byte 4 LE deve conter parte do ram shift.
        let byte4 = bytes[4];
        assert_eq!(byte4, ((0xABCDEu64 >> 4) & 0xFF) as u8);
    }

    /// TDD: pack_state(0,0,0,0,0) é o vetor de zeros canônico.
    /// Este é o payload inicial que o WebView recebe antes do primeiro
    /// tick do watchdog.
    #[test]
    fn pack_state_zero_yields_eight_zero_bytes() {
        let state = pack_state(0, 0, 0.0, 0.0, 0);
        assert_eq!(state, 0u64);
        assert_eq!(state.to_le_bytes(), [0u8; 8]);
    }

    /// TDD: roundtrip — pack → unpack deve devolver os mesmos valores
    /// dentro da precisão x2 das temperaturas (0.5 °C LSB).
    #[test]
    fn pack_state_roundtrip_preserves_metrics() {
        let vram = 4500u32;
        let ram = 24576u32;
        let cpu = 67.5f32;
        let gpu = 78.0f32;
        let state = pack_state(vram, ram, cpu, gpu, 0);
        assert_eq!(hardware_watchdog::decode_vram_mb(state), vram);
        assert_eq!(hardware_watchdog::decode_ram_mb(state), ram);
        assert!((hardware_watchdog::decode_cpu_temp_c(state) - cpu).abs() < 0.01);
        assert!((hardware_watchdog::decode_gpu_temp_c(state) - gpu).abs() < 0.01);
    }
}
