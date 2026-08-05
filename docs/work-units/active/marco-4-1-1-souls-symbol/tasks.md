---
spec: marco-4-1-1-souls-symbol
phase: 3-tasks
design: docs/work-units/active/marco-4-1-1-souls-symbol/design.md
branch: TRAE-IDE
---

# Tasks — Marco 4.1.1: `souls_symbol` (Motor Sensorial de Assinaturas)

Cada task tem um DoD (Definition of Done) executável. Tasks marcadas `[RED]` exigem teste vazio de falha antes da lógica real (Lei do Scaffold do SDD). O fluxo é **Red → Green → Refactor → Wire → Validate**.

---

## TASK-01 — `[RED]` Caderno TDD com 3 contratos rígidos

**Arquivo:** `src-tauri/tests/test_souls_symbol.rs` (NOVO)

**Escopo:** Criar o caderno de testes com 3 contratos que devem falhar antes da implementação (Red puro).

- [ ] `test_resolve_symbol_struct` — cria arquivo temp com `pub struct TestCore;`, busca por `TestCore`, espera `(file, line, col)` exato.
- [ ] `test_symbol_comment_protection` — arquivo com `/* fn TargetCommented() */` + `fn TargetActive()`. `TargetCommented` deve falhar (NotFound), `TargetActive` deve passar.
- [ ] `test_symbol_empty_or_invalid_workspace` — busca de símbolo inexistente + arquivo corrompido, ambos retornam erro tratado (sem panic).

**DoD:**
- Arquivo existe
- `cargo test --test test_souls_symbol` retorna **3 falhas** (Red) com `symbol` ainda não implementado no novo módulo
- Função-alvo: `souls_mc_lib::cognition::lean_vacuum::souls_symbol::resolve_symbol`

---

## TASK-02 — `[GREEN]` Módulo nativo `souls_symbol.rs`

**Arquivo:** `src-tauri/src/cognition/lean_vacuum/souls_symbol.rs` (NOVO)

**Escopo:** Implementação canibalizada: WalkDir + 22 extensões canônicas + 3 regex `OnceLock` + validação via WasmEngine global.

- [ ] `pub fn resolve_symbol(root: &Path, name: &str) -> Result<Option<SymbolLocation>, SymbolError>`
- [ ] `pub struct SymbolLocation { pub file: PathBuf, pub line: usize, pub col: usize, pub kind: SymbolKind }`
- [ ] `pub enum SymbolKind { Struct, Fn, Class, Def, Unknown }`
- [ ] `pub enum SymbolError { NotFound, InvalidInput(String) }`
- [ ] `static DECL_REGEX: OnceLock<Regex>` — pattern `r"\b(?:struct|fn|class|def)\s+<NAME>\b"` compilado em `init` (build lazy)
- [ ] Reusa `extensions::{is_source_ext, is_excluded_dir}`
- [ ] Reusa `wasm_engine::WasmEngine::global()` + `WASM_RUST_GRAMMAR` para validação
- [ ] Validação AST: se o match regex cai dentro de `comment` ou `string`, pula (fail-soft)
- [ ] Limite de 256 chars no `name` (alinhado ao dispatcher existente)

**DoD:**
- `cargo test --test test_souls_symbol` retorna **3 verdes** (Green)
- `cargo check --all-targets` Exit Code 0
- Zero warnings

---

## TASK-03 — Exposição do módulo em `lean_vacuum/mod.rs`

**Arquivo:** `src-tauri/src/cognition/lean_vacuum/mod.rs` (EDIT)

**Escopo:** Adicionar `pub mod souls_symbol;` e re-exports públicos.

- [ ] `pub mod souls_symbol;`
- [ ] `pub use souls_symbol::{resolve_symbol, SymbolLocation, SymbolKind, SymbolError};`

**DoD:**
- `cargo check` Exit Code 0
- Testes TDD continuam verdes (módulo exposto)

---

## TASK-04 — `[WIRE]` Dispatcher MCP: `run_souls_symbol`

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)

**Escopo:** Adicionar handler JSON-RPC `run_souls_symbol` que delega ao módulo nativo, mantendo a forma de retorno JSON-RPC padrão do gateway.

- [ ] `async fn run_souls_symbol(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError>`
- [ ] Dispatcher: `let "symbol" | "souls_symbol" | "ctx_symbol" => run_souls_symbol(params).await,`
- [ ] `tools/list`: entrada `souls_symbol` com `description` ≤ 120 chars e `inputSchema` válido
- [ ] **Remover** o stub antigo `run_symbol` (linha 1571) — substituído por `run_souls_symbol`
- [ ] Remover `"symbol" | "souls_symbol" | "ctx_symbol" => run_symbol(params).await,` do dispatcher (linha 818)

**DoD:**
- `cargo check --all-targets` Exit Code 0
- Teste existente `tools_list_cura_3_falsos_verdes` continua verde (description do `symbol` ainda menciona `O(1)` ou similar)
- Aliases `symbol` / `ctx_symbol` / `souls_symbol` continuam funcionando

---

## TASK-05 — `[VALIDATE]` Master Validation

**Escopo:** Compilar e testar a suite inteira.

- [ ] `cd src-tauri && cargo test --test test_souls_symbol` → **3 verdes**
- [ ] `cd src-tauri && cargo test --workspace` → **Exit Code 0** (zero regressão)
- [ ] `cd src-tauri && cargo clippy --workspace --all-targets -- -D warnings` → **Exit Code 0, zero warnings**

**DoD Final (Marco 4.1.1):**
- 3 testes novos verdes
- ~587+ testes totais verdes no workspace
- Zero clippy warnings
- Zero `not_implemented_yet` para `symbol` em `tools/list`

---

## TASK-06 — Blast Radius Report

- [ ] `git diff --stat` capturado
- [ ] Listar arquivos novos / editados em formato HITL:
  - **NEW** `src-tauri/src/cognition/lean_vacuum/souls_symbol.rs`
  - **NEW** `src-tauri/tests/test_souls_symbol.rs`
  - **EDIT** `src-tauri/src/cognition/lean_vacuum/mod.rs` (+3 linhas)
  - **EDIT** `src-tauri/src/bin/souls_mcp_server.rs` (dispatcher + handler)
- [ ] Aguardar aprovação do Arquiteto antes do PR Semântico
