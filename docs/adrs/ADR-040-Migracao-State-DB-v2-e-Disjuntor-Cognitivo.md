---
id: "ADR-040"
title: "ADR-040: Migração do souls_state.db v1→v2 e Disjuntor Cognitivo (5→7) no Marco 3.5"
version: 1.0
status: Ativo_Inegociavel
epic: "Cognição / Memória"
description: "Promove o banco L2 `souls_state.db` para v2 (PRAGMA user_version), separando observações em tabela normalizada própria com triggers FTS5; institui o disjuntor cognitivo 5→7 (HITL) do scratchpad socrático `souls_thinking`. Materializa o Marco 3.5 (Core Cognitivo) sob PRD-031 e PRD-032 sem dependências novas no Cargo.toml."
---

# ADR-040: Migração do souls_state.db v1→v2 e Disjuntor Cognitivo (5→7)

## Status

Aceito (Ativo e Inegociável) — Branch `feat/cognition-core-v1` em forja. Marco 3.5 (Core Cognitivo) do SOULS MC.

## Contexto Técnico e Ameaça à Cognição Local

A esteira SOULS MC (Rust + Tauri v2, RTX 2060m 6GB, Z: ReFS) opera 100% offline, air-gapped, com latências sub-milissegundo. Subagentes locais (LlamaVanguardEngine, LlamaCppEngine) sofrem de duas patologias agudas que comprometem a autonomia do sistema:

