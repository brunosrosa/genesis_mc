# Tasks: MARCO VI — O Hipocampo Ativo, Reator Híbrido RRF e Metabolismo de Langevin

## Task List

- [x] **Task 1: LanceDB Zero-VRAM Coupling & Pre-Filtering**
  - [x] Implementar schema canônico (`id`, `text_content`, `embedding` [384], `temporal_stability`, `valid_from`, `valid_to`) em `src-tauri/src/cognition/memory/vector_retriever.rs`.
  - [x] Implementar pré-filtragem escalar com `only_if` temporal e tratamento de falsos negativos (fallback `bypass_vector_index` para SQLite FTS5).
  - [x] *DoD:* Teste `test_lancedb_mmap_zero_vram_isolation` aprovado no cargo test.

- [x] **Task 2: Reator Híbrido RRF com AVX2 & Bônus de Termo Exato**
  - [x] Implementar cálculo de Reciprocal Rank Fusion na CPU com AVX2 e constante $k=60$ em `src-tauri/src/cognition/memory/rrf_fusion.rs`.
  - [x] Adicionar algoritmo de detecção de termos exatos (constantes rígidas, IDs, arquivos) para prioridade máxima no topo do ranking.
  - [x] Integrar no MCP Server (`souls_mcp.semantic_search` / `souls_semantic_search`).
  - [x] *DoD:* Teste `test_hybrid_search_rrf_avx2_fusion` aprovado com tempo de unificação < 5ms.

- [x] **Task 3: Firewall Ontológico LadybugDB (Anti-RAG Poisoning)**
  - [x] Implementar `src-tauri/src/cognition/memory/ladybug_firewall.rs` com `DashMap` em RAM para grafo ontológico.
  - [x] Implementar travessia BFS rápida (`depends_on`, `replaces`, `implements`, `violates`) e interceptor de chunks do RRF.
  - [x] *DoD:* Teste `test_ladybug_graph_bfs_poison_prevention` aprovado, expurgando violações com advertência na telemetria.

- [x] **Task 4: Metabolismo de Langevin e Desfragmentação Não-Destrutiva**
  - [x] Implementar decaimento estocástico de Langevin com invariância estrita ($\lambda = 0$) para `STABLE` e $\lambda = 0.05$ para `EVOLVING` em `src-tauri/src/cognition/memory/langevin_decay.rs` e `chyros_daemon.rs`.
  - [x] Implementar consolidação periódica idempotente via `VACUUM INTO` no FrankenSQLite com exponential backoff.
  - [x] *DoD:* Teste `test_chyros_langevin_decay_vacuum_into` aprovado com 3 ciclos e execução limpa do VACUUM INTO.

- [x] **Task 5: Validação Global e Quality Gate**
  - [x] Executar todos os testes do workspace (`cargo test`).
  - [x] Executar verificação estrita sem warnings (`cargo check --workspace --all-features`).
  - [x] Compilar binário release standalone com Svelte 5 embarcado e copiar para `.agents/bin/souls_mc.exe`.
  - [x] *DoD:* Exit code 0 absoluto sem quebras de contrato ou avisos.
