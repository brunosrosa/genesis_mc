<script lang="ts">
  // SOULS MC — Camada 4: Spotlight Zen Conversacional (Alt+Space)
  //
  // Barra AMOLED ultra-translúcida com autofocus e roteamento de intenções JIT.
  // Conformidade: ADR-005, ADR-014 (Fricção Produtiva), ADR-041.

  import { tick } from "svelte";
  import type { CockpitView } from "./HorizonTopbar.svelte";
  import { governanceStore } from "$lib/stores/governance.svelte.ts";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    onSelectView: (view: CockpitView) => void;
  }

  let { isOpen, onClose, onSelectView }: Props = $props();

  let inputQuery = $state("");
  let inputRef: HTMLInputElement | null = $state(null);

  $effect(() => {
    if (isOpen) {
      void tick().then(() => {
        inputRef?.focus();
      });
    } else {
      inputQuery = "";
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      dispatchIntent();
    }
  }

  function dispatchIntent() {
    const trimmed = inputQuery.trim().toLowerCase();
    if (!trimmed) return;

    governanceStore.recordUsage(60, 0.00002);

    if (trimmed.startsWith("/chat") || trimmed.includes("chat") || trimmed.includes("diálogo")) {
      onSelectView("chat");
      onClose();
    } else if (trimmed.startsWith("/bancada") || trimmed.includes("bancada") || trimmed.includes("sandbox")) {
      onSelectView("bancada");
      onClose();
    } else if (trimmed.startsWith("@memoria") || trimmed.startsWith("/memoria") || trimmed.includes("grafo")) {
      onSelectView("memory");
      onClose();
    } else if (trimmed.startsWith("/tarefas") || trimmed.includes("kanban") || trimmed.includes("tasks")) {
      onSelectView("tasks");
      onClose();
    } else if (trimmed.startsWith("/settings") || trimmed.includes("config") || trimmed.includes("governança")) {
      onSelectView("settings");
      onClose();
    } else if (trimmed.startsWith("/inbox") || trimmed.includes("pr") || trimmed.includes("blast")) {
      onSelectView("inbox");
      onClose();
    } else if (trimmed.startsWith("/telemetry") || trimmed.includes("vram") || trimmed.includes("cpu")) {
      onSelectView("telemetry");
      onClose();
    } else {
      // Intenção aberta -> navega para o chat socrático
      onSelectView("chat");
      onClose();
    }
  }

  function handleChipClick(view: CockpitView) {
    onSelectView(view);
    onClose();
  }
</script>

{#if isOpen}
  <!-- Backdrop com Fade-in 150ms -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/75 backdrop-blur-md z-50 flex items-start justify-center pt-28 transition-all duration-150 select-none"
    onclick={onClose}
  >
    <!-- Modal Card -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="w-full max-w-2xl bg-surface-mid ghost-border-active p-4 shadow-2xl relative"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="flex items-center gap-3 border-b border-white/10 pb-3">
        <svg class="w-5 h-5 text-cyber-purple shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>

        <input
          bind:this={inputRef}
          bind:value={inputQuery}
          onkeydown={handleKeyDown}
          type="text"
          placeholder="Invoque uma intenção (/chat, /bancada, @memoria, /tarefas, /settings)..."
          class="w-full bg-transparent font-mono text-sm text-text-main placeholder-text-muted outline-none border-none"
        />

        <kbd class="font-mono text-[10px] text-text-muted bg-surface-high px-2 py-1">
          ESC
        </kbd>
      </div>

      <!-- Quick Commands Chips -->
      <div class="flex items-center gap-2 mt-3 pt-1 flex-wrap font-mono text-xs">
        <button
          type="button"
          onclick={dispatchIntent}
          class="px-3 py-1 bg-cyber-purple text-black font-bold text-[11px] hover:bg-white transition-colors flex items-center gap-1.5"
        >
          <span>✨ Processar Intenção AI</span>
        </button>

        <button
          type="button"
          onclick={() => handleChipClick("chat")}
          class="px-2.5 py-1 bg-surface-high hover:bg-cyber-purple/20 hover:text-cyber-purple border border-white/5 text-[11px] flex items-center gap-1.5 transition-colors text-text-main"
        >
          <span class="text-cyber-purple">/chat</span> Diálogo
        </button>

        <button
          type="button"
          onclick={() => handleChipClick("bancada")}
          class="px-2.5 py-1 bg-surface-high hover:bg-cyber-purple/20 hover:text-cyber-purple border border-white/5 text-[11px] flex items-center gap-1.5 transition-colors text-text-main"
        >
          <span class="text-telemetry-cyan">/bancada</span> Sandbox
        </button>

        <button
          type="button"
          onclick={() => handleChipClick("memory")}
          class="px-2.5 py-1 bg-surface-high hover:bg-cyber-purple/20 hover:text-cyber-purple border border-white/5 text-[11px] flex items-center gap-1.5 transition-colors text-text-main"
        >
          <span class="text-cyber-purple">@memoria</span> Grafo
        </button>

        <button
          type="button"
          onclick={() => handleChipClick("tasks")}
          class="px-2.5 py-1 bg-surface-high hover:bg-cyber-purple/20 hover:text-cyber-purple border border-white/5 text-[11px] flex items-center gap-1.5 transition-colors text-text-main"
        >
          <span class="text-amber-400">/tarefas</span> Kanban
        </button>

        <button
          type="button"
          onclick={() => handleChipClick("settings")}
          class="px-2.5 py-1 bg-surface-high hover:bg-cyber-purple/20 hover:text-cyber-purple border border-white/5 text-[11px] flex items-center gap-1.5 transition-colors text-text-main"
        >
          <span class="text-emerald-400">/settings</span> Governança
        </button>
      </div>

      <div class="mt-4 pt-2 border-t border-white/5 flex justify-between items-center text-[10px] font-mono text-text-muted">
        <span>PRESSIONE <kbd class="text-text-main font-bold">ALT + SPACE</kbd> EM QUALQUER LUGAR</span>
        <span>SOULS MC // JIT INTENT ROUTER</span>
      </div>
    </div>
  </div>
{/if}
