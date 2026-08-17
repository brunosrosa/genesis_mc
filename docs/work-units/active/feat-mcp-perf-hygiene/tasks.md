# Tarefas: Saneamento de Performance, Timeouts Inteligentes e Extirpação de Stubs MCP

**Work Unit:** `feat-mcp-perf-hygiene`  
**Status:** IN_PROGRESS  

## Backlog de Implementação

- [ ] **Tarefa 1: Purificação de Latência de Routes, Repo Heatmap, Repo Impact e Symbol**
  - [ ] 1.1 Cache estático `OnceLock<RouteReport>` em `routes.rs` / `handlers/observability.rs` (< 1ms).
  - [ ] 1.2 `repo_heatmap` com query atômica indexada no SQLite do `souls_state.db` sem WalkDir repetido no hot path (< 3ms).
  - [ ] 1.3 `repo_impact` com resolução prioritária via BFS em RAM no `SYMBOL_INDEX` / `CALL_GRAPH` (`DashMap`) (< 3ms).
  - [ ] 1.4 `symbol` e `WasmtimeTreeSitterEngine` com reuso estrito de `GLOBAL_MODULES_CACHE` (< 1ms).

- [ ] **Tarefa 2: Timeouts Inteligentes e Blindagem de Canais**
  - [ ] 2.1 Envolver `run_web_fetch` / download + parsing com `tokio::time::timeout(Duration::from_secs(25))` e aborto gracioso com erro estruturado.
  - [ ] 2.2 `execute` no `router.rs` retornando determinística e graciosamente o erro estruturado `-32001` (`HitlDenied`).

- [ ] **Tarefa 3: Extirpação de Stubs Internos e Conexão no Silício**
  - [ ] 3.1 `intent`: Conectar `run_intent` / `run_souls_intent` via `LlamaCpp4LogitEngine` / `LlamaCppEpistemicProber` na CPU com AVX2 (< 150ms).
  - [ ] 3.2 `metrics`: Conectar `run_metrics` à tabela `telemetry_logs` do `souls_state.db` (< 2ms).
  - [ ] 3.3 `headroom_retrieve`: Conectar à `SodaCcrStore` e `ccr_cache()` em RAM Host (< 1ms).
  - [ ] 3.4 `KVCacheSwapController`: Buffer em Host RAM via `Arc<Mutex<Vec<u8>>>` sem prints síncronos de texto.

- [ ] **Tarefa 4: Suíte TDD e Homologação**
  - [ ] 4.1 Testes em `tests.rs`: `test_routes_performance_under_1ms`, `test_fetch_web_smart_timeout_abort`, `test_intent_real_logit_probing_execution`, `test_metrics_real_aggregation_from_sqlite`.
  - [ ] 4.2 Executar `cargo test --bin souls_mcp_server` e `cargo clippy` com saída em `.souls_scratchpad/logs/cargo/clippy_mcp_perf.log`.
