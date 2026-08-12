---
spec: marco-ii-hipocampo-cura-fnv1a
phase: 3-tasks
design: docs/work-units/active/feat-marco-ii-hipocampo-cura-fnv1a/design.md
branch: feat/marco-ii-hipocampo-cura-fnv1a
---

# Tasks — CURA DO FANTASMA FNV-1a (Marco II)

Cada task tem DoD (Definition of Done) executável. Lei do Scaffold: teste de falha antes da lógica real.

## TASK-01 — Substituir FNV-1a por `LogitSource` enum (real | prompt_derived | test_fixture)

**Arquivo:** `src-tauri/src/core/llama_logit_probing.rs` (EDIT)

**Escopo:** Eliminar `mock_logits: Vec<f32>` inicializado com FNV-1a do struct de produção. Substituir por um enum `LogitSource` que materializa as 3 fontes de logits (Real, PromptDerived, TestFixture).

- [ ] Remover o campo `mock_logits: Vec<f32>` do struct `LlamaLogitProber`
- [ ] Adicionar enum `pub enum LogitSource { RealLlama(Arc<Mutex<LlamaCpp2Context>>), PromptDerived, TestFixture(Vec<f32>) }`
- [ ] Mover a função `seed_logit()` e a constante `0x5A5A_C0DE` para um submódulo `#[cfg(test)] mod test_fixtures { ... }`
- [ ] Implementar `PromptDerived::compute(prompt: &str) -> Vec<f32>`: vetor 128-dim derivado de (i) `byte_entropy(prompt)`, (ii) `char_class_distribution(prompt)`, (iii) `estimated_token_count(prompt)`. **Zero hash, zero FNV-1a**.
- [ ] Implementar `LogitSource::extract_logits(&self, prompt: &str) -> Vec<f32>`: dispatch para o modo correto.
- [ ] Atualizar `extract_last_token_raw_logits()` e `last_token_logits()` para usar `LogitSource::extract_logits`.

**DoD:**
- `cargo check -p souls_mc_lib --features llama_backend` Exit Code 0
- `cargo check -p souls_mc_lib` (sem features) Exit Code 0
- Zero warnings de `unused`
- Grep `FNV|0x811C|seed_logit` no arquivo modificado retorna APENAS ocorrências dentro de `#[cfg(test)] mod test_fixtures`

## TASK-02 — Adicionar `test_logit_probing_cpu_avx2` (TDD Red→Green)

**Arquivo:** `src-tauri/src/core/llama_logit_probing.rs` (EDIT, no `#[cfg(test)] mod tests`)

**Escopo:** Validar que o caminho PROMPT_DERIVED produz logits matematicamente válidos (soma do Softmax = 1.0, entropia ∈ [0, log2(128)]).

- [ ] Função `test_logit_probing_cpu_avx2`:
  1. Cria `LlamaLogitProber::with_prompt_derived()`
  2. Chama `extract_last_token_raw_logits("edite o arquivo config de hoje")`
  3. Asserções:
     - `result.len() == 128`
     - Todos os valores em `[-50.0, 50.0]` (clamp defensivo do Softmax)
     - `softmax(result).iter().sum::<f32>() ≈ 1.0` (tolerância 1e-5)
     - `shannon_entropy(softmax(result)) >= 0.0 && <= log2(128)`
     - Duração `start.elapsed() < 150ms`

**DoD:**
- `cargo test -p souls_mc_lib test_logit_probing_cpu_avx2` Exit Code 0
- Teste reproduzível (mesmo prompt → mesmos logits)

## TASK-03 — Adicionar `test_thinking_hitl_extension_to_7` (TDD Red→Green)

**Arquivo:** `src-tauri/src/cognition/state_thinking/thinking/engine.rs` (EDIT, no `#[cfg(test)] mod tests`)

**Escopo:** Validar que `hitlAuthorized=true` no payload estica o teto de 5 → 7 pensamentos sem disparar `OverthinkingThresholdBreached` no 6º e 7º pensamentos.

- [ ] Função `test_thinking_hitl_extension_to_7`:
  1. Cria `ThinkingEngine::new()` (teto default = 5)
  2. Faz push de 5 pensamentos regulares (1..=5), todos OK
  3. Constrói um 6º pensamento com `hitl_authorized: Some(true)`
  4. Asserções:
     - `engine.push_thought(t6).is_ok()` (não dispara disjuntor)
     - `engine.current_limit() == 7` (HITL esticou o teto)
     - `engine.is_hitl_authorized() == true`
  5. Faz push do 7º pensamento, também OK
  6. Faz push do 8º pensamento, deve disparar `OverthinkingThresholdBreached { actual: 8, max: 7 }`

**DoD:**
- `cargo test -p souls_mc_lib test_thinking_hitl_extension_to_7` Exit Code 0
- Os 3 testes TDD pré-existentes (`test_thinking_disjuntor_loop`, `test_revision_validation`, `test_dynamic_paradigm_selection`) permanecem verdes

## TASK-04 — Validação Ralph Loop (cargo check + cargo test)

**Comandos:**
- [ ] `cd src-tauri && cargo check --workspace --all-targets` → Exit Code 0
- [ ] `cd src-tauri && cargo test --workspace --lib test_database_migration_v5` → 0 (legacy alias se existir)
- [ ] `cd src-tauri && cargo test --workspace --lib test_logit_probing_cpu_avx2` → 0
- [ ] `cd src-tauri && cargo test --workspace --lib test_thinking_disjuntor_loop` → 0
- [ ] `cd src-tauri && cargo test --workspace --lib test_thinking_hitl_extension_to_7` → 0
- [ ] Se falhar (lifetime/ownership): invocar `souls-ralph-loop` (3 tentativas ceiling, Fail-Closed)

**DoD:**
- Todos os 5 comandos acima retornam Exit Code 0
- Zero regressão em testes pré-existentes

## TASK-05 — Blast Radius Report + Notificar Agent Inbox

**Escopo:** Compilar diff stats e enviar para aprovação humana.

- [ ] `git diff --stat` capturado
- [ ] Mensagem HITL gerada com: branch, arquivos novos/modificados, contagem de linhas adicionadas/removidas
- [ ] Confirmar zero match de `FNV|seed_logit` em runtime (`rg "FNV|seed_logit|0x811C" src-tauri/src --type rust -g '!*test*'`)
- [ ] NÃO fazer merge
- [ ] Aguardar aprovação do Arquiteto para Rebase Semântico
