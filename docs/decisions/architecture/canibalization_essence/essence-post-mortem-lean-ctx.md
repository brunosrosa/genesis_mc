# Autópsia e Extermínio do lean-ctx: O Laudo de Antropofagia do SOULS MC

## 1. Introdução e Contexto Histórico
A pasta `third_party/lean-ctx` representava o maior vestígio de infraestrutura herdada e acoplamento desnecessário no ecossistema do **Souls MC** [53, 61]. Embora tenha servido originalmente como fonte de inspiração conceitual para o gerenciamento de contexto, ela arrastou dependências obesificantes que asfixiavam a CPU Host e ameaçavam a integridade termodinâmica da RTX 2060m [207, 319, 323]. 

Em conformidade com a **Doutrina de Antropofagia Algorítmica (ADR-026)**, executamos a varredura completa do seu código-fonte, absorvemos sua essência matemática e lógica puras em Rust, e agora decretamos o extermínio físico de seus resquícios e dependências [99, 137, 211].

---

## 2. Inventário Físico do Expurgo
A auditoria cega do `lean-ctx` revelou uma estrutura que continha [687, 692]:
*   **42 arquivos `.rs`** de ferramentas no diretório `src/tools/` [687, 692].
*   **41 entradas** declaradas de ferramentas no `tool_defs/granular.rs` [687, 692].

Dessas 41 ferramentas declaradas, apenas uma fração possuía valor computacional real aplicável à nossa dGPU restrita e à reatividade do Svelte 5 no frontend do **Souls MC** [101, 107]. O restante consistia em "falsos atalhos" e lixo de sustentação [319, 323].

---

## 3. Matriz de Canibalização: O Destino de Cada Órgão

### A. Os Órgãos Transplantados e Aperfeiçoados (Ativos no Metal)
As ferramentas fundamentais do `lean-ctx` foram completamente reescritas como código nativo e seguro em Rust no nosso chassi [219]:

1.  **`ctx_read` ➔ `read`**: Transmutado de forma a suportar TOON + SymbolMap sem alocações extras de string [693].
2.  **`ctx_delta` ➔ `delta_diff`**: Myers diff estrutural nativo implementado via crate `similar`, com zero FFI [681, 693].
3.  **`ctx_tree` ➔ `tree`**: Lente de diretórios não-bloqueante baseada em `Dot-Flattening` estrito e exclusão ativa de caminhos tóxicos (como `node_modules/` e `target/`) [710].
4.  **`ctx_outline` ➔ `outline` / `symbol`**: Extração de assinaturas estruturais de classes e métodos sem corpos de funções, enjaulada com segurança no **Wasmtime (WASI 0.2)** para impedir segfaults do tree-sitter em C [118, 693].
5.  **`ctx_smart_read` ➔ `smart_read`**: Leitura de arquivo com auto-shrink adaptativo medindo preventivamente tokens na CPU via `tiktoken` (cl100k_base) [710].
6.  **`ctx_search` ➔ `search`**: Busca textual rápida por expressões regulares combinando compressão estrutural agrupada por arquivo [710].
7.  **`ctx_dedup` ➔ `dedup`**: Motor que compacta blocos duplicados de 5 linhas consecutivas por hashes curtos de deduplicação [709].
8.  **`ctx_compress` ➔ `compress`**: Compressor LEAN para podar comentários e ruídos de prosa, gerando deflação de prompt de até 71% [224, 709].
9.  **`ctx_edit` ➔ `edit`**: Mecanismo de busca e substituição por bloco com escrita atômica (`atomic-write-file`) e trava concorrente de caminhos de arquivos para eliminar o risco de corrupção de código [256].
10. **`ctx_fill` ➔ `souls_stub_fill`**: Rehidratador de stubs comprimidos [710].
11. **`ctx_handoff` e `ctx_knowledge` ➔ `handoff` / `knowledge`**: Sistemas de estado relacional e transferência de tarefas de subagentes salvos no **SOULS State (SQLite L2)** [230, 709, 710].

### B. Os "Falsos Verdes" Curados (Marco 3.9.1)
Durante a faxina e auditoria de stubs, identificamos ferramentas que mentiam no barramento MCP [689, 690]. A interface as declarava como stubs, mas o dispatcher invocava o código real [689, 690]. Foram limpos e documentados em 3.9.1:
*   **`multi_read`**: Agora chama corretamente o compressor CCR lossless [689].
*   **`shell`**: Desvia o comando de compilação síncrona para thread dedicada da CPU com pattern compression de logs, evitando asfixia do loop do Tokio [259, 266, 267].
*   **`symbol`**: Promovido a ferramenta canônica ativa rodando na DashMap de RAM [user query].

### C. A Poda do Lixo Tóxico (13 Crateras Descartadas)
Sob o crivo do **Pessimismo da Razão**, identificamos que **13 ferramentas do `lean-ctx` original eram lixo operacional redundante** [701]. Elas tentavam resolver na camada de infraestrutura problemas que o nosso barramento de Skills e o **Chyros Daemon** hoje cobrem com consumo zero de VRAM e zero overhead de rede [635, 700]. Elas foram oficialmente **deletadas do roadmap** e não serão canibalizadas [700, 701]:
*   *Descartados:* `ctx_agent`, `ctx_overview`, `ctx_preload`, `ctx_prefetch`, `ctx_wrapped`, `ctx_gain`, `ctx_feedback`, `ctx_task`, `ctx_workflow`, `ctx_context`, `ctx_response`, `ctx_graph_diagram`, `ctx_graph`, `ctx_share` [700].

