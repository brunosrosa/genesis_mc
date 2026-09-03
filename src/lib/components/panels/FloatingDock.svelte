<script lang="ts">
  import { windowManager } from "$lib/stores/windowManager.svelte.ts";

  let inputValue = $state("");
  let isFocused = $state(false);

  function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!inputValue.trim()) return;
    windowManager.bringToFront("agent_task");
    inputValue = "";
  }
</script>

<!-- Dock Flutuante Central Inferior -->
<div class="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 pointer-events-auto select-none">
  <form
    onsubmit={handleSubmit}
    class="flex items-center gap-3 px-4 py-2 macos-glass rounded-full border border-white/[0.15] shadow-2xl backdrop-blur-2xl transition-all duration-200 {isFocused ? 'ring-2 ring-[#007AFF]/50 border-[#007AFF]' : ''}"
  >
    <!-- Tag de Atalho / Sessão -->
    <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-white/10 text-[10.5px] font-mono text-neutral-300">
      <span class="text-neutral-400">05</span>
      <span class="w-1 h-1 rounded-full bg-white/30"></span>
      <span class="text-cyan-300">T454</span>
    </div>

    <!-- Input de Comando / Query Rápida -->
    <div class="flex items-center gap-2 w-96">
      <span class="text-neutral-400 text-xs">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
      </span>
      <input
        type="text"
        bind:value={inputValue}
        onfocus={() => { isFocused = true; }}
        onblur={() => { isFocused = false; }}
        placeholder="Continuar com 'Search for the latest news about open source AI'..."
        class="w-full bg-transparent border-none outline-none text-xs text-neutral-100 placeholder-neutral-400 font-sans"
      />
    </div>

    <!-- Status / Indicador de Agentes -->
    <div class="flex items-center gap-2 pl-2 border-l border-white/10">
      <button
        type="button"
        class="w-6 h-6 rounded-full bg-emerald-500/20 border border-emerald-500/40 flex items-center justify-center text-emerald-400 text-[11px] font-mono font-medium hover:scale-105 transition-transform"
        title="Agentes Prontos"
        onclick={() => windowManager.toggleWindow("agent_task")}
      >
        0
      </button>
    </div>
  </form>
</div>
