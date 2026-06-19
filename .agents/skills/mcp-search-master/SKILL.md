---
name: mcp-search-master
description: Motor de Deep Research Híbrido do SODA. Orquestra a busca Local-First via `soda_fetch_web` governada pelo `sequentialthinking`. Aplica a Tríade de Proteção (Jitter/Delay) contra banimento de IP e finaliza com Síntese em Prompt Único do IterResearch.
triggers: ["mcp-search-master", "buscar na web", "pesquisar erro", "ler documentação web", "procurar tutorial", "pesquisar na internet", "deep research", "soda_fetch_web"]
---

### skill: MCP Search Master (Motor IterResearch e Resiliência de Rede V7.0)

#### Goal
Atuar como a esteira autônoma de pesquisa profunda (Deep Research) do SODA para erradicar o "Knowledge Cutoff". Você orquestra um Processo de Decisão de Markov (IterResearch) usando o `sequentialthinking` como maestro. O objetivo inegociável é realizar a raspagem de forma Local-First via `soda_fetch_web`, protegendo o FinOps e a VRAM (descartando HTML bruto). A navegação deve ser blindada contra banimentos de IP através de atrasos sintéticos (Jitter) e operar sob a regra *Fail-Open* do Gateway: se a web bloquear o acesso, o agente contorna o problema, não desiste.

#### Instructions
Sempre que uma pesquisa web, leitura de documentação ou "Deep Research" for solicitada, engate OBRIGATORIAMENTE a seguinte máquina de estados:

1. **O Maestro Analítico:**
   * Inicie com a invocação invisível do `sequentialthinking`. Use *Regular Thoughts* para decompor o problema central em rotas de exploração web.

2. **A Tríade de Proteção (Anti-Banimento) e Delegação Local:**
   * Antes de acionar `soda_fetch_web` repetidamente no mesmo domínio, aplique mentalmente e estruturalmente um limite de cadência: garanta um atraso estocástico (*Jitter*) entre as raspagens contínuas.
   * Acione `soda_fetch_web` para ler a página localmente via biblioteca nativa do Gateway Rust.

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
* **COMPLIANCE DE GATEWAY:** As ferramentas prioritárias são `soda_fetch_web` e `duckduckgo_search`. Só use busca web auxiliar para descobrir URLs; a extração deve ficar no poder intrínseco bare-metal.
* **SOBREVIVÊNCIA DE REDE:** Se o limite de *rate limit* for atingido, recue e faça ramificação. O SODA nunca paralisa por causa de uma recusa de servidor web.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é a fundação imutável do roteamento SODA.

#### Examples
**Entrada do Usuário:** "SODA, faça um Deep Research sobre como implementar o PagedAttention no vLLM e me traga só as restrições da arquitetura."

**Ação do Agente:**
1. **Maestro:** Emite chamada para `sequentialthinking` com a tarefa de mapear a documentação.
2. **Delegação:** Usa busca auxiliar para encontrar a URL oficial e então aciona `soda_fetch_web`.
3. **Fail-Open na Prática:** O agente tenta `soda_fetch_web` na doc. O site bloqueia a requisição. O agente não entra em pânico. Registra a falha, faz um *Branching Thought* e usa busca auxiliar para encontrar uma URL secundária acessível.
4. **Iteração e Jitter:** Aplica atraso sintético para não ser banido, raspa o blog, extrai os tensores KV e a topologia. Limpa a VRAM do HTML inútil.
5. **Síntese Única:** Cruza os dados do blog com o README do Github. Encerra o pensamento e retorna no Canvas estritamente a especificação solicitada.
