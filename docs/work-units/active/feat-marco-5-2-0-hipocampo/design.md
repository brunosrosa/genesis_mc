# MARCO 5.2.0 — Design Arquitetural: Hipocampo Real (LlamaCpp4LogitEngine) na CPU

## 1. Visão Geral
Este documento especifica o design da ignição do **Avaliador Epistêmico Real (Hipocampo)** executando o motor `LlamaCpp4LogitEngine` em CPU (AVX2) sem offload para a dGPU (RTX 2060m mantida 100% livre), conforme as diretrizes do SODA Canon V6, ADR-027 (confinamento VRAM), ADR-014 (Doutrina de Fricção Produtiva) e ADR-010 (Anti-Vibe Coding).

## 2. Topologia Orchestrator-Worker & Concorrência Isolada

```mermaid
graph TD
    A["Tokio Event Loop (Gateway Proxy / MCP)"] -->|MPSC Bounded Channel| B["Thread OS Dedicada 'souls-l7-shield'"]
    B -->|Prefill Forward Pass (0 GPU Layers)| C["llama-cpp-2 / CPU AVX2"]
    C -->|llama_get_logits_ith| D["Vetor de Logits f32 (Último Token N-1)"]
    D -->|Top-50 Softmax & Shannon Entropy| E["EpistemicScores { ambiguidade, risco_relacional, conflito_memoria }"]
    E -->|Oneshot Reply| A
    A -->|Se ambiguidade > 0.75| F["Disjuntor Socrático: Tauri Event 'socratic_interrupt' + JSON-RPC -32001 (HitlDenied)"]
    A -->|Se ambiguidade <= 0.75| G["Bypass: Repasse Normal ao Upstream"]
```

## 3. Agnosticismo de Hardware & Regras de CPU/AVX2
- **Offload dGPU:** 0 camadas (`n_gpu_layers = 0`). O modelo de pesos (Gemma 4 E2B Q8_0 / Phi-4-mini) é mapeado via `mmap2` em RAM contígua do host.
- **Isolamento de Scheduler:** A thread OS nativa `'souls-l7-shield'` roda isolada da pool Tokio para prevenir stalls no event loop ou congelamentos na UI Svelte 5.
- **Zero-Token Decoding:** Aborta a execução imediatamente após o prefill do prompt. Zero completion tokens gerados (sem loop autorregressivo de amostragem).

## 4. Matemática da Entropia de Shannon Numericamente Estável
1. **Top-50 Softmax:**
   $$p_i = \frac{\exp(x_i - \max(x))}{\sum_{j=1}^{50} \exp(x_j - \max(x))}$$
2. **Entropia de Shannon Normalizada:**
   $$H = -\sum_{i=1}^{50} p_i \log_2(p_i)$$
   $$\text{ambiguidade} = \frac{H}{\log_2(50.0)} \in [0.0, 1.0]$$
3. **Verbalizadores Binários:** Mapeamento de IDs físicos no vocabulário para os tokens de polaridade `'0'`/`'1'` e `'true'`/`'false'` para extração direta das razões probabilísticas de `risco_relacional` e `conflito_memoria`.

## 5. Roteamento FinOps & Circuit Breaker
- Limiar de ambiguidade: `ambiguidade > 0.75` dispara o disjuntor de incerteza.
- Disparo: Emite o Tauri Event `socratic_interrupt` e retorna JSON-RPC `-32001 (HitlDenied)`.
