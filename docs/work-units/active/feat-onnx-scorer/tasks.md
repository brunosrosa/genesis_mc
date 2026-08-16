# Tasks & DoD: Operação Sentinela de Borda (feat-onnx-scorer)

## Task 1: Governança Territorial & Design da Work Unit
- [x] Documentar `design.md` com arquitetura Orchestrator-Worker, topologia FinOps e Linhas Vermelhas.
- [x] Documentar `tasks.md` com decomposição atômica e Definition of Done (DoD).
- [x] Configurar diretório `.souls_scratchpad/logs/cargo/` para logs de build e clippy.

## Task 2: Refatoração e Expurgo do Stub em `ort_scorer.rs`
- [x] Remover stubs de similaridade baseados em string length (`len.ln() / 1024.0`).
- [x] Implementar motor real de inferência e scoring vetorial na CPU com aceleração SIMD AVX2.
- [x] Configurar thread pool seguro (`intra_threads=2`, `inter_threads=1`) e otimização `GraphOptimizationLevel::Level3`.
- [x] Implementar resolução de caminhos canônicos para `gliclass-multilang-ultra.onnx` e `tokenizer.json` em `src-tauri/resources/models/` e `src-tauri/models/`.
- [x] Implementar truncagem segura estrita de 4096 caracteres com limites UTF-8.

## Task 3: Integração de Telemetria FinOps & Despacho MPSC
- [x] Capturar latência TTFT de triagem na CPU em milissegundos.
- [x] Integrar envio não-bloqueante de métricas via `STATE_DB_TX` para persistência em `telemetry_logs` no `souls_state.db`.

## Task 4: Suíte de Testes TDD (Marcha Rápida)
- [x] `test_onnx_scorer_real_inference_avx2`: Validação de inferência real em português com scores normalizados na CPU.
- [x] `test_onnx_scorer_vram_isolation_proof`: Prova de 0 MB de alteração na VRAM da dGPU via NVML.
- [x] `test_onnx_scorer_input_exhaustion_truncation`: Poda estrita de prompts > 8000 caracteres mantendo latência < 20ms.
- [x] Execução com Exit Code 0 e zero clippy warnings.
