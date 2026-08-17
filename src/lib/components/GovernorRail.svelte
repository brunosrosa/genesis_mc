<script lang="ts">
  // SOULS MC — Camada 1: Governor Rail (Sidebar Esquerda w-16)
  //
  // Barra de navegação planar ultra-rápida (50ms - 150ms).
  // Chaveia instantaneamente as visões do Active Canvas:
  // - "telemetry": Reator / Telemetry Dashboard
  // - "socratic": Cérebro / Thinking Explorer & Graph
  // - "inbox": Gaveta / Agent Inbox & Blast Radius HITL

  export type ActiveCanvasView = "telemetry" | "socratic" | "inbox";

  interface Props {
    currentView: ActiveCanvasView;
    onViewChange: (view: ActiveCanvasView) => void;
    hasPendingBlast?: boolean;
  }

  let { currentView, onViewChange, hasPendingBlast = false }: Props = $props();
</script>

<aside
  class="w-16 h-full flex flex-col items-center justify-between py-4 bg-[oklch(0.04_0_0_/_80%)] backdrop-blur-xl border-r border-[rgba(255,255,255,0.06)] z-30 select-none"
  aria-label="Governor Rail"
>
  <!-- Top: Logo / Brand Indicator -->
  <div class="flex flex-col items-center gap-3">
    <div
      class="w-10 h-10 rounded-xl flex items-center justify-center bg-[oklch(0.08_0_0)] shadow-[inset_0_0_0_1px_rgba(255,255,255,0.12)] text-[oklch(0.65_0.28_296)] font-bold text-xs tracking-tighter"
      title="SOULS Mission Control"
    >
      SODA
    </div>

    <!-- Navigation Switchers -->
    <nav class="flex flex-col gap-2 mt-4" aria-label="Views">
      <!-- 1. Telemetry Dashboard (Reactor) -->
      <button
        type="button"
        onclick={() => onViewChange("telemetry")}
        class="relative w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-100 ease-[cubic-bezier(0.2,0.8,0.2,1)] {currentView === 'telemetry' ? 'bg-[oklch(0.14_0_0)] text-[oklch(0.75_0.20_200)] shadow-[inset_0_0_0_1px_oklch(0.75_0.20_200_/_0.5),_0_0_12px_oklch(0.75_0.20_200_/_0.25)]' : 'text-[oklch(0.50_0_0)] hover:text-[oklch(0.85_0_0)] hover:bg-[oklch(0.08_0_0)]'}"
        title="Telemetry Dashboard (Reactor)"
        aria-label="Telemetry View"
      >
        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <!-- Reactor / Gauge icon -->
          <circle cx="12" cy="12" r="9" />
          <circle cx="12" cy="12" r="3" />
          <path d="M12 3v3" />
          <path d="M12 18v3" />
          <path d="M3 12h3" />
          <path d="M18 12h3" />
        </svg>
        {#if currentView === 'telemetry'}
          <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 bg-[oklch(0.75_0.20_200)] rounded-r-full"></span>
        {/if}
      </button>

      <!-- 2. Socratic Thinking Explorer (Brain / Graph) -->
      <button
        type="button"
        onclick={() => onViewChange("socratic")}
        class="relative w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-100 ease-[cubic-bezier(0.2,0.8,0.2,1)] {currentView === 'socratic' ? 'bg-[oklch(0.14_0_0)] text-[oklch(0.65_0.28_296)] shadow-[inset_0_0_0_1px_oklch(0.65_0.28_296_/_0.5),_0_0_12px_oklch(0.65_0.28_296_/_0.25)]' : 'text-[oklch(0.50_0_0)] hover:text-[oklch(0.85_0_0)] hover:bg-[oklch(0.08_0_0)]'}"
        title="Socratic Thinking Explorer (Graph)"
        aria-label="Socratic View"
      >
        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <!-- Brain / Cognition Graph icon -->
          <path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 4.44-2.04z" />
          <path d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-4.44-2.04z" />
        </svg>
        {#if currentView === 'socratic'}
          <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 bg-[oklch(0.65_0.28_296)] rounded-r-full"></span>
        {/if}
      </button>

      <!-- 3. Agent Inbox / Blast Radius (Inbox) -->
      <button
        type="button"
        onclick={() => onViewChange("inbox")}
        class="relative w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-100 ease-[cubic-bezier(0.2,0.8,0.2,1)] {currentView === 'inbox' ? 'bg-[oklch(0.14_0_0)] text-[oklch(0.70_0.18_50)] shadow-[inset_0_0_0_1px_oklch(0.70_0.18_50_/_0.5),_0_0_12px_oklch(0.70_0.18_50_/_0.25)]' : 'text-[oklch(0.50_0_0)] hover:text-[oklch(0.85_0_0)] hover:bg-[oklch(0.08_0_0)]'}"
        title="Agent Inbox / Blast Radius (HITL)"
        aria-label="Agent Inbox View"
      >
        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <!-- Inbox / Safe Drawer icon -->
          <polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
          <path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
        </svg>
        {#if hasPendingBlast}
          <span class="absolute top-2 right-2 w-2 h-2 rounded-full bg-[oklch(0.65_0.28_296)] animate-ping"></span>
          <span class="absolute top-2 right-2 w-2 h-2 rounded-full bg-[oklch(0.65_0.28_296)]"></span>
        {/if}
        {#if currentView === 'inbox'}
          <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 bg-[oklch(0.70_0.18_50)] rounded-r-full"></span>
        {/if}
      </button>
    </nav>
  </div>

  <!-- Bottom: Hardware Heartbeat Dot -->
  <div class="flex flex-col items-center gap-2">
    <div
      class="w-2.5 h-2.5 rounded-full bg-[oklch(0.78_0.20_145)] shadow-[0_0_8px_oklch(0.78_0.20_145)]"
      title="Hardware Watchdog Online (1Hz Zero-Copy Stream)"
    ></div>
    <span class="text-[9px] font-mono text-[oklch(0.35_0_0)]">v0.1</span>
  </div>
</aside>
