<script lang="ts">
  import MacWindowFrame from "$lib/components/ui/MacWindowFrame.svelte";
  import { telemetry } from "$lib/stores/telemetry.svelte.ts";

  let selectedTab = $state("Kernel & Metrics");

  // Controles reativos do Scheduler & Context Window
  let schedulingPolicy = $state("FIFO");
  let maxQueueSize = $state(100);
  let defaultMaxTokens = $state(16400);
  let reservedTokens = $state(30);
  let autoPrune = $state(true);
  let autoCompact = $state(false);

  const navItems = [
    { id: "all_engine", label: "All Engine", icon: "cpu" },
    { id: "appearance", label: "Appearance", icon: "palette" },
    { id: "develop", label: "Develop", icon: "code" },
    { id: "spaces", label: "Spaces", icon: "layout" },
    { id: "sync", label: "Sync Manager", icon: "refresh" },
    { id: "synthesis", label: "Synthesis", icon: "sparkles" },
    { id: "privacy", label: "Privacy & Data", icon: "shield" },
    { id: "network", label: "Network", icon: "globe" },
    { id: "storage", label: "Storage", icon: "hard-drive" },
    { id: "memory", label: "Data & Memory", icon: "database" },
    { id: "performance", label: "Performance", icon: "zap" },
    { id: "display", label: "Display", icon: "monitor" },
    { id: "audio", label: "Audio", icon: "volume" },
    { id: "notifications", label: "Notifications", icon: "bell" },
    { id: "agent_loop", label: "Agent Loop", icon: "repeat" },
    { id: "personas", label: "Agent Personas", icon: "users" },
    { id: "system_tools", label: "System Tools", icon: "tool" },
    { id: "chatbots", label: "Chatbots", icon: "message" },
    { id: "account", label: "Account", icon: "user" },
    { id: "users", label: "Users", icon: "user-check" },
    { id: "kernel_metrics", label: "Kernel & Metrics", icon: "activity" },
    { id: "advanced", label: "Advanced", icon: "sliders" },
  ];
</script>

