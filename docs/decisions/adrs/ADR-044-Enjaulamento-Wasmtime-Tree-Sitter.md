---
id: "ADR-044"
title: "ADR-044-Enjaulamento-Wasmtime-Tree-Sitter"
version: 1.0
status: Aprovado
epic: "Cognicao / Enjaulamento WASM (Call Graph & Symbol Index)"
amends: ["ADR-029", "ADR-040", "ADR-041", "ADR-042", "ADR-043"]
description: "Marco 3.8 Fase C.2: institui o Sandbox Wasmtime WASI 0.2 como cerca perimetrica para parsers C do tree-sitter, estabelece o SYMBOL_INDEX e CALL_GRAPH em DashMap RAM Host e quita a divida dos stubs symbol/callers/callees do gateway MCP."
mathematical_anchors: ["wasm_fuel_metering", "dashmap_constant_time_lookup", "BFS_callgraph_transposto"]
physical_paths: ["src-tauri\\src\\cognition\\observability\\wasm_engine.rs", "src-tauri\\src\\cognition\\observability\\call_graph.rs", "src-tauri\\src\\cognition\\observability\\mpsc_telemetry.rs", "resources\\wasm_grammars\\"]
pr: "https://github.com/brunosrosa/souls_mc/pull/20"
test_coverage: "38/38 verdes em <0.1s (Fast Pass Marcha Rapida)"
---

# ADR-044: Enjaulamento Wasmtime do Tree-Sitter e Indice de Símbolos em RAM

## Status

**Aprovado (Ativo, Inegociável e Fundacional para o SOULS V4).** Emenda cumulativa das ADRs 029 (Visão Cognitiva O(1)), 040 (State DB v2), 041 (Servername Soberano `souls_mcp`), 042 (CCR Conveyor Belt) e 043 (Observabilidade Cognitiva Sensorial). Homologado pelo Arquiteto-Chefe em 2026-08-02 após laudo técnico de 38/38 testes verdes (35 baseline + 3 novos da Fase C.2).

## Contexto Técnico e Desafio Operacional

O gateway `souls_mcp` carrega três ferramentas canônicas — `symbol`, `callers` e `callees` — declaradas no `tools/list` desde o Marco 3.5, mas implementadas como stubs `not_implemented_yet`. Esses stubs constituem **falsos verdes** documentais: o cliente MCP (Trae IDE, Claude Desktop) vê o tool no `tools/list`, chama-o, recebe um `is_error: true` com payload stringificado e desperdiça tokens de round-trip. A causa-raiz histórica foi a **covardia arquitetural** de incorporar o `tree-sitter` C nativo diretamente no binário Rust do gateway, contaminando o runtime do Tokio com:

1. **Segfaults não-determinísticos.** Parsers C do tree-sitter carregam estado mutável em `static mut` herdado do upstream; sob concorrência do Tokio, qualquer `panic!` interno vira `SIGSEGV` no gateway.
2. **Loops infinitos silenciosos.** Gramáticas malformadas ou entradas patológicas (Rust macro expansion, C++ templates) podem levar o parser a consumir 100% da CPU sem retornar, monopolizando um worker thread do Tokio.
3. **Footprint de memória ilimitado.** O `tree-sitter` aloca o AST completo na heap sem teto físico; arquivos de 5MB viram árvores de 200MB+ em `bumpalo`, sufocando a dGPU RTX 2060m durante o `malloc` no Tauri.

A solução canônica adotada por `bun`, `fastly` e `wasmtime` em produção é **enjaular os parsers em WebAssembly com WASI Preview 2** e impor uma cerca de recursos físicos (memory cap + fuel metering) que mata o guest em O(1) ao violar qualquer limite.

## Decisões de Engenharia e Arquitetura

Fica decretada a implementação do **Enjaulamento Wasmtime do Tree-Sitter** sob as seguintes leis e equações imutáveis:

### 1. Sandbox Wasmtime com WASI Preview 2 e Cerca de Recursos Físicos

* **Runtime Estanque.** O `wasmtime::Engine` é configurado uma única vez por processo (`OnceLock<Engine>` em `wasm_engine.rs`) com as flags defensivas:
  - `wasm_component_model(true)` — ativa o component model WIT/WASI 0.2.
  - `consume_fuel(true)` — habilita o fuel metering (cada instrução WASM consome 1 unidade de fuel; o host interrompe quando o contador zera).
  - `epoch_interruption(true)` — segunda linha de defesa contra loops infinitos não-financeiros.
  - **Teto de memória linear:** 16 MB (`Store::limiter(|_| 16 * 1024 * 1024)`). Gramáticas tree-sitter nunca excedem 8MB; o teto de 16MB tem folga 2x para entradas grandes sem permitir exaustão.
  - **Teto de fuel:** 10.000.000 unidades por chamada (`Store::set_fuel(10_000_000)`). Gramática típica consome ~50K; o teto 10M tem folga 200x.

