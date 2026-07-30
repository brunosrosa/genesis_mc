# PRD-004: Refinamento e Purificação das Garras de Contexto (lean_vacuum)
**Status:** ATIVO / REFINAMENTO (Rota A)  
**Escopo:** Souls MC - Unificação e Extermínio de Slop no `lean_vacuum`  
**Data:** 2026-07-30  

---

## 1. INTRODUÇÃO E ENTENDIMENTO DO GARGALO (O FIM DO SLOP)

Sob as leis estritas do **SODA V4** (operando sob as diretrizes do SODA Canon V5), o principal papel do subsistema de contexto (**`lean_vacuum`**) é garantir a desidratação e a imunização da janela de contexto da nossa GPU (RTX 2060m com limite estrito de 6GB de VRAM).

A auditoria forense do último ciclo revelou que a IDE, guiada pelo otimismo estocástico (vibe coding), implementou a ferramenta `souls_dedup` de forma preguiçosa: um mero deduplicador intra-arquivo que limpa linhas adjacentes em um único buffer. **...** Os agentes de desenvolvimento multiplicam redundâncias lógicas gigantescas ao abrirem múltiplos arquivos do projeto na mesma sessão (ex: duplicando structs, imports, helpers e definições).

Este documento estabelece o contrato técnico definitivo para transmutar e ligar as três principais garras cognitivas de leitura do `lean_vacuum`, garantindo uma deflação de contexto de até **71%** na memória de trabalho da dGPU.

---

## 2. ESPECIFICAÇÃO DAS TRÊS GARRAS COGNITIVAS

### 2.1. `souls_dedup` (Deduplicação de Sessão Cross-File)

A ferramenta deve gerenciar e expurgar a redundância entre múltiplos arquivos lidos sequencialmente pelo agente de IA ao longo do ciclo de trabalho.

*   **O Core do Algoritmo (Zero-Clone):**
    *   O backend em Rust manterá um estado em memória RAM chamado **`SESSION_DEDUP_CACHE`** utilizando um mapa estático concorrente thread-safe (`DashMap<u64, (PathBuf, usize, usize)>`) que mapeia hashes de 64 bits para trios de `(Caminho, LinhaInicial, LinhaFinal)`.
    *   O cache é limpo a cada nova sessão ou via sinal explícito de reinicialização.
*   **O Fluxo de Processamento de Texto:**
    1.  Ao receber o texto de qualquer arquivo lido (ex: em `souls_read` ou `souls_multi_read`), o buffer é quebrado em linhas.
    2.  O algoritmo executa uma varredura deslizante em blocos de **exatamente 5 linhas consecutivas**.
    3.  Cada bloco de 5 linhas é normalizado: remoção de todos os espaços em branco, tabulações, quebras de linha e comentários descartáveis.
    4.  O hash de 64 bits do bloco normalizado é calculado utilizando a crate de altíssima performance e sem alocação de Heap **`rustc-hash` (FxHash)**.
    5.  O sistema interroga o `SESSION_DEDUP_CACHE`:
        *   **SE HIT (Colisão de Hash de arquivo DIFERENTE):** O trecho correspondente de 5 linhas é sumariamente deletado do buffer final de saída de RAM do SODA, sendo substituído por uma única linha compacta de marcador semântico:
            `// [dedup: 5 lines hidden. Duplicate of "src/core/model_registry.rs" lines L142-L146]`
        *   **SE MISS:** O hash é registrado com o caminho do arquivo atual e o índice de linhas físicas correspondentes no cache global.
*   **Impacto de VRAM:** Reduz em até **40%** a redundância acumulada nos chats de desenvolvimento de múltiplos turnos.

### 2.2. `souls_smart_read` (Leitor Token-Aware com Auto-Shrink)

A leitura ingênua de logs imensos ou arquivos de código de 3.000 linhas causa estouro imediato de VRAM e asfixia de atenção no LLM. O `souls_smart_read` atuará como o disjuntor de leitura consciente do orçamento.

*   **Parâmetros de Entrada:**
    *   `file_path`: Caminho canônico do arquivo.
    *   `max_tokens_budget`: Limite estrito de tokens permitido (default: 8.000 tokens).