<MacWindowFrame id="settings" title="Settings" width={660} height={700}>
  <div class="flex h-full overflow-hidden text-neutral-200">
    <!-- Sub-sidebar esquerda com navegação estilo macOS -->
    <aside class="w-48 bg-black/25 border-r border-white/[0.06] flex flex-col p-2 overflow-y-auto shrink-0 select-none text-[11.5px]">
      <div class="space-y-0.5">
        {#each navItems as item}
          <button
            type="button"
            class="w-full flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg transition-colors font-sans text-left {selectedTab === item.label ? 'bg-[#007AFF] text-white font-medium shadow-sm' : 'text-neutral-400 hover:text-white hover:bg-white/[0.04]'}"
            onclick={() => { selectedTab = item.label; }}
          >
            <!-- Ícone genérico minimalista -->
            <span class="w-3.5 h-3.5 flex items-center justify-center opacity-85">
              {#if item.icon === "activity"}
                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>
              {:else if item.icon === "cpu"}
                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="14" x2="23" y2="14"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="14" x2="4" y2="14"/></svg>
              {:else if item.icon === "shield"}
                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              {:else if item.icon === "database"}
                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>
              {:else}
                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
              {/if}
            </span>
            <span class="truncate">{item.label}</span>
          </button>
        {/each}
      </div>
    </aside>

    <!-- Área de Conteúdo da Direita -->
    <section class="flex-1 p-5 overflow-y-auto space-y-6">
      <div class="flex items-center justify-between pb-3 border-b border-white/[0.08]">
        <h2 class="text-sm font-semibold tracking-wide text-white uppercase font-sans">
          Kernel & Metrics
        </h2>
        <span class="text-[10px] font-mono text-cyan-400/90 bg-cyan-950/60 px-2 py-0.5 rounded border border-cyan-500/30">
          BARE-METAL ACTIVE
        </span>
      </div>

      <!-- KERNEL DASHBOARD -->
      <div class="space-y-3">
        <h3 class="text-[11px] font-semibold text-neutral-400 uppercase tracking-wider font-sans">
          Kernel Dashboard
        </h3>
        
        <div class="grid grid-cols-3 gap-2 text-xs">
          <!-- Card Uptime -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Uptime</span>
            <span class="text-sm font-mono font-semibold text-neutral-100 mt-1">14h 47m</span>
          </div>

          <!-- Total Dispatches -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Total Dispatches</span>
            <span class="text-sm font-mono font-semibold text-neutral-100 mt-1">10</span>
          </div>

          <!-- Active Agents -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Active Agents</span>
            <span class="text-sm font-mono font-semibold text-cyan-400 mt-1">0</span>
          </div>

          <!-- LLM Latency -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">LLM Latency</span>
            <span class="text-sm font-mono font-semibold text-neutral-100 mt-1">142.51ms</span>
          </div>

          <!-- Queue Size -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Queue Size</span>
            <span class="text-sm font-mono font-semibold text-neutral-100 mt-1">2</span>
          </div>

          <!-- Memory -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Memory</span>
            <span class="text-sm font-mono font-semibold text-amber-400 mt-1">72%</span>
          </div>

          <!-- Success Rate -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Success</span>
            <span class="text-sm font-mono font-semibold text-emerald-400 mt-1">9/10</span>
          </div>

          <!-- Avg TTFT -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Avg TTFT</span>
            <span class="text-sm font-mono font-semibold text-neutral-100 mt-1">4.2</span>
          </div>

          <!-- Avg Execution -->
          <div class="macos-subpanel p-2.5 flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-neutral-400 font-medium">Avg Execution</span>
            <span class="text-sm font-mono font-semibold text-neutral-100 mt-1">181.4s</span>
          </div>
        </div>

        <div class="pt-1 flex items-center justify-between text-[10px] font-mono text-neutral-500">
          <span>LOCK HASH: e24f1</span>
          <span>MEM_PAGE_SZ: 4096</span>
        </div>
      </div>

      <!-- SCHEDULER -->
      <div class="space-y-3 pt-2">
        <h3 class="text-[11px] font-semibold text-neutral-400 uppercase tracking-wider font-sans">
          Scheduler
        </h3>
        <p class="text-[11px] text-neutral-400 leading-relaxed font-sans">
          The scheduler is the heart of the kernel. It decides in what order syscalls from agents get processed. Different policies trade off fairness, latency, and throughput. FIFO is the simplest; advanced EoS policies prevent any single agent from starving the others.
        </p>

        <div class="space-y-4 pt-1">
          <!-- Scheduling Policy Dropdown -->
          <div class="flex items-center justify-between">
            <label for="sched_policy" class="text-xs font-medium text-neutral-300">Scheduling Policy</label>
            <select
              id="sched_policy"
              bind:value={schedulingPolicy}
              class="bg-black/50 border border-white/10 rounded-lg px-3 py-1 text-xs text-neutral-200 outline-none hover:border-white/20 transition-colors cursor-pointer"
            >
              <option value="FIFO">FIFO</option>
              <option value="Priority">Priority Weighted</option>
              <option value="RoundRobin">Round Robin</option>
            </select>
          </div>

          <!-- Max Queue Size Slider -->
          <div class="space-y-1.5">
            <div class="flex items-center justify-between text-xs">
              <label for="max_queue_slider" class="text-neutral-300 font-medium">Max Queue Size</label>
              <span class="font-mono text-neutral-400">{maxQueueSize}</span>
            </div>
            <input
              id="max_queue_slider"
              type="range"
              min="10"
              max="500"
              bind:value={maxQueueSize}
              class="slider-mac"
            />
          </div>
        </div>
      </div>

      <!-- CONTEXT WINDOW -->
      <div class="space-y-3 pt-2">
        <h3 class="text-[11px] font-semibold text-neutral-400 uppercase tracking-wider font-sans">
          Context Window
        </h3>
        <p class="text-[11px] text-neutral-400 leading-relaxed font-sans">
          Each agent maintains a context window — the conversation history the LLM sees when making decisions. These settings control how much memory each agent gets and what happens when it fills up. Larger windows give agent better recall but cost more tokens per round.
        </p>

        <div class="space-y-4 pt-1">
          <!-- Default Max Tokens Slider -->
          <div class="space-y-1.5">
            <div class="flex items-center justify-between text-xs">
              <label for="max_tokens_slider" class="text-neutral-300 font-medium">Default Max Tokens</label>
              <span class="font-mono text-neutral-400">{defaultMaxTokens.toLocaleString()}</span>
            </div>
            <input
              id="max_tokens_slider"
              type="range"
              min="2048"
              max="65536"
              step="1024"
              bind:value={defaultMaxTokens}
              class="slider-mac"
            />
          </div>

          <!-- Reserved Tokens (%) Slider -->
          <div class="space-y-1.5">
            <div class="flex items-center justify-between text-xs">
              <label for="reserved_tokens_slider" class="text-neutral-300 font-medium">Reserved Tokens (%)</label>
              <span class="font-mono text-neutral-400">{reservedTokens}%</span>
            </div>
            <input
              id="reserved_tokens_slider"
              type="range"
              min="5"
              max="50"
              bind:value={reservedTokens}
              class="slider-mac"
            />
          </div>

          <!-- Auto-Prune Toggle -->
          <div class="flex items-start justify-between gap-4 pt-2">
            <div class="space-y-1">
              <span class="text-xs font-medium text-neutral-200 block">Auto-Prune</span>
              <p class="text-[10.5px] text-neutral-400 leading-normal">
                When the context fills beyond Default Max Tokens, automatically drop the oldest user/agent messages to make room without errors. 40% is the default.
              </p>
            </div>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="toggle-mac-track shrink-0 {autoPrune ? 'active' : ''}"
              onclick={() => { autoPrune = !autoPrune; }}
            >
              <div class="toggle-mac-thumb"></div>
            </div>
          </div>

          <!-- Auto-Compact Toggle -->
          <div class="flex items-start justify-between gap-4 pt-1">
            <div class="space-y-1">
              <span class="text-xs font-medium text-neutral-200 block">Auto-Compact</span>
              <p class="text-[10.5px] text-neutral-400 leading-normal">
                Instead of simply dropping old messages, summarize them into a compressed memory checkpoint before removing them. Preserves key historical context.
              </p>
            </div>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="toggle-mac-track shrink-0 {autoCompact ? 'active' : ''}"
              onclick={() => { autoCompact = !autoCompact; }}
            >
              <div class="toggle-mac-thumb"></div>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</MacWindowFrame>
