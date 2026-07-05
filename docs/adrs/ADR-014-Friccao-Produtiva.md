---
id: "ADR-014"
title: "ADR-014-Friccao-Produtiva"
version: 1.0
status: Ativo_Inegociavel
epic: "Cognição"
description: "Introduz atrasos deliberados e feedbacks sutis na UI para evitar submissão cognitiva às sugestões da IA."
---

# ADR-014-Friccao-Produtiva

## Status
Aceito (Ativo e Inegociável)

## Contexto
O Viés de Automação (*Automation Bias*) induz seres humanos a aceitar de forma rápida e irrefletida propostas complexas geradas por sistemas de inteligência artificial. Em ambientes de desenvolvimento agêntico rápidos, se o agente de IA propõe e injeta modificações extensas de código em milissegundos na tela, o usuário tende a clicar impulsivamente em "aprovar" para economizar esforço mental imediato. Esse hábito impulsivo sabota a integridade do código ("slop") e anula o rigor analítico do Spec-Driven Development, gerando bugs graves.

## Decisão
Implementar a arquitetura de **Fricção Produtiva (Fricção Cognitiva Estruturada)** nas vias de interação visual e agêntica do SODA:
1. **Divergência de Resposta:** O tempo de reação da interface gráfica do SODA divide-se intencionalmente baseando-se na origem da ação:
   - *Ação Mecânica Direta (Humano):* Deve responder de forma tátil e instantânea em menos de **50ms a 150ms**.
   - *Ação Agêntica Autônoma (IA):* Tarefas complexas de alto Blast Radius geradas de forma autônoma pela IA (ex: planos de refatoração, mutações atômicas de arquivos ou rebases semânticos) exigem a inserção compulsória de um **Atraso Sintético de 800ms a 1500ms**.
2. **Exibição de Telemetria de Progresso:** Durante o decurso do atraso sintético, a interface gráfica bloqueia interações rápidas impulsivas do usuário e exibe logs monoespaçados estáticos monoespaçados na Bottom Bar de telemetria (ex: `-> Computando Blast Radius -> Validando compilador -> Pronto`).
3. **Ancoragem Consciente do Neocórtex:** A lentidão intencional e controlada obriga a atenção do usuário a focar na análise das implicações reais documentadas no Blast Radius Canvas antes de conceder a autorização tátil final.

## Consequências
- **Erradicação do Slop Impulsivo:** O usuário audita conscientemente as modificações de IA com rigor, detectando incoerências lógicas de inferência antes de fundi-las à branch principal.
- **Redução do Context Rot Mental:** O atraso estruturado fornece respiro mental para o programador, mitigando o cansaço mental e a ansiedade visual associados ao desenvolvimento acelerado.
- **Rigor Metodológico:** A esteira de engenharia mantém alto nível de conformidade técnica com o Spec-Driven Development.

## Restrições Bare-Metal
- **Atraso Sintético Rígido:** Atraso artificial parametrizado obrigatoriamente entre **800ms e 1500ms** para ações cognitivas de agentes de IA na interface.
- **Latência Humana:** Interações que partam de comandos puramente manuais do usuário do SODA ignoram o atraso, respondendo em menos de **150ms** para manter o feedback físico de controle de máquina.
