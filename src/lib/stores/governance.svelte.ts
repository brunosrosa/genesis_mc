// SOULS MC — SODA Canvas: Governança, FinOps & Safety Kill-Switch Store (Svelte 5 Runes)
// Conformidade: ADR-008 (FinOps), ADR-011 (Governanca HITL), ADR-027 (Termodinâmica VRAM).

export type GovernanceMode = "HOTL" | "HITL";

class GovernanceStore {
  mode = $state<GovernanceMode>("HOTL");
  isKillSwitchActive = $state(false);
  totalUsd = $state(0.04);
  totalTokens = $state(12400);
  monthlyCapUsd = $state(10.00);

  toggleMode() {
    this.mode = this.mode === "HOTL" ? "HITL" : "HOTL";
  }

  triggerKillSwitch() {
    this.isKillSwitchActive = true;
  }

  resetKillSwitch() {
    this.isKillSwitchActive = false;
  }

  recordUsage(tokens: number, costEstimateUsd = 0.0001) {
    this.totalTokens += tokens;
    this.totalUsd += costEstimateUsd;
  }
}

export const governanceStore = new GovernanceStore();
