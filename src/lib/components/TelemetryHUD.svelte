<script lang="ts">
  // SOULS MC — Camada 3: Telemetry HUD (Mini-Widget de Hardware Flutuante)
  //
  // Mini-card translúcido de canto de tela (macOS Frosted Glass + Cyberpunk):
  // 1. VRAM (RTX 2060m — Teto rígido de 6.0 GB / Alerta de 5000 MB)
  // 2. Velocidade de Inferência (Tokens por Segundo)
  // 3. Termometria de Silício (CPU e GPU Temp)
  import { telemetry, thermal_status } from "$lib/stores/telemetry.svelte.ts";

  interface Props {
    onOpenDashboard?: () => void;
    class?: string;
  }

  let { onOpenDashboard, class: customClass = "" }: Props = $props();

  const isCritical = $derived(thermal_status() === "PRESSAO_CRITICA");
  const vramPercent = $derived(Math.min(100, Math.round((telemetry.vram_mb / 6000) * 100)));
  const vramGb = $derived((telemetry.vram_mb / 1024).toFixed(1));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  onclick={onOpenDashboard}
  class="macos-glass p-3 flex flex-col gap-2 select-none cursor-pointer hover:border-cyan-400/40 transition-all duration-200 group {customClass}"
  title="Clique para abrir o Painel Completo de Telemetria de Hardware"
>
  <!-- Header: LED de Status e Título -->
  <div class="flex items-center justify-between gap-3 text-[11px] font-sans">
    <div class="flex items-center gap-2">
      <span
        class="w-2 h-2 rounded-full {isCritical ? 'bg-red-500 shadow-[0_0_8px_#ef4444] animate-ping' : 'bg-emerald-400 shadow-[0_0_8px_#34d399]'}"
      ></span>
      <span class="font-semibold text-white tracking-wide text-xs">SILICON HUD</span>
    </div>
    <span class="font-mono text-[9px] text-neutral-400 group-hover:text-cyan-400 transition-colors uppercase">
      RTX 2060m
    </span>
  </div>

  <!-- Vetor 1: VRAM (RTX 2060m - Teto de 6.0 GB) -->
  <div class="space-y-1">
    <div class="flex items-center justify-between text-[10.5px] font-mono">
      <span class="text-neutral-400 uppercase text-[9.5px]">VRAM Usage</span>
      <span class="font-semibold {isCritical ? 'text-red-400' : 'text-cyan-300'}">
        {vramGb} <span class="text-neutral-500 font-normal">/ 6.0 GB</span>
      </span>
    </div>
    <!-- Barra de VRAM com threshold visual -->
    <div class="w-full h-1.5 rounded-full bg-white/10 overflow-hidden">
      <div
        class="h-full rounded-full transition-all duration-300 {isCritical ? 'bg-red-500 shadow-[0_0_6px_#ef4444]' : 'bg-gradient-to-r from-cyan-500 to-[#007AFF]'}"
        style="width: {vramPercent}%"
      ></div>
    </div>
  </div>

  <!-- Vetores 2 e 3: Inferência (tok/s) e Térmico (CPU/GPU) -->
  <div class="grid grid-cols-2 gap-2 pt-1 border-t border-white/[0.08] text-[10px] font-mono">
    <!-- Throughput de Tokens -->
    <div class="flex flex-col">
      <span class="text-[9px] text-neutral-400 uppercase">Throughput</span>
      <span class="text-white font-semibold flex items-center gap-1">
        <span class="text-emerald-400">⚡</span> 42.8 <span class="text-[8.5px] text-neutral-400">tok/s</span>
      </span>
    </div>

    <!-- Temperatura CPU e GPU -->
    <div class="flex flex-col text-right">
      <span class="text-[9px] text-neutral-400 uppercase">Thermals</span>
      <span class="text-neutral-200 font-semibold">
        C: <span class="text-white">{telemetry.cpu_temp.toFixed(0)}°</span> | G: <span class="text-white">{telemetry.gpu_temp.toFixed(0)}°</span>
      </span>
    </div>
  </div>
</div>
