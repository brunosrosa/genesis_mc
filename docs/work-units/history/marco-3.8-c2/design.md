# Design — Marco 3.8 Fase C.2: Enjaulamento Wasmtime do Tree-Sitter

> Documento de design arquitetural para o Marco 3.8 Fase C.2.
> Referência normativa: [ADR-044](docs/adrs/ADR-044-Enjaulamento-Wasmtime-Tree-Sitter.md).
> Regido pelo protocolo BMAD + SDD + TDD (Red-Green-Refactor).

## 1. Topologia FinOps

```mermaid
graph LR
    subgraph Cliente["Cliente MCP (Trae IDE / Claude Desktop)"]
        LLM[LLM Agent]
    end

    subgraph Gateway["Gateway MCP (souls_mcp_server)"]
        Dispatcher[Dispatcher / tools/call]
        SymbolTool[symbol]
        CallersTool[callers]
        CalleesTool[callees]
        MPSC_TX[MPSC Sender<br/>try_send fire-and-forget]
    end

    subgraph Host["Host Rust Process"]
        WasmEngine["WasmEngine (OnceLock)<br/>fuel=10M, mem=16MiB"]
        DashMaps["DashMap SYMBOL_INDEX<br/>+ DashMap CALL_GRAPH"]
        Worker["std::thread<br/>Telemetry Worker"]
    end

    subgraph Sandbox["Wasm Guest (sandbox)"]
        WAT[WAT/WASM grammar<br/>tree-sitter-c / tree-sitter-rust]
    end

    LLM -->|tools/call symbol/callers/callees| Dispatcher
    Dispatcher --> SymbolTool
    Dispatcher --> CallersTool
    Dispatcher --> CalleesTool

    SymbolTool -->|DashMap.get O(1)| DashMaps
    CallersTool -->|DashMap.get adjacents| DashMaps
    CalleesTool -->|DashMap.get adjacents| DashMaps

    SymbolTool -.->|HIPER-FORWARD telemetry| MPSC_TX
    MPSC_TX -->|try_send bounded 256| Worker
    Worker -->|WasmEngine.execute_safely| WasmEngine
    WasmEngine -->|fuel + mem limiter| WAT
    WAT -.->|Trap = StructuredFailure| WasmEngine
    WasmEngine -.->|insert/replace| DashMaps
```

## 2. Padrão Orchestrator-Worker

- **Orchestrator (Tokio runtime).** O `Dispatcher` do gateway MCP recebe `tools/call` para `symbol`/`callers`/`callees`. Caminho de cache hit: `DashMap::get` em O(1). Sem alocação de regex, sem I/O de disco, sem spawn de task.
- **Worker (std::thread dedicada).** A `Telemetry Worker` consome `TelemetryEvent` do canal MPSC e executa o parse tree-sitter **dentro do sandbox Wasmtime**. Síncrono, sem `spawn_blocking` do Tokio, isolado do event loop.
- **Caminho de Telemetria (HIPER-FORWARD).** Toda mutação de arquivo via `read`/`edit` dispara `try_send(TelemetryEvent::FileMutated)`. Se o canal estiver cheio (write storm), descarta + `tracing::warn!` — **nunca bloqueia o critical path**.

## 3. Agnosticismo de Hardware

A cerca Wasmtime é 100% CPU-bound. Não emite instruções CUDA, Metal ou Vulkan. Em produção, o mesmo binário roda em:

- **RTX 2060m (Piso de Validação).** i5/i7 + AVX2 + 16GB RAM. Engine Cranelift usa SIMD automaticamente.
- **Apple Silicon M1/M2/M3 (Teto Agnóstico).** Recompilação para `aarch64-apple-darwin` mantém fuel/mem metering idênticos.
- **NPU/DSP futuros.** Wasmtime já tem backend para WASM em NPUs; o contrato `WasmEngine::execute_safely` permanece estável.

## 4. Estado e Transações

- **SYMBOL_INDEX e CALL_GRAPH** são estruturas **lock-free read, sharded write** via `DashMap` (6.1.0).
- **Não há transação SQLite** para o Call Graph: tudo é RAM Host. Persistência para cold start é responsabilidade de Marco futuro (snapshot binário `.souls_cache/callgraph.snapshot`).
- **Wasm guest** é descartável: cada chamada cria um `Store` novo, executa, descarta. RAII garante zero leak.

## 5. Custos e Hot Paths

| Operação | Custo | Observação |
|----------|-------|------------|
| `symbol(name)` cache hit | ~500ns | DashMap.get + 1 alloc String |
| `symbol(name)` cache miss | ~50ms | Telemetry worker (WASM parse) |
| `callers/callees` cache hit | ~1μs | DashMap.get + clone HashSet |
| Telemetry parse arquivo 5KB | ~30ms | Wasmtime fuel 10M, mem 16MiB |
| Telemetry parse arquivo 500KB | ~150ms | Mesmo fuel, AST denso |

## 6. Lei do Scaffold (DoD Pré-Codificação)

Cada tarefa em [tasks.md](tasks.md) tem teste vermelho obrigatório antes da lógica real. Esta é a cerca que blinda o arquiteto contra **Vibe Coding**.

## 7. Riscos e Mitigações (Pessimismo da Razão)

| Risco | Mitigação |
|-------|-----------|
| Engine Wasmtime inicializa devagar | `OnceLock<Engine>` singleton |
| `tree-sitter-c` falhar em gramática malformada | Wasmtime trap → `WasmTrap::StructuredFailure` |
| DashMap crescer sem teto | (Marco futuro) LRU eviction por `last_updated` Langevin |
| Tool `symbol` chamada com `name=""` | Validação rigorosa no handler → `-32602` |
| Conflito de alias `souls_symbol` vs `symbol` | Dispatcher testado em `tools_list_returns_unprefixed_names` |
