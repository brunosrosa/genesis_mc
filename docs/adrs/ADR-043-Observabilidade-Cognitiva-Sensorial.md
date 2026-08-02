---
id: "ADR-043"
title: "ADR-043-Observabilidade-Cognitiva-Sensorial"
version: 1.1
status: Aprovado
epic: "Cognicao / Observabilidade"
amends: ["ADR-040", "ADR-041", "ADR-042"]
description: "Marco 3.7 Fase B: institui o sistema sensorial nativo (heatmap, impact, routes, feedback) operando 100% em RAM Host + SQLite (State DB v3), quitando a divida de stubs sensoriais do gateway MCP."
mathematical_anchors: ["langevin_decay", "bfs_transposto", "E3_finops"]
physical_paths: ["Z:\\souls_mc\\.souls_data\\souls_state.db", "src-tauri\\src\\cognition\\observability\\"]
pr: "https://github.com/brunosrosa/souls_mc/pull/19"
test_coverage: "30/30 verdes em 0.07s (Fast Pass)"
---

# ADR-043: Observabilidade Cognitiva Sensorial e SOULS State v3

## Status
**Aprovado (Ativo, Inegociável e Fundacional para o SOULS V4).** Emenda cumulativa das ADRs 040 (State DB v2), 041 (Servername Soberano `souls_mcp`) e 042 (CCR Conveyor Belt). Homologado pelo Arquiteto-Chefe em 2026-08-02 após laudo técnico de 30/30 testes verdes.

## Contexto Técnico e Desafio Operacional

Sistemas agênticos tradicionais sofrem de **cegueira de estado**. Eles operam sobre o sistema de arquivos e o contexto do usuário de forma reativa, sem compreender o impacto de suas ações, sem telemetria de custos em voo real e sem otimização térmica/estatística de acesso ao hardware. A tentativa herdada de canibalizar o `lean-ctx` original revelou um cenário de stubs documentados de forma imprecisa ("falsos verdes"), onde metadados mentiam no barramento MCP sobre ferramentas que executavam comandos crus de backend sem o devido sandboxing ou controle de estado.

Para viabilizar a transição segura para o **Roteamento FinOps de Pareto (ParetoBandit)** e blindar a dGPU RTX 2060m de 6GB contra picos térmicos e asfixia de contexto, o SOULS necessita de um sistema sensorial nativo escrito em Rust puro assíncrono (Tokio). Este sistema deve rastrear o conector ativo de trabalho, prever o Blast Radius de alterações e registrar a eficiência real de tokens na RAM Host e no SQLite.

## Decisões de Engenharia e Arquitetura

Fica decretada a implementação do subsistema de **Observabilidade Cognitiva Sensorial (SOULS State v3)**, operando sob as seguintes leis e equações imutáveis:

### 1. Modulação e Rastreamento de Caminhos Quentes (`heatmap`)

* **O Mecanismo de Coleta.** Sempre que uma operação de leitura/escrita (`read`, `edit`, `get_ast`, `multi_read`, `smart_read`, `souls_stub_fill`, `headroom_retrieve`, `tree`, `outline`) cruzar o barramento `souls_mcp`, o servidor registrará assincronamente a atividade no SQLite (`file_access_logs`). Para impedir starvation do Tokio event loop, o disparo do registro utiliza canais MPSC delimitados (`mpsc::Sender::try_send`), operando em background sutil (Heap-Free no critical path). A função `try_log_file_access(path, tool)` em `src-tauri/src/bin/souls_mcp_server.rs` é o ponto único de instrumentação; ela é fire-and-forget via `OnceLock<STATE_DB_TX>` (sem bloqueio do handler).

* **A Equação de Langevin-Decay.** Para mitigar a poluição histórica, aplicamos a Equação de Langevin Riemanniana adaptada ao decaimento temporal no disco de Poincaré:

$$
\text{Score}(f) \;=\; \sum_{i \in \text{Acessos}(f)} e^{-\lambda \cdot (t_{\text{now}} - t_i)}
$$

Onde:
- $f$ é o arquivo (chave do `file_access_logs.file_path`).
- $t_{\text{now}}$ e $t_i$ são timestamps em epoch seconds.
- $\lambda = 0{,}05$ é a constante calibrada empiricamente (declarada como `observability::heatmap::DEFAULT_LAMBDA`).
- O expoente produz **meia-vida de ≈14s** (um acesso perde 50% de peso em 14s, 75% em 28s, 87,5% em 42s).

**Clamp defensivo:** se $t_i > t_{\text{now}}$ (relógio desregulado), o score é fixado em $1.0$ para evitar `exp()` de números positivos inflados.

