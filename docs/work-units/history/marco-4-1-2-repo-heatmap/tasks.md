---
spec: marco-4-1-2-repo-heatmap-frecency-monitor
phase: 3-tasks
design: docs/work-units/active/marco-4-1-2-repo-heatmap/design.md
branch: TRAE-IDE
---

# Tasks — Marco 4.1.2: `repo_heatmap` (Monitor Termico de Frecency)

Cada task tem um DoD (Definition of Done) executavel. Tasks marcadas `[RED]` exigem teste vazio de falha antes da logica real (Lei do Scaffold do SDD). O fluxo e **Red → Green → Refactor → Wire → Validate → Intercept**.

---

## TASK-01 — `[RED]` Caderno TDD com 3 contratos rigidos

**Arquivo:** `src-tauri/tests/test_repo_heatmap.rs` (NOVO)

**Escopo:** Criar o caderno de testes com 3 contratos que devem falhar antes da implementacao (Red puro). Os testes usam `tempfile::TempDir` para workspaces sinteticos descartaveis e `tempfile`-based SQLite in-memory.

- [ ] `test_calculate_frecency_decay` — provar matematicamente que arquivo modificado ha 1h possui score estritamente maior que modificado ha 48h (`score_1h > score_48h`). Testa a formula pura `calculate_frecency(count, mtime, now, lambda) -> f64` antes de qualquer I/O.
- [ ] `test_heatmap_respects_exclusions` — `target/`, `.git/`, `node_modules/`, e arquivos com extensao nao-canonica (`.png`, `.log`, `.exe`) sao **imunes** a insercao na tabela `repo_heatmap`. Garante `score = 0.0` e ausencia de rows.
- [ ] `test_sqlite_upsert_collision_protection` — simula 2 threads paralelas fazendo UPSERT no mesmo `file_path` 100 vezes. Prova que o `ON CONFLICT(file_path) DO UPDATE` resolve a corrida sem panic, sem deadlock, e o `modification_count` final == 100.

**DoD:**
- Arquivo existe
- `cargo test --test test_repo_heatmap` retorna **3 falhas** (Red) com `repo_heatmap` ainda nao implementado
- Funcao-alvo primaria: `souls_mc_lib::cognition::lean_vacuum::repo_heatmap::{calculate_frecency, record_access, compute_repo_heatmap}`

---

## TASK-02 — `[GREEN]` Modulo nativo `repo_heatmap.rs`

**Arquivo:** `src-tauri/src/cognition/lean_vacuum/repo_heatmap.rs` (NOVO)

**Escopo:** Implementacao canibalizada: WalkDir filtrado + 22 extensoes canonicas + 22 exclusoes + calculo de decaimento + UPSERT SQLite STRICT.

- [ ] `pub const DEFAULT_LAMBDA: f64 = 0.0001;` (meia-vida ~1h55min)
- [ ] `pub const MAX_SCORE: f64 = 5.0;` (saturacao)
- [ ] `pub const MAX_FILES_SCAN: usize = 50_000;` (anti-OOM)
- [ ] `pub fn calculate_frecency(count: i64, mtime: i64, now: i64, lambda: f64) -> f64` — formula pura, deterministica, testavel
- [ ] `pub fn ensure_heatmap_table(conn: &Connection) -> Result<(), HeatmapError>` — `CREATE TABLE IF NOT EXISTS ... STRICT` + `CREATE INDEX IF NOT EXISTS ...` idempotente
- [ ] `pub fn record_access(conn: &Connection, file_path: &str, now: i64)` — hook fire-and-forget (R15-R17), NUNCA retorna Err ao caller, filtra por extensao canonica
- [ ] `pub fn compute_repo_heatmap(root: &Path, conn: &Connection, now: i64, lambda: f64, limit: usize) -> Result<HeatmapReport, HeatmapError>` — varredura WalkDir + UPSERT por arquivo + SELECT ranking final
- [ ] `pub struct HeatmapEntry { pub file_path: String, pub score: f64, pub modification_count: i64, pub last_modified_epoch: i64 }`
- [ ] `pub struct HeatmapReport { pub lambda: f64, pub now: i64, pub total: usize, pub entries: Vec<HeatmapEntry> }`
- [ ] `pub enum HeatmapError { InvalidPath(String), Io(String), Sqlite(String) }`
- [ ] Reusa `extensions::{is_excluded_dir, is_source_ext}` (22/22 SSOT)
- [ ] WalkDir com `filter_entry` (poda subarvore toxxica)
- [ ] `std::fs::metadata` para mtime O(1) nativo do SO
- [ ] UPSERT: `INSERT ... ON CONFLICT(file_path) DO UPDATE SET ...`
- [ ] Clamp `dt = max(0, now - mtime)` (anti-relogio-desregulado)
- [ ] Saturao `score = min(count * exp(-lambda * dt), MAX_SCORE)`

**DoD:**
- `cargo test --test test_repo_heatmap` retorna **3 verdes** (Green)
- `cargo check --all-targets` Exit Code 0
- Zero warnings

---

## TASK-03 — Exposicao do modulo em `lean_vacuum/mod.rs`

**Arquivo:** `src-tauri/src/cognition/lean_vacuum/mod.rs` (EDIT)

**Escopo:** Adicionar `pub mod repo_heatmap;` e re-exports publicos.

