# PRD: Higiene Física de I/O, Controle de WalkDir e Transmutação do `souls_dedup` (SOULS V4)

---

## 1. OBJETIVO DO DOCUMENTO
Este documento estabelece as especificações de requisitos físicos e de engenharia para sanear o motor local de inferência do **Souls MC (SOULS V4)** na branch `feature/souls-rebranding-and-state-db` (ou correlata). 

O foco é eliminar o desperdício de I/O de disco (dupla leitura GGUF), blindar o Event Loop contra congelamento por varreduras recursivas profundas e restaurar a sintonização do nosso "Saco a Vácuo" (`lean_vacuum`) ativando os stubs de compressão e deduplicação de tokens.

---

## 2. ESPECIFICAÇÕES TÉCNICAS

### 2.1. Trava de Profundidade do `WalkDir` (`max_depth = 5`)
Fica proibido o uso de varreduras recursivas cegas sem limite de profundidade. A varredura contínua de diretórios de modelos locais de Inteligência Artificial deve ser controlada de forma estrita para evitar travamentos do Tokio causados por symlinks circulares ou diretórios infinitos.

*   **Arquivos Alvos de Modificação:**
    1.  `src-tauri/src/core/model_registry.rs` (na função `sync_local_models_to_registry`)
    2.  `src-tauri/src/core/model_registry.rs` (na função `collect_local_models`)
    3.  `src-tauri/src/bin/scan_local_models_cli.rs` (no utilitário de varredura via linha de comando)
*   **Regra de Implementação:** 
    *   Injetar explicitamente o limite `.max_depth(5)` imediatamente após o construtor `WalkDir::new(...)` em todas as rotas listadas.

---

### 2.2. Erradicação de Duplo `mmap` e Cache de Metadados GGUF
A função `run_inference` em `llama_engine.rs` está atualmente cometendo um pecado grave de I/O: chamando `parse_gguf_metadata_zero_copy` de forma redundante em duas etapas sucessivas (L112 e L132), abrindo e mapeando em memória virtual o mesmo arquivo de pesos GGUF de múltiplos gigabytes repetidamente.

*   **Arquivos Alvos de Modificação:**
    *   `src-tauri/src/core/llama_engine.rs`
*   **Regra de Implementação:**
    *   A inferência não deve abrir fisicamente o arquivo `.gguf` no disco para extrair parâmetros estáticos de modelagem (`context_length`, `head_count_kv`, `family`) a cada nova requisição.
    *   **A Cura:** No momento em que o modelo é carregado, os metadados extraídos pelo Local Model Manager e persistidos estritamente no SQLite SSOT (`souls_state.db`) devem ser interrogados ou mantidos em cache de RAM (L1 Transiente) dentro do estado de execução do `LlamaEngine`.
    *   Remover a chamada dupla e asfixiante de `parse_gguf_metadata_zero_copy` na fase crítica de inferência quente, transformando a busca em tempo constante real $\mathcal{O}(1)$.

---

### 2.3. Integração do `souls_compress`
A ferramenta MCP `souls_compress` está exposta como um stub ocioso que não entrega valor.

*   **Arquivos Alvos de Modificação:**
    *   `src-tauri/src/bin/souls_mcp_server.rs`
*   **Regra de Implementação:**
    *   Conectar fisicamente a ferramenta ao nosso módulo de compressão `lean_vacuum::compress_to_lean` implementado em `src/cognition/lean_vacuum/mod.rs`, removendo a casca de stub anterior.

---

### 2.4. Transmutação de `souls_dedup` (O Dedup de 5 Linhas)
Implementar de forma nativa e soberana o algoritmo de deduplicação por blocos de 5 linhas consecutivas para poupar o envio de contextos idênticos ao LLM.

*   **Novos Arquivos a Criar:**
    *   `src-tauri/src/cognition/lean_vacuum/dedup.rs`
*   **Regra de Algoritmo:**
    *   O decodificador analisa o buffer de entrada de múltiplos arquivos lidos na sessão.
    *   Ele divide o conteúdo em blocos deslizantes de 5 linhas consecutivas.
    *   Caso um bloco de 5 linhas consecutivas se repita exatamente em arquivos ou seções distintas, a duplicata é suprimida do contexto que será despachado ao LLM, sendo substituída por uma referência curta de ponteiro semântico.
    *   A performance de dedup deve operar de forma *Zero-Copy* utilizando referências de lifetimes de Rust (`&'a str`) sobre os buffers na RAM.

---

## 3. VALIDAÇÃO E SUÍTE DE TESTES (TDD)
O agente deve implementar testes unitários explícitos para assegurar que nenhuma alteração fure a nossa esteira de confiabilidade:

1.  `test_walkdir_max_depth_enforced`: Garante que o scanner não avança além de 5 níveis de pastas sob estresse.
2.  `test_gguf_cached_metadata_reads`: Comprova que a inferência do motor lê os parâmetros de modelagem a partir da RAM/SQLite e executa exatos zero mapeamentos redundantes de arquivos GGUF em runtime.
3.  `test_mcp_compress_active_pipeline`: Testa o roteamento real do comando `souls_compress` batendo no vacuum em memória e retornando o buffer LEAN compactado.
4.  `test_vacuum_dedup_sliding_blocks`: Alimenta o dedup com strings contendo blocos duplicados de 5 linhas consecutivas e valida a remoção física de duplicatas com substituição semântica zero-copy.

---

## 4. CRITÉRIOS DE ACEITE (DoD)
*   Compilação limpa via `cargo check --bin souls_mcp_server` com **EXIT CODE 0** e zero warnings.
*   Suíte de testes estendida para abranger as novas validações de I/O com sucesso absoluto.
