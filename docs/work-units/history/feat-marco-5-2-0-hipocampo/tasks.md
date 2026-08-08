# MARCO 5.2.0 — Plano de Tarefas (DoD)

## Tarefa 1: ModelRegistry & Resolução Dinâmica de Modelos GGUF (CPU)
- **DoD:** `ModelRegistry` fornece método de resolução `resolve_epistemic_model_path()` consultando o banco SQLite L2 (`model_registry`) e/ou varredura local para localizações de Gemma 4 E2B / Phi-4-mini GGUF.
- **Fail-Soft:** Se nenhum arquivo GGUF for encontrado em ambiente de teste, o prober executa o fallback determinístico sem panicar.

## Tarefa 2: Motor LlamaCpp4LogitEngine & Zero-Token Logit Probing (`epistemic_prober.rs`)
- **DoD:** `LlamaCpp4LogitEngine` carrega o modelo em RAM com 0 camadas GPU.
- **DoD:** Realiza apenas o forward pass do prompt (`batch.set_logits(last_token_idx, true)` + `ctx.decode`). Aborta imediatamente sem decoding loop.
- **DoD:** Extrai os logits em `f32` via `ctx.get_logits_ith` / FFI `llama_get_logits_ith`.

## Tarefa 3: Softmax Estável & Entropia de Shannon Top-50 (`epistemic_prober.rs`)
- **DoD:** Implementa a Softmax numericamente estável sobre os Top-50 logits.
- **DoD:** Calcula a Entropia de Shannon normalizada dividindo por `log2(50.0)` para obter o score de `ambiguidade`.
- **DoD:** Mapeia IDs físicos dos verbalizadores `'0'`/`'1'` e `'true'`/`'false'` para calcular `risco_relacional` e `conflito_memoria`.

## Tarefa 4: Concorrência Isolada na Thread OS `souls-l7-shield` (`l7_shield.rs`)
- **DoD:** `EpistemicShieldChannel` spawna a thread nativa do SO batizada como `'souls-l7-shield'`.
- **DoD:** Comunicação Tokio <-> Worker via `tokio::sync::mpsc` (com capacidade restrita 16) e `oneshot` para retornos.

## Tarefa 5: Interceptor Gateway L7 & Evento Tauri `socratic_interrupt` (`souls_mcp_server.rs`)
- **DoD:** Se `ambiguidade > 0.75` (ou `risco_relacional > 0.70`), dispara o disjuntor de incerteza.
- **DoD:** Emite o Tauri Event `socratic_interrupt` e retorna erro JSON-RPC `-32001 (HitlDenied)`.

## Tarefa 6: Suíte de Testes TDD (`test_gemma_logit_probing_entropy`)
- **DoD:** Teste `test_gemma_logit_probing_entropy` assevera que prompts ambíguos ("execute o script de ontem") resultam em entropia > 0.75 e disjuntor ativo.
- **DoD:** Teste `test_clear_prompt_bypass` assevera que prompts diretos e imperativos resultam em score < 0.40 e bypass sem interrupção.
- **DoD:** `cargo check --workspace` e `cargo clippy` executam com sucesso total (Exit Code 0) sem nenhum warning.
