---
spec: feat-mcp-perf-hygiene
version: 2.0  # upgrade: extirpação de stubs de fachada
status: Active
branch: feat/mcp-perf-hygiene
author: souls-sdd + souls-rust-expert
date: 2026-08-20
red_line: |
  PROIBIDO Vec<u8> com 0xAA como swap de VRAM.
  PROIBIDO heurística de bytes (FNV-1a / prompt_derived) servindo como logits.
  PROIBIDO Wasmtime::Instance::new ser construída e descartada sem get_func/call.
  PROIBIDO regex como método primário de extração AST (apenas edge-case residual para arquivos >50MB sob AllowList).
  PROIBIDO executar ort/llama-cpp-2 na dGPU RTX 2060m.
  PROIBIDO estourar 6.144 MB de VRAM alocada para modelos.
  PROIBIDO panic de guest WASM ou de ONNX derrubar thread do Tokio.
  PROIBIDO mocks que retornam scores plausíveis (qualquer stub passa por revisão HITL).
acao_de_canibalizacao: |
  Extirpar em definitivo os 3 stubs de fachada dos motores críticos:
  (1) KvCacheSwapController.swap_out_kv_cache_q4k / swap_in_kv_cache_q4k:
      trocar host_mem.resize(128MB, 0xAA) por FFI real ik_llama_cpp_2
      (llama_memory_clear, llama_state_save/load) em Dedicated Worker Thread
      + canal MPSC para notificação, histerese anti-flap preservada.
  (2) OrtScorerEngine.score / classify: trocar multiplicação hash (line 202) e
      keyword matching por sessão ort 2.x com GLiClass ONNX real, GraphOpt
      Level3, intra_threads(2) inter_threads(1) em CPU AVX2, 0 MB dGPU.
  (3) WasmtimeTreeSitterEngine.parse_and_extract: drenar a Instance
      construída, expor get_typed_func("parse"/"extract_symbols"), copiar
      bytes da memória linear do guest para Vec no host, fuel=10M + 16MB
      de teto rígido + epoch interruption.
  Roteamento: ort e llama-cpp-2 gated por feature ort_backend / llama_backend;
  default = [] para preservar o build destravado. Em produção, ativar via
  build profile.
---

# Operação Extirpação de Slop: Materialização Real dos 3 Motores de Background

## 1. Contexto & Mandato

A WU pai `feat-mcp-perf-hygiene` (v1.0) saneou latência e telemetria mas
**deixou intactos os 3 stubs de fachada** que mascaram inatividade computacional
como produção. A presente v2.0 extirpa esses stubs e solda a lógica física
real, em conformidade com:

- **ADR-001** (Core Stack: Tokio Bare-Metal Rust)
- **ADR-003** (Isolamento de Stdio / Cerca de Stderr)
- **ADR-010** (Pipeline SDD-TDD / TDD Atômico)
- **ADR-025** (Qualidade 100/100 — zero mock de fachada)
- **ADR-027** (Termodinâmica de VRAM — 0 MB dGPU para ort/llama-cpp, 6.144 MB teto modelos)
- **ADR-028** (Decodificação Restrita via llguidance)
- **ADR-041** (Nomenclatura Soberana `souls_mcp`)
- **ADR-044** (Enjaulamento Wasmtime — Fuel + RAM Cap + Fail-Soft)

## 2. Linhas Vermelhas (Invioláveis)

| # | Regra | Mecanismo de Bloqueio |
|---|-------|------------------------|
| R1 | **Zero dGPU Allocation** para ort/llama-cpp | `ort::SessionBuilder` com `ExecutionProvider::CPU()` explícito; `llama_model_params.with_n_gpu_layers(0)` |
| R2 | **Histerese Anti-Flap 90/80** | 2 amostras consecutivas em `KvCacheSwapController::evaluate()` antes de despachar swap |
| R3 | **Teto de RAM Linear 16 MB no guest WASM** | `WasmMemoryLimiter` via `ResourceLimiter` trait + `Store::limiter` |
| R4 | **Fuel Metering 10M units** | `store.set_fuel(10_000_000)` antes de `func.call()`; captura de `TrapCode::OutOfFuel` |
| R5 | **Epoch Interruption como cinto de segurança** | `Engine::epoch_deadline_callback` (>= 1ms tick) aborta o guest sem panic |
| R6 | **Telemetria via MPSC** (não stdout) | `try_send_cold(StateDbOp::LogTelemetry {...})` em vez de `println!` |
| R7 | **Zero Tokio starvation** | ort e llama-cpp isolados em `std::thread::spawn` + `crossbeam_channel` para reply, ou `tokio::task::spawn_blocking` com `JoinHandle` |
| R8 | **Fail-Soft Estruturado** | Toda falha (OOM, fuel exhausted, ort error) vira `Result::Err` tipado, nunca `unwrap`/`panic` |
| R9 | **Telemetry Honest Gate** | `swap_out_kv_cache_q4k` só emite `tracing::info!` SE `bytes_moved > 0` E `nvml_free_bytes_after < nvml_free_bytes_before` |

