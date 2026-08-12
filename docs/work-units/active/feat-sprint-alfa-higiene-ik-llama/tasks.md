---
spec: sprint-alfa-higiene-cura-cuda-ik-llama
phase: 3-tasks
design: docs/work-units/active/feat-sprint-alfa-higiene-ik-llama/design.md
branch: feat/sprint-alfa-higiene-ik-llama
---

# Tasks — Sprint Alfa (Higiene Clippy, Cura CUDA, Transplante ik_llama.cpp)

Cada task tem DoD executável. Tarefas `[SCAFFOLD]` exigem teste de regressão vermelho antes da correção. Sequência otimizada: Battle 1 → Battle 2 → Battle 3 (deps de invariante).

---

## BATTLE 1 — Extermínio dos 4 Warnings Históricos (ADR-025)

### TASK-01.1 — `pareto_bandit.rs:336` (manual_arithmetic_check)

**Arquivo:** `src-tauri/src/finops/pareto_bandit.rs` (EDIT — linha 336)
**Tipo:** Clippy lint `manual_arithmetic_check`
**Mecânica:** Substituir aritmética manual de comparação (e.g., `(a > b) as i32 - (a < b) as i32`) por `.clamp()` ou `.signum()` nativo.

- [ ] `[SCAFFOLD]` Adicionar teste de regressão que valida o comportamento atual preservado
- [ ] Aplicar correção idiomática Rust (nativo `.clamp()` / `.signum()` / `.partial_cmp` → ordering)
- [ ] `cargo clippy -- -D warnings` → sem warning em `pareto_bandit.rs`

**DoD:**
- `cargo clippy --features "tauri-app,gateway_ccr,lora_adapter" -- -D warnings` Exit Code 0
- Teste de regressão passa
- Comportamento externo idêntico (pure refactor)

### TASK-01.2 — `peak_ewma.rs:145` (manual_arithmetic_check — não `redundant_closure`)

**Arquivo:** `src-tauri/src/core/peak_ewma.rs` (EDIT — linha 145)
**Tipo:** Clippy lint `manual_arithmetic_check`
**NOTA IMPORTANTE:** O user atribuiu `redundant_closure` mas a linha real é `let start = if write_idx >= N { write_idx - N } else { 0 };` — `manual_arithmetic_check`. Aplicar a correção certa.

- [ ] `[SCAFFOLD]` Adicionar teste `test_snapshot_handles_wrap_around_zero` que valida `start` correto em wrap
- [ ] Substituir por `let start = write_idx.saturating_sub(N);` (já temos `n_filled = write_idx.min(N)` garantindo `write_idx <= 2N`)
- [ ] `cargo clippy -- -D warnings` → sem warning em `peak_ewma.rs`

**DoD:**
- Teste de regressão passa (sample 60.0, 70.0, 80.0, 90.0 em ring N=4)
- `cargo clippy -D warnings` Exit Code 0

### TASK-01.3 — `pii_redactor.rs:157` (needless_lifetimes)

**Arquivo:** `src-tauri/src/core/pii_redactor.rs` (EDIT — linha 157)
**Tipo:** Clippy lint `needless_lifetimes`
**Mecânica:** `pub fn redact<'a>(&self, body: &'a [u8]) -> Vec<u8>` → `pub fn redact(&self, body: &[u8]) -> Vec<u8>` (o lifetime `'a` é desnecessário pois `Vec<u8>` não retorna nada que dependa dele)

- [ ] `[SCAFFOLD]` Adicionar teste `test_redact_bearer_token_returns_vec` para validar comportamento preservado
- [ ] Remover `<'a>` da assinatura
- [ ] `cargo clippy -- -D warnings` → sem warning em `pii_redactor.rs`

**DoD:**
- Teste de regressão passa
- Comportamento externo idêntico

### TASK-01.4 — `telemetry_dispatcher.rs:130` (too_many_arguments — 7 params)

**Arquivo:** `src-tauri/src/core/telemetry_dispatcher.rs` (EDIT — método `dispatch_latency`)
**Tipo:** Clippy lint `too_many_arguments`
**Mecânica:** Agrupar 7 parâmetros em struct dedicada `LatencyPayload`. Manter API pública via overload/método alternativo que aceita a struct.

- [ ] `[SCAFFOLD]` Adicionar teste `test_dispatch_latency_via_payload_struct` que valida o novo caminho
- [ ] Criar `pub struct LatencyPayload { tool, ttft_ms, peak_ewma_ms, tokens_in, tokens_out, cost_usd, session_id }`
- [ ] Refatorar `dispatch_latency` para delegar a `dispatch_latency_payload(&self, payload: LatencyPayload)`
- [ ] `cargo clippy -- -D warnings` → sem warning em `telemetry_dispatcher.rs`

