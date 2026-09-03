<script lang="ts">
  // SOULS MC — Camada 2: Horizon Topbar (Menu Superior Fixo - h-12)
  //
  // Controles de navegação, seletor de workspaces cognitivos, telemetria FinOps e governança.
  // Conformidade: ADR-005, ADR-008, ADR-014, ADR-041.

  import { workspaceStore, WORKSPACES, type CognitiveWorkspace } from "$lib/stores/workspace.svelte.ts";
  import { governanceStore } from "$lib/stores/governance.svelte.ts";

  export type CockpitView = "chat" | "bancada" | "memory" | "tasks" | "settings" | "inbox" | "telemetry";

  interface Props {
    currentView: CockpitView;
    onViewChange: (view: CockpitView) => void;
    onToggleSpotlight: () => void;
  }

  let { currentView, onViewChange, onToggleSpotlight }: Props = $props();

  let isWorkspaceDropdownOpen = $state(false);

  function handleSelectWorkspace(ws: CognitiveWorkspace) {
    workspaceStore.setWorkspace(ws);
    isWorkspaceDropdownOpen = false;
  }

  const isHotl = $derived(governanceStore.mode === "HOTL");
</script>

<header class="h-11 w-full macos-glass flex items-center justify-between px-4 shrink-0 font-mono text-xs select-none shadow-xl border border-white/[0.12]">
  <!-- Left: Identity & Active Workspace Selector -->
  <div class="flex items-center gap-3">
    <!-- Ghost Avatar / Soul Pip -->
    <button 
      type="button"
      class="flex items-center gap-2 group cursor-pointer text-left bg-transparent border-none p-0" 
      title="Soul Agent: Active & Listening"
      onclick={() => onViewChange("chat")}
    >
      <div class="w-3 h-3 rounded-full bg-cyber-purple soul-pip"></div>
      <span class="font-headline font-bold tracking-wider text-text-main group-hover:text-cyber-purple transition-colors">
        SOULS MC
      </span>
    </button>

    <div class="h-4 w-[1px] bg-white/10 mx-1"></div>

    <!-- Workspace Selector Dropdown -->
    <div class="relative">
      <button 
        type="button"
        onclick={() => isWorkspaceDropdownOpen = !isWorkspaceDropdownOpen}
        class="flex items-center gap-2 bg-surface-mid border border-white/10 hover:border-cyber-purple/50 px-2.5 py-1 transition-colors text-text-main font-bold"
      >
        <span>{workspaceStore.activeWorkspace.icon}</span>
        <span class="truncate max-w-[140px]">{workspaceStore.activeWorkspace.title}</span>
        <svg class="w-3 h-3 text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="m7 15 5 5 5-5M7 9l5-5 5 5" />
        </svg>
      </button>

      {#if isWorkspaceDropdownOpen}
        <!-- Backdrop to close dropdown -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div 
          class="fixed inset-0 z-40" 
          onclick={() => isWorkspaceDropdownOpen = false}
        ></div>

        <!-- Dropdown Menu -->
        <div class="absolute top-full left-0 mt-1 w-64 bg-surface-mid border border-white/10 shadow-2xl p-1 z-50 space-y-1">
          <div class="px-2 py-1 text-[9px] text-text-muted uppercase tracking-wider font-bold">
            Selecione o Universo Cognitivo
          </div>
          {#each WORKSPACES as ws (ws.id)}
            <button 
              type="button"
              onclick={() => handleSelectWorkspace(ws)}
              class="w-full text-left px-2 py-1.5 hover:bg-surface-high flex items-center gap-2 text-xs font-mono transition-colors {workspaceStore.activeWorkspace.id === ws.id ? 'text-cyber-purple bg-surface-high/50' : 'text-text-main'}"
            >
              <span class="text-base">{ws.icon}</span>
              <div class="flex flex-col">
                <span class="font-semibold">{ws.title}</span>
                <span class="text-[9px] text-text-muted">{ws.description}</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Center: Focus Canvas Tabs (5 Primary Modes) -->
  <nav class="flex items-center gap-1 bg-surface-mid p-1 border border-white/5" aria-label="Modos do Cockpit">
    <button 
      type="button"
      onclick={() => onViewChange("chat")} 
      class="px-3 py-1 font-mono text-[11px] font-medium transition-colors flex items-center gap-1.5 {currentView === 'chat' ? 'bg-surface-high text-cyber-purple border border-cyber-purple/30' : 'text-text-muted hover:text-text-main'}"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
      </svg>
      <span>DIÁLOGO</span>
    </button>

    <button 
      type="button"
      onclick={() => onViewChange("bancada")} 
      class="px-3 py-1 font-mono text-[11px] font-medium transition-colors flex items-center gap-1.5 {currentView === 'bancada' ? 'bg-surface-high text-telemetry-cyan border border-telemetry-cyan/30' : 'text-text-muted hover:text-text-main'}"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
      </svg>
      <span>BANCADA</span>
    </button>

    <button 
      type="button"
      onclick={() => onViewChange("memory")} 
      class="px-3 py-1 font-mono text-[11px] font-medium transition-colors flex items-center gap-1.5 {currentView === 'memory' ? 'bg-surface-high text-emerald-400 border border-emerald-400/30' : 'text-text-muted hover:text-text-main'}"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1-2.5-2.5Z" />
        <path d="M6 6h10M6 10h10" />
      </svg>
      <span>MEMÓRIA</span>
    </button>

    <button 
      type="button"
      onclick={() => onViewChange("tasks")} 
      class="px-3 py-1 font-mono text-[11px] font-medium transition-colors flex items-center gap-1.5 relative {currentView === 'tasks' ? 'bg-surface-high text-amber-400 border border-amber-400/30' : 'text-text-muted hover:text-text-main'}"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect width="18" height="18" x="3" y="3" rx="2" />
        <path d="M8 7v7M12 7v4M16 7v9" />
      </svg>
      <span>TAREFAS</span>
      <span class="w-1.5 h-1.5 rounded-full bg-amber-400 absolute top-1 right-1"></span>
    </button>

    <button 
      type="button"
      onclick={() => onViewChange("settings")} 
      class="px-3 py-1 font-mono text-[11px] font-medium transition-colors flex items-center gap-1.5 {currentView === 'settings' ? 'bg-surface-high text-emerald-400 border border-emerald-400/30' : 'text-text-muted hover:text-text-main'}"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
      <span>CONFIGURAÇÕES</span>
    </button>
  </nav>

  <!-- Right: Governance, FinOps & Overlay Window Controls -->
  <div class="flex items-center gap-4">
    <!-- FinOps Live Counter -->
    <div class="flex items-center gap-2 bg-surface-mid px-2.5 py-1 border border-white/5 text-[11px]" title="Consumo FinOps Local + Cloud">
      <span class="text-amber-400 font-bold">$</span>
      <span class="text-text-muted">${governanceStore.totalUsd.toFixed(2)}</span>
      <span class="text-white/20">|</span>
      <span class="text-telemetry-cyan font-semibold">{(governanceStore.totalTokens / 1000).toFixed(1)}k tok</span>
    </div>

    <!-- Approval Mode Lock Toggle -->
    <button 
      type="button"
      onclick={() => governanceStore.toggleMode()} 
      class="flex items-center gap-1.5 px-2 py-1 bg-surface-high border text-[11px] transition-colors {isHotl ? 'border-emerald-500/30 text-emerald-400' : 'border-amber-500/40 text-amber-300'}" 
      title="Alternar Modo de Governança (HOTL Auto vs HITL Rígido)"
    >
      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        {#if isHotl}
          <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
          <path d="M7 11V7a5 5 0 0 1 9.9-1" />
        {:else}
          <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
          <path d="M7 11V7a5 5 0 0 1 10 0v4" />
        {/if}
      </svg>
      <span class="font-bold">{isHotl ? 'HOTL (Auto)' : 'HITL (Trava)'}</span>
    </button>

    <!-- Spotlight Trigger Button -->
    <button 
      type="button"
      onclick={onToggleSpotlight} 
      class="p-1 hover:bg-surface-high text-text-muted hover:text-cyber-purple transition-colors border border-white/5" 
      title="Abrir Spotlight Zen (Alt+Space)"
    >
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
    </button>
  </div>
</header>
