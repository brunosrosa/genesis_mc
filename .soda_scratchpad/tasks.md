---
title: "Tasks de Higiene (Pivotagem / Fase 1.5)"
status: "ativo"
---

## Sequência

1. Criar/atualizar estado documental:
   - **docs/SODA_CURRENT_STATE.md**
   - Atualizar **docs/milestones/PRD_MILESTONE_01.md** com nota de hibernação e referência ao estado atual.
2. Higienizar `FinOpsRouter`:
   - remover paths hardcoded;
   - ajustar smoke tests para usar paths relativos do repo;
   - garantir compatibilidade Linux/CI.
3. Consertar registro do IPC:
   - `src-tauri/src/main.rs` deve registrar `invoke_handler` (ex.: `genesis_ping`) sem quebrar o bootstrap do AgentGateway.
4. Expurgo do React:
   - remover arquivos e dependências React;
   - preparar scaffold mínimo Svelte 5 + Vite.

## Validação Obrigatória

- `cargo test finops::phase1_5`
- `cargo clippy -- -D warnings`

