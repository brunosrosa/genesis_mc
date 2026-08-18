<script lang="ts">
  // SOULS MC — Camada 3: Active Canvas (Área Central de Trabalho)
  //
  // Renderizador planar passivo (Zero-VDOM) com chaveamento JIT:
  // - "telemetry": TelemetryDashboard (Reator de Silício e Hardware)
  // - "socratic": SocraticExplorer (Grafo Cognitivo Socrático)
  // - "inbox": AgentInbox (Blast Radius & Decisão HITL)

  import type { ActiveCanvasView } from "./GovernorRail.svelte";
  import TelemetryDashboard from "./TelemetryDashboard.svelte";
  import SocraticExplorer from "./SocraticExplorer.svelte";
  import AgentInbox from "./AgentInbox.svelte";

  interface Props {
    currentView: ActiveCanvasView;
    socraticPrompt?: string | null;
    onCloseSocratic?: () => void;
  }

  let { currentView, socraticPrompt = null, onCloseSocratic }: Props = $props();
</script>

<main class="flex-1 flex overflow-hidden relative bg-[oklch(0%_0_0)]" aria-label="Active Canvas">
  {#if currentView === "telemetry"}
    <TelemetryDashboard />
  {:else if currentView === "socratic"}
    <SocraticExplorer
      initialPrompt={socraticPrompt}
      onCloseSession={onCloseSocratic}
    />
  {:else if currentView === "inbox"}
    <div class="flex-1 p-8 overflow-y-auto">
      <AgentInbox />
    </div>
  {/if}
</main>
