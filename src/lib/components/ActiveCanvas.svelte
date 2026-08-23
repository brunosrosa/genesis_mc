<script lang="ts">
  // SOULS MC — Camada 3: Active Canvas (Roteador Planar Central)
  //
  // Renderizador passivo Zero-VDOM das 6 visões principais do Cockpit V3.
  // Conformidade: ADR-001, ADR-005, ADR-014.

  import type { CockpitView } from "./HorizonTopbar.svelte";
  import SocraticChatView from "./views/SocraticChatView.svelte";
  import BancadaSandboxView from "./views/BancadaSandboxView.svelte";
  import MemoryGraphView from "./views/MemoryGraphView.svelte";
  import AgentKanbanView from "./views/AgentKanbanView.svelte";
  import GovernanceHubView from "./views/GovernanceHubView.svelte";
  import AgentInboxView from "./views/AgentInboxView.svelte";
  import TelemetryDashboard from "./TelemetryDashboard.svelte";

  interface Props {
    currentView: CockpitView;
    onViewChange: (view: CockpitView) => void;
  }

  let { currentView, onViewChange }: Props = $props();
</script>

<main class="flex-1 flex overflow-hidden relative bg-void/50 p-3 select-none" aria-label="Active Canvas">
  {#if currentView === "chat"}
    <SocraticChatView onNavigateToBancada={() => onViewChange("bancada")} />
  {:else if currentView === "bancada"}
    <BancadaSandboxView />
  {:else if currentView === "memory"}
    <MemoryGraphView />
  {:else if currentView === "tasks"}
    <AgentKanbanView onNavigateToInbox={() => onViewChange("inbox")} />
  {:else if currentView === "settings"}
    <GovernanceHubView />
  {:else if currentView === "inbox"}
    <AgentInboxView />
  {:else if currentView === "telemetry"}
    <TelemetryDashboard />
  {/if}
</main>
