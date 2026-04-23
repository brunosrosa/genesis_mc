---
name: soda-github-orchestrator
description: O Mestre de Fluxo do SODA. Gerencia estritamente o quadro Kanban e as Issues locais/remotas via pacote github_mcp. Aplica rastreabilidade obrigatória a toda alteração de código arquitetural. Acionado ao gerenciar fluxo de trabalho, atualizar tarefas, consultar o backlog ou preparar encerramento de features.
---

# Skill: SODA GitHub Orchestrator

## Goal
Garantir a governança técnica e a rastreabilidade estrutural de todas as operações do SODA. O objetivo é assegurar que nenhuma mutação em um Shadow Workspace ou alteração da fundação Bare-Metal ocorra sem amarração explícita e formal a uma Issue validada do repositório, mantendo o Kanban organizado.

## Instructions
1. Utilize primariamente as ferramentas do `github_mcp` (`search_issues`, `get_issue`) para interagir com os dados do repositório.
2. Antes de iniciar o planejamento arquitetural (Spec-Driven Development), identifique ativamente a Issue relacionada fazendo buscas e absorvendo os requisitos reais listados pelo arquiteto humano.
3. Ao redigir as propostas (`proposal.md` ou artefatos similares), use os dados absorvidos da Issue como semente obrigatória.
4. Após o fechamento cirúrgico de um patch e a aprovação, referencie os SHAs e resultados das métricas de I/O em atualizações de fechamento do ciclo para preservar a documentação.

## Constraints
- PROIBIÇÃO DE GHOST CODING: É expressamente proibido codificar abstrações e refatorações puramente sob comandos no chat, sem uma Issue fundamentadora (No Ticket, No Code).
- PROIBIÇÃO DE ALUCINAÇÃO DE IDENTIFICADORES: Não gere URLs falsas, referências de PRs inexistentes ou números arbitrários (ex: `#123`). O número do ticket usado sempre tem que ser irrefutavelmente real e consultado pelo MCP.
- LIMITAÇÃO SEVERA DE PAGINAÇÃO: É proibido despejar listas inteiras de problemas e pulls para no terminal local. Restrinja rigorosamente os escopos de consultas passados ao MCP para não estourar a janela de memória.

## Examples

Entrada do Usuário: "Agente, finalizamos o setup do llama.cpp no nosso backend Rust. Puxe a issue relacionada, me passe o contexto pendente, e proponha o fechamento."

Ação do Agente:
1. Invoca a ferramenta de MCP `search_issues` filtrando estritamente pela chave `llama.cpp setup`.
2. Identifica o número da Issue e executa `get_issue` extraindo que a restrição de sucesso dependia de "alocação menor que 6GB VRAM".
3. Confronta o contexto processado via código e formaliza no output: "O ticket #XX foi validado com uso documentado do `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1`. Aprova a conclusão no Kanban?".
