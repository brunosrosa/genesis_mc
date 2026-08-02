---
spec: v4-engines-mocks-and-compression-stubs
phase: 3-tasks
design: docs/fixes/design.md
branch: fix/v4-engines-mocks-and-compression-stubs
---

# Tasks — V4 Engines Mocks + Compression Stubs

Cada task tem um DoD (Definition of Done) executável. Tarefas marcadas `[SCAFFOLD]` exigem teste vazio de falha antes da lógica real (Lei do Scaffold). Engines novos sao stubs conformantes sob `EphemeralInferEngine` — compilam, retornam mock determinístico, e respeitam o `thermal_rx` watchdog.

## TASK-01 — Engine: LlamaCpp4LogitEngine (CPU AVX2, logit probing)

**Arquivo:** `src-tauri/src/core/llama_cpp4_logit.rs` (NOVO)

**Escopo:** Stub CPU-only que retorna logits brutos do último token do prefill, sem decodificar string. Usado pelo Hipocampo Epistêmico.

- [ ] Struct `pub struct LlamaCpp4LogitEngine`
- [ ] `impl EphemeralInferEngine for LlamaCpp4LogitEngine`
- [ ] `mock_logits: Vec<f32>` interno (128 entradas) com distribution determinística
- [ ] Respeita `thermal_rx` (watchdog `SystemState::Paused` → sleep 100ms)
- [ ] `pub fn last_token_logits(&self) -> &[f32]` para o Hipocampo

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_llama_cpp4_logit_engine_returns_logits` valida 128 entradas normalizadas
- Logits retornados sao reprodutíveis (mesma seed → mesmos valores)

## TASK-02 — Engine: MistralRsSidecarEngine (sidecar, FlashAttention 2, SIGKILL)

**Arquivo:** `src-tauri/src/core/mistral_sidecar.rs` (NOVO)

**Escopo:** Stub de sidecar efêmero que spawnaria `mistralrs-server` real mas cai em mock se binário ausente.

- [ ] Struct `pub struct MistralRsSidecarEngine`
- [ ] `impl EphemeralInferEngine for MistralRsSidecarEngine`
- [ ] Stub: spawn via `tokio::process::Command` do binário `mistral_sidecar.exe`
- [ ] Se binário ausente → fallback em `MockEphemeralInferEngine`
- [ ] RAII Drop com SIGKILL atômico (`start_kill` + `wait`)

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_mistral_sidecar_engine_falls_back_when_binary_missing` confirma fallback mock

## TASK-03 — Engine: BitnetEngine (Ternary 1.58-bit, Job Object, iceoryx2 IPC)

**Arquivo:** `src-tauri/src/core/bitnet_engine.rs` (NOVO)

**Escopo:** Wrap da `BitNetDaemon` existente sob o trait `EphemeralInferEngine`. Mantém o enjaulamento Job Object do Windows já herdado.

- [ ] Struct `pub struct BitnetEngine { inner: BitNetDaemon }`
- [ ] `impl EphemeralInferEngine for BitnetEngine`
- [ ] Stub: `BitNetDaemon::mock_for_testing` quando daemon ausente
- [ ] Documenta integração futura com iceoryx2 IPC (comment `// FUTURE: iceoryx2 IPC bridge`)

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_bitnet_engine_fails_soft_on_non_existent_model` valida `ModelNotFound`

## TASK-04 — Engine: PulpLeleEngine (AOT CPU math < 22µs)

**Arquivo:** `src-tauri/src/core/pulp_lele.rs` (NOVO)

**Escopo:** Stub AOT CPU-only para matmul pequeno e embedding lookup. Latência alvo: p99 < 22µs.

- [ ] Struct `pub struct PulpLeleEngine`
- [ ] `impl EphemeralInferEngine for PulpLeleEngine`
- [ ] Stub: `instant::Instant::now()` antes/depois da hot loop
- [ ] Documenta hot path esperado (intrinsics AVX2/NEON, sem alocação dinâmica)

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_pulp_lele_engine_completes_under_22us` valida latência sintética

## TASK-05 — Engine: BurnAgnosticEngine (Burn/CubeCL, agnostic megakernels)

**Arquivo:** `src-tauri/src/core/burn_agnostic.rs` (NOVO)

**Escopo:** Stub agnóstico de hardware. Sem dependência de CUDA. Documenta integração futura com `burn` + `cubecl` (transmutável para Metal/Vulkan/NPU).