* **Carregamento Dinâmico de Gramáticas.** A função `WasmEngine::load_grammar(&self, bytes: &[u8]) -> Result<Module, WasmTrap>` aceita bytes WASM arbitrários (incluindo WAT compilado em runtime) e devolve um `Module` pré-compilado pelo Cranelift. Gramáticas estáticas (Rust, C, Python) vivem em `resources/wasm_grammars/*.wasm` via `include_bytes!`; gramáticas dinâmicas podem ser injetadas via MPSC para suportar extensões de usuário.

* **Tratamento Gracioso de Traps.** O método `WasmEngine::execute_safely<F, T>(&self, module: &Module, f: F) -> Result<T, WasmTrap>` envolve a invocação do guest em `std::panic::catch_unwind` (Rust-side panic catcher) **e** em pattern matching sobre `anyhow::Error` (Wasm-side trap matcher). Qualquer estouro de memória, divisão por zero, `unreachable` ou exaustão de fuel retorna `WasmTrap::StructuredFailure { reason, fuel_consumed }` em vez de derrubar a thread do Tokio. O `Store` é descartado imediatamente após o erro (RAII garante liberação de todas as páginas lineares).

### 2. SYMBOL_INDEX e CALL_GRAPH em DashMap de RAM Host

* **Índice de Símbolos O(1).** A estrutura `SYMBOL_INDEX: OnceLock<DashMap<String, SymbolEntry>>` mapeia cada símbolo sintático (função, struct, enum, trait) à sua localização física `(file:line)`. O tipo de entrada é:

```rust
pub struct SymbolEntry {
    pub qualified_name: String,    // "crate::module::Type::method"
    pub kind: SymbolKind,          // Fn | Struct | Enum | Trait | Const | Static
    pub file_path: PathBuf,        // caminho canonicalizado
    pub line: u32,                 // linha 1-based
    pub column: u32,               // coluna 0-based
}
```

A lookup `SYMBOL_INDEX.get(name)` resolve em **O(1) médio** via hashmap segmentado do `DashMap` (lock-free read path). Para a ferramenta `symbol`, a busca é direta: a query FTS5 do `mem_search` vira `DashMap::get` quando o nome é exato; FTS5 só é invocado para nomes parciais.

* **Grafo de Chamadas Direcionado.** A estrutura `CALL_GRAPH: OnceLock<DashMap<String, CallGraphNode>>` armazena dois DashMaps simétricos (`callees_map` e `callers_map`) para responder ambas as direções da relação `f → g` (f chama g) em O(1) sem BFS no caminho de cache hit. Cada nó:

```rust
pub struct CallGraphNode {
    pub symbol: String,            // qualified_name
    pub adjacents: HashSet<String>, // conjunto de arestas adjacentes
    pub last_updated: i64,         // epoch seconds (Langevin hook)
}
```

* **Custo de Memória.** Para um monorepo de 50K símbolos com fan-out médio de 8 chamadas, o footprint estimado é ~50K * 200 bytes = 10MB. Cabe folgadamente nos 16MB de memória linear do Wasm sandbox e na heap Host (RTX 2060m tem 16GB RAM sistema).

### 3. Telemetria MPSC Assíncrona (Fire-and-Forget)

* **Canal MPSC Dedicado.** A função `spawn_callgraph_telemetry_worker(symbols_tx, callers_tx)` cria um canal `tokio::sync::mpsc::channel::<TelemetryEvent>(256)` bufferizado. Os eventos são:

```rust
pub enum TelemetryEvent {
    FileMutated { path: PathBuf, content: String },
    FileDeleted { path: PathBuf },
}
```

* **Worker em Background.** O consumer roda em `std::thread::spawn` (NÃO `tokio::spawn`) para manter o `tree-sitter` síncrono isolado do event loop, espelhando o padrão de `mpsc_bridge.rs` do `souls_graph`. O ciclo:
  1. `recv()` bloqueia esperando próximo evento.
  2. Parseia o arquivo com o grammar WASM enjaulado.
  3. Extrai `(name, kind, line, column)` por símbolo.
  4. Atualiza `SYMBOL_INDEX` (insert or replace).
  5. Extrai `caller → callee` edges.
  6. Atualiza `CALL_GRAPH` (insert or replace com merge de `HashSet`).

* **HIPER-FORWARD.** As tools `read`, `edit` e `write` chamam `try_send` (não `send`) — se o canal estiver cheio (256 eventos enfileirados = workspace sob write storm), o evento é descartado silenciosamente e uma métrica é emitida em `tracing::warn!`. O critical path do tool nunca bloqueia.

### 4. Registro Canônico das 3 Tools e Remoção de Prefixo Redundante

* **Lei 32/120 (ADR-041).** As três novas tools são registradas no `tools/list` com nome curto e descrição ≤120 chars:

