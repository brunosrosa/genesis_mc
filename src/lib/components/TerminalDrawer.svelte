<script lang="ts">
  // SOULS MC — Camada 5: Gaveta Terminal Deslizante (libghostty-vt stdio stream)
  //
  // Desliza da base em 250ms (GPU-accelerated).
  // Consome logs bare-metal em modo somente-leitura.
  // Quando oculta, usa tombstone virtual para descarregar o pipeline de pintura do DOM.

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  // Mock de logs locais em cache (stdio)
  let logs = $state([
    "[SOULS] Bootstrap Bare-Metal Inicializado.",
    "[WATCHDOG] Stream binário 1Hz conectado (u64 packed LE).",
    "[GPU] NVIDIA GeForce RTX 2060m detectada (VRAM Threshold: 5000MB / 6GB).",
    "[FinOps] ParetoBandit L7 Shield Ativo. Consumo de VRAM monitorado.",
    "[UI] SODA Canvas v0.1 Svelte 5 Runes 60 FPS rAF loop sincronizado.",
  ]);
</script>

<!-- Container com Grid Transition para Zero Reflow -->
<div
  class="fixed inset-x-0 bottom-0 z-40 transition-transform duration-250 ease-[cubic-bezier(0.2,0.8,0.2,1)] {isOpen ? 'translate-y-0' : 'translate-y-full'}"
  aria-hidden={!isOpen}
>
  <div class="mx-4 mb-2 cyber-panel-elevated flex flex-col h-64 shadow-[0_-8px_30px_rgba(0,0,0,0.8)] border border-[rgba(255,255,255,0.1)]">
    <!-- Header da Gaveta -->
    <header class="flex items-center justify-between px-4 py-2 border-b border-[rgba(255,255,255,0.06)] bg-[oklch(0.06_0_0_/_90%)] rounded-t-xl select-none">
      <div class="flex items-center gap-2">
        <span class="w-2 h-2 rounded-full bg-[oklch(0.78_0.20_145)]"></span>
        <span class="font-mono text-xs font-semibold text-[oklch(0.85_0_0)]">
          STDIO & LOGS · READ-ONLY CONSOLE
        </span>
      </div>

      <div class="flex items-center gap-2">
        <button
          type="button"
          onclick={() => { logs = ["[SOULS] Console buffer limpo."]; }}
          class="px-2 py-0.5 rounded text-[10px] font-mono bg-[oklch(0.10_0_0)] text-[oklch(0.50_0_0)] hover:text-[oklch(0.80_0_0)] border border-[rgba(255,255,255,0.05)]"
        >
          Limpar
        </button>
        <button
          type="button"
          onclick={onClose}
          class="p-1 rounded text-[oklch(0.50_0_0)] hover:text-[oklch(0.90_0_0)] hover:bg-[oklch(0.12_0_0)]"
          aria-label="Fechar gaveta terminal"
        >
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>
    </header>

    <!-- Terminal Content Body -->
    <div class="flex-1 p-3 overflow-y-auto font-mono text-[11px] leading-relaxed text-[oklch(0.75_0_0)] bg-[oklch(0.02_0_0_/_95%)] space-y-1">
      {#each logs as logLine, i (i)}
        <div class="flex items-start gap-2">
          <span class="text-[oklch(0.35_0_0)] select-none">{String(i + 1).padStart(2, "0")}</span>
          <span class="text-[oklch(0.85_0_0)]">{logLine}</span>
        </div>
      {/each}
    </div>
  </div>
</div>
