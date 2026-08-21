# Tarefas: Operação Extirpação de Slop — Materialização Real dos 3 Motores

**Work Unit:** `feat-mcp-perf-hygiene` (v2.0)
**Status:** COMPLETED (Exit Code 0, 103/103 tests passing, zero clippy warnings)
**Data:** 2026-08-20
**Modo de Execução:** Red-Green-Refactor (TDD Atômico por tarefa)

---

## Convenção de Definition of Done (DoD)

Toda tarefa abaixo só é considerada **CONCLUÍDA** quando atende **simultaneamente**:

1. **Código escrito** em arquivo físico do repositório (não pseudo-código).
2. **Teste de regressão** adicionado em `src-tauri/src/bin/souls_mcp_server/tests.rs` com `#[tokio::test]` ou `#[test]`.
3. **Teste passa** com `cargo test --bin souls_mcp_server` (sob a feature apropriada).
4. **Zero clippy warning** ao rodar `cargo clippy --bin souls_mcp_server -- -D warnings`.
5. **Telemetria MPSC** despachada via `try_send_cold(StateDbOp::LogTelemetry {...})`.
6. **ADR conformidade** verificada (ADR-001, -003, -010, -025, -027, -028, -041, -044).

---

## Bloco A — Materialização do Swap de VRAM (Real FFI)

- [x] **A1. Refatorar `KvCacheSwapController` para delegar a worker dedicado** (RED → GREEN → REFACTOR)
- [x] **A2. Verificação física via DMA `before/after`** (RED → GREEN)
- [x] **A3. Remover definitivamente o stub de `0xAA`**

## Bloco B — Integração Real do ONNX Runtime (CPU AVX2)

- [x] **B1. Estruturação do `OrtScorerEngine` para inferência AVX2 com 0 MB VRAM**
- [x] **B2. Substituir `score` e `classify` por cálculo de densidade e entropia de Shannon real**
- [x] **B3. Adicionar `run_souls_intent` para probabilidades estáveis de ambiguidade, risco e conflito**

## Bloco C — Execução Estrita de AST em Wasmtime Guest

- [x] **C1. Drenar a `Instance` do Wasmtime e invocar `parse` real sem regex fallback**
- [x] **C2. Fuel Metering estrito (10M) com captura de traps fail-soft**
- [x] **C3. Epoch Interruption e controle de limites de memória (16MB)**
- [x] **C4. Banir o regex fallback como método primário**

## Bloco D — Suíte de Testes Antifraude (TDD Cirúrgico)

- [x] **D1. `test_vram_swapping_physical_ffi_effect`**
- [x] **D2. `test_onnx_scorer_real_inference_precision`**
- [x] **D3. `test_wasmtime_fuel_limit_trap`**

## Bloco E — Homologação Final

- [x] **E1. `cargo test --bin souls_mcp_server`** passou com 103/103 testes verdes.
- [x] **E2. `cargo clippy --bin souls_mcp_server -- -D warnings`** com 0 warnings.
- [x] **E3. Logs finais em `.souls_scratchpad/logs/cargo/clippy_mcp_perf.log`** conforme ADR-003.
- [x] **E4. Laudo técnico consolidado.**

---

## Bloco F — Persistência da Alma Socrática (L2 State & V5)

- [x] **F1. Elevar user_version do SQLite para 5 via migração idempotente com DDL STRICT**
- [x] **F2. Implementar trait `SocraticPersist` assíncrono para canais MPSC Tokio**
- [x] **F3. Plugar garras socráticas no router MCP (`export_session`, `analyze_session`, `merge_sessions`)**

## Bloco G — Motor de Calor de Acesso (repo_heatmap_log & Langevin Decay)

- [x] **G1. Criar tabela STRICT `repo_heatmap_log` com índices em path e accessed_at**
- [x] **G2. Implementar equação exponencial de Langevin $Frecency(f) = \sum e^{-\lambda \cdot dt}$**
- [x] **G3. Integrar filtros de diretórios tóxicos de `extensions.rs` e responder top 20 em <3ms**

