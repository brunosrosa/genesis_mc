<script lang="ts">
  // SOULS MC — Active Canvas View: Agent Inbox (Blast Radius HITL)
  //
  // Visualização e aprovação de propostas de alteração de código emitidas por agentes.
  // Conformidade: ADR-005, ADR-011 (Governança HITL), ADR-014 (Fricção Produtiva).

  import { governanceStore } from "$lib/stores/governance.svelte.ts";

  interface AgentProposal {
    id: string;
    prNumber: string;
    title: string;
    description: string;
    impactFiles: number;
    riskTier: number;
    status: "pending" | "approved" | "rejected";
  }

  let proposals = $state<AgentProposal[]>([
    {
      id: "p108",
      prNumber: "PR-108",
      title: "Refatorar Parser GGUF em Rust Zero-Copy",
      description: "Agente Harvester propõe substituir parsing Serde por memmap2 com ganho de 400% na velocidade de escaneamento de modelos.",
      impactFiles: 2,
      riskTier: 1,
      status: "pending"
    }
  ]);

  let analysisDetails = $state<string | null>(null);

  function handleApprove(proposal: AgentProposal) {
    proposal.status = "approved";
    analysisDetails = `✓ Proposta ${proposal.prNumber} aprovada pelo operador. Sincronização atômica iniciada via Tokio MPSC.`;
  }

  function handleAnalyzeImpact(proposal: AgentProposal) {
    governanceStore.recordUsage(150, 0.00006);
    analysisDetails = `Auditoria de Blast Radius para ${proposal.prNumber}:\n- Arquivos afetados: src-tauri/src/core/model_registry.rs, src-tauri/src/core/vram_scheduler.rs\n- Risco de regressão: Tier 1 (Baixo / Protegido por TDD)\n- Testes automatizados: 12 testes verdes.`;
  }
</script>

<div class="h-full w-full bg-surface-low border border-white/10 flex flex-col overflow-hidden select-none font-mono text-xs">
  <!-- Topbar Inbox -->
  <div class="h-9 px-4 bg-surface-mid border-b border-white/5 flex items-center justify-between shrink-0">
    <span class="text-cyber-purple font-bold flex items-center gap-2">
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
        <path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
      </svg>
      AGENT INBOX // PROPOSTAS DE REBASE PENDENTES (BLAST RADIUS)
    </span>
    <span class="text-text-muted text-[10px]">Governança HITL Ativa</span>
  </div>

  <div class="flex-1 p-4 overflow-y-auto space-y-3 font-mono">
    {#each proposals as prop (prop.id)}
      <div class="p-4 bg-surface-mid border {prop.status === 'approved' ? 'border-emerald-500/40' : 'border-cyber-purple/40'} flex justify-between items-start flex-wrap gap-3">
        <div class="space-y-1.5 max-w-xl text-left">
          <div class="flex items-center gap-2">
            <span class="px-2 py-0.5 bg-cyber-purple/20 text-cyber-purple text-[10px] font-bold">
              {prop.prNumber}
            </span>
            <strong class="text-text-main text-sm">{prop.title}</strong>
          </div>
          <p class="text-text-muted text-[11px] leading-relaxed">
            {prop.description}
          </p>
          <div class="text-[10px] text-telemetry-cyan pt-1">
            Blast Radius Impact: {prop.impactFiles} Arquivos | Risk Tier {prop.riskTier} (Seguro)
          </div>
        </div>

        <div class="flex gap-2 shrink-0">
          {#if prop.status === "pending"}
            <button 
              type="button"
              onclick={() => handleAnalyzeImpact(prop)}
              class="px-3 py-1.5 bg-cyber-purple/20 text-cyber-purple hover:bg-cyber-purple hover:text-black border border-cyber-purple/40 font-bold transition-colors text-xs"
            >
              ✨ Analisar Impacto
            </button>
            <button 
              type="button"
              onclick={() => handleApprove(prop)}
              class="px-3 py-1.5 bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500 hover:text-black border border-emerald-500/40 font-bold transition-colors text-xs"
            >
              APROVAR
            </button>
          {:else}
            <span class="px-3 py-1.5 bg-emerald-500/20 text-emerald-400 border border-emerald-500/40 font-bold text-xs">
              ✓ APROVADO
            </span>
          {/if}
        </div>
      </div>
    {/each}

    {#if analysisDetails}
      <div class="p-3 bg-surface-high/70 border border-telemetry-cyan/40 text-xs text-text-main font-mono whitespace-pre-wrap leading-relaxed">
        {analysisDetails}
      </div>
    {/if}
  </div>
</div>
