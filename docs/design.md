# DESIGN TÉCNICO: CLUSTER 2 — EXPLORAÇÃO E TOPOLOGIA AST (OS OLHOS DO AGENTE)

## 1. Visão Geral e Arquitetura Orchestrator-Worker
O **Cluster 2** adiciona duas ferramentas de exploração topológica e AST ao servidor `souls_mcp_server`:
1. `souls_tree`: Lente de diretórios não-bloqueante com **Dot-Flattening** rigoroso e exclusão rígida de caminhos tóxicos (`target/`, `node_modules/`, `.git/`, `.souls_cache/`, `.souls_data/`, `.cargo/`) e `.gitignore`.
2. `souls_outline`: Lente de extração de assinaturas AST executada dentro de uma **jaula isolada Wasmtime WASI 0.2 (wasip2)** para garantir resiliência militar contra Segmentation Faults e traps de parsers.

### Diagrama Arquitetural (Mermaid)

```mermaid
graph TD
    Client[LLM / MCP Client] -->|tools/call souls_tree| TreeHandler[souls_tree Handler]
    Client -->|tools/call souls_outline| OutlineHandler[souls_outline Handler]
    
    subgraph Host RAM - Tokio Async Runtime
        TreeHandler -->|tokio::fs::read_dir| DirWalker[Directory Walker + GitIgnore Filter]
        DirWalker -->|Check entries.len() == 1 && entry.is_dir()| DotFlattenEncoder[Strict Dot-Flattening Engine]
        DotFlattenEncoder -->|Return LEAN Tree| Client
        
        OutlineHandler -->|Read File Buffer| HostRAM[Host Memory Buffer]
        HostRAM -->|Pass Buffer to WASI 0.2 Store| WasmCage[Wasmtime WASI 0.2 Sandbox Engine]
    end
    
    subgraph Wasmtime Cage (WASI 0.2 Isolated Component Execution)
        WasmCage -->|Load Embedded WASM Grammar via include_bytes!| ASTParser[Embedded WASM Grammar Module]
        ASTParser -->|Traverse AST & Strip Bodies| SignatureExtractor[Signature Extractor]
        SignatureExtractor -->|Return Signatures| HostRAM
    end
    
    subgraph Fail-Closed Error Boundary
        ASTParser -- WASM Trap / Panics --> TrapHandler[Wasmtime Trap Interceptor]
        TrapHandler -->|Map Trap -> RPC Error -32022| Client
    end
```

---

## 2. Estratégia de Carregamento de Gramáticas WASM (Sem Sidecars e Sem Mocks)

### 2.1. Embarcamento Físico via `include_bytes!`
As gramáticas compiladas para WebAssembly (ex: `tree_sitter_rust.wasm`, `tree_sitter_typescript.wasm`) e a casca do parser de outlines são armazenadas em `src-tauri/resources/wasm_grammars/` e embarcadas diretamente no binário Rust via `include_bytes!`:

```rust
static WASM_RUST_GRAMMAR: &[u8] = include_bytes!("../resources/wasm_grammars/tree_sitter_rust.wasm");
static WASM_OUTLINE_PARSER: &[u8] = include_bytes!("../resources/wasm_grammars/outline_parser.wasm");
```

- **Vantagem:** Zero I/O de disco para carregar gramáticas em runtime, implantação bare-metal em arquivo único executável (Zero Sidecars) e imune a corrupção de sistema de arquivos.

### 2.2. Execução sob o WASI 0.2 (wasip2)
- O runtime `wasmtime` é instanciado com o WASI 0.2 Component Model (`wasmtime::component::Component` + `wasmtime_wasi::p2`).
- As chamadas de parsing em RAM passam a string do código para o buffer linear da sandbox WASI 0.2.

---

## 3. Especificação das Ferramentas MCP

### 3.1. `souls_tree` (Dot-Flattening Rigoroso)
- **Regra de Achatamento (Dot-Flattening):**
  Um diretório $D$ é achatado com seu filho $C$ em `$D/$C` **APENAS E SOMENTE SE** $D$ contiver **EXATAMENTE 1 elemento total** (`entries.len() == 1`) E esse elemento for um subdiretório.
  - Se $D$ contiver o subdiretório $C$ **E** o arquivo `main.rs`, o número de elementos é 2. O achatamento é **PROIBIDO**, e $D$ é exibido com seus filhos recuados/listados no nível correspondente.
