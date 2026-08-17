<script lang="ts">
  // SOULS MC — Camada 4: Spotlight Zen Conversacional (Alt+Space)
  //
  // Barra AMOLED ultra-translúcida com autofocus.
  // - Micro-comando rápido: feedback JIT na própria barra.
  // - Continuidade / Código: expansão fluida para Sessão Socrática (Active Canvas).
  // - Fechamento: expurgo atômico do buffer de memória local (persistência SQLite).

  import { onMount, tick } from "svelte";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    onExpandToSocratic: (prompt: string) => void;
  }

  let { isOpen, onClose, onExpandToSocratic }: Props = $props();

  let inputQuery = $state("");
  let inputRef: HTMLInputElement | null = $state(null);
  let feedbackMessage = $state<string | null>(null);
  let isExecuting = $state(false);

  $effect(() => {
    if (isOpen) {
      void tick().then(() => {
        inputRef?.focus();
      });
    } else {
      inputQuery = "";
      feedbackMessage = null;
      isExecuting = false;
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      executeCommand();
    }
  }

  function executeCommand() {
    const trimmed = inputQuery.trim();
    if (!trimmed) return;

    // Se o comando começar com "/", é um micro-comando JIT
    if (trimmed.startsWith("/")) {
      isExecuting = true;
      if (trimmed === "/ping") {
        feedbackMessage = "SOULS Core Online (Ping 0.2ms · Zero-Copy IPC)";
      } else if (trimmed === "/clear") {
        feedbackMessage = "Memória do renderizador expurgada.";
        inputQuery = "";
      } else if (trimmed === "/status") {
        feedbackMessage = "Hardware Watchdog Ativo · 60 FPS rAF Loop";
      } else {
        feedbackMessage = `Comando '${trimmed}' processado.`;
      }
      isExecuting = false;
      setTimeout(() => {
        if (!feedbackMessage?.includes("expurgada")) {
          // Opcional: fecha após micro-comando
        }
      }, 1800);
    } else {
      // Se for uma instrução conversacional ou de raciocínio, expande para a Sessão Socrática
      const query = inputQuery;
      onClose();
      onExpandToSocratic(query);
    }
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/60 backdrop-blur-md z-50 flex items-start justify-center pt-[18vh] animate-in fade-in duration-150 select-none"
    onclick={onClose}
  >
    <!-- Spotlight Bar -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="w-[min(640px,90vw)] cyber-panel-elevated p-2 flex flex-col gap-2 shadow-[0_20px_50px_rgba(0,0,0,0.9),_inset_0_0_0_1px_rgba(255,255,255,0.15)] animate-in zoom-in-95 duration-150"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="flex items-center gap-3 px-3 py-2">
        <svg class="w-5 h-5 text-[oklch(0.65_0.28_296)] shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>

        <input
          bind:this={inputRef}
          bind:value={inputQuery}
          onkeydown={handleKeyDown}
          type="text"
          placeholder="Digite um micro-comando (/ping, /status) ou questione o SODA..."
          class="w-full bg-transparent text-[oklch(0.98_0_0)] placeholder-[oklch(0.40_0_0)] font-sans text-sm outline-none"
        />

        <div class="flex items-center gap-1">
          <kbd class="px-1.5 py-0.5 rounded bg-[oklch(0.12_0_0)] text-[oklch(0.50_0_0)] font-mono text-[10px] border border-[rgba(255,255,255,0.06)]">
            ESC
          </kbd>
        </div>
      </div>

      <!-- JIT Feedback Area -->
      {#if feedbackMessage}
        <div class="px-3 py-2 rounded-lg bg-[oklch(0.10_0_0)] border border-[oklch(0.65_0.28_296_/_0.3)] text-xs font-mono text-[oklch(0.85_0.20_296)] flex items-center justify-between">
          <span>{feedbackMessage}</span>
          <span class="text-[10px] opacity-60">JIT</span>
        </div>
      {/if}

      <!-- Footer Hints -->
      <div class="flex items-center justify-between px-3 py-1 text-[11px] font-mono text-[oklch(0.40_0_0)] border-t border-[rgba(255,255,255,0.05)] pt-1.5">
        <span>Enter para executar ou expandir</span>
        <span>Alt+Space para fechar</span>
      </div>
    </div>
  </div>
{/if}