**DoD:**
- Teste de regressão passa
- API existente (`dispatch_latency` com 7 args) ainda funciona (via construtor de LatencyPayload) ou substituída de forma limpa
- `cargo clippy -D warnings` Exit Code 0

### TASK-01.5 — Validation Battle 1

- [ ] `cargo clippy --features "tauri-app,gateway_ccr,lora_adapter" -- -D warnings` → Exit Code 0
- [ ] `cargo test --features "tauri-app,gateway_ccr,lora_adapter" --lib` → Exit Code 0
- [ ] Se falhar: invocar `souls-ralph-loop` (3-tentativas ceiling, Fail-Closed)

---

## BATTLE 2 — Cura do Bug CUDA "fatbinary fatal" (ADR-039 / sccache)

### TASK-02.1 — Reforço do Patch CUDA em `boot.ps1`

**Arquivo:** `boot.ps1` (EDIT — após linha 53, dentro do bloco try antes de `$ErrorActionPreference = "Stop"`)
**Tipo:** Patch idempotente PowerShell
**Mecânica:** Adicionar bloco que injeta:
- `$env:GGML_CCACHE = "OFF"` (variável de ambiente para cmake)
- `$env:CMAKE_CUDA_COMPILER_LAUNCHER = ""` (string vazia desabilita wrapper)
- `$env:CUDA_CACHE_DISABLE = "1"` (defesa em profundidade NVCC)

- [ ] `[SCAFFOLD]` Verificar que sem o patch, `cargo build --features llama_backend` falha com "fatbinary fatal"
- [ ] Adicionar bloco injetor de env vars
- [ ] Verificar que sccache continua ativo para rustc puro (log de `$env:RUSTC_WRAPPER`)
- [ ] `cargo build --features llama_backend --bin souls_vanguard_worker` → Exit Code 0 (ou erro não-relacionado a fatbinary)

**DoD:**
- Build com `--features llama_backend` não emite "fatbinary fatal: Could not open input file"
- `$env:RUSTC_WRAPPER` permanece "sccache" (não tocado)
- Patch é idempotente (re-executar boot.ps1 não duplica)

### TASK-02.2 — Validation Battle 2

- [ ] `cargo build --features "tauri-app,llama_backend" --bin souls_vanguard_worker --message-format short` → Exit Code 0 (ou erro não-CUDA)
- [ ] Se persistir: investigar `sccache nvcc` injection (pode ser via `.cargo/config.toml`)

---

## BATTLE 3 — Transplante Unificado de ik_llama.cpp

### TASK-03.1 — `Cargo.toml` swap `llama-cpp-2` → `ik-llama-cpp-2`/`ik-llama-cpp-sys`

**Arquivo:** `src-tauri/Cargo.toml` (EDIT — linhas 132-135, 211-213)
**Tipo:** Dependency swap
**Mecânica:**
- Adicionar `ik-llama-cpp-2 = { version = "0.1.7", features = ["cuda", "openmp"] }` no `[workspace.dependencies]`
- Adicionar `ik-llama-cpp-sys = { version = "0.1.2", features = ["cuda"] }` no `[workspace.dependencies]`
- **REMOVER** `llama-cpp-2 = { version = "0.1", features = ["cuda"] }` do workspace
- No crate `souls_mc`, substituir `llama-cpp-2 = { workspace = true, optional = true }` por `ik-llama-cpp-2 = { workspace = true, optional = true }` + `ik-llama-cpp-sys = { workspace = true, optional = true }`
- Ajustar feature `llama_backend = ["dep:ik-llama-cpp-2", "ik-llama-cpp-2/cuda", ...]`

- [ ] `[SCAFFOLD]` `cargo check` falha com erro de `llama_cpp_2` not found (esperado)
- [ ] Aplicar swap conforme mecânica
- [ ] `cargo check` → Exit Code 0 com nova dep

**DoD:**
- `cargo check --features llama_backend` → Exit Code 0
- `grep -r "llama_cpp_2" src/` retorna apenas `llama_lora_adapter.rs` (módulo de compat, não-engine) e referências em `l7_shield.rs` (ajustar)
- Zero uso do upstream `llama_cpp_2` no código de engine

### TASK-03.2 — `llama_engine.rs` TurboQuant (K=FP16, V=Q4_K)

**Arquivo:** `src-tauri/src/core/llama_engine.rs` (EDIT — imports + `LlamaContextParams` construction)
**Tipo:** Engine refactor
**Mecânica:**
- Trocar `use llama_cpp_2::*` por `use ik_llama_cpp_2::*` em todo o arquivo
- No método que constrói `LlamaContextParams`, adicionar `.with_type_k(KvCacheType::F16).with_type_v(KvCacheType::Q4_K)`
- Ajustar `type_k/type_v` se API do ik usar nomes diferentes (verificar docs.rs)
- Manter compat com `n_ctx(32_768)` para contexto longo

