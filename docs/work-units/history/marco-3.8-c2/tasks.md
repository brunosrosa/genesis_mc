# Tasks — Marco 3.8 Fase C.2: Enjaulamento Wasmtime + Symbol/Call Graph

> Cada tarefa tem **Definition of Done (DoD)** rigorosa.
> **Lei do Scaffold:** TDD Red antes da lógica real.
> **Validação final:** `cargo test --bin souls_mcp_server` ≥ 38/38 verdes em < 0.1s (Marcha Rápida, sem CUDA/Tauri).

---

## T-1: WasmEngine (Cerca Wasmtime)

**Arquivo:** `src-tauri/src/cognition/observability/wasm_engine.rs` (novo)

**DoD:**
- [ ] `OnceLock<Engine>` singleton com `consume_fuel(true)`, `epoch_interruption(true)`, `wasm_component_model(true)`.
- [ ] Função `WasmEngine::execute_safely<F, T>(module: &Module, f: F) -> Result<T, WasmTrap>` com:
  - `Store` com memory limiter 16 MiB.
  - `set_fuel(10_000_000)` antes de `f`.
  - Pattern matching sobre `anyhow::Error` para classificar trap em `WasmTrap::Unreachable | Oom | FuelExhausted | StructuredFailure`.
  - `std::panic::catch_unwind` para panics Rust-side.
- [ ] Tipo `WasmTrap` com variantes documentadas.
- [ ] `pub mod wasm_engine;` em `observability/mod.rs`.
- [ ] `pub use wasm_engine::*;` em `observability/mod.rs`.

---

## T-2: SYMBOL_INDEX e CALL_GRAPH (DashMap de RAM)

**Arquivo:** `src-tauri/src/cognition/observability/call_graph.rs` (novo)

**DoD:**
- [ ] Struct `SymbolEntry { qualified_name, kind, file_path, line, column }`.
- [ ] Enum `SymbolKind { Fn, Struct, Enum, Trait, Const, Static }`.
- [ ] Struct `CallGraphNode { symbol, adjacents: HashSet<String>, last_updated }`.
- [ ] `SYMBOL_INDEX: OnceLock<DashMap<String, SymbolEntry>>`.
- [ ] `CALL_GRAPH: OnceLock<DashMap<String, CallGraphNode>>`.
- [ ] Função `init_or_get_symbol_index() -> &'static DashMap<...>`.
- [ ] Função `init_or_get_call_graph() -> &'static DashMap<...>`.
- [ ] Função `insert_symbol(entry)` e `insert_edge(caller, callee)`.
- [ ] `pub mod call_graph;` em `observability/mod.rs`.

---

## T-3: MPSC Telemetry Worker

**Arquivo:** `src-tauri/src/cognition/observability/mpsc_telemetry.rs` (novo)

**DoD:**
- [ ] Enum `TelemetryEvent { FileMutated { path, content }, FileDeleted { path } }`.
- [ ] Função `spawn_telemetry_worker() -> mpsc::Sender<TelemetryEvent>`.
- [ ] Canal `mpsc::channel::<TelemetryEvent>(256)`.
- [ ] Worker em `std::thread::spawn` (NÃO `tokio::spawn`).
- [ ] HIPER-FORWARD: API `try_emit_event(TelemetryEvent) -> bool` para uso no critical path.
- [ ] Teste de ping: `try_emit_event` não bloqueia mesmo após 256 envios rápidos.
- [ ] `pub mod mpsc_telemetry;` em `observability/mod.rs`.

---

## T-4: Registro das 3 Tools no MCP Server

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs`

**DoD:**
- [ ] Substituir stub `not_implemented_yet` por handlers reais:
  - `run_symbol(params)` → consulta `SYMBOL_INDEX` O(1).
  - `run_callers(params)` → consulta `CALL_GRAPH` adjacents.
  - `run_callees(params)` → consulta `CALL_GRAPH` adjacents.
- [ ] Dispatcher atualizado com aliases: `"symbol" | "souls_symbol" | "ctx_symbol"` etc.
- [ ] `tools/list` substituído:
  - `symbol`: "Resolve a localização física (file:line) de símbolos sintáticos da AST do monorepo em O(1)." (≤120 chars)
  - `callers`: "Lista os nós do grafo de dependências que invocam um determinado símbolo no workspace." (≤120 chars)
  - `callees`: "Mapeia quais funções e structs são consumidos internamente pelo símbolo interrogado." (≤120 chars)
- [ ] Validação rigorosa: argumento `name` obrigatório, não-vazio, ≤256 chars.

---

## T-5: Testes TDD (3 novos)

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (módulo `tests`)

**DoD:**
- [ ] `test_wasm_tree_sitter_isolation`: WAT com `unreachable` → `WasmTrap::Unreachable` retornado, thread Tokio sobrevive, suite de testes continua.
- [ ] `test_symbol_resolution_o1`: Inserir 10K entradas em `SYMBOL_INDEX` (após `init_or_get`); `symbol(name)` retorna `Some(entry)` em <10μs médio.
- [ ] `test_callers_callees_graph`: Popular 5 nós `a→b`, `a→c`, `b→d`, `c→d`, `d→e` em `CALL_GRAPH`; `callers("d")` retorna `{b, c}` exato; `callees("a")` retorna `{b, c}` exato.
- [ ] Concorrência serializada: `static TELEMETRY_TDD_LOCK: std::sync::Mutex<()>` se houver risco de race entre `init_or_get_*` chamado de múltiplos tests.

---

## T-6: Validação Fast Pass

**Comando:** `cargo test --bin souls_mcp_server --release=false`

**DoD:**
- [ ] 38/38 testes verdes (35 baseline + 3 novos) em < 0.1s.
- [ ] `cargo clippy --bin souls_mcp_server -- -D warnings` zero issues.
- [ ] `tools_list_respects_32_120_tetos` continua verde.
- [ ] `tools_list_returns_unprefixed_names` continua verde.
- [ ] Nenhum stub `not_implemented_yet` permanece para `symbol`/`callers`/`callees`.

---

## T-7: Commit + PR (HITL)

**DoD:**
- [ ] Commit com mensagem descritiva: `feat(wasm-callgraph): Marco 3.8 Fase C.2 — enjaulamento Wasmtime + SYMBOL_INDEX/CALL_GRAPH`.
- [ ] Push para `origin/feat/wasm-callgraph-v1`.
- [ ] `gh pr create --base main --head feat/wasm-callgraph-v1 --title "Marco 3.8 Fase C.2: Wasmtime Cage + Symbol/Call Graph" --body-file .github/PULL_REQUEST_TEMPLATE.md`.
- [ ] URL do PR enviada ao Arquiteto para HITL.
- [ ] **Nenhum merge automático** — aguarda Code Review Humano.
