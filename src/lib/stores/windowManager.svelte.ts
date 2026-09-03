/**
 * Window Manager Store - Svelte 5 Runes
 * Gerencia o espaço de trabalho do Desktop Overlay (posicionamento espacial,
 * z-index, arraste fluido e visibilidade dos painéis flutuantes).
 */

export type WindowId = "settings" | "agent_task" | "music" | "terminal";

export interface WindowState {
  id: WindowId;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  isOpen: boolean;
  isMinimized: boolean;
  zIndex: number;
}

class WindowManager {
  private highestZ = $state(10);

  windows = $state<Record<WindowId, WindowState>>({
    settings: {
      id: "settings",
      title: "Settings // Kernel & Metrics",
      x: 80,
      y: 50,
      width: 640,
      height: 680,
      isOpen: true,
      isMinimized: false,
      zIndex: 1,
    },
    agent_task: {
      id: "agent_task",
      title: "Search for the latest news about open source AI models released this week",
      x: 740,
      y: 110,
      width: 580,
      height: 620,
      isOpen: true,
      isMinimized: false,
      zIndex: 2,
    },
    music: {
      id: "music",
      title: "Music",
      x: 1060,
      y: 35,
      width: 250,
      height: 100,
      isOpen: true,
      isMinimized: false,
      zIndex: 3,
    },
    terminal: {
      id: "terminal",
      title: "Bare-Metal Terminal // Sandbox",
      x: 320,
      y: 200,
      width: 720,
      height: 480,
      isOpen: false,
      isMinimized: false,
      zIndex: 4,
    },
  });

  bringToFront(id: WindowId) {
    this.highestZ += 1;
    if (this.windows[id]) {
      this.windows[id].zIndex = this.highestZ;
      if (this.windows[id].isMinimized) {
        this.windows[id].isMinimized = false;
      }
    }
  }

  toggleWindow(id: WindowId) {
    if (!this.windows[id]) return;
    if (!this.windows[id].isOpen) {
      this.windows[id].isOpen = true;
      this.windows[id].isMinimized = false;
      this.bringToFront(id);
    } else if (this.windows[id].isMinimized) {
      this.windows[id].isMinimized = false;
      this.bringToFront(id);
    } else {
      this.windows[id].isOpen = false;
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

  setPosition(id: WindowId, x: number, y: number) {
    if (this.windows[id]) {
      this.windows[id].x = Math.max(10, Math.min(window.innerWidth - 100, x));
      this.windows[id].y = Math.max(10, Math.min(window.innerHeight - 80, y));
    }
  }

  resetLayout() {
    this.windows.settings.x = 80;
    this.windows.settings.y = 50;
    this.windows.settings.isOpen = true;
    this.windows.settings.isMinimized = false;

    this.windows.agent_task.x = 740;
    this.windows.agent_task.y = 110;
    this.windows.agent_task.isOpen = true;
    this.windows.agent_task.isMinimized = false;

    this.windows.music.x = 1060;
    this.windows.music.y = 35;
    this.windows.music.isOpen = true;
    this.windows.music.isMinimized = false;
  }
}

export const windowManager = new WindowManager();
