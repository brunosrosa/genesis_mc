---
name: mcp-search-master
description: Motor de Deep Research Autônomo Híbrido do SODA. Orquestra a busca local/web via 'webcrawl-mcp', governada pelo 'sequentialthinking' (Maestro). Aplica o paradigma IterResearch (Markov) para sintetizar conhecimento, expurgando o HTML cru. Protege a VRAM e otimiza custos (FinOps) delegando a raspagem 100% local, acionando Firecrawl apenas como Fallback.
triggers: ["mcp-search-master", "buscar na web", "pesquisar erro", "webcrawl", "ler documentação web", "procurar tutorial", "pesquisar na internet", "deep research"]
---

### skill: MCP Search Master (Motor IterResearch e FinOps Webcrawl)

#### Goal
Atuar como a esteira autônoma de pesquisa profunda (Deep Research) do SODA para erradicar o "Knowledge Cutoff". O objetivo inegociável é não tratar a web como um "despejo de contexto", mas orquestrar um Processo de Decisão de Markov (IterResearch). Você DEVE utilizar o `sequentialthinking` para guiar a exploração. Para blindagem FinOps, a extração DEVE ser Local-First via `webcrawl-mcp`, sintetizando a verdade e descartando o HTML bruto para não asfixiar os 6GB de VRAM locais.

#### Instructions
Sempre que uma pesquisa web, documentação ou "Deep Research" for solicitada, engate OBRIGATORIAMENTE a seguinte máquina de estados:

1. **O Maestro Analítico (Fail-Closed):**
   * Toda pesquisa DEVE iniciar com a invocação do servidor MCP `sequentialthinking`.
   * Use *Pensamentos Regulares* para decompor o que você não sabe e mapear os alvos de URL.

2. **Delegação Sensorial Local-First (FinOps):**
   * Você está PROIBIDO de usar APIs pagas de raspagem massiva imediatamente.
   * Invoque as ferramentas `webcrawl_search` ou `webcrawl_scrape`. O servidor tentará ler a página localmente e de graça (via `trafilatura`). Ele só acionará a chave do Firecrawl se o site for JS-heavy.

3. **O Paradigma IterResearch (A Síntese O(1)):**
   * É PROIBIDO despejar o conteúdo bruto ou Markdown de 10.000 tokens do site no Canvas ou no seu prompt final.
   * Ao ler o retorno da ferramenta, aplique a função de síntese: extraia apenas a "alma matemática", o código corrigido ou a resposta exata.
   * Atualize o seu estado de raciocínio no `sequentialthinking` e **esqueça/descarte** ativamente o texto bruto da URL.

4. **O Laço de Reflexão Agêntica (While Loop):**
   * Após a primeira síntese, realize a Reflexão Agêntica no MCP de pensamento: *"Quais lacunas epistemológicas ainda restam para fundamentar o relatório final?"*
   * Se houver lacunas ou necessidade de mapear mais links, invoque `webcrawl_map` ou `webcrawl_crawl` e repita os Passos 2 e 3.
   * **Escudo Anti-Redundância:** Se a nova busca trouxer semântica idêntica à anterior, corte o laço.

5. **A Entrega Final (Síntese Pura):**
   * Quando a evidência estiver completa, encerre o `sequentialthinking`.
   * Devolva ao usuário no Canvas APENAS o documento estruturado final (ou escreva o código no IDE), mantendo o histórico de navegação invisível.

#### Constraints
* **REQUISITO DE GATEWAY (Firewall L7):** O Gateway `gateway-config.yaml` deve ter as rotas `^(webcrawl_.*|sequentialthinking)$` liberadas na válvula CEL. Alerte o usuário para atualizar as configurações do Antigravity caso ocorra "Method Not Found".
* **PROIBIÇÃO DE SCRAPERS NATIVOS:** Nunca use comandos bash/curl na máquina hospedeira. Tudo trafega pelo MCP efêmero.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável.

#### Examples

**Entrada do Usuário:** "SODA, faça um Deep Research sobre como implementar o PagedAttention no vLLM e me dê só o plano." 

**Comportamento Esperado do Agente:**
1.  **Invocação do Maestro:** O agente emite a chamada para o MCP de pensamento: `sequentialthinking.instruct` com a tarefa de "Mapear documentação técnica".
2.  **Delegação para o Webcrawl:** O agente *não executa* o comando de busca diretamente. Ele instrui o MCP de pensamento a usar as ferramentas do `webcrawl-mcp`. Exemplo de Pensamento Interno: *"Vou pedir ao Maestro para buscar o link oficial da documentação do PagedAttention no GitHub do vLLM."*
3.  **Execução Local:** O MCP `webcrawl-mcp` é acionado e executa `webcrawl_search` ou `webcrawl_scrape` localmente. Ele retorna o conteúdo limpo ao Maestro.
4.  **Síntese (O(1)):** O Maestro (pensamento) processa esse conteúdo, extrai os parâmetros de implementação e descarta o HTML. Ele pode então pedir para buscar exemplos de código (mais uma chamada).
5.  **Saída:** O agente retorna ao usuário o plano arquitetural destilado.