- [ ] `[SCAFFOLD]` Teste de invariante: `n_ctx_train` do modelo ≤ `n_ctx` configurado
- [ ] Aplicar troca de imports
- [ ] Adicionar TurboQuant (K=FP16, V=Q4_K)
- [ ] `cargo check --features llama_backend` → Exit Code 0
- [ ] Log informativo no startup: "TurboQuant ativado: K=FP16, V=Q4_K"

**DoD:**
- Engine compila com ik bindings
- `KvCacheType::F16` e `KvCacheType::Q4_K` materializados
- Estimativa: ctx 32k → ~800MB VRAM (validado via `n_gpu_layers` log)

### TASK-03.3 — `llama_logit_probing.rs` FFI real `llama_get_logits_ith`

**Arquivo:** `src-tauri/src/core/llama_logit_probing.rs` (EDIT — braço `RealLlama` do `LogitSource`)
**Tipo:** Engine refactor + FFI real
**Mecânica:**
- Substituir stub fail-soft do `RealLlama` por chamada FFI real
- Em `extract_logits`, usar `ik_llama_cpp_sys_2::llama_get_logits_ith(ctx.as_ptr(), last_idx)` para extrair logits do último token
- Implementar `stable_softmax` (log-sum-exp trick) para processar os verbalizadores do Gemma-4 E2B (IQ3_M)
- Conectar ao `l7_shield.rs::EpistemicShieldChannel` via `spawn<LlamaLogitProber>(...)`
- Thread dedicada nomeada `souls-l7-shield` (verificar se já existe; senão criar)

- [ ] `[SCAFFOLD]` Teste `test_logit_probing_under_150ms` que falha com o stub atual (deve passar após FFI real)
- [ ] Remover stub `RealLlama` fail-soft
- [ ] Implementar FFI real via `llama_get_logits_ith`
- [ ] Implementar `stable_softmax` (log-sum-exp)
- [ ] Conectar ao L7 Shield via thread dedicada
- [ ] `cargo test --features llama_backend --lib llama_logit_probing` → Exit Code 0

**DoD:**
- Teste de latência < 150ms passa em CI
- `llama_get_logits_ith` é chamado (verificável via log/tracing)
- Softmax estável (sem NaN em logits extremos)
- Prober conectado ao `l7_shield.rs`

### TASK-03.4 — Validation Battle 3

- [ ] `cargo check --features "tauri-app,gateway_ccr,lora_adapter,llama_backend"` → Exit Code 0
- [ ] `cargo test --features llama_backend --lib` → Exit Code 0
- [ ] `cargo build --features llama_backend --bin souls_vanguard_worker` → Exit Code 0 (verifica CUDA + ik compilam juntos)

---

## VALIDAÇÃO FINAL (FASE 5)

### TASK-99.1 — Suite Master + Clippy Estrito

- [ ] `cd src-tauri && cargo test --workspace` → Exit Code 0
- [ ] `cd src-tauri && cargo clippy --features "tauri-app,gateway_ccr,lora_adapter" -- -D warnings` → Exit Code 0 com **ZERO warnings**
- [ ] Se falhar: invocar `souls-ralph-loop` (3-tentativas ceiling, Fail-Closed)

### TASK-99.2 — Blast Radius Report + HITL

- [ ] `git diff --stat` capturado
- [ ] Mensagem HITL gerada com: branch, número de arquivos editados, paths críticos
- [ ] **NÃO** fazer merge
- [ ] Aguardar aprovação do Arquiteto para rebase semântico em direção à main

---

## CRONOGRAMA DE EXECUÇÃO (Ordem de Mutação Atômica)

| Ordem | Task | Custo Esperado | Risco |
|-------|------|----------------|-------|
| 1 | TASK-01.1 (pareto) | 5 min | Baixo (pure refactor) |
| 2 | TASK-01.2 (peak_ewma) | 5 min | Baixo |
| 3 | TASK-01.3 (pii) | 5 min | Baixo |
| 4 | TASK-01.4 (telemetry) | 15 min | Médio (refactor API) |
| 5 | TASK-01.5 (validate) | 30s | — |
| 6 | TASK-02.1 (boot.ps1) | 10 min | Baixo (PowerShell) |
| 7 | TASK-02.2 (validate build CUDA) | 2-5 min (depende de CUDA) | Médio (toolchain) |
| 8 | TASK-03.1 (Cargo.toml) | 5 min | Baixo |
| 9 | TASK-03.2 (llama_engine) | 15 min | Médio (ik API differences) |
| 10 | TASK-03.3 (logit_probing) | 30 min | Alto (FFI + timing) |
| 11 | TASK-03.4 (validate) | 2-5 min | — |
| 12 | TASK-99.1 (master) | 5-10 min | — |
| 13 | TASK-99.2 (HITL) | 2 min | — |

**Total estimado**: ~1h a 2h de execução efetiva (builds não inclusos).
