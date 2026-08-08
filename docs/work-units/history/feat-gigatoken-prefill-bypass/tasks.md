# TASKS: MARCO 5.4.0 — Gigatoken Auto-Curativo, Prefill Bypass & GPU Ignition

## Definition of Done (DoD)
- [ ] **Etapa 1**: `src-tauri/src/core/gigatoken_encoder.rs` criado com `GigaTokenEncoder` auto-curativo (`tokenizer_recovered.json` a partir do GGUF) e `tokenize_to_bin`.
- [ ] **Etapa 2**: `InferenceInput` enum em `inference_adapter.rs` e `llama_engine.rs` integrado com bypass compulsório de `llama_tokenize` e conversão FFI segura `u32` -> `i32`.
- [ ] **Etapa 3**: Inicialização de GPU com `n_gpu_layers = 99`, KV Cache FP16 (K) / Q4_K (V) e trava de VRAM em 5.5 GB (`test_vram_budget_math`).
- [ ] **Etapa 4**: Sanitização completa do `LlamaBatch` (`batch.clear()` e zero vazamento de logits em cada transação).
- [ ] **Etapa 5**: Suíte TDD (`test_gigatoken_prefill_bypass`, `test_gigatoken_vocab_self_healing`, `test_gigatoken_throughput_benchmark`, `test_vram_budget_math`) 100% GREEN.

## Tarefas Atômicas
1. **InferenceInput Enum**: Adicionar `InferenceInput` e campo opcional `input` em `inference_adapter.rs`.
2. **Gigatoken Encoder & Autocura**: Criar `gigatoken_encoder.rs` com extração de vocabulário GGUF fallback e registrar em `core/mod.rs`.
3. **Refatoração LlamaEngine**: Adicionar interceptação `PreTokenized` com conversão `u32` -> `i32` e sanitização `batch.clear()`.
4. **Cálculos de VRAM**: Validar orçamentação de KV Cache Q4_K + FP16 em `llama_engine.rs`.
5. **Suíte TDD & Clippy**: Executar `cargo test --lib core::gliclass_engine` e `cargo clippy` com zero warnings.
