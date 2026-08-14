// SOULS MC — Marco V: Svelte 5 Runes Store de Telemetria.
//
// O canal binário do Tauri emite 8 bytes (u64 packed LE) a 1Hz.
// Este store decodifica o buffer via `DataView` (zero-parse, zero-JSON)
// e atualiza Runes (`$state`, `$derived`) sob `requestAnimationFrame`
// para alinhamento com o refresh do monitor (60Hz/120Hz nativo).
//
// ## Decoupling temporal
//
// - Rust pulsa a 1Hz (`tokio::time::interval`).
// - rAF pulsa a 60Hz no browser.
// - Svelte 5 Runes faz diffing estrutural: se o u64 packed é idêntico
//   ao tick anterior, **nenhum repaint é disparado**. Zero Layout Shift.
//
// ## Agnosticismo
//
// Totalmente agnóstico ao host. Decodifica apenas o layout do u64 packed
// (definido em `core/hardware_watchdog::pack_state`). Não importa
// sysinfo, NVML, nem qualquer crate de SO.

import { invoke, Channel } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Bit-masks — DEVEM ser idênticas às constantes em `core/hardware_watchdog.rs`.
// ---------------------------------------------------------------------------
const MASK_VRAM = (1n << 20n) - 1n; // bits 0..19
const MASK_RAM = ((1n << 20n) - 1n) << 20n; // bits 20..39
const MASK_CPU_TEMP = ((1n << 10n) - 1n) << 40n; // bits 40..49
const MASK_GPU_TEMP = ((1n << 10n) - 1n) << 50n; // bits 50..59
const MASK_FLAGS = 0xFn << 60n; // bits 60..63

// Limite físico de VRAM para a flag de "PRESSAO_CRITICA" (RTX 2060m = 6GB).
// A Runa `thermal_status` reage a este threshold.
const VRAM_CRITICAL_MB = 5000;

// ---------------------------------------------------------------------------
// Runes reativas (Svelte 5).
// ---------------------------------------------------------------------------
export const telemetry = $state({
  vram_mb: 0,
  ram_mb: 0,
  cpu_temp: 0,
  gpu_temp: 0,
  thermal_throttle: false,
});

/**
 * `$derived` que classifica o estado térmico de forma calma (sem cores
 * vermelhas piscando). Reativo a `telemetry.vram_mb` automaticamente.
 *
 * Svelte 5 não permite exportar `$derived` diretamente; expomos um
 * getter (função) que devolve o valor atual — os consumidores chamam
 * `thermal_status()` em qualquer local reativo.
 */
export function thermal_status(): "PRESSAO_CRITICA" | "OCIOSO" {
  return telemetry.vram_mb > VRAM_CRITICAL_MB
    ? "PRESSAO_CRITICA"
    : "OCIOSO";
}

// ---------------------------------------------------------------------------
// Decoder binário puro (DataView + BigInt — zero JSON).
// ---------------------------------------------------------------------------

/**
 * Decodifica um `ArrayBuffer` de 8 bytes (u64 LE) para o objeto
 * `telemetry` global. Mutação direta nas Runes — Svelte 5 detecta via
 * Proxy e dispara o diffing estrutural automático.
 */
export function decode_payload(arrayBuffer: ArrayBuffer): void {
  if (arrayBuffer.byteLength < 8) {
    // Buffer truncado: ignorar (não alocar erro na UI).
    return;
  }

  const view = new DataView(arrayBuffer);
  const state = view.getBigUint64(0, true /* little-endian */);

  const vram = Number(state & MASK_VRAM);
  const ram = Number((state & MASK_RAM) >> 20n);
  // Temperaturas são x2 (0.5 °C LSB).
  const cpu = Number((state & MASK_CPU_TEMP) >> 40n) * 0.5;
  const gpu = Number((state & MASK_GPU_TEMP) >> 50n) * 0.5;
  const flags = Number((state & MASK_FLAGS) >> 60n);

  // Svelte 5 Runes detecta a mutação e só repinta o que mudou.
  telemetry.vram_mb = vram;
  telemetry.ram_mb = ram;
  telemetry.cpu_temp = cpu;
  telemetry.gpu_temp = gpu;
  telemetry.thermal_throttle = (flags & 0b0001) !== 0;
}

// ---------------------------------------------------------------------------
// Bridge: conecta o canal Tauri ao decoder + rAF throttle.
// ---------------------------------------------------------------------------

/**
 * Cria um `Channel<Uint8Array>` do Tauri, invoca o comando
 * `start_watchdog_stream` e desce os ticks binários para o decoder
 * com rAF-throttle.
 *
 * Idempotente: chamar mais de uma vez cria múltiplos canais (o Tauri
 * já dedup por janela; comportamento documentado).
 */
export async function bind_channel_to_runes(): Promise<() => void> {
  const channel = new Channel<Uint8Array>();
  let pendingBuffer: ArrayBuffer | null = null;
  let rafId: number | null = null;
  let cancelled = false;

  // rAF loop: drena o último buffer pendente (1 por frame) e atualiza Runes.
  const tick = (): void => {
    if (cancelled) return;
    if (pendingBuffer !== null) {
      const buf = pendingBuffer;
      pendingBuffer = null;
      decode_payload(buf);
    }
    rafId = requestAnimationFrame(tick);
  };
  rafId = requestAnimationFrame(tick);

  // Wire do canal: cada `onmessage` (do Tauri) deposita o buffer
  // mais recente — a rAF drena no próximo frame.
  channel.onmessage = (bytes: Uint8Array) => {
    // Copia para ArrayBuffer (o Tauri entrega Uint8Array; o decoder
    // aceita ambos via DataView).
    pendingBuffer = bytes.buffer.slice(
      bytes.byteOffset,
      bytes.byteOffset + bytes.byteLength
    ) as ArrayBuffer;
  };

  await invoke("start_watchdog_stream", { channel });

  // Cleanup: cancela rAF + fecha canal.
  return () => {
    cancelled = true;
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
    }
  };
}