### D. Joias Recuperadas no Marco 4.0.1 (Projeto Guilhotina)
Sob nova auditoria forense do cadáver READ-ONLY em `src-tauri/third_party/lean-ctx/`, identificamos **3 essências adicionais** que escaparam à varredura original e cuja "Alma Matemática" foi **formalmente canibalizada** para o chassi nativo do SOULS MC. Estas joias não eram ferramentas MCP (não estavam no barramento), mas padrões de design e catálogos de configuração que mereciam preservação.

1.  **`autonomy` ➔ `cognition::lean_vacuum::atomic_once::FireOnce`**: O padrão de inicialização atômica `compare_exchange(SeqCst)` sobre `AtomicBool` foi transplantado para um helper genérico `FireOnce<T>`. A técnica garante "fire-once" lock-free (sem `Mutex`/`RwLock`), eliminando o anti-pattern Zero-Slop de manter `MutexGuard` através de `.await`. Validação empírica: o teste de integração `src-tauri/tests/test_atomic_fire_once.rs` dispara 64 threads concorrentes e observa exatamente 1 vencedor / 63 perdedores em 0.01s [Marco 4.0.1].
2.  **`ctx_impact` ➔ `cognition::lean_vacuum::extensions::is_source_ext`** (parcial): O padrão de **BFS reverso de import edges** para análise de Blast Radius foi documentado como cânone arquitetural para um futuro módulo `cognition::impact`. A heurística de listar 17 extensões de código-fonte produtivas foi transplantada verbatim para o catálogo unificado.
3.  **`ctx_heatmap` ➔ `cognition::lean_vacuum::extensions::{SOURCE_EXTENSIONS, EXCLUDE_DIRS}`**: O catálogo de 17 extensões de código-fonte + 6 exclusões de diretório tóxico (`node_modules`, `target`, `dist`, `__pycache__`, `.git`, dot) foi unificado e estendido com o vocabulário do SOULS (`.souls_cache`, `.souls_data`, `.souls_scratchpad`, `.cargo`, `.vscode`, `.idea`, `vendor`, `build`, `_build`, `deps`, `.venv`, `__snapshots__`). Total canônico: **22 extensões + 22 exclusões**, governados por invariantes em compile-time via `assert_eq!` nos testes de unidade.

*   *Descartado e arquivado:* `ctx_routes` — sem aplicação para o SOULS MC (backend Rust com IPC Tauri, sem rotas HTTP server-side). O struct `RouteEntry` foi documentado como referência se um futuro módulo HTTP server for necessário.

---

## 4. Consequências da Amputação
1.  **Higiene Máxima de Workspace**: Removemos dezenas de arquivos mortos e pastas órfãs, mantendo apenas o código-fonte que ativamente compila no nosso produto final [304, 740].
2.  **Extirpação do Overhead de Compilação**: Banimos definitivamente do Cargo Workspace qualquer chance de o compilador tentar processar sub-dependências transitivas do `lean-ctx` [640].
3.  **Segurança Semântica**: Clientes MCP agora consultam um `tools/list` com descrições fiéis e precisas sobre o que cada ferramenta de fato executa no metal, eliminando stubs e incoerências estruturais [687].

---
## 5. Saco a Vácuo Nativo — O Destino Canônico (Marco 4.0.1)
O transplante integral da "Alma Matemática" do `lean-ctx` vive em um único módulo do nosso chassi:

*   **`src-tauri/src/cognition/lean_vacuum/`** — O Saco a Vácuo Nativo do SOULS. Módulos:
    *   `mod.rs` — Orquestrador `compress_to_lean()` e `read_to_lean()` (pipeline ANSI → comments → blank lines).
    *   `ansi_filter.rs` — `strip_ansi`, `ansi_density`.
    *   `dedup.rs` — `deduplicate_blocks`, `SESSION_DEDUP_CACHE` (canibalizado do `ctx_dedup`).
    *   `dot_flatten.rs` — `dot_flatten` (canibalizado do `ctx_tree`).
    *   `extensions.rs` — `SOURCE_EXTENSIONS` (22) + `EXCLUDE_DIRS` (22) (canibalizado de `ctx_heatmap` + extensions do `search.rs` SOULS) [Marco 4.0.1].
    *   `myers_diff.rs` — `myers_diff` (canibalizado do `ctx_delta` via crate `similar`).
    *   `search.rs` — `search_lean`, `format_lean_notation` (canibalizado do `ctx_search`).
    *   `smart_read.rs` — `smart_read_text`, `count_tokens` via `tiktoken` (canibalizado do `ctx_smart_read`).
    *   `text_compress.rs` — `aggressive_compress`, `lightweight_cleanup` (canibalizado do `ctx_compress`).
    *   `atomic_once.rs` — `FireOnce<T>` (canibalizado do `autonomy.rs`) [Marco 4.0.1].

**Regra de ferro:** qualquer crate do SOULS que precise de filtro de extensões, exclusão de diretórios tóxicos, deduplicação, compressão, leitura inteligente, busca textual, diff Myers, ou inicialização atômica lock-free deve importar daqui. Nenhum arquivo fora de `cognition/lean_vacuum/` deve redefinir essas listas — é o **SSOT vivo** das constantes de configuração derivada do lean-ctx.

---
## Veredito do Arquiteto-Chefe
> "O extermínio físico de terceiros é o rito que consagra a nossa soberania Bare-Metal. O SOULS MC agora é uma catedral de silício limpa e autossuficiente."

*Marco 4.0.1 (Projeto Guilhotina):* Após a varredura cega do cadáver em `src-tauri/third_party/lean-ctx/`, canibalizamos as **3 essências residuais** (`autonomy`, `ctx_impact`, `ctx_heatmap`) e unificamos as listas de extensões/exclusões no `cognition::lean_vacuum::extensions`. A pasta `third_party/lean-ctx/` foi extinta fisicamente; o `third_party/` residual foi removido. A catedral está purificada.
