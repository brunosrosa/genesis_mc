// SOULS MC — Marco V: Svelte 5 Runes Store de Telemetria Zero-Copy.
//
// O canal binário do Tauri emite 8 bytes (u64 packed LE).
// Este store decodifica o buffer via `DataView` (zero-parse, zero-JSON)
// e armazena as métricas em `$state.raw` sob `requestAnimationFrame` (60 FPS).
//
// Conformidade: ADR-001, ADR-005, ADR-010, ADR-025, ADR-027.

import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { soulsIpc, type TelemetrySnapshot } from "$lib/services/ipc";

// ---------------------------------------------------------------------------
// Bit-masks — DEVEM ser idênticas às constantes em `core/hardware_watchdog.rs`.
// ---------------------------------------------------------------------------
export const MASK_VRAM = (1n << 20n) - 1n; // bits 0..19
export const MASK_RAM = ((1n << 20n) - 1n) << 20n; // bits 20..39
export const MASK_CPU_TEMP = ((1n << 10n) - 1n) << 40n; // bits 40..49
export const MASK_GPU_TEMP = ((1n << 10n) - 1n) << 50n; // bits 50..59
export const MASK_FLAGS = 0xFn << 60n; // bits 60..63

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
// Runa interna com reatividade rasa ($state.raw) para aniquilar o GC do V8.
// ---------------------------------------------------------------------------
let rawState = $state.raw<TelemetryState>({
  vram_mb: 0,
  ram_mb: 0,
  cpu_temp: 0,
  gpu_temp: 0,
  thermal_throttle: false,
});

/**
 * Interface reativa pública sem casca de proxy mutável.
 */
export const telemetry = {
  get vram_mb() { return rawState.vram_mb; },
  get ram_mb() { return rawState.ram_mb; },
  get cpu_temp() { return rawState.cpu_temp; },
  get gpu_temp() { return rawState.gpu_temp; },
  get thermal_throttle() { return rawState.thermal_throttle; },
  get current(): TelemetryState { return rawState; }
};

/**
 * Remove cascas de Proxy do Javascript antes de enviar qualquer payload ao Rust ($state.snapshot).
 */
export function snapshot_telemetry(): TelemetryState {
  return $state.snapshot(rawState);
}

/**
 * Classifica o estado térmico reativamente sem spinners.
 */
export function thermal_status(): "PRESSAO_CRITICA" | "OCIOSO" {
  return rawState.vram_mb > VRAM_CRITICAL_MB
    ? "PRESSAO_CRITICA"
    : "OCIOSO";
}

// ---------------------------------------------------------------------------
// Decoder binário puro (DataView + BigInt — zero JSON).
// ---------------------------------------------------------------------------
export function decode_raw_u64(state: bigint): TelemetryState {
  const vram = Number(state & MASK_VRAM);
  const ram = Number((state & MASK_RAM) >> 20n);
  // Temperaturas são x2 (0.5 °C LSB).
  const cpu = Number((state & MASK_CPU_TEMP) >> 40n) * 0.5;
  const gpu = Number((state & MASK_GPU_TEMP) >> 50n) * 0.5;
  const flags = Number((state & MASK_FLAGS) >> 60n);

  return {
    vram_mb: vram,
    ram_mb: ram,
    cpu_temp: cpu,
    gpu_temp: gpu,
    thermal_throttle: (flags & 0b0001) !== 0,
  };
}

export function decode_payload(bufferLike: ArrayBuffer | Uint8Array | number[]): void {
  let view: DataView;

  if (bufferLike instanceof ArrayBuffer) {
    if (bufferLike.byteLength < 8) return;
    view = new DataView(bufferLike);
  } else if (bufferLike instanceof Uint8Array) {
    if (bufferLike.byteLength < 8) return;
    view = new DataView(bufferLike.buffer, bufferLike.byteOffset, bufferLike.byteLength);
  } else if (Array.isArray(bufferLike)) {
    if (bufferLike.length < 8) return;
    const u8 = new Uint8Array(bufferLike);
    view = new DataView(u8.buffer);
  } else {
    return;
  }

  const telemetryRaw = view.getBigUint64(0, true /* little-endian */);
  rawState = decode_raw_u64(telemetryRaw);
}

