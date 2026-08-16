# Tasks: PACOTE 1 — Resiliência e Coerção Contra Stubs

## Status Geral: COMPLETED (100/100)
- **Work Unit:** `feat-core-resilience`
- **Data Início:** 2026-08-16
- **Data Conclusão:** 2026-08-16
- **Responsável:** Engenheiro Bare-Metal de Sistemas SOULS

---

## 1. Definição de Tarefas & DoD (Definition of Done)

### [x] Task 1.1: Compliance Territorial e Design SDD
- **Objetivo:** Criar os documentos canônicos `design.md` e `tasks.md` em `docs/work-units/active/feat-core-resilience/`.
- **DoD:** Arquivos criados e aprovados conforme ADR-025 e WORKSPACE_MAP.
- **Evidência:** `docs/work-units/active/feat-core-resilience/design.md` e `tasks.md`.

### [x] Task 1.2: Cura do Loop OOM no `system.rs` e `context.rs`
- **Objetivo:** Inserir guarda inflexível contra `old_string.is_empty()` e `stub_marker.is_empty()`, impedindo alocações na RAM via `match_indices`.
- **DoD:** Retornar `RpcError` estruturado com `is_error: true` e código `-32602` de forma imediata. Preservar `PathLockManager` assíncrono.
- **Evidência:** Modificações em `handlers/system.rs` e `handlers/context.rs`.

### [x] Task 1.3: Imposição de Timeout Guilhotina de 30 Segundos no Despachante MCP
- **Objetivo:** Envolver a chamada assíncrona de `router::handle_tool_call` em `tokio::time::timeout(Duration::from_secs(30), ...)`.
- **DoD:** Abortar a tarefa e retornar JSON-RPC error sob namespace `souls_mcp` caso ultrapasse 30 segundos.
- **Evidência:** Modificação no método `handle_mcp` em `main.rs`.

### [x] Task 1.4: Blindagem de Fronteiras FFI com `catch_unwind`
- **Objetivo:** Proteger chamadas FFI externas contra panics não tratados usando `std::panic::catch_unwind(std::panic::AssertUnwindSafe(...))`.
- **DoD:** `safe_ffi_call` implementado e integrado em `llama_logit_probing.rs` e handlers FFI.
- **Evidência:** Modificações em `src/core/llama_logit_probing.rs` e `handlers/system.rs`.

### [x] Task 1.5: Suíte de Testes TDD Mandatória
- **Objetivo:** Adicionar testes `test_match_indices_empty_string_guard`, `test_mcp_tool_execution_timeout_guilhotina` e `test_ffi_panic_boundary_isolation` em `tests.rs`.
- **DoD:** `cargo test --bin souls_mcp_server` executando com Exit Code 0 absoluto e zero warnings do clippy.
- **Evidência:** Log de testes e clippy em `.souls_scratchpad/logs/cargo/clippy.log` (91/91 testes aprovados).