- [ ] Struct `pub struct BurnAgnosticEngine`
- [ ] `impl EphemeralInferEngine for BurnAgnosticEngine`
- [ ] Stub: retorna `InferenceError::ExecutionError("PENDING_ENGINE: Burn ainda nao integrado")` para o cascade

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_burn_agnostic_engine_pending_error` confirma `PENDING_ENGINE`

## TASK-06 — Engine: OrtScorerEngine (ONNX Runtime CPU, small scorers)

**Arquivo:** `src-tauri/src/core/ort_scorer.rs` (NOVO)

**Escopo:** Stub para scorers pequenos (GLiClass, BGE-reranker). Sem GPU; apenas CPU EP.

- [ ] Struct `pub struct OrtScorerEngine`
- [ ] `impl EphemeralInferEngine for OrtScorerEngine`
- [ ] Stub: mock score determinístico baseado em `len()` do user_query
- [ ] Documenta integração futura com `ort` crate (ONNX Runtime Rust bindings)

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_ort_scorer_engine_mock_score` valida score reproduzível

## TASK-07 — `core/mod.rs` exposicao dos 6 novos módulos

**Arquivo:** `src-tauri/src/core/mod.rs` (EDIT)

**Escopo:** Adicionar 6 `pub mod` lines + 6 `pub use` re-exports.

- [ ] `pub mod llama_cpp4_logit;`
- [ ] `pub mod mistral_sidecar;`
- [ ] `pub mod bitnet_engine;`
- [ ] `pub mod pulp_lele;`
- [ ] `pub mod burn_agnostic;`
- [ ] `pub mod ort_scorer;`

**DoD:**
- `cargo check` Exit Code 0
- Nenhum warning de "unused" (engines consumidos por outros módulos via cascade)

## TASK-08 — `engine_trait.rs` exposicao do Cascade V4 com 8 engines

**Arquivo:** `src-tauri/src/core/engine_trait.rs` (EDIT)

**Escopo:** Adicionar 6 novos `EngineProbe` structs para que `EngineCascade` possa rotear entre os 8 motores.

- [ ] `pub struct LlamaCpp4LogitProbe`
- [ ] `pub struct MistralRsSidecarProbe`
- [ ] `pub struct BitnetProbe`
- [ ] `pub struct PulpLeleProbe`
- [ ] `pub struct BurnAgnosticProbe`
- [ ] `pub struct OrtScorerProbe`
- [ ] `EngineCascade::new()` registra todos os 8 probes (em ordem de prioridade)

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_engine_cascade_has_8_probes` confirma contagem

## TASK-09 — `bin/souls_mcp_server.rs` registro de `headroom_retrieve`

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)

**Escopo:** Adicionar entrada em `tools/list` para `headroom_retrieve` + dispatcher em `handle_tool_call`.

- [ ] Schema: `{ "name": "headroom_retrieve", "inputSchema": { "properties": { "hash": { "type": "string" } } } }`
- [ ] Dispatcher: lê `SoulsCcrStore::from_env().intercept_loopback(json_call)`
- [ ] Erro se `SoulsCcrStore::from_env()` falhar (com mensagem clara)

**DoD:**
- `cargo check` Exit Code 0
- Teste: `test_tools_list_includes_headroom_retrieve` confirma presença

## TASK-10 — Documentacao da proibicao LLMLingua-2 em `headroom_engine.rs`

**Arquivo:** `src-tauri/src/core/headroom_engine.rs` (EDIT — apenas comment block + 1 teste)

**Escopo:** Documentar a regra R4 da Linha Vermelha como comentario canonico + teste de rejeição.

- [ ] Comment block antes de `CodeCompressor::compress_ast_zero_copy` explicando que LLMLingua-2 é PROIBIDO
- [ ] Teste: `test_llmlingua2_forbidden_on_ast_block` valida que `lean_vacuum::compress_to_lean` em código Rust preserva assinaturas (o que LLMLingua-2 NÃO faria)

**DoD:**
- `cargo check` Exit Code 0
- Teste passa sem modificar lógica existente

## TASK-11 — Validation: cargo check + cargo test

**Escopo:** Provar que o silício assimilou a materialização V4.

- [ ] `cd src-tauri && cargo check --all-targets` → Exit Code 0
- [ ] `cd src-tauri && cargo test --no-run` → Exit Code 0
- [ ] Se falhar por lifetime/ownership: invocar `souls-ralph-loop` (3-tentativas ceiling, Fail-Closed)
- [ ] Se falhar por feature gating: ajustar `#[cfg(feature = "...")]` para stubs sem feature

**DoD:**
- `cargo check` retorna `Exit Code 0` com zero warnings
- `cargo test` retorna `Exit Code 0`

## TASK-12 — Blast Radius Report + HITL

**Escopo:** Compilar diff stats e enviar para aprovação humana.

- [ ] `git diff --stat` capturado
- [ ] Mensagem HITL gerada com: branch, número de arquivos novos, lista de paths
- [ ] NÃO fazer merge
- [ ] Aguardar aprovação do Arquiteto para rebase semântico
