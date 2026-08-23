<script lang="ts">
  // SOULS MC — Active Canvas View: Memória & Caderno Local (NotebookLM Soberano)
  //
  // Visualizador do grafo causal de conhecimento e memórias L1/L2/L3.
  // Conformidade: ADR-004 (Tríade de Memória), ADR-005, ADR-023 (Grafo de Proximidade).

  import { governanceStore } from "$lib/stores/governance.svelte.ts";

  interface GraphNode {
    id: string;
    title: string;
    type: string;
    meta: string;
    x: number;
    y: number;
    color: string;
  }

  let nodes = $state<GraphNode[]>([
    { id: "n1", title: "Bruno (User Person)", type: "Nó Central", meta: "Perfil: 2e / TDAH / Sovereign Operator", x: 120, y: 100, color: "border-cyber-purple text-cyber-purple" },
    { id: "n2", title: "Hábitos Circadianos", type: "Caderno", meta: "4 Notas | 2 PDFs Indexados", x: 380, y: 220, color: "border-telemetry-cyan text-telemetry-cyan" },
    { id: "n3", title: "Protocolo de Sono & Foco", type: "Síntese", meta: "Status: Sincronizado no LanceDB", x: 620, y: 140, color: "border-emerald-400 text-emerald-400" },
    { id: "n4", title: "Motor Zero-Copy IPC", type: "Engenharia", meta: "Mapeamento Flatbuffers / Tokio", x: 460, y: 360, color: "border-amber-400 text-amber-400" }
  ]);

  let selectedNode = $state<GraphNode | null>(nodes[0]);
  let isSynthesizing = $state(false);
  let synthesisText = $state<string | null>(null);

  function handleSelectNode(node: GraphNode) {
    selectedNode = node;
  }

  function handleSynthesizeKnowledge() {
    isSynthesizing = true;
    governanceStore.recordUsage(210, 0.00008);

    setTimeout(() => {
      synthesisText = `Síntese Consolidada do Grafo: O operador mantém sincronia entre o ecossistema de Hábitos Circadianos e o Motor Zero-Copy. Recomenda-se ancoragem de rotinas no primeiro bloco do dia para proteger o Flow-Debt.`;
      isSynthesizing = false;
    }, 700);
  }
</script>

<div class="h-full w-full bg-surface-low border border-white/10 flex flex-col overflow-hidden select-none font-mono text-xs">
  <!-- Topbar Memória -->
  <div class="h-9 px-4 bg-surface-mid border-b border-white/5 flex items-center justify-between shrink-0">
    <span class="text-cyber-purple font-bold flex items-center gap-2">
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1-2.5-2.5Z" />
        <path d="M6 6h10M6 10h10" />
      </svg>
      MEMÓRIA & CADERNO LOCAL // NOTEBOOKLM SOBERANO (LANCE-DB + LADYBUG)
    </span>
    <button 
      type="button"
      onclick={handleSynthesizeKnowledge}
      disabled={isSynthesizing}
      class="px-2.5 py-0.5 bg-cyber-purple/20 text-cyber-purple border border-cyber-purple/30 text-[10px] font-bold hover:bg-cyber-purple hover:text-black transition-colors disabled:opacity-50"
    >
      {isSynthesizing ? 'Sintetizando...' : '✨ Sintetizar Conhecimento do Workspace'}
    </button>
  </div>

  <!-- Graph Canvas Area -->
  <div class="flex-1 relative flex items-center justify-center cyber-grid overflow-hidden bg-void/70">
    <!-- SVG Vector Connections -->
    <svg class="w-full h-full absolute inset-0 pointer-events-none">
      <line x1="200" y1="140" x2="420" y2="240" stroke="#8455EF" stroke-width="1.5" stroke-dasharray="4" />
      <line x1="420" y1="240" x2="660" y2="160" stroke="#61C2FF" stroke-width="1.5" />
      <line x1="420" y1="240" x2="500" y2="380" stroke="#BA9EFF" stroke-width="2" />
    </svg>

    <!-- Dynamic Graph Nodes -->
    {#each nodes as node (node.id)}
      <div 
        role="button"
        tabindex="0"
        onkeydown={(e) => { if (e.key === 'Enter') handleSelectNode(node); }}
        onclick={() => handleSelectNode(node)}
        style="transform: translate({node.x - 250}px, {node.y - 200}px);"
        class="absolute p-3 bg-surface-mid border {node.color} shadow-lg cursor-pointer hover:scale-105 transition-transform max-w-[220px] text-left {selectedNode?.id === node.id ? 'ring-2 ring-cyber-purple shadow-cyber-purple/30' : ''}"
      >
        <div class="font-bold truncate">[{node.type}] {node.title}</div>
        <div class="text-[10px] text-text-muted mt-1 leading-snug">{node.meta}</div>
      </div>
    {/each}

    <!-- Synthesis Modal Box -->
    {#if synthesisText}
      <div class="absolute bottom-4 left-4 right-4 bg-surface-mid border border-cyber-purple p-4 text-xs font-mono text-text-main shadow-2xl space-y-2">
        <div class="flex justify-between items-center font-bold text-cyber-purple">
          <span>SÍNTESE DE CONHECIMENTO DO WORKSPACE (NOTEBOOKLM SOBERANO)</span>
          <button 
            type="button"
            onclick={() => synthesisText = null} 
            class="text-text-muted hover:text-white"
          >
            ✕
          </button>
        </div>
        <div class="leading-relaxed text-text-main text-xs">{synthesisText}</div>
      </div>
    {/if}
  </div>
</div>
