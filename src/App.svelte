<script lang="ts">
  // SOULS MC — SODA Canvas v0.1: AppShell (Arquitetura Geométrica de 5 Camadas)
  //
  // Camada 0: Substrate (Tauri Window Shell)
  // Camada 1: Governor Rail (Sidebar w-16)
  // Camada 2: Telemetry HUD (Topbar)
  // Camada 3: Active Canvas (Telemetry / Socratic / Inbox)
  // Camada 4: Ephemeral Layer (Spotlight Zen)
  // Camada 5: Terminal Drawer (libghostty-vt Stdio Reader)

  import { onMount } from "svelte";
  import GovernorRail, { type ActiveCanvasView } from "$lib/components/GovernorRail.svelte";
  import TelemetryHUD from "$lib/components/TelemetryHUD.svelte";
  import TelemetryDashboard from "$lib/components/TelemetryDashboard.svelte";
  import SocraticExplorer from "$lib/components/SocraticExplorer.svelte";
  import AgentInbox from "$lib/components/AgentInbox.svelte";
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

  function handleKeyDown(e: KeyboardEvent) {
    // Atalho global Alt+Space
    if (e.altKey && e.code === "Space") {
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
    })();

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      cleanupTelemetry?.();
      cleanupBlast?.();
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
    href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Space+Grotesk:wght@400;500;600;700&display=swap"
  />
</svelte:head>

<!-- Camada 0: Substrate Shell -->
<div class="relative w-screen h-screen overflow-hidden flex bg-black text-[oklch(0.985_0_0)] font-sans select-none">
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
    <main class="flex-1 flex overflow-hidden relative">
      {#if currentView === "telemetry"}
        <TelemetryDashboard />
      {:else if currentView === "socratic"}
        <SocraticExplorer
          initialPrompt={socraticPrompt}
          onCloseSession={() => { socraticPrompt = null; }}
        />
      {:else if currentView === "inbox"}
        <div class="flex-1 p-8 overflow-y-auto">
          <AgentInbox />
        </div>
      {/if}
    </main>
  </div>

  <!-- Camada 4: Ephemeral Layer (Spotlight Zen) -->
  <SpotlightZen
    isOpen={isSpotlightOpen}
    onClose={() => { isSpotlightOpen = false; }}
    onExpandToSocratic={handleExpandToSocratic}
  />

  <!-- Camada 5: Bottom Drawer (Terminal Logs) -->
  <TerminalDrawer
    isOpen={isTerminalOpen}
    onClose={() => { isTerminalOpen = false; }}
  />
</div>
