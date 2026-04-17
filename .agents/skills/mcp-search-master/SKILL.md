---
name: mcp-search-master
description: Navegador invisível e furtivo do SODA. Ensina a IA a buscar na web via DuckDuckGo MCP e extrair conteúdo limpo de URLs para contornar CAPTCHAs e alucinações, paginando resultados para proteger a VRAM.
triggers: ["mcp-search-master", "buscar na web", "pesquisar erro", "duckduckgo", "ler documentação web", "procurar tutorial", "fetch url", "pesquisar na internet"]
---

# Skill: MCP Search Master (Busca Furtiva e RAG Dinâmico)

## Goal
Atuar como a ponte de conhecimento em tempo real do sistema quando a memória L2 (SQLite) ou os documentos locais forem insuficientes. Modelos de linguagem sofrem de "corte temporal" (Knowledge Cutoff) e alucinam sobre pacotes atualizados ou erros recentes de compilador. O objetivo inegociável desta habilidade é forçar o agente a buscar a verdade na internet utilizando estritamente as ferramentas gratuitas do `duckduckgo-mcp-server`, contornando bloqueios anti-bot e extraindo o texto limpo das URLs sem estourar o limite de VRAM da máquina host com lixo HTML [1, 3].

## Instructions
Sempre que você não souber a solução exata para um erro, ou precisar consultar a documentação mais recente de uma biblioteca (ex: uma nova API do Tauri ou Rust), você DEVE abortar a adivinhação e seguir este fluxo de pesquisa estruturada:

1. **Interdição de Comandos Nativos:** Você está expressamente PROIBIDO de usar `curl`, `wget` ou scripts Python para baixar páginas web. Eles falharão por causa de CAPTCHAs e sujarão o seu contexto [1].
2. **Reconhecimento (Search):** Invoque a ferramenta `duckduckgo_search` (ou `web_search` equivalente exposta no Gateway) passando a sua string de consulta detalhada. Avalie os títulos e URLs retornados na primeira página de resultados.
3. **Extração de Texto Limpo (Fetch):** Ao escolher a URL mais promissora para ler a fundo, invoque OBRIGATORIAMENTE a ferramenta `duckduckgo_search_fetch_content` (ou `fetch_content`) [3, 4]. Esta ferramenta remove automaticamente a navegação, cabeçalhos, rodapés e scripts do site, devolvendo apenas o texto puro e legível [3].
4. **Proteção de VRAM via Paginação:** A ferramenta de extração possui suporte a paginação (`start_index` e `max_length`). O padrão de retorno é de 8000 caracteres [3]. NUNCA peça blocos maiores que isso de uma só vez para não colapsar sua janela de contexto local. Se o texto for cortado, faça uma segunda chamada ajustando o `start_index` [3].

## Constraints
* **ZERO ALUCINAÇÃO (GROUNDING):** Baseie sua solução estritamente no texto extraído da web. Se a pesquisa do DuckDuckGo não retornar nada útil, reformule a busca. Não invente parâmetros de código.
* **ECONOMIA DE CONSULTAS:** Não leia 5 URLs ao mesmo tempo. Extraia uma, leia o conteúdo, avalie se resolveu o problema e, apenas se necessário, extraia a próxima.

## Examples

**Entrada do Usuário:** 
"O compilador do Rust está dando um erro obscuro no `wgpu` na versão 0.19. Pesquisa na web o que mudou na API."

**Ação do Agente:**
1. O agente NÃO inventa o código.
2. Ele invoca a ferramenta de busca: `duckduckgo_search(query: "Rust wgpu 0.19 breaking changes error")`.
3. O servidor retorna 5 links. O agente identifica uma Issue no GitHub promissora.
4. O agente invoca a extração: `duckduckgo_search_fetch_content(url: "https://github.com/gfx-rs/wgpu/issues/...", max_length: 8000)` [3].
5. O agente lê o texto limpo retornado, entende a nova assinatura da API e escreve o código corrigido no Antigravity IDE, respondendo ao usuário com a solução embasada.