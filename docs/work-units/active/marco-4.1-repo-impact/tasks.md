---
spec: marco-4.1-repo-impact
phase: 4-tasks
design: docs/work-units/active/marco-4.1-repo-impact/design.md
branch: TRAE-IDE
---

# Tasks — Marco 4.1.0 Motor Sensorial de Blast Radius `repo_impact`

Lei do Scaffold: cada task marcada `[SCAFFOLD-RED]` exige um teste
vazio de falha antes da lógica real. TDD atômico (Red → Green →
Refactor) sob `cargo test --test test_repo_impact`. Zero `MutexGuard`
sobre `.await`. Zero `serde_json::from_slice` > 1 MB. Zero drift de
`EXCLUDE_DIRS`/`SOURCE_EXTENSIONS`.

---

## TASK-01 — `[SCAFFOLD-RED]` Contrato `direct_dependents`

**Arquivo:** `src-tauri/tests/test_repo_impact.rs` (NOVO)

- [ ] Stub de teste vazio com 3 arquivos temporários no disco
      (`A.rs → B.rs → C.rs` via `tempfile::TempDir`).
- [ ] Invoca `repo_impact(file_path="C.rs", max_depth=3)`.
- [ ] Asserto: `B` aparece em `impact_graph.nodes` no nível 1.
- [ ] Asserto: `A` aparece em `impact_graph.nodes` no nível 2.
- [ ] Asserto: `target_file == "C.rs"`.

**DoD:**
- `cargo test --test test_repo_impact test_repo_impact_direct_dependents`
  retorna **FAIL** (Red), confirmando que o módulo ainda não compila.
- Zero warnings no `cargo build --tests`.

---

## TASK-02 — `[SCAFFOLD-RED]` Contrato `cyclic_protection`

**Arquivo:** `src-tauri/tests/test_repo_impact.rs` (EDIT)

- [ ] Stub de teste com loop A↔B.
- [ ] Invoca `repo_impact(file_path="A.rs", max_depth=10)`.
- [ ] Asserto: BFS encerra em O(V+E) sem Stack Overflow.
- [ ] Asserto: `B` aparece exatamente **1 vez** em `nodes` (dedup).
- [ ] Asserto: nenhum panico, nenhum loop infinito.

**DoD:**
- `cargo test --test test_repo_impact test_repo_impact_cyclic_protection`
  retorna **FAIL** (Red), confirmando que `HashSet<PathBuf>` ainda
  não foi soldado.

---

## TASK-03 — `[SCAFFOLD-RED]` Contrato `respects_max_depth`

**Arquivo:** `src-tauri/tests/test_repo_impact.rs` (EDIT)

- [ ] Cadeia A → B → C → D.
- [ ] Invoca `repo_impact(file_path="D.rs", max_depth=1)`.
- [ ] Asserto: `C` presente em `nodes` (nível 1).
- [ ] Asserto: `A` e `B` **ausentes** de `nodes` (níveis 2 e 3).
- [ ] Asserto: `max_depth_reached == 1`.

**DoD:**
- `cargo test --test test_repo_impact test_repo_impact_respects_max_depth`
  retorna **FAIL** (Red), confirmando que o gate `max_depth` ainda
  não foi implementado.

---

## TASK-04 — `[GREEN]` Módulo `lean_vacuum::repo_impact`

**Arquivo:** `src-tauri/src/cognition/lean_vacuum/repo_impact.rs` (NOVO)

- [ ] `pub struct ImpactReport { target, total, max_depth, nodes, edges }`.
- [ ] `pub fn repo_impact(root: &Path, target: &Path, max_depth: u8) -> ImpactReport`.
- [ ] WalkDir filtrado por `is_excluded_dir` (22) + `is_source_ext` (22).
- [ ] `ImportExtractor` multilíngue (regex `use|import|require|from`).
- [ ] `ImportGraph = BTreeMap<PathBuf, Vec<PathBuf>>`.
- [ ] Transpor arestas, BFS reverso com `HashSet<PathBuf>` visited.
- [ ] `thiserror::Error` para `RepoImpactError` (path inválido, I/O).
- [ ] `Cargo.toml`: zero novas deps; reusa `regex`, `walkdir`,
      `thiserror`, `serde`.

**DoD:**
- `cargo check -p souls_mc` retorna **Exit Code 0**.
- 5 testes unitários internos verdes (parser, grafo, transpor,
  cycle cut, max_depth gate).

---

## TASK-05 — `[GREEN]` Registro no `lean_vacuum::mod`

**Arquivo:** `src-tauri/src/cognition/lean_vacuum/mod.rs` (EDIT)

