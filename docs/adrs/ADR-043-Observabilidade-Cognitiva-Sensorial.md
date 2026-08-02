---
id: "ADR-043"
title: "ADR-043-Observabilidade-Cognitiva-Sensorial"
version: 1.0
status: Ativo
epic: "Cognicao / Observabilidade"
amends: ["ADR-040", "ADR-041", "ADR-042"]
description: "Marco 3.7 Fase B: institui o sistema sensorial nativo (heatmap, impact, routes, feedback) operando 100% em RAM Host + SQLite (State DB v3), quitando a divida de stubs sensoriais do gateway MCP."
---

# ADR-043 — Observabilidade Cognitiva Sensorial (SODA State v3)

## Status
Aceito. Emenda cumulativa das ADRs 040 (State DB v2), 041 (Servername Soberano) e 042 (CCR Conveyor Belt).

## Contexto

O Marco 3.5 (cognicao) e o Marco 3.6 (CCR/compressao) dotaram o `souls_mcp_server` de um nucleo cognitivo (`mem_search`, `core_think`, `souls_multi_read`) mas **nao** de um sistema sensorial: o servidor nao sabe quais arquivos foram tocados, quem importa quem, quais rotas IPC estao ativas, nem quanto foi consumido em FinOps.

A ausencia de telemetria local gera tres patologias:

1. **Cegueira Termica:** o orquestrador nao detecta arquivos *quentes* (frequentemente re-lidos) que deveriam ser candidatos a cache agressivo.
2. **Blast Radius Cego:** mutacoes em modulos centrais (ex: `cognition/lean_vacuum/dedup.rs`) sao aplicadas sem ciencia de quem sera afetado pela arvore de imports.
3. **Dossie FinOps Inexistente:** a eficiencia E3 (tokens salvos / tokens brutos) nao pode ser auditada porque nao ha `telemetry_logs` local.

A `tools/list` atual expoe 4 ferramentas stub `not_implemented_yet` (`metrics`, `intent`, `callers`, `callees`) com descricoes mentirosas que violam o **principio da verdade semantica** da ADR-041.

## Decisao

Ficam instituidas as seguintes decisoes arquitetonicas, todas sob o namespace canonico **`souls_mcp.<tool>`** (ADR-041):

### 1. State DB v3 (Migracao Idempotente)

- Bump de `PRAGMA user_version` para **3** via DDL idempotente (mesma forma da ADR-040 V1→V2).
- Novas tabelas no banco `souls_state.db` (reaproveita o worker, sem novo arquivo):
  - `file_access_logs`: `(id, file_path TEXT, tool TEXT, accessed_at INTEGER)` — append-only.
  - `telemetry_logs`: `(id, tool TEXT, tokens_in INTEGER, tokens_out INTEGER, cost_usd REAL, duration_ms INTEGER, created_at INTEGER)`.
- Indices: `idx_file_path_time` (path, time), `idx_telemetry_tool_time` (tool, time).

### 2. Heatmap (Langevin Decay)

- Aplica **decaimento exponencial** do tipo Langevin sobre os timestamps de `file_access_logs`:
  - `score(path, t_now) = sum_i exp(-lambda * (t_now - t_i))` com `lambda = 0.05`.
  - Constante calibrada empiricamente: para um arquivo acessado ha 24h, o peso e ≈ 0.012 (morno); ha 1h, ≈ 0.74 (quente).
- Ordenacao deterministica (desempate por path) e truncada em top-N (default 50, configuravel).
- Output JSON canonico: `{ "scores": [{ "path": ..., "score": 0.83, "access_count": 7 }, ...] }`.

### 3. Impact (Blast Radius DAG)

- Constrói um **grafo de imports** do monorepo em RAM via `BTreeMap<String, Vec<String>>` (sem crate extra — so std).
- BFS no **grafo transposto** a partir do arquivo-alvo → retorna todos os importadores recursivos.
- Output JSON: `{ "target": "cognition/lean_vacuum/dedup.rs", "affected": ["cognition/mod.rs", "bin/souls_mcp_server.rs", ...], "depth": 3 }`.
- Algoritmo O(V+E) com `VecDeque` — defensivo contra ciclos.

### 4. Routes (IPC Contract Mapping)

- Varredura estatica via **`regex::Regex` compilado uma unica vez** (lazy_static-style via `OnceLock`).
- Deteccao de comandos Tauri backend: regex `r#"tauri::command"#` em `.rs`.
- Deteccao de invocacoes Svelte 5 frontend: regex `r#"invoke\(['"]([a-z_]+)['"]"#` em `.svelte/.ts`.
- Output: `{ "backend": [...], "frontend": [...], "orphans": [...] }` (orphans = comandos sem invoke equivalente).

### 5. Feedback (FinOps Telemetry Dump)