## 3. Topologia Orchestrator-Worker & Agnosticismo de Hardware

```mermaid
graph TB
    subgraph CLIENTE [Cliente MCP / IDE]
        REQ[JSON-RPC tools/call]
    end

    subgraph GATEWAY [Gateway Tokio MCP]
        ROUTER[router.rs: handle_tool_call]
        ROUTER -->|intent| INTENT_HND[handlers/system.rs::run_intent]
        ROUTER -->|symbol / outline| AST_HND[handlers/...::run_repo_ast]
        ROUTER -->|mcp_audit| VRAM_HND[handlers/...::run_swap]
    end

    subgraph SWAP_ENGINE [Motor de Swapping - Dedicado]
        KVCTL[KvCacheSwapController]
        ANTI[Histerese 90/80 com 2 samples]
        DEDIC[std::thread::spawn Dedicated Worker]
        MPSC_IN[mpsc::Receiver: comandos]
        MPSC_OUT[mpsc::Sender: ack + métricas]
        KVCTL --> ANTI
        ANTI -->|decisão| DEDIC
        DEDIC -->|llama_memory_clear| IK_LLAMA[ik_llama_cpp_2 FFI n_gpu_layers=0]
        IK_LLAMA -->|save state 128MB| HOST_DRAM[Host RAM 32GB pinned]
        HOST_DRAM -.->|load state| IK_LLAMA
        DEDIC -->|nvml_query free| NVML[nvml-wrapper: free before/after]
        DEDIC -->|MPSC reply| MPSC_OUT
    end

    subgraph ONNX_ENGINE [Motor ONNX CPU - Dedicado]
        ORTENG[OrtScorerEngine]
        TOK[tokenizers crate: BPE Fast]
        SESS[ort::Session CPU EP + GraphOpt::Level3]
        GLICLASS[gliclass_multilang.onnx 0 MB dGPU]
        VEC[Tensor Output: intent/risk/conflict]
        ORTENG --> TOK
        TOK --> SESS
        SESS --> GLICLASS
        GLICLASS --> VEC
    end

    subgraph WASM_ENGINE [Wasmtime Jail - Wasmtime 29.x]
        WMOD[OnceLock<Module> por linguagem]
        WINST[wasmtime::Instance: get_typed_func parse()]
        WFUEL[store.set_fuel 10M]
        WLIM[ResourceLimiter 16MB RAM Linear]
        WEPOC[epoch_deadline_callback 1ms]
        WHST[Guest Memory: source bytes + cursor]
        WMOD --> WINST
        WFUEL --> WINST
        WLIM --> WINST
        WEPOC -.->|tick| WINST
        WINST -->|Func::call| WHST
        WHST -->|copy_from_linear| HOST_BUF[Vec<u8> no Host]
    end

    INTENT_HND --> ONNX_ENGINE
    AST_HND --> WASM_ENGINE
    VRAM_HND --> SWAP_ENGINE
    SWAP_ENGINE -->|try_send_cold| TEL_BUS[STATE_DB_TX MPSC]
    ONNX_ENGINE -->|try_send_cold| TEL_BUS
    WASM_ENGINE -->|try_send_cold| TEL_BUS
    TEL_BUS -->|Batch 5s| SQLITE[(souls_state.db: telemetry_logs)]
```

## 4. Estado Atual vs Estado Desejado

| Componente | Antes (v1.0) | Depois (v2.0) |
|------------|---------------|----------------|
| `swap_out_kv_cache_q4k` | `Vec::resize(128MB, 0xAA)` + log mentiroso | `ik_llama_cpp_2::llama_memory_clear(ctx, true)` + `n_ctx` save em host pinned memory + `nvml.Device.free()` delta verificado |
| `swap_in_kv_cache_q4k` | `Vec::clear() + shrink_to_fit()` | `llama_state_load` + `nvml` re-check delta negativo |
| `OrtScorerEngine::score` | `t.wrapping_mul(2654435761) % 10000` | `ort::Session::run([input_ids, attention_mask])` + softmax estável |
| `OrtScorerEngine::classify` | `lower.contains("jailbreak")` etc. | Inferência real zero-shot GLiClass contra labels verbais |
| `WasmtimeTreeSitterEngine::parse_and_extract` | `Instance::new` + drop; loop de `starts_with()` | `Instance::get_typed_func::<(i32, i32), i32>("parse")` + `func.call()` + drain linear memory |

