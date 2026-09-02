---
spec: refactor-workspace-and-codebase-360
phase: 3-tasks
design: docs/work-units/active/refactor-workspace-and-codebase-360/design.md
---

# Tasks — Refatoração Holística 360° do Workspace e Codebase

## TASK-01 — Higiene Territorial da Raiz e .souls_scratchpad
- [x] Remover `.pytest_cache` da raiz e atualizar `.gitignore` para ignorar `.pytest_cache/`, `.pnpm-*` se aplicável
- [x] Mover logs soltos de `.souls_scratchpad/` para `.souls_scratchpad/logs/misc/`
- [x] Mover scripts efêmeros de `.souls_scratchpad/` para `.souls_scratchpad/scripts/`
- [x] Mover specs de `.souls_scratchpad/` para `docs/decisions/specs/` (incluindo `spec_ui_hibrida_windows11_rust.md`)
- [x] Mover relatórios e protótipos de `.souls_scratchpad/` para `.souls_scratchpad/reports/` e `.archive/`

## TASK-02 — Governança Canônica e Alinhamento de Docs
- [x] Consolidar scripts de runtime em `docs/runtime/scripts/`
- [x] Mover `docs/relatorio-reconhecimento-bare-metal-v2.txt` para `docs/observability/reports/`
- [x] Atualizar scripts utilitários (`souls_context_dumps_compiler.py`, `audit_workspace_compliance.py`)

## TASK-03 — Higiene do Frontend Svelte 5 & Contratos TypeScript
- [x] Remover diretório vazio `src/bin/`
- [x] Criar módulo de contratos DTO TypeScript em `src/lib/types/ipc.ts` e `src/lib/types/index.ts`
- [x] Validar imports e consistência com Svelte 5 Runes
- [x] Rodar build e validação de tipos TypeScript (`tsc --noEmit && vite build`) com 100% de sucesso

## TASK-04 — Desacoplamento e Modularização da Camada IPC (Rust)
- [x] Criar `src-tauri/src/ipc/commands/telemetry.rs` com os handlers `souls_ping` e `start_watchdog_stream`
- [x] Criar `src-tauri/src/ipc/commands/socratic.rs` com os handlers `socratic_export_session`, `socratic_analyze_session` e `socratic_merge_sessions`
- [x] Criar `src-tauri/src/ipc/mod.rs` e registrar `pub mod ipc;` em `lib.rs` (gated sob `feature = "tauri-app"`)
- [x] Limpar `main.rs` transferindo toda a lógica de comandos para a camada IPC e delegando ao `generate_handler!`

## TASK-05 — Modularização Física em 5 Subdomínios de src-tauri/src/core/
- [x] Criar e migrar 18 arquivos para `src-tauri/src/core/inference/` com `inference/mod.rs`
- [x] Criar e migrar 6 arquivos para `src-tauri/src/core/vram_hardware/` com `vram_hardware/mod.rs`
- [x] Criar e migrar 5 arquivos para `src-tauri/src/core/security/` com `security/mod.rs`
- [x] Criar e migrar 7 arquivos para `src-tauri/src/core/socratic/` com `socratic/mod.rs`
- [x] Criar e migrar 12 arquivos para `src-tauri/src/core/governance/` com `governance/mod.rs`
- [x] Reestruturar `src-tauri/src/core/mod.rs` com re-exports transparentes e zero warnings de compilação
- [x] Resolver colisões de nomes (`BitNetJobGuard` vs `WindowsJobGuard`)

## TASK-06 — Validação Final e Auditoria de Compliance
- [x] Rodar `cargo check -p souls_mc --features tauri-app` (Exit Code 0 em 15.68s, 0 warnings)
- [x] Rodar `cargo check --workspace` (Exit Code 0 em 37.23s)
- [x] Rodar `python docs/runtime/scripts/audit_workspace_compliance.py` (0 erros, 0 caminhos não-canônicos em 1308 arquivos)
- [x] Atualizar walkthrough e relatório de entrega
