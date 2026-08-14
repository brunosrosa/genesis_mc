---
spec: marco-iv-watchdog-thermal-vram
version: 1.0
status: Draft_Aguardando_Aprovacao
branch: feat/marco-iv-watchdog-thermal
author: souls-rust-expert
date: 2026-08-12
red_line:
  - NUNCA delegar monitoria de hardware a script Python/daemon externo.
  - NUNCA permitir que o loop Tokio leia sysinfo diretamente (inanição do reactor).
  - NUNCA emitir swap-out do KV Cache sem evidência física de estouro (>=90% VRAM).
  - NUNCA re-alocar LoRA sem antes desalocar o adaptador anterior (leak de handle CUDA).
  - NUNCA usar `lora-init-without-apply` como substituto para hot-swap — apenas para pré-registro.
acao_de_canibalizacao: Canibalizar a coluna de sysinfo 0.30.13 (pino do workspace) e a syscall NVML já herdada de hardware_profiler.rs; reaproveitar o trait watch::Receiver<SystemState> do souls_thermal_governor para o sinal de thermal pressure.
---

# MARCO IV — O Watchdog Térmico e VRAM Scheduler

## 1. Contexto

O `souls_mc` opera sob o teto rígido de **6.144 MB de VRAM** na RTX 2060m (ADR-027). O estado atual da fábrica tem dois pontos de fragilidade silenciosa:

1. Nenhum daemon nativo de S.O. coleta telemetria física de RAM/VRAM/temperatura em tempo real. O `hardware_profiler.rs` apenas lê *snapshot* one-shot no startup.
2. O `vram_scheduler.rs` (Marco 5.12.0) opera em nível de **modelos** (LRU de pesos) — não há proteção contra estouro de VRAM dentro de uma janela de inferência ativa (KV cache saturado).
3. Não existe hot-swap de adaptadores LoRA — toda troca de especialidade (Coder, Socrático, Heurístico) requer reload do `llama_context`, o que custa segundos.

O presente Marco materializa três peças:

- **Hardware Watchdog**: thread nativa S.O. com sysinfo 0.30.13 publicando telemetria compactada num `AtomicU64` lock-free.
- **KvCacheSwapController**: consumidor async do `WATCHDOG_STATE` que dispara swap-out Q4_K GPU→RAM e swap-in de retorno.
- **LlamaLoraAdapter**: registro preguiçoso + hot-swap sub-milissegundo via FFI ik_llama.cpp.

## 2. Linha Vermelha (Inviolável)

| #  | Regra                                                                                          | Justificativa                                                                       |
|----|------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------|
| R1 | Watchdog em `std::thread::spawn` dedicada (NUNCA tokio task)                                 | Isolamento de cache L1/L2 e proteção do reactor contra `sysinfo::refresh_*` blocking |
| R2 | `WATCHDOG_STATE` é `OnceLock<Arc<AtomicU64>>` com bit-mask (RAM\|VRAM\|Temp)                  | Lock-free, O(1) read, sem mutex, sem RwLock                                         |
| R3 | `KvCacheSwapController` só dispara swap-out se `vram_pct >= 90` confirmado em 2 amostras     | Anti-flap: ruído do driver não pode causar thrashing                                 |
| R4 | `apply_lora_adapter_in_flight` falha fechada se o adaptador anterior ainda estiver aplicado | Previne leak de handle CUDA                                                         |
| R5 | Hot-swap LoRA deve completar em < 5ms (orç. de telemetria com `Instant::now`)                | Cumpre a meta do Marco IV §3.4 sem vazar tempo ao reactor                            |
| R6 | Módulos de FFI (ik_llama.cpp) gateados por `#[cfg(feature = "llama_backend")]`                | Mantém o core compilável em esteira de CI sem CUDA                                  |

## 3. Agnosticismo Hardware (ADR-027 / Marco 4.9.0)

| Componente          | Treino de Gravidade (RTX 2060m) | Agnosticismo                                          |
|---------------------|--------------------------------|-------------------------------------------------------|
| HardwareWatchdog    | NVML + sysinfo cross-OS        | sysinfo é cross-OS; NVML é fallback fail-soft         |
| KvCacheSwapController | CUDA VRAM                    | Trait `VramPressureSink` permite backends futuros    |
| LlamaLoraAdapter    | ik_llama.cpp FFI               | Trait `LoraAdapter` permite backends pytorch/rust     |

A RTX 2060m é o *treino de gravidade*; o código é estruturado para ser transmutado para Metal/Vulkan/NPU sem reescrita (ADR-027).

