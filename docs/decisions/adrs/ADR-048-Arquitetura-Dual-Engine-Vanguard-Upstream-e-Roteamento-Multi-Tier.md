# ADR-048: Arquitetura Multi-Engine (Vanguard + Upstream + Mistral) e Orquestração Multi-Tier (A Alfândega de Alta Segurança)

## Status
**APROVADO / EM PRODUÇÃO** (Marco 4.11 — 2026-08-28)

## Contexto e Motivação
Historicamente, o ecossistema SOULS operava com uma flag genérica `llama_backend`, vinculada estaticamente ao motor experimental `ik_llama.cpp` (Vanguard). Embora o `ik_llama.cpp` ofereça aceleração de vanguarda para quantizações *i-matrix* e compressão assimétrica de KV Cache (TurboQuant `K:F16/V:Q4` e `< Q4`), ele não contém kernels recentes para novas arquiteturas do mercado de 2026 (tais como `Phi-4-mini`, `NVIDIA-Nemotron`, `LFM2.5`, `Hy-MT2` e variantes GGUF de SSM/Mamba).

A tentativa de forçar todas as arquiteturas em um único motor gerava falhas de carregamento de vocabulário/tensores ou degradação de throughput.

## Decisão de Arquitetura

### 1. A Alfândega de Alta Segurança: Metáfora Operacional dos Tiers
A governança de inferência do SOULS divide o silício (Intel Core i9 AVX2 + NVIDIA RTX 2060m 6GB) em três funcionários de uma alfândega:

```
[ Entrada do Prompt ]
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. TIER 0: Porteiro Rápido (GLiClass na CPU / ONNX - 12ms)  │
│ - Forward pass bidirecional não-gerativo O(1).              │
│ - Classificação instantânea de intenção e ambiguidade.       │
│ - Consumo: 0 MB de VRAM (RAM Host).                         │
└──────────────┬──────────────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. TIER 0.5: Investigador Cético (Gemma 4 E2B na CPU - 50ms)│
│ - O HIPOCAMPO / SENSOR DE DÚVIDA EPISTÊMICA.                │
│ - Logit Probing O(1): Faz o prefill, congela antes do       │
│   primeiro token gerado, captura logits e calcula Softmax.  │
│ - Mede hesitação, incerteza e conflito de memória.          │
│ - Consumo: 0 MB de VRAM (RAM Host).                         │
└──────────────┬──────────────────────────────────────────────┘
               │
               ▼ (Prompt Higienizado e Sem Ambiguidade)
┌─────────────────────────────────────────────────────────────┐
│ 3. TIER 1 / 1.5: Operário Especialista (dGPU - Full VRAM)   │
│ - Qwen 3.5 Coder 4B / Modelos 7B (IQ3/Q4_K de ~3.1-3.5 GB). │
│ - Ocupa ~3.3 GB de pesos, deixando 1.5 a 2.5 GB limpos na   │
│   dGPU exclusivamente para KV Cache TurboQuant (45+ tok/s). │
└─────────────────────────────────────────────────────────────┘
```

### 2. Fronteiras Rígidas dos Tiers de Silício

| Tier | Função no SOULS | Faixa de Parâmetros / Peso | Silício Alvo | Exemplos Canônicos |
| :--- | :--- | :---: | :---: | :--- |
| **Tier 0** | **Porteiro & Sanity Boot** | $< 500\text{M}$ ($< 600\text{ MB}$) | CPU (AVX2) | `GLiClass` (ONNX), `SmolLM-135M`, `SmolLM2-360M`, `NeuralAI-Mamba-K1` |
| **Tier 0.5** | **Sensor Epistêmico & Hipocampo** | $500\text{M} - 1.8\text{B}$ ($0.6 - 1.8\text{ GB}$) | CPU (Logit Probing) | `Gemma 4 E2B` (Titular Absoluto), `DeepSeek-R1-1.5B`, `LFM2.5-1.2B` |
| **Tier 1 / 1.5** | **Live Chat & Master Coder** | $2\text{B} - 8\text{B}$ ($2.0 - 4.2\text{ GB}$) | GPU Full VRAM | `Qwen 3.5 Coder 4B`, `Gemma 4 E2B` (modo chat), `Phi-4-mini`, `Nemotron-4B`, `Fara-7B`, `Falcon3-Mamba-7B`, `Mamba-Codestral-7B` |
| **Tier 2** | **Background Agent & Heavy MoE** | $\ge 14\text{B}$ ($> 4.5\text{ GB}$) | Hybrid (GPU + RAM Host) | `Bonsai-27B-Q1_0` (16/64L na GPU), `Laguna-XS-2.1-APEX` (11/40L na GPU) |
| **Tier 3** | **Multimodal Vision (VLM)** | Modelos com Projetor mmproj | GPU VRAM + Vision | `UI-TARS-7B`, `Gemma-4-VL`, `Fara-VL` |
| **Tier 4** | **Speculative Drafters** | Modelos de Rascunho MTP | Isolado / dGPU | `Bonsai-dspark`, `qwen35-dflash`, `Laguna-DSpark` |

*Regra de Ouro dos Modelos 7B*: Modelos 7B quantizados cabem no **Tier 1-1.5 (Full VRAM)** com TurboQuant. Se um modelo 7B não couber com KV Cache suficiente na VRAM de 6GB, ele **não é rebaixado para o Tier 2** (onde ficaria lento sem entregar a inteligência de 27B); ele é reportado como inadequado para o hardware. O Tier 2 é restrito para inteligências massivas ($\ge 14\text{B}$ / MoE).

### 3. Separação de Motores no `Cargo.toml`
- **`ik_llama_backend`**: Motor TurboQuant com V-Cache $< \text{Q4}$, FlashAttention O(1) e micro-adaptadores residuais (~1MB / LoRA SVD).
- **`llama_upstream_backend`**: Binding C-API oficial do `llama.cpp` 2026 para compatibilidade universal (Phi-4, Nemotron, LFM, Hy-MT2, Mamba GGUF).
- **`mistral_backend`**: Runtime especializado para arquiteturas State Space Models (SSM/Mamba, Zamba).
- **`all_engines`**: Feature unificada contendo todos os motores do ecossistema.

### 4. V-Cache Ultra-Comprimido no TurboQuant ($< \text{Q4}$)
O `ik_llama_vanguard` suporta compressões assimétricas extremas para o V-Cache (`Q2_K`, `Q3_K`, `IQ2_XXS`), reduzindo a pegada de contexto em até 75% em relação ao F16 e tornando-se um diferencial exclusivo de aceleração bare-metal.

## Consequências e Ganhos
- **Zero VRAM Desperdiçada**: Tiers 0 e 0.5 rodam na CPU com throughput de **70 a 125 tok/s** via AVX2 e gastam 0 MB de VRAM.
- **VRAM Totalmente Livre para o Trabalho Pesado**: A RTX 2060m (6GB) dedica 100% de sua capacidade para o Tier 1 (pesos + KV Cache gigante).
- **Imunidade a Panics Externos**: Toda chamada em worker é blindada com `std::panic::catch_unwind`.
