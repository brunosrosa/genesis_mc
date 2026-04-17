---
name: notebooklm-context
description: O Oráculo de Arquitetura. Use EXCLUSIVAMENTE para consultar as pesadas documentações do projeto no Google NotebookLM via MCP, poupando a VRAM local.
triggers: ["notebooklm-context", "consultar arquitetura", "ler documentação pesada", "notebooklm", "oráculo", "pesquisar projeto", "regras do soda"]
---

# Skill: NotebookLM Context (O Oráculo e Gatekeeper)

## Goal
Atuar como a memória profunda do sistema (L3) durante o desenvolvimento no Antigravity IDE, preservando a janela de contexto local. O servidor MCP do NotebookLM expõe 35 ferramentas diferentes que podem causar "Tool Bloat" e colapso de memória. O objetivo desta skill é atuar como um **Gatekeeper**, proibindo o uso de ferramentas de geração de mídia e forçando o agente a usar estritamente os endpoints de extração de conhecimento aterrado (Grounded RAG) para consultar a arquitetura do projeto.

## Instructions
Sempre que o usuário fizer uma pergunta sobre a arquitetura do sistema, restrições do hardware, regras do projeto, ou solicitar a leitura dos documentos de base, você DEVE buscar essas informações no NotebookLM seguindo este protocolo estrito:

1. **O Filtro de Ferramentas (Whitelist):** Você está PROIBIDO de utilizar as ferramentas de geração de mídia do MCP (como `studio_create`, `audio_podcast`, `video_overview`). Você deve ignorá-las completamente.
2. **Identificação do Alvo:** Use a ferramenta `notebook_list` apenas se não souber o ID do caderno atual do projeto.
3. **Consulta Cirúrgica (Zero-Shot RAG):** Para obter respostas, utilize SEMPRE a ferramenta `notebook_query`. Formule uma pergunta detalhada e específica. O NotebookLM processará os documentos externamente e devolverá a você apenas a resposta final com as citações.
4. **Extração de Conteúdo Bruto (Fallback):** Se você precisar ler a íntegra de um arquivo de código ou texto que está no NotebookLM, e não apenas um resumo, utilize exclusivamente a ferramenta `source_get_content` passando o ID do documento.
5. **Incorporação Passiva:** Ao receber a resposta do MCP, absorva o conhecimento semântico e proceda com a tarefa de codificação ou planejamento no Antigravity IDE, sem injetar os logs inteiros da pesquisa no seu output para o usuário.

## Constraints
* **NUNCA INVENTE ARQUITETURA:** Se você não sabe como o sistema deve ser implementado, não alucine. Pare, acione a ferramenta `notebook_query` e pergunte ao oráculo.
* **ECONOMIA DE CONTEXTO:** Nunca tente extrair todos os documentos de uma vez. Faça perguntas direcionadas à intenção atual da tarefa.

## Examples

**Entrada do Usuário:** 
"Como deve ser a estrutura do Ralph Loop no SODA? Pesquise na nossa documentação."

**Ação Incorreta (NÃO FAÇA):**
O agente tenta ler arquivos locais gigantes usando `cat` ou tenta usar ferramentas MCP desconhecidas do NotebookLM para gerar relatórios completos, estourando o limite de contexto.

**Ação Correta (Obrigatória):**
1. O agente invoca a ferramenta MCP `notebook_query` com o parâmetro: "Explique em detalhes a arquitetura e as regras do Ralph Loop no sistema SODA".
2. O MCP retorna o resumo exato embasado nos 28 documentos originais.
3. O agente lê a resposta e começa a programar a lógica no Antigravity de acordo com a verdade extraída.