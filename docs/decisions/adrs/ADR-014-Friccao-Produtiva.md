---
id: "ADR-014"
title: "ADR-014-Friccao-Produtiva"
version: 2.0
status: Ativo_Inegociavel
epic: "Cognição"
description: "Introduz a Fricção Cognitiva Estruturada e a interrupção socrática em sessão ativa (chat/CLI) para erradicar o viés de automação e o aceite impulsivo de código, contornando a UI gráfica enquanto a Milestone 4 estiver inativa."
---

# ADR-014: Fricção Produtiva, Rapport Socrático e Pragmatismo HITL em Chat

## Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SOULS V6)

## Contexto Técnico e o Perigo do Viés de Automação
O Viés de Automação (*Automation Bias*) induz seres humanos a aceitar de forma impulsiva e irrefletida propostas complexas geradas por sistemas agênticos. Se o agente de IA injeta modificações extensas de código em milissegundos sem forçar uma pausa reflexiva, o operador tende a aprovar a alteração para economizar esforço mental imediato. Esse hábito sabota o Spec-Driven Development e introduz código degradado ("slop") no repositório.

## Decisão Arquitetural (Fricção Cognitiva Estruturada e Rapport Socrático em CLI)
Fica estabelecida a obrigatoriedade da **Fricção Produtiva** adaptada à **Regra de Pragmatismo de Interface**:

### 1. Interrupção Socrática em Sessão Ativa (Chat/CLI)
*   Como a Milestone 4 (Frontend Canvas) não está ativa, toda verificação de ambiguidade e aprovação de Blast Radius ocorre **na própria thread de chat ativo**.
*   A runtime Rust/Tokio pausa assincronamente a execução e exibe o diff/resumo no stdout/chat.
*   O agente formula perguntas socráticas direcionadas e objetivas (**Rapport Socrático sem "Por que"**), forçando o operador a engajar o neocórtex e validar o alinhamento da intenção antes de digitar a confirmação.

### 2. Divergência de Resposta e Telemetria
*   **Comandos Mecânicos Diretos (Humano):** Resposta instantânea tátil em menos de **50ms a 150ms**.
*   **Ações Agênticas Autônomas (IA):** Modificações de alto Blast Radius exigem a inserção de um **Atraso Sintético de 800ms a 1500ms** na transmissão de eventos para estabilização de contexto e exibição de telemetria monoespaçada (ex.: `-> Gerando diff -> Validando AST -> Pausado para aprovação HITL`).

## Consequências Operacionais
- **Erradicação do Slop Impulsivo:** A interrupção no chat obriga o operador a ler e autorizar conscientemente as alterações propostas.
- **Transparência e Respiro Mental:** Mitigação da ansiedade visual e prevenção do *Context Rot* do operador durante refatorações extensas.

## Restrições Bare-Metal
- **Atraso Sintético:** Parametrizado entre **800ms e 1500ms** para eventos agênticos de alto impacto.
- **Rapport Socrático Rígido:** Perguntas de esclarecimento devem ser estritamente objetivas e diretas, sendo banido o uso do advérbio acusatório "Por que".