```json
{"name": "symbol",  "description": "Resolve a localização física (file:line) de símbolos sintáticos da AST do monorepo em O(1)."},
{"name": "callers", "description": "Lista os nós do grafo de dependências que invocam um determinado símbolo no workspace."},
{"name": "callees", "description": "Mapeia quais funções e structs são consumidos internamente pelo símbolo interrogado."}
```

* **Aliases Unificados.** O dispatcher aceita as 3 formas canônicas para máxima compatibilidade durante a transição:

```rust
"symbol"  | "souls_symbol"  | "ctx_symbol"  => run_symbol(params).await,
"callers" | "souls_callers" | "ctx_callers" => run_callers(params).await,
"callees" | "souls_callees" | "ctx_callees" => run_callees(params).await,
```

* **Sem Prefixo no tools/list.** Conforme ADR-041 §3 (canibalização preservada), os aliases `souls_*` e `ctx_*` continuam aceitos no dispatcher, mas o `name` exposto é o curto. O teste `tools_list_respects_32_120_tetos` (já existente) valida a invariante em runtime.

### 5. Agnosticismo de Hardware e Transmutabilidade

* **Piso de Validação:** RTX 2060m (6GB VRAM, AVX2, 16GB RAM).
* **Teto Agnóstico:** O motor Wasmtime é CPU-puro; roda identicamente em Metal/Vulkan/NPU desde que o host Rust seja recompilado. Nenhuma instrução CUDA, Metal ou Vulkan é emitida — o sandbox é 100% bare-metal.
* **Sandbox Tripartite (Landlock/AppContainer).** Em produção, o `Store` é executado sob Landlock (Linux) ou AppContainer (Windows) com whitelist explícita de paths (apenas `resources/wasm_grammars/` é read-only; o resto do filesystem é inacessível ao guest).

## Equações de Validação

### Cerca de Combustível (Fuel Metering)

$$
\text{Fuel}_{\max} = 10^7, \quad \text{Mem}_{\max} = 16 \text{ MiB}
$$

Se o guest consumir todo o fuel antes de retornar, o `Store` é descartado e a função retorna `WasmTrap::FuelExhausted { consumed: 10^7 }`. A complexidade de detecção é $\mathcal{O}(1)$ (single atomic decrement por instrução WASM).

### Complexidade de Lookup

$$
T_{\text{symbol}}(n) = \mathcal{O}(1) \text{ médio via DashMap hash}
$$

$$
T_{\text{callers}}(n) = \mathcal{O}(d) \text{ onde } d = |\text{adjacents}|
$$

## Testes de Homologação (TDD Fast Pass < 0.1s)

| # | Teste | Validação |
|---|-------|-----------|
| 1 | `test_wasm_tree_sitter_isolation` | Wasm guest com `unreachable` ou divisão por zero é interceptado; retorna `WasmTrap::StructuredFailure`; thread Tokio sobrevive. |
| 2 | `test_symbol_resolution_o1` | Insere 10K símbolos em `SYMBOL_INDEX`; `symbol` resolve cada um em <10μs (caminho de cache hit DashMap). |
| 3 | `test_callers_callees_graph` | Popula `CALL_GRAPH` com 5 nós e 8 arestas; `callers(X)` retorna o conjunto exato de chamadores; `callees(X)` retorna o conjunto exato de chamados. |

## Consequências

### Positivas

* **Quitação de dívida técnica.** Os 3 stubs `symbol`/`callers`/`callees` deixam de ser falsos verdes.
* **Segurança operacional.** Parser tree-sitter não pode mais derrubar o gateway; qualquer crash é contido em `WasmTrap`.
* **Performance preditível.** Telemetry fuel/mem determinística permite cálculo de custo FinOps por chamada WASM.
* **Padrão replicável.** O sandbox Wasmtime torna-se o template para futuras integrações de parsers (Python, Markdown, SQL) sob a mesma cerca.

### Negativas (Aceitas sob Pessimismo da Razão)

* **Overhead de inicialização do Engine.** ~5ms na primeira chamada (Cranelift JIT). Mitigado por `OnceLock<Engine>` que compartilha a instância entre todos os workers.
* **Impossibilidade de chamar APIs do Host diretamente.** O guest é estanque; qualquer I/O (file read, network) passa por host functions explicitamente registradas. Aceitável — tree-sitter é CPU-puro.
* **Footprint de ~10MB para o índice de símbolos.** Aceitável no Teto de 6GB VRAM e nos 16GB de RAM sistema.

## Referências Cruzadas

- **ADR-029** (Visão Cognitiva O(1)) — O SYMBOL_INDEX é a concretização RAM Host do lookup O(1) antes delegado ao FTS5 SQLite.
- **ADR-041** (Servername Soberano) — Teto 32/120 para nomes/descrições.
- **ADR-043** (Observabilidade Sensorial) — Langevin decay reaproveitado para `last_updated` no CALL_GRAPH.
- **Marco 3.7 Fase B** — `observability::routes` e `observability::impact` continuam usando regex puro (não-WASM); a Fase C.2 introduz a cerca Wasmtime para gramáticas estruturadas.
