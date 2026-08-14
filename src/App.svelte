// SOULS MC — Marco V: App Shell (passive).
//
// Inicializa o canal de telemetria (bind_channel_to_runes) e o listener
// de blast radius (listen_for_blast_radius) em onMount. Renderiza
// AgentInbox passivamente (visível só quando há pendingBlast).

<script lang="ts">
  import { onMount } from "svelte";
  import AgentInbox from "$lib/components/AgentInbox.svelte";
  import { bind_channel_to_runes, telemetry } from "$lib/stores/telemetry.svelte";
  import { listen_for_blast_radius } from "$lib/stores/blast.svelte";

  let cleanupTelemetry: (() => void) | null = null;
  let cleanupBlast: (() => void) | null = null;

  onMount(() => {
    void (async () => {
      cleanupTelemetry = await bind_channel_to_runes();
      cleanupBlast = await listen_for_blast_radius();
    })();

    return () => {
      cleanupTelemetry?.();
      cleanupBlast?.();
    };
  });
</script>

<svelte:head>
  <title>SOULS · MC</title>
  <meta name="description" content="SOULS Mission Control — passive overlay" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link
    rel="stylesheet"
    href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&family=Space+Grotesk:wght@400;500;600&display=swap"
  />
</svelte:head>

<main class="shell">
  <div class="brand">
    <h1 class="brand-title">SOULS</h1>
    <p class="brand-subtitle">Mission Control · Passive Overlay</p>
  </div>

  <dl class="metrics" aria-label="Hardware Telemetry">
    <div class="metric">
      <dt>VRAM</dt>
      <dd>{telemetry.vram_mb} <span class="unit">MB</span></dd>
    </div>
    <div class="metric">
      <dt>RAM</dt>
      <dd>{telemetry.ram_mb} <span class="unit">MB</span></dd>
    </div>
    <div class="metric">
      <dt>CPU</dt>
      <dd>{telemetry.cpu_temp.toFixed(1)} <span class="unit">°C</span></dd>
    </div>
    <div class="metric">
      <dt>GPU</dt>
      <dd>{telemetry.gpu_temp.toFixed(1)} <span class="unit">°C</span></dd>
    </div>
  </dl>
</main>

<AgentInbox />

<style>
  .shell {
    min-height: 100dvh;
    padding: 2rem 2.5rem;
    color: oklch(0.95 0 0);
    font-family: var(--font-sans, "Space Grotesk", system-ui, sans-serif);
  }
  .brand {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 2rem;
  }
  .brand-title {
    font-size: 1.5rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    margin: 0;
  }
  .brand-subtitle {
    font-family: var(--font-mono, "JetBrains Mono", ui-monospace, monospace);
    font-size: 0.75rem;
    opacity: 0.6;
    margin: 0;
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 0.75rem;
    max-width: 720px;
    margin: 0;
  }
  .metric {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem 1rem;
    border: 1px solid oklch(0.22 0 0);
    border-radius: 0.5rem;
    background: oklch(0.10 0 0 / 60%);
    contain: layout paint;
  }
  .metric dt {
    font-family: var(--font-mono, "JetBrains Mono", ui-monospace, monospace);
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    opacity: 0.55;
  }
  .metric dd {
    font-family: var(--font-mono, "JetBrains Mono", ui-monospace, monospace);
    font-size: 1.1rem;
    font-weight: 500;
    margin: 0;
  }
  .unit {
    font-size: 0.7rem;
    opacity: 0.55;
  }
</style>
