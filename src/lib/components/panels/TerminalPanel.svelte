<script lang="ts">
  import MacWindowFrame from "$lib/components/ui/MacWindowFrame.svelte";
  import TerminalDrawer from "$lib/components/TerminalDrawer.svelte";
</script>

<MacWindowFrame id="terminal" title="Bare-Metal Terminal // Sandbox" width={720} height={460}>
  <div class="flex-1 overflow-hidden p-2 flex flex-col bg-black/60 font-mono text-xs">
    <TerminalDrawer isOpen={true} />
  </div>
</MacWindowFrame>
