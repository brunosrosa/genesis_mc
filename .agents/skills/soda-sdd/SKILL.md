---
name: soda-sdd
description: A Lei de Ferro do SODA (Constitutional Spec-Driven Development). Proíbe a geração de código antes da aprovação de especificações, diagramas Mermaid arquiteturais e quebra de tarefas atômicas. Erradica o Vibe Coding.
triggers: ["soda-sdd", "iniciar feature", "escrever código", "planejar tarefa", "spec-driven development", "programar", "criar módulo"]
---

# Skill: SODA SDD (Spec-Driven Development & First Draft Protocol)

## Goal
Erradicar o *Flow-Debt* e as alucinações arquiteturais derivadas do *Vibe Coding*. Atuar como o motor de orquestração de código do Genesis MC. O objetivo inegociável desta habilidade é garantir que você (o Agente) NUNCA escreva uma linha de código de produção sem antes passar por um rigoroso processo de 4 fases (Specify, Plan, Tasks, Implement), utilizando templates estruturados e testes coercitivos (TDD).

## Instructions
Sempre que o usuário solicitar a criação de um código, módulo ou refatoração, você está expressamente PROIBIDO de gerar código na sua primeira resposta. Você DEVE honrar a seguinte Máquina de Estados em 4 Fases:

1. **Fase 1: Specify (Clarificação Socrática & Invariante Base)**
   - Não adivinhe requisitos. Dialogue com o usuário usando a Maiêutica para extrair o escopo exato e os limites físicos da tarefa.
   - Preencha mentalmente as intenções e formalize os requisitos.
   - *Referência Obrigatória:* Utilize a estrutura do arquivo `assets/project_brief_template.md` (se existir) para nortear as perguntas.

2. **Fase 2: Plan (O Tratado de Arquitetura)**
   - Traduza a intenção aprovada em topologia de software.
   - Especifique como o Rust, Tauri IPC ou SQLite serão alterados.
   - Você DEVE desenhar Diagramas Mermaid (Contexto ou Sequência) para ancorar o raciocínio espacial.
   - *Referência Obrigatória:* Preencha a topologia utilizando o esqueleto do arquivo `assets/design_template.md`. Exija a aprovação humana do diagrama antes de seguir.

3. **Fase 3: Tasks (A Desfragmentação em Grafo)**
   - O escopo global aprovado deve ser quebrado em tarefas microscópicas, atômicas e testáveis em isolamento (Grafo Acíclico Dirigido - DAG).
   - *Referência Obrigatória:* Formate a lista estrita utilizando o modelo `assets/tasks_template.md`.

4. **Fase 4: Implement (Síntese Guiada & Strict TDD)**
   - Apenas neste estágio o código em Rust/React é gerado.
   - Você deve operar sob a "Lei de Ferro do TDD" (Red-Green-Refactor): Escreva o teste que falha PRIMEIRO. Se o teste passar sem código de produção, você alucinou um falso positivo; descarte e refaça.
   - *Execução Coercitiva:* Onde aplicável, instrua a execução de scripts de validação na pasta `scripts/soda_tdd_interceptor.sh` ou use o terminal para compilar (`cargo check`/`npm run test`) antes de dar a tarefa como concluída.

## Constraints
* **PROIBIÇÃO DE VIBE CODING:** Sob nenhuma hipótese entregue funções completas ou gere arquivos `.rs`/`.tsx` finais sem que as Fases 1, 2 e 3 tenham sido impressas na tela e aprovadas.
* **PROIBIÇÃO DE HOMOGENEIZAÇÃO:** Respeite a Constituição da arquitetura SODA. O código deve ser *Bare-Metal* (Rust), usar concorrência *Tokio*, e interagir com React de forma passiva via Canvas/IPC.
* **FIRST DRAFT PROTOCOL:** Todo código gerado é considerado um "Primeiro Rascunho". Se o usuário rejeitar a compilação, você não deve debater; reverta a lógica e analise a falha do compilador friamente.

## Examples
**Entrada do Usuário:** "Crie um módulo de login com banco local."
**Ação do Agente:**
- (Recusa código imediato).
- Inicia a Fase 1 perguntando sobre a tabela exata no SQLite e se a autenticação exige integração biométrica.
- Gera o `design.md` com diagrama Mermaid do fluxo IPC (Fase 2) e aguarda "OK".
- Gera as tarefas no `tasks.md` (Fase 3).
- Inicia a codificação (Fase 4), criando primeiro o `#[test]` em Rust para a query do banco.