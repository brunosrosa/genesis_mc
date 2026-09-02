// =============================================================================
// SOULS MC — TypeScript IPC & DTO Contracts (Svelte 5 Runes Architecture)
// Conformidade: ADR-001, ADR-005 (Frontend Passivo), ADR-014 (Zero-Copy)
// =============================================================================

export type CockpitView = "chat" | "bancada" | "graph" | "kanban" | "governance" | "inbox";

export interface SocraticSessionMetadata {
  sessionId: string;
  format?: "json" | "markdown" | "raw";
}

export interface SocraticSessionMetrics {
  totalThoughts: number;
  epistemicCoherence: number;
  tokensConsumed: number;
  estimatedCostUsd: number;
}

export interface WatchdogState {
  vramUsedMb: number;
  vramTotalMb: number;
  thermalZone: "optimal" | "elevated" | "critical" | "throttled";
  activeInferenceEngine?: string;
  isPaused: boolean;
}

export interface BlastRadiusEvent {
  actionId: string;
  riskLevel: "low" | "medium" | "high" | "critical";
  description: string;
  requiresHitlApproval: boolean;
}
