# Tasks — Marco 3.9 Fase E.2: Hardening E2E e Barramento Assíncrono Socrático

> Emenda ao [tasks-marco-3.9-e.md](tasks-marco-3.9-e.md).
> Rito: SDD estrito (Red → Green → Refactor). Cada task tem DoD binário (verificável).

## Task 1 — SocraticWriteWorker (MPSC bridge)
**DoD:**
- [x] Criar `src-tauri/src/cognition/thinking/socratic_bridge.rs` com:
  - `pub enum SocraticOp` (variantes: `UpsertSession`, `UpsertThought`, `UpsertThoughtFire`).
  - `pub fn spawn_socratic_write_worker(db_path: PathBuf) -> Result<mpsc::Sender<SocraticOp>, Box<dyn Error>>`.
- [x] Canal `tokio::sync::mpsc` bounded em **512 mensagens** (não 100 — para absorver pico de 10k).
- [x] Worker dedicado via `std::thread::spawn` consumindo `SocraticOp` com `blocking_recv`.
- [x] Migração V3→V5 idempotente no boot do worker.
- [x] Compile limpo: `cargo check -p souls_mc --bin souls_mcp_server` sem warnings novos.

## Task 2 — Extirpação do `MARCO_39_FASE_E_LOCK`
**DoD:**
- [x] Remover a global `static MARCO_39_FASE_E_LOCK: std::sync::Mutex<()>`.
- [x] Remover a função `marco_39_lock()` e todas as invocações em testes.
- [x] Atualizar os 4 testes TDD existentes (T1-T4) para usar o canal MPSC
      via helper público `spawn_socratic_write_worker_for_testing` (análogo a
      `init_state_db_for_testing`).
- [x] `grep -r MARCO_39_FASE_E_LOCK src/` retorna 0 matches.

## Task 3 — Wire do `SOCRATIC_TX` no `init_state_db_and_worker`
**DoD:**
- [x] Adicionar `static SOCRATIC_TX: OnceLock<mpsc::Sender<SocraticOp>>` em
      `bin/souls_mcp_server.rs`.
- [x] Chamar `spawn_socratic_write_worker` dentro de `init_state_db_and_worker`,
      armazenando o `Sender` em `SOCRATIC_TX`.
- [x] Modificar `run_souls_merge_sessions` para despachar `UpsertSession` +
      `UpsertThought` via `try_send` (Hiper-Forward, sem ACK).
- [x] `run_souls_export_session` e `run_souls_analyze_session` permanecem
      read-only (sem MPSC).
- [x] `cargo test --bin souls_mcp_server test_database_migration_v5` verde.

## Task 4 — Comandos Tauri v2 para Svelte 5
**DoD:**
- [x] Criar em `src-tauri/src/main.rs` os comandos:
  - `#[tauri::command] async fn socratic_export_session(session_id: String, format: Option<String>) -> Result<Value, String>`
  - `#[tauri::command] async fn socratic_analyze_session(session_id: String) -> Result<Value, String>`
  - `#[tauri::command] async fn socratic_merge_sessions(source_session_id: String, target_session_id: String) -> Result<Value, String>`
- [x] Todos retornam `Result<Value, String>` para que o renderer Svelte 5
      capture erros SQLite graciosamente sem travar o IPC.
- [x] Adicionar os 3 ao `tauri::generate_handler![...]`.
- [x] `cargo check -p souls_mc --bin souls_mc` verde (feature `tauri-app`).

## Task 5 — Stress Test 10k Pensamentos
**DoD:**
- [x] Escrever `test_socratic_load_10k_thoughts` em
      `src-tauri/src/bin/souls_mcp_server.rs` (módulo `tests`).
- [x] Cenário: cria 1 sessão, dispara 10.000 pensamentos encadeados via
      `try_send`, espera o worker drenar o canal, lê de volta do SQLite.
- [x] Assertiva 1: **nenhum pânico, deadlock ou violação de FK**.
- [x] Assertiva 2: **tempo de despacho (10k `try_send` em loop) < 500ms**
      (Hiper-Forward confirmado).
- [x] Assertiva 3: **10k pensamentos presentes no banco após drain**, com
      `step_number` único 1..=10000 e `parent_thought_id` formando cadeia.
- [x] `cargo test --bin souls_mcp_server test_socratic_load_10k_thoughts` verde.

## Task 6 — Higiene Térmica e Validação Final
**DoD:**
- [x] `cargo test --bin souls_mcp_server --no-default-features` roda **toda** a
      suíte verde em < 10s (Marcha Rápida, sem CUDA/Tauri).
- [x] `cargo clippy --bin souls_mcp_server --no-default-features -- -D warnings`
      limpo (sem `#[allow]` órfão, sem `unused`).
- [x] `grep -r MARCO_39_FASE_E_LOCK src/` retorna 0.
- [x] Blast Radius documentado: arquivos tocados, linhas adicionadas/removidas.
- [x] Commit na branch `feat/marco-39-socratic-hardening`.
- [x] PR via `gh pr create` para HITL.