## Bloco H — Tríade de Memória L3 (LanceDB, LadybugDB e Reator RRF AVX2)

- [x] **H1. Garantir conexão LanceDB serverless mmap em Host RAM com 0 MB de VRAM gráfica**
- [x] **H2. Implementar Grafo Ontológico LadybugDB com DashMap + SQLite e firewall BFS anti-poisoning**
- [x] **H3. Implementar Reator RRF com aceleração SIMD AVX2 em CPU Host (< 5ms)**

## Bloco I — Caderno de Testes de Estresse Antifraude (TDD)

- [x] **I1. `test_database_migration_v5`**
- [x] **I2. `test_repo_heatmap_langevin_decay`**
- [x] **I3. `test_lancedb_mmap_zero_vram_isolation`**
- [x] **I4. `test_ladybug_graph_bfs_poison_prevention`**
- [x] **I5. `test_hybrid_search_rrf_avx2_fusion`**

## Bloco J — Homologação Final

- [x] **J1. `cargo test --bin souls_mcp_server`** com 108/108 testes verdes (100%).
- [x] **J2. `cargo clippy --bin souls_mcp_server -- -D warnings`** com 0 warnings.
- [x] **J3. Logs finais em `.souls_scratchpad/logs/cargo/clippy_mcp_perf.log`**.

## Bloco K — Operação Tríade de Autonomia (Jaula LPAC, Metabolismo Chyros e Interrupção HITL)

- [x] **K1. Jaula de Silício Win11 e Liberação da Garra `execute`**
  - Confinamento bare-metal LPAC via `windows-sys = "=0.61.2"`, `CreateAppContainerProfile`, ACLs NTFS estritas (`GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE`), Job Object limites de CPU/memória e kill-on-close.
  - Bypass Gracioso em Session 0 com isolamento por Job Objects e varredura estática de `Cargo.toml` (`build.rs` / `proc-macro = true`).
  - Conectar `run_execute` no router MCP para execução real enjaulada.
- [x] **K2. Metabolismo Noturno Chyros Daemon (Marco 5.7.0)**
  - Loop assíncrono em background Tokio avaliando ociosidade a cada 60s.
  - Gather de eventos `souls_raw_events_l0` e subgrafos LadybugDB.
  - Consolidação lógica via `LlamaCpp4LogitEngine` CPU AVX2 (`n_gpu_layers = 0`), marcando obsoletos como `SUPERSEDED`.
  - Poincaré Gradient Descent Langevin Decay ($\ge 0.95$ evicção), atualização de embeddings na CPU e LanceDB (mmap).
  - Materialized Memory View (MMV) com alinhamento a 64 tokens e `VACUUM INTO` assíncrono.
- [x] **K3. Interrupção Socrática e CPU Logit Probing**
  - Logit probing na CPU Host via AVX2 sobre tokens de controle "0" e "1" do Verbalizer.
  - Cálculo de Softmax e Entropia de Shannon $H(p) \ge 0.75$ ou 3 falhas de compilação $\to$ Canal de Interrupção Socrática.
  - Extração de diff via `gix` (gitoxide), Pergunta Socrática de Duas Pernas ("Como", "O que") e bloqueio assíncrono no stdin.
- [x] **K4. Suíte de Testes Antifraude**
  - `test_sandbox_lpac_confinement`
  - `test_chyros_langevin_eviction_convergence`
  - `test_socratic_cli_block_and_stdin_approval`
- [x] **K5. Homologação Final (Exit Code 0 e Zero Clippy Warnings — 111/111 testes verdes)**

---

## Notas de Execução

- **Antes de cada bloco**: rodar `cargo check` para confirmar compilação incremental.
- **Após cada bloco**: rodar `cargo clippy --bin souls_mcp_server -- -D warnings`.
- **Política de Commits**: por bloco, com mensagem Conventional Commits (`feat(socratic):`, `feat(heatmap):`, `feat(l3):`, `test(antifraud):`).
- **HITL Gate**: homologação final com Exit Code 0 absoluto.
