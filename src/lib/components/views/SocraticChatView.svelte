<script lang="ts">
  // SOULS MC — Active Canvas View: Socratic Chat + Bancada JIT (Split View)
  //
  // Diálogo socrático em tempo real, streaming do evento Tauri 'socratic-thought'
  // visualização de tags invariantes [Tsys/Ttools/Tstate] e projeção na Bancada JIT.
  // Conformidade: ADR-005, ADR-010, ADR-014, ADR-022, ADR-025, ADR-045.

  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { workspaceStore } from "$lib/stores/workspace.svelte.ts";
  import { governanceStore } from "$lib/stores/governance.svelte.ts";

  interface Props {
    onNavigateToBancada?: () => void;
  }

  let { onNavigateToBancada }: Props = $props();

  let isSubCanvasOpen = $state(true);
  let chatInputText = $state("");

  export type SocraticThoughtType = "regular" | "revision" | "branching";

  export interface SocraticThoughtPayload {
    id?: string;
    type?: SocraticThoughtType;
    content?: string;
    text?: string;
    thought?: string;
    model?: string;
    timestamp?: string;
    codeSnippet?: {
      targetFile: string;
      tag: string;
      code: string;
    };
  }

  export interface ChatMessage {
    id: string;
    sender: "user" | "soul";
    thoughtType?: SocraticThoughtType;
    text: string;
    timestamp: string;
    model?: string;
    codeSnippet?: {
      targetFile: string;
      tag: string;
      code: string;
    };
  }

  let messages = $state<ChatMessage[]>([
    {
      id: "m1",
      sender: "user",
      text: "Como podemos organizar as rotinas de hiperfoco e saúde sem sobrecarregar a memória de trabalho?",
      timestamp: "16:42:01"
    },
    {
      id: "m2",
      sender: "soul",
      thoughtType: "regular",
      text: "Mestre Bruno, para evitar o esgotamento do **Flow-Debt**, isolamos o planejamento em um **Kanban Agêntico** e testamos os protótipos diretamente na **Bancada**.",
      timestamp: "16:42:04",
      model: "Gemini 2.5 Flash",
      codeSnippet: {
        targetFile: "src-tauri/src/routine_engine.rs",
        tag: "Zero-Copy Wasm",
        code: `pub fn calc_circadian_anchor(wake_time: u32) -> Result<AnchorWindow> {\n    // Cálculo atômico de janela de dopamina para evitar fadiga\n    let window = AnchorWindow::new(wake_time)?;\n    Ok(window)\n}`
      }
    }
  ]);

  let currentSubCanvasCode = $state<{ targetFile: string; tag: string; code: string }>({
    targetFile: "src-tauri/src/routine_engine.rs",
    tag: "Zero-Copy Wasm",
    code: `pub fn calc_circadian_anchor(wake_time: u32) -> Result<AnchorWindow> {\n    // Cálculo atômico de janela de dopamina para evitar fadiga\n    let window = AnchorWindow::new(wake_time)?;\n    Ok(window)\n}`
  });

  let unlistenSocratic: UnlistenFn | null = null;

  onMount(() => {
    void (async () => {
      try {
        unlistenSocratic = await listen<SocraticThoughtPayload>("socratic-thought", (event) => {
          const payload = event.payload;
          if (!payload) return;

          const text = payload.content || payload.thought || payload.text || "";
          const thoughtType = payload.type || "regular";
          const newMsg: ChatMessage = {
            id: payload.id || `st_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
            sender: "soul",
            thoughtType,
            text,
            timestamp: payload.timestamp || new Date().toLocaleTimeString(),
            model: payload.model || "Gemini 2.5 Flash",
            codeSnippet: payload.codeSnippet
          };

          messages.push(newMsg);

          if (payload.codeSnippet) {
            currentSubCanvasCode = payload.codeSnippet;
            isSubCanvasOpen = true;
          }
        });
      } catch {
        // Fallback gracioso em ambiente web sem Tauri backend
      }
    })();

    return () => {
      unlistenSocratic?.();
    };
  });

  function handleSendMessage() {
    const text = chatInputText.trim();
    if (!text) return;

    const userMessage: ChatMessage = {
      id: `usr_${Date.now()}`,
      sender: "user",
      text,
      timestamp: new Date().toLocaleTimeString()
    };

    messages.push(userMessage);
    chatInputText = "";
    governanceStore.recordUsage(120, 0.00005);
  }

  function projectToSubCanvas(snippet: { targetFile: string; tag: string; code: string }) {
    currentSubCanvasCode = snippet;
    isSubCanvasOpen = true;
  }
</script>

<div class="h-full w-full flex gap-3 overflow-hidden select-none">
  <!-- Chat Panel (Left Split) -->
  <div class="flex-1 h-full bg-void-black/90 border border-white/10 flex flex-col justify-between overflow-hidden">
    <!-- Chat Header -->
    <div class="h-9 px-4 bg-surface-mid border-b border-white/5 flex items-center justify-between font-mono text-xs shrink-0">
      <div class="flex items-center gap-2">
        <span class="w-2 h-2 rounded-full bg-cyber-purple animate-pulse"></span>
        <span class="font-semibold text-text-main">Diálogo Socrático // Master Soul (SODA v3)</span>
      </div>
      <span class="text-[10px] text-text-muted">
        Notação LEAN Ativa | Workspace: <strong class="text-cyber-purple">{workspaceStore.activeWorkspace.title}</strong>
      </span>
    </div>

    <!-- Chat Messages Stream -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4 font-body text-xs">
      <!-- Invariant System Tags -->
      <div class="flex justify-center">
        <div class="bg-surface-high/70 border border-white/10 px-3 py-1 font-mono text-[10px] text-text-muted flex items-center gap-2.5 shadow-inner">
          <span class="text-cyber-purple font-bold tracking-tight bg-cyber-purple/10 px-1.5 py-0.5 border border-cyber-purple/20">[Tsys: Imutável]</span>
          <span>Soul Core Loaded</span>
          <span class="text-telemetry-cyan font-bold tracking-tight bg-telemetry-cyan/10 px-1.5 py-0.5 border border-telemetry-cyan/20">[Ttools: 12 Active MCPs]</span>
          <span class="text-emerald-400 font-bold tracking-tight bg-emerald-400/10 px-1.5 py-0.5 border border-emerald-400/20">[Tstate_mv: Synced]</span>
        </div>
      </div>

      {#each messages as msg (msg.id)}
        {#if msg.sender === "user"}
          <!-- User Message -->
          <div class="flex flex-col items-end gap-1">
            <div class="bg-surface-high border border-white/10 p-3 max-w-[80%] text-text-main font-mono shadow-md">
              <span class="text-cyber-purple font-bold">Bruno:</span> "{msg.text}"
            </div>
            <span class="font-mono text-[9px] text-text-muted">{msg.timestamp}</span>
          </div>
        {:else}
          <!-- Soul Agent / Socratic Thought Message -->
          <div class="flex flex-col items-start gap-1">
            <div 
              class="p-3.5 max-w-[85%] text-text-main space-y-2 border {msg.thoughtType === 'revision' ? 'bg-surface-mid border-telemetry-cyan/40' : msg.thoughtType === 'branching' ? 'bg-surface-mid border-emerald-400/40' : 'bg-surface-mid border-cyber-purple/30'}"
            >
              <!-- Socratic Thought Bullet Header -->
              <div class="flex items-center justify-between font-mono text-[11px] font-semibold border-b border-white/5 pb-1">
                <span class="flex items-center gap-1.5 {msg.thoughtType === 'revision' ? 'text-telemetry-cyan' : msg.thoughtType === 'branching' ? 'text-emerald-400' : 'text-cyber-purple'}">
                  {#if msg.thoughtType === 'revision'}
                    <span class="w-2 h-2 rounded-full bg-telemetry-cyan"></span>
                    <span>REVISÃO SOCRÁTICA</span>
                  {:else if msg.thoughtType === 'branching'}
                    <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
                    <span>RAMIFICAÇÃO HIPÓTESE</span>
                  {:else}
                    <span class="w-2 h-2 rounded-full bg-cyber-purple"></span>
                    <span>PENSAMENTO SOCRÁTICO</span>
                  {/if}
                </span>
                <span class="text-[9px] bg-white/5 px-1.5 py-0.5 text-text-muted font-mono">
                  {msg.model || 'Gemini 2.5'}
                </span>
              </div>

              <!-- Thought Content -->
              <p class="leading-relaxed font-sans text-xs">
                {msg.text}
              </p>

              {#if msg.codeSnippet}
                <!-- A2UI Projection Card -->
                <div class="mt-3 p-3 bg-surface-high/80 border border-cyber-purple/40 font-mono text-[11px]">
                  <div class="flex items-center justify-between text-cyber-purple font-bold mb-1.5">
                    <span class="flex items-center gap-1.5">
                      <svg class="w-3.5 h-3.5 text-telemetry-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
                      </svg>
                      Projeção Sugerida na Bancada
                    </span>
                    <span class="text-[9px] bg-cyber-purple/20 px-1.5 py-0.5 text-cyber-purple">JIT UI</span>
                  </div>
                  <p class="text-text-muted text-[10px] mb-2 truncate">Alvo: {msg.codeSnippet.targetFile}</p>
                  <button 
                    type="button"
                    onclick={() => msg.codeSnippet && projectToSubCanvas(msg.codeSnippet)} 
                    class="w-full py-1.5 bg-cyber-purple text-black font-bold hover:bg-white transition-colors flex items-center justify-center gap-1 text-xs"
                  >
                    PROJETAR NA BANCADA AO LADO
                  </button>
                </div>
              {/if}
            </div>
            <span class="font-mono text-[9px] text-text-muted">{msg.timestamp} | {msg.model || 'Gemini 2.5'}</span>
          </div>
        {/if}
      {/each}
    </div>

    <!-- Chat Input Bar -->
    <div class="p-3 bg-surface-mid border-t border-white/10 shrink-0">
      <form 
        onsubmit={(e) => { e.preventDefault(); handleSendMessage(); }} 
        class="flex items-center gap-2 bg-surface-low border border-white/10 p-2 focus-within:border-cyber-purple/60 transition-colors"
      >
        <svg class="w-4 h-4 text-text-muted shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="4 17 10 11 4 5" />
          <line x1="12" y1="19" x2="20" y2="19" />
        </svg>
        <input 
          bind:value={chatInputText} 
          type="text" 
          placeholder="Digite uma ordem socrática ou intenção (/bancada, /tarefas, /settings)..." 
          class="flex-1 bg-transparent text-xs font-mono text-text-main placeholder-text-muted outline-none border-none"
        />
        
        <button 
          type="button"
          onclick={() => isSubCanvasOpen = !isSubCanvasOpen} 
          class="px-2.5 py-1 bg-surface-high hover:bg-cyber-purple/20 text-cyber-purple border border-cyber-purple/30 font-mono font-bold text-xs transition-colors" 
          title="Alternar visibilidade do Sub-Canvas JIT"
        >
          {isSubCanvasOpen ? 'Ocultar Bancada' : 'Exibir Bancada'}
        </button>
        
        <button 
          type="submit"
          class="px-3 py-1 bg-cyber-purple/20 hover:bg-cyber-purple text-cyber-purple hover:text-black font-mono font-bold text-xs transition-colors flex items-center gap-1"
        >
          <span>DISPATCH</span>
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="22" y1="2" x2="11" y2="13" />
            <polygon points="22 2 15 22 11 13 2 9 22 2" />
          </svg>
        </button>
      </form>
    </div>
  </div>

  <!-- Dynamic Sub-Canvas / Bancada JIT (Right Split) -->
  {#if isSubCanvasOpen}
    <div class="w-[38%] h-full bg-void-black/90 border border-white/10 flex flex-col justify-between overflow-hidden transition-all duration-150 shrink-0">
      <div class="h-9 px-3 bg-surface-mid border-b border-white/5 flex items-center justify-between font-mono text-xs shrink-0">
        <div class="flex items-center gap-2 text-telemetry-cyan font-semibold">
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
          </svg>
          <span>BANCADA JIT // SANDBOX</span>
        </div>
        <div class="flex items-center gap-1">
          <button 
            type="button"
            onclick={onNavigateToBancada} 
            class="p-1 hover:text-cyber-purple transition-colors text-text-muted" 
            title="Expandir para Bancada Completa"
          >
            <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
            </svg>
          </button>
          <button 
            type="button"
            onclick={() => isSubCanvasOpen = false} 
            class="p-1 hover:text-alert-crimson transition-colors text-text-muted" 
            title="Fechar Sub-Canvas"
          >
            <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
      </div>

      <div class="flex-1 p-3 overflow-y-auto font-mono text-[11px] space-y-3 bg-[#070709]">
        <div class="p-2 bg-surface-mid border border-white/5 text-text-muted flex justify-between items-center text-[10px]">
          <span class="truncate max-w-[200px]">TARGET: <strong class="text-text-main">{currentSubCanvasCode.targetFile}</strong></span>
          <span class="text-emerald-400">{currentSubCanvasCode.tag}</span>
        </div>

        <pre class="p-3 bg-surface-low border border-white/5 text-text-main overflow-x-auto leading-relaxed whitespace-pre-wrap font-mono text-xs">{currentSubCanvasCode.code}</pre>
      </div>

      <div class="p-2 bg-surface-mid border-t border-white/5 flex justify-between items-center font-mono text-[10px] shrink-0">
        <span class="text-text-muted">STATUS: PRONTO PARA ENSAIO</span>
        <button 
          type="button"
          onclick={onNavigateToBancada} 
          class="px-2 py-1 bg-cyber-purple/20 hover:bg-cyber-purple text-cyber-purple hover:text-black font-bold transition-colors"
        >
          ABRIR NA BANCADA COMPLETA
        </button>
      </div>
    </div>
  {/if}
</div>
