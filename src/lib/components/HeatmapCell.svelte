<script lang="ts">
  // Svelte 5 Runes — props tipadas via $props().
  interface Props {
    severity: number; // 0..1
    path: string;
  }
  const { severity, path }: Props = $props();

  // Clamp defensivo (severidade inválida não pode quebrar a UI).
  const safe = $derived(Math.max(0, Math.min(1, severity)));

  // Cor interpolada em OKLCH: chroma fixo, hue roda do verde (142)
  // ao vermelho (25) conforme severity sobe.
  const bg = $derived(`oklch(0.42 ${0.10 + safe * 0.15} ${142 - safe * 117})`);
  const ring = $derived(`oklch(0.65 ${0.18 + safe * 0.10} ${142 - safe * 117})`);
</script>

<div
  class="heatmap-cell"
  style:background={bg}
  style:box-shadow={`inset 0 0 0 1px ${ring}`}
  title={path}
  aria-label={`Impact: ${path} severity ${(safe * 100).toFixed(0)}%`}
>
  <span class="path">{path.split("/").pop() ?? path}</span>
</div>

<style>
  .heatmap-cell {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.7rem;
    line-height: 1;
    color: oklch(0.98 0 0);
    contain: layout paint;
    will-change: background;
    transition: background 250ms ease-out, box-shadow 250ms ease-out;
  }
  .path {
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
    max-width: 100%;
  }
</style>
