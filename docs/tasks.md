# TASKS: SOULS V4 — UPGRADE DE MOTORES DE INFERÊNCIA (SPIKE/BUMP-LLAMA-CPP)

## Tasks & Definition of Done (DoD)

- [ ] **Task 1: Isolamento de Branch (BMAD)**
  - **DoD:** Executar `git checkout -b spike/bump-llama-cpp` e confirmar via `git branch` que a branch ativa é `spike/bump-llama-cpp`.

- [ ] **Task 2: Vendorização do llama-cpp-2 v0.1.153**
  - **DoD:** Mover/baixar código do `llama-cpp-2 v0.1.153` em `src-tauri/vendor/llama-cpp-2`. Atualizar `src-tauri/Cargo.toml` com a patch `llama-cpp-2 = { path = "vendor/llama-cpp-2" }`. Validar compatibilidade CUDA 13.3 + MSVC 14.51 em `build.rs`.

- [ ] **Task 3: Revisão do KV Cache e APIs em llama_engine.rs**
  - **DoD:** Atualizar `src-tauri/src/core/llama_engine.rs` para adaptar à API do `llama-cpp-2 v0.1.153`. Garantir que `build_context_params_with_fallback` configure Key Cache em `KvCacheType::F16` e Value Cache em `KvCacheType::Q4_K` / `KvCacheType::Q8_0`.

- [ ] **Task 4: Estruturação do Subprocesso CPU BitNetDaemon**
  - **DoD:** Criar `src-tauri/src/core/bitnet_daemon.rs` implementando a struct `BitNetDaemon` com gerenciamento de subprocesso via Tokio (`tokio::process::Child`) e implementando `Drop` com `child.start_kill()` / `kill()` para garantir destruição atômica. Registrar o módulo em `src-tauri/src/core/mod.rs`.

- [ ] **Task 5: Cobertura TDD e Validação Bare-Metal**
  - **DoD:** Implementar e passar nos 3 testes unitários mandatórios:
    1. `test_llama_engine_context_init_asymmetric_kv_cache`
    2. `test_bitnet_daemon_lifecycle_sigkill`
    3. `test_cuda_msvc_build_compatibility`
  - **DoD Final:** `cargo check --features llama_backend` e `cargo clippy --features llama_backend -- -D warnings` executados sem erros ou warnings.

---

## Milestone: Universalização AST Poliglota & OXC Routing

- [ ] **Task 1: Roteamento de Alto Rendimento JS/TS via OXC em `ast_parser.rs`**
  - **DoD:** Implementar `extract_with_oxc` em `ast_parser.rs` utilizando `oxc::allocator::Allocator` e `oxc::parser::Parser`. Rotear extensões `.js`, `.ts`, `.jsx`, `.tsx` diretamente para a AST do OXC com alocação zero-copy na arena.

- [ ] **Task 2: Executor Sandbox Wasmtime para Gramáticas Tree-Sitter WASM**
  - **DoD:** Implementar `WasmtimeTreeSitterEngine` em `ast_parser.rs` utilizando a crate `wasmtime`. Carregar gramáticas `.wasm` de `.souls_data/wasm_grammars/` ou `resources/wasm_grammars/` de forma lazy, capturando qualquer trap/erro e retornando fallback limpo sem panic no host.

- [ ] **Task 3: Atualização de Rotas e Fallback Fail-Soft em `extract_structural_signatures`**
  - **DoD:** Rotear JS/TS -> OXC, C# -> Native Tree-Sitter, Rust/Python/Go/Elixir -> WASM Tree-Sitter (com fallback estrito se o arquivo `.wasm` estiver ausente).

- [ ] **Task 4: Cobertura de Testes TDD Mandatórios**
  - **DoD:** Implementar os 3 testes unitários mandatórios (`test_oxc_js_ts_outline`, `test_wasm_tree_sitter_rust_outline`, `test_fail_soft_corrupted_wasm_grammar`) e obter Exit Code 0 em `cargo test --lib --features "tauri-app,gateway_ccr,llama_backend"`.

