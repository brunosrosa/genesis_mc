---
name: mcp-memory-master
description: O Controlador da Tríade de Memória do SODA. Impõe RAG Temporal e métrica FRQAD. Blinda o LanceDB contra falsos negativos via 'Tentativa Dupla' (bypass_vector_index). Impõe Guilhotina de Profundidade (max_depth) em grafos do LadybugDB para proteger a VRAM de OOM. Orquestra a ponte estrutural L3->L2 cruzando IDs de vetores com o SQLite para curar memórias órfãs.
triggers: ["mcp-memory-master", "buscar na memória", "pesquisa vetorial", "consultar lancedb", "consultar ladybug", "rag", "resgatar contexto"]
---

### skill: MCP Memory Master (Controlador de Tríade e RAG Temporal V7.0)

#### Goal
Atuar como o navegador cirúrgico das Memórias L3 (Semântica via LanceDB) e L4 (Grafos via LadybugDB). Seu objetivo inegociável é recuperar conhecimento profundo sem asfixiar a VRAM da RTX 2060m (OOM). Você deve erradicar falsos negativos vetoriais, impor a matemática FRQAD em vez do cosseno falho, aplicar Guilhotinas de Profundidade nas buscas de grafos e sempre reconstruir o contexto cruzando as chaves primárias dos vetores encontrados com o banco relacional (SQLite).

#### Instructions
Sempre que precisar consultar o histórico, regras de arquitetura antigas ou relações causais de código, engatilhe esta Máquina de Estados:

1. **Firewall Compliance (Bypass L7):**
   * As ferramentas permitidas são ESTRITAMENTE: `memory_search`, `memory_graph_query` e `sqz_compress`. Ignore qualquer prefixo do multiplexador.

2. **A Guilhotina de Grafos (LadybugDB - L4):**
   * Se a busca for multi-hop causal (ex: dependências de arquivos): Use `memory_graph_query`.
   * **Lei da Proteção Exponencial:** É PROIBIDO enviar uma busca de grafos sem limite. Você DEVE injetar OBRIGATORIAMENTE os parâmetros `max_depth: 2` (ou no máximo 3) e `limit: 50`. Sem isso, o nó central asfixiará a memória.

3. **A Matemática FRQAD e RAG Temporal (LanceDB - L3):**
   * Se a busca for conceitual: Use `memory_search`.
   * **Fobia de Cosseno:** Injete FORÇOSAMENTE `metric: "FRQAD"`. Nossos vetores mmap quantizados exigem a Distância Fisher-Rao; o cosseno falhará.
   * **Filtro Epoch:** Aplique o `pre_filter` SQL com as lógicas de `valid_from` e `valid_to` em Epoch UTC.

4. **A Tentativa Dupla (Cura do Falso Negativo ANN):**
   * **MANDATÓRIO:** Se a sua query no LanceDB usando filtro temporal retornar vazia (0 resultados), NÃO assuma que o dado não existe. O índice aproximado (ANN) pode ter colapsado devido à restrição do filtro.
   * Execute uma **segunda busca imediata** com a mesmíssima query, mas adicione o parâmetro `bypass_vector_index: true`. Isso forçará a engine em Rust a ler todas as linhas habilitadas via força bruta (kNN Exato).

5. **Reconstrução de Memória Órfã (Ponte L3 -> L2):**
   * Os dados do LanceDB são vetores puros e fragmentados (`chunk_id`, `text`).
   * Para descobrir a qual arquivo físico ou PR esse fragmento pertence, pegue o `chunk_id` retornado e delegue ao `@mcp-sqlite-master` a busca dos metadados completos na tabela principal. Nunca alucine contextos ausentes.

#### Constraints
* **COMPRESSÃO DE SAÍDA:** Se os nós de retorno forem massivos, obrigue a passagem do texto pela ferramenta `sqz_compress` antes de internalizá-lo no seu pensamento.
* **PROIBIÇÃO DE SQL VETORIAL COMPLEXO:** O LanceDB não faz JOINs. Filtre apenas por `temporal_stability`, `valid_from` e `valid_to`.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo é a âncora inegociável do roteamento L7.

#### Examples
**Entrada do Usuário:** "SODA, busca na memória qual era a diretriz de UX sobre animações do Svelte 5."
**Ação do Agente:**
1. Roteia para L3 (Semântico). Invoca `memory_search(query: "UX Svelte 5", metric: "FRQAD", pre_filter: "temporal_stability = 'STABLE'")`.
2. A busca retorna 0 resultados devido ao colapso do índice ANN.
3. Agente ativa a Tentativa Dupla OBRIGATÓRIA: refaz a busca com `bypass_vector_index: true`.
4. O kNN exato encontra a diretriz de 50ms para "Ambient Status". O agente pega o `chunk_id`.
5. Busca silenciosa no `@mcp-sqlite-master` revela que a regra veio do documento *Epic 17*.
6. Devolve no Canvas a resposta rica e exata: *"-> Memória L3 resgatada via kNN exato. Cruzamento L2 confirma: Regra do Epic 17 de UX aplicada."*