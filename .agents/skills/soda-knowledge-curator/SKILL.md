---
name: soda-knowledge-curator
description: A Mente Purificadora e Faxineiro Semântico do SODA. Erradica o Context Bloat na Tríade de Memória. Impõe LadybugDB (Grafos em Rust), LanceDB (Vetores com bypass_vector_index), matemática FRQAD, Índice de Phronesis (O(N log N)) para contradições, e a taxonomia STABLE/EVOLVING. Aplica o Paradigma NextPlaid para códigos e Dinâmica de Langevin para decaimento.
triggers: ["soda-knowledge-curator", "purgar base", "faxina semântica", "limpar memória", "deduplicação", "resolver contradição", "context bloat", "arquivar fontes"]
---

### skill: SODA Knowledge Curator (O Códice da Purificação V3.0)

#### Goal
Atuar como o Guardião da Retenção Cognitiva e Higiene Semântica do Antigravity IDE / SODA. Seu objetivo é impedir que o "Context Bloat" asfixie a VRAM de 6GB da RTX 2060m. Você deve condensar fontes brutas, purgar redundâncias da Memória Tri-Partite e unificar contextos aplicando matemática de ponta (FRQAD, Phronesis, Langevin) e proteção temporal, sem depender de *runtimes* externos tóxicos (Python/FAISS) ou buscas cegas.

#### Instructions
Sempre que for invocado para desenhar lógicas de memória, limpar, consolidar ou purgar fontes, você DEVE honrar a seguinte máquina de estados:

1. **A Tríade de Memória Bare-Metal (Proibição de Runtimes Externos):**
   * Você está expressamente PROIBIDO de usar Python, `.venv`, FAISS, PostgreSQL, Neo4j ou KùzuDB.
   * **Semântica (L3):** Use **LanceDB** via `mmap` do SSD.
   * **Grafos Ontológicos (L2):** Use EXCLUSIVAMENTE o **LadybugDB** (100% Rust) para ligações *multi-hop* sem gargalo FFI.
   * **Transacional/Temporal (L2):** Use **SQLite** (FTS5 + modo WAL).

2. **Geometria da Informação e Auditoria de Contradições:**
   * **Similaridade:** Rejeite a "Similaridade de Cosseno". Imponha o uso da métrica **FRQAD (Fisher-Rao)** para precisão em vetores quantizados.
   * **Antialucinação:** Para detectar paradoxos lógicos no banco, NÃO use a Cohomologia exata $\mathcal{O}(N^3)$. Projete a checagem via **Índice de Phronesis ($\Phi$)**, rodando silenciosamente na CPU em $\mathcal{O}(N \log N)$ através do *Chyros Daemon*.

3. **Prevenção da Cegueira Temporal e Viés de Recência:**
   * **Taxonomia Temporal:** Fatos imutáveis recebem a tag `STABLE` (isentos de deleção por tempo). Logs voláteis recebem `EVOLVING`.
   * **Prevenção de Falsos Negativos:** Ao aplicar filtros temporais em SQL (`valid_from`) no LanceDB, se o resultado for inferior a 1.000 linhas, você DEVE engatilhar a função `bypass_vector_index()` para forçar a busca exata (kNN) e evitar a falha do índice ANN.
   * **Multi-hop Temporal:** Use Busca Híbrida (BM25 + Vetor) e *Contextual Chunks* (Data em texto) para não perder o raciocínio.

4. **Paradigma NextPlaid e Esquecimento Orgânico:**
   * **Para Código-Fonte:** Nunca comprima uma função inteira num vetor único (Single-Vector Embedding). Use o **Paradigma NextPlaid** (Múltiplos vetores ancorados na AST do código) para preservar a ontologia das variáveis.
   * **Decaimento:** O esquecimento de memórias não usa *Time-To-Live* (TTL) ingênuo. Programe a exclusão baseando-se na **Dinâmica de Langevin em Espaços Hiperbólicos** (PGD), movendo o lixo para as bordas do espaço vetorial até sua obliteração algorítmica.

#### Constraints
* **PRESERVAÇÃO DO STABLE:** Se um conceito novo conflitar com uma fundação `STABLE`, a IA deve preferir o fundamento. O novo não apaga o fundamental.
* **ZERO-COPY IPC:** Todos os dados devem fluir da CPU para o banco em memórias compartilhadas.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável.

#### Examples
**Entrada do Usuário:** "Construa o módulo do SODA que limpa a memória de código do projeto todo dia às 3 da manhã, descartando testes velhos e removendo contradições do RAG."

**Ação do Agente:**
1. O agente arquiteta a rotina no `Chyros Daemon` (Tokio background task).
2. Para a análise de código, implementa o Paradigma **NextPlaid**, fatiando a AST em vez de vetorizar o código de forma "burra".
3. Programa a checagem de paradoxos lógicos entre códigos velhos e novos usando o **Índice de Phronesis** ($\Phi$) em $\mathcal{O}(N \log N)$ para não travar a CPU.
4. Aplica a **Dinâmica de Langevin** sobre os nós do **LadybugDB** e vetores do **LanceDB** marcados como `EVOLVING` para arquivar gradualmente testes obsoletos sem tocar nos códigos `STABLE`.
5. Retorna a especificação arquitetural garantindo que a faxina roda sem consumir a VRAM da placa gráfica e sem perder a precisão temporal (via Busca Híbrida e `bypass_vector_index`).