- [ ] `pub mod repo_impact;` (ordem alfabética).
- [ ] `pub use repo_impact::{repo_impact as repo_impact_fn, ImpactReport};`
- [ ] Const `pub const DEFAULT_MAX_DEPTH: u8 = 3;`

**DoD:**
- `cargo check` **Exit Code 0**.
- `cargo doc --no-deps` reconhece o símbolo público.

---

## TASK-06 — `[GREEN]` Schema `repo_impact` no `tools/list`

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)

- [ ] Inserir entrada `repo_impact` no array `tools` antes de `heatmap`
      (bloco Observabilidade Cognitiva Sensorial).
- [ ] Descrição: `"Analisa o raio de impacto (Blast Radius) de alterações de arquivos via travessia reversa de dependências."` (120 chars, sem marketing).
- [ ] `inputSchema` com `file_path` (string, required) +
      `max_depth` (integer, default 3, min 1, max 10).
- [ ] Preservar aliases `souls_impact` e `ctx_impact` no array
      `tools` (mesma description, mesmo schema, `additionalProperties: false`).

**DoD:**
- `cargo check` **Exit Code 0**.
- Teste unitário (no próprio `souls_mcp_server.rs` ou em test de
  integração) confirma: `tools/list` contém `repo_impact`,
  `souls_impact`, `ctx_impact` com descrições idênticas.

---

## TASK-07 — `[GREEN]` Dispatcher unificado em `run_repo_impact`

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)

- [ ] Função `async fn run_repo_impact(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError>`.
- [ ] Extrai `file_path` (obrigatório) e `max_depth` (default 3,
      clamp 1..=10).
- [ ] Resolve `file_path` relativamente a `workspace_root()` se não
      for absoluto.
- [ ] Chama `lean_vacuum::repo_impact_fn(&root, &target, max_depth)`.
- [ ] Serializa payload canônico (1×) com `serde_json::json!`.
- [ ] Dispatcher: `"repo_impact" | "souls_impact" | "ctx_impact"
      => run_repo_impact(params).await`.
- [ ] Remover `run_impact` legado (canibalizado pelo novo).
- [ ] Atualizar stubs de teste em `handle_tool_call` (se houver).

**DoD:**
- `cargo check` **Exit Code 0**.
- `cargo test` (todos os módulos de teste) **Exit Code 0**.
- Aliases `souls_impact` e `ctx_impact` retornam o **mesmo payload**
  que `repo_impact` (validado por novo teste no test_repo_impact.rs).

---

## TASK-08 — `[REFACTOR]` Garantir Agnosticismo e 0 warnings

- [ ] `pub use` apenas o necessário; sem `pub` acidental.
- [ ] Doc comments (`///`) em todos os símbolos públicos.
- [ ] `const _: () = assert!(...);` invariantes em compile-time para
      defaults (ex.: `DEFAULT_MAX_DEPTH <= 10`).
- [ ] Zero `unwrap()` no hot path (apenas em testes e no boundary MCP
      com mensagem de erro clara).
- [ ] Zero `clippy::needless_*` ou `clippy::pedantic` não suprimido.

**DoD:**
- `cargo clippy --workspace --all-targets -- -D warnings` retorna
  **Exit Code 0** com **0 warnings**.

---

## TASK-09 — Validação do Silício (Green Real)

- [ ] `cargo clean` para garantir cold build.
- [ ] `cargo test --test test_repo_impact -- --nocapture` → 3
      contratos verdes.
- [ ] `cargo test --workspace` → 100% verde, sem regressão.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` →
      **0 warnings**.
- [ ] `git status --short` lista apenas os arquivos da work unit
      ativa.

**DoD:**
- **Exit Code 0** em todos os comandos acima.
- Laudo Green em `.souls_scratchpad/reports/_PHASE_REPORT_marco-4.1.0.txt`.

---

## TASK-10 — Blast Radius Report + HITL

- [ ] `git diff --stat` capturado e publicado no report.
- [ ] Lista dos arquivos novos/alterados:
      - `src-tauri/src/cognition/lean_vacuum/repo_impact.rs` (NOVO)
      - `src-tauri/src/cognition/lean_vacuum/mod.rs` (EDIT)
      - `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)
      - `src-tauri/tests/test_repo_impact.rs` (NOVO)
- [ ] **NÃO** fazer merge. Aguardar aprovação do Arquiteto para
      Rebase Semântico em direção a `TRAE-IDE`.

**DoD:**
- Mensagem HITL gerada e entregue via `notify_user`.
