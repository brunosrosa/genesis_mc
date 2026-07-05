---
id: "ADR-019"
title: "ADR-019-Maquina-de-Estados-ETL"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Formaliza as fases sequenciais (de N0 a N6) para processamento, ingestão e estruturação de novos repositórios."
---

# ADR-019-Maquina-de-Estados-ETL

## Status
Aceito (Ativo e Inegociável)

## Contexto
Durante tarefas de extração estruturada de repositórios ("canibalização") e geração de código complexo por IAs, processos paralelos sem orquestração determinística podem causar descompasso de estados no repositório. Por exemplo: um agente iniciar a escrita da lógica de negócios sem ter validado o design, ou realizar o rebase no disco de produção sem ter passado pelos testes Clippy. Esse comportamento estocástico fragmentado destrói a integridade do código e gera confusão espacial letal na linha de montagem.

## Decisão
Impor a orquestração do pipeline cognitivo e da esteira de montagem sob uma **Máquina de Estados Finitos (FSM)** rígida e atômica compilada no core Rust (Tokio):
1. **Transições de Fases Sequenciais e Inquebráveis:** O processamento obedece incondicionalmente a gatilhos determinísticos de transição de estado da tarefa:
   - `PENDENTE_FASE_0` (Triagem & Viabilidade): Análise estática inicial, verificação de Linhas Vermelhas da SSOT e cálculo preliminar do ParetoBandit.
   - `EM_ANDAMENTO_FASE_1` (Isolamento Físico): Instanciação do snapsafe no Shadow Workspace e importação do contorno AST.
   - `PRONTO_PARA_FASE_2` (Design & Contrato): Escrita do Mermaid e do `docs/design.md` em disco com aprovação de arquitetura do usuário.
   - `EM_ANDAMENTO_FASE_3` (TDD Atômico): Escrita do `tasks.md`, scaffolds de falha (Red) e início do Ralph Loop.
   - `PRONTO_PARA_FASE_4` (Consolidação): Código purificado aprovado no compilador e Clippy com Exit Code 0 (Green).
   - `PRONTO_PARA_FASE_5` (Rebase Semântico & Audit): HITL consolidado na Agent Inbox, rebase atômico em disco, fechamento de telemetria no Sheets e auditoria de release finalizada.
2. **Garantia Fail-Closed de Estado:** Qualquer falha que quebre a esteira (erros não corrigidos pelo Ralph Loop, rejeição de design ou drifts) congela a tarefa. O SODA força o recuo atômico imediato da FSM para o último estado seguro persistido na L2, limpando a mesa de rascunhos.
3. **Persistência de Logs de Fases:** Cada mudança de fase do pipeline é gravada como um evento temporal imutável e assinado no SQLite L2 para auditoria retrospectiva da IA e do usuário.

## Consequências
- **Consistência Sistêmica Absoluta:** O repositório permanece previsível e sintaticamente íntegro, bloqueando ações prematuras ou incompletas de IAs.
- **Transparência de Linha de Montagem:** O usuário visualiza com clareza o estágio exato de processamento do enxame de agentes diretamente na Bottom Bar de status de telemetria.
- **Robustez Térmica:** O agendador Rust suspende lógicas de forma segura em caso de falhas mecânicas locais, mitigando picos de processamento na CPU.

## Restrições Bare-Metal
- **Latência de Transição de Estado:** O cálculo de transição da FSM e escrita no SQLite local deve rodar em menos de **3ms**.
- **Travamento por Tipagem Estrita (Rust Typestate):** Cada estado da FSM é modelado por um tipo de dado específico no Rust. Uma tarefa no estado `PENDENTE_FASE_0` não possui fisicamente métodos implementados para disparar o Rebase em disco, blindando o software no nível do compilador.
