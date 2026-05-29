---
title: "SODA Canon V5: Governança Topológica e Execução Bare-Metal Unificada"
version: "5.0.0"
status: "Ativo / Imutável"
scope: "Global (Antigravity IDE, Trae PRO+, Agentes Autônomos)"
author: "SODA Architectural Core"
tags: [governança, topologia, gitops, bmad, rust, svelte5, sdd, anti-slop]
description: "Lei máxima do ecossistema SODA. Funde a topologia física aprovada, protocolos de execução TDD/BMAD e restrições absolutas de hardware/UI."
---

# SODA Canon V5: Governança de Território e Protocolos de Execução

Este artefato é a Constituição do ecossistema SODA (Sistema Operacional Agêntico Soberano). Ele foi forjado para combater a entropia natural de IAs autônomas ("slop"). Todo LLM ou Agente operando neste *workspace* está hierárquica e fisicamente subordinado às cláusulas aqui dispostas.

---

## 1. O Paradigma Temporal (Fábrica vs. Produto)

A linha do tempo do SODA repudia o "Context Rot" dividindo a construção da ferramenta e a construção do produto final.

* **Fase 0 (A Fábrica / ETL Cognitivo):** O motor de ingestão $\mathcal{O}(1)$ (via MCPs e `jcodemunch`) que mastiga repositórios e extrai o "ouro matemático" para a matriz de dados. Ambiente pragmático onde scripts temporários (Python/Bash) habitam o `.soda_scratchpad`.
* **Fases A, B, C... (O Produto / Genesis MC):** O código de produção final. Intolerância absoluta a runtimes interpretados. Backend 100% Rust assíncrono (Tokio). Frontend 100% Svelte 5 (Runes).

---

## 2. A Topologia Imutável (Governança de Território)

A estrutura de pastas é uma barreira de proteção. O Agente é **proibido** de invocar `mkdir` ou criar diretórios como `utils/` ou `scripts/` fora desta cartografia.

### 2.1. O Motor Cognitivo (Ignorado pelo Git)
* `.soda_data/`: **O Disco de Estado.** Bancos SQLite, LanceDB (`lancedb_store`) e Grafos (`ladybug_graph`). Intocável por engenheiros de UI.
* `.soda_cache/`: **O Hipocampo L1.** Respostas de APIs e ASTs brutas transientes.
* `.soda_scratchpad/`: **O Chão de Fábrica.** OBRIGATÓRIO salvar relatórios analíticos (`/reports/`) e logs de erro (`.log`) aqui. *Proibido ejetar logs na raiz.*
* `.soda_sandbox/`: **Isolamento.** Ambientes para execução não confiável de código de terceiros.

### 2.2. A Fortaleza Documental (`docs/`)
* `docs/adrs/`: **O Por Quê.** Architecture Decision Records.
* `docs/dags/`: **O Como.** Grafos Acíclicos Dirigidos determinando fluxos de dados (ex: `DAG_fase_0_design_phase1.md`).
* `docs/prds/`: **A Execução.** Tarefas atômicas de TDD. Arquivos em andamento ficam na raiz, finalizados (Exit Code 0) migram para `docs/prds/archive/`.
* `docs/state/`: **O Agora.** A "Foto do Momento" (`SODA_CURRENT_STATE.md`) para o Agente sincronizar o contexto sem ler arquivos defasados.

### 2.3. A Fundação Bare-Metal (Código de Produção)
* `src-tauri/src/core/` & `src-tauri/src/harvester/`: Motores de negócio em Rust puro.
* `src-tauri/src/ipc/`: Alfândega Zero-Copy. Transição de dados brutos para a UI.
* `src/components/` & `src/routes/`: UI Passiva Svelte 5. Nenhuma regra de negócio deve habitar aqui.

---

## 3. Restrições Arquiteturais (A Lei do Bare-Metal)

O SODA foi desenhado para operar livre de nuvem, respeitando os limites físicos de uma máquina com 32GB de RAM e GPUs de classe consumidor (RTX 2060m).
* **Fronteira UI (Anti-VRAM Burn):** É formalmente **banida** a utilização da estética "Liquid Glass" (filtros de desfoque, `backdrop-filter` agressivo). O processamento de GPU é vitalício da inferência local (LLMs via `llama.cpp` mmap). A UI Svelte 5 deve ser utilitária, planar e renderizada com baixo custo energético.
* **Comunicação Zero-Copy:** O uso massivo de serialização JSON entre Rust e Svelte é repudiado. Utiliza-se ponteiros de transferência e memória alocada estritamente para evitar estrangulamento do *Garbage Collector* do navegador (V8).

---

## 4. O Protocolo GitOps de Orquestração (BMAD Assíncrono)

Agentes no modo autônomo **NUNCA** efetuam commits diretos na branch `main`. A esteira operacional exige o padrão **BMAD**:

1. **[B]ranch (Isolamento):** Ao receber o `PRD_001`, o agente executa `git checkout -b feature/PRD_001`.
2. **[M]utate (Micro-Commits via TDD):** O Agente opera no ciclo *Red-Green-Refactor*:
   * Commit Red: Teste falho.
   * Commit Green: Lógica Rust com Exit Code 0.
   * Commit Refactor: Limpeza de *warnings* do `cargo clippy`.
3. **[A]pprove (Alfândega HITL):** O código paralisa. O Arquiteto Humano revisa o Diff para garantir alinhamento neuro-inclusivo e arquitetural.
4. **[D]iff & Merge (Fusão Histórica):** Fusão sem *fast-forward* para manter a história atômica (`git merge --no-ff feature/PRD_001`), seguida da deleção da branch.

---

## 5. Orquestração Pós-TDD: A Costura de Sistemas

Passar no TDD unitário não significa que a *feature* está apta.
* **PRD de Costura (**`PRD_INT_XXX`**):** Módulos construídos independentemente são acoplados através de um PRD exclusivo focado no roteamento End-to-End, sem injeção de novas lógicas de negócio.
* **Auditoria de Release:** Nenhuma versão é fundida ao núcleo do sistema sem superar o juízo final dos linters:
  `cargo clippy --all-targets --all-features -- -D warnings && cargo test --all && cargo build --release`