# Tasks: MARCO VI — O Hipocampo Ativo, Reator Híbrido RRF e Metabolismo de Langevin

## Task List

- [ ] **Task 1: LanceDB Zero-VRAM Coupling & Pre-Filtering**
  - [ ] Implementar schema canônico (`id`, `text_content`, `embedding` [384], `temporal_stability`, `valid_from`, `valid_to`) em `src-tauri/src/cognition/memory/vector_retriever.rs`.
  - [ ] Implementar pré-filtragem escalar com `only_if` temporal e tratamento de falsos negativos (fallback `bypass_vector_index` para SQLite FTS5).
  - [ ] *DoD:* Teste `test_lancedb_mmap_zero_vram_isolation` aprovado no cargo test.

- [ ] **Task 2: Reator Híbrido RRF com AVX2 & Bônus de Termo Exato**
  - [ ] Implementar cálculo de Reciprocal Rank Fusion na CPU com AVX2 e constante $k=60$ em `src-tauri/src/cognition/memory/rrf_fusion.rs`.
  - [ ] Adicionar algoritmo de detecção de termos exatos (constantes rígidas, IDs, arquivos) para prioridade máxima no topo do ranking.
  - [ ] Integrar no MCP Server (`souls_mcp.semantic_search` / `souls_semantic_search`).
  - [ ] *DoD:* Teste `test_hybrid_search_rrf_avx2_fusion` aprovado com tempo de unificação < 5ms.

- [ ] **Task 3: Firewall Ontológico LadybugDB (Anti-RAG Poisoning)**
  - [ ] Implementar `src-tauri/src/cognition/memory/ladybug_firewall.rs` com `DashMap` em RAM para grafo ontológico.
  - [ ] Implementar travessia BFS rápida (`depends_on`, `replaces`, `implements`, `violates`) e interceptor de chunks do RRF.
  - [ ] *DoD:* Teste `test_ladybug_graph_bfs_poison_prevention` aprovado, expurgando violações com advertência na telemetria.

- [ ] **Task 4: Metabolismo de Langevin e Desfragmentação Não-Destrutiva**
  - [ ] Implementar decaimento estocástico de Langevin com invariância estrita ($\lambda = 0$) para `STABLE` e $\lambda = 0.05$ para `EVOLVING` em `src-tauri/src/cognition/memory/langevin_decay.rs` e `chyros_daemon.rs`.
  - [ ] Implementar consolidação periódica idempotente via `VACUUM INTO` no FrankenSQLite.
  - [ ] *DoD:* Teste `test_chyros_langevin_decay_vacuum_into` aprovado com 3 ciclos e execução limpa do VACUUM INTO.

- [ ] **Task 5: Validação Global e Quality Gate**
  - [ ] Executar todos os testes do workspace (`cargo test`).
  - [ ] Executar verificação estrita sem warnings (`cargo check --workspace --all-features`).
  - [ ] *DoD:* Exit code 0 absoluto sem quebras de contrato ou avisos.
