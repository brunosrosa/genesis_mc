# Tasks — feat-mcp-stdio-hygiene

## Status: COMPLETED (DoD Validado - 94/94 Testes OK, Clippy Zero Warnings)

### Definições de Tarefas e DoD (Definition of Done)

- [x] **Task 1: Contenção de Logs e Silenciador C++ FFI**
  - Configurado `tracing_subscriber` com direcionamento explícito para `std::io::stderr` em `main.rs`.
  - Ausência de `println!` soltos em todos os handlers MCP.
  - Aplicado silenciador `backend.void_logs()` em `LlamaBackend::init` (`llama_engine.rs`, `llama_logit_probing.rs`).
  - **DoD:** Compilação limpa e nenhum log emitido em stdout.

- [x] **Task 2: Despacho Assíncrono Supervisionado com Captura de Pânico**
  - Implementado isolamento via `tokio::spawn` em `handle_mcp` para `tools/call`.
  - Capturado `JoinError::is_panic()` e mapeado para JSON-RPC error `-32603`, message `"Internal error: Tool panicked in worker thread"`, `is_error: true`.
  - Adicionado suporte a `_simulate_panic` em `router.rs` para testes de injeção de falha.
  - **DoD:** Pânicos internos não derrubam a thread de transporte e retornam JSON-RPC determinístico.

- [x] **Task 3: Envelopamento de Chamadas de Disco em `spawn_blocking` e Flush Imediato**
  - Envelopado `resolve_symbol` em `run_souls_symbol` com `tokio::task::spawn_blocking`.
  - Envelopado parsing de assinaturas em `run_souls_outline` com `tokio::task::spawn_blocking`.
  - Garantido `.flush().await` imediato em `stdout` após cada payload JSON-RPC.
  - **DoD:** Zero bloqueio do reactor Tokio por I/O síncrono em Z:.

- [x] **Task 4: Suíte TDD de Saneamento de Canal**
  - Implementado `test_mcp_server_stdout_unpolluted` em `tests.rs`.
  - Implementado `test_mcp_handler_panic_unwind_safety` em `tests.rs`.
  - Executado `cargo test --bin souls_mcp_server` com 94/94 testes aprovados (Exit Code 0).
  - Executado `cargo clippy --bin souls_mcp_server` com zero warnings e log salvo em `.souls_scratchpad/logs/cargo/clippy_mcp_hygiene.log`.
  - **DoD:** Suíte 100% verde, zero warnings clippy e veredito final documentado.
