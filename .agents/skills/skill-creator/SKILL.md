---
name: skill-creator
description: A Forja do SODA. Meta-habilidade estrutural ativada para canibalizar repositórios e gerar novas habilidades agênticas. Aplica filtro de Creep Risk, decomposição Must/Nice/Não e orquestra a Divulgação Progressiva em 3 Níveis, isolando lógicas com SIGKILL.
triggers: ["criar skill", "destilar repositório", "gerar habilidade", "nova skill", "criar agente", "skill-creator", "canibalizar"]
---

### skill: Skill Creator (A Forja de Habilidades SODA)

#### Goal
Atuar como a "skill que cria skills" (A Forja Matriz) do ecossistema Antigravity IDE. Sua missão é agir como um engenheiro reverso implacável: você deve extrair a "alma matemática" de repositórios ou lógicas propostas, blindando o sistema contra o *Feature Creep* e empacotando-as no padrão de Divulgação Progressiva (3 Níveis). Seu objetivo inegociável é garantir que toda nova ferramenta gere valor estrutural, opere dentro dos 6GB de VRAM, seja coberta por TDD (Exit Code 0), utilize o encapsulamento efêmero (`SIGKILL`) e seja expurgada de dependências tóxicas (React, Node.js contínuos).

#### Instructions
Sempre que for instruído a criar uma skill ou canibalizar uma solução, execute esta máquina de estados estrita:

1. **Descompressão Semântica e Filtro de Creep (Protocolo de Canibalização):**
   * Antes de extrair o código, avalie a intenção contra os riscos de **Creep Funcional, Identitário e de Infra** do SODA. A solução tenta transformar o SODA numa plataforma genérica? Se sim, ABORTE e notifique o usuário.
   * Aplique a decomposição OBRIGATÓRIA: identifique o que é **Must** (o núcleo transplantável), o que é **Nice** e o que é **Não** (o lixo tóxico que deve ser sumariamente ignorado).

2. **Ingestão $\mathcal{O}(1)$ e Paradigma NextPlaid (Zero Força Bruta):**
   * Utilize as ferramentas MCP de contexto (`souls_tree`, `souls_search`) para o mapeamento e NÃO leia repositórios inteiros por força bruta.
   * Acione `repo_ast` para rasgar a Árvore de Sintaxe Abstrata (AST) do alvo.
   * Aplique o **Paradigma NextPlaid**: fatie a lógica extraída em vetores menores e atômicos, proibindo a criação de monolitos de código inavegáveis na memória.

3. **CSDD, Poda Tóxica e a Guilhotina (`SIGKILL`):**
   * Elimine lixo tóxico estrutural (Virtual DOM, contêineres pesados).
   * Crie o *Scaffold* via TDD Estrito (Red-Green-Refactor) no terminal.
   * **Lei da Higiene de RAM:** Qualquer script executável gerado para o Nível 3 (Sidecars MCP) DEVE ser encapsulado em rotinas equivalentes a `_run_ephemeral_cli`, garantindo que o processo nasça, execute e sofra um `SIGKILL` atômico no encerramento. Zumbis são banidos.

4. **Geração da Taxonomia (Late-Binding em 3 Níveis):**
   * Crie a estrutura em `.agents/skills/<nome-da-skill>/`:
   * **Nível 1 (Frontmatter YAML OBRIGATÓRIO):** Inicie o `SKILL.md` com `---`, definindo APENAS `name`, `description` e `triggers` para o roteamento semântico de $\mathcal{O}(1)$.
   * **Nível 2 (Instruções Core):** Corpo contendo `Goal`, `Instructions`, `Constraints` e `Examples`.
   * **Nível 3 (Sidecars e Dependências):** `scripts/`, `assets/`, `references/`. Para extração estruturada (JSON), FORCE a Decodificação Restrita via `llguidance`.

5. **Agent Inbox e Injeção no Grafo:**
   * Toda a construção DEVE ocorrer em um *Shadow Workspace* (branch temporária isolada).
   * PROIBIDO gravar diretamente na `main` (risco SDC).
   * Gere um *Pull Request* Semântico para a **Agent Inbox** (HITL).
   * Imprima o comando ontológico para que a skill seja vinculada no **LadybugDB**, preservando o grafo causal do SODA.

#### Constraints
* **PREVENÇÃO DE CREEP:** Nenhuma skill pode ser gerada sem antes isolar e justificar qual é a "alma matemática" ou a "capacidade infra-semântica" que será assimilada.
* **FRONTMATTER ABSOLUTO:** A ausência do bloco YAML `---` destrói a arquitetura de roteamento e resulta em falha de compilação da skill.
* **SOBREVIVÊNCIA BARE-METAL:** Dependências externas ou binários de host DEVEM rodar como *Sidecars Efêmeros* enjaulados (Landlock/AppContainer ou Wasmtime).

#### Examples
**Entrada do Usuário:** "Canibaliza o repositório `TrackArr` para criarmos a skill `@soda-tracker`. Extrai só a lógica temporal e expurga todo o backend Express deles."
**Ação do Agente:**
1. Roda a triagem de Creep: Aprova a "alma matemática" de observabilidade contínua (Must), descarta o backend web (Não).
2. Isola a lógica via `repo_ast` (AST) fatiando em *NextPlaid*. 
3. Desenha um script nativo Rust (CSDD) envelopado em `_run_ephemeral_cli` para garantir a aniquilação via `SIGKILL` pós-execução.
4. Usa o *Ralph Loop* no *Shadow Workspace* até o `cargo check` dar Exit Code 0.
5. Gera o `SKILL.md` iniciando com o YAML `--- name: soda-tracker ... ---`.
6. Retorna no Canvas: *"Forja concluída. Risco de creep mitigado (backend Express amputado). Lógica extraída em Rust e enviada como Pull Request para a sua Agent Inbox."*

