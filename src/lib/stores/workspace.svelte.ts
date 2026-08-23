// SOULS MC — SODA Canvas: Workspace & Focus Rack Store (Svelte 5 Runes)
// Conformidade: ADR-005 (Frontend Passivo), ADR-014 (Fricção Produtiva), ADR-041.

export interface CognitiveWorkspace {
  id: string;
  icon: string;
  title: string;
  description: string;
}

export interface FocusRackSlot {
  id: string;
  viewId: "chat" | "bancada" | "memory" | "tasks" | "settings" | "inbox" | "telemetry";
  title: string;
  subtitle: string;
  icon: string;
  color: string;
}

export const WORKSPACES: CognitiveWorkspace[] = [
  { id: "life", icon: "🌿", title: "Vida & Rotinas", description: "Hiperfocos, saúde circadiana e hábitos atômicos" },
  { id: "finance", icon: "💰", title: "Finanças Pessoais", description: "Planejamento orçamentário e SSOT" },
  { id: "research", icon: "🔬", title: "Pesquisas & Hiperfocos", description: "Exploração e síntese com NotebookLM" },
  { id: "engineering", icon: "⚡", title: "Engenharia & Projetos", description: "Desenvolvimento de software e arquitetura SOULS" }
];

class WorkspaceStore {
  activeWorkspace = $state<CognitiveWorkspace>(WORKSPACES[0]);
  focusSlots = $state<FocusRackSlot[]>([
    { id: "s1", viewId: "chat", title: "Diálogo Socrático", subtitle: "Master Soul Ativo", icon: "chat_bubble", color: "text-cyber-purple" },
    { id: "s2", viewId: "bancada", title: "Bancada de Testes", subtitle: "Sandbox Zero-Copy", icon: "construction", color: "text-telemetry-cyan" },
    { id: "s3", viewId: "memory", title: "Caderno & Grafo", subtitle: "LanceDB + Ladybug", icon: "menu_book", color: "text-emerald-400" },
  ]);

  setWorkspace(ws: CognitiveWorkspace) {
    this.activeWorkspace = ws;
  }

  setWorkspaceById(id: string) {
    const found = WORKSPACES.find(w => w.id === id);
    if (found) this.activeWorkspace = found;
  }
}

export const workspaceStore = new WorkspaceStore();
