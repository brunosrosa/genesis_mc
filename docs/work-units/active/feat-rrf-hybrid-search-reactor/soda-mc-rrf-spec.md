# ESPECIFICAÇÃO ARQUITETURAL: MARCO 5.6.0 — REATOR DE BUSCA HÍBRIDA RRF E INVALIDAÇÃO JIT (DoD GREEN)

## 🏛️ 1. O Racional de Design (A Busca Soberana e Híbrida)
RAGs tradicionais baseados exclusivamente em similaridade de cosseno vetorial sofrem de miopia de correspondência exata. Se o usuário busca por uma variável específica ou hash de commit, o embedding vetorial pode trazer conceitos próximos, mas falhar em trazer a linha exata. 

O **Marco 5.6.0** resolve este ponto cego combinando a precisão exata do índice **FTS5 (BM25)** do SQLite à semântica densa do **LanceDB (L3)** de longo prazo. A unificação ocorre em Rust puro no core por meio do algoritmo **RRF (Reciprocal Rank Fusion)**, operando em tempo $\mathcal{O}(N \log N)$ com consumo de VRAM de exatos **0 MB** na RTX 2060m.

---

## 📐 2. Formulação Matemática e Alinhamento Atômico

### A. A Equação do RRF (Reciprocal Rank Fusion)
A fusão das duas listas concorrentes de resultados (Léxico vs Vetorial) não utiliza avaliações lentas de IA. Ela é calculada de forma determinística pela fórmula:

$$RRF(d) = \sum_{m \in M} \frac{1}{k + r_m(d)}$$

Onde:
*   $k = 60.0$ (constante de suavização padrão).
*   $r_m(d)$ é o ranking (1-indexed) do documento $d$ no sistema de busca $m$ (Lexical ou Vectorial).
*   Se o documento não estiver presente em uma das listas, seu ranking é considerado infinito, somando $0.0$ ao escore final.

### B. O Vínculo Crítico: A Chave-Mestra UUIDv7
Para que a unificação seja possível em tempo $\mathcal{O}(N)$ usando um `HashMap`, os IDs retornados pelo `FtsRetriever` e pelo `VectorRetriever` devem ser idênticos. O SODA utiliza obrigatoriamente a string de 36 caracteres do **UUIDv7** (`soda_universal_uuid` / `observation_id`) como chave primária de correspondência.

---

## 💾 3. Detalhamento dos Módulos Rust

### A. fts_retriever.rs
*   **Papel:** Consultar o índice de texto completo do FrankenSQLite (`souls_state.db`).
*   **Query SQL BM25:**
    ```sql
    SELECT observation_id, content, rank 
    FROM observations_fts 
    WHERE observations_fts MATCH ? 
    ORDER BY rank 
    LIMIT ?;
    ```
*   **Complexidade:** $\mathcal{O}(\log N)$ via busca em árvore B-Tree.

### B. vector_retriever.rs
*   **Papel:** Realizar a busca vetorial de longo prazo (L3) no LanceDB local.
*   **Agnosticismo VRAM:** Abre os arquivos do LanceDB mapeados em memória virtual via **`mmap`** no SSD NVMe. Consumo de VRAM adicional: **0 MB**.
*   **Entrada:** Embeddings densos de 384 floats gerados na CPU via `generate_cpu_embedding_384` (ou ONNX model).

### C. rrf_fusion.rs (A Invalidação JIT / Tombstone)
*   **Papel:** Fusão matemática dos rankings e expulsão JIT de contradições lógicas (Anti-RAG Poisoning).
*   **A Invalidação JIT:** Antes de emitir o ranking consolidado, o motor faz um lookup único no SQLite:
    ```sql
    SELECT observation_id FROM observations WHERE status IN ('superseded', 'invalid');
    ```
*   Esses IDs de observações invalidadas ou superadas são carregados em um `HashSet<String>` em RAM. Durante o loop RRF, qualquer documento presente neste `HashSet` é **sumariamente expurgado em tempo constante $\mathcal{O}(1)$**, impedindo que a IA receba premissas obsoletas.

---

## 🚦 4. Alinhamento com as Leis de Ferro e ADR-041

1.  **Limites de Caracteres da Tool MCP (ADR-041):**
    *   **Nome:** `souls_semantic_search` (21 caracteres $\le 32$ chars).
    *   **Descrição:** "Executa a busca híbrida RRF combinando FTS5 (BM25) e LanceDB vetorial local com invalidação JIT." (98 caracteres $\le 120$ chars).
2.  **Higiene Estrita de Stdio:**
    *   Nenhum comando em `fts_retriever`, `vector_retriever` ou `rrf_fusion` pode utilizar `println!`. Toda e qualquer telemetria de erro ou aviso operacional deve ser direcionada ao fluxo de erro padrão (`eprintln!`) para manter o `stdout` 100% puro para mensagens JSON-RPC.

---

## 🧪 5. Caderno de Testes TDD (Verificação de Sucesso)

O fechamento em **DoD GREEN** exige a aprovação de 5 testes de integração contidos em `souls_mcp_server.rs`:
1.  `test_fts5_lexical_retrieval`: Valida que a consulta MATCH do SQLite recupera strings exatas em menos de 1ms.
2.  `test_lancedb_mmap_vram_safety`: Assevere matematicamente e via logs que a conexão do LanceDB utiliza mmap e consome zero de VRAM.
3.  `test_rrf_mathematical_fusion`: Insere rankings parciais mockados e assevere que a ordenação e os escores finais seguem rigorosamente a equação RRF.
4.  `test_jit_tombstone_invalidation`: Simula uma premissa marcada como `superseded` no SQLite e assevere que ela foi expurgada do resultado híbrido final.
5.  `tools_list_respects_32_120_tetos`: Garante por meio de asserção dinâmica que a nova toolMCP se enquadra nos limites da ADR-041.
