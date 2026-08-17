<script lang="ts">
  // SOULS MC — Camada 2: Telemetry HUD (Topbar Superior)
  //
  // Exibe o "eletrocardiograma" de hardware do SODA em tempo real:
  // - VRAM (Threshold de 6GB / 5000MB da RTX 2060m)
  // - RAM (Memória do Host)
  // - CPU Temp / GPU Temp
  // - FinOps Tokens & Custo Acumulado

  import { telemetry, thermal_status } from "$lib/stores/telemetry.svelte.ts";

  interface Props {
    onToggleSpotlight?: () => void;
    onToggleTerminal?: () => void;
    isTerminalOpen?: boolean;
  }

  let { onToggleSpotlight, onToggleTerminal, isTerminalOpen = false }: Props = $props();

  const isCritical = $derived(thermal_status() === "PRESSAO_CRITICA");
  const vramPercent = $derived(Math.min(100, Math.round((telemetry.vram_mb / 6000) * 100)));
</script>

<header
  class="h-14 w-full flex items-center justify-between px-6 bg-[oklch(0.04_0_0_/_85%)] backdrop-blur-xl border-b border-[rgba(255,255,255,0.06)] z-20 select-none"
  aria-label="Telemetry HUD"
>
  <!-- Left: Core Status ECG & Title -->
  <div class="flex items-center gap-4">
    <div class="flex items-center gap-2.5">
      <span
        class="w-2 h-2 rounded-full transition-colors duration-150 {isCritical ? 'bg-[oklch(0.70_0.18_50)] shadow-[0_0_10px_oklch(0.70_0.18_50)]' : 'bg-[oklch(0.78_0.20_145)] shadow-[0_0_10px_oklch(0.78_0.20_145)]'}"
      ></span>
      <span class="font-sans font-semibold text-xs tracking-wider text-[oklch(0.95_0_0)] uppercase">
        SOULS COCKPIT
      </span>
      <span class="font-mono text-[10px] px-1.5 py-0.5 rounded bg-[oklch(0.10_0_0)] text-[oklch(0.60_0_0)] border border-[rgba(255,255,255,0.05)]">
        60 FPS rAF
      </span>
    </div>
  </div>

  <!-- Center: Hardware ECG Metrics (VRAM, RAM, CPU, GPU) -->
  <div class="flex items-center gap-6 font-mono text-xs text-[oklch(0.85_0_0)]">
    <!-- VRAM with RTX 2060m 6GB limit -->
    <div class="flex items-center gap-2">
      <span class="text-[10px] text-[oklch(0.45_0_0)] tracking-widest uppercase">VRAM</span>
      <div class="flex items-center gap-1.5">
        <span class="font-medium {isCritical ? 'text-[oklch(0.70_0.18_50)] font-bold' : 'text-[oklch(0.92_0_0)]'}">
          {telemetry.vram_mb}
        </span>
        <span class="text-[10px] text-[oklch(0.45_0_0)]">/ 6000 MB</span>
      </div>
      <!-- Mini VRAM gauge bar -->
      <div class="w-12 h-1.5 rounded-full bg-[oklch(0.12_0_0)] overflow-hidden">
        <div
          class="h-full transition-all duration-150 {isCritical ? 'bg-[oklch(0.70_0.18_50)]' : 'bg-[oklch(0.75_0.20_200)]'}"
          style="width: {vramPercent}%"
        ></div>
      </div>
    </div>

    <!-- RAM -->
    <div class="flex items-center gap-2">
      <span class="text-[10px] text-[oklch(0.45_0_0)] tracking-widest uppercase">RAM</span>
      <span class="text-[oklch(0.92_0_0)] font-medium">{telemetry.ram_mb}</span>
      <span class="text-[10px] text-[oklch(0.45_0_0)]">MB</span>
    </div>

    <!-- CPU Temp -->
    <div class="flex items-center gap-2">
      <span class="text-[10px] text-[oklch(0.45_0_0)] tracking-widest uppercase">CPU</span>
      <span class="text-[oklch(0.92_0_0)] font-medium">{telemetry.cpu_temp.toFixed(1)}</span>
      <span class="text-[10px] text-[oklch(0.45_0_0)]">°C</span>
    </div>

    <!-- GPU Temp -->
    <div class="flex items-center gap-2">
      <span class="text-[10px] text-[oklch(0.45_0_0)] tracking-widest uppercase">GPU</span>
      <span class="text-[oklch(0.92_0_0)] font-medium">{telemetry.gpu_temp.toFixed(1)}</span>
      <span class="text-[10px] text-[oklch(0.45_0_0)]">°C</span>
    </div>
  </div>

  <!-- Right: Quick Actions (Spotlight Alt+Space & Terminal Drawer) -->
  <div class="flex items-center gap-3">
    <!-- Spotlight Quick Trigger -->
    <button
      type="button"
      onclick={onToggleSpotlight}
      class="flex items-center gap-2 px-2.5 py-1 rounded-lg bg-[oklch(0.08_0_0)] hover:bg-[oklch(0.12_0_0)] text-[oklch(0.70_0_0)] hover:text-[oklch(0.95_0_0)] border border-[rgba(255,255,255,0.08)] transition-all duration-100 ease-[cubic-bezier(0.2,0.8,0.2,1)]"
      title="Spotlight Zen (Alt+Space)"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <span class="font-mono text-[11px]">Alt+Space</span>
    </button>

    <!-- Terminal Drawer Toggle -->
    <button
      type="button"
      onclick={onToggleTerminal}
      class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg {isTerminalOpen ? 'bg-[oklch(0.14_0_0)] text-[oklch(0.78_0.20_145)] border-[oklch(0.78_0.20_145_/_0.4)]' : 'bg-[oklch(0.08_0_0)] text-[oklch(0.60_0_0)] hover:text-[oklch(0.90_0_0)]'} border border-[rgba(255,255,255,0.08)] transition-all duration-100 ease-[cubic-bezier(0.2,0.8,0.2,1)]"
      title="Terminal Drawer (Logs & Stdio)"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="4 17 10 11 4 5" />
        <line x1="12" y1="19" x2="20" y2="19" />
      </svg>
      <span class="font-mono text-[11px]">Logs</span>
    </button>
  </div>
</header>
