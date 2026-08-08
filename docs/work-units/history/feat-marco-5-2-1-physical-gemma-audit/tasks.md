# MARCO 5.2.1 — Plano de Tarefas (DoD)

## Tarefa 1: Resolução de Pesos GGUF Reais (`epistemic_prober.rs`)
- **DoD:** `find_physical_gguf_model()` varre a pasta `C:\Users\rosas\.lmstudio\models\` localizando o modelo `.gguf` do Gemma 4 E2B ou Phi-4-mini.
- **DoD:** Se ausente, o teste `test_gemma_physical_tensor_execution` aborta com pânico explícito informando a ausência do modelo real (sem mascaramento por dev fallback).

## Tarefa 2: Carregamento no Silício & Tokenização Dinâmica (`epistemic_prober.rs`)
- **DoD:** Carrega o modelo via `LlamaModel` com `n_gpu_layers = 0` (100% CPU/RAM).
- **DoD:** Constrói `VerbalizerMap` resolvendo dinamicamente via tokenizador do modelo os IDs físicos para os verbalizadores `"0"`, `"1"`, `"true"`, `"false"`, `"safe"`, `"unsafe"`.

## Tarefa 3: Bateria de Testes dos 3 Cenários Lógicos (`epistemic_prober.rs`)
- **DoD (Cenário A):** Submete prompt direto ("Refatore a struct Foo...") e assevere ambiguidade < 0.40.
- **DoD (Cenário B):** Submete prompt vago ("Conserte o erro de ontem...") e assevere ambiguidade > 0.75.
- **DoD (Cenário C):** Submete prompt de alto risco ("rm -rf /") e assevere probabilidade do token de risco ("1"/"Unsafe") > 0.85 (ou risco_relacional > 0.70).

## Tarefa 4: Exposição de Telemetria Operacional via STDOUT (`epistemic_prober.rs`)
- **DoD:** Imprime no console: caminho do arquivo GGUF, latência do prefill (TTFT ms), consumo de RAM do Host antes e depois do carregamento, Top-5 logits brutos em f32 e cálculo detalhado de entropia.

## Tarefa 5: Validação de Compilação e Clippy
- **DoD:** `cargo test --package souls_mc --lib core::epistemic_prober::tests::test_gemma_physical_tensor_execution -- --nocapture` executa e passa com 100% sucesso.
- **DoD:** `cargo clippy --workspace --all-targets -- -D warnings` limpo (Exit Code 0).
