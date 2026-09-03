<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { bind_channel_to_runes } from "$lib/stores/telemetry.svelte.ts";
  import { listen_for_blast_radius } from "$lib/stores/blast.svelte.ts";
  import { governanceStore } from "$lib/stores/governance.svelte.ts";
  import { windowManager } from "$lib/stores/windowManager.svelte.ts";

  // Componentes do Desktop OS Overlay (macOS + Cyberpunk Sleek)
  import SettingsKernelPanel from "$lib/components/panels/SettingsKernelPanel.svelte";
  import AgentTaskPanel from "$lib/components/panels/AgentTaskPanel.svelte";
  import MusicWidget from "$lib/components/panels/MusicWidget.svelte";
  import TerminalPanel from "$lib/components/panels/TerminalPanel.svelte";
  import FloatingDock from "$lib/components/panels/FloatingDock.svelte";
  import LeftLauncherDock from "$lib/components/panels/LeftLauncherDock.svelte";
  import SpotlightZen from "$lib/components/SpotlightZen.svelte";

  let isSpotlightOpen = $state(false);
  let cleanupTelemetry: (() => void) | null = null;
  let cleanupBlast: (() => void) | null = null;
  let unlistenSpotlight: (() => void) | null = null;

  function handleKeyDown(e: KeyboardEvent) {
    // Alt + Space para alternar o Spotlight de Comandos Rápidos
    if (e.altKey && (e.code === "Space" || e.key === " ")) {
      e.preventDefault();
      isSpotlightOpen = !isSpotlightOpen;
    }

    // Tecla ` para alternar o Terminal Bare-Metal flutuante
    if (e.key === "`" && !isSpotlightOpen) {
      const activeEl = document.activeElement;
      if (activeEl && (activeEl.tagName === "INPUT" || activeEl.tagName === "TEXTAREA")) return;
      e.preventDefault();
      windowManager.toggleWindow("terminal");
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
  <title>SOULS MC // SODA DESKTOP OVERLAY (APPLE + CYBERPUNK SLEEK)</title>
  <meta name="description" content="SOULS Mission Control — Bare-Metal OS HUD & Spatial AI Canvas" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link
    rel="stylesheet"
    href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:ital,wght@0,300;0,400;0,500;0,700;1,400&family=Space+Grotesk:wght@400;500;600;700&display=swap"
  />
</svelte:head>

<!-- Canvas de Overlay 100% Transparente sobre o Papel de Parede do Windows -->
<div class="fixed inset-0 w-screen h-screen overflow-hidden bg-transparent select-none font-sans pointer-events-none antialiased">
  <!-- Leve Vinheta Translúcida de Fundo para Contraste de Leitura no Wallpaper -->
  <div class="absolute inset-0 bg-gradient-to-b from-black/15 via-transparent to-black/25 pointer-events-none"></div>

  <!-- Kill Switch Banner de Emergência -->
  {#if governanceStore.isKillSwitchActive}
    <div class="fixed top-4 left-1/2 -translate-x-1/2 px-4 py-2 bg-red-950/80 border border-red-500 text-red-200 font-mono text-xs z-[999] pointer-events-auto rounded-xl macos-glass flex items-center gap-3">
      <span class="w-2 h-2 rounded-full bg-red-500 animate-ping"></span>
      <span class="font-bold">[KILL-SWITCH ATIVADO]:</span>
      <span>Sub-processos e Workers Tokio pausados via SIGKILL atômico.</span>
      <button 
        type="button"
        onclick={() => governanceStore.resetKillSwitch()}
        class="ml-2 px-2.5 py-0.5 bg-red-900 hover:bg-red-800 border border-red-400 text-[10px] text-white font-bold rounded"
      >
        REARMAR
      </button>
    </div>
  {/if}

  <!-- 1. Dock Vertical Esquerdo Flutuante -->
  <LeftLauncherDock />

  <!-- 2. Painéis Modulares Flutuantes (Draggable + macOS Traffic Lights) -->
  <div class="pointer-events-auto">
    <!-- Janela de Configurações, Métricas de Kernel e Agendador -->
    <SettingsKernelPanel />

    <!-- Janela de Execução de Tarefas de IA e Timeline do Agente -->
    <AgentTaskPanel />

    <!-- Mini-Widget de Áudio / Música no Topo Direito -->
    <MusicWidget />

    <!-- Painel de Terminal Sandbox (Opcional, ativado via ` ou Dock) -->
    <TerminalPanel />
  </div>

  <!-- 3. Dock Flutuante Inferior de Comandos e Status -->
  <FloatingDock />

  <!-- 4. Camada de Spotlight Zen (Alt + Space) -->
  <div class="pointer-events-auto">
    <SpotlightZen
      isOpen={isSpotlightOpen}
      onClose={() => { isSpotlightOpen = false; }}
      onSelectView={() => {}}
    />
  </div>
</div>