- [ ] `pub mod repo_heatmap;`
- [ ] `pub use repo_heatmap::{calculate_frecency, record_access, compute_repo_heatmap, ensure_heatmap_table, HeatmapEntry, HeatmapReport, HeatmapError, DEFAULT_LAMBDA, MAX_SCORE, MAX_FILES_SCAN};`

**DoD:**
- `cargo check` Exit Code 0
- Testes TDD continuam verdes (modulo exposto)

---

## TASK-04 — `[WIRE]` Dispatcher MCP: `run_repo_heatmap`

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)

**Escopo:** Adicionar handler JSON-RPC `run_repo_heatmap` que delega ao modulo nativo. Registrar entrada no `tools/list`. Adicionar aliases no dispatcher.

- [ ] `async fn run_repo_heatmap(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError>` — handler que extrai `limit`, `lambda`, `repo_path` dos argumentos, abre `souls_state.db`, chama `ensure_heatmap_table` + `compute_repo_heatmap`, retorna JSON-RPC
- [ ] Dispatcher: `"repo_heatmap" | "souls_heatmap" | "ctx_heatmap" => run_repo_heatmap(params).await,` (adicionar ao match, **sem remover** `"heatmap" | "souls_heatmap" => run_heatmap(params).await,` legado - R12)
- [ ] `tools/list`: nova entrada `repo_heatmap` com `description` exatamente: `"Calcula o ranking de calor (Frecency) dos arquivos do monorepo baseando-se em modificacoes e acessos."` (107 chars, ≤ 120 ✓) e `inputSchema` com `repo_path`, `limit`, `lambda`
- [ ] `tools/list` continua expondo `heatmap` legado (R12 - nao-regressao)

**DoD:**
- `cargo check --all-targets` Exit Code 0
- Aliases `repo_heatmap` / `souls_heatmap` / `ctx_heatmap` funcionam no dispatcher
- `heatmap` legado continua funcionando
- Description da nova tool tem exatamente 107 chars (sem marketing, sem "not_implemented_yet")

---

## TASK-05 — `[INTERCEPT]` Hook de Interceptacao Cognitiva (R15)

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)

**Escopo:** Invocar `lean_vacuum::record_access(&conn, &file_path, now)` silenciosamente apos chamadas bem-sucedidas de 6 tools canonicas (read, edit, symbol, repo_impact, repo_ast, multi_read).

- [ ] Helper `fn try_record_repo_heatmap(file_path: &str)` que abre `souls_state.db`, captura `now`, e invoca `record_access` em modo fire-and-forget (try-block que absorve qualquer erro)
- [ ] Apos `try_log_file_access(path_str, "read")` no `run_souls_read` (linha 865), adicionar `try_record_repo_heatmap(path_str);`
- [ ] Apos `try_log_file_access` no `run_souls_edit`, adicionar `try_record_repo_heatmap(path_str);`
- [ ] Apos resolver symbol em `run_souls_symbol`, adicionar `try_record_repo_heatmap(loc.file.to_str().unwrap_or(""));` (somente se `Some(loc)`)
- [ ] Apos `repo_impact_fn` em `run_repo_impact`, adicionar `try_record_repo_heatmap(&report.target_file);`
- [ ] Apos extrair AST em `run_repo_ast`, adicionar `try_record_repo_heatmap(repo_path_raw);`
- [ ] Apos cada path lido em `run_souls_multi_read`, adicionar `try_record_repo_heatmap(path_str);`

**DoD:**
- `cargo check --all-targets` Exit Code 0
- Testes TDD existentes continuam verdes
- Hook e fire-and-forget: NUNCA propaga erro ao caller

---

## TASK-06 — `[VALIDATE]` Master Validation

**Escopo:** Compilar e testar a suite inteira.

- [ ] `cd src-tauri && cargo test --test test_repo_heatmap` → **3 verdes**
- [ ] `cd src-tauri && cargo test --workspace` → **Exit Code 0** (zero regressao, todos os testes pre-existentes continuam verdes)
- [ ] `cd src-tauri && cargo clippy --workspace --all-targets -- -D warnings` → **Exit Code 0, zero warnings**

**DoD Final (Marco 4.1.2):**
- 3 testes novos verdes
- Suite master workspace verde
- Zero clippy warnings
- Tabela `repo_heatmap` criada em `souls_state.db` apos a 1ª chamada
- Description da tool `repo_heatmap` no `tools/list` exatamente: `"Calcula o ranking de calor (Frecency) dos arquivos do monorepo baseando-se em modificacoes e acessos."`
- Ferramenta legada `heatmap` continua funcionando (R12)

---

## TASK-07 — Blast Radius Report

- [ ] `git diff --stat` capturado
- [ ] Listar arquivos novos / editados em formato HITL:
  - **NEW** `docs/work-units/active/marco-4-1-2-repo-heatmap/design.md`
  - **NEW** `docs/work-units/active/marco-4-1-2-repo-heatmap/tasks.md`
  - **NEW** `src-tauri/src/cognition/lean_vacuum/repo_heatmap.rs`
  - **NEW** `src-tauri/tests/test_repo_heatmap.rs`
  - **EDIT** `src-tauri/src/cognition/lean_vacuum/mod.rs` (+3 linhas)
  - **EDIT** `src-tauri/src/bin/souls_mcp_server.rs` (dispatcher + tools/list + 6 hooks de interceptacao)
- [ ] Aguardar aprovacao do Arquiteto antes do PR Semantico
