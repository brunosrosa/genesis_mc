<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { bind_channel_to_runes } from "$lib/stores/telemetry.svelte.ts";
  import { listen_for_blast_radius } from "$lib/stores/blast.svelte.ts";
  import { governanceStore } from "$lib/stores/governance.svelte.ts";

  // Segmento 1: Chassi Estrutural
  import HorizonTopbar, { type CockpitView } from "$lib/components/HorizonTopbar.svelte";
  import GovernorRail from "$lib/components/GovernorRail.svelte";
  import CockpitFooter from "$lib/components/CockpitFooter.svelte";

  // Segmento 2: HUD & Portais Rápidos
  import SpotlightZen from "$lib/components/SpotlightZen.svelte";
  import TerminalDrawer from "$lib/components/TerminalDrawer.svelte";

  // Segmento 3: Monitoramento de Baixo Nível
  import TelemetryHUD from "$lib/components/TelemetryHUD.svelte";

  // Segmento 4: Workspaces Cognitivos
  import ActiveCanvas from "$lib/components/ActiveCanvas.svelte";

  let currentView = $state<CockpitView>("chat");
  let isSpotlightOpen = $state(false);
  let isTerminalOpen = $state(false);

  let cleanupTelemetry: (() => void) | null = null;
  let cleanupBlast: (() => void) | null = null;
  let unlistenSpotlight: (() => void) | null = null;

  function handleKeyDown(e: KeyboardEvent) {
    // Alt + Space (ou Shift + CapsLock) para alternar o SpotlightZen
    if (e.altKey && (e.code === "Space" || e.key === " ")) {
      e.preventDefault();
      isSpotlightOpen = !isSpotlightOpen;
      return;
    }

    // Tecla ` (ou Ctrl + ') para alternar a Gaveta do Terminal
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
        // Modo Wry bare-metal sem Tauri runtime
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
  <title>SOULS MC // SODA MISSION CONTROL V6 (BARE-METAL DESKTOP HUD)</title>
  <meta name="description" content="SOULS Mission Control — SODA V6 Cockpit com Composição Nativa DWM e Svelte 5" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link
    rel="stylesheet"
    href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:ital,wght@0,300;0,400;0,500;0,700;1,400&family=Space+Grotesk:wght@400;500;600;700&display=swap"
  />
</svelte:head>

<!-- Canvas Raiz 100% Transparente sobre o Desktop/Wallpaper do Windows 11 -->
<div class="fixed inset-0 w-screen h-screen overflow-hidden bg-transparent select-none font-sans pointer-events-none antialiased">
  <!-- Leve Vinheta Translúcida de Fundo para Contraste Óptico no Wallpaper -->
  <div class="absolute inset-0 bg-gradient-to-b from-black/20 via-transparent to-black/35 pointer-events-none"></div>

  <!-- Kill Switch Banner de Segurança Atômica -->
  {#if governanceStore.isKillSwitchActive}
    <div class="fixed top-3 left-1/2 -translate-x-1/2 px-4 py-2 bg-red-950/90 border border-red-500 text-red-200 font-mono text-xs z-50 pointer-events-auto rounded-xl macos-glass flex items-center gap-3 shadow-2xl">
      <span class="w-2 h-2 rounded-full bg-red-500 animate-ping"></span>
      <span class="font-bold">[KILL-SWITCH ATIVADO]:</span>
      <span>Sub-processos e Workers Tokio pausados via SIGKILL atômico.</span>
      <button 
        type="button"
        onclick={() => governanceStore.resetKillSwitch()}
        class="ml-2 px-2.5 py-0.5 bg-red-900 hover:bg-red-800 border border-red-400 text-[10px] text-white font-bold rounded cursor-pointer"
      >
        REARMAR SISTEMA
      </button>
    </div>
  {/if}

  <!-- =========================================================================
       SEGMENTO 1: O CHASSI ESTRUTURAL (A MOLDURA DE CONTROLE)
       ========================================================================= -->

  <!-- 1.1 HorizonTopbar (Linha de Topo Ultra-Fina Flutuante) -->
  <div class="fixed top-2.5 left-3 right-3 z-40 pointer-events-auto">
    <HorizonTopbar
      {currentView}
      onViewChange={(v) => { currentView = v; }}
      onToggleSpotlight={() => { isSpotlightOpen = !isSpotlightOpen; }}
    />
  </div>

  <!-- 1.2 GovernorRail (Barra Lateral Esquerda de Controle e Foco) -->
  <div class="fixed left-3 top-15 bottom-12 w-16 z-30 pointer-events-auto">
    <GovernorRail
      {currentView}
      onViewChange={(v) => { currentView = v; }}
      onOpenSpotlight={() => { isSpotlightOpen = true; }}
      pendingInboxCount={1}
    />
  </div>

  <!-- 1.3 CockpitFooter (Barra de Rodapé com Roteador de Modelos e Status) -->
  <div class="fixed bottom-2 left-3 right-3 z-40 pointer-events-auto">
    <CockpitFooter
      {isTerminalOpen}
      onToggleTerminal={() => { isTerminalOpen = !isTerminalOpen; }}
    />
  </div>

  <!-- =========================================================================
       SEGMENTO 3: MONITORAMENTO DE BAIXO NÍVEL (PAINÉIS DE HARDWARE)
       ========================================================================= -->

  <!-- 3.1 TelemetryHUD (Mini-Widget Flutuante no Topo Direito - RTX 2060m VRAM, tok/s, temp) -->
  <aside class="fixed top-15 right-3 w-64 z-30 pointer-events-auto">
    <TelemetryHUD
      onOpenDashboard={() => { currentView = "telemetry"; }}
    />
  </aside>

  <!-- =========================================================================
       SEGMENTO 4: AS VISÕES DE TRABALHO ATIVO (WORKSPACES COGNITIVOS)
       ========================================================================= -->

  <!-- 4.1 ActiveCanvas (Diálogo Socrático, Bancada JIT, Memória, Kanban, Inbox, Governança) -->
  <div class="fixed left-21 right-70 top-15 bottom-12 z-20 pointer-events-auto flex flex-col overflow-hidden">
    <ActiveCanvas
      {currentView}
      onViewChange={(v) => { currentView = v; }}
    />
  </div>

  <!-- =========================================================================
       SEGMENTO 2: HUD E PORTAIS RÁPIDOS (SISTEMAS EFÊMEROS)
       ========================================================================= -->

  <!-- 2.1 TerminalDrawer (Gaveta do Console Deslizante) -->
  <div class="fixed bottom-10 left-3 right-3 z-50 pointer-events-auto">
    <TerminalDrawer
      isOpen={isTerminalOpen}
      onClose={() => { isTerminalOpen = false; }}
    />
  </div>

  <!-- 2.2 SpotlightZen (Portal de Entrada Rápido - Shift+CapsLock / Alt+Space) -->
  <div class="pointer-events-auto">
    <SpotlightZen
      isOpen={isSpotlightOpen}
      onClose={() => { isSpotlightOpen = false; }}
      onSelectView={(v) => { currentView = v; }}
    />
  </div>
</div>
