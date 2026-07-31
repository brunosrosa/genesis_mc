# PRD-002: CLUSTER 2 — EXPLORAÇÃO E TOPOLOGIA AST (OS OLHOS DO AGENTE)
**Status:** PROPOSED (Sob Revisão do Arquiteto-Chefe)  
**Versão:** 1.0 (Alinhada à Arquitetura SOULS V4 / Cânone V5)  
**Épico Relacionado:** Épico 18 (Criação de Lentes de Exploração e Prevenção de Segment Faults de C-FFI)  

---

## 1. INTRODUÇÃO E PROPÓSITO COGNITIVO
O **Cluster 2 (Os Olhos do Agente)** introduz a infraestrutura de telemetria visual e topológica estrutural do **Souls MC**. Se o Cluster 1 dotou a máquina de "Mãos" cirúrgicas (`souls_edit`, `souls_fill`), o Cluster 2 estabelece as "Lentes" de exploração de altíssima fidelidade. 

O objetivo termodinâmico deste cluster é **eliminar a leitura sequencial desnecessária de arquivos (I/O sínclono redundante)** e a **asfixia do contexto de tokens** de LLM. Em vez de ler arquivos inteiros para mapear símbolos, o SOULS passará a enxergar a topologia do workspace de forma matemática, em tempo constante $O(1)$ e com compressão ativa na RAM.

---

## 2. A DUPLA LEITURA DE REQUISITOS (SOULS DUALITY)

### 2.1. Visão Declarada (O que a IA "vende")
O sistema possui ferramentas rápidas para mapear a árvore de arquivos do projeto e extrair assinaturas lógicas (outlines) de códigos sem requerer que o usuário abra manualmente os arquivos na IDE.

### 2.2. Abstração Estrutural (A Física do Sistema)
O Cluster 2 é uma barreira de segurança e performance que executa varreduras de arquivos usando **Zero-Copy String Slicing** (`&'a str` na RAM do Host) e isolamento militar contra falhas de segmentação (**Segmentation Faults**) causadas por bibliotecas legadas de *tree-sitter* em C. 

O parser de assinaturas funcionará encapsulado em um módulo **WebAssembly (Wasmtime WASI 0.3)** embutido na RAM do nosso daemon, garantindo que o estouro de pilha de um parser instável de terceiros morra em uma "jaula" isolada sem derrubar o Event Loop principal do Tokio.

---

## 3. AS DUAS FERRAMENTAS DO CLUSTER 2

### 3.1. Tool: `souls_tree` (A Lente de Diretórios Eficiente)
Substitui varreduras lentas ou saídas massivas de árvores de diretórios tradicionais por uma notação altamente comprimida baseada em **Dot-Flattening** e regras rígidas de exclusão de lixo.

*   **Entradas:**
    *   `file_path` (TEXT - Opcional, padrão: raiz do workspace).
    *   `depth` (INTEGER - Opcional, padrão: 3).
*   **Comportamento Físico:**
    *   Lê a árvore de diretórios de forma não-bloqueante (`tokio::fs::read_dir`).
    *   Filtra e ignora compulsoriamente os caminhos e arquivos descritos no `.gitignore` [121].
    *   Ignora pastas proibidas ou de alto fardo térmico (`target/`, `node_modules/`, `.git/`, `.souls_cache/`, `.souls_data/`, `.cargo/`).
    *   Aplica **Dot-Flattening** se um diretório possuir apenas um subdiretório (ex: `src-tauri/third_party/lean-ctx/src/core/` -> compacta em uma única linha no output para economizar tokens).
*   **Cenário de Falha:** 
    *   Tentativa de travessia fora da raiz (bloqueado pelo Firewall de Caminhos com erro RPC `-32015`).

### 3.2. Tool: `souls_outline` / `souls_symbol` (A Lente Estrutural AST)
Extrai cirurgicamente a assinatura lógica de arquivos de código (Rust, Python, Svelte/JS/TS) e de configuração, removendo corpos de funções e deixando apenas a topologia semântica do arquivo.