- Leitura agregada de `telemetry_logs` em uma unica query SQL.
- Calculo da **eficiencia E3**: `E3 = 1 - (tokens_out / max(1, tokens_in + tokens_out))` (normalizada em [0, 1]).
- Output: `{ "total_tokens_in": N, "total_tokens_out": N, "total_cost_usd": N, "e3_efficiency": 0.42, "by_tool": { "read": {...}, "compress": {...} } }`.

### 6. Registro MCP (Emenda ADR-041)

As 4 novas ferramentas canônicas (todas respeitando tetos 32/120):

| Tool | Descricao (≤120 chars) | Aliases |
|---|---|---|
| `heatmap` | "Mapeia dinamicamente os caminhos quentes de acesso a arquivos locais na RAM Host usando Langevin decay." | `heatmap` \| `souls_heatmap` |
| `impact` | "Calcula o Blast Radius (importadores afetados) de qualquer arquivo no monorepo via BFS em grafo transposto." | `impact` \| `souls_impact` |
| `routes` | "Mapeia os contratos de endpoints ativos e a reatividade de comunicacao entre Tauri Rust e Svelte 5." | `routes` \| `souls_routes` |
| `feedback` | "Dumps FinOps de telemetria, latencia e eficiencia de token E3 a partir de logs locais de execucao." | `feedback` \| `souls_feedback` |

Os stubs `metrics`, `intent`, `callers`, `callees` **permanecem** (outras engrenagens), mas a verdade semantica das 4 novas cobre as 3 dores declaradas (cegueira termica, blast radius, FinOps).

### 7. Instrumentacao do Dispatcher (Lei do Filesystem Spy)

- O `handle_tool_call` recebe um hook observavel: para tools que tocam filesystem (`read`, `edit`, `get_ast`, `multi_read`, `smart_read`, `souls_stub_fill`, `headroom_retrieve`, `tree`, `outline`), insere um registro async em `file_access_logs` via canal MPSC ja existente (`STATE_DB_TX`).
- Zero-overhead quando a chamada falha (try_log, nao bloqueia o critical path).

## Consequencias

* **+4 ferramentas canonicas** no `tools/list` — `souls_mcp.*` permanece unico servername nativo.
* **+0 crates externos** (BTreeMap, regex, walkdir ja no Cargo.toml).
* **+0 chamadas CUDA/Tauri** — Marcha Rapida (Fast Pass) mantem TDD abaixo de 10s.
* **Reuso do worker MPSC** do Marco 3.5 — sem novo thread pool, sem `spawn_blocking`.
* **Idempotencia total:** rodar `init_state_db_and_worker` em um banco v3 existente e no-op.

## Restricoes Bare-Metal e Blast Radius

* **`src-tauri/src/cognition/observability/` (NOVO):** `mod.rs`, `heatmap.rs`, `impact.rs`, `routes.rs`, `feedback.rs`, `ops.rs`, `types.rs`. Toda a logica vive aqui, testavel de forma isolada.
* **`src-tauri/src/cognition/mod.rs`:** adicionar `pub mod observability;` (zero-regressao).
* **`src-tauri/src/bin/souls_mcp_server.rs`:**
  * Importar `cognition::observability`.
  * Estender `StateDbOp` com 2 variantes: `LogFileAccess`, `LogTelemetry`.
  * Inserir 4 entradas no array `tools` da resposta `tools/list`.
  * Adicionar 4 ramos no `match` do `handle_tool_call` com aliases `name | souls_name`.
  * Estender `init_state_db_and_worker` com DDL V3 (CREATE TABLE IF NOT EXISTS + PRAGMA user_version=3).
* **`docs/adrs/ADR-043-Observabilidade-Cognitiva-Sensorial.md`:** este arquivo.
* **Zero mutacao** em `gateway-config.yaml` (servername inalterado) ou skills (canibalizacao preservada).

## Metricas de Sucesso

* `cargo test --bin souls_mcp_server` verde com **4 novos testes** (`test_file_access_logging_and_heatmap_decay`, `test_blast_radius_dag_bfs`, `test_routes_contract_regex`, `test_feedback_telemetry_insert_and_e3_calc`).
* `cargo clippy --bin souls_mcp_server -- -D warnings` sem avisos.
* `git grep "not_implemented_yet: Métricas" src-tauri/src/bin/souls_mcp_server.rs` retorna 1 match (stub metrics legado preservado, mas nao substituido).
* Snapshot do `tools/list` mostra exatamente **4 novas tools** com tetos 32/120 respeitados.

## Razao de Ser desta ADR

> "Sem telemetria local, o SODA e um cerebro sem olhos nem ouvidos. O Marco 3.7 doa olhos (heatmap), ouvidos (impact), fala (routes) e memoria metabolica (feedback). Tudo em Rust, tudo em RAM, tudo na casa dos 6GB de VRAM." — Bruno, 2026-08-02.
