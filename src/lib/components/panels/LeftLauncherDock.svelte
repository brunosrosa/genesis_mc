<script lang="ts">
  import { windowManager } from "$lib/stores/windowManager.svelte.ts";

  const dockIcons = [
    {
      id: "settings",
      label: "Settings & Kernel",
      icon: "sliders",
      action: () => windowManager.toggleWindow("settings"),
    },
    {
      id: "agent_task",
      label: "Agent Task Timeline",
      icon: "activity",
      action: () => windowManager.toggleWindow("agent_task"),
    },
    {
      id: "music",
      label: "Audio / Music",
      icon: "music",
      action: () => windowManager.toggleWindow("music"),
    },
    {
      id: "terminal",
      label: "Bare-Metal Terminal",
      icon: "terminal",
      action: () => windowManager.toggleWindow("terminal"),
    },
  ];
</script>

<!-- Dock Vertical Esquerdo Flutuante -->
<aside class="fixed left-4 top-1/2 -translate-y-1/2 z-50 pointer-events-auto select-none">
  <div class="flex flex-col items-center gap-3 p-2 macos-glass rounded-2xl border border-white/[0.12] shadow-2xl backdrop-blur-2xl">
    <!-- Logo / Brand Indicator -->
    <button
      type="button"
      class="w-9 h-9 rounded-xl bg-gradient-to-tr from-[#007AFF] to-[#00E5FF] flex items-center justify-center text-white shadow-lg shadow-[#007AFF]/30 hover:scale-105 transition-transform"
      title="SOULS MC // Reset Layout"
      onclick={() => windowManager.resetLayout()}
    >
      <span class="font-bold text-xs font-mono">S</span>
    </button>

    <div class="w-5 h-[1px] bg-white/10 my-0.5"></div>

    <!-- Lista de Ações do Dock -->
    {#each dockIcons as item}
      <button
        type="button"
        class="w-8 h-8 rounded-xl flex items-center justify-center transition-all duration-150 relative group {windowManager.windows[item.id as any]?.isOpen ? 'bg-white/15 text-white shadow-sm' : 'text-neutral-400 hover:text-white hover:bg-white/10'}"
        onclick={item.action}
        title={item.label}
      >
        {#if item.icon === "sliders"}
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="4" y1="21" x2="4" y2="14"/><line x1="4" y1="10" x2="4" y2="3"/><line x1="12" y1="21" x2="12" y2="12"/><line x1="12" y1="8" x2="12" y2="3"/><line x1="20" y1="21" x2="20" y2="16"/><line x1="20" y1="12" x2="20" y2="3"/><line x1="1" y1="14" x2="7" y2="14"/><line x1="9" y1="8" x2="15" y2="8"/><line x1="17" y1="16" x2="23" y2="16"/></svg>
        {:else if item.icon === "activity"}
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>
        {:else if item.icon === "music"}
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>
        {:else if item.icon === "terminal"}
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>
        {/if}

        <!-- Ponto indicador de janela ativa -->
        {#if windowManager.windows[item.id as any]?.isOpen}
          <span class="absolute -left-1 w-1 h-2 rounded-r-full bg-[#007AFF]"></span>
        {/if}

        <!-- Tooltip flutuante -->
        <span class="absolute left-11 px-2.5 py-1 rounded-lg macos-glass text-[11px] text-white font-sans whitespace-nowrap opacity-0 pointer-events-none group-hover:opacity-100 transition-opacity z-50">
          {item.label}
        </span>
      </button>
    {/each}
  </div>
</aside>
