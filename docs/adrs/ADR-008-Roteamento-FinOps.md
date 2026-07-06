---
id: "ADR-008"
title: "ADR-008-Roteamento-FinOps-e-Matematica-AOT"
version: 2.2
status: Ativo_Inegociavel
epic: "FinOps"
description: "Governa o roteamento via ParetoBandit (Tiers), Model Registry dinâmico, e impõe cálculos matemáticos AOT (lele/pulp), banindo runtimes obesos como tch-rs."
---

### ADR-008: Roteamento FinOps, Abstração de Tiers e Trator Numérico AOT

#### Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SODA V4)

#### Contexto Técnico e Ameaça Operacional (Inference Bill Shock e Overhead Matemático)
O uso indiscriminado de modelos de fronteira em nuvem provoca custos financeiros proibitivos (Inference Bill Shock). Para mitigar isso, o sistema requer um roteador (*ParetoBandit*) que calcule rotas com base na métrica E3 (Efficiency-Aware Effectiveness Evaluation). Contudo, se as micro-rotinas de cálculo vetorial para esse roteamento utilizarem *runtimes* de Machine Learning tradicionais (como `tch-rs` ou LibTorch), o binário inflará em gigabytes e as compilações *Just-In-Time* (JIT) gerarão latência letal que ultrapassará o orçamento de <22 microssegundos.

#### Decisão Arquitetural (Tiers Abstratos e Matemática AOT)
Fica decretado o uso da malha de roteamento dinâmico "Zero-Trust Sandwich" operando sob as seguintes abstrações inegociáveis:

**Módulo 1: O Model Registry Dinâmico (O Orçamentista Noturno)**
*   O SODA implementa um **Model Registry** local em SQLite.
*   O *Daemon Chyros* acorda durante a madrugada e realiza um *pull* assíncrono nas APIs para atualizar os preços exatos em microdólares, limites de contexto e latência (Time-To-First-Token - TTFT). O roteamento calcula as rotas em tempo $\mathcal{O}(1)$ consultando apenas o banco local.

**Módulo 2: Taxonomia Abstrata de Roteamento (Tiers de Inteligência)**
*   O roteamento obedecerá estritamente a três camadas (Tiers) escaláveis:
    *   **Tier 1 (Cloud Brain / Orchestrator):** Modelos de fronteira premium (Latest Pro Models) acionados **exclusivamente** para elaborar planos de execução, resolver pânicos severos do compilador Rust ou gerar o DAG de tarefas.
    *   **Tier 3 (Cloud Fast / Batch Workers):** Modelos de altíssima velocidade e baixo custo (Latest Flash/Lite Models) usados para processamento em lote e contingência assíncrona (Failover).
    *   **Tier 4 (Local SLM Workers):** O esforço braçal contínuo é roteado para a GPU local (RTX 2060m via `llama-cpp-2` contíguo ou `mistral.rs` efêmero) a custo rigorosamente zero.

**Módulo 3: O Trator Numérico AOT (A Morte do LibTorch e JIT)**
*   Fica terminantemente **PROIBIDA** a injeção de bibliotecas C++ de tensores obesas (ex: `tch-rs`, LibTorch, ndarray com MKL) para os cálculos de predição do roteador.
*   A matemática matricial do ParetoBandit e outras heurísticas operacionais deverão rodar 100% estáticas via **AOT (Ahead-of-Time)** na CPU.
*   O paralelismo vetorial (SIMD/AVX2) será forçado cirurgicamente em Rust nativo utilizando a combinação mandatória das *crates* **`lele`** e **`pulp`**, garantindo execução O(1) sem concorrência pelo barramento PCIe da GPU.

#### Consequências Operacionais (Trade-offs)
*   **Impacto Positivo:** Proteção absoluta do orçamento financeiro via *Tiers*. O binário Rust se mantém leve e a tomada de decisão do roteador é estritamente sub-milissegundo, preservando o Cache L3 e a memória RAM.
*   **Impacto Negativo (Rigidez Matemática):** A escrita de redes de predição locais exigirá que o Arquiteto lide com matrizes de baixo nível usando `pulp` e `lele` em Rust puro, impedindo a comodidade de importar scripts prontos de Python/PyTorch para cálculos internos.
