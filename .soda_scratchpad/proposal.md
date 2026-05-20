---
title: "Higiene de Repositório + Alinhamento Pós-Pivot (Fase 1.5)"
status: "ativo"
owner: "antigravity-ide"
---

## Problema

O repositório contém sinais de drift entre documentação e implementação (principalmente em torno do Milestone 01 e do uso de ferramentas de “Fábrica”), além de pontos de fragilidade (paths hardcoded e anomalias de IPC) que atrapalham manutenção e CI.

## Objetivo

Higienizar documentação e código **sem alterar o comportamento do Motor de ETL (Fase 1.5)**, garantindo:

- documentação alinhada à pivotagem e à dualidade Fábrica vs Produto;
- testes do núcleo FinOps/Phase 1.5 estáveis em Linux/CI;
- `cargo clippy -- -D warnings` sem regressões;
- remoção do legado React do frontend (preparação para Svelte 5).

## Não-Objetivos

- criar novas features do ETL (Fase 1.5 permanece intacta);
- avançar Milestone 01 além de alinhamento documental e preparação de terreno;
- refatorações profundas fora do escopo de higiene.

## Portões de Qualidade (DoD)

- `cargo test finops::phase1_5` passa (suite completa).
- `cargo clippy -- -D warnings` passa.
- Sem paths hardcoded dependentes de Windows em testes críticos.

