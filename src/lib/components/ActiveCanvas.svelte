<script lang="ts">
  import type { CockpitView } from "./HorizonTopbar.svelte";
  import MacWindowFrame from "$lib/components/ui/MacWindowFrame.svelte";
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

  const viewConfig = $derived.by(() => {
    switch (currentView) {
      case "chat":
        return {
          title: "Diálogo Socrático // ThinkingEngine & Branches",
          badge: "GEMINI 2.5 + PHI-4-MINI",
          badgeColor: "text-purple-400 bg-purple-950/60 border-purple-500/30",
        };
      case "bancada":
        return {
          title: "Bancada JIT // Code Sandbox & Micro-VM",
          badge: "SANDBOX ISOLADO",
          badgeColor: "text-cyan-400 bg-cyan-950/60 border-cyan-500/30",
        };
      case "memory":
        return {
          title: "Grafo de Memória // LadybugDB + LanceDB + SQLite",
          badge: "RAG TEMPORAL FRQAD",
          badgeColor: "text-emerald-400 bg-emerald-950/60 border-emerald-500/30",
        };
      case "tasks":
        return {
          title: "Quadro de Tarefas // Enxame de Subagentes em Paralelo",
          badge: "CHYROS DAEMON",
          badgeColor: "text-amber-400 bg-amber-950/60 border-amber-500/30",
        };
      case "inbox":
        return {
          title: "Agent Inbox // Portão de Decisão Humana (HITL)",
          badge: "BLAST RADIUS ATIVO",
          badgeColor: "text-rose-400 bg-rose-950/60 border-rose-500/30",
        };
      case "settings":
        return {
          title: "Hub de Governança // FinOps & Circuit Breakers",
          badge: "PARETO BANDIT",
          badgeColor: "text-blue-400 bg-blue-950/60 border-blue-500/30",
        };
      case "telemetry":
        return {
          title: "Hardware Telemetry // Silicon Watchdog & RTX 2060m",
          badge: "6.0 GB VRAM LIMIT",
          badgeColor: "text-cyan-400 bg-cyan-950/60 border-cyan-500/30",
        };
      default:
        return {
          title: "Workspace Cognitivo",
          badge: "SOULS MC",
          badgeColor: "text-neutral-400 bg-neutral-900 border-white/10",
        };
    }
  });
</script>

<div class="flex-1 w-full h-full flex overflow-hidden p-2 select-none" aria-label="Active Canvas">
  <MacWindowFrame
    title={viewConfig.title}
    isFloating={false}
    class="w-full h-full shadow-2xl"
  >
    {#snippet headerExtra()}
      <span class="text-[9.5px] font-mono px-2 py-0.5 rounded-full border flex items-center gap-1.5 {viewConfig.badgeColor}">
        <span class="w-1.5 h-1.5 rounded-full bg-current animate-pulse"></span>
        {viewConfig.badge}
      </span>
    {/snippet}

    <div class="flex-1 overflow-auto flex flex-col">
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
    </div>
  </MacWindowFrame>
</div>