*   **A Mecânica de Proteção Termodinâmica:**
    1.  O sistema executa a medição de tokens síncrona na CPU em <1ms utilizando o tokenizador local (`tiktoken` com encoding `cl100k_base`).
    2.  **SE tokens <= `max_tokens_budget`:** O arquivo é retornado em formato limpo.
    3.  **SE tokens > `max_tokens_budget`:** O leitor aciona recursivamente as seguintes defesas de desidratação em RAM:
        *   **Passo 1 (Poda de Prosa):** Executa `lean_vacuum::lightweight_cleanup` para expurgar comentários de bloco obeso (`/* ... */`), quebras de linha duplicadas sequenciais e logs ANSI poluídos.
        *   **Passo 2 (Poda Sintática de Funções):** Se ainda exceder o orçamento e for código compatível com o parser tree-sitter WASM, o sistema preserva apenas o outline do arquivo (estruturas, traits, impls, assinaturas de fns) e amputa/comprime os corpos funcionais das funções menos acessadas do histórico, forçando o arquivo a caber cirurgicamente dentro do orçamento exigido.
    4.  Se após todos os passos de desidratação o arquivo permanecer maior que o orçamento, o sistema aplica o comportamento **Fail-Closed** retornando o erro RPC `-32010` (Context Budget Exceeded).

### 2.3. `souls_search` (A Busca Compactada no Padrão LEAN)

Ferramentas tradicionais de busca (como grep) cospem dumps massivos de caminhos e textos de log repetitivos no chat, gerando colapso de contexto. O novo `souls_search` unifica a busca textual mas a formata sob a Notação Adaptativa Eficiente (LEAN).

*   **A Notação Compacta LEAN (Deflação de 71% de tokens):**
    *   A busca por expressão regular (regex) varre os arquivos locais de forma assíncrona.
    *   O output gerado é estruturado agrupando os achados sob o cabeçalho único do arquivo e achatando as ocorrências de linha em coordenadas sequenciais compactas, sem repetir o caminho do arquivo a cada linha de correspondência.
    *   *Exemplo de formato tradicional (ruído):*
        ```text
        /src-tauri/src/bin/souls_mcp_server.rs:42: fn run_inference() {
        /src-tauri/src/bin/souls_mcp_server.rs:112: let gguf_meta = parse_gguf_metadata_zero_copy(model_path);
        /src-tauri/src/bin/souls_mcp_server.rs:132: let meta = gguf_meta.clone();
        ```
    *   *Exemplo de formato compactado LEAN (ouro):*
        ```text
        @src-tauri/src/bin/souls_mcp_server.rs
        L42: fn run_inference()
        L112, L132: parse_gguf_metadata_zero_copy / gguf_meta
        ```
    *   A compactação de trajetos e termos idênticos garante que a IA enxergue o panorama inteiro do código sem asfixiar a VRAM com redundâncias de strings.

---

## 3. COBERTURA TDD MANDATÓRIA (CRITÉRIOS DE ACEITAÇÃO)

A esteira de testes automatizada do `souls_mcp_server` e da biblioteca `souls_mc` deve conter e comprovar estritamente:

1.  **`test_cross_file_deduplication_successful`**: Prova que ao lermos dois arquivos diferentes contendo o mesmo bloco de 5 linhas lógicas consecutivas, o segundo arquivo sofre a amputação cirúrgica de RAM, exibindo o marcador de dedup apontando o local correto do primeiro arquivo (incluindo caminho e intervalo de linhas).
2.  **`test_smart_read_budget_enforcement`**: Prova que arquivos que excedem o limite de tokens sofrem o processo de limpeza e desidratação, reduzindo sua contagem de tokens de forma comprovável antes de atingir o limite limite de OOM.
3.  **`test_search_lean_notation_formatting`**: Prova que a busca por regex retorna resultados agrupados no formato LEAN, reduzindo em mais de 60% os bytes gerados em comparação com a saída tradicional de varredura.

---

## 4. PROIBIÇÕES TÓXICAS GERAIS (HARD LAWS SODA)

*   **PROIBIDA** qualquer alocação de memória de vídeo (VRAM) para realizar deduplicação, contagem de tokens ou compactação LEAN. Todo o Hot-Path roda estritamente na CPU (RAM do Host).
*   **PROIBIDO** o uso de clones desnecessários de strings grandes no Heap central. O processamento deslizante de linhas em `souls_dedup` e `souls_search` deve operar estritamente sobre fatias de memória estável e lifetimes de Rust (`&'a str`), minimizando a latência.
*   **PROIBIDA** a permanência de qualquer stub `not_implemented_yet` para as ferramentas `souls_compress` e `souls_dedup`. Todas as pontas devem ser devidamente soldadas e compiladas.

---
**FIM DO PRD-004**
