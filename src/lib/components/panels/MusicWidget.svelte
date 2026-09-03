<script lang="ts">
  import MacWindowFrame from "$lib/components/ui/MacWindowFrame.svelte";

  let isPlaying = $state(true);
  let trackName = $state("Ambient Cognition // Neural Flow");
  let artist = $state("SODA Soundscape");
</script>

<MacWindowFrame id="music" title="Music" width={260} height={110} showTrafficLights={false}>
  <div class="flex items-center gap-3 p-3 text-neutral-200">
    <!-- Ícone de Música Verde com Glow -->
    <div class="w-10 h-10 rounded-lg bg-emerald-950/80 border border-emerald-500/30 flex items-center justify-center shrink-0 shadow-[0_0_12px_rgba(16,185,129,0.25)]">
      <svg class="w-5 h-5 text-emerald-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M9 18V5l12-2v13" />
        <circle cx="6" cy="18" r="3" />
        <circle cx="18" cy="16" r="3" />
      </svg>
    </div>

    <!-- Informações da Faixa -->
    <div class="flex-1 min-w-0">
      <span class="text-[9px] font-mono uppercase tracking-wider text-neutral-400 block">
        Now Playing
      </span>
      <p class="text-xs font-medium text-white truncate">
        {trackName}
      </p>
      <p class="text-[10px] text-neutral-400 truncate">
        {artist}
      </p>
    </div>

    <!-- Mini Controle de Play / Pause -->
    <button
      type="button"
      class="p-1.5 rounded-full hover:bg-white/10 text-neutral-300 hover:text-white transition-colors"
      onclick={() => { isPlaying = !isPlaying; }}
    >
      {#if isPlaying}
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/></svg>
      {:else}
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"/></svg>
      {/if}
    </button>
  </div>
</MacWindowFrame>
