---
spec: refactor-workspace-and-codebase-360
phase: 2-design
work_unit: docs/work-units/active/refactor-workspace-and-codebase-360/
---

# Design — Refatoração Holística 360° do Workspace e Codebase

## 1. Contexto e Motivação
O SOULS MC passou por múltiplos marcos de evolução rápida, gerando acúmulo de arquivos efêmeros fora de zonas delimitadas, arquivos órfãos em `.souls_scratchpad`, scripts de fábrica dispersos e um crescimento monolítico em `src-tauri/src/core/` (48 arquivos).
Este documento desenha a arquitetura de limpeza e modularização para atingir 100% de compliance com o `_WORKSPACE_MAP.md` (v6.0) e as leis de FinOps e Bare-Metal do SOULS.

## 2. Decisões Arquiteturais e Zonas

### 2.1. ZONA 1 & 2: Higiene Territorial e Workspace
- Remover `.pytest_cache` da raiz e reforçar `.gitignore`.
- Mover os 26 arquivos soltos de `.souls_scratchpad` para suas 5 subzonas oficiais:
  - `logs/misc/` e `logs/cargo/`: Logs de MCP e profiling.
  - `scripts/`: Scripts efêmeros `.py` e `.ps1`.
  - `reports/` e `.archive/`: Relatórios de auditoria e protótipos html.
- Mover specs preliminares `soda-mc-*-spec.md` para `docs/decisions/specs/`.

### 2.2. ZONA 4: Modularização do Backend Rust (`src-tauri`)
- Organizar `src-tauri/src/core/` em submódulos coesos:
  - `inference/`: `llama_engine.rs`, `mistral_engine.rs`, `mistral_sidecar.rs`, `bitnet_engine.rs`, `bitnet_daemon.rs`, `burn_engine.rs`, `gliclass_engine.rs`, `pulp_matrix_engine.rs`, `ort_scorer.rs`, `engine_trait.rs`, `inference_adapter.rs`, `model_registry.rs`, `model_manager.rs`, `gigatoken.rs`, `gigatoken_encoder.rs`, `llama_logit_probing.rs`, `llama_lora_adapter.rs`, `llama_upstream_engine.rs`.
  - `vram_hardware/`: `vram_scheduler.rs`, `vram_scheduler/`, `hardware_watchdog.rs`, `hardware_profiler.rs`, `peak_ewma.rs`, `headroom_engine.rs`.
  - `security/`: `sandbox.rs`, `subprocess_guard.rs`, `file_locker.rs`, `pii_redactor.rs`, `l7_shield.rs`.
  - `socratic/`: `socratic_event_bus.rs`, `socratic_thought_stream.rs`, `socratic_interrupt.rs`, `socratic_cli.rs`, `epistemic_prober.rs`, `terminal_drawer_stream.rs`.
  - `governance/`: `gateway_config.rs`, `sdd.rs`, `drift_sentinel.rs`, `cohomology.rs`, `response_healing.rs`, `late_binding_router.rs`, `sticky_router.rs`, `mcp_transport.rs`, `ipc_bridge.rs`, `telemetry_dispatcher.rs`, `chyros_daemon.rs`, `v3_ignition_tests.rs`.
- Re-exportar todos os tipos públicos em `src-tauri/src/core/mod.rs` para preservar 100% de retrocompatibilidade com chamadas externas (`main.rs`, testes, CLIs).
- Consolidar `telemetry.rs` vs `telemetry/` e erradicar warnings de compilação.

### 2.3. ZONA 5: Frontend Svelte 5
- Deletar pasta vazia `src/bin/`.
- Garantir alinhamento de componentes com Svelte 5 Runes.
- Executar `pnpm check`.

### 2.4. ZONA 3: Governança e Canon
- Mover scripts utilitários de `docs/runtime/scripts/` para `docs/runtime/scripts/`.
- Mover `docs/relatorio-reconhecimento-bare-metal-v2.txt` para `docs/observability/reports/`.
- Validar via `python docs/runtime/scripts/audit_workspace_compliance.py`.