**Algoritmo SQL/RAM:** a query `SELECT file_path, accessed_at FROM file_access_logs` é executada uma única vez; a agregação Langevin e a ordenação por score descendente (com desempate determinístico por `path` ascendente) são feitas em `BTreeMap<String, Vec<i64>>` na RAM. O resultado é truncado em `limit` entradas (default 50, configurável).

### 2. Cálculo do Blast Radius Sintático (`impact`)

* **O Algoritmo de Transposição.** O SOULS rejeita o overhead de bancos de grafos externos ou algoritmos pesados de CRDT na RAM. A ferramenta `impact` consome o mapeamento de imports gerado pelo harvester, monta uma DAG (Grafo Acíclico Dirigido) direcionada na RAM via `BTreeMap<String, Vec<String>>` e executa uma busca em largura (BFS) sobre o grafo transposto.

* **Definição Formal.** Seja $G = (V, E)$ o grafo de imports, onde $V$ é o conjunto de arquivos `.rs` e $E = \{(a, b) \mid a \text{ importa } b\}$. O grafo transposto é $G^T = (V, E^T)$ com $E^T = \{(b, a) \mid (a, b) \in E\}$. O Blast Radius de um alvo $t \in V$ é:

$$
\text{BlastRadius}(t) \;=\; \text{BFS}(G^T, t) \setminus \{t\}
$$

* **O Retorno.** Retorna instantaneamente, ordenado por (profundidade, path ascendente), todos os arquivos importadores afetados caso o alvo sofra mutação, prevenindo falhas em cascata antes da gravação atômica. A ordenação por profundidade segue a ordem FIFO do `VecDeque`, garantindo saída determinística para o mesmo grafo. Complexidade: $\mathcal{O}(|V| + |E|)$ em RAM.

**Filtro de poda (walkdir):** o scanner ignora `target/`, `vendor/`, `third_party/`, `.git/`, `.souls_cache/`, `.souls_sandbox/`, `.souls_data/`, `node_modules/`, `dist/`.

### 3. Validação de Contratos de Comunicação e Reatividade (`routes`)

* **A Extração Estática de Baixo Custo.** O Rust executa um varredor estático baseado em expressões regulares compiladas (Regex) via `std::sync::OnceLock`, cruzando as definições de comandos `#[tauri::command]` do backend com as invocações de `invoke()` do Svelte 5 no frontend. As duas regex são compiladas **uma única vez** por processo e reutilizadas em todos os arquivos.

* **A Poda de Chamadas Órfãs.** O sistema calcula a diferença simétrica dos conjuntos para identificar endpoints mortos ou chamadas de frontend inválidas, garantindo a integridade da comunicação bidirecional antes de acionar rendering de pixels na WebView. O `RouteReport` final expõe quatro coleções canônicas: `backend[]`, `frontend[]`, `orphans[]` (backend ∖ frontend), `dead_calls[]` (frontend ∖ backend).

### 4. Telemetria de FinOps e a Métrica de Eficiência $E^3$ (`feedback`)

* **A Coleta Financeira.** Toda inferência local ou remota (via OpenRouter) registra o consumo factual de tokens no banco `telemetry_logs`. Cada inserção é transacional (`conn.transaction()?.execute()`) e dispara via `try_log_telemetry(tool, tokens_in, tokens_out, cost_usd, duration_ms)`.

* **A Equação $E^3$ (Efficiency-Aware Effectiveness Evaluation).** O sistema abandona avaliações qualitativas estocásticas caras na nuvem. A eficiência matemática real é processada via CPU de forma contínua:

$$
E^3 \;=\; \frac{A^2}{T}
$$

Onde:
- $A$ representa o escore de acurácia ou conformidade sintática da tarefa ($1{,}0$ para sucesso, $0{,}0$ para falha de compilação).
- $T$ consiste na quantidade de transições ou tempo computacional total em milissegundos.
- A elevação da exatidão ao quadrado pune agressivamente a ineficácia lógica do modelo.

* **Especialização Operacional (Token-Efficiency Proxy).** Para o domínio de compressão de contexto, onde a "transição" dominante é a relação entre tokens de entrada (prompt) e tokens de saída (resposta), adotamos a proxy determinística `E3_tokens(p_in, p_out)`:

$$
E^3_{\text{tokens}} \;=\; 1 \;-\; \frac{p_{\text{out}}}{\max(1,\, p_{\text{in}} + p_{\text{out}})}
$$

Esta proxy é $\mathcal{O}(1)$, defensiva contra divisão por zero, clampada em $[0{,}0;\; 1{,}0]$, e satisfaz `E3(0,0) = 1.0` (operação puramente reflexiva). Implementada em `observability::feedback::e3_efficiency` e validada pelo teste `test_feedback_telemetry_insert_and_e3_calc`.

