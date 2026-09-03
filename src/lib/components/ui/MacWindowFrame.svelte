<script lang="ts">
  import type { Snippet } from "svelte";
  import { windowManager, type WindowId } from "$lib/stores/windowManager.svelte.ts";

  interface Props {
    id?: WindowId;
    title: string;
    width?: number;
    height?: number;
    isFloating?: boolean;
    children?: Snippet;
    headerExtra?: Snippet;
    showTrafficLights?: boolean;
    class?: string;
    onClose?: () => void;
    onMinimize?: () => void;
    onExpand?: () => void;
  }

  let {
    id = "settings",
    title,
    width,
    height,
    isFloating = false,
    children,
    headerExtra,
    showTrafficLights = true,
    class: customClass = "",
    onClose,
    onMinimize,
    onExpand,
  }: Props = $props();

  const winState = $derived(windowManager.windows[id]);

  let isDragging = $state(false);
  let dragOffset = { x: 0, y: 0 };

  function handlePointerDown(e: PointerEvent) {
    if (!isFloating) return;
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest("button") || target.closest("input") || target.closest("select")) return;

    windowManager.bringToFront(id);
    isDragging = true;
    dragOffset.x = e.clientX - (winState?.x || 0);
    dragOffset.y = e.clientY - (winState?.y || 0);

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    const newX = e.clientX - dragOffset.x;
    const newY = e.clientY - dragOffset.y;
    windowManager.setPosition(id, newX, newY);
  }

  function handlePointerUp() {
    isDragging = false;
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", handlePointerUp);
  }

  function handleFrameClick() {
    if (isFloating) {
      windowManager.bringToFront(id);
    }
  }

  function handleCloseClick() {
    if (onClose) onClose();
    else windowManager.closeWindow(id);
  }

  function handleMinimizeClick() {
    if (onMinimize) onMinimize();
    else windowManager.minimizeWindow(id);
  }

  function handleExpandClick() {
    if (onExpand) onExpand();
    else if (isFloating) windowManager.bringToFront(id);
  }
</script>

{#if !isFloating || (winState && winState.isOpen)}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="{isFloating ? 'fixed' : 'flex-1 w-full h-full'} flex flex-col macos-glass overflow-hidden select-none transition-shadow duration-200 {customClass}"
    style={isFloating && winState
      ? `left: ${winState.x}px; top: ${winState.y}px; width: ${width || winState.width}px; height: ${winState.isMinimized ? 40 : (height || winState.height)}px; z-index: ${winState.zIndex};`
      : ""
    }
    onpointerdown={handleFrameClick}
  >
    <!-- Header / Barra de Título macOS com Traffic Lights -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <header
      class="h-9 px-3.5 flex items-center justify-between border-b border-white/[0.08] {isFloating ? 'cursor-grab active:cursor-grabbing' : ''} bg-white/[0.02] shrink-0"
      onpointerdown={handlePointerDown}
    >
      <div class="flex items-center gap-2.5">
        {#if showTrafficLights}
          <div class="flex items-center gap-2">
            <button
              type="button"
              aria-label="Close"
              class="macos-traffic-light traffic-light-red"
              onclick={handleCloseClick}
            ></button>
            <button
              type="button"
              aria-label="Minimize"
              class="macos-traffic-light traffic-light-yellow"
              onclick={handleMinimizeClick}
            ></button>
            <button
              type="button"
              aria-label="Maximize"
              class="macos-traffic-light traffic-light-green"
              onclick={handleExpandClick}
            ></button>
          </div>
        {/if}

        <div class="flex items-center gap-2 ml-1 text-xs font-sans text-neutral-300 font-medium truncate max-w-[360px]">
          <span class="truncate">{title}</span>
        </div>
      </div>

      {#if headerExtra}
        <div class="flex items-center gap-2">
          {@render headerExtra()}
        </div>
      {/if}
    </header>

    <!-- Corpo da Janela -->
    {#if !isFloating || !winState?.isMinimized}
      <main class="flex-1 overflow-auto flex flex-col">
        {#if children}
          {@render children()}
        {/if}
      </main>
    {/if}
  </div>
{/if}
