---
spec: feat-onnx-scorer
version: 1.0
status: Active
branch: feat/onnx-scorer
author: souls-rust-expert
date: 2026-08-16
red_line: PROIBIDO stubs de similaridade baseados em string length. PROIBIDO alocacao ou execucao na dGPU RTX 2060m (isolar estritamente em CPU AVX2). PROIBIDO interrupcao do event loop do Tokio por computacao densa.
acao_de_canibalizacao: Substituir os stubs heurísticos e simulações do OrtScorerEngine pelo motor físico real de triagem de borda com execução vetorial CPU AVX2, carregamento estático e thread pool isolado (intra_threads=2, inter_threads=1, Level3 graph optimization). Roteamento de telemetria assíncrona para souls_state.db (telemetry_logs via STATE_DB_TX) e truncagem estrita em 4096 caracteres para blindagem contra exaustão de contexto.
---

# Operação Sentinela de Borda: Implantação do Pacote 3 — Real OrtScorerEngine (ONNX Runtime CPU AVX2)

## 1. Contexto & Objetivos

A presente Work Unit implanta o motor físico de inferência de borda para o **PACOTE 3: A SENTINELA DE TRIAGEM RÁPIDA (ONNX CPU)**. Em conformidade absoluta com:
- **ADR-001** (Core Stack: Tokio Bare-Metal Rust)
- **ADR-003** (Isolamento de Stdio / Canais Protegidos)
- **ADR-010** (Pipeline SDD-TDD Rigoroso)
- **ADR-025** (Qualidade 100/100, zero warnings)
- **ADR-027** (Termodinâmica de VRAM: RTX 2060m intocada com 0 MB de alteração)
- **ADR-030** (Higiene de Crates e Pinning Rígido)
- **ADR-043** (Observabilidade e Telemetria FinOps)

Erradicamos todos os mocks e stubs simulados em `src-tauri/src/core/ort_scorer.rs` e integramos o classificador de intenções e segurança de alta fidelidade com aceleração SIMD AVX2.

## 2. Linhas Vermelhas (Invioláveis)

| # | Regra | Justificativa |
|---|-------|---------------|
| R1 | Zero dGPU Allocation | A sentinela de triagem rápida roda **estritamente na CPU do host (EP CPU)**. VRAM da RTX 2060m permanece com 0 MB de consumo adicional. |
| R2 | Thread Pool Contido | Sessão configurada com `intra_threads(2)` e `inter_threads(1)` e nível de otimização de grafo `Level3` (AVX2/SIMD). |
| R3 | Truncagem Segura (4096 chars) | Fatiamento antecipado de prompts > 4096 caracteres no boundary de segurança para prevenir DoS/exaustão. |
| R4 | Zero Tokio Event-Loop Stall | Execução encapsulada em `tokio::task::spawn_blocking` ou threads dedicadas. |
| R5 | Telemetria FinOps MPSC | Despacho não-bloqueante de latência (TTFT da sentinela), classificação de intenção e risco para `telemetry_logs` via `STATE_DB_TX`. |

## 3. Topologia Orchestrator-Worker & Agnosticismo de Hardware

```mermaid
flowchart TD
    REQ[Prompt de Entrada / Ingestão] --> SAN[Boundary Sanitizer: Truncagem 4096 chars]
    SAN --> CLK[Instant::now Clock CPU]
    
    CLK --> ORT_ENG[OrtScorerEngine: CPU Execution Provider]
    
    subgraph CPU_JAIL [CPU AVX2 SIMD Core]
        SES[Session Config: intra=2, inter=1, GraphOpt=Level3]
        TOK[HuggingFace Tokenizer Fast BPE]
        MODEL[gliclass_multilang.onnx Graph / Embeddings]
        SES --> TOK
        TOK --> MODEL
    end
    
    ORT_ENG --> CPU_JAIL
    CPU_JAIL --> VEC[Tensor Output: Softmax Intenção & Risco]
    
    VEC --> TTFT[Cálculo de Latência TTFT ms]
    TTFT --> TEL[Telemetria Event Payload]
    
    TEL -->|try_send MPSC| TX[(STATE_DB_TX Bus)]
    TX -->|Batch Flush 5s| DB[(souls_state.db: telemetry_logs)]
    
    VEC --> RESP[SoulsInferenceResponse / Classificação Final]
```

## 4. Estrutura de Telemetria e FinOps

1. **Latência de Triagem (TTFT)**: Medida em microssegundos / milissegundos na CPU do host.
2. **Registro no SQLite**: Gravação assíncrona via barramento `STATE_DB_TX` na tabela `telemetry_logs`.
3. **Isolamento Térmico**: Zero ativação de CUDA / NVML VRAM delta = 0 MB.
