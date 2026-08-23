<script lang="ts">
  // SOULS MC — Active Canvas View: Kanban Agêntico BMAD
  //
  // Gestão visual e determinística de tarefas entre o Operador e Sub-Agentes.
  // Conformidade: ADR-005, ADR-010 (SDD), ADR-011 (HITL).

  interface TaskItem {
    id: string;
    title: string;
    description: string;
    column: "backlog" | "running" | "approval" | "done";
    badge?: string;
  }

  let tasks = $state<TaskItem[]>([
    {
      id: "t1",
      title: "Mapear suplementação diária",
      description: "Brain-dump recebido via áudio no Spotlight.",
      column: "backlog"
    },
    {
      id: "t2",
      title: "Configurar alertas FinOps",
      description: "Definir trava dura de $10/mês para OpenRouter.",
      column: "backlog"
    },
    {
      id: "t3",
      title: "Indexando PDFs no LanceDB",
      description: "Chyros Daemon extraindo embeddings em background.",
      column: "running",
      badge: "Chyros Daemon"
    },
    {
      id: "t4",
      title: "PR-108: Refatorar GGUF Scanner",
      description: "Impacto: 2 arquivos | Risco Rastro 1.",
      column: "approval",
      badge: "HITL Trava"
    },
    {
      id: "t5",
      title: "Criar schema da tabela socratic_thoughts",
      description: "Migração no FrankenSQLite souls_state.db.",
      column: "done"
    },
    {
      id: "t6",
      title: "Otimizar atalho Alt+Space no Tauri",
      description: "HotKey global no Windows OS.",
      column: "done"
    }
  ]);

  interface Props {
    onNavigateToInbox?: () => void;
  }

  let { onNavigateToInbox }: Props = $props();

  const backlogTasks = $derived(tasks.filter(t => t.column === "backlog"));
  const runningTasks = $derived(tasks.filter(t => t.column === "running"));
  const approvalTasks = $derived(tasks.filter(t => t.column === "approval"));
  const doneTasks = $derived(tasks.filter(t => t.column === "done"));
</script>

<div class="h-full w-full bg-surface-low border border-white/10 flex flex-col overflow-hidden select-none font-mono text-xs">
  <!-- Header Kanban -->
  <div class="h-9 px-4 bg-surface-mid border-b border-white/5 flex items-center justify-between shrink-0">
    <span class="text-amber-400 font-bold flex items-center gap-2">
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect width="18" height="18" x="3" y="3" rx="2" />
        <path d="M8 7v7M12 7v4M16 7v9" />
      </svg>
      KANBAN AGÊNTICO // EXECUÇÃO & TRIAGEM DE METAS BMAD
    </span>
    <span class="text-text-muted text-[10px]">BMAD Task Engine · Consistência Eventual</span>
  </div>

  <!-- 4 Columns Grid -->
  <div class="flex-1 p-4 grid grid-cols-1 md:grid-cols-4 gap-3 overflow-hidden">
    <!-- Coluna 1: Backlog -->
    <div class="bg-surface-mid p-3 border border-white/5 flex flex-col gap-2 overflow-y-auto">
      <div class="text-text-muted font-bold text-[11px] pb-2 border-b border-white/5 flex justify-between items-center">
        <span>TRIAGEM / BACKLOG</span>
        <span class="text-cyber-purple">{backlogTasks.length}</span>
      </div>
      {#each backlogTasks as task (task.id)}
        <div class="p-2.5 bg-surface-low border border-white/10 space-y-1 cursor-pointer hover:border-cyber-purple/50 transition-colors">
          <div class="text-text-main font-bold text-xs">{task.title}</div>
          <div class="text-[10px] text-text-muted">{task.description}</div>
        </div>
      {/each}
    </div>

    <!-- Coluna 2: Em Execução IA -->
    <div class="bg-surface-mid p-3 border border-white/5 flex flex-col gap-2 overflow-y-auto">
      <div class="text-telemetry-cyan font-bold text-[11px] pb-2 border-b border-white/5 flex justify-between items-center">
        <span>EM EXECUÇÃO (IA)</span>
        <span class="text-telemetry-cyan">{runningTasks.length}</span>
      </div>
      {#each runningTasks as task (task.id)}
        <div class="p-2.5 bg-surface-low border border-telemetry-cyan/40 space-y-1">
          <div class="flex items-center gap-1.5 text-telemetry-cyan font-bold text-xs">
            <svg class="w-3.5 h-3.5 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12a9 9 0 1 1-6.219-8.56" />
            </svg>
            <span>{task.title}</span>
          </div>
          <div class="text-[10px] text-text-muted">{task.description}</div>
          {#if task.badge}
            <span class="inline-block mt-1 text-[9px] bg-telemetry-cyan/10 text-telemetry-cyan px-1.5 py-0.5 border border-telemetry-cyan/20">
              {task.badge}
            </span>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Coluna 3: Aprovação (HITL) -->
    <div class="bg-surface-mid p-3 border border-white/5 flex flex-col gap-2 overflow-y-auto">
      <div class="text-cyber-purple font-bold text-[11px] pb-2 border-b border-white/5 flex justify-between items-center">
        <span>APROVAÇÃO (INBOX)</span>
        <span class="text-cyber-purple">{approvalTasks.length}</span>
      </div>
      {#each approvalTasks as task (task.id)}
        <div class="p-2.5 bg-surface-low border border-cyber-purple/50 space-y-2">
          <div class="text-cyber-purple font-bold text-xs">{task.title}</div>
          <div class="text-[10px] text-text-muted">{task.description}</div>
          <button 
            type="button"
            onclick={onNavigateToInbox}
            class="w-full py-1 bg-cyber-purple text-black font-bold text-[10px] hover:bg-white transition-colors"
          >
            REVISAR NA INBOX
          </button>
        </div>
      {/each}
    </div>

    <!-- Coluna 4: Concluído -->
    <div class="bg-surface-mid p-3 border border-white/5 flex flex-col gap-2 overflow-y-auto opacity-75">
      <div class="text-emerald-400 font-bold text-[11px] pb-2 border-b border-white/5 flex justify-between items-center">
        <span>CONCLUÍDO</span>
        <span class="text-emerald-400">{doneTasks.length}</span>
      </div>
      {#each doneTasks as task (task.id)}
        <div class="p-2.5 bg-surface-low border border-white/5 space-y-1 text-text-muted line-through">
          <div class="text-xs">{task.title}</div>
          <div class="text-[10px] opacity-60">{task.description}</div>
        </div>
      {/each}
    </div>
  </div>
</div>
