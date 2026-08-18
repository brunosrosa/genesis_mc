// SOULS MC — Marco V: Svelte 5 Runes Store de Telemetria.
//
// O canal binário do Tauri emite 8 bytes (u64 packed LE) a 1Hz.
// Este store decodifica o buffer via `DataView` (zero-parse, zero-JSON)
// e atualiza Runes (`$state`, `$derived`) sob `requestAnimationFrame`
// para alinhamento com o refresh do monitor (60 FPS consistente).
//
// ## Decoupling temporal
// - Rust pulsa a 1Hz (`tokio::time::interval`).
// - rAF pulsa a 60Hz no browser.
// - Svelte 5 Runes faz diffing estrutural: se o u64 packed é idêntico
//   ao tick anterior, nenhum repaint é disparado. Zero Layout Shift.

import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ---------------------------------------------------------------------------
// Bit-masks — DEVEM ser idênticas às constantes em `core/hardware_watchdog.rs`.
// ---------------------------------------------------------------------------
const MASK_VRAM = (1n << 20n) - 1n; // bits 0..19
const MASK_RAM = ((1n << 20n) - 1n) << 20n; // bits 20..39
const MASK_CPU_TEMP = ((1n << 10n) - 1n) << 40n; // bits 40..49
const MASK_GPU_TEMP = ((1n << 10n) - 1n) << 50n; // bits 50..59
const MASK_FLAGS = 0xFn << 60n; // bits 60..63

// Limite físico de VRAM para a flag de "PRESSAO_CRITICA" (RTX 2060m = 6GB).
const VRAM_CRITICAL_MB = 5000;

export interface TelemetryState {
  vram_mb: number;
  ram_mb: number;
  cpu_temp: number;
  gpu_temp: number;
  thermal_throttle: boolean;
}

// ---------------------------------------------------------------------------
// Runes reativas (Svelte 5).
// ---------------------------------------------------------------------------
export const telemetry = $state<TelemetryState>({
  vram_mb: 0,
  ram_mb: 0,
  cpu_temp: 0,
  gpu_temp: 0,
  thermal_throttle: false,
});

/**
 * Função utilitária estrita para sanitização de proxy (ADR-005).
 * Remove completamente a casca de Proxy das Runes antes de qualquer repasse ao Rust.
 */
export function snapshot_telemetry(): TelemetryState {
  return $state.snapshot(telemetry);
}

/**
 * Classifica o estado térmico reativamente sem spinners.
 */
export function thermal_status(): "PRESSAO_CRITICA" | "OCIOSO" {
  return telemetry.vram_mb > VRAM_CRITICAL_MB
    ? "PRESSAO_CRITICA"
    : "OCIOSO";
}

// ---------------------------------------------------------------------------
// Decoder binário puro (DataView + BigInt — zero JSON).
// ---------------------------------------------------------------------------
export function decode_payload(arrayBuffer: ArrayBuffer): void {
  if (arrayBuffer.byteLength < 8) return;

  const view = new DataView(arrayBuffer);
  const state = view.getBigUint64(0, true /* little-endian */);

  const vram = Number(state & MASK_VRAM);
  const ram = Number((state & MASK_RAM) >> 20n);
  // Temperaturas são x2 (0.5 °C LSB).
  const cpu = Number((state & MASK_CPU_TEMP) >> 40n) * 0.5;
  const gpu = Number((state & MASK_GPU_TEMP) >> 50n) * 0.5;
  const flags = Number((state & MASK_FLAGS) >> 60n);

  telemetry.vram_mb = vram;
  telemetry.ram_mb = ram;
  telemetry.cpu_temp = cpu;
  telemetry.gpu_temp = gpu;
  telemetry.thermal_throttle = (flags & 0b0001) !== 0;
}

/**
 * Atualiza o estado da telemetria diretamente a partir de payload descompactado.
 */
export function update_unpacked_state(state: Partial<TelemetryState>): void {
  if (state.vram_mb !== undefined) telemetry.vram_mb = state.vram_mb;
  if (state.ram_mb !== undefined) telemetry.ram_mb = state.ram_mb;
  if (state.cpu_temp !== undefined) telemetry.cpu_temp = state.cpu_temp;
  if (state.gpu_temp !== undefined) telemetry.gpu_temp = state.gpu_temp;
  if (state.thermal_throttle !== undefined) telemetry.thermal_throttle = state.thermal_throttle;
}

// ---------------------------------------------------------------------------
// Bridge: Conecta canal Tauri Zero-Copy e evento 'hardware-telemetry' via rAF
// ---------------------------------------------------------------------------
export async function bind_channel_to_runes(): Promise<() => void> {
  const channel = new Channel<Uint8Array>();
  let pendingBuffer: ArrayBuffer | null = null;
  let pendingState: Partial<TelemetryState> | null = null;
  let rafId: number | null = null;
  let cancelled = false;
  let unlistenEvent: UnlistenFn | null = null;

  // rAF loop: Micro-batching a 60 FPS
  const tick = (): void => {
    if (cancelled) return;
    if (pendingBuffer !== null) {
      const buf = pendingBuffer;
      pendingBuffer = null;
      decode_payload(buf);
    }
    if (pendingState !== null) {
      const st = pendingState;
      pendingState = null;
      update_unpacked_state(st);
    }
    rafId = requestAnimationFrame(tick);
  };
  rafId = requestAnimationFrame(tick);

  // Wire do canal binário 1Hz (u64 LE packed)
  channel.onmessage = (bytes: Uint8Array) => {
    pendingBuffer = bytes.buffer.slice(
      bytes.byteOffset,
      bytes.byteOffset + bytes.byteLength
    ) as ArrayBuffer;
  };

  try {
    await invoke("start_watchdog_stream", { channel });
  } catch {
    // Fallback gracioso se start_watchdog_stream não estiver disponível
  }

  // Assinatura do canal de eventos 'hardware-telemetry'
  try {
    unlistenEvent = await listen<Partial<TelemetryState>>("hardware-telemetry", (event) => {
      pendingState = event.payload;
    });
  } catch {
    // Fallback gracioso em ambiente web standalone
  }

  // Cleanup: cancela rAF + desinscreve listeners
  return () => {
    cancelled = true;
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
    }
    unlistenEvent?.();
  };
}
