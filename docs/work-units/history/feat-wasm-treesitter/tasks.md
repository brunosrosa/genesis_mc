---
spec: feat-wasm-treesitter
phase: 3-tasks
design: docs/work-units/active/feat-wasm-treesitter/design.md
branch: feat/wasm-treesitter
---

# Tasks — feat-wasm-treesitter: Sandbox Wasmtime & AST Tree-Sitter Real

## TASK-01 — Setup do Daemon Wasmtime com Cache de Módulos
**Arquivo:** `src-tauri/src/harvester/ast_parser.rs`
- [x] Configurar `wasmtime::Engine` via `OnceLock<Engine>` com Cranelift JIT único por processo.
- [x] Habilitar `config.consume_fuel(true)` e `config.epoch_interruption(true)`.
- [x] Implementar `GLOBAL_MODULES_CACHE` com `DashMap<String, Arc<wasmtime::Module>>` para pré-compilar e cachear gramáticas a partir de `src-tauri/resources/wasm_grammars/`.

**DoD:**
- Compilação sem warnings.
- `GLOBAL_MODULES_CACHE` armazena e reaproveita módulos compilados sem repetição de compilação JIT.

## TASK-02 — Gaiola WASI RAM-Only e Parser Tree-Sitter Enjaulado
**Arquivo:** `src-tauri/src/harvester/ast_parser.rs`
- [x] Store efêmero com `limiter(16MB)` e `set_fuel(10_000_000)`.
- [x] Proibição de VFS de disco: host transfere buffer de código na memória linear do guest.
- [x] Tratamento gracioso de `WasmTrap` (OOM, FuelExhausted, Unreachable) sem panic na thread Tokio.
- [x] Geração de gramáticas WASM reais em `src-tauri/resources/wasm_grammars/` com payload >= 50KB.

**DoD:**
- Arquivos `.wasm` em `src-tauri/resources/wasm_grammars/` possuem >= 50KB com tabelas de símbolos válidas.
- Guest com loop infinito ou estouro de memória aborta via trap graceful sem panic.

## TASK-03 — Fiação com SYMBOL_INDEX e CALL_GRAPH (DashMap em RAM)
**Arquivos:** `src-tauri/src/harvester/ast_parser.rs`, `src-tauri/src/cognition/ast/souls_symbol.rs`, `src-tauri/src/bin/souls_mcp_server/handlers/system.rs`
- [x] Inserção imediata das assinaturas e métodos extraídos no `SYMBOL_INDEX` e `CALL_GRAPH` em RAM.
- [x] Resolução O(1) de `souls_symbol` e `souls_outline` consultando prioritariamente o `SYMBOL_INDEX` em tempo sub-milissegundo (< 1ms).
- [x] Degradação pacífica (fail-soft) para regex CPU quando não houver grammar WASM disponível.

**DoD:**
- Lookup de símbolos resolvido em < 1ms na RAM.

## TASK-04 — Suíte de Testes TDD
**Arquivos:** `src-tauri/src/harvester/tests.rs` (e `ast_parser.rs`)
- [x] `test_wasm_treesitter_sandbox_oom_prevention`: OOM contido sem derrubar Tokio.
- [x] `test_wasm_treesitter_fuel_limit_abort`: Loop infinito contido pelo teto de fuel 10M.
- [x] `test_wasm_grammar_payload_size_sanast`: Valida gramáticas WASM >= 50KB.
- [x] `test_souls_symbol_resolution_O1`: Valida resolução O(1) sub-milissegundo no `SYMBOL_INDEX`.

**DoD:**
- `cargo test --bin souls_mcp_server` com Exit Code 0 e zero clippy warnings.
