# Marco 3.7 — Fase B: Observabilidade Cognitiva (Tasks)

**Branch:** `feat/observability-v1`
**ADR:** [ADR-043](docs/adrs/ADR-043-Observabilidade-Cognitiva-Sensorial.md)
**Modo de Validacao:** Marcha Rapida (Fast Pass) — `cargo test --bin souls_mcp_server`

---

## T0. Scaffold Cognitivo (DoD: pasta + mod.rs compativeis)

- [ ] Criar pasta `src-tauri/src/cognition/observability/`
- [ ] Criar `mod.rs` com stubs `pub mod` (heatmap, impact, routes, feedback, ops, types)
- [ ] Editar `src-tauri/src/cognition/mod.rs` → `pub mod observability;`
- [ ] DoD: `cargo check --bin souls_mcp_server` passa (apenas stubs vazios, sem logica).

## T1. V3 Schema (DoD: PRAGMA=3 + tabelas + indices)

- [ ] Criar `src-tauri/src/cognition/observability/types.rs` com a struct `FileAccessLog` e `TelemetryLog`.
- [ ] Criar `src-tauri/src/cognition/observability/ops.rs` com a funcao `migrate_v2_to_v3(conn) -> Result<(), CognitiveError>`.
- [ ] Editar `init_state_db_and_worker()` em `souls_mcp_server.rs` para chamar `ops::migrate_v2_to_v3` no boot (apos V2 ja rodar).
- [ ] DDL:
  - `file_access_logs` (id, file_path, tool, accessed_at) + idx `(file_path, accessed_at)`
  - `telemetry_logs` (id, tool, tokens_in, tokens_out, cost_usd, duration_ms, created_at) + idx `(tool, created_at)`
- [ ] DoD: `cargo check --bin souls_mcp_server` compila; o teste de migracao `test_migration_user_version_v3_bump` passa.

## T2. Heatmap (Langevin decay) (DoD: F1=heatmap.rs com F2=test_langevin_decay)

- [ ] Implementar `pub fn langevin_score(accesses: &[(i64,)], now: i64, lambda: f64) -> f64`.
- [ ] Implementar `pub fn compute_heatmap(conn: &Connection, now: i64, lambda: f64, limit: usize) -> Result<Vec<HeatmapEntry>, CognitiveError>`.
- [ ] Implementar `pub async fn run_heatmap(params) -> Result<Value, RpcError>` em `souls_mcp_server.rs`.
- [ ] DoD: Teste `test_file_access_logging_and_heatmap_decay` valida `score(0s) = 1.0`, `score(20s, lambda=0.05) ≈ 0.368`.

## T3. Impact (DAG BFS) (DoD: F1=impact.rs com F2=test_blast_radius_dag_bfs)

- [ ] Implementar `pub fn build_import_graph(root: &Path) -> Result<BTreeMap<String, Vec<String>>, CognitiveError>` (regex `use\s+crate::|use\s+super::|use\s+[^:]+::`).
- [ ] Implementar `pub fn blast_radius(graph: &BTreeMap<String, Vec<String>>, target: &str) -> Vec<String>` (BFS no grafo transposto, dedupe).
- [ ] Implementar `pub async fn run_impact(params) -> Result<Value, RpcError>`.
- [ ] DoD: Teste `test_blast_radius_dag_bfs` com grafo `A→B→C` retorna `impact("C") == ["B", "A"]` ordenado por profundidade.

## T4. Routes (Regex contracts) (DoD: F1=routes.rs com F2=test_routes_contract_regex)

- [ ] Implementar `pub fn scan_routes(root: &Path) -> Result<RouteReport, CognitiveError>` usando `regex::Regex::new()` compilado em `OnceLock`.
- [ ] Regex backend: `r#"\#\[tauri::command\]"` + nome da fn seguinte.
- [ ] Regex frontend: `r#"invoke\(\s*['"]([a-z_][a-z0-9_]*)['"]"` em `.svelte/.ts`.
- [ ] Implementar `pub async fn run_routes(params) -> Result<Value, RpcError>`.
- [ ] DoD: Teste `test_routes_contract_regex` com mocks inline valida deteccao de 2 comandos backend e 3 invokes frontend.

## T5. Feedback (FinOps E3) (DoD: F1=feedback.rs com F2=test_e3_calc)

- [ ] Implementar `pub fn e3_efficiency(tokens_in: i64, tokens_out: i64) -> f64` → `1 - tokens_out / max(1, in+out)`.
- [ ] Implementar `pub fn aggregate_telemetry(conn: &Connection) -> Result<TelemetryReport, CognitiveError>`.
- [ ] Implementar `pub async fn run_feedback(params) -> Result<Value, RpcError>`.
- [ ] DoD: Teste `test_feedback_telemetry_insert_and_e3_calc` valida `E3(0,0) = 1.0`, `E3(100, 25) = 0.80`.

## T6. MPSC + Dispatcher Wiring (DoD: 4 tools chamadas via alias)

- [ ] Estender `StateDbOp` em `souls_mcp_server.rs` com variantes `LogFileAccess` e `LogTelemetry`.
- [ ] Adicionar 4 entradas no array `tools` do response `tools/list` (com tetos 32/120).
- [ ] Adicionar 4 ramos no `match` do `handle_tool_call` com aliases `name | souls_name`.
- [ ] Instrumentar o dispatcher para chamar `log_file_access()` antes/depois das tools `read | edit | get_ast | multi_read`.
- [ ] DoD: Snapshot test `test_tools_list_includes_observability_v3` valida presenca das 4 tools e tetos 32/120.

## T7. TDD Final & Validacao (DoD: Fast Pass verde + clippy clean)

- [ ] Rodar `cargo test --bin souls_mcp_server` → todos os testes verdes (legado + 4 novos).
- [ ] Rodar `cargo clippy --bin souls_mcp_server -- -D warnings` → sem warnings.
- [ ] Atualizar `docs/state/SODA_CURRENT_STATE.md` com a entrada do Marco 3.7.
- [ ] Sanity check: `git grep "todo!\|unimplemented!\|not_implemented_yet:.*Heatmap\|not_implemented_yet:.*Impact\|not_implemented_yet:.*Routes\|not_implemented_yet:.*Feedback"` retorna 0 matches em codigo novo.

---

## Lei do Scaffold (Marco 3.5+)

Cada tarefa T1-T5 sera precedida por um teste de falha (Red) antes da implementacao (Green), conforme a doutrina TDD da skill `souls-sdd` (Fase 4: Mutacao Atomica). Em caso de erro de compilacao, invocar a skill `souls-ralph-loop` (max 3 tentativas).

## Modo de Execucao

Todas as builds locais via `cargo test --bin souls_mcp_server` (Fast Pass) — proibido CUDA/Tauri-app durante este Marco. As features `llama_backend` e `tauri-app` NAO devem ser ativadas.

## Comandos Uteis

```powershell
# Marcha Rapida (TDD)
cargo test --bin souls_mcp_server

# Clippy paranoico
cargo clippy --bin souls_mcp_server -- -D warnings

# Git status do branch
git status --short
```
