# SODA — Estado Atual (Pivotagem Estratégica)

## Resumo Executivo

O SODA passou por uma pivotagem estratégica: a prioridade absoluta do repositório, neste momento, é o **Motor de ETL Cognitivo em Rust** (Pipeline **Fase 1.5**), usado para mastigar e dissecar centenas de repositórios open-source e extrair arquitetura/heurísticas para gerar o roadmap e os PRDs futuros.

A **UI (Milestone 01)** está em **hibernação intencional**: não estamos avançando a casca visual enquanto o ETL não preencher o banco soberano e não houver clareza de priorização.

## O que é “Produto” vs “Fábrica”

Este repositório contém dois modos de existência:

- **Produto (target final):** stack imutável **Rust/Tokio + Svelte 5 + Vite + Tauri v2**, IPC eficiente e sem dependências pesadas em runtime. A UI é um renderizador passivo.
- **Fábrica (modo de construção):** ferramentas auxiliares podem existir para acelerar a construção do ETL e a análise em lote (ex.: roteador/gateway e integrações), desde que **não deformem** o núcleo bare-metal e não quebrem o pipeline de ETL.

## Prioridade de Entrega (Agora)

1. **Fase 1:** concluída (Harvester/extração base e persistência inicial).
2. **Fase 1.5:** em construção e prioridade absoluta (FinOps + orquestração de pipeline para operação em lote).
3. **Fase 2/3/4:** ainda não iniciadas (dependem do ETL preencher o vault e da priorização do roadmap).
4. **Milestone 01 (UI):** pausado por decisão estratégica.

## Regras de Higiene (para evitar “drift”)

- Documentação deve refletir a pivotagem: o Milestone 01 é “produto final”, mas não é o foco atual.
- React é considerado legado/ruído: o terreno do frontend deve ser preparado para **Svelte 5**.
- Mudanças de higiene não podem quebrar a **Fase 1.5** nem introduzir warnings em Rust.

## Sinais de Saúde do Repositório

- O foco de validação obrigatória, por ora, é: `cargo test finops::phase1_5` e `cargo clippy -- -D warnings`.

