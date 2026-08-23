<script lang="ts">
  // SOULS MC — Camada 5: Terminal Drawer (Engine Room & Logs)
  //
  // Stream de telemetria e logs assíncronos do runtime Tokio Rust.
  // Conformidade: ADR-001, ADR-005, ADR-038 (Compressão de Logs).

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  let logs = $state<Array<{ id: string; time: string; tag: string; tagColor: string; message: string }>>([
    { id: "l1", time: "16:42:00.001", tag: "[TOKIO-MAIN]", tagColor: "text-emerald-400", message: "Daemon SODA iniciado na thread #0 (AVX2 alocado)." },
    { id: "l2", time: "16:42:00.012", tag: "[IPC-ZEROCOPY]", tagColor: "text-telemetry-cyan", message: "Ring buffer alocado em 0x7FFF004 (64MB flatbuffers)." },
    { id: "l3", time: "16:42:01.045", tag: "[MODEL-MANAGER]", tagColor: "text-cyber-purple", message: "Scanner GGUF: 14 modelos mapeados via mmap2." },
    { id: "l4", time: "16:42:02.110", tag: "[L7-SHIELD]", tagColor: "text-emerald-400", message: "Gateway souls_mcp online na porta 3000." },
  ]);
</script>

{#if isOpen}
  <div 
    id="terminal-drawer" 
    class="h-64 w-full bg-[#070709] border-t border-cyber-purple/30 font-mono text-xs flex flex-col justify-between shadow-2xl transition-all duration-200 shrink-0 z-40 select-text"
  >
    <!-- Header Engine Room -->
    <div class="h-8 px-4 bg-surface-mid border-b border-white/10 flex items-center justify-between text-text-muted select-none shrink-0">
      <div class="flex items-center gap-3">
        <span class="text-telemetry-cyan font-bold flex items-center gap-1.5 text-xs">
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          ENGINE ROOM // RUST TOKIO TELEMETRY STREAM
        </span>
        <span class="text-[10px] bg-white/5 px-2 py-0.5 border border-white/5">IPC: ZERO-COPY FLATBUFFERS</span>
      </div>
      <button 
        type="button"
        onclick={onClose} 
        class="hover:text-text-main transition-colors p-1"
        title="Fechar Gaveta"
      >
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>

    <!-- Logs Stream Buffer -->
    <div class="flex-1 p-3 overflow-y-auto space-y-1 text-[11px] text-text-muted font-mono">
      {#each logs as log (log.id)}
        <div class="leading-relaxed">
          <span class="text-white/30">{log.time}</span>
          <span class="{log.tagColor} font-semibold">{log.tag}</span>
          <span class="text-text-main">{log.message}</span>
        </div>
      {/each}
    </div>

    <!-- Footer Bar -->
    <div class="h-6 px-4 bg-surface-low border-t border-white/5 flex items-center justify-between text-[10px] text-text-muted select-none shrink-0">
      <span>BUFFER: 1,024 LINES | LOGGING: STDERR ONLY</span>
      <span class="text-emerald-400">EXIT CODE 0 (OK)</span>
    </div>
  </div>
{/if}
