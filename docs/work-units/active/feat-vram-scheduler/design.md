---
spec: feat-vram-scheduler
version: 1.0
status: Active
branch: feat/vram-scheduler
author: souls-rust-expert
date: 2026-08-16
red_line: PROIBIDO stubs e simulações estatísticas de memória em spawn_blocking. PROIBIDO ultrapassar 6GB VRAM da RTX 2060m. PROIBIDO stalling do event loop Tokio por swapping síncrono. PROIBIDO decodificação sem restrição gramatical para saídas de código e JSON do SODA.
acao_de_canibalizacao: Extirpar stubs de simulação de KV Cache no KvCacheSwapController e implantar o swapping físico em DMA/Host RAM com histerese anti-flap (>=90% swap-out, <80% swap-in) via NVML/Win32. Implementar a algema de decodificação restrita llguidance com mascaramento vetorial AVX2 (256-bit) para coerção JSON em tempo <50µs/token com fail-closed. Conectar o hardware watchdog ao barramento assíncrono STATE_DB_TX para gravação contínua em telemetry_logs no SQLite WAL.
---

# Operação Guardião Térmico: Implantação do Pacote 5 — Swapping de VRAM em DMA Real e Algema em CPU AVX2 via llguidance

## 1. Contexto & Objetivos

A presente Work Unit implanta a infraestrutura física de controle térmico, proteção de VRAM e decodificação estruturada determinística para o **PACOTE 5: O GUARDIÃO TÉRMICO E A ALGEMA DE CONTEXTO**. Em conformidade absoluta com:
- **ADR-001** (Core Stack: Tokio Bare-Metal Rust)
- **ADR-003** (Isolamento de Stdio / Canais Protegidos)
- **ADR-010** (Pipeline SDD-TDD Rigoroso)
- **ADR-025** (Qualidade 100/100, zero warnings)
- **ADR-027** (Termodinâmica de VRAM: Teto rígido de 6.144 MB / RTX 2060m)
- **ADR-028** (Cercadinho do Determinismo: llguidance + AVX2 CPU Zero-Cost)
- **ADR-043** (Observabilidade e Telemetria FinOps no SQLite WAL)

Erradicamos todos os mocks e stubs de console em `src-tauri/src/core/vram_scheduler.rs` e conectamos a telemetria do `hardware_watchdog.rs` e a decodificação estruturada do `llguidance` com aceleração vetorial SIMD AVX2.

## 2. Linhas Vermelhas (Invioláveis)

| # | Regra | Justificativa |
|---|-------|---------------|
| R1 | Swapping Físico Real | O swapping de KV Cache Q4_K opera com buffers físicos em Host RAM via DMA / `spawn_blocking`, sem mensagens simuladas. |
| R2 | Histerese Anti-Flap | Exige exatamente 2 amostras consecutivas com VRAM >= 90% para swap-out (GPU -> Host RAM) e VRAM < 80% para swap-in (Host RAM -> GPU). |
| R3 | Algema CPU AVX2 llguidance | Gramática JSON estrita coercida na CPU do host via AVX2 em < 50 microssegundos por token, forçando tokens ilegais para $-\infty$. |
| R4 | Fail-Closed Determinístico | Se o motor llguidance encontrar erro de parsing ou estouro de stack, a geração é abortada com erro tipado sem emitir JSON corrompido. |
| R5 | Telemetria Lock-Free & MPSC | Telemetria compactada em `AtomicU64` (`WATCHDOG_STATE`) sem heap allocation no hot path e despachada via `STATE_DB_TX` para `telemetry_logs`. |

## 3. Topologia Orchestrator-Worker & Agnosticismo de Hardware

```mermaid
flowchart TD
    subgraph HW_LAYER [Camada de Hardware & Watchdog]
        NVML[NVML / Win32 Sysinfo Probe]
        WD_THREAD[Thread: souls-hardware-watchdog]
        NVML -->|1000ms poll| WD_THREAD
        WD_THREAD -->|pack_state| ATOMIC_STATE[AtomicU64: WATCHDOG_STATE]
        WD_THREAD -.->|try_send| MPSC_TX[STATE_DB_TX Barramento]
        MPSC_TX -->|Flush Async| SQLITE[(souls_state.db: telemetry_logs)]
    end

    subgraph SCHED_LAYER [VRAM Scheduler & Anti-Flap Hysteresis]
        ATOMIC_STATE -->|Acquire| SINK[WatchdogSink / VramPressureSink]
        SINK --> CTRL[KvCacheSwapController]
        CTRL -->|>=90% 2x consecutivas| SWAP_OUT[DMA Async Swap-Out: GPU -> Host RAM]
        CTRL -->|<80% 2x consecutivas| SWAP_IN[DMA Async Swap-In: Host RAM -> GPU]
    end

    subgraph INFER_LAYER [Inferência & Algema llguidance CPU AVX2]
        DGPU[dGPU RTX 2060m: Logits Brutos]
        DGPU -->|Passa Logits| LLG_ENGINE[llguidance Constraint Engine]
        
        subgraph AVX2_MASK [CPU AVX2 SIMD Core]
            CFG[JSON CFG Grammar / Tokenizer Trie]
            MASK_CALC[Constraint::compute_mask]
            AVX2_VEC[_mm256 Logit Masking: Força Não-CFG para -inf < 50µs]
            CFG --> MASK_CALC
            MASK_CALC --> AVX2_VEC
        end
        
        LLG_ENGINE --> AVX2_MASK
        AVX2_VEC --> MASKED_LOGITS[Logits Coercidos JSON-Strict]
        MASKED_LOGITS --> SAMPLE[Token Amostrado Válido]
    end
```

## 4. Estrutura de Telemetria e FinOps

1. **Compactação Lock-Free**: `pack_state` codifica `vram_mb`, `ram_mb`, `cpu_temp`, `gpu_temp` e `flags` em 64 bits.
2. **Registro no SQLite**: Gravação assíncrona via barramento `STATE_DB_TX` na tabela `telemetry_logs`.
3. **Isolamento Térmico**: Proteção ativa contra superaquecimento (flag de thermal throttle quando GPU >= 85°C).
