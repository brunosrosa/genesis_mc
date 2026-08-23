<script lang="ts">
  // SOULS MC — Camada 5: Cockpit Footer (Rodapé Fixo h-8)
  //
  // Telemetria de silício (VRAM, CPU, GPU), modelos ativos e trigger do terminal drawer.
  // Conformidade: ADR-001, ADR-005, ADR-027.

  import { telemetry } from "$lib/stores/telemetry.svelte.ts";

  interface Props {
    isTerminalOpen: boolean;
    onToggleTerminal: () => void;
  }

  let { isTerminalOpen, onToggleTerminal }: Props = $props();

  const vramGb = $derived((telemetry.vram_mb / 1024).toFixed(1));
</script>

<footer class="h-8 w-full bg-surface-low border-t border-white/10 flex items-center justify-between px-4 font-mono text-[11px] select-none shrink-0 z-40">
  <div class="flex items-center gap-6">
    <div class="flex items-center gap-2">
      <span class="text-cyber-purple">🧠</span>
      <span class="text-text-main font-semibold">GEMINI 2.5 + PHI-4-MINI</span>
    </div>
    <div class="flex items-center gap-2 text-text-muted">
      <span class="w-1.5 h-1.5 bg-emerald-400 rounded-full"></span>
      <span>DB: WAL SYNCING</span>
    </div>
    <div class="flex items-center gap-2 text-text-muted">
      <span>KERNEL:</span>
      <span class="text-telemetry-cyan font-bold">SOULS KERNEL v0.1.2</span>
    </div>
  </div>

  <div class="flex items-center gap-6">
    <div class="flex items-center gap-4 text-text-muted">
      <span>VRAM: <strong class="text-cyber-purple">{vramGb} / 6.0 GB</strong></span>
      <span>CPU: <strong class="text-text-main">{telemetry.cpu_temp.toFixed(0)}°C</strong></span>
      <span>GPU: <strong class="text-emerald-400">{telemetry.gpu_temp.toFixed(0)}°C</strong></span>
    </div>

    <button 
      type="button"
      onclick={onToggleTerminal} 
      class="px-2 py-0.5 {isTerminalOpen ? 'bg-cyber-purple text-black' : 'bg-surface-high hover:bg-cyber-purple/20 hover:text-cyber-purple'} border border-white/10 transition-colors flex items-center gap-1.5 text-text-main font-bold"
      title="Alternar Gaveta do Terminal (Logs Tokio)"
    >
      <svg class="w-3 h-3 text-telemetry-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="4 17 10 11 4 5" />
        <line x1="12" y1="19" x2="20" y2="19" />
      </svg>
      <span>&gt;_ TERMINAL</span>
    </button>
  </div>
</footer>
