---
name: soda-sdd
description: A Lei de Ferro do Antigravity IDE (Spec-Driven Development). Proíbe a geração de código (Vibe Coding) antes da aprovação de diagramas estáticos em disco. Orquestra o BMAD local, exige consumo de ADRs, bloqueia Falso Consenso e aciona o Ralph Loop para falhas sintáticas.
triggers: ["soda-sdd", "iniciar feature", "escrever código", "planejar tarefa", "spec-driven development", "programar", "criar módulo", "implementar"]
---

### skill: SODA SDD (Spec-Driven Development & First Draft Protocol)

#### Goal
Erradicar o *Flow-Debt* e as alucinações arquiteturais derivadas do *Vibe Coding* [7]. Atuar como o chicote metodológico de orquestração de código do IDE. O objetivo inegociável desta habilidade é garantir que você (o Agente) NUNCA escreva uma linha de código fonte sem antes: identificar a origem da tarefa (No Ticket, No Code), consumir as ADRs do projeto, gravar diagramas estruturais em disco, e provar a lógica no terminal nativo através do TDD estrito [5, 6].

#### Instructions
Sempre que o usuário solicitar a codificação de uma nova funcionalidade, refatoração profunda ou criação de módulo, você está expressamente PROIBIDO de gerar código-fonte diretamente [7]. Você DEVE honrar o protocolo BMAD (Branch, Mutate, Approve, Diff) executando estas 5 Fases estritas:

1. **Fase 1: Isolamento Físico e Rastreador de Intenção (Branch)**
   * **Proibição de Ghost Coding:** Identifique o escopo ou a *Issue* que justifica esta tarefa. Se não houver contexto, interrogue o usuário [5, 6].
   * O SDD *consome* arquitetura, não a inventa. Leia os Contratos Imutáveis (ex: `ARCHITECTURE.md` ou regras do workspace) [8, 9].
   * Utilize o terminal local para criar e mover-se para uma ramificação temporária (Shadow Workspace): `git checkout -b feat/<nome>` [6, 10].

2. **Fase 2: O Tratado de Arquitetura no Disco (Plan)**
   * Você DEVE gravar as definições em um arquivo estático real (ex: `docs/design.md`).
   * Especifique as regras físicas (ex: IPC Zero-Copy, Rust/Svelte 5) e grave obrigatoriamente um Diagrama Mermaid (Contexto ou Sequência) dentro do arquivo para ancorar o raciocínio espacial [6].
   * Exija que o usuário leia o arquivo gerado e digite "Aprovado" no chat antes de avançar [6].

3. **Fase 3: Desfragmentação Obrigatória (Tasks)**
   * Quebre o design aprovado em passos estritamente atômicos e testáveis dentro de um arquivo físico `tasks.md` [6].

4. **Fase 4: Mutação, TDD e Delegação ao Ralph Loop (Mutate & Implement)**
   * Apenas nesta fase o código de produção (`.rs`, `.ts`, `.svelte`) nasce [6].
   * **Lei do TDD Local:** Escreva o teste de unidade PRIMEIRO. Abra o terminal nativo da IDE e rode `cargo test` para provar que falha (Red). Escreva o código. Rode `cargo check` ou `cargo test` para obter *Exit Code 0* (Green) [6].
   * **Falha Crítica:** O compilador é o seu único juiz. Se o código falhar repetidamente, você está PROIBIDO de gerar "falso consenso". Não fique adivinhando: delegue a resolução sintática invocando a skill `@soda-ralph-loop` [3, 11].

5. **Fase 5: Anti-Consenso e Aprovação Humana (Approve & Diff)**
   * Se o código compilar com sucesso absoluto na branch isolada, NÃO faça o *merge* para a `main` sozinho [10].
   * Resuma o "Blast Radius" (todos os arquivos que foram tocados/criados) no Canvas e aguarde o usuário digitar explicitamente "Aprovado" para você finalizar a mesclagem local [10].

#### Constraints
* **PROIBIÇÃO ABSOLUTA DE VIBE CODING:** O salto direto do prompt para o código final é uma violação gravíssima. O planejamento em Markdown precede a mutação do código.
* **CONSENSUS-FREE MAD:** Destrua premissas frágeis. Em caso de impasse arquitetural, você deve paralisar e exigir a decisão do Arquiteto (usuário) [11].
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável para o roteamento correto no IDE.

#### Examples
**Entrada do Usuário:** "Precisamos de um novo endpoint IPC para gerenciar as rotas de FinOps do ParetoBandit."

**Ação do Agente:**
1. Confirma o escopo e consome a documentação do Gateway. Roda no terminal: `git checkout -b feat/finops-ipc`.
2. Cria o arquivo `docs/design.md` contendo um Diagrama Mermaid do fluxo Zero-Copy [6]. E avisa no chat: *"Arquiteto, o design em docs/design.md está aprovado?"*.
3. Após o 'Sim', gera as etapas no arquivo `tasks.md` [6].
4. Escreve um teste no Rust. Roda `cargo test` e comprova a falha. Implementa o endpoint. O `cargo check` falha com um erro brutal de *Borrow Checker*.
5. O agente **não tenta adivinhar** e invoca imediatamente: `"Executando @soda-ralph-loop para destrinchar a saída do compilador..."` [4].
6. Após a skill do Ralph Loop resolver o erro e atingir Exit Code 0, o agente relata o "Blast Radius" e pede a autorização final para o merge [4, 10].
