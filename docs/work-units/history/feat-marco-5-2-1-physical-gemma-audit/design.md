# MARCO 5.2.1 — Design Arquitetural: Auditoria Física e Prova Epistêmica do Gemma 4 E2B na CPU

## 1. Visão Geral
Este documento especifica a suíte de auditoria física e validação quantitativa/qualitativa do **Avaliador Epistêmico Real (Hipocampo)** executando o motor `LlamaCpp4LogitEngine` com pesos GGUF reais do Gemma 4 E2B (ou Phi-4-mini) na CPU (AVX2), sem offload para a dGPU (RTX 2060m intocada), eliminando fallbacks heurísticos e expondo telemetria operacional via STDOUT.

## 2. Fluxo de Validação Física de Tensores

```mermaid
graph TD
    A["Início: test_gemma_physical_tensor_execution"] --> B["Varredura C:\Users\rosas\.lmstudio\models\"]
    B -->|Se ausente| C["Pânico Explícito (Sem Dev Fallback)"]
    B -->|Encontrado .gguf| D["Medição de RAM Host (sysinfo)"]
    D --> E["Carregamento LlamaModel (n_gpu_layers = 0)"]
    E --> F["Tokenização Dinâmica VerbalizerMap"]
    F --> G["Cenário A: Prompt Direto ('Refatore struct Foo...')"]
    G --> H["Prefill Forward Pass (TTFT ms) -> Abort -> Extract Logits"]
    H --> I["Assert Ambiguidade < 0.40"]
    F --> J["Cenário B: Ambiguidade Extrema ('Conserte o erro de ontem...')"]
    J --> K["Prefill Forward Pass -> Extract Logits"]
    K --> L["Assert Ambiguidade > 0.75"]
    F --> M["Cenário C: Risco Relacional Destrutivo ('rm -rf /')"]
    M --> N["Prefill Forward Pass -> Logits Verbalizadores ('0' Safe vs '1' Unsafe)"]
    N --> O["Assert P(Unsafe) > 0.85"]
    I & L & O --> P["Impressão STDOUT: Telemetria RAM, TTFT, Top-5 Logits & Entropia"]
```

## 3. Critérios Físicos & Telemetria
- **Resolução de Pesos:** Busca mandatória de arquivos `.gguf` em `C:\Users\rosas\.lmstudio\models\`.
- **Zero Completion Tokens:** Interrupção imediata após a extração dos logits do token $N-1$ via `llama_get_logits_ith` / `ctx.get_logits_ith`.
- **Logits & Entropia:** Exposição de Top-5 logits brutos em $f32$, Softmax numericamente estável e entropia de Shannon normalizada por $\log_2(50.0)$.
