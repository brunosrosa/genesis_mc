/**
 * Window Manager Store - Svelte 5 Runes
 * Gerencia o estado das janelas espaciais e modais no Desktop OS Overlay do SOULS MC.
 */

export type WindowId = "workspace" | "telemetry_dashboard";

export interface WindowState {
  id: WindowId;
  title: string;
  isOpen: boolean;
  isMinimized: boolean;
  zIndex: number;
}

class WindowManager {
  private highestZ = $state(10);

  windows = $state<Record<WindowId, WindowState>>({
    workspace: {
      id: "workspace",
      title: "Active Cognitive Workspace",
      isOpen: true,
      isMinimized: false,
      zIndex: 1,
    },
    telemetry_dashboard: {
      id: "telemetry_dashboard",
      title: "Telemetry & Hardware Dashboard",
      isOpen: false,
      isMinimized: false,
      zIndex: 2,
    },
  });

  bringToFront(id: WindowId) {
    this.highestZ += 1;
    if (this.windows[id]) {
      this.windows[id].zIndex = this.highestZ;
      this.windows[id].isMinimized = false;
    }
  }

  toggleWindow(id: WindowId) {
    if (!this.windows[id]) return;
    this.windows[id].isOpen = !this.windows[id].isOpen;
    if (this.windows[id].isOpen) {
      this.bringToFront(id);
    }
  }

  minimizeWindow(id: WindowId) {
    if (this.windows[id]) {
      this.windows[id].isMinimized = !this.windows[id].isMinimized;
    }
  }

  closeWindow(id: WindowId) {
    if (this.windows[id]) {
      this.windows[id].isOpen = false;
    }
  }
}

export const windowManager = new WindowManager();