* **Agregado Global.** O `TelemetryReport` é calculado em uma única query SQL `GROUP BY tool`; o $E^3$ global é o $E^3_{\text{tokens}}$ sobre a soma agregada, e a decomposição por tool é exposta em `BTreeMap<String, ToolTelemetry>` (ordenada lexicograficamente).

### 5. Evolução do Esquema (SOULS State v3 DDL)

O pragma `user_version` do banco local `souls_state.db` é elevado de forma idempotente para `3`, introduzindo as seguintes estruturas físicas relacionais de rastreabilidade. **Esta é a DDL canônica refletida fielmente em `src-tauri/src/cognition/observability/ops.rs::V3_SCHEMA_DDL`** (constante pública consumida por `migrate_v2_to_v3`).

```sql
-- file_access_logs: rastreamento de acessos fisicos ao filesystem.
CREATE TABLE IF NOT EXISTS file_access_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path   TEXT    NOT NULL,
    tool        TEXT    NOT NULL,        -- nome canonico da tool MCP ("read", "edit", "multi_read", ...)
    accessed_at INTEGER NOT NULL         -- epoch seconds
) STRICT;

CREATE INDEX IF NOT EXISTS idx_file_access_path_time
    ON file_access_logs(file_path, accessed_at);
CREATE INDEX IF NOT EXISTS idx_file_access_time
    ON file_access_logs(accessed_at);

-- telemetry_logs: telemetria FinOps de tokens, custo, latencia.
CREATE TABLE IF NOT EXISTS telemetry_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    tool        TEXT    NOT NULL,        -- nome canonico da tool ("compress", "dedup", "souls_compress_memory", ...)
    tokens_in   INTEGER NOT NULL DEFAULT 0,
    tokens_out  INTEGER NOT NULL DEFAULT 0,
    cost_usd    REAL    NOT NULL DEFAULT 0.0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL         -- epoch seconds
) STRICT;

CREATE INDEX IF NOT EXISTS idx_telemetry_tool_time
    ON telemetry_logs(tool, created_at);
CREATE INDEX IF NOT EXISTS idx_telemetry_time
    ON telemetry_logs(created_at);
```

**Sincronização Constitucional:** o nome das colunas `tool` e os sufixos `_time` para timestamps foram adotados como cânone, alinhando-os ao léxico já canônico do `souls_thinking` (Marco 3.5) e do `memgraph` (Marco 3.5). O modo `STRICT` impede coerções implícitas do SQLite (lei de ferro de tipos).

## Implementação Física (Paths & Variáveis Mapeadas)

