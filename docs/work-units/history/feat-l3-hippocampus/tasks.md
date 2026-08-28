# Tasks: MARCO VI — Operação Âncora do Hipocampo (feat-l3-hippocampus)

## Metas e Definition of Done (DoD)

- [x] **Task 1: Governança Territorial & SDD**
  - [x] Documentar `design.md` e `tasks.md` em `docs/work-units/active/feat-l3-hippocampus/`.
  - [x] Redirecionar logs de clippy para `.souls_scratchpad/logs/cargo/clippy_l3_hippocampus.log`.

- [x] **Task 2: Acoplamento Zero-VRAM do LanceDB (mmap)**
  - [x] Implementar `src-tauri/src/core/semantic_search.rs` com inicialização canônica em `Z:\souls_mc\.souls_data\semantic_memories`.
  - [x] Garantir 0 MB de VRAM na RTX 2060m via mapeamento `mmap` Arrow em Host RAM.
  - [x] Implementar barreira condicional para filtros escalares restritivos (< 1000 linhas) ativando `bypass_vector_index()`.

- [x] **Task 3: Reator de Fusão Híbrida RRF (CPU AVX2)**
  - [x] Implementar RRF com $k=60$ e aceleração AVX2 nativa para fusão de FTS5 + LanceDB em latência sub-5ms.
  - [x] Conectar saída do reator à ferramenta `souls_semantic_search` no MCP Server (`handlers/system.rs`).

- [x] **Task 4: Firewall Ontológico LadybugDB (Anti-RAG Poisoning)**
  - [x] Instanciar LadybugDB via `DashMap` em RAM Host espelhado no SQLite.
  - [x] Executar varredura BFS nos chunks recuperados e banir sumariamente itens em contradição com ADRs `STABLE`.

- [x] **Task 5: Suíte de Testes TDD (Marco VI)**
  - [x] Implementar `test_lancedb_mmap_zero_vram_isolation`.
  - [x] Implementar `test_hybrid_search_rrf_avx2_fusion`.
  - [x] Implementar `test_ladybug_graph_bfs_poison_prevention`.
  - [x] Garantir Exit Code 0 em `cargo test --bin souls_mcp_server` e zero clippy warnings.
