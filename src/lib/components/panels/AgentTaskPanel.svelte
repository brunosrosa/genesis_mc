<script lang="ts">
  import MacWindowFrame from "$lib/components/ui/MacWindowFrame.svelte";

  let queryText = $state("Search for the latest news about open source AI models released this week");
  let isReasoningExpanded = $state(true);
  let activeTab = $state<"timeline" | "output">("timeline");

  const timelineSteps = [
    {
      id: 1,
      type: "reasoning",
      title: "I'm reasoning",
      color: "#007AFF",
      detail: "Tasking agent for goal: Search for the latest news about open source AI models released this week",
    },
    {
      id: 2,
      type: "reasoning",
      title: "I'm reasoning",
      color: "#007AFF",
      detail: "Formulating multi-hop query parameters and identifying primary source feeds.",
    },
    {
      id: 3,
      type: "tool",
      title: "web_search",
      color: "#00E5FF",
      detail: "Searching: 'open source AI models releases github huggingface 2026'",
    },
    {
      id: 4,
      type: "reasoning",
      title: "I'm reasoning",
      color: "#007AFF",
      detail: "Synthesizing retrieved models: DeepSeek-V3 checkpoints, Mistral NeMo updates, Qwen2.5-Coder enhancements.",
    },
  ];
</script>

<MacWindowFrame id="agent_task" title="agent_task" width={580} height={640}>
  {#snippet headerExtra()}
    <span class="text-[10px] font-mono font-medium text-emerald-400 bg-emerald-950/60 border border-emerald-500/30 px-2 py-0.5 rounded-full flex items-center gap-1.5">
      <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
      Agent Task
    </span>
  {/snippet}

  <div class="flex-1 flex flex-col p-4 overflow-hidden text-neutral-200 gap-3">
    <!-- Query Banner -->
    <div class="macos-subpanel p-3 flex flex-col gap-1.5 border-white/[0.08]">
      <div class="flex items-center justify-between text-[11px] text-neutral-400">
        <span class="font-sans font-medium text-neutral-300">Goal Query</span>
        <span class="text-[10px] font-mono text-emerald-400">● 5/5 tasks executed</span>
      </div>
      <p class="text-xs font-sans text-neutral-100 font-medium leading-snug">
        {queryText}
      </p>
    </div>

    <!-- Timeline e Output Tabs -->
    <div class="flex items-center gap-2 border-b border-white/[0.08] pb-1.5 text-xs">
      <button
        type="button"
        class="px-2.5 py-1 rounded transition-colors font-medium {activeTab === 'timeline' ? 'bg-white/10 text-white' : 'text-neutral-400 hover:text-neutral-200'}"
        onclick={() => { activeTab = "timeline"; }}
      >
        Reasoning Timeline
      </button>
      <button
        type="button"
        class="px-2.5 py-1 rounded transition-colors font-medium {activeTab === 'output' ? 'bg-white/10 text-white' : 'text-neutral-400 hover:text-neutral-200'}"
        onclick={() => { activeTab = "output"; }}
      >
        Structured Results
      </button>
    </div>

    <!-- Conteúdo Scrollável -->
    <div class="flex-1 overflow-y-auto pr-1 space-y-4">
      {#if activeTab === "timeline"}
        <!-- Timeline com Linha Conectiva Vertical -->
        <div class="relative pl-6 space-y-4 before:absolute before:left-2.5 before:top-2 before:bottom-2 before:w-[1.5px] before:bg-white/10">
          {#each timelineSteps as step}
            <div class="relative group">
              <!-- Nó / Ponto da Timeline -->
              <span
                class="absolute -left-6 top-1 w-3 h-3 rounded-full border-2 border-black flex items-center justify-center"
                style="background-color: {step.color}; box-shadow: 0 0 8px {step.color};"
              ></span>

              <!-- Conteúdo da Etapa -->
              <div class="macos-subpanel p-2.5 rounded-lg border-white/[0.06] hover:border-white/15 transition-colors">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-mono font-medium" style="color: {step.color};">
                    {step.title}
                  </span>
                  <span class="text-[10px] font-mono text-neutral-500">done</span>
                </div>
                <p class="text-[11px] text-neutral-300 font-sans mt-1 leading-relaxed">
                  {step.detail}
                </p>
              </div>
            </div>
          {/each}

          <!-- Raw Execution Output Card -->
          <div class="macos-subpanel p-3 rounded-lg border-white/[0.08] bg-black/40 font-mono text-[10.5px] text-neutral-300 space-y-1.5 overflow-x-auto">
            <div class="text-cyan-400 font-semibold">[Yield-Rendering]:</div>
            <div class="text-neutral-400 whitespace-pre-wrap leading-relaxed">
{`{"feed":"HuggingFace/Trending", "model":"Qwen/Qwen2.5-Coder-32B-Instruct"}
{"eval":"HumanEval: 92.4%", "context_window": 131072, "license": "Apache-2.0"}
{"source":"https://huggingface.co/Qwen/Qwen2.5-Coder-32B-Instruct"}
{"summary":"Leading open-source code generation model demonstrating parity with proprietary frontier models."}`}
            </div>
          </div>
        </div>
      {:else}
        <!-- Tabela Estruturada de Resultados (Como na Imagem 2) -->
        <div class="macos-subpanel p-3.5 space-y-3">
          <div class="flex items-center justify-between pb-2 border-b border-white/[0.08]">
            <h4 class="text-xs font-semibold text-neutral-200">
              Top 5 Programming Languages — 2026 Estimates
            </h4>
            <span class="text-[10px] text-neutral-400">Aggregated Sources</span>
          </div>

          <div class="overflow-x-auto">
            <table class="w-full text-left text-[11px] font-sans">
              <thead>
                <tr class="text-neutral-400 border-b border-white/10 uppercase text-[9.5px] tracking-wider">
                  <th class="pb-1.5 font-medium">Language</th>
                  <th class="pb-1.5 font-medium">Share</th>
                  <th class="pb-1.5 font-medium">Primary Use Case</th>
                  <th class="pb-1.5 font-medium">Trend</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-white/[0.06] text-neutral-300">
                <tr>
                  <td class="py-2 font-medium text-white">JavaScript</td>
                  <td class="py-2 font-mono text-cyan-400">18%</td>
                  <td class="py-2">Web frontend, full stack, edge runtimes</td>
                  <td class="py-2 text-emerald-400">Stable</td>
                </tr>
                <tr>
                  <td class="py-2 font-medium text-white">Python</td>
                  <td class="py-2 font-mono text-cyan-400">16%</td>
                  <td class="py-2">Data science, AI/ML orchestration, backends</td>
                  <td class="py-2 text-emerald-400">Slight positive</td>
                </tr>
                <tr>
                  <td class="py-2 font-medium text-white">Rust</td>
                  <td class="py-2 font-mono text-cyan-400">13%</td>
                  <td class="py-2">Bare-metal systems, high-perf IPC, security</td>
                  <td class="py-2 text-emerald-400">Accelerating</td>
                </tr>
                <tr>
                  <td class="py-2 font-medium text-white">Java</td>
                  <td class="py-2 font-mono text-cyan-400">12%</td>
                  <td class="py-2">Enterprise backends, legacy large systems</td>
                  <td class="py-2 text-amber-400">Mild decline</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      {/if}
    </div>
  </div>
</MacWindowFrame>
