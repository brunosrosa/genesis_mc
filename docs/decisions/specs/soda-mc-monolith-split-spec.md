# ESPECIFICAÇÃO DE REFATORAÇÃO: CLIVAGEM E DESACOPLAMENTO DO MONÓLITO (C4 SPLIT)

## 🏛️ 1. O Racional de Design (A Cura do Context Hoarding)
O arquivo `souls_mcp_server.rs` ultrapassou a marca de 8.000 linhas [user query]. Manter um monólito desse tamanho causa três patologias físicas graves:
1. **Asfixia de Contexto (Context Hoarding):** O Agente de IA é obrigado a engolir e re-enviar milhares de linhas de código redundante em toda pequena alteração sintática, destruindo o orçamento FinOps de tokens [445].
2. **Starvation do Compilador (Compile Slowness):** Modificar um único handler obriga o `rustc` a reavaliar e re-checar todo o macro de correspondência e recursão de tipos do `handle_tool_call`, inflando os tempos de build incrementais [607, 610].
3. **Blast Radius Incontrolável:** Uma falha de sintaxe ou de lifetimes em uma ferramenta específica de background paralisa as outras 47 ferramentas do barramento [185, 273].

Esta especificação define o **C4 Split** (Frente Tática C4) [610] para fatiar o monólito em um módulo de diretório limpo e isolado sob a branch `refactor/monolith-split`, sem quebrar a compatibilidade de aliases no gateway [618] e garantindo a manutenção da nossa suíte de testes.

---

## 🗂️ 2. A Nova Arquitetura de Diretório (`src/bin/souls_mcp_server/`)

O arquivo plano único `src/bin/souls_mcp_server.rs` [648] é totalmente clivado na seguinte topologia modular:

```text
src/bin/souls_mcp_server/
├── main.rs          # Loop do Tokio, canais MPSC, bootstrap do SQLite e stdio reader
├── tools.rs         # Dicionário declarativo e schemas de entrada/saída (tetos 32/120)
├── router.rs        # Match centralizador de chamadas de ferramentas (handle_tool_call)
└── handlers/        # Lógicas físicas de execução separadas por domínios
    ├── context.rs       # Leitura, compressão e reidratação (read, multi_read, compress, fill)
    ├── memory_graph.rs  # Grafo cognitivo relacional (mem_create_entities, mem_search, etc.)
    ├── observability.rs # Heatmaps, Langevin decay, Blast Radius (heatmap, impact, routes)
    ├── thinking.rs      # Persistência socrática (thinking, export_session, merge_sessions)
    └── system.rs        # Ferramentas de sistema (shell, execute, sys_time)
```

---

## 💾 3. Mudança no Manifesto (`Cargo.toml`)

No `Cargo.toml` [648], alteramos a definição do binário de arquivo plano para apontar para a nova pasta modular:

```toml
# Antes:
# [[bin]]
# name = "souls_mcp_server"
# path = "src/bin/souls_mcp_server.rs"

# Depois:
[[bin]]
name = "souls_mcp_server"
path = "src/bin/souls_mcp_server/main.rs"
```

---

## 🔬 4. Práticas de Isolamento e Passagem de Estado

1. **State Ownership:** O `StateDbWorker` e os handles de canais MPSC (`STATE_DB_TX`, `MEMORY_GRAPH_TX`, `SOCRATIC_TX`) continuam inicializados no `main.rs` [715]. As referências ou clones dos transmissores de canais são passados aos handlers no momento do despacho no `router.rs`.
2. **Zero-Copy Lifetimes:** As assinaturas das funções nos handlers utilizam fatias de referência `&'a str` e referências a `serde_json::Map` sempre que aplicável, evitando alocações e clonagens redundantes na Heap [688].
3. **Mantenha os Testes Locais:** A suíte inteira de testes de integração (como `test_database_migration_v6_schema`, `test_vram_scheduler`, `test_drift_sentinel`) é movida para um arquivo próprio de testes `tests.rs` ou embutida no `main.rs` para validação rápida via `cargo test --bin souls_mcp_server`.

---

## 🚦 5. Critérios de Aceite (Definition of Done)

* [ ] O compilador Rust retorna sucesso com **Exit Code 0** [user query].
* [ ] O Cargo Clippy retorna **Zero Warnings** e sem erros [user query].
* [ ] Todos os **74 testes de integração** passam perfeitamente em modo de marcha rápida [user query].
* [ ] O tamanho total do arquivo `main.rs` de bootstrap é reduzido para menos de **800 linhas**, distribuindo a complexidade de forma assíncrona.
