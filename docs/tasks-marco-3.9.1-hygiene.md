# Tasks — Marco 3.9.1: Faxina de Higiene + Detecção de Backpressure

> Emenda ao [tasks-marco-3.9-e2-hardening.md](tasks-marco-3.9-e2-hardening.md).
> Rito: SDD estrito (Red → Green → Refactor).

## Task 1 — Extinção de `init_state_db_for_testing` + `TEST_STATE_DB_TX_OVERRIDE` (P1+P3)
**DoD:**
- [x] Deletar `init_state_db_for_testing` (L2958-3017) de `bin/souls_mcp_server.rs`.
- [x] Deletar `TEST_STATE_DB_TX_OVERRIDE` (L2951-2953).
- [x] `try_log_telemetry` atualizado para não mais referenciar o override.
- [x] `grep -r init_state_db_for_testing src/` retorna 0.
- [x] `grep -r TEST_STATE_DB_TX_OVERRIDE src/` retorna 0.
- [x] Warning `clippy::await_holding_lock` desaparece automaticamente.
- [x] `cargo check --bin souls_mcp_server --no-default-features` verde.

## Task 2 — Centralização dos helpers socráticos (P2)
**DoD:**
- [x] Criar `cognition::thinking::test_helpers` (submódulo sob `#[cfg(test)]`).
- [x] Mover `build_socratic_tree` e `render_socratic_markdown` do bin para o submódulo.
- [x] Re-exportar via `pub use test_helpers::{build_socratic_tree, render_socratic_markdown};`
      apenas em `#[cfg(test)]` no `mod.rs`.
- [x] T2 (test_export_session_formatting) verde: importa do novo submódulo.
- [x] `open_socratic_state_db` PERMANECE no bin (usado por T-bootstrap, depende de `RpcError`).
- [x] `cargo test --bin souls_mcp_server test_export_session_formatting` verde.

## Task 3 — Instrumentação da telemetria de backpressure (P4)
**DoD:**
- [x] Criar `try_log_socratic_backpressure()` em `bin/souls_mcp_server.rs`:
  - Lê `socratic_handle().is_under_backpressure()`.
  - Se `true`: `try_log_telemetry("socratic_backpressure_active", 0, 0, 0.0, 0, 0.0)`.
  - Se `false`: `try_log_telemetry("socratic_backpressure_inactive", 0, 0, 0.0, 0, 1.0)`.
- [x] Chamar a função no **início** de `run_souls_merge_sessions` (após o check
      do handle, antes do loop de despacho).
- [x] Teste: validar que a métrica aparece em `telemetry_logs` após uma chamada
      de `merge_sessions` em ambiente de teste.
- [x] `cargo test --bin souls_mcp_server` 41/41 verde.

## Task 4 — Cura `tools/list` (verificação de duplicatas)
**DoD:**
- [x] `grep -n souls_stub_fill bin/souls_mcp_server.rs` deve mostrar APENAS:
  - L778 (alias de back-compat no dispatcher)
  - L5550 (assertiva de teste de ausência)
- [x] `grep -n lean-ctx bin/souls_mcp_server.rs` = 0 (zero stubs legados).
- [x] `grep -n stub_not_implemented_yet` listado e validado.
- [x] Documentar no commit message: "tools/list já está limpo per os 2 testes
      `tools_list_fill_unico_sem_duplicata` e `server_info_name_is_souls_mcp`".

## Task 5 — Validação Fast Pass
**DoD:**
- [x] `cargo test --bin souls_mcp_server --no-default-features` 41/41 verde em < 12s.
- [x] `cargo check --bin souls_mcp_server --no-default-features` sem warnings novos.
- [x] `cargo check --bin souls_mc --features tauri-app` verde (Tauri IPC ainda compila).
- [x] `grep -r MARCO_39_FASE_E_LOCK src/` = 0 (regressão preexistente).
- [x] `grep -r init_state_db_for_testing src/` = 0.
- [x] `grep -r TEST_STATE_DB_TX_OVERRIDE src/` = 0.
- [x] Blast Radius documentado.
- [x] Commit + push + PR via `gh pr create` para HITL.
