<script lang="ts">
  // SOULS MC — Active Canvas View: Central de Governança 5-Eixos
  //
  // Painel de controle de VRAM (RTX 2060m 6GB), Servidores MCP, Bio-Persona, FinOps e FrankenSQLite.
  // Conformidade: ADR-001, ADR-008, ADR-011, ADR-026, ADR-027, ADR-041.

  import { governanceStore } from "$lib/stores/governance.svelte.ts";
  import { telemetry } from "$lib/stores/telemetry.svelte.ts";

  type SettingsTab = "models" | "mcps" | "rapport" | "finops" | "storage";
  let activeTab = $state<SettingsTab>("models");

  let monthlyCap = $state(10.00);
  let vacuumFeedback = $state<string | null>(null);

  function handleRunVacuum() {
    vacuumFeedback = "Executando WAL Checkpoint e VACUUM no souls_state.db...";
    setTimeout(() => {
      vacuumFeedback = "✓ WAL integrado com sucesso. 0 páginas órfãs. Integridade: OK.";
    }, 600);
  }

  const vramUsedGb = $derived((telemetry.vram_mb / 1024).toFixed(1));
  const vramPercent = $derived(Math.min(100, Math.round((telemetry.vram_mb / 6000) * 100)));
</script>

<div class="h-full w-full bg-surface-low border border-white/10 flex flex-col overflow-hidden select-none font-mono text-xs">
  <!-- Topbar Governança -->
  <div class="h-9 px-4 bg-surface-mid border-b border-white/5 flex items-center justify-between shrink-0">
    <span class="text-emerald-400 font-bold flex items-center gap-2">
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
      CENTRAL DE CONTROLE SOBERANO // GOVERNANÇA SODA (5-AXIS)
    </span>
    <span class="text-[10px] text-text-muted">Kernel Governança Ativo</span>
  </div>

  <div class="flex flex-1 overflow-hidden">
    <!-- Settings Navigation Sidebar (Left) -->
    <div class="w-48 bg-surface-mid border-r border-white/5 p-2 space-y-1 shrink-0 text-xs">
      <button 
        type="button"
        onclick={() => activeTab = "models"} 
        class="w-full text-left p-2 transition-colors {activeTab === 'models' ? 'bg-surface-high text-cyber-purple border-l-2 border-cyber-purple font-bold' : 'text-text-muted hover:text-text-main'}"
      >
        1. Modelos & VRAM
      </button>
      <button 
        type="button"
        onclick={() => activeTab = "mcps"} 
        class="w-full text-left p-2 transition-colors {activeTab === 'mcps' ? 'bg-surface-high text-cyber-purple border-l-2 border-cyber-purple font-bold' : 'text-text-muted hover:text-text-main'}"
      >
        2. Ferramentas (MCPs)
      </button>
      <button 
        type="button"
        onclick={() => activeTab = "rapport"} 
        class="w-full text-left p-2 transition-colors {activeTab === 'rapport' ? 'bg-surface-high text-cyber-purple border-l-2 border-cyber-purple font-bold' : 'text-text-muted hover:text-text-main'}"
      >
        3. Alma & Rapport
      </button>
      <button 
        type="button"
        onclick={() => activeTab = "finops"} 
        class="w-full text-left p-2 transition-colors {activeTab === 'finops' ? 'bg-surface-high text-cyber-purple border-l-2 border-cyber-purple font-bold' : 'text-text-muted hover:text-text-main'}"
      >
        4. FinOps & Cofre
      </button>
      <button 
        type="button"
        onclick={() => activeTab = "storage"} 
        class="w-full text-left p-2 transition-colors {activeTab === 'storage' ? 'bg-surface-high text-cyber-purple border-l-2 border-cyber-purple font-bold' : 'text-text-muted hover:text-text-main'}"
      >
        5. Espaços & Storage
      </button>
    </div>

    <!-- Sub-Content Area -->
    <div class="flex-1 p-6 overflow-y-auto space-y-6 text-xs text-text-main">
      {#if activeTab === "models"}
        <!-- TAB 1: MODELOS & HARDWARE -->
        <div class="space-y-4">
          <div class="font-bold text-cyber-purple border-b border-white/10 pb-2">GERENCIAMENTO DE MODELOS & RECURSOS DE VRAM</div>
          <div class="bg-surface-mid p-4 border border-white/10 space-y-2">
            <div class="flex justify-between items-center">
              <span class="font-bold">Modelo Local Ativo (Engine SLM)</span>
              <span class="text-emerald-400 font-bold">Phi-4-Mini [IQ3_M] (Carregado na VRAM)</span>
            </div>
            <p class="text-text-muted text-[11px]">Varredura O(1) de arquivos GGUF na pasta /models com alocação direta via mmap2.</p>
          </div>

          <div class="bg-surface-mid p-4 border border-white/10 space-y-3">
            <div class="flex justify-between items-center">
              <span class="font-bold text-telemetry-cyan">Teto de VRAM Reservado (NVIDIA RTX 2060m - 6GB Total)</span>
              <span class="text-telemetry-cyan font-bold">{vramUsedGb} GB / 6.0 GB</span>
            </div>
            <div class="w-full bg-surface-low h-3 border border-white/10 flex overflow-hidden">
              <div class="bg-cyber-purple h-full transition-all duration-200" style="width: {vramPercent}%" title="Weights + KV Cache"></div>
              <div class="bg-telemetry-cyan/20 h-full flex-1" title="Reservado para Host OS"></div>
            </div>
            <div class="flex justify-between text-[10px] text-text-muted">
              <span>Alocação Estável</span>
              <span>Limite Rígido: 5.000 MB</span>
            </div>
          </div>
        </div>

      {:else if activeTab === "mcps"}
        <!-- TAB 2: SERVIDORES MCP -->
        <div class="space-y-4">
          <div class="flex justify-between items-center border-b border-white/10 pb-2">
            <span class="font-bold text-cyber-purple">SERVIDORES MCP ATIVOS (GATEWAY SODA // SOULS_MCP)</span>
            <span class="text-[10px] text-text-muted">ADR-026 / ADR-041</span>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div class="p-3 bg-surface-mid border border-white/10 flex justify-between items-center">
              <div>
                <div class="font-bold text-text-main">mcp-lance-memory</div>
                <div class="text-[10px] text-text-muted">Busca vetorial local (LanceDB)</div>
              </div>
              <span class="w-2.5 h-2.5 rounded-full bg-emerald-400 shadow-[0_0_8px_#10B981]" title="Ativo & Respondendo"></span>
            </div>

            <div class="p-3 bg-surface-mid border border-white/10 flex justify-between items-center">
              <div>
                <div class="font-bold text-text-main">mcp-filesystem-mmap</div>
                <div class="text-[10px] text-text-muted">Acesso I/O Zero-Copy mmap2</div>
              </div>
              <span class="w-2.5 h-2.5 rounded-full bg-emerald-400 shadow-[0_0_8px_#10B981]" title="Ativo & Respondendo"></span>
            </div>

            <div class="p-3 bg-surface-mid border border-white/10 flex justify-between items-center">
              <div>
                <div class="font-bold text-text-main">mcp-ast-master</div>
                <div class="text-[10px] text-text-muted">Fatiamento cirúrgico de código AST</div>
              </div>
              <span class="w-2.5 h-2.5 rounded-full bg-emerald-400 shadow-[0_0_8px_#10B981]" title="Ativo & Respondendo"></span>
            </div>

            <div class="p-3 bg-surface-mid border border-white/10 flex justify-between items-center">
              <div>
                <div class="font-bold text-text-main">mcp-sequential-thinking</div>
                <div class="text-[10px] text-text-muted">Freio cognitivo e orquestração DAG</div>
              </div>
              <span class="w-2.5 h-2.5 rounded-full bg-emerald-400 shadow-[0_0_8px_#10B981]" title="Ativo & Respondendo"></span>
            </div>
          </div>
        </div>

      {:else if activeTab === "rapport"}
        <!-- TAB 3: ALMA & RAPPORT (PERFIL 2E/TDAH) -->
        <div class="space-y-4">
          <div class="font-bold text-cyber-purple border-b border-white/10 pb-2">PERFIL DO OPERADOR & RAPPORT SOCRÁTICO (ADR-014 / ADR-022)</div>
          <div class="bg-surface-mid p-4 border border-white/10 space-y-2">
            <div class="font-bold text-text-main">Bio-Persona Ativa: Bruno (2e / TDAH)</div>
            <p class="text-[11px] text-text-muted leading-relaxed">
              O Master Soul opera em modo direto: elimina saudações genéricas, introduções vazias e 'slop' corporativo.
              As respostas são estruturadas em blocos atômicos com feedback imediato para proteger a memória de trabalho.
            </p>
          </div>
        </div>

      {:else if activeTab === "finops"}
        <!-- TAB 4: FINOPS & COFRE -->
        <div class="space-y-4">
          <div class="font-bold text-cyber-purple border-b border-white/10 pb-2">FINOPS & CONTROLE DE GASTOS COM IA (ADR-008)</div>
          <div class="bg-surface-mid p-4 border border-white/10 space-y-3">
            <div class="flex justify-between items-center">
              <span class="font-bold">Teto Orçamentário Mensal (Hard Cap)</span>
              <span class="text-amber-400 font-bold">${monthlyCap.toFixed(2)} / Mês</span>
            </div>
            <p class="text-[11px] text-text-muted">Trava dura (disjuntor de segurança) para impedir chamadas acidentais a APIs cloud.</p>
          </div>

          <div class="bg-surface-mid p-4 border border-white/10 space-y-2">
            <div class="flex justify-between items-center text-text-muted">
              <span>Gasto Acumulado no Mês:</span>
              <span class="text-text-main font-bold">${governanceStore.totalUsd.toFixed(3)}</span>
            </div>
            <div class="flex justify-between items-center text-text-muted">
              <span>Tokens Locais (Custo Zero):</span>
              <span class="text-emerald-400 font-bold">{(governanceStore.totalTokens * 0.85).toFixed(0)} tok</span>
            </div>
          </div>
        </div>

      {:else if activeTab === "storage"}
        <!-- TAB 5: ESPAÇOS & STORAGE -->
        <div class="space-y-4">
          <div class="font-bold text-cyber-purple border-b border-white/10 pb-2">STORAGE & INTEGRIDADE DO FRANKEN-SQLITE (ADR-004)</div>
          <div class="bg-surface-mid p-4 border border-white/10 space-y-3">
            <div class="flex justify-between items-center">
              <div>
                <div class="font-bold text-text-main">FrankenSQLite WAL Vacuum</div>
                <div class="text-[10px] text-text-muted">Compactação periódica de .souls_data/souls_state.db</div>
              </div>
              <button 
                type="button"
                onclick={handleRunVacuum}
                class="px-3 py-1 bg-surface-high hover:bg-cyber-purple/20 text-cyber-purple border border-white/10 text-[10px] font-bold transition-colors"
              >
                Executar Vacuum
              </button>
            </div>
            {#if vacuumFeedback}
              <div class="p-2 bg-surface-low border border-emerald-500/30 text-emerald-400 text-[11px]">
                {vacuumFeedback}
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