*   **Entradas:**
    *   `file_path` (TEXT - Caminho absoluto ou relativo à raiz).
*   **Comportamento Físico:**
    *   Abre o arquivo em RAM e intercepta o buffer de texto.
    *   Carrega a gramática da linguagem correspondente pré-compilada em WebAssembly.
    *   Executa o parsing de nós (estruturas, funções, métodos, enums, assinaturas de traits e blocos `impl`).
    *   Cospe na saída o outline formatado in JSON contendo apenas o esqueleto estrutural (sem o miolo executável das funções), reduzindo em até **88%** o volume de tokens gerados em relação ao arquivo de código original.
*   **Cenário de Falha (Fail-Closed):**
    *   Se o arquivo estiver malformado sintaticamente, retorna o erro RPC `-32021` ("Falha sintática ao parsear o outline do arquivo").
    *   Se o parser embutido em WASM disparar um trap (pânico interno ou estouro), o Rust captura a falha, isola o erro na jaula WASM, retorna o erro RPC `-32022` e o Tokio continua rodando normalmente.

---

## 4. O DESIGN DA JAULA WASMTIME (WASI 0.3)
Para realizar o parsing de AST sem expor o daemon a Segmentation Faults nativos das bibliotecas em C que residem no coração das gramáticas do tree-sitter:

1.  **A Compilação do Parser para WASM:**
    As gramáticas do tree-sitter (ex: C, Rust, JS) são compiladas com alvo `wasm32-wasi`.
2.  **O Link de Execução:**
    O `souls_mcp_server` abre o interpretador `wasmtime` na inicialização do sistema, pré-carregando os módulos binários `.wasm`.
3.  **A Passagem de Buffer:**
    A string do arquivo de código é enviada para a memória linear isolada da máquina virtual WASM.
4.  **A Captura do Trap:**
    ```rust
    // Esqueleto Teórico da Captura do Trap no Backend Rust
    let result = linker.get_default(&mut store, "")?
        .typed::<(), ()>(&store)?
        .call(&mut store, ());
        
    if let Err(trap) = result {
        // O parser C-FFI quebrou, mas a jaula WASM capturou.
        // O daemon Souls MC permanece intocado e de pé.
        return Err(RpcError::from_trap(trap));
    }
    ```

---

## 5. TDD E VERIFICAÇÃO UNITÁRIA REQUERIDA (OS TESTES DE FOGO)

A entrega deste cluster só será declarada verde se atingir **Exit Code 0** com zero warnings no Clippy e aprovar os seguintes testes unitários em `souls_mcp_server.rs`:

1.  `test_tree_flattening_successful`:
    Garante que subpastas lineares vazias sejam compactadas usando dot-flattening no output para economizar tokens.
2.  `test_tree_ignores_toxic_paths`:
    Valida que pastas como `node_modules` ou `target` são completamente invisíveis para a varredura do `souls_tree`.
3.  `test_outline_rust_signatures`:
    Garante que passar um arquivo `.rs` de teste ao `souls_outline` retorne as estruturas e impls sem expor o conteúdo interno das funções.
4.  `test_wasm_sandbox_trap_containment`:
    Simula uma quebra/pânico violento dentro do parser WASM e valida que o thread do Tokio intercepta o erro graciosamente sem causar pânico no processo pai.

---

## 6. CRITÉRIOS DE ACEITE (DEFINITION OF DONE - DoD)
*   [ ] O código compila na branch `feature/souls-ast-eyes` sem warnings no clippy.
*   [ ] As ferramentas `souls_tree` e `souls_outline` estão registradas no barramento MCP.
*   [ ] O tempo de resposta de ambas as ferramentas para arquivos médios (< 5.000 lines) é inferior a **1.0 milissegundo** na CPU Host.
*   [ ] A jaula Wasmtime isola erros de compilação sem deixar threads zumbis ativas.