## 5. Metas de Desempenho Real (Latência P50)

| Operação | Meta | Mecanismo |
|----------|------|-----------|
| `swap_out_kv_cache_q4k` 4GB Q4_K | < 8s PCIe Gen3 x4 (~1GB/s) | `cudaMemcpy` bloqueante em thread dedicada |
| `swap_in_kv_cache_q4k` 4GB | < 8s | inverso |
| `OrtScorerEngine::classify` (prompt 4KB) | < 15ms | ort 2.x CPU AVX2 Level3 |
| `WasmtimeTreeSitterEngine::parse_and_extract` (Rust 50KB) | < 50ms | Wasmtime Cranelift cached + fuel 10M |
| `Wasmtime fuel exhaustion` (loop infinito) | < 5ms | epoch deadline + TrapCode::OutOfFuel |

## 6. Agnosticismo de Hardware (ADR-001)

Toda a lógica de CPU é **transmutável**: o código NÃO é engessado para a RTX
2060m. A estrutura está pronta para ser recompilada via `cubecl` / `burn` /
`candle` para Metal/Vulkan/NPU. A RTX 2060m serve apenas como **treino de
gravidade** (piso de validação: 0 MB dGPU em ort e llama-cpp).

- `ort` é executado exclusivamente em CPU EP — portable para ARM, RISC-V, etc.
- `ik_llama_cpp_2` é compilado com `n_gpu_layers=0` — CPU-only.
- `Wasmtime` é ISA-agnóstico (Cranelift gera código nativo para qualquer alvo suportado).

## 7. Riscos Arquiteturais Identificados

| # | Risco | Mitigação |
|---|-------|-----------|
| K1 | Build do `ort` 2.x precisa baixar o binário do Microsoft ONNX Runtime (libonnxruntime.so/.dll). Em sandbox sem internet, falha. | Pin `ort = "2.0.0-rc.10"` + `ort::download_binaries()` com feature opcional; em sandbox, fallback explícito para fail-soft com erro tipado. |
| K2 | `llama_backend` feature exige CUDA no Windows. RTX 2060m tem CUDA mas build pode falhar se toolchain ausente. | Feature `llama_backend` permanece opt-in (default = []). O swap real só ativa se feature ON. |
| K3 | Os `.wasm` em `resources/wasm_grammars/` podem não exportar uma função `parse` com a assinatura esperada. | Validar export com `module.exports()` antes de chamar; emitir `WasmTrap::MissingExport` se ausente. |
| K4 | O GLiClass `.onnx` em `models/` é de triagem zero-shot. As labels verbais precisam estar no formato esperado pelo grafo. | Usar o template de prompt documentado em `scripts/convert_gliclass.py`; passar labels como input separado. |
| K5 | `cargo test --bin souls_mcp_server` precisa passar com `Exit Code 0` e zero clippy warnings. Isso exige que TODOS os 3 motores funcionem sob feature default (sem ort/llama). | Os 3 testes antifraude serão `#[cfg(feature = "...")]` e só rodam com feature ON. O smoke test de `cargo test` rodará com `default = []` e validará que os caminhos sem feature retornam erro tipado (não panic). |

## 8. Conformidade com ADR

| ADR | Aplicação |
|-----|-----------|
| ADR-001 | Rust + Tokio, sem dependência Python/JS em runtime |
| ADR-003 | Toda telemetria via MPSC, zero `println!` síncrono |
| ADR-010 | TDD Red-Green-Refactor, testes antes do código de produção |
| ADR-025 | Zero `unwrap()`/`expect()` em hot-path; `Result<T, E>` tipado |
| ADR-027 | ort e llama-cpp com `n_gpu_layers=0`; 0 MB dGPU |
| ADR-028 | llguidance coerção AVX2 preservada (não regredida) |
| ADR-041 | Tools MCP expostas limpas; nenhum prefixo `souls_*` em tool name público |
| ADR-044 | Wasmtime: fuel + RAM cap + epoch interruption + cache lock-free |
