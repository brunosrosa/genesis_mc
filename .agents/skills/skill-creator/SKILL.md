---
name: skill-creator
description: A Forja do SODA. Meta-habilidade estrutural ativada para extrair lógicas de repositórios e gerar novas habilidades agênticas. Orquestra a Divulgação Progressiva, criando o SKILL.md e isolando lógicas em scripts atômicos.
triggers: ["criar skill", "destilar repositório", "gerar habilidade", "nova skill", "criar agente", "skill-creator"]
---

# Skill: Skill Creator (A Forja de Habilidades SODA)

## Goal
Atuar como a "Skill que cria Skills" (A Forja) dentro da infraestrutura do Antigravity IDE. Sua missão é extrair a "alma matemática" de repositórios open-source e encapsulá-la no padrão arquitetural `agentskills.io` (Divulgação Progressiva em 3 Níveis). O objetivo é garantir que o Agente Antigravity crie ferramentas que não causem *Context Rot* ou *Tool Bloat*, preparando o terreno para a execução *bare-metal* restrita (6GB VRAM) do Genesis MC.

## Instructions
Sempre que o usuário solicitar a criação de uma nova habilidade ou a destilação de um repositório, você DEVE executar os seguintes passos em ordem estrita:

1. **Ingestão Cirúrgica (Proibição de Força Bruta):**
   - Você NUNCA deve ler o repositório inteiro. 
   - Utilize OBRIGATORIAMENTE a skill `@mcp-jcodemunch-master` para extrair a Árvore de Sintaxe Abstrata (AST) do alvo, resgatando apenas as lógicas e heurísticas essenciais.

2. **Destilação da "Alma Matemática":**
   - Elimine qualquer dependência visual, interpretadores em *background* (Python/Node.js) ou arquiteturas pesadas de nuvem (Docker) do código original.
   - Transmute os conceitos extraídos em regras lógicas puras e algoritmos compatíveis com a arquitetura SODA (Rust/Tauri/Wasmtime/SQLite).

3. **Geração da Taxonomia de 3 Níveis (Late-Binding):**
   A nova habilidade DEVE ser estruturada na pasta `.agents/skills/<nome-da-skill>/` utilizando os três níveis de divulgação progressiva:
   - **Nível 1 (Frontmatter YAML):** Crie o cabeçalho no arquivo `SKILL.md` contendo APENAS `name`, `description` (curta e semântica) e `triggers`. Isso garante um carregamento inicial na memória com menos de 100 tokens.
   - **Nível 2 (Instruções Core):** No corpo do `SKILL.md`, estabeleça a Máquina de Estados finita da tarefa (Goal, Instructions, Constraints e Examples). Proíba alucinações.
   - **Nível 3 (Isolamento de Carga Pesada):** Crie as subpastas:
     - `assets/`: Para templates, esquemas JSON ou marcações vazias.
     - `scripts/`: Para executáveis coercitivos (Bash/Python) que o agente tratará como caixas-pretas, não precisando ler o código-fonte na memória ativa.
     - `references/`: Para manuais ou docs de arquitetura pesados.

## Constraints
* **Foco no Metal Nu:** A habilidade gerada deve ser desenhada com o princípio de que o SODA rodará em uma dGPU de 6GB de VRAM e não tolerará daemons em *background*.
* **Economia Extrema de Tokens:** O `SKILL.md` deve ser espartano. Detalhes excessivos devem ser empurrados para a pasta `references/` e consultados pelo agente gerado apenas sob demanda.

## Examples
**Usuário:** "Quero uma skill para gerar diagramas de arquitetura baseada no repo X."
**Ação do Agente:** 
1. Invoca `jcodemunch` para ler a AST do repo X e abstrai a lógica vetorial.
2. Cria `.agents/skills/soda-diagrams/SKILL.md` com YAML de triggers restritos e instruções curtas.
3. Extrai as regras de cores SVG pesadas e salva em `.agents/skills/soda-diagrams/assets/colors.json`, orientando o SKILL.md a consultá-las apenas quando for renderizar.