- **Filtro Absoluto:** ignora `target/`, `node_modules/`, `.git/`, `.souls_cache/`, `.souls_data/`, `.cargo/` e padrões em `.gitignore`.
- Erro RPC `-32015` se houver tentativa de travessia fora da raiz do workspace.

### 3.2. `souls_outline`
- Extrai assinaturas (`struct`, `enum`, `trait`, `impl`, `fn`) omitindo corpos funcionais.
- Erro RPC `-32021` em caso de falha de parsing sintático.
- Erro RPC `-32022` interceptando `wasmtime::Trap` na sandbox WASI 0.2 sem afetar o processo pai Tokio.

---

## 4. Matriz de Cobertura TDD e Estrutura de Testes

1. `test_tree_flattening_successful`:
   Testa tanto a compactação linear de diretórios únicos quanto a preservação espacial de arquivos adjacentes (ex: `src/a/` com `b/` e `main.rs`).
2. `test_tree_ignores_toxic_paths`:
   Valida que `node_modules` e `target` não aparecem na árvore.
3. `test_outline_rust_signatures`:
   Valida extração de assinaturas reais sem corpo de função via parser WASM.
4. `test_wasm_sandbox_trap_containment`:
   Compila via WAT/WASM um componente com instrução `unreachable` e valida a captura do Trap retornando erro RPC `-32022`.

---

## 5. DESIGN TÉCNICO: SOULS V4 — UPGRADE DE MOTORES DE INFERÊNCIA E BITNET DAEMON (SPIKE)

### 5.1. Visão Geral e Arquitetura Orchestrator-Worker
O backend em Rust (Tokio) atua como o **Orchestrator**, delegando workloads para dois motores de inferência **Workers**:
1. `LlamaCppEngine` (dGPU CUDA Worker): Carregado intra-processo via bindings CFFI (`llama-cpp-2` v0.1.153) para geração contínua $O(1)$ com KV Cache Assimétrico (`Key: F16`, `Value: Q4_K` / `Q8_0`).
2. `BitNetDaemon` (CPU Sidecar Worker): Subprocesso efêmero executando o binário `bitnet_daemon.exe` via Tokio `Command`, protegido por um Drop Guard com cancelamento atômico (`SIGKILL` / `child.kill()`).

### 5.2. Diagrama Arquitetural (Mermaid)
```mermaid
graph TD
    Client[Tauri IPC / MCP Client] -->|Inference Request| Orchestrator[Souls Model Manager - Tokio Runtime]
    
    subgraph Host Process - Rust Tokio Runtime
        Orchestrator -->|GPU Inference Request| LlamaEngine[LlamaCppEngine - llama-cpp-2 v0.1.153]
        Orchestrator -->|CPU 1-Bit Request| BitNetManager[BitNetDaemon Struct]
        
        LlamaEngine -->|Asymmetric KV Cache| KVCache[Key: F16 / Value: Q4_K]
        BitNetManager -->|Subprocess Control| TokioChild[Tokio Async Child Process]
        BitNetManager -->|Drop Handler| DropGuard[Atomic SIGKILL / child.kill()]
    end

    subgraph GPU Execution Context (CUDA v13.3 / MSVC 14.51)
        LlamaEngine -->|Zero-Copy / mmap| CudaKernels[NVIDIA dGPU - RTX 2060m]
    end

    subgraph CPU Isolated Process Context
        TokioChild -->|IPC StdIn/StdOut| BitNetExe[bitnet_daemon.exe Subprocess]
        DropGuard -.->|SIGKILL on Drop| BitNetExe
    end
```

### 5.3. Agnosticismo de Hardware e Termodinâmica
- **Piso de Validação:** NVIDIA RTX 2060 Mobile (6GB VRAM) rodando CUDA 13.3 + MSVC 14.51.
- **Transmutabilidade:** O KV Cache assimétrico esmaga a alocação de VRAM $< 1.0 \text{ GB}$, permitindo que modelos GGUF coexistam com sidecars de CPU sem causar OOM.
- **Isolamento de CPU:** O `bitnet_daemon.exe` roda isolado na CPU e sofre destruição imediata e atômica quando desalocado.
