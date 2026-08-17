<script lang="ts">
  // SOULS MC — Active Canvas View: Telemetry Dashboard (Reactor View)
  //
  // Visualização imersiva dos sensores físicos do processador e placa de vídeo (RTX 2060m).
  // Zero VDOM, renderização ultra-passiva.

  import { telemetry, thermal_status } from "$lib/stores/telemetry.svelte.ts";

  const isCritical = $derived(thermal_status() === "PRESSAO_CRITICA");
  const vramPercent = $derived(Math.min(100, (telemetry.vram_mb / 6000) * 100));
  const ramPercent = $derived(Math.min(100, (telemetry.ram_mb / 16384) * 100));
</script>

<div class="flex-1 flex flex-col gap-6 p-8 overflow-y-auto" aria-label="Reactor Telemetry Canvas">
  <!-- Section Title -->
  <div class="flex flex-col gap-1">
    <div class="flex items-center gap-2">
      <span class="w-2.5 h-2.5 rounded-full bg-[oklch(0.75_0.20_200)] shadow-[0_0_10px_oklch(0.75_0.20_200)]"></span>
      <h2 class="font-sans text-xl font-bold tracking-tight text-[oklch(0.98_0_0)]">
        Hardware Reactor Core
      </h2>
    </div>
    <p class="font-mono text-xs text-[oklch(0.50_0_0)]">
      Telemetria física nativa via stream binário 1Hz (u64 LE packed) · Sincronização 60 FPS rAF
    </p>
  </div>

  <!-- Big Metrics Grid -->
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
    <!-- VRAM Card -->
    <div class="cyber-panel p-5 flex flex-col justify-between gap-4 border border-[rgba(255,255,255,0.08)]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-xs uppercase text-[oklch(0.50_0_0)] tracking-wider">VRAM (RTX 2060m)</span>
        <span class="font-mono text-[10px] px-2 py-0.5 rounded bg-[oklch(0.12_0_0)] {isCritical ? 'text-[oklch(0.70_0.18_50)] font-bold' : 'text-[oklch(0.75_0.20_200)]'}">
          {isCritical ? "CRITICAL (6GB)" : "ESTÁVEL"}
        </span>
      </div>
      <div>
        <div class="font-mono text-3xl font-bold tracking-tight text-[oklch(0.98_0_0)]">
          {telemetry.vram_mb} <span class="text-sm font-normal text-[oklch(0.50_0_0)]">MB</span>
        </div>
        <div class="font-mono text-xs text-[oklch(0.45_0_0)] mt-1">
          Capacidade nominal: 6.000 MB
        </div>
      </div>
      <div class="w-full h-2 rounded-full bg-[oklch(0.12_0_0)] overflow-hidden">
        <div
          class="h-full transition-all duration-150 {isCritical ? 'bg-[oklch(0.70_0.18_50)]' : 'bg-[oklch(0.75_0.20_200)]'}"
          style="width: {vramPercent}%"
        ></div>
      </div>
    </div>

    <!-- Host RAM Card -->
    <div class="cyber-panel p-5 flex flex-col justify-between gap-4 border border-[rgba(255,255,255,0.08)]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-xs uppercase text-[oklch(0.50_0_0)] tracking-wider">Memória RAM</span>
        <span class="font-mono text-[10px] px-2 py-0.5 rounded bg-[oklch(0.12_0_0)] text-[oklch(0.78_0.20_145)]">
          HOST
        </span>
      </div>
      <div>
        <div class="font-mono text-3xl font-bold tracking-tight text-[oklch(0.98_0_0)]">
          {telemetry.ram_mb} <span class="text-sm font-normal text-[oklch(0.50_0_0)]">MB</span>
        </div>
        <div class="font-mono text-xs text-[oklch(0.45_0_0)] mt-1">
          Uso de memória física do SO
        </div>
      </div>
      <div class="w-full h-2 rounded-full bg-[oklch(0.12_0_0)] overflow-hidden">
        <div
          class="h-full bg-[oklch(0.78_0.20_145)] transition-all duration-150"
          style="width: {ramPercent}%"
        ></div>
      </div>
    </div>

    <!-- CPU Temperature -->
    <div class="cyber-panel p-5 flex flex-col justify-between gap-4 border border-[rgba(255,255,255,0.08)]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-xs uppercase text-[oklch(0.50_0_0)] tracking-wider">CPU Temp</span>
        <span class="font-mono text-[10px] px-2 py-0.5 rounded bg-[oklch(0.12_0_0)] text-[oklch(0.65_0.28_296)]">
          SENSOR
        </span>
      </div>
      <div>
        <div class="font-mono text-3xl font-bold tracking-tight text-[oklch(0.98_0_0)]">
          {telemetry.cpu_temp.toFixed(1)} <span class="text-sm font-normal text-[oklch(0.50_0_0)]">°C</span>
        </div>
        <div class="font-mono text-xs text-[oklch(0.45_0_0)] mt-1">
          Precisão LSB: 0.5 °C
        </div>
      </div>
      <div class="w-full h-2 rounded-full bg-[oklch(0.12_0_0)] overflow-hidden">
        <div
          class="h-full bg-[oklch(0.65_0.28_296)] transition-all duration-150"
          style="width: {Math.min(100, telemetry.cpu_temp)}%"
        ></div>
      </div>
    </div>

    <!-- GPU Temperature -->
    <div class="cyber-panel p-5 flex flex-col justify-between gap-4 border border-[rgba(255,255,255,0.08)]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-xs uppercase text-[oklch(0.50_0_0)] tracking-wider">GPU Temp</span>
        <span class="font-mono text-[10px] px-2 py-0.5 rounded bg-[oklch(0.12_0_0)] text-[oklch(0.70_0.18_50)]">
          NVML
        </span>
      </div>
      <div>
        <div class="font-mono text-3xl font-bold tracking-tight text-[oklch(0.98_0_0)]">
          {telemetry.gpu_temp.toFixed(1)} <span class="text-sm font-normal text-[oklch(0.50_0_0)]">°C</span>
        </div>
        <div class="font-mono text-xs text-[oklch(0.45_0_0)] mt-1">
          Silício NVIDIA RTX 2060m
        </div>
      </div>
      <div class="w-full h-2 rounded-full bg-[oklch(0.12_0_0)] overflow-hidden">
        <div
          class="h-full bg-[oklch(0.70_0.18_50)] transition-all duration-150"
          style="width: {Math.min(100, telemetry.gpu_temp)}%"
        ></div>
      </div>
    </div>
  </div>

  <!-- FinOps & System Architecture Specs -->
  <div class="cyber-panel p-6 border border-[rgba(255,255,255,0.06)] flex flex-col gap-4">
    <h3 class="font-sans font-semibold text-sm text-[oklch(0.90_0_0)]">
      Políticas de Governança FinOps & Limites de Silício (ADR-001 / ADR-014)
    </h3>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 font-mono text-xs text-[oklch(0.70_0_0)]">
      <div class="p-3 rounded-lg bg-[oklch(0.04_0_0)] border border-[rgba(255,255,255,0.05)]">
        <div class="text-[oklch(0.45_0_0)] text-[10px] uppercase">Limite VRAM</div>
        <div class="font-bold text-[oklch(0.90_0_0)] mt-1">6GB Fixo (RTX 2060m)</div>
        <div class="text-[11px] text-[oklch(0.50_0_0)] mt-1">Pausa SLM em 5000MB</div>
      </div>
      <div class="p-3 rounded-lg bg-[oklch(0.04_0_0)] border border-[rgba(255,255,255,0.05)]">
        <div class="text-[oklch(0.45_0_0)] text-[10px] uppercase">Renderizador</div>
        <div class="font-bold text-[oklch(0.90_0_0)] mt-1">Zero-VDOM Svelte 5</div>
        <div class="text-[11px] text-[oklch(0.50_0_0)] mt-1">Diff estrutural com Runes</div>
      </div>
      <div class="p-3 rounded-lg bg-[oklch(0.04_0_0)] border border-[rgba(255,255,255,0.05)]">
        <div class="text-[oklch(0.45_0_0)] text-[10px] uppercase">IPC Protocol</div>
        <div class="font-bold text-[oklch(0.90_0_0)] mt-1">Zero-Copy ArrayBuffer</div>
        <div class="text-[11px] text-[oklch(0.50_0_0)] mt-1">8 bytes Little-Endian LE</div>
      </div>
    </div>
  </div>
</div>
