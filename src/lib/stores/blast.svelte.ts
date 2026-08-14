// SOULS MC — Marco V: Svelte 5 Runes Store do Blast Radius.
//
// Recebe o `ImpactReport` emitido pelo backend Rust via evento Tauri
// `blast_radius_pending` e o expõe como Runa para os componentes
// `AgentInbox` e `HeatmapCell` consumirem de forma reativa.
//
// ## Por que evento (e não Channel)?
//
// O `ImpactReport` é um payload **discreto e raro** (não contínuo).
// A "alfândega" do JSON aqui é o control plane (ADR-003 §32-36),
// não o Data Plane. O Data Plane estritamente binário fica em
// `telemetry.svelte.ts`.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

// Tipo canônico do `ImpactReport` (espelha `cognition::ast::repo_impact::ImpactReport`).
// Mantido minimalista para evitar acoplamento forte com a estrutura Rust.
export interface ImpactNode {
  path: string;
  severity: number; // 0..1 (0 = trivial, 1 = crítico)
  reason: string;
}

export interface ImpactReport {
  target: string;
  affected_files: ImpactNode[];
  edge_count: number;
  depth: number;
  generated_at_epoch_secs: number;
}

// Runa reativa (Svelte 5): null = sem pending, objeto = há um plano aguardando HITL.
export const pendingBlast = $state<{ report: ImpactReport | null }>({ report: null });

/**
 * Inicia o listener do evento `blast_radius_pending`. Retorna uma função
 * de cleanup que cancela o `unlisten`.
 *
 * Idempotente: chamar 2× registra 2 listeners (use cleanup antes de
 * re-chamar para evitar vazamento).
 */
export async function listen_for_blast_radius(): Promise<UnlistenFn> {
  return await listen<ImpactReport>("blast_radius_pending", (event) => {
    pendingBlast.report = event.payload;
  });
}

/**
 * Despacha a decisão HITL para o backend. `0` = rejeição total,
 * `100` = aceitação total, valores intermediários = aceitação parcial
 * (executa apenas arquivos com `severity <= value / 100`).
 */
export async function dispatch_blast_decision(
  plan_id: string,
  decision: number
): Promise<void> {
  if (decision <= 0) {
    await invoke("reject_blast_radius", { planId: plan_id });
  } else {
    await invoke("approve_blast_radius", {
      planId: plan_id,
      approvalGauge: decision,
    });
  }
  pendingBlast.report = null;
}