/**
 * Atualiza o estado da telemetria diretamente a partir de payload descompactado.
 */
export function update_unpacked_state(state: Partial<TelemetryState>): void {
  rawState = {
    vram_mb: state.vram_mb ?? rawState.vram_mb,
    ram_mb: state.ram_mb ?? rawState.ram_mb,
    cpu_temp: state.cpu_temp ?? rawState.cpu_temp,
    gpu_temp: state.gpu_temp ?? rawState.gpu_temp,
    thermal_throttle: state.thermal_throttle ?? rawState.thermal_throttle,
  };
}

// ---------------------------------------------------------------------------
// Bridge: Conecta canal Tauri Zero-Copy e evento 'hardware-telemetry' via rAF sob demanda
// ---------------------------------------------------------------------------
export async function bind_channel_to_runes(): Promise<() => void> {
  const channel = new Channel<Uint8Array>();
  let pendingBuffer: ArrayBuffer | Uint8Array | null = null;
  let pendingState: Partial<TelemetryState> | null = null;
  let rafScheduled = false;
  let cancelled = false;
  let unlistenEvent: UnlistenFn | null = null;

  const flush = (): void => {
    rafScheduled = false;
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
  };

  const scheduleUpdate = (): void => {
    if (cancelled) return;
    // Se a aba/janela estiver oculta no Systray, não agenda rAF na GPU
    if (typeof document !== "undefined" && document.hidden) {
      return;
    }
    if (!rafScheduled) {
      rafScheduled = true;
      requestAnimationFrame(flush);
    }
  };

  const handleVisibilityChange = (): void => {
    if (typeof document !== "undefined" && !document.hidden) {
      scheduleUpdate();
    }
  };

  if (typeof document !== "undefined") {
    document.addEventListener("visibilitychange", handleVisibilityChange);
  }

  // Wire do canal binário (u64 LE packed)
  channel.onmessage = (bytes: Uint8Array) => {
    pendingBuffer = bytes;
    scheduleUpdate();
  };

  try {
    await invoke("start_watchdog_stream", { channel });
  } catch {
    // Fallback gracioso se start_watchdog_stream não estiver disponível
  }

  // Assinatura do canal unificado IPC souls_ui_shell
  const unlistenIpc = soulsIpc.onEvent<TelemetrySnapshot>("telemetry/snapshot", (snapshot) => {
    pendingState = {
      vram_mb: snapshot.vram_used_mb,
      ram_mb: snapshot.ram_used_mb,
      cpu_temp: snapshot.cpu_usage_percent,
      gpu_temp: snapshot.gpu_temperature_c,
      thermal_throttle: snapshot.is_kill_switch_active,
    };
    scheduleUpdate();
  });

  // Assinatura do canal de eventos 'hardware-telemetry'
  try {
    unlistenEvent = await listen<Uint8Array | ArrayBuffer | number[] | Partial<TelemetryState>>("hardware-telemetry", (event) => {
      const p = event.payload;
      if (p instanceof Uint8Array || p instanceof ArrayBuffer || Array.isArray(p)) {
        pendingBuffer = p as Uint8Array | ArrayBuffer;
      } else if (p && typeof p === "object" && "buffer" in (p as Record<string, unknown>)) {
        const withBuf = p as { buffer: ArrayBuffer };
        pendingBuffer = withBuf.buffer;
      } else if (p && typeof p === "object") {
        pendingState = p as Partial<TelemetryState>;
      }
      scheduleUpdate();
    });
  } catch {
    // Fallback gracioso em ambiente web standalone
  }

  // Cleanup: cancela agendamento + desinscreve listeners
  return () => {
    cancelled = true;
    if (typeof document !== "undefined") {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    }
    unlistenIpc();
    unlistenEvent?.();
  };
}