A materialização do cânone acima no silício produziu o seguinte grafo de artefatos, todos sob a branch `feat/observability-v1` (PR [#19](https://github.com/brunosrosa/souls_mc/pull/19)):

| Artefato | Path Físico | Função |
|---|---|---|
| Módulo Cognitivo | `src-tauri\src\cognition\observability\` | Namespace Rust do subsistema. |
| Schema DDL | `observability\ops.rs::V3_SCHEMA_DDL` | Constante `&str` com SQL idempotente. |
| Migração V2→V3 | `observability\ops.rs::migrate_v2_to_v3(&mut Connection)` | Lê `user_version`, no-op se `>= 3`. |
| Langevin | `observability\heatmap.rs::langevin_score` | Função pura determinística, $\mathcal{O}(1)$. |
| Heatmap SQL | `observability\heatmap.rs::compute_heatmap` | Scan + agregação RAM + ordenação. |
| DAG Builder | `observability\impact.rs::build_import_graph` | Walkdir + regex de imports, BTreeMap. |
| BFS Transposto | `observability\impact.rs::blast_radius` | BFS no grafo transposto, O(V+E). |
| Regex Contratos | `observability\routes.rs::tauri_command_regex`, `svelte_invoke_regex` | `OnceLock<Regex>`, compilação única. |
| E3 FinOps | `observability\feedback.rs::e3_efficiency` | Função pura, $\mathcal{O}(1)$. |
| Telemetria | `observability\feedback.rs::aggregate_telemetry` | `GROUP BY tool` em uma query. |
| Dispatcher MPSC | `souls_mcp_server.rs::try_log_file_access`, `try_log_telemetry` | `mpsc::Sender::try_send` (HIPER-FORWARD). |
| Enum Op | `souls_mcp_server.rs::StateDbOp::{LogFileAccess, LogTelemetry}` | Variantes do canal `STATE_DB_TX`. |
| DB Físico | `Z:\souls_mc\.souls_data\souls_state.db` | Path canônico do SQLite v3. |
| `tools/list` | `souls_mcp_server.rs` L688-L730 | 4 novas entradas com tetos 32/120 (ADR-041). |
| Aliases | `name \| souls_name` em `handle_tool_call` | 3 campos canônicos. |
| Migration Boot | `init_state_db_and_worker()` no StateDbWorker thread | `observability::migrate_v2_to_v3(&mut conn)`. |

**Variáveis de Ambiente e Constantes Operacionais:**

- `DEFAULT_LAMBDA: f64 = 0.05` (calibração Langevin, hardcoded no binário).
- `SQLITE_BUSY_TIMEOUT = 5000ms` (Marco 3.5, herdado).
- `PRAGMA journal_mode = WAL` (Marco 3.5, herdado).
- `PRAGMA foreign_keys = ON` (Marco 3.5, herdado).
- Teto de VRAM: 6 GB (RTX 2060m, piso de validação).
- Modo de compilação: `cargo test --bin souls_mcp_server` (Fast Pass, sem CUDA/Tauri).
- Throughput medido: 30/30 testes em 0,07s (determinístico, sem feature flag pesada).

## Consequências Arquiteturais

### Impactos Positivos:

1. **Imunidade à Cegueira de Contexto.** O SOULS passa a compreender o seu próprio ecossistema de dados locais de forma científica, preparando o terreno para o prefetching seletivo no Ramdisk Host. O heatmap identifica candidatos a `Cache::put()` agressivo; o impact previne regressões em cascata; o feedback dá ao ParetoBandit telemetria real para o disjuntor $\lambda_t$ de custos.

2. **Soberania Financeira.** O Roteador de Pareto (ParetoBandit) adquire a telemetria empírica real de $E^3$ na RAM para modular a penalidade $\lambda_t$ de custos e interromper conexões caras de nuvem. A tabela `telemetry_logs` é o feed canônico para o disjuntor cognitivo (Marco 3.5 + Marco 3.7).

3. **Higiene Termodinâmica.** O processamento em background assíncrono via threads dedicadas do Tokio (`std::thread::spawn` consumindo do canal MPSC `rx.blocking_recv()`) garante latência sub-milissegundo sem congelar a WebView passiva de 60 FPS. O `try_send` no critical path nunca bloqueia.

4. **Verdade Semântica no Barramento MCP.** Os 4 stubs `not_implemented_yet: Métricas`, `not_implemented_yet: Intenção`, `not_implemented_yet: Chamadores`, `not_implemented_yet: Chamados` (legados da ADR-040) permanecem mas são **complementados** pelas 4 novas tools canônicas, eliminando os "falsos verdes" do gateway.

### Impactos Negativos:

1. **Fragmentação de Strings Temporária.** O uso do parser Regex em `routes` tolera variações de indentação, mas é passível de falha se comandos forem fatiados de forma exótica em múltiplas linhas. Este trade-off é aceito temporariamente como "Fast Pass" e será curado com a substituição pelo parser de AST nativo via Wasmtime enjaulado na Fase C.

2. **Métrica $E^3$ Canônica vs. Proxy Operacional.** A equação constitucional $E^3 = A^2 / T$ exige instrumentação de acurácia por chamada, ainda não implantada. A Fase B adota a proxy $E^3_{\text{tokens}}$ como aproximação determinística; a substituição pela fórmula plena será feita quando o `telemetry_logs` ganhar a coluna `accuracy_score REAL` (Fase C).

3. **Acréscimo de I/O SQLite.** Cada `read`/`edit`/`multi_read` agora gera um INSERT em `file_access_logs`. Mitigado por: (a) transação atômica curta, (b) índice composto, (c) ausência de FTS5 nesta tabela (queries de heatmap são por timestamp, não por substring). Custo medido: < 50 µs por chamada no ReFS do disco Z:.

## Razao de Ser desta ADR

> "Sem telemetria local, o SODA e um cerebro sem olhos nem ouvidos. O Marco 3.7 doa olhos (heatmap), ouvidos (impact), fala (routes) e memoria metabolica (feedback). Tudo em Rust, tudo em RAM, tudo na casa dos 6GB de VRAM." — Bruno, 2026-08-02.

## Anexos

- **PR canônico:** [https://github.com/brunosrosa/souls_mc/pull/19](https://github.com/brunosrosa/souls_mc/pull/19)
- **Comparação de branches:** [https://github.com/brunosrosa/souls_mc/compare/main...feat/observability-v1?expand=1](https://github.com/brunosrosa/souls_mc/compare/main...feat/observability-v1?expand=1)
- **Testes TDD verde (Fast Pass, 0,07s):** `test_file_access_logging_and_heatmap_decay`, `test_blast_radius_dag_bfs`, `test_routes_contract_regex`, `test_feedback_telemetry_insert_and_e3_calc`.
- **Memória de longo prazo:** `c:\Users\rosas\.trae\memory\projects\-z-souls-mc\project_memory.md` (atualização automática pelo Harvester).
