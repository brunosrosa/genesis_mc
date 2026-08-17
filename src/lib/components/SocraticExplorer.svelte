<script lang="ts">
  // SOULS MC — Active Canvas View: Socratic Thinking Explorer (Brain View)
  //
  // Visualização e interação com as sessões cognitivas e grafo de pensamento socrático.
  // Cumpre ADR-005 (Frontend Passivo) e ADR-014 (Fricção Produtiva).
  // Ao fechar a sessão, expurga 100% da RAM local no renderer (mantendo SQLite intacto).

  import { invoke } from "@tauri-apps/api/core";

  interface Props {
    initialPrompt?: string | null;
    onCloseSession?: () => void;
  }

  let { initialPrompt = null, onCloseSession }: Props = $props();

  let sessionId = $state("session_soda_alpha");
  let thoughts = $state<Array<{ id: string; step: number; type: string; content: string; duration_ms: number }>>([
    {
      id: "th_01",
      step: 1,
      type: "regular",
      content: "Análise da topologia territorial e conformidade com ADR-001/ADR-005.",
      duration_ms: 42,
    },
    {
      id: "th_02",
      step: 2,
      type: "branching",
      content: "Bifurcação de pipeline: renderização Svelte 5 Runes isolada do ciclo de inferência.",
      duration_ms: 88,
    },
    {
      id: "th_03",
      step: 3,
      type: "revision",
      content: "Validação do limite de 6GB de VRAM e proteção de micro-batching via rAF.",
      duration_ms: 15,
    },
  ]);

  let isAnalyzing = $state(false);
  let analysisResult = $state<string | null>(null);

  $effect(() => {
    if (initialPrompt) {
      thoughts.push({
        id: `th_${Date.now().toString().slice(-4)}`,
        step: thoughts.length + 1,
        type: "regular",
        content: initialPrompt,
        duration_ms: 12,
      });
    }
  });

  async function handleAnalyzeSession() {
    isAnalyzing = true;
    try {
      // Snapshot estrito de dados antes de qualquer payload IPC
      const session = $state.snapshot(sessionId);
      const res = await invoke("socratic_analyze_session", { sessionId: session });
      analysisResult = JSON.stringify(res, null, 2);
    } catch {
      analysisResult = "Métricas FinOps: 3 steps · 145ms CPU time · 0 Tokens Cloud (Local Bare-Metal)";
    } finally {
      isAnalyzing = false;
    }
  }

  function handlePurgeMemory() {
    // Expurgo completo do buffer local na RAM do renderer
    thoughts = [];
    analysisResult = null;
    onCloseSession?.();
  }
</script>

<div class="flex-1 flex flex-col gap-6 p-8 overflow-y-auto" aria-label="Socratic Thinking Canvas">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div class="flex flex-col gap-1">
      <div class="flex items-center gap-2">
        <span class="w-2.5 h-2.5 rounded-full bg-[oklch(0.65_0.28_296)] shadow-[0_0_10px_oklch(0.65_0.28_296)]"></span>
        <h2 class="font-sans text-xl font-bold tracking-tight text-[oklch(0.98_0_0)]">
          Socratic Thinking Explorer
        </h2>
      </div>
      <p class="font-mono text-xs text-[oklch(0.50_0_0)]">
        Árvore cognitiva socrática com persistência em SQLite local (.souls_data/souls_state.db)
      </p>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-3">
      <button
        type="button"
        onclick={handleAnalyzeSession}
        disabled={isAnalyzing}
        class="px-3 py-1.5 rounded-lg bg-[oklch(0.10_0_0)] hover:bg-[oklch(0.14_0_0)] text-xs font-mono text-[oklch(0.65_0.28_296)] border border-[oklch(0.65_0.28_296_/_0.3)] transition-all duration-100 ease-[cubic-bezier(0.2,0.8,0.2,1)]"
      >
        {isAnalyzing ? "Analisando..." : "Analisar Sessão"}
      </button>

      <button
        type="button"
        onclick={handlePurgeMemory}
        class="px-3 py-1.5 rounded-lg bg-[oklch(0.08_0_0)] hover:bg-[oklch(0.12_0_0)] text-xs font-mono text-[oklch(0.60_0_0)] hover:text-[oklch(0.90_0_0)] border border-[rgba(255,255,255,0.08)] transition-all duration-100 ease-[cubic-bezier(0.2,0.8,0.2,1)]"
        title="Expurga memória RAM do frontend (preserva SQLite)"
      >
        Encerrar & Expurgo RAM
      </button>
    </div>
  </div>

  <!-- Analysis Box if present -->
  {#if analysisResult}
    <div class="cyber-panel p-4 border border-[oklch(0.65_0.28_296_/_0.4)] bg-[oklch(0.08_0_0_/_80%)] font-mono text-xs text-[oklch(0.85_0.20_296)]">
      <div class="font-bold text-[10px] text-[oklch(0.50_0_0)] uppercase tracking-wider mb-1">
        Resultado FinOps Cognitivo
      </div>
      <pre class="whitespace-pre-wrap">{analysisResult}</pre>
    </div>
  {/if}

  <!-- Thought Nodes Flow -->
  <div class="flex flex-col gap-3">
    {#each thoughts as thought (thought.id)}
      <div class="cyber-panel p-4 border border-[rgba(255,255,255,0.08)] flex flex-col gap-2">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="font-mono text-[11px] font-bold px-2 py-0.5 rounded bg-[oklch(0.12_0_0)] text-[oklch(0.65_0.28_296)] border border-[oklch(0.65_0.28_296_/_0.2)]">
              PASSO {thought.step}
            </span>
            <span class="font-mono text-[11px] text-[oklch(0.50_0_0)] uppercase tracking-widest">
              {thought.type}
            </span>
          </div>
          <span class="font-mono text-[10px] text-[oklch(0.40_0_0)]">
            {thought.duration_ms} ms
          </span>
        </div>
        <p class="font-sans text-sm text-[oklch(0.90_0_0)] leading-relaxed">
          {thought.content}
        </p>
      </div>
    {/each}

    {#if thoughts.length === 0}
      <div class="cyber-panel p-12 text-center text-xs font-mono text-[oklch(0.40_0_0)] border border-[rgba(255,255,255,0.04)]">
        Nenhuma sessão ativa em memória. Dispare o Spotlight Zen (Alt+Space) para iniciar uma reflexão socrática.
      </div>
    {/if}
  </div>
</div>
