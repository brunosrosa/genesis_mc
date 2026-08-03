# Product Requirements Document (PRD) - Milestone 2.5: AST Parser Universalization

**Status do Documento:** Aprovado e Selado  
**Objetivo:** Universalizar a extração sintática de outlines de código (Fase 0) do Souls MC sem introduzir dependências de compiladores externos no host, sem inchar o Cargo.toml com crates C-FFI e eliminando de vez o risco de falhas de segmentação (segfaults) no processo assíncrono principal (Tokio).

---

## 1. Contexto & Diagnóstico de Baixo Nível

A nossa auditoria forense do repositório revelou um "furo mecânico" crítico: o arquivo `ast_parser.rs` realizava fallback silencioso para expressões regulares simples para a quase totalidade das linguagens suportadas (Rust, TypeScript, Python, Go, Elixir) [user query]. A única linguagem com suporte real ao Tree-Sitter era o C# (`tree-sitter-c-sharp`), cuja gramática em C era compilada nativamente [user query].

### Por que Expressões Regulares são Tóxicas para Parsing?
1. **Fragilidade Sintática:** Se o desenvolvedor declarar strings ou comentários contendo chaves `{}` ou aspas escapadas, a expressão regular sofrerá quebras catastróficas de escopo, truncando a assinatura e gerando dados corrompidos para a nossa Lente B de Arquitetura.
2. **Degradação de RAG:** Chaves cortadas e assinaturas incompletas poluem a nossa planilha de Blueprint, fazendo com que as IAs das Fases 2 e 3 tomem decisões baseadas em falsas estruturas de código.

### Por que a Compilação Estática de Wrappers C-FFI é Inviável?
Importar crates como `tree-sitter-rust` ou `tree-sitter-typescript` diretamente no Rust força a compilação de código C bruto no host. Isso:
- Aumenta o tempo de build em minutos.
- Exige ferramentas de compilação adicionais (Clang, GCC, MSVC) instaladas e expostas no ambiente de execução.
- **Risco Letal de Segfaults:** Se uma gramática em C falhar (por estouro de pilha ou ponteiro nulo ao tentar ler arquivos com sintaxe malformada), o pânico ocorrerá na camada C-FFI não segura, derrubando imediatamente o Daemon do Souls MC, sem possibilidade de recuperação pelo Tokio.

---

## 2. Decisões de Arquitetura e Leis Físicas (ADR-018-v2)

Para sanar essas deficiências de forma bare-metal e local-first, o Milestone 2.5 impõe duas rotas de alta fidelidade:

### 2.1. Sandboxing de Gramáticas via WebAssembly (`wasmtime` = 29.0.0)
Em vez de compilar as gramáticas do Tree-Sitter estaticamente no Rust, o Souls MC adota o **enjaulamento em WebAssembly**:
- Todas as gramáticas (Rust, Python, Go, Elixir) serão compiladas para WebAssembly como arquivos `.wasm` (ex: `tree-sitter-rust.wasm`) e armazenadas localmente em `.souls_data/wasm_grammars/` ou `src-tauri/resources/wasm_grammars/` [user query].
- O backend em Rust utilizará a crate `wasmtime` para ler esses arquivos binários e instanciar os motores de parsing de forma preguiçosa (Lazy Loading) e sob demanda.
- **Isolamento de Memória e CPU:** Cada parse de arquivo executará isolado dentro do sandbox do Wasmtime com limites de tempo de execução e memória. Se o bytecode da gramática C falhar por ponteiro corrompido ou trap, o Wasmtime capturará o erro e o retornará como um `wasmtime::Trap` amigável para o Rust. O Tokio interceptará a falha de forma graciosa, permitindo aplicar o fallback de texto limpo (`lean_vacuum`) sem derrubar o Daemon principal [user query].

### 2.2. Roteamento de Alta Velocidade JS/TS via OXC (= 0.120.0)
Para arquivos `.js`, `.jsx`, `.ts` e `.tsx`, não passaremos pelo gargalo de compilação ou carregamento do Wasmtime. Utilizaremos diretamente o compilador de alto rendimento **OXC** [user query]:
- O OXC opera utilizando alocação zero no Heap dinâmico, recorrendo a uma arena de alocação de alta performance baseada em *bump allocation* (`oxc::allocator::Allocator`).
- O parser de arquivos de frontend extrairá interfaces, tipos, classes, funções e arestas de importação na velocidade máxima da CPU AVX2, sem gerar "lixo de memória" [user query].

---

## 3. Especificação Técnica e Contratos de I/O

### 3.1. Roteamento no `ast_parser.rs`
A função principal `extract_structural_signatures` em `ast_parser.rs` deve obedecer ao seguinte mapeamento estrito:

| Extensão do Arquivo | Motor de Destino | Fallback de Falha |
|---|---|---|
| `.js`, `.ts`, `.jsx`, `.tsx` | `oxc::parser::Parser` (Zero-Copy) | Regex Fallback / `lean_vacuum` |
| `.cs` | Tree-Sitter Nativo (`tree-sitter-c-sharp`) | Regex Fallback / `lean_vacuum` |
| `.rs`, `.py`, `.go`, `.ex`, `.exs` | `wasmtime::Engine` (Tree-Sitter WASM) | Regex Fallback / `lean_vacuum` |
| Outras extensões | Regex Fallback (`lean_vacuum`) | N/A |

### 3.2. Estrutura de Retorno Unificada
Todas as funções internas de parsing devem retornar uma estrutura limpa compatível com as necessidades do Harvester e do MCP Server:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceOutline {
    pub language: String,
    pub signatures: Vec<String>,
    pub import_edges: Vec<String>,
    pub total_lines: usize,
    pub is_fallback: bool,
}
```

---

## 4. Resolução das Três Armadilhas de Baixo Nível (Invariantes de Engenharia)

### 🚨 Armadilha 1: Tempo de Vida do Allocator do OXC (Borrow Checker Trap)
O parser do OXC retorna nós da AST amarrados ao tempo de vida `'a` do `Allocator` da arena. 
- *A consequência:* Tentar retornar Strings geradas a partir do parsing para fora da função sem cloná-las fará o compilador do Rust rejeitar a compilação por violação de tempo de vida (*lifetime mismatch*).
- *A Cura:* O extrator OXC deve converter explicitamente todos os identificadores de nomes de funções, classes ou importações para Strings alocadas de forma independente (`String` ou `Arc<str>`) utilizando os métodos `.to_string()` ou `.into()` antes que o escopo do alocador seja destruído.

### 🚨 Armadilha 2: Overhead de Cold-Start na Compilação WASM
Ler bytes binários e compilar o bytecode `.wasm` para código nativo do processador via `wasmtime::Module::new(engine, wasm_bytes)` leva dezenas de milissegundos. Se fizermos isso a cada arquivo, destruiremos o desempenho bare-metal em repositórios massivos.
- *A Cura:* A struct `WasmtimeTreeSitterEngine` deve implementar um cache estático (utilizando `std::sync::OnceLock` ou uma estrutura global thread-safe como `Arc<DashMap>`) para reter as instâncias compiladas de `wasmtime::Module` de cada gramática em RAM. Cada parse subsequente reutilizará o módulo compilado aquecido, reduzindo a latência a frações de sub-milissegundo.

### 🚨 Armadilha 3: Thread-Safety (Send/Sync) do Wasmtime no Tokio
As instâncias de `wasmtime::Store` e motores de execução de bytecode WASM não são seguras para serem enviadas indiscriminadamente entre threads concorrentes se forem manipuladas de forma incorreta no Tokio assíncrono.
- *A Cura:* Todo o processo de instanciação do Wasmtime e parsing Tree-Sitter deve ocorrer encapsulado de forma sínclona em blocos de thread dedicados ou despachados através do pool isolado de threads do Tokio via `tokio::task::spawn_blocking`. Isso protege o Event Loop principal de interrupções e garante a compilação sem alertas de violação de `Send` ou `Sync`.

---

## 5. Plano de Verificação (TDD)

### 5.1. Testes Automatizados Mandatórios
O arquivo `ast_parser.rs` deve conter os seguintes testes de estresse em Rust:
1. `test_oxc_js_ts_outline`: Injetar código TypeScript complexo contendo interfaces genéricas e imports. Verificar se o OXC extraiu com perfeição o outline sem vazamentos ou erros de lifetime.
2. `test_wasm_tree_sitter_rust_outline`: Validar o carregamento da gramática `tree-sitter-rust.wasm`, ler um arquivo de teste Rust com structs e métodos, e asseverar a integridade das assinaturas coletadas.
3. `test_fail_soft_corrupted_wasm_grammar`: Tentar instanciar o parser passando um array de bytes corrompidos (lixo binário). Validar se o Wasmtime e o Tokio capturaram a trap de erro de forma totalmente segura e retornaram o fallback correto, mantendo a integridade do Daemon 100% livre de crashes.

### 5.2. Critérios de Aceitação (DoD)
- `cargo check --features "tauri-app,gateway_ccr,llama_backend"` cravando **Exit Code 0**.
- `cargo test --features "tauri-app,gateway_ccr,llama_backend"` aprovando todos os testes do projeto de forma limpa.
- `cargo clippy` executado sem alertas ou violações de regras estritas.
