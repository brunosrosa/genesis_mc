---
name: mcp-time-master
description: O Relógio Absoluto do SODA. Resolve o Paradoxo do Multiplexador (Firewall L7). Impõe a Bifurcação Temporal (ISO-8601 para ETL/Sheets, Epoch Int64 para LanceDB/SQLite B-Tree). Aplica a Estratégia de Recuo (NULL) para prevenir colapso vetorial e força Decodificação Restrita (llguidance) em SLM fallbacks.
triggers: ["mcp-time-master", "ver hora", "que dia é hoje", "data atual", "ancoragem temporal", "agendar", "timestamp", "fuso horário"]
---

### skill: MCP Time Master (A Âncora Cronológica e RAG Temporal V7.0)

#### Goal
Atuar como a âncora matemática de realidade temporal do SODA. O objetivo inegociável é erradicar alucinações de fuso horário, garantir ordenação causal e blindar os bancos de dados. Para evitar bloqueios do Gateway, você deve usar táticas de *Bypass* na nomenclatura L7. Na modelagem de dados, você DEVE dominar a Bifurcação de Formatos (ISO-8601 vs Int64) e aplicar a "Estratégia de Recuo para NULL" para não asfixiar os pré-filtros do LanceDB com datas alucinadas.

#### Instructions
Sempre que uma tarefa exigir marcação de tempo, registro de logs, inserções em banco ou ETL para o Google Sheets, utilize este MCP sob a seguinte máquina de estados:

1. **Invocação e Bypass L7 (Firewall):**
   * Invoque a ferramenta com o NOME EXATO `get_current_time`. 
   * **Fail-Closed:** Se o sistema listar a ferramenta como `time_server_get_current_time`, ignore o prefixo. A Válvula CEL do Gateway aceita apenas `get_current_time`.

2. **A Bifurcação de Destino (ISO-8601 vs Epoch Int64):**
   * **Para o Google Sheets / ETL (ex: coluna `data_ultima_analise`):** Formate a data EXCLUSIVAMENTE em **ISO-8601 UTC** (ex: `2026-05-07T03:08:00Z`).
   * **Para SQLite / LanceDB (ex: `valid_from`, `valid_to`, `timestamp`):** Exporte ESTRITAMENTE em **UNIX Epoch Int64 UTC**. O motor B-Tree falhará se receber strings.
   * **Batch Insert:** Para inserções atômicas no mesmo milissegundo, aplique Micro-incrementos (+1, +2) no Epoch de cada linha.

3. **Estratégia de Recuo para o LanceDB (Anti-Colapso):**
   * Ao classificar a caducidade temporal (`valid_to`) de uma memória `EVOLVING`, seja pragmático.
   * **Emita `NULL`:** Na ausência de um limite temporal explícito ou facilmente dedutível, preencha com `NULL`. Tentar "inventar" uma data limite criará um filtro hiper-seletivo falso que cegará a Busca Vetorial Híbrida.

4. **Guilhotina do SLM (Decodificação Restrita via llguidance):**
   * Se você (ou o motor em Rust) precisar delegar o entendimento de datas relativas (ex: "até o fim do semestre") para um Micro-SLM (ex: Phi-4-mini), é PROIBIDA a geração de texto livre.
   * Force o formato JSON Schema usando as tags de *Function Calling* (`<|tool|>`). A saída DEVE ser estritamente `{ start: string, end: string }` ou `NULL` para economizar VRAM e latência no `llguidance`.

5. **Contextual Chunks (Para RAG):**
   * Ao criar fragmentos persistentes em Markdown, injete fisicamente no topo do texto: `Carimbo Temporal: [Data ISO-8601]` para fortalecer a Busca Híbrida BM25.

#### Constraints
* **PROIBIÇÃO DE SHELL SCRIPTS:** É letal usar comandos bash como `date +%s`. O SODA opera bare-metal em Windows/Linux e requer portabilidade absoluta via servidor MCP.
* **SILÊNCIO OPERACIONAL:** O relógio trabalha nos bastidores. Não perca tokens explicando o horário no Canvas.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação do roteamento O(1).

#### Examples
**Entrada do Usuário:** "SODA, grava no Blueprint a análise de hoje, e injeta isso como memória volátil pro mês."
**Ação do Agente:**
1. Invoca estritamente `get_current_time` (ignorando o prefixo bloqueado pelo CEL).
2. Bifurcação: Para a planilha (ETL), gera a string `2026-05-07T03:10:00Z`. Para o LanceDB, gera a chave Epoch de base `1715051400`.
3. Determina o `valid_to`. Como o usuário disse "pro mês", calcula +2592000 segundos.
4. Conclui em silêncio.