<script lang="ts">
  // SOULS MC — SODA Canvas v0.1: Cockpit V3 (Arquitetura Geométrica de 5 Camadas)
  //
  // Camada 0: Substrate Shell (Frameless, Void Dark, Zero-VDOM)
  // Camada 1: Governor Rail (Sidebar w-16 / w-60 com Bio-Persona, Focus Rack e Kill-Switch)
  // Camada 2: Horizon Topbar (h-12 com Workspaces, 5 Modos, FinOps e Governança)
  // Camada 3: Adaptive Central Canvas (6 Visões: Diálogo+Bancada JIT, Bancada Full, Grafo, Kanban, Governança, Inbox)
  // Camada 4: Spotlight Zen Conversacional (Alt+Space)
  // Camada 5: Engine Room Terminal Drawer + Cockpit Footer (h-8)
  //
  // Conformidade: ADR-001, ADR-005, ADR-008, ADR-011, ADR-014, ADR-027, ADR-041.

  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import HorizonTopbar, { type CockpitView } from "$lib/components/HorizonTopbar.svelte";
  import GovernorRail from "$lib/components/GovernorRail.svelte";
  import ActiveCanvas from "$lib/components/ActiveCanvas.svelte";
  import SpotlightZen from "$lib/components/SpotlightZen.svelte";
  import TerminalDrawer from "$lib/components/TerminalDrawer.svelte";
  import CockpitFooter from "$lib/components/CockpitFooter.svelte";

  import { bind_channel_to_runes } from "$lib/stores/telemetry.svelte.ts";
  import { listen_for_blast_radius } from "$lib/stores/blast.svelte.ts";
  import { governanceStore } from "$lib/stores/governance.svelte.ts";

  let currentView = $state<CockpitView>("chat");
  let isSpotlightOpen = $state(false);
  let isTerminalOpen = $state(false);

  let cleanupTelemetry: (() => void) | null = null;
  let cleanupBlast: (() => void) | null = null;
  let unlistenSpotlight: (() => void) | null = null;

  function handleKeyDown(e: KeyboardEvent) {
    // Atalho global / local Alt+Space para Spotlight
    if (e.altKey && (e.code === "Space" || e.key === " ")) {
      e.preventDefault();
      isSpotlightOpen = !isSpotlightOpen;
    }
    // Atalho ` ou Ctrl+' para alternar gaveta terminal
    if ((e.key === "`" || (e.ctrlKey && e.key === "'")) && !isSpotlightOpen) {
      const activeEl = document.activeElement;
      if (activeEl && (activeEl.tagName === "INPUT" || activeEl.tagName === "TEXTAREA")) return;
      e.preventDefault();
      isTerminalOpen = !isTerminalOpen;
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeyDown);

    void (async () => {
      cleanupTelemetry = await bind_channel_to_runes();
      cleanupBlast = await listen_for_blast_radius();

      try {
        unlistenSpotlight = await listen("toggle-spotlight", () => {
          isSpotlightOpen = true;
        });
      } catch {
        // Fallback em ambiente dev/mock sem backend ativo
      }
    })();

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      cleanupTelemetry?.();
      cleanupBlast?.();
      unlistenSpotlight?.();
    };
  });
</script>

<svelte:head>
  <title>SOULS MC // SODA MISSION CONTROL (COCKPIT V3)</title>
  <meta name="description" content="SOULS Mission Control — SODA Cockpit V3 Zero-VDOM" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link
    rel="stylesheet"
    href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:ital,wght@0,300;0,400;0,500;0,700;1,400&family=Space+Grotesk:wght@400;500;600;700&display=swap"
  />
</svelte:head>

<!-- Camada 0: Substrate Shell (Void Dark, Cyber Grid) com Gaiola Ortogonal Indestrutível -->
<div class="cockpit-grid relative bg-void text-text-main font-body select-none cyber-grid antialiased">
  
  <!-- Camada 1: Governor Rail (Sidebar Esquerda Fixa 4rem) -->
  <div style="grid-area: rail;" class="h-full w-16 overflow-hidden">
    <GovernorRail
      {currentView}
      onViewChange={(v) => { currentView = v; }}
      onOpenSpotlight={() => { isSpotlightOpen = true; }}
      pendingInboxCount={1}
    />
  </div>

  <!-- Camada 2: Horizon Topbar (h-12 / 3rem) -->
  <div style="grid-area: topbar;" class="h-12 w-full overflow-hidden">
    <HorizonTopbar
      {currentView}
      onViewChange={(v) => { currentView = v; }}
      onToggleSpotlight={() => { isSpotlightOpen = !isSpotlightOpen; }}
    />
  </div>

  <!-- Camada 3: Active Canvas (Contêiner Exclusivo das 6 Visões) -->
  <div style="grid-area: canvas;" class="flex flex-col h-full w-full overflow-hidden relative">
    {#if governanceStore.isKillSwitchActive}
      <div class="m-3 p-3 bg-alert-crimson/40 border border-alert-crimson flex items-center justify-between text-red-200 font-mono text-xs z-30 shrink-0">
        <div class="flex items-center gap-2">
          <span class="font-bold text-red-400">[KILL-SWITCH ATIVADO]:</span>
          <span>Todos os Workers Tokio, SLMs Locais e sub-processos MCP foram parados via SIGKILL atômico.</span>
        </div>
        <button 
          type="button"
          onclick={() => governanceStore.resetKillSwitch()}
          class="px-2.5 py-1 bg-red-950 border border-red-500 hover:bg-red-800 transition-colors text-[10px] text-white font-bold"
        >
          REARMAR SISTEMA
        </button>
      </div>
    {/if}

    <ActiveCanvas
      {currentView}
      onViewChange={(v) => { currentView = v; }}
    />
  </div>

  <!-- Camada 5: Engine Room Terminal Drawer + Cockpit Footer (h-8 / 2rem) -->
  <div style="grid-area: terminal;" class="h-8 w-full overflow-visible relative">
    <TerminalDrawer
      isOpen={isTerminalOpen}
      onClose={() => { isTerminalOpen = false; }}
    />
    <CockpitFooter
      {isTerminalOpen}
      onToggleTerminal={() => { isTerminalOpen = !isTerminalOpen; }}
    />
  </div>

  <!-- Camada 4: Ephemeral Layer (Spotlight Zen) -->
  <SpotlightZen
    isOpen={isSpotlightOpen}
    onClose={() => { isSpotlightOpen = false; }}
    onSelectView={(v) => { currentView = v; }}
  />
</div>