## 4. Padrão Orchestrator-Worker

```mermaid
flowchart TD
    subgraph "HardwareWatchdog (std::thread)"
        W1[sysinfo::System::refresh_all] --> W2[Bit-pack: RAM|VRAM|Temp]
        W2 --> W3[AtomicU64::store]
    end

    W3 --> WS[static WATCHDOG_STATE<br/>OnceLock&lt;Arc&lt;AtomicU64&gt;&gt;]

    subgraph "Tokio Control Loop"
        SC[Scheduler::poll_vram_pressure] --> WS
        SC -->|>= 90% sample #1| CO[Cooldown counter]
        CO -->|>= 90% sample #2| SO[swap_out_kv_cache_q4k]
        SO --> RAM[Host RAM Standby]

        SC -->|sample &lt; 80% | SI[swap_in_kv_cache_q4k]
        SI --> GPU[RTX 2060m VRAM]
    end

    subgraph "LlamaLoraAdapter (gated: llama_backend)"
        LL1[pre_register: lora-init-without-apply] --> RAM2[Host RAM inert weights]
        LL2[apply_lora_adapter_in_flight] --> CTX[*mut llama_context]
        LL3[release previous adapter] --> CTX
    end

    style W1 fill:#1e3a5f,stroke:#fff
    style SC fill:#5f1e1e,stroke:#fff
    style LL2 fill:#1e5f3a,stroke:#fff
```

## 5. Bit-Mask do WATCHDOG_STATE (64 bits)

| Bits  | Campo                  | Resolução                    | Range             |
|-------|------------------------|------------------------------|-------------------|
| 0-19  | `vram_used_mb`         | 1 MB                         | 0 .. 1.048.575    |
| 20-39 | `ram_used_mb`          | 1 MB                         | 0 .. 1.048.575    |
| 40-49 | `cpu_temp_celsius`     | 0.5 °C (x2 + offset)         | 0 .. 1023 (×0.5)  |
| 50-59 | `gpu_temp_celsius`     | 0.5 °C (x2 + offset)         | 0 .. 1023 (×0.5)  |
| 60-63 | flags (thermal/power)  | boolean (thermal_throttle)   | bits reservados   |

Decodificação: helpers `decode_vram_mb(u64)`, `decode_ram_mb(u64)`, `decode_cpu_temp_c(u64)`, `decode_gpu_temp_c(u64)`, `decode_thermal_flag(u64)`.

## 6. Tabela de Marcas (Camada por Camada)

| Camada | Arquivo                                              | Estrutura                                  | DoD                                                  |
|--------|------------------------------------------------------|--------------------------------------------|------------------------------------------------------|
| L1     | `core/hardware_watchdog.rs` (NOVO)                   | `HardwareWatchdog` + `WATCHDOG_STATE`      | `cargo check` + telemetria < 5ms em 1000 amostras   |
| L1     | `core/vram_scheduler.rs` (EDIT)                      | `KvCacheSwapController` + `VramPressureSink` trait | Swap-out dispara em 92% simulado                  |
| L1     | `core/llama_lora_adapter.rs` (NOVO, gated)           | `LlamaLoraAdapter` + FFI                   | Hot-swap medido < 5ms                                |
| L2     | `core/mod.rs` (EDIT)                                 | `pub mod` dos 2 novos                      | `cargo check` Exit 0                                 |
| L3     | `tests/vram_scheduler_tests.rs` (NOVO)               | 3 testes TDD                                | Todos verdes                                         |
| L4     | `cargo clippy --features "tauri-app,gateway_ccr,llama_backend" -- -D warnings` | Validação estática | Exit 0 ou workaround documentado |

## 7. Critério de Aceitação (DoD Global)

- `cargo check --all-targets` Exit Code 0
- `cargo test --test vram_scheduler_tests` Exit Code 0 (3/3 contratos)
- `cargo clippy --features "tauri-app,gateway_ccr,llama_backend" -- -D warnings` Exit Code 0 **ou** workaround documentado (issue pré-existente CUDA do `llama-cpp-2 v0.1.154`)
- `core/mod.rs` exporta os 2 novos módulos
- `WATCHDOG_STATE` é um único `OnceLock<Arc<AtomicU64>>` global

## 8. Pedido de Aprovação

**Arquiteto, o design e o roteamento agnóstico estão aprovados?**

- [ ] Aprovado para Fase 4 (criar `tasks.md` com DoD atômico)
- [ ] Aprovado com ajustes (especificar)
- [ ] Rejeitado (justificar)
