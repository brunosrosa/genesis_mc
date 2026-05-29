---
name: mcp-search-master
description: Motor de Deep Research Híbrido do SODA. Orquestra a busca Local-First via 'webcrawl-mcp' governada pelo 'sequentialthinking'. Aplica a Tríade de Proteção (Jitter/Delay) contra banimento de IP. Respeita a política Fail-Open do Gateway (contornando bloqueios via Branching) e finaliza com a Síntese em Prompt Único do IterResearch.
triggers: ["mcp-search-master", "buscar na web", "pesquisar erro", "webcrawl", "ler documentação web", "procurar tutorial", "pesquisar na internet", "deep research"]
---

### skill: MCP Search Master (Motor IterResearch e Resiliência de Rede V7.0)

#### Goal
Atuar como a esteira autônoma de pesquisa profunda (Deep Research) do SODA para erradicar o "Knowledge Cutoff". Você orquestra um Processo de Decisão de Markov (IterResearch) usando o `sequentialthinking` como maestro. O objetivo inegociável é realizar a raspagem de forma Local-First (`webcrawl-mcp`) protegendo o FinOps e a VRAM (descartando HTML bruto). A navegação deve ser blindada contra banimentos de IP através de atrasos sintéticos (Jitter) e operar sob a regra *Fail-Open* do Gateway: se a web bloquear o acesso, o agente contorna o problema, não desiste.

#### Instructions
Sempre que uma pesquisa web, leitura de documentação ou "Deep Research" for solicitada, engate OBRIGATORIAMENTE a seguinte máquina de estados:

1. **O Maestro Analítico:**
   * Inicie com a invocação invisível do `sequentialthinking`. Use *Regular Thoughts* para decompor o problema central em rotas de exploração web.

2. **A Tríade de Proteção (Anti-Banimento) e Delegação Local:**
   * Antes de acionar `webcrawl_search`, `webcrawl_scrape`, `webcrawl_map` ou `webcrawl_crawl`, aplique mentalmente e estruturalmente um limite de cadência: garanta um atraso estocástico (*Jitter*) de cerca de 5 segundos entre as raspagens contínuas do mesmo domínio.
   * Acione o *scrape* para ler a página localmente via biblioteca nativa.

3. **Arquitetura Fail-Open e Reflexão (Resiliência Web):**
   * O mundo web é instável. Se a ferramenta retornar *Error 403/503/Timeout* (bloqueio Cloudflare, etc.), NÃO aborte a rotina do SODA. 
   * A política do Gateway é **Fail-Open**. Codifique a indisponibilidade do site como uma observação normal no seu raciocínio. Ative imediatamente um *Branching Thought* (Pensamento de Ramificação) no MCP para testar uma URL secundária ou cache alternativo.

4. **Extração O(1) e o Laço IterResearch:**
   * É PROIBIDO despejar milhares de tokens de HTML cru ou Markdown bruto na janela de contexto prolongada.
   * Ao obter sucesso na leitura, extraia apenas a "alma matemática" (fatos, código puro, documentação exata) e **descarte** o restante do texto da sua mente.
   * Raciocine: *"Quais lacunas epistemológicas restam?"*. Se faltar algo, repita o ciclo de busca. Se houver redundância, quebre o laço.

5. **Síntese em Prompt Único (Convergência de Evidências):**
   * Ao finalizar o laço, todas as evidências isoladas que você reteve DEVEM passar por uma "Síntese em Prompt Único" interna.
   * Resolva ativamente qualquer sobreposição narrativa ou informações em conflito geradas por sites diferentes.
   * Apenas após essa convergência, encerre o `sequentialthinking` (`nextThoughtNeeded: false`) e entregue a resposta limpa e destilada no Canvas do usuário.

#### Constraints
* **COMPLIANCE DE GATEWAY:** As ferramentas permitidas são ESTRITAMENTE: `webcrawl_search`, `webcrawl_scrape`, `webcrawl_map`, `webcrawl_crawl` e `duckduckgo_search`.
* **SOBREVIVÊNCIA DE REDE:** Se o limite de *rate limit* for atingido, recue e faça ramificação. O SODA nunca paralisa por causa de uma recusa de servidor web.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é a fundação imutável do roteamento SODA.

#### Examples
**Entrada do Usuário:** "SODA, faça um Deep Research sobre como implementar o PagedAttention no vLLM e me traga só as restrições da arquitetura."

**Ação do Agente:**
1. **Maestro:** Emite chamada para `sequentialthinking` com a tarefa de mapear a documentação.
2. **Delegação:** Aciona `webcrawl_search`. Encontra o Github oficial e a documentação.
3. **Fail-Open na Prática:** O agente tenta `webcrawl_scrape` na doc. O site bloqueia a requisição (Error 403). O agente não entra em pânico. Registra a falha, faz um *Branching Thought* e usa `duckduckgo_search` para buscar um post em um blog técnico acessível.
4. **Iteração e Jitter:** Aplica atraso sintético para não ser banido, raspa o blog, extrai os tensores KV e a topologia. Limpa a VRAM do HTML inútil.
5. **Síntese Única:** Cruza os dados do blog com o README do Github. Encerra o pensamento e retorna no Canvas estritamente a especificação solicitada.