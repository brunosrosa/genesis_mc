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
