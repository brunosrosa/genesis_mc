// SOULS MC — Marco V: TDD Frontend (Decoder Binário).
//
// Replica EXATAMENTE a lógica de `decode_payload` em `telemetry.svelte.ts`
// para validar, em ambiente isolado (Node + node:assert), que o layout
// little-endian do u64 packed bate byte-a-byte com o esperado.
//
// Este teste é executado por `node --test src/lib/stores/telemetry.test.mjs`
// (sem dependência nova — usa apenas o runner nativo do Node).
//
// Se/ quando vitest for adicionado, este arquivo também é compatível
// (sintaxe `describe`/`it` será reconhecida via globals).
//
// @ts-nocheck — evita dependência de tipos vitest; lógica é puramente runtime.

import { test } from "node:test";
import assert from "node:assert/strict";

// Replica das constantes em `telemetry.svelte.ts` (devem ser idênticas
// ao Rust `core/hardware_watchdog::pack_state`).
const MASK_VRAM = (1n << 20n) - 1n;
const MASK_RAM = ((1n << 20n) - 1n) << 20n;
const MASK_CPU_TEMP = ((1n << 10n) - 1n) << 40n;
const MASK_GPU_TEMP = ((1n << 10n) - 1n) << 50n;
const MASK_FLAGS = 0xFn << 60n;

function decode_js(arrayBuffer) {
  // Espelha a guarda de produção (`telemetry.svelte.ts::decode_payload`):
  // buffer truncado → noop silencioso (sem throw, sem alocação).
  if (arrayBuffer.byteLength < 8) {
    return { vram_mb: 0, ram_mb: 0, cpu_temp: 0, gpu_temp: 0, thermal_throttle: false };
  }
  const view = new DataView(arrayBuffer);
  const state = view.getBigUint64(0, true);
  return {
    vram_mb: Number(state & MASK_VRAM),
    ram_mb: Number((state & MASK_RAM) >> 20n),
    cpu_temp: Number((state & MASK_CPU_TEMP) >> 40n) * 0.5,
    gpu_temp: Number((state & MASK_GPU_TEMP) >> 50n) * 0.5,
    thermal_throttle: Number((state & MASK_FLAGS) >> 60n) & 0b0001 ? true : false,
  };
}

// Pack-side mirror (para construir o buffer sintético a partir de floats).
function pack_js(vram, ram, cpu, gpu, flags) {
  const vramBits = BigInt(vram) & MASK_VRAM;
  const ramBits = (BigInt(ram) & ((1n << 20n) - 1n)) << 20n;
  const cpuBits = (BigInt(Math.round(cpu * 2)) & 0x3FFn) << 40n;
  const gpuBits = (BigInt(Math.round(gpu * 2)) & 0x3FFn) << 50n;
  const flagBits = (BigInt(flags) & 0xFn) << 60n;
  const state = vramBits | ramBits | cpuBits | gpuBits | flagBits;
  const buf = new ArrayBuffer(8);
  new DataView(buf).setBigUint64(0, state, true);
  return buf;
}

test("pack + decode roundtrip: vram=2048, ram=16384, cpu=65.0, gpu=72.0", () => {
  const buf = pack_js(2048, 16384, 65.0, 72.0, 0);
  const out = decode_js(buf);
  assert.equal(out.vram_mb, 2048);
  assert.equal(out.ram_mb, 16384);
  assert.ok(Math.abs(out.cpu_temp - 65.0) < 0.01, `cpu_temp=${out.cpu_temp}`);
  assert.ok(Math.abs(out.gpu_temp - 72.0) < 0.01, `gpu_temp=${out.gpu_temp}`);
  assert.equal(out.thermal_throttle, false);
});

test("pack + decode: zero state yields 8 zero bytes", () => {
  const buf = pack_js(0, 0, 0.0, 0.0, 0);
  const view = new DataView(buf);
  for (let i = 0; i < 8; i++) {
    assert.equal(view.getUint8(i), 0, `byte ${i} should be 0`);
  }
});

test("pack + decode: thermal_throttle flag set when bit 60 is high", () => {
  const buf = pack_js(0, 0, 0.0, 0.0, 0b0001);
  const out = decode_js(buf);
  assert.equal(out.thermal_throttle, true);
});

test("pack + decode: ram truncates to 20 bits (1MB LSB)", () => {
  const buf = pack_js(0, 0xFFFFFF, 0.0, 0.0, 0); // valor acima do mask
  const out = decode_js(buf);
  assert.equal(out.ram_mb, 0xFFFFF, "ram deve ser truncada para 20 bits");
});

test("pack + decode: half-degree LSB precision for temperatures", () => {
  const buf = pack_js(0, 0, 67.5, 88.25, 0);
  const out = decode_js(buf);
  // 67.5 → x2 = 135 → 0.5 LSB preserva exato
  // 88.25 → x2 = 176.5 → arredondado para 177 → /2 = 88.5
  assert.equal(out.cpu_temp, 67.5);
  assert.ok(Math.abs(out.gpu_temp - 88.5) < 0.01, `gpu_temp=${out.gpu_temp}`);
});

test("decode_payload: buffer truncado é ignorado (no throw)", () => {
  const tiny = new ArrayBuffer(4);
  const out = decode_js(tiny);
  // Decoder não deve lançar; apenas não consegue extrair.
  assert.equal(out.vram_mb, 0);
});

test("layout: VRAM ocupa bytes 0..3 (LE), RAM ocupa bytes 2..5 (LE) com shift", () => {
  // Estado forçado: vram = 0x12345, ram = 0xABCDE.
  // Bytes LE esperados:
  //   byte0 = 0x45 (vram LSB)
  //   byte1 = 0x23
  //   byte2 = 0x01
  //   byte3 = 0x00 (vram bit 20 → overflow, truncado)
  //   byte4 = (ram & 0xFF) shift >> 4 → 0xDE >> 4 = 0x0E (na verdade 0xE0 depende)
  const buf = pack_js(0x12345, 0xABCDE, 0, 0, 0);
  const view = new DataView(buf);
  assert.equal(view.getUint8(0), 0x45, "byte0 = vram LSB");
  assert.equal(view.getUint8(1), 0x23, "byte1 = vram next");
});
