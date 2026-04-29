---
name: skill-creator
description: A Forja do SODA. Meta-habilidade estrutural ativada para extrair lógicas de repositórios e gerar novas habilidades agênticas. Orquestra a Divulgação Progressiva, criando o SKILL.md e isolando lógicas em scripts atômicos.
triggers: ["criar skill", "destilar repositório", "gerar habilidade", "nova skill", "criar agente", "skill-creator"]
---

### skill: Skill Creator (A Forja de Habilidades SODA)

#### Goal
Atuar como a "skill que cria skills" (A Forja Matriz) dentro do ecossistema Antigravity IDE. Sua missão é extrair a "alma matemática" de repositórios ou lógicas propostas, empacotando-as no padrão agentskills.io (Divulgação Progressiva em 3 Níveis) sob as "Leis Duras" do SODA. O objetivo inegociável é garantir que toda nova ferramenta gerada seja encapsulada com um Frontmatter YAML para roteamento O(1), opere com 6GB de VRAM, seja coberta por TDD (Exit Code 0), utilize IPC Zero-Copy e seja imune a Context Rot.

#### Instructions
Sempre que o usuário solicitar a criação de uma nova habilidade ou a canibalização de um repositório, você DEVE executar esta máquina de estados rigorosa:

1. **Context Engineering & Ingestão O(1) (Proibição de Força Bruta):**
   * Você NUNCA deve ler arquivos inteiros massivamente.
   * Utilize OBRIGATORIAMENTE as ferramentas do `lean-ctx` (`ctx_tree`, `ctx_search`, `ctx_read`) para mapeamento estrutural compacto.
   * Acione o MCP `@mcp-jcodemunch-master` para rasgar a Árvore de Sintaxe Abstrata (AST) do alvo, extraindo heurísticas lógicas em tempo constante $O(1)$.

2. **CSDD & Poda Tóxica (Constitutional Spec-Driven Development):**
   * Elimine lixo tóxico do código extraído (React, Virtual DOM, daemons Node.js, contêineres pesados).
   * Se a nova habilidade requerer lógica executável, aplique TDD Estrito (Red-Green-Refactor). Crie o Scaffold e escreva o teste de compilação (Rust/Wasm) que falha ANTES de codificar o script do Nível 3.

3. **Geração da Taxonomia de 3 Níveis (Late-Binding):** 
   Crie a estrutura na pasta `.agents/skills/<nome-da-skill>/` respeitando a Divulgação Progressiva:
   * **Nível 1 (Frontmatter YAML OBRIGATÓRIO):** O arquivo `SKILL.md` DEVE iniciar rigorosamente com um bloco YAML delimitado por `---`, contendo APENAS `name`, `description` e `triggers`. Isso é vital para a amarração tardia.
   * **Nível 2 (Instruções Core):** Corpo do `SKILL.md` (abaixo do YAML) definindo `Goal`, `Instructions` (em passos imperativos curtos), `Constraints` e `Examples`.
   * **Nível 3 (Sidecars Efêmeros e Validação):** Crie as subpastas `scripts/`, `assets/` e `references/`. Qualquer extração estruturada (JSON/ETL) DEVE impor Decodificação Restrita (`llguidance` ou modo estrito FastMCP) para aniquilar alucinações de schema.

4. **Agent Inbox e Injeção no Grafo (Prevenção SDC):**
   * Você está PROIBIDO de gravar a skill final diretamente na ramificação principal do usuário.
   * A construção deve ocorrer em um *Shadow Workspace*. Gere um *Pull Request* (Diff) para a **Agent Inbox**, aguardando aprovação HITL (Human-in-the-Loop).
   * Prepare o comando de injeção relacional para o **LadybugDB**, mapeando as dependências da nova *skill* para garantir a ontologia do agente.

#### Constraints
* **FRONTMATTER ABSOLUTO:** Nenhuma skill pode ser gerada sem o bloco `---` inicial com nome, descrição e triggers. É a fundação do Roteamento Semântico.
* **SOBREVIVÊNCIA BARE-METAL:** Scripts pesados na pasta `scripts/` devem ser explicitamente desenhados como *Sidecars Efêmeros* confinados em Wasmtime (WASI 0.2) ou Micro-VMs (KVM), abortáveis atómicamente.
* **SEM DEPENDÊNCIAS FANTASMAS:** Toda ferramenta MCP ou pacote invocado pela nova skill deve estar validado no `gateway-config.yaml` ou ser empacotado puramente em Rust.

#### Examples
**Entrada do Usuário:** "SODA, extraia a lógica de parsing daquele repositório de markdown e crie a skill @soda-md-parser."
**Ação do Agente:**
1. Mapeia a estrutura em $O(1)$ usando `ctx_tree` e extrai o núcleo de parsing usando `jcodemunch` (AST).
2. Purifica a lógica descartando dependências de Python regex legadas, planejando um script em Rust puro (CSDD).
3. Escreve os testes no *Shadow Workspace* e codifica a lógica até obter Exit Code 0.
4. Gera o `SKILL.md` iniciando estritamente com o bloco YAML `--- name: soda-md-parser ... ---` e estrutura a subpasta `scripts/`.
5. Reporta no Canvas: *"Forja concluída e testada via TDD. A skill @soda-md-parser foi enviada à Agent Inbox para sua revisão."*