# Tasks & DoD: Operação Guardião Térmico (feat-vram-scheduler)

## Task 1: Governança Territorial & Design da Work Unit
- [x] Documentar `design.md` com arquitetura Orchestrator-Worker, topologia FinOps e Linhas Vermelhas.
- [x] Documentar `tasks.md` com decomposição atômica e Definition of Done (DoD).
- [x] Configurar diretório `.souls_scratchpad/logs/cargo/` para logs de build e clippy (`clippy_vram_scheduler.log`).

## Task 2: Extirpação do Stub e Swapping Físico no `vram_scheduler.rs`
- [x] Remover funções stubs que emitiam mensagens simuladas de console.
- [x] Implementar a máquina de estados de histerese anti-flap (2 leituras consecutivas com VRAM >= 90% para swap-out e VRAM < 80% para swap-in).
- [x] Implementar swapping físico de KV Cache Q4_K em Host RAM via DMA / buffers gerenciados em `spawn_blocking`.
- [x] Integrar leituras de telemetria física via NVML / Win32.

## Task 3: Algema de Decodificação Restrita via llguidance & AVX2
- [x] Integrar motor llguidance com suporte a gramática JSON estrita na CPU Host de custo zero.
- [x] Implementar mascaramento vetorial SIMD AVX2 de 256 bits (`is_x86_feature_detected!("avx2")`) forçando tokens inválidos para $-\infty$ em tempo < 50µs.
- [x] Implementar comportamento fail-closed com tratamento seguro contra panic/overflow.

## Task 4: Despacho de Telemetria Térmica MPSC
- [x] Manter compactação atômica em `AtomicU64` (`WATCHDOG_STATE`) com bit-masking no `hardware_watchdog.rs`.
- [x] Integrar despacho assíncrono para o barramento `STATE_DB_TX` para gravação na tabela `telemetry_logs` do SQLite WAL.

## Task 5: Suíte de Testes TDD Mandatória
- [x] `test_vram_scheduler_hysteresis_anti_flap`: Prova de anti-flap com 2 leituras consecutivas para transição.
- [x] `test_llguidance_avx2_json_coercion_speed`: Mascaramento de logits estocásticos via AVX2 em < 50µs na CPU.
- [x] `test_watchdog_state_bit_masking_integrity`: Compactação e descompactação de estados de hardware sem perda de sinal.
- [x] Execução com Exit Code 0 e zero clippy warnings.
