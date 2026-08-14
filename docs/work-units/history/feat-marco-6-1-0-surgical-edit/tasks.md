---
marco: 6.1.0
status: tasks_aprovado
data: 2026-08-09
---

# MARCO 6.1.0 — Tasks (TDD Atômico)

> Lei do Scaffold: cada tarefa abaixo exige que a infraestrutura executável
> (teste vazio em vermelho) exista **antes** da lógica. Nenhum `cargo check`
> deve passar antes de a tarefa RED correspondente ter falhado.

## T1 — Catalogar `edit`/`replace` em `tools.rs` (RED → GREEN)

- **RED:** Adicionar um teste `test_tools_list_includes_edit_and_replace`
  em `tests.rs` que invoca `tools::list_tools()` e asserta a presença dos
  dois tool names com as descrições literais especificadas pela ADR-041
  (108 e 111 chars respectivamente). Confirma `cargo test` falha.
- **GREEN:** Atualizar `list_tools()` em
  `src-tauri/src/bin/souls_mcp_server/tools.rs` com:
  - `edit` description exata (108 chars)
  - `replace` description exata (111 chars)
  - Ambos com schema `path`/`old_string`/`new_string`/`verify_ast`
- **DoD:** `cargo test test_tools_list_includes_edit_and_replace` GREEN.

## T2 — `run_souls_replace` em `handlers/system.rs` (RED → GREEN)

- **RED:** Adicionar `test_replace_successful_block_mutation` em `tests.rs`
  que invoca `handle_mcp` com `name: "replace"` sobre uma fixture, e asserta
  que a substituição foi aplicada. Confirma falha antes do handler existir.
- **GREEN:** Implementar `pub async fn run_souls_replace(...)` em
  `handlers/system.rs` reusando `file_locker::acquire_file_lock` +
  `atomic_write_file`. Mesma assinatura e contratos de erro de
  `run_souls_edit`.
- **DoD:** `cargo test test_replace_successful_block_mutation` GREEN.

## T3 — `verify_ast` em `run_souls_edit` (RED → GREEN)

- **RED:** Adicionar `test_edit_optional_verify_ast_keeps_valid_file` em
  `tests.rs` que ativa `verify_ast: true` em uma fixture `.rs` válida e
  assevera sucesso.
- **GREEN:** Adicionar parâmetro `verify_ast` opcional em `run_souls_edit`,
  e em caso positivo, após `atomic_write_file`, invocar
  `WasmTimeTreeSitterValidator::validate` (novo em `core/file_locker.rs`)
  sobre o conteúdo gravado. Em falha → rollback via `snapsafe_restore` e
  `RpcError` com `code: -32002` (UntrustedExecutionBlocked).
- **DoD:** `cargo test test_edit_optional_verify_ast_keeps_valid_file`
  GREEN, com `verify_ast: true` rejeitando código quebrado em
  `test_edit_failed_syntax_rollback`.

## T4 — `snapsafe_create_hardlink` em `core/file_locker.rs`

- Adicionar função pública:
  ```rust
  pub fn snapsafe_create_hardlink(target: &Path) -> Result<PathBuf, std::io::Error>
  pub async fn snapsafe_restore(snapshot: &Path, target: &Path) -> Result<(), std::io::Error>
  ```
- **DoD:** Teste unitário em `file_locker.rs` que cria um snapshot, modifica
  o original, restaura e asseira que `target` volta ao conteúdo original
  bit-a-bit.

## T5 — `WasmTimeTreeSitterValidator` em `core/file_locker.rs`

- Adicionar:
  ```rust
  pub struct WasmTimeTreeSitterValidator;
  impl WasmTimeTreeSitterValidator {
      pub fn validate(path: &Path, source: &str) -> Result<(), CognitiveError>
  }
  ```
- A implementação carrega o `tree-sitter-rust` (já em deps como
  `tree-sitter-c-sharp` / `tree-sitter-yaml`) **OU** usa o engine `tree-sitter`
  puro diretamente (canibalizado de `harvester/ast_parser.rs`). Se o arquivo
  não for `.rs`/`.svelte`, retorna `Ok(())` (fail-soft — apenas Rust/Svelte
  têm validador ativo nesta release).
- **DoD:** Teste de unidade que valida `"fn main() { println!(\"x\"); }"`
  retorna `Ok`, e `"fn main() {"` retorna `Err`.

## T6 — Testes de Integração (3 testes físicos)

Localização: `src-tauri/src/bin/souls_mcp_server/tests.rs`.

1. `test_edit_exact_block_match_validation` — Prova casamento exato
   (sucesso com string correta, erro -32001 com string off-by-one).
2. `test_edit_tokio_path_lock_concurrency` — Dispara 20 tasks Tokio
   concorrentes contra o mesmo `PathBuf` e assevera que o lock serializou
   sem bytes perdidos (soma de bytes final == soma esperada).
3. `test_edit_failed_syntax_rollback` — Insere `fn main() {` em uma
   fixture `.rs` com `verify_ast: true`; asseira erro e prova via
   `std::fs::read_to_string` que o conteúdo original foi restaurado pelo
   snapsafe.

## T7 — Validação Final (ADR-025)

- `cargo check --all-targets` deve passar.
- `cargo clippy --all-targets -- -D warnings` deve passar (0 warnings).
- `cargo test --release` deve passar todos os 3 testes novos + os
  existentes (regressão zero).
- `cargo build --bin souls_mcp_server` deve produzir um binário válido.
- Wall-time total: < 60s em release / < 30s em dev.

## Anti-Consenso (Fase 5)

Após T7 GREEN, **NÃO** fazer merge. Compilar o **Blast Radius** (arquivos
tocados) e enviar para a Agent Inbox para aprovação HITL do Arquiteto.
