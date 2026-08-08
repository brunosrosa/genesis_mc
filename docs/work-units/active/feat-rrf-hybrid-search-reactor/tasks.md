# TAREFAS E DEFINITION OF DONE (DoD) — MARCO 5.6.0

## TAREFA 1: FtsRetriever (fts_retriever.rs)
- [ ] Criar `src-tauri/src/cognition/memory/fts_retriever.rs`
- [ ] Implementar `FtsRetriever::search_lexical` consultando `observations_fts` via `bm25()` em $\mathcal{O}(\log N)$
- [ ] Retornar `Vec<LexicalMatch>` ordenado por relevância
- [ ] DoD: Teste `test_fts5_lexical_retrieval` executando com sucesso em tempo sub-milissegundo

## TAREFA 2: VectorRetriever (vector_retriever.rs)
- [ ] Criar `src-tauri/src/cognition/memory/vector_retriever.rs`
- [ ] Implementar `VectorRetriever::search_vectorial` abrindo LanceDB via `mmap` NVMe
- [ ] Garantir 0 MB de alocação VRAM na RTX 2060m
- [ ] DoD: Teste `test_lancedb_mmap_vram_safety` executando sem alocação GPU

## TAREFA 3: RrfFusionEngine (rrf_fusion.rs)
- [ ] Criar `src-tauri/src/cognition/memory/rrf_fusion.rs`
- [ ] Implementar a fórmula de fusão RRF $1 / (k + rank)$ com $k = 60.0$
- [ ] Ordenar resultados decrescentemente pelo `RRF_Score`
- [ ] DoD: Teste `test_rrf_mathematical_fusion` validando estaticamente os rankings RRF

## TAREFA 4: Invalidação JIT Tombstone
- [ ] Implementar varredura atômica em `souls_state.db` para conferir `status_atualizacao` / `status_processamento`
- [ ] Expurgar registros com status `superseded` ou `invalid`
- [ ] DoD: Teste `test_jit_tombstone_invalidation` provando o expurgo lossless de premissa superada

## TAREFA 5: Registro no MCP e Higiene Stdio (souls_mcp_server.rs)
- [ ] Registrar garra `souls_semantic_search` em `tools/list` respeitando ADR-041 (32/120)
- [ ] Conectar o handler `handle_tool_call` para orquestrar Tokio tasks paralelas
- [ ] Manter `stdout` 100% limpo com telemetria via `eprintln!`
- [ ] DoD: Executar `cargo check --bin souls_mcp_server` e `cargo clippy --bin souls_mcp_server` com zero avisos

## TAREFA 6: Suíte de Testes TDD (DoD GREEN)
- [ ] Rodar `cargo test --bin souls_mcp_server` para os 4 testes de integração
- [ ] DoD: Todos os 4 testes passando limpos (DoD GREEN)
