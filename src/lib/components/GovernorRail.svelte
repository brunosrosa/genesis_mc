<script lang="ts">
  // SOULS MC — Camada 1: Governor Rail (Sidebar Esquerda Retrátil w-16 / w-60)
  //
  // Menu planar ultra-rápido (50ms - 150ms) com Focus Rack, Bio-Persona e Kill-Switch físico.
  // Conformidade: ADR-001, ADR-005, ADR-014, ADR-027.

  import { workspaceStore } from "$lib/stores/workspace.svelte.ts";
  import { governanceStore } from "$lib/stores/governance.svelte.ts";
  import type { CockpitView } from "./HorizonTopbar.svelte";

  interface Props {
    currentView: CockpitView;
    onViewChange: (view: CockpitView) => void;
    onOpenSpotlight: () => void;
    pendingInboxCount?: number;
  }

  let { currentView, onViewChange, onOpenSpotlight, pendingInboxCount = 1 }: Props = $props();
</script>

<aside 
  id="governor-rail" 
  class="w-16 hover:w-60 bg-surface-low border-r border-white/10 flex flex-col justify-between z-30 transition-all duration-200 group shrink-0 overflow-hidden select-none"
  aria-label="Governor Rail"
>
  <!-- Top: Bio-Persona Profile & Focus Rack -->
  <div class="flex flex-col p-2 gap-3 w-full">
    
    <!-- Bio-Persona Card -->
    <button
      type="button"
      onclick={() => onViewChange("settings")}
      class="w-full p-2 bg-surface-mid border border-white/5 flex items-center gap-2.5 cursor-pointer hover:border-cyber-purple/40 transition-colors text-left"
      title="Perfil do Operador Soberano (2e/TDAH)"
    >
      <div class="w-7 h-7 rounded-full bg-cyber-purple/20 border border-cyber-purple text-cyber-purple font-mono font-bold text-xs flex items-center justify-center shrink-0">
        B
      </div>
      <div class="flex flex-col overflow-hidden opacity-0 group-hover:opacity-100 transition-opacity">
        <span class="font-mono text-xs font-bold text-text-main truncate">Bruno</span>
        <span class="font-mono text-[9px] text-telemetry-cyan truncate">2e / TDAH • Sovereign</span>
      </div>
    </button>

    <!-- New Intent Action -->
    <button 
      type="button"
      onclick={onOpenSpotlight} 
      class="w-full h-9 bg-cyber-purple/10 hover:bg-cyber-purple/20 border border-cyber-purple/30 text-cyber-purple flex items-center justify-start px-2.5 gap-2.5 transition-colors shrink-0"
      title="Invoque uma intenção (/chat, /bancada, @memoria, /tarefas)..."
    >
      <svg class="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 5v14M5 12h14" />
      </svg>
      <span class="font-mono text-xs font-bold tracking-wider opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap">
        + NOVO INTENT
      </span>
    </button>

    <div class="h-[1px] bg-white/10 my-0.5"></div>

    <!-- Focus Rack Label -->
    <div class="px-2 font-mono text-[10px] text-text-muted uppercase tracking-widest flex items-center justify-between opacity-0 group-hover:opacity-100 transition-opacity">
      <span>Focus Rack (5 Max)</span>
      <span class="text-cyber-purple">{workspaceStore.focusSlots.length}/5</span>
    </div>

    <!-- Focus Slots Dynamic Loop -->
    {#each workspaceStore.focusSlots as slot (slot.id)}
      <button 
        type="button"
        onclick={() => onViewChange(slot.viewId as CockpitView)} 
        class="w-full p-2 {currentView === slot.viewId ? 'bg-surface-high border-l-2 border-cyber-purple' : 'bg-surface-mid border-l-2 border-transparent hover:border-white/20'} flex items-center gap-3 cursor-pointer hover:bg-surface-mid transition-colors text-left"
      >
        <span class="material-symbols-outlined {slot.color} shrink-0 text-sm">
          {#if slot.icon === 'chat_bubble'}
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>
          {:else if slot.icon === 'construction'}
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>
          {:else}
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1-2.5-2.5Z"/><path d="M6 6h10M6 10h10"/></svg>
          {/if}
        </span>
        <div class="flex flex-col overflow-hidden opacity-0 group-hover:opacity-100 transition-opacity">
          <span class="font-mono text-xs font-semibold truncate text-text-main">{slot.title}</span>
          <span class="font-mono text-[9px] text-text-muted">{slot.subtitle}</span>
        </div>
      </button>
    {/each}

    <!-- Empty Slot Marker -->
    <div class="w-full p-1.5 border border-dashed border-white/5 flex items-center gap-2.5 text-text-muted/40">
      <svg class="w-3.5 h-3.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect width="18" height="18" x="3" y="3" rx="2" />
      </svg>
      <span class="font-mono text-[9px] opacity-0 group-hover:opacity-100 transition-opacity uppercase">
        Slot #4 [Livre]
      </span>
    </div>
  </div>

  <!-- Bottom: Agent Inbox, Settings & Compact Safety Kill-Switch -->
  <div class="p-2 flex flex-col gap-2 w-full border-t border-white/10 bg-surface-low">
    <!-- Agent Inbox Link -->
    <button 
      type="button"
      onclick={() => onViewChange("inbox")} 
      class="w-full p-2 {currentView === 'inbox' ? 'bg-surface-high border-cyber-purple/40' : 'bg-surface-mid hover:bg-surface-high'} border border-white/5 flex items-center gap-3 transition-colors text-text-main text-left"
      title="Agent Inbox (Decisão HITL)"
    >
      <div class="relative shrink-0">
        <svg class="w-4 h-4 text-cyber-purple" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
          <path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
        </svg>
        {#if pendingInboxCount > 0}
          <span class="absolute -top-1.5 -right-1.5 w-3.5 h-3.5 bg-cyber-purple text-black font-mono font-bold text-[8px] flex items-center justify-center rounded-full">
            {pendingInboxCount}
          </span>
        {/if}
      </div>
      <div class="flex flex-col text-left overflow-hidden opacity-0 group-hover:opacity-100 transition-opacity">
        <span class="font-mono text-xs font-bold text-cyber-purple">Agent Inbox</span>
        <span class="font-mono text-[9px] text-text-muted">{pendingInboxCount} Propostas</span>
      </div>
    </button>

    <!-- Settings Link -->
    <button 
      type="button"
      onclick={() => onViewChange("settings")} 
      class="w-full p-2 {currentView === 'settings' ? 'bg-surface-high border-emerald-400/40' : 'bg-surface-mid hover:bg-surface-high'} border border-white/5 flex items-center gap-3 transition-colors text-text-main text-left"
      title="Central de Governança SODA"
    >
      <svg class="w-4 h-4 text-emerald-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
      <div class="flex flex-col text-left overflow-hidden opacity-0 group-hover:opacity-100 transition-opacity">
        <span class="font-mono text-xs font-bold text-text-main">Configurações</span>
        <span class="font-mono text-[9px] text-text-muted">Governança & MCPs</span>
      </div>
    </button>

    <!-- Compact Safety Kill-Switch -->
    <button 
      type="button"
      onclick={() => governanceStore.triggerKillSwitch()} 
      class="w-full h-8 bg-alert-crimson/30 hover:bg-alert-crimson border border-alert-crimson/60 text-red-200 flex items-center justify-start px-2.5 gap-2.5 transition-all group/kill" 
      title="Parada de Emergência Atômica (SIGKILL)"
    >
      <svg class="w-4 h-4 text-red-400 shrink-0 group-hover/kill:animate-pulse" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18.36 6.64a9 9 0 1 1-12.73 0M12 2v10" />
      </svg>
      <span class="font-mono text-[10px] font-bold text-red-300 uppercase tracking-wider opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap">
        KILL-SWITCH
      </span>
    </button>
  </div>
</aside>
