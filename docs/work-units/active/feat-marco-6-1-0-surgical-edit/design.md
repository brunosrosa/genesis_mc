---
marco: 6.1.0
titulo: Sistema de Escrita Cirúrgica Atômica (Edit/Replace)
adr_vinculadas: [ADR-010, ADR-025, ADR-027, ADR-030, ADR-041]
status: design_aprovado
data: 2026-08-09
---

# MARCO 6.1.0 — Sistema de Escrita Cirúrgica Atômica

## 1. Contexto & Linha Vermelha (SSOT)

Este marco implementa as duas garras de mutação textual expostas pelo Agent
Gateway Rust sob o servername soberano `souls_mcp`: `edit` e `replace`. Ambas
operam estritamente em CPU (ADR-027), sem alocação na dGPU, e devem satisfazer
as leis de proteção de I/O da ADR-010, a doutrina de qualidade 100/100 da
ADR-025 e o teto de nomenclatura da ADR-041.

**Linha Vermelha (Red Line):**
- Proibido adicionar novas dependências ao `Cargo.toml`. Toda a implementação
  deve **canibalizar** crates já presentes: `tokio`, `dashmap`, `serde_json`,
  `dunce`, `tempfile`, `uuid`, `tree-sitter`, `wasmtime`, `wasmtime-wasi`.
- Nenhuma mutação em disco pode ocorrer sem o lock assíncrono do `PathLockManager`.
- Em caso de falha do linter WASM, **rollback atômico** é obrigatório.
- A trava por `PathBuf` (canonizado via `dunce::canonicalize`) deve serializar
  todas as escritas concorrentes no mesmo arquivo (zero condições de corrida).

## 2. Topologia & Roteamento FinOps

```mermaid
flowchart TD
    A[JSON-RPC tools/call] --> B[router::normalize_tool_name]
    B --> C{Tool == edit|replace?}
    C -- "edit" --> D[run_souls_edit]
    C -- "replace" --> E[run_souls_replace]
    D --> F[PathLockManager.acquire]
    E --> F
    F --> G[snapsafe: hard-link snapshot]
    G --> H[Match exato de old_string]
    H -- "0 ou >1 ocorrências" --> I[Fail-Closed -32001]
    H -- "1 ocorrência" --> J[Compose conteúdo em tmp]
    J --> K{verify_ast == true?}
    K -- "sim" --> L[WASM tree-sitter validate]
    L -- "parser OK" --> M[atomic_write_file]
    L -- "parser falhou" --> N[rollback snap + UntrustedExecutionBlocked]
    K -- "não" --> M
    M --> O[Response OK]
```

**Padrão Orchestrator-Worker (ACONIC):**
- **Orchestrator:** `router::handle_tool_call` recebe a chamada, normaliza
  aliases e delega para o handler.
- **Worker (single-thread de I/O):** `run_souls_edit` / `run_souls_replace`
  adquirem o `Arc<tokio::sync::Mutex<()>>` específico do `PathBuf`
  canonizado, garantindo exclusão mútua serializada por arquivo.
- **Backup físico O(1):** `snapsafe_create_hardlink` cria um hard link NTFS
  do arquivo alvo antes da mutação, permitindo rollback atômico.
- **Validador WASM (CPU-only):** `WasmTimeTreeSitterValidator` carrega a
  gramática apropriada (`.rs` → Rust tree-sitter) sob `wasmtime` com
  `WASI 0.2` e executa o parser num fuel budget de 1.000.000. Em trap
  sintático → rollback.

## 3. Agnosticismo de Hardware (ADR-030)

A escolha de **tree-sitter** + **Wasmtime** é deliberadamente portável:
- O binário `.wasm` do tree-sitter é determinístico, igual em qualquer
  arquitetura (x86_64, ARM64, RISC-V).
- A validação sintática roda 100% em CPU com `fuel` limitada — zero dGPU.
- O `PathLockManager` usa exclusivamente `tokio::sync::Mutex`, que é
  transmutável entre Tokio (atual) e qualquer runtime embarcado.
- O backup `hard-link` é uma primitiva NTFS/ReFS/ext4 — agnóstico de GPU.

A RTX 2060m (6GB VRAM) é usada apenas como **treino de gravidade** (piso de
validação), nunca como dependência funcional do fluxo de edição.

## 4. Contrato JSON-RPC (ADR-041)

### Tool: `edit`
- **Nome:** `edit` (≤ 32 chars ✓)
- **Descrição (108 chars):** "Aplica edições cirúrgicas baseadas em casamento
  exato de blocos (Search and Replace) com proteção de travamento."
- **Schema:**
  - `path: string` (obrigatório)
  - `old_string: string` (obrigatório)
  - `new_string: string` (obrigatório)
  - `verify_ast: boolean` (opcional, default `false`)

### Tool: `replace`
- **Nome:** `replace` (≤ 32 chars ✓)
- **Descrição (111 chars):** "Substitui blocos textuais extensos sob
  verificação sintática e com rollback atômico em caso de falha de TDD."
- **Schema:** idêntico ao `edit`.

### Aliases retroativos (em `normalize_tool_name`)
- `souls_edit`, `ctx_edit`, `souls_mcp.edit` → `edit`
- `souls_replace`, `ctx_replace`, `souls_mcp.replace` → `replace`
- O stripping de prefixo `souls_` / `ctx_` / `souls_mcp.` já existe; logo
  **zero código novo** é necessário no router para os aliases.

## 5. Lei do Scaffold (DoD por tarefa)

| # | Tarefa | DoD |
|---|--------|-----|
| T1 | Atualizar descrições/schemas em `tools.rs` | `cargo check` + grep confirma strings exatas |
| T2 | Adicionar `run_souls_replace` em `handlers/system.rs` | Função compila, retorna `RpcError` em casos de borda |
| T3 | Adicionar `verify_ast` em `run_souls_edit` | AST ativável via flag opcional, fail-soft quando indisponível |
| T4 | Adicionar `snapsafe_create_hardlink` em `core/file_locker.rs` | Teste isolado prova O(1) e rollback |
| T5 | Adicionar `WasmTimeTreeSitterValidator` em `core/file_locker.rs` | Valida `.rs` real, retorna `Err` em fonte inválida |
| T6 | Adicionar 3 testes em `tests.rs` | RED → GREEN, ≤ 1s de wall-time |
| T7 | `cargo clippy -D warnings` e `cargo test` | 0 erros, 0 warnings, todos passam |

## 6. Riscos & Mitigações

| Risco | Mitigação |
|-------|-----------|
| Path traversal (`../../etc/passwd`) | `validate_and_canonicalize_path` já bloqueia |
| Hard-link cross-volume (ReFS) | `snapsafe_create_hardlink` faz fallback para `copy` |
| Fuel WASM excedido em arquivo gigante | `Parser::set_timeout_micros(50_000)` + `set_included_ranges` opcional |
| Gramática tree-sitter não carregada | `WasmTimeTreeSitterValidator::validate` retorna `Ok(())` (fail-soft) sem gramática |
| I/O race condition | `PathLockManager` (DashMap + tokio::Mutex) serializa por `PathBuf` |
