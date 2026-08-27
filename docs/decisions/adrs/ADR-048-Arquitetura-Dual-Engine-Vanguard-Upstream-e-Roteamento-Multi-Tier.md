# ADR-048: Arquitetura Dual-Engine (ik_llama_vanguard + llama_upstream) e Orquestração Multi-Tier

## Status
**APROVADO / EM PRODUÇÃO** (Marco 4.11 — 2026-08-26)

## Contexto e Motivação
Historicamente, o ecossistema SOULS operava com uma flag genérica `llama_backend`, vinculada estaticamente ao motor experimental `ik_llama.cpp` (Vanguard). Embora o `ik_llama.cpp` ofereça aceleração de vanguarda para quantizações *i-matrix* e compressão assimétrica de KV Cache (TurboQuant `K:F16/V:Q4`), ele não contém kernels recentes para novas arquiteturas do mercado de 2026 (tais como `Phi-4-mini`, `NVIDIA-Nemotron`, `LFM2.5` e modelos SSM/Mamba).

A tentativa de forçar todas as arquiteturas em um único motor gerava falhas de carregamento de vocabulário/tensores ou degradação de throughput.

## Decisão de Arquitetura

### 1. Separação Estrita de Features no Cargo (`Cargo.toml`)
Erradicou-se a ambiguidade de nomenclatura através da especialização explícita:
- **`ik_llama_backend`**: Motor de Vanguarda compilado com TurboQuant, suporte a micro-adaptadores residuais (~1MB / LoRA SVD) e aceleração MMQ/F16 para tensores *i-matrix*.
- **`llama_upstream_backend`**: Motor Oficial Upstream (llama.cpp Baunilha de 2026) que atua como padrão universal de compatibilidade.
- **`mistral_backend`**: Runtime especializado para arquiteturas State Space Models (SSM/Mamba, Zamba, Codestral-SSM).
- **`all_engines`**: Feature unificada contendo todos os motores do ecossistema.
- **`llama_backend`**: Alias de retrocompatibilidade apontando para `ik_llama_backend`.

### 2. Política de Roteamento no `EngineCascade`
O roteamento no `EngineCascade` opera sob a tríade de compatibilidade:
1. **SSM / Mamba / Zamba**: Roteados nativamente com prioridade máxima (`Native(300)`) para `mistral_rs`.
2. **Quantizações i-matrix / TurboQuant V-Cache / LoRA Residual**: Roteados com prioridade (`Native(250)`) para `ik_llama_vanguard`.
3. **Padrão Universal e Arquiteturas Upstream (Phi-4, Nemotron, LFM, Hy-MT2)**: Roteados para `llama_upstream` (`Native(220)`).

### 3. Orquestração Multi-Tier da SOULS Arena
A Arena opera em 5 Tiers Físicos de silício:
- **Tier 0 (Bootstrap & Sanity)**: Execução comparativa obrigatória em **CPU (AVX2)** e **GPU (CUDA)**.
- **Tier 0.5 (Sensor Epistêmico & Probing)**: Execução em CPU (`llama_cpp4`) e GPU.
- **Tier 1 / 1.5 (Live Chat & Master Agents)**: GPU Full VRAM com TurboQuant.
- **Tier 2 (Background Agents & MoE)**: Hybrid Offload VRAM + RAM Host.
- **Tier 4 (Speculative Drafters)**: Rascunho especulativo (DSpark/dflash) isolado.

## Consequências e Ganhos
- **Zero Crashes**: Nenhum modelo é submetido a um motor incompatível.
- **Transparência FinOps**: Modelos aguardando sidecars upstream são reportados com clareza (`PENDING_UPSTREAM`, `PENDING_MISTRAL`).
- **Throughput Bare-Metal**: Modelos padrão atingem picos de até **130.7 TPS** em CPU AVX2 e **128.7 TPS** em GPU CUDA na RTX 2060m.
