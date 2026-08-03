# Tasks — Marco 3.9 Fase E: Persistência Socrática (Souls State V5)

> Cada tarefa tem **Definition of Done (DoD)** rigoroso.
> **Lei do Scaffold:** TDD Red antes da lógica real.
> **Validação final:** `cargo test --bin souls_mcp_server` ≥ 39/39 verdes em < 0.2s (Marcha Rápida).

---

## T-1: ADR-045 + design.md

**Arquivos:** `docs/adrs/ADR-045-Persistencia-da-Alma-Socratica.md`, `docs/design-marco-3.9-e.md`

**DoD:**
- [x] ADR-045 escrito com Constituição Sintática completa (Lei 32/120, O(n) reconstruction, FK CASCADE, last-write-wins).
- [x] design-marco-3.9-e.md com diagrama Mermaid (Topologia FinOps).

---

## T-2: State DB V5 — Migração Idempotente

**Arquivo:** `src-tauri/src/cognition/thinking/ops.rs` (novo) + `souls_mcp_server.rs:init_state_db_and_worker`

**DoD:**
- [ ] Módulo `cognition::thinking::ops` com:
  - `pub const V5_SCHEMA_DDL: &str` com as 2 tabelas + 4 índices.
  - `pub const TARGET_VERSION: i64 = 5;`
  - `pub fn migrate_v3_to_v5(conn: &mut Connection) -> Result<(), CognitiveError>`.
  - `pub fn insert_socratic_session(conn: &Connection, session_id, metadata) -> Result<()>`.
  - `pub fn insert_socratic_thought(conn: &Connection, t: &SocraticThought) -> Result<()>`.
  - `pub fn list_thoughts_for_session(conn: &Connection, session_id) -> Result<Vec<SocraticThought>>`.
  - `pub fn delete_session(conn: &Connection, session_id) -> Result<()>`.
- [ ] `init_state_db_and_worker` chama `thinking::ops::migrate_v3_to_v5(&mut conn)` APÓS `observability::migrate_v2_to_v3`.
- [ ] Idempotente: se `user_version >= 5`, no-op.

---

## T-3: Struct SocraticThought (Persistência)

**Arquivo:** `src-tauri/src/cognition/thinking/persistence.rs` (novo)

**DoD:**
- [ ] Struct `SocraticThought { thought_id, session_id, branch_id, parent_thought_id, thought_type, content, step_number, duration_ms, created_at }`.
- [ ] Tipo `ThoughtType { Regular, Revision, Branching }` (pode ser `as_str()` para SQLite).
- [ ] `pub type SessionId = String;` (UUIDv4 simples).
- [ ] `pub type ThoughtId = String;` (UUIDv4 simples).

---

## T-4: 3 Tools Canônicas + Dispatcher

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs`

**DoD:**
- [ ] `tools/list` substituído:
  - `export_session`: "Exporta a árvore relacional de pensamentos socráticos de uma sessão em formato estruturado (JSON/Markdown)." (≤120 chars)
  - `analyze_session`: "Processa as métricas comportamentais e de revisão de hipóteses socráticas de uma sessão na RAM." (≤120 chars)
  - `merge_sessions`: "Executa a fusão atômica de ramificações e fluxos de raciocínio concorrentes sob consistência eventual." (≤120 chars)
- [ ] Dispatcher com aliases: `name | souls_name | ctx_name`.
- [ ] Handlers:
  - `run_souls_export_session` — query + reconstruct + render JSON/Markdown.
  - `run_souls_analyze_session` — query + computa revision_rate/branching/latency.
  - `run_souls_merge_sessions` — BEGIN EXCLUSIVE + INSERT OR IGNORE + remap parent + COMMIT.
- [ ] Validação rigorosa: `session_id` não-vazio, ≤128 chars.

---

## T-5: Análise (analytics.rs)

**Arquivo:** `src-tauri/src/cognition/thinking/analytics.rs` (novo)

**DoD:**
- [ ] Struct `SessionMetrics { revision_rate, branching_factor, latency_mean_ms, total_thoughts, branch_count }`.
- [ ] Função `pub fn compute_metrics(thoughts: &[SocraticThought]) -> SessionMetrics` pura, sem I/O.
- [ ] Lei 0/0: divisão por zero defendida (default 0.0 se `|T| == 0`).

---

## T-6: Quitação Harvester (BLOCO 4)

**Arquivo:** `src-tauri/src/harvester/sast/native_ast.rs`

**DoD:**
- [ ] `SCORE_URL_RULES: &[(&str, i32)]` const no escopo de módulo, extraído do `score_external_doc_url`.
- [ ] `score_external_doc_url` itera sobre a const (DRY).
- [ ] 4 testes unitários para Camada A (keywords que promovem `kind: SkillLibrary`):
  - `test_skill_signal_skills_for_ai`
  - `test_skill_signal_coding_agents`
  - `test_skill_signal_diagram`
  - `test_skill_signal_visualization`

---

## T-7: 4 Testes TDD (souls_mcp_server.rs)

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (módulo `tests`)

**DoD:**
- [ ] `test_database_migration_v5`: idempotente; FK rejeita insert inválido.
- [ ] `test_export_session_formatting`: Tese→Antítese→Síntese; JSON e Markdown com indentação correta.
- [ ] `test_analyze_session_metrics`: equações de contagem e latência validadas.
- [ ] `test_merge_sessions_atomic_last_write_wins`: 2 branches fundidas; ponteiros reconciliados.
- [ ] Todos serializados pelo `TELEMETRY_TDD_LOCK` (race contra StateDbWorker).

---

## T-8: Validação Fast Pass

**Comando:** `cargo test --bin souls_mcp_server`

**DoD:**
- [ ] 39/39 testes verdes (35 baseline + 4 novos) em < 0.2s.
- [ ] `cargo test --lib harvester::sast::native_ast` 15 + 4 novos = 19/19 verdes.
- [ ] Zero novos warnings de clippy no código novo.

---

## T-9: Commit + Push + Atualiza PR #21

**DoD:**
- [ ] Commit com mensagem descritiva: `feat(cognition): Marco 3.9 Fase E — State DB V5 + Persistência Socrática + Harvester DRY`.
- [ ] Push para `origin/feat/wasm-callgraph-v1`.
- [ ] PR #21 atualizado automaticamente (sem merge; aguarda HITL).
