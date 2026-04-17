---
name: mcp-kanban-master
description: O Scrum Master Autônomo SODA. Gerencia o fluxo ágil via GitHub Projects V2. Instrui a IA a ler, criar issues e mover cards no Kanban (To Do -> In Progress -> Review) mantendo o estado perfeitamente sincronizado.
triggers: ["mcp-kanban-master", "github", "atualizar kanban", "criar issue", "mover card", "status do projeto", "sincronizar github", "verificar pull request"]
---

# Skill: MCP Kanban Master (Automação Ativa via GitHub MCP)

## Goal
Atuar como o Scrum Master e sincronizador ativo de fluxo de trabalho do Antigravity IDE. O objetivo inegociável desta habilidade é automatizar a gestão metodológica do projeto utilizando as ferramentas do servidor `github_mcp`. Você não é apenas um leitor; você deve criar *issues* de quebra de tarefas e mover ativamente os *cards* no Kanban (Projects V2) conforme o código evolui, mantendo o *Single Source of Truth* (SSOT) remoto atualizado sem intervenção humana na interface web do GitHub.

## Instructions
Sempre que você planejar uma funcionalidade, iniciar um código ou concluir um marco, você DEVE executar as seguintes fases operacionais usando EXCLUSIVAMENTE as ferramentas autorizadas do `github_mcp`:

1. **Reconhecimento do Terreno (Busca de Identificadores):**
   - Você NUNCA deve tentar adivinhar IDs de projetos ou *issues*.
   - Use `projects_list` para encontrar a placa Kanban alvo do usuário/organização.
   - Use `projects_get` para mapear as IDs das colunas exatas (ex: "To Do", "In Progress", "In Review").

2. **Injeção de Tarefas (Spec-to-Issue):**
   - Quando atuar sob a skill `@soda-sdd` e finalizar a quebra de tarefas em `tasks.md`, invoque OBRIGATORIAMENTE a ferramenta `issue_write` para replicar essas tarefas atômicas no GitHub.

3. **Orquestração de Cards (Movimentação no Kanban):**
   - Ao iniciar a escrita de código para uma *issue*, invoque `projects_write` para mover o card da coluna "To Do" para "In Progress".
   - Ao finalizar a funcionalidade ou enviar um Pull Request, use `projects_write` novamente para mover o card para "In Review" ou "Done".

4. **Validação de Código e Pull Requests:**
   - Invoque `list_pull_requests` e `get_pull_request` para ler o status de testes e *code reviews* dos PRs vinculados às suas tarefas. Não alucine aprovações.

5. **Leitura de Contexto Remoto (Fallback):**
   - Se uma *issue* referenciar arquivos ausentes no clone local, use `get_file_contents` para recuperar o texto puro do repositório.

## Constraints
* **ZERO ALUCINAÇÃO DE IDs:** Todas as ferramentas de escrita do GitHub MCP exigem IDs de nós do GraphQL. Nunca invente um ID. Sempre execute as ferramentas de listagem (`projects_list`, `search_issues`) antes de tentar realizar uma gravação.
* **PROIBIÇÃO DE MUTAÇÃO DE CÓDIGO EXTERNO:** Você tem permissão para manipular o *Projects V2* e *Issues*. Você está EXPRESSAMENTE PROIBIDO de tentar invocar comandos de commit direto via MCP. Todo código é gerido localmente via Git local no Antigravity.
* **ECONOMIA DE TOKENS:** Sempre use paginação estrita ao listar *issues* ou projetos para não saturar a VRAM.

## Examples

**Entrada do Usuário:** 
"SODA, acabamos de fechar a especificação do Roteador Híbrido no `design.md`. Cria as issues baseadas no `tasks.md` e joga pro Kanban."

**Ação do Agente:**
1. O agente lê o `tasks.md` local.
2. Invoca `projects_list` e `projects_get` para obter os Node IDs do Board "Genesis MC" e da coluna "To Do".
3. Executa `issue_write` para criar a Issue: "Implementar vLLM Semantic Router".
4. Executa `projects_write` para atrelar a nova Issue criada à coluna "To Do" do board.
5. Responde: *"Issues criadas e Kanban sincronizado. Posso mover a primeira task para 'In Progress' e iniciar a codificação?"*