<script lang="ts">
  // SOULS MC — SODA Canvas v0.1: AppShell (Arquitetura Geométrica de 5 Camadas)
  //
  // Camada 0: Substrate (Tauri Window Shell / Acrylic Frameless)
  // Camada 1: Governor Rail (Sidebar w-16 / 64px)
  // Camada 2: Telemetry HUD (Topbar ECG 60 FPS rAF)
  // Camada 3: Active Canvas (Telemetry / Socratic / Inbox)
  // Camada 4: Ephemeral Layer (Spotlight Zen Conversacional)
  // Camada 5: Terminal Drawer (libghostty-vt Stdio Reader com GPU Transform)

  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import GovernorRail, { type ActiveCanvasView } from "$lib/components/GovernorRail.svelte";
  import TelemetryHUD from "$lib/components/TelemetryHUD.svelte";
  import ActiveCanvas from "$lib/components/ActiveCanvas.svelte";
  import SpotlightZen from "$lib/components/SpotlightZen.svelte";
  import TerminalDrawer from "$lib/components/TerminalDrawer.svelte";

  import { bind_channel_to_runes } from "$lib/stores/telemetry.svelte.ts";
  import { listen_for_blast_radius, pendingBlast } from "$lib/stores/blast.svelte.ts";

  let currentView = $state<ActiveCanvasView>("telemetry");
  let isSpotlightOpen = $state(false);
  let isTerminalOpen = $state(false);
  let socraticPrompt = $state<string | null>(null);

  let cleanupTelemetry: (() => void) | null = null;
  let cleanupBlast: (() => void) | null = null;
  let unlistenSpotlight: (() => void) | null = null;

  function handleKeyDown(e: KeyboardEvent) {
    // Atalho global / local Alt+Space
    if (e.altKey && (e.code === "Space" || e.key === " ")) {
      e.preventDefault();
      isSpotlightOpen = !isSpotlightOpen;
    }
    // Atalho ` para alternar gaveta terminal
    if (e.key === "`" && !e.ctrlKey && !e.altKey && !isSpotlightOpen) {
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
        // Fallback em ambiente dev sem backend ativo
      }
    })();

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      cleanupTelemetry?.();
      cleanupBlast?.();
      unlistenSpotlight?.();
    };
  });

  function handleExpandToSocratic(prompt: string) {
    socraticPrompt = prompt;
    currentView = "socratic";
  }
</script>

<svelte:head>
  <title>SOULS · SODA Canvas v0.1</title>
  <meta name="description" content="SOULS Mission Control — SODA Canvas v0.1 Zero-VDOM" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link
    rel="stylesheet"
    href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Space+Grotesk:wght@400;500;600;700&family=Space+Mono:wght@400;700&display=swap"
  />
</svelte:head>

<!-- Camada 0: Substrate Shell (Frameless, Acrylic Black, Zero-VDOM) -->
<div class="relative w-screen h-screen overflow-hidden flex bg-[oklch(0%_0_0)] text-[oklch(0.985_0_0)] font-sans select-none rounded-none">
  <!-- Camada 1: Governor Rail -->
  <GovernorRail
    {currentView}
    onViewChange={(v) => { currentView = v; }}
    hasPendingBlast={pendingBlast.report !== null}
  />

  <!-- Main View Column -->
  <div class="flex-1 flex flex-col h-full overflow-hidden">
    <!-- Camada 2: Telemetry HUD -->
    <TelemetryHUD
      onToggleSpotlight={() => { isSpotlightOpen = !isSpotlightOpen; }}
      onToggleTerminal={() => { isTerminalOpen = !isTerminalOpen; }}
      {isTerminalOpen}
    />

    <!-- Camada 3: Active Canvas -->
    <ActiveCanvas
      {currentView}
      {socraticPrompt}
      onCloseSocratic={() => { socraticPrompt = null; }}
    />
  </div>

  <!-- Camada 4: Ephemeral Layer (Spotlight Zen) -->
  <SpotlightZen
    isOpen={isSpotlightOpen}
    onClose={() => { isSpotlightOpen = false; }}
    onExpandToSocratic={handleExpandToSocratic}
  />

  <!-- Camada 5: Bottom Drawer (Terminal Logs com Ocultação Virtual) -->
  <TerminalDrawer
    isOpen={isTerminalOpen}
    onClose={() => { isTerminalOpen = false; }}
  />
</div>