1. **Amnésia Epistêmica Temporal (Context Rot)** — subagentes perdem o rastro de decisões tomadas em turnos passados, repetindo buscas e redigitando código. Causa raiz: a memória relacional (`entities`/`relations`) existe fisicamente no `souls_state.db` (criada por `init_state_db_and_worker()` em [src-tauri/src/bin/souls_mcp_server.rs:2206](file:///z:/souls_mc/src-tauri/src/bin/souls_mcp_server.rs#L2206)), porém **as 9 ferramentas MCP canônicas do `memory-mcp-rs` (`mem_create_entities`, `mem_search`, etc.) não estão expostas no `tools/list`**. As observações estão embutidas como coluna JSON em `entities.observations` — violando a 3FN e impossibilitando busca observacional indexada.
2. **Paralisia de Análise (Overthinking)** — modelos compactos locais tentam cuspir a resposta inteira em um único passo estocástico, falhando em manter restrições lógicas. Causa raiz: ausência de uma máquina de estados socrática (`Regular | Revision | Branching`) que force decomposição iterativa sob orçamento rígido, e ausência de um disjuntor FinOps que limite a profundidade do raciocínio.

A auditoria de trincheira (laudo 2026-08-01) confirmou que **70% do alicerce físico já existe** (WAL, `busy_timeout=5000ms`, `foreign_keys=ON`, `entities`/`relations`, FTS5 e MPSC buffer 100 em `StateDbOp`). O gap é 100% código novo, sem dependências externas, com canibalização cirúrgica das APIs do `memory-mcp-rs` e do `ultrafast-mcp-sequential-thinking`.

## Decisão Arquitetural (A Matriz do Marco 3.5)

### 1. Migração V1 → V2 (PRAGMA user_version)
- Fica introduzido o versionamento explícito do schema via `PRAGMA user_version` (canônica do SQLite, presente no trunk desde 3.x).
- V1 = estado atual (somente `entities`/`relations` com `observations` como coluna JSON).
- V2 = adiciona tabela normalizada `observations` + virtual table FTS5 espelhada + triggers de sincronização.
- **Sem drops destrutivos** (a coluna `entities.observations` é mantida para retrocompatibilidade; nenhum dado existente é perdido). A hidratação de `Entity.observations: Vec<String>` na API é feita via JOIN em runtime, lendo exclusivamente da nova tabela.
- Toda migração é executada dentro de **transação atômica** (`conn.transaction()` do rusqlite), garantindo que um crash no meio do DDL não corrompa o banco.

### 2. A Tríade de Segurança do SQLite (já canônica, agora auditada)
- `PRAGMA journal_mode = WAL` — append-only, protege SSD (TBW) [ADR-004].
- `PRAGMA foreign_keys = ON` — `ON DELETE CASCADE` real entre `entities` e `observations`.
- `PRAGMA busy_timeout = 5000ms` — tolerância a contenção sob swarm.
- Idempotência: `CREATE TABLE IF NOT EXISTS` + `CREATE TRIGGER IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS`. Re-rodar `init_state_db_and_worker` é seguro.

### 3. O MPSC Buffer 100 e a Concorrência
- O canal `tokio::sync::mpsc` (buffer 100) já existe como `STATE_DB_TX: OnceLock<mpsc::Sender<StateDbOp>>` para `SubAgent`/`Handoff`/`Knowledge`. O Core Cognitivo **estende o mesmo padrão** introduzindo variantes `MemGraphOp` (9 operações do grafo) e o estado in-RAM do `ThinkingEngine` (sem persistência obrigatória).
- Worker canônico: `std::thread::spawn` + `rx.blocking_recv()` — sincronia pura para `rusqlite`, isolada do event loop do Tokio (sem `spawn_blocking`).
- Backpressure: `try_send` com jitter de 0–8ms (Jitter determinístico via `rand::thread_rng` se disponível; caso contrário, sleep de 0 nanossegundos) para telemetria. `request().await` apenas para confirmação transacional.

### 4. Disjuntor Cognitivo 5 → 7 (HITL)
- `DEFAULT_HARD_LIMIT = 5` — teto absoluto por padrão.
- `HITL_EXTENDED_LIMIT = 7` — teto elástico sob autorização explícita do Arquiteto via `hitl_authorized: true` no payload MCP de `core_think`.
- Validação server-side pura: nenhuma variável de ambiente, nenhum bypass por flag de boot. O sinal de "liberação" viaja **fim-de-fio** no payload e é validado pelo `ThinkingEngine` antes de cada `push_thought`.
- Erro tipado: `CognitiveError::OverthinkingThresholdBreached { actual: u32, max: u32 }` — Fail-Closed L7 conforme [ADR-006](file:///z:/souls_mc/docs/adrs/ADR-006-SSOT-Sheets.md).
- Tríade obrigatória no `souls_thinking`: `Regular | Revision | Branching`. Validação de `is_revision=true ⇒ revises_thought.is_some()` (sob `CognitiveError::RevisionWithoutTarget`).

### 5. Lei do Zero-Dep (Hard Constraint)
- **Nenhuma crate nova é adicionada ao `Cargo.toml`**. A forja inteira usa exclusivamente: `rusqlite = "=0.39.0"` (já bundled), `tokio = "=1.51.1"` (full), `thiserror`, `serde`, `serde_json`, `dashmap`, `tracing`. Timestamps via `std::time::SystemTime` em milissegundos — `chrono` permanece banido (canon do ADR-005 / ADR-030).

### 6. Canonical Souls_* Naming
- Módulo: `src-tauri/src/cognition/memory_graph/` (souls_graph canibalizado).
- Módulo: `src-tauri/src/cognition/thinking/` (souls_thinking canibalizado).
- Operações: `mem_create_entities`, `mem_create_relations`, `mem_add_observations`, `mem_search`, `mem_open_nodes`, `mem_read_graph`, `mem_delete_entities`, `mem_delete_observations`, `mem_delete_relations`, `core_think`. **Não recebem o prefixo `souls_`** no `tools/list` (canônico do memory-mcp-rs), mas o módulo Rust subjacente é `memory_graph::*` e `thinking::*`.

## Caminhos Físicos da Mutação

```
docs/adrs/ADR-040-Migracao-State-DB-v2-e-Disjuntor-Cognitivo.md   [NEW] este arquivo
src-tauri/src/cognition/mod.rs                                     [EDIT] +2 pub mod
src-tauri/src/cognition/memory_graph/mod.rs                        [NEW]
src-tauri/src/cognition/memory_graph/types.rs                      [NEW] Entity, Relation, Observation
src-tauri/src/cognition/memory_graph/errors.rs                     [NEW] CognitiveError
src-tauri/src/cognition/memory_graph/ops.rs                        [NEW] 9 funções canônicas
src-tauri/src/cognition/memory_graph/fts.rs                        [NEW] DDL dos triggers FTS5
src-tauri/src/cognition/memory_graph/mpsc_bridge.rs                [NEW] MemGraphOp + spawn worker
src-tauri/src/cognition/thinking/mod.rs                            [NEW]
src-tauri/src/cognition/thinking/types.rs                          [NEW] ThoughtData, BranchId, ThoughtId
src-tauri/src/cognition/thinking/errors.rs                         [NEW] ThinkingError (estende CognitiveError)
src-tauri/src/cognition/thinking/state_machine.rs                  [NEW] ThinkingEngine + disjuntor
src-tauri/src/bin/souls_mcp_server.rs                              [EDIT] migração V2, tools/list, handle_mcp, testes
docs/state/SODA_CURRENT_STATE.md                                   [EDIT] Marco 3.5 → Ativo
```

## Consequências Operacionais (Trade-offs)

**Positivo:**
- Grafo relacional cognitivo (9 ops) e scratchpad socrático (1 op) nativos, sem daemons Python/Node, sem dependências novas.
- Custo de migration é O(1) em cold start (DDL idempotente) e zero em warm start (apenas `PRAGMA user_version`).
- Disjuntor FinOps: o `core_think` nunca ultrapassa 7 pensamentos, blindando VRAM contra loops socráticos infinitos.
- FTS5 sub-milissegundo: triggers de sincronização mantêm `observations_fts` consistente com `observations` em <1ms por batch.

**Negativo (manutenção):**
- O `entities.observations` legado permanece como coluna morta no schema (backwards compat). Decisão consciente: drop em ADR futura, após validação de migração estável em produção.
- O `ThinkingEngine` mantém estado in-RAM (`HashMap<BranchId, Vec<ThoughtId>>`); sessões de pensamento não sobrevivem a restart do `souls_mc`. Persistência é ADR futura (Marco 4).

## Definition of Done (DoD) — Verificável

1. `cargo test --features "tauri-app,gateway_ccr,llama_backend"` retorna Exit Code 0.
2. `cargo clippy --features "tauri-app,gateway_ccr,llama_backend" -- -D warnings` retorna 0.
3. `Cargo.toml` permanece com **0 deps novas** (verificável via `git diff src-tauri/Cargo.toml`).
4. Os 5 testes TDD obrigatórios passam: `test_graph_cascade_delete`, `test_thinking_disjuntor_loop`, `test_thinking_hitl_extension_to_7`, `test_migration_user_version_bump`, `test_fts5_observational_grounding`.
5. `PRAGMA user_version = 2` cravado após `init_state_db_and_worker` em banco migrado (verificável via `sqlite3 .souls_data/souls_state.db "PRAGMA user_version"`).
6. `tools/list` retorna 10 novas ferramentas canônicas (`mem_*` + `core_think`).
