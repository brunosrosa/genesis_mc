<script lang="ts">
  // SOULS MC — Marco V: Agent Inbox (Gaveta Lateral Passiva).
  //
  // Cumpre ADR-014 (Fricção Produtiva): invisível durante trabalho
  // profundo, habita uma gaveta lateral sutil que aparece SÓ quando
  // há um `pendingBlast` aguardando decisão HITL.
  //
  // ## Anatomia
  // 1. Header com telemetria passiva (Ghost Border animada).
  // 2. Matriz Térmica (heatmap CSS grid de HeatmapCell).
  // 3. Myers Diff em fonte mono dentro de container `contain: layout size`.
  // 4. Glow Slider — único controle tátil. Arraste único despacha HITL.

  import HeatmapCell from "./HeatmapCell.svelte";
  import {
    pendingBlast,
    dispatch_blast_decision,
  } from "$lib/stores/blast.svelte.ts";
  import { telemetry, thermal_status } from "$lib/stores/telemetry.svelte.ts";

  let gauge = $state(50);
  let isDragging = $state(false);

  // A gaveta só aparece quando há pendingBlast (Fricção Produtiva).
  const isOpen = $derived(pendingBlast.report !== null);

  // Ghost border dinâmico: muda cor conforme estado do daemon.
  const ghostClass = $derived(
    thermal_status() === "PRESSAO_CRITICA"
      ? "ghost-border ghost-border--compiling"
      : isOpen
        ? "ghost-border ghost-border--thinking"
        : "ghost-border ghost-border--idle"
  );

  // Cor do glow do slider proporcional ao gauge (CSS variable, GPU-friendly).
  const glowStyle = $derived(`--value: ${gauge}%`);

  function handleSliderInput(e: Event) {
    const target = e.currentTarget as HTMLInputElement;
    gauge = Number(target.value);
    isDragging = true;
  }

  function handleSliderCommit() {
    if (!pendingBlast.report) return;
    isDragging = false;
    void dispatch_blast_decision(pendingBlast.report.target, gauge);
  }
</script>

{#if isOpen && pendingBlast.report}
  <aside
    class="agent-inbox {ghostClass}"
    class:open={isOpen}
    aria-label="Agent Inbox — Blast Radius pending HITL"
  >
    <header class="header">
      <div class="title-row">
        <span class="dot" aria-hidden="true"></span>
        <h2 class="title">Blast Radius Pending</h2>
        <span class="meta">{pendingBlast.report.affected_files.length} files · depth {pendingBlast.report.depth}</span>
      </div>
      <div class="telemetry">
        <span>VRAM {telemetry.vram_mb} MB</span>
        <span>RAM {telemetry.ram_mb} MB</span>
        <span>CPU {telemetry.cpu_temp.toFixed(1)} °C</span>
        <span>GPU {telemetry.gpu_temp.toFixed(1)} °C</span>
      </div>
    </header>

    <section class="matrix" aria-label="Blast Radius Heatmap">
      {#each pendingBlast.report.affected_files as node (node.path)}
        <HeatmapCell severity={node.severity} path={node.path} />
      {/each}
    </section>

    <section class="diff" aria-label="Myers Diff preview">
      <div class="diff-pane">
        <div class="diff-line add">+ {pendingBlast.report.target} (added in plan)</div>
        <div class="diff-line rem">- (replaced via ImpactReport BFS reverse)</div>
        <div class="diff-line ctx">  affected: {pendingBlast.report.affected_files.length}</div>
        <div class="diff-line ctx">  edges:    {pendingBlast.report.edge_count}</div>
      </div>
    </section>

    <footer class="footer">
      <label class="slider-label" for="blast-glow-slider">
        Approval Gauge (0 = reject · 100 = apply)
      </label>
      <input
        id="blast-glow-slider"
        type="range"
        min="0"
        max="100"
        step="1"
        value={gauge}
        class="glow-slider"
        style={glowStyle}
        oninput={handleSliderInput}
        onchange={handleSliderCommit}
        onpointerup={handleSliderCommit}
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={gauge}
      />
      <div class="gauge-readout">
        <span class="gauge-value">{gauge}</span>
        <span class="gauge-status">
          {#if gauge === 0}REJECT{:else if gauge === 100}APPLY{:else if isDragging}DRAG…{:else}PARTIAL{/if}
        </span>
      </div>
    </footer>
  </aside>
{/if}

<style>
  .agent-inbox {
    position: fixed;
    top: 1rem;
    right: 1rem;
    bottom: 1rem;
    width: min(420px, 92vw);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem;
    overflow-y: auto;
    transform: translateX(calc(100% + 2rem));
    transition: transform 250ms cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 40;
    font-family: var(--font-sans, system-ui, sans-serif);
    color: oklch(0.95 0 0);
    contain: layout size;
  }
  .agent-inbox.open {
    transform: translateX(0);
  }

  .header {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 999px;
    background: oklch(0.75 0.18 296);
    box-shadow: 0 0 8px oklch(0.75 0.18 296);
  }
  .title {
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    margin: 0;
  }
  .meta {
    margin-left: auto;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.7rem;
    opacity: 0.6;
  }
  .telemetry {
    display: flex;
    gap: 0.75rem;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.7rem;
    opacity: 0.75;
  }

  .matrix {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.25rem;
    padding: 0.5rem;
    border: 1px solid oklch(0.25 0 0);
    border-radius: 0.375rem;
    contain: layout paint;
  }

  .diff {
    contain: layout size;
  }
  .diff-pane {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.72rem;
    line-height: 1.5;
    background: oklch(0.08 0 0);
    border-radius: 0.375rem;
    padding: 0.5rem 0.75rem;
    contain: layout size;
  }
  .diff-line.add { color: oklch(0.78 0.18 145); }
  .diff-line.rem { color: oklch(0.78 0.18 25); }
  .diff-line.ctx { color: oklch(0.7 0 0); opacity: 0.7; }

  .footer {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid oklch(0.2 0 0);
  }
  .slider-label {
    font-size: 0.7rem;
    opacity: 0.7;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .gauge-readout {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.85rem;
  }
  .gauge-value { color: oklch(0.85 0.18 296); }
  .gauge-status { opacity: 0.7; }
</style>
