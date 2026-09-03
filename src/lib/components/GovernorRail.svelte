<script lang="ts">
  // SOULS MC — Camada 1: Governor Rail (Sidebar Esquerda Fixa w-16 / 4rem)
  //
  // Menu planar ultra-rápido com Focus Rack, Bio-Persona e Kill-Switch físico.
  // Zero Layout Shift: Largura matemática fixa de 4rem.
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
  class="w-16 h-full macos-glass flex flex-col justify-between z-30 shrink-0 overflow-hidden select-none shadow-2xl border border-white/[0.12]"
  aria-label="Governor Rail"
>
  <!-- Top: Bio-Persona Profile & Focus Rack -->
  <div class="flex flex-col p-2 gap-2.5 w-full items-center">
    
    <!-- Bio-Persona Card -->
    <button
      type="button"
      onclick={() => onViewChange("settings")}
      class="w-12 h-12 bg-surface-mid border border-white/5 flex flex-col items-center justify-center cursor-pointer hover:border-cyber-purple/40 transition-colors"
      title="Perfil do Operador Soberano (2e/TDAH) — Bruno"
    >
      <div class="w-7 h-7 rounded-full bg-cyber-purple/20 border border-cyber-purple text-cyber-purple font-mono font-bold text-xs flex items-center justify-center shrink-0">
        B
      </div>
      <span class="font-mono text-[8px] text-telemetry-cyan uppercase tracking-tighter truncate max-w-[44px]">SOV</span>
    </button>

    <!-- New Intent Action -->
    <button 
      type="button"
      onclick={onOpenSpotlight} 
      class="w-12 h-10 bg-cyber-purple/10 hover:bg-cyber-purple/20 border border-cyber-purple/30 text-cyber-purple flex flex-col items-center justify-center gap-0.5 transition-colors shrink-0"
      title="Invoque uma intenção (/chat, /bancada, @memoria, /tarefas)..."
    >
      <svg class="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 5v14M5 12h14" />
      </svg>
      <span class="font-mono text-[8px] font-bold tracking-wider">INTENT</span>
    </button>

    <div class="h-[1px] w-10 bg-white/10 my-0.5"></div>

    <!-- Focus Slots Dynamic Loop -->
    {#each workspaceStore.focusSlots as slot (slot.id)}
      <button 
        type="button"
        onclick={() => onViewChange(slot.viewId as CockpitView)} 
        class="w-12 h-10 {currentView === slot.viewId ? 'bg-surface-high border-l-2 border-cyber-purple text-cyber-purple' : 'bg-surface-mid text-text-muted hover:text-text-main border-l-2 border-transparent hover:border-white/20'} flex flex-col items-center justify-center gap-0.5 cursor-pointer transition-colors"
        title="{slot.title} — {slot.subtitle}"
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
        <span class="font-mono text-[8px] font-semibold truncate max-w-[44px]">{slot.title}</span>
      </button>
    {/each}

    <!-- Empty Slot Marker -->
    <div class="w-12 h-8 border border-dashed border-white/5 flex items-center justify-center text-text-muted/40" title="Slot #4 [Livre]">
      <svg class="w-3.5 h-3.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect width="18" height="18" x="3" y="3" rx="2" />
      </svg>
    </div>
  </div>

  <!-- Bottom: Agent Inbox, Settings & Compact Safety Kill-Switch -->
  <div class="p-2 flex flex-col gap-2 w-full items-center border-t border-white/10 bg-surface-low">
    <!-- Agent Inbox Link -->
    <button 
      type="button"
      onclick={() => onViewChange("inbox")} 
      class="w-12 h-10 {currentView === 'inbox' ? 'bg-surface-high border-cyber-purple/40 text-cyber-purple' : 'bg-surface-mid hover:bg-surface-high text-text-muted hover:text-text-main'} border border-white/5 flex flex-col items-center justify-center relative transition-colors"
      title="Agent Inbox (Decisão HITL) — {pendingInboxCount} Propostas"
    >
      <div class="relative shrink-0">
        <svg class="w-4 h-4 text-cyber-purple" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
          <path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
        </svg>
        {#if pendingInboxCount > 0}
          <span class="absolute -top-1.5 -right-2 w-3.5 h-3.5 bg-cyber-purple text-black font-mono font-bold text-[8px] flex items-center justify-center rounded-full">
            {pendingInboxCount}
          </span>
        {/if}
      </div>
      <span class="font-mono text-[8px] font-bold text-cyber-purple">INBOX</span>
    </button>

    <!-- Settings Link -->
    <button 
      type="button"
      onclick={() => onViewChange("settings")} 
      class="w-12 h-10 {currentView === 'settings' ? 'bg-surface-high border-emerald-400/40 text-emerald-400' : 'bg-surface-mid hover:bg-surface-high text-text-muted hover:text-text-main'} border border-white/5 flex flex-col items-center justify-center gap-0.5 transition-colors"
      title="Central de Governança SODA & MCPs"
    >
      <svg class="w-4 h-4 text-emerald-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
      <span class="font-mono text-[8px] text-text-muted">CONFIG</span>
    </button>

    <!-- Compact Safety Kill-Switch -->
    <button 
      type="button"
      onclick={() => governanceStore.triggerKillSwitch()} 
      class="w-12 h-9 bg-alert-crimson/30 hover:bg-alert-crimson border border-alert-crimson/60 text-red-200 flex flex-col items-center justify-center gap-0.5 transition-all group/kill" 
      title="Parada de Emergência Atômica (SIGKILL)"
    >
      <svg class="w-4 h-4 text-red-400 shrink-0 group-hover/kill:animate-pulse" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18.36 6.64a9 9 0 1 1-12.73 0M12 2v10" />
      </svg>
      <span class="font-mono text-[8px] font-bold text-red-300 uppercase tracking-tighter">
        KILL
      </span>
    </button>
  </div>
</aside>
