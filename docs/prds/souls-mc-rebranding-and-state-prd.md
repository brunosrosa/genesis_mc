# PRD E SPEC DE ENGENHARIA: REBRANDING TOPOLÓGICO SODA ➔ SOULS E CONSOLIDAÇÃO DE PERSISTÊNCIA L2
**Versão:** 1.0.0  
**Status:** Aprovado para Execução (HITL)  
**Autor:** Principal Bare-Metal Systems Architect & Co-Piloto Cognitivo  
**Foco:** Extermínio de Slop, Higiene de Disco, Unificação de Identidade Verbal e Forja de Schemas STRICT SQLite [1014, 1015, 1021].

---

## 1. INTRODUÇÃO & DIRETRISES GERAIS (A VOZ DE VIDRO)
Este documento estabelece as leis duras e a especificação mecânica inegociável para a reestruturação e consolidação do ecossistema de dados e nomenclatura do **Souls MC** [1014, 1021]. Sob o manto do **Pessimismo da Razão**, a presença de arquivos planos caóticos espalhados pelo disco e caminhos contaminados por siglas inconsistentes representam um vazamento de entropia térmica intolerável [271, 273]. 

Não há espaço para remendos temporários (*vibe coding*) ou compilações com caminhos órfãos [1071]. Toda e qualquer operação detalhada a seguir deve ser executada pela IDE em um único commit atômico sob a branch de trabalho `feature/souls-rebranding-and-state-db`, aplicando de forma estrita o ciclo **TDD (Red-Green-Refactor)** e garantindo o **Exit Code 0** sem nenhum warning de compilação ou do linter `cargo clippy` [1065, 1070, 1071].

---

## 2. EIXO 1: A TRANSMUTAÇÃO TOPOLÓGICA (SODA ➔ SOULS)
Mudar o nome das artérias semânticas no disco e no código garante a paridade lógica entre o produto final (**Souls MC**) e as pastas que suportam seu estado. O *Blast Radius* físico desta mutação está delimitado a exatos **38 arquivos únicos**, englobando as pastas de estado, cache, scratchpads e conexões internas do servidor MCP Rust e o proxy L7 [ gateway-config.yaml ] [User's previous prompt].

### 2.1. Renomeação de Pastas Físicas (Mapeamento do Disco)
A IDE deve preparar comandos atômicos do Windows PowerShell para renomear e organizar os caminhos físicos na raiz de desenvolvimento:
*   `Z:\souls_mc\.soda_data\` $\rightarrow$ `Z:\souls_mc\.souls_data\`
*   `Z:\souls_mc\.soda_cache\` $\rightarrow$ `Z:\souls_mc\.souls_cache\`
*   `Z:\souls_mc\.soda_scratchpad\` $\rightarrow$ `Z:\souls_mc\.souls_scratchpad\`

### 2.2. Renomeação de Arquivos de Banco de Dados
No interior do novo diretório `.souls_data/`, os dois mundos (a Fábrica e o Produto) ganham nomenclatura canônica unificada:
*   `soda_heuristic_vault.db` $\rightarrow$ `souls_heuristic_vault.db` (Bancário Analítico de ETL) [13, 1002]
*   `soda_state.db` $\rightarrow$ `souls_state.db` (Banco Transacional e Cognitivo de Runtime) [13, 1012]

### 2.3. Search-and-Replace Estrito (Tabela de Paridade Lógica)
A IDE deve aplicar a substituição exata das strings nas seguintes frentes, sob o princípio do isolamento de *third-party* (sem tocar no código bruto do cadáver `lean-ctx` a não ser em seu conector de caminhos):

| String Antiga (Origem) | String Nova (Destino) | Arquivos Alvos Críticos |
| :--- | :--- | :--- |
| `.soda_data` | `.souls_data` | `src-tauri/src/persist/ssot_injector.rs`, `src-tauri/src/finops/finops_router.rs`, `src-tauri/src/bin/souls_mcp_server.rs`, todos os scripts de ETL (`f0_harvester_cli.rs` a `f5`), `gateway-config.yaml`, `test_gw.yaml` |
| `.soda_cache` | `.souls_cache` | `src-tauri/third_party/lean-ctx/src/core/data_dir.rs`, `test_gw.yaml`, `_WORKSPACE_MAP.md` |
| `.soda_scratchpad` | `.souls_scratchpad` | `src-tauri/src/harvester/github_tracker.rs`, scripts Python de compilação de contexto, `_WORKSPACE_MAP.md` |
| `soda_state.db` | `souls_state.db` | `src-tauri/src/bin/souls_mcp_server.rs` (linhas de conexão), `gateway-config.yaml`, `test_target.json` |
| `soda_heuristic_vault.db` | `souls_heuristic_vault.db` | `src-tauri/src/bin/souls_mcp_server.rs`, `src-tauri/src/persist/ssot_injector.rs`, `src-tauri/src/core/model_registry.rs`, `src-tauri/src/bin/soda_arena_cli.rs` |

---

## 3. EIXO 2: HIGIENE FÍSICA & EXORCISMO DO "SLOP"
O inventário de disco revelou que a IDE espalhou arquivos planos para logs e telemetria, gerando latências perigosas e riscos de condições de corrida durante execuções de enxames concorrentes [User's previous prompt, 878]. 

### 3.1. Arquivos Planos Sumariamente Extirpados (Delete-On-Boot)
Fica determinado o extermínio físico e a remoção completa dos seguintes arquivos da raiz de `.souls_data/`:
*   `events.jsonl`, `feedback.json`, `cost_attribution.json`, `heatmap.json`, `stats.json`, `tool-calls.log`, `context_ledger.json`, `mcp-live.json`, `pipeline_stats.json`.

Todo o seu conteúdo factual e registros serão portados e armazenados exclusivamente de forma relacional no `souls_state.db`.

### 3.2. Purificação do Vault (`souls_heuristic_vault.db`)
As tabelas `kanban_tasks` e `weevolve_learnings` foram erroneamente criadas dentro do banco analítico de 167MB (o Vault), violando o princípio de isolamento físico entre a Fábrica e o Produto [User's previous prompt, 13].
*   **Ação:** A IDE deve remover estas duas tabelas do `souls_heuristic_vault.db` (gerando um comando `DROP TABLE IF EXISTS` seguro e aplicando o `VACUUM` no banco analítico para reduzir o fardo térmico de disco).
*   Estas duas tabelas serão recriadas na raiz de runtime do `souls_state.db`.

---

## 4. EIXO 3: A MÁQUINA DE ESTADO RELACIONAL L2 (`souls_state.db`)
O banco de runtime `souls_state.db` (atualmente com 64KB) será a nossa única fonte local para persistência de curto e médio prazo [User's previous prompt]. Para blindar a integridade dos dados, **todas as tabelas novas devem ser declaradas utilizando explicitamente a cláusula `STRICT` do SQLite** (introduzida no SQLite 3.37+), forçando o compilador e o motor C do SQLite a rejeitarem dados malformados em tempo de inserção [User's previous prompt].

A IDE deve programar a inicialização automática e idempotente (Zero-Config) destas tabelas no reator de inicialização do Rust [172]. Os schemas DDL a serem injetados são os seguintes:

```sql
-- 1. DIÁRIOS DE SUBAGENTES (Substitui stubs e isola a RAM do Governador)
CREATE TABLE IF NOT EXISTS sub_agent_diaries (
    diary_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    sub_agent_name TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    axioms_derived TEXT NOT NULL,
    blockers TEXT
) STRICT;

-- 2. LIVRO DE CONTEXTO / CONTEXT LEDGER PROTOCOL (A2A Cryptography)
CREATE TABLE IF NOT EXISTS context_ledger (
    handoff_id TEXT PRIMARY KEY,
    from_agent TEXT NOT NULL,
    to_agent TEXT NOT NULL,
    encrypted_payload BLOB NOT NULL,
    timestamp INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('PENDING', 'ACCEPTED', 'EXPIRED'))
) STRICT;

-- 3. APRENDIZADOS DE VERIFICAÇÃO / VACINAÇÃO DE GOTCHAS
CREATE TABLE IF NOT EXISTS weevolve_learnings (
    learning_id TEXT PRIMARY KEY,
    trigger_error TEXT NOT NULL,
    resolution_approach TEXT NOT NULL,
    occurrence_count INTEGER DEFAULT 1,
    last_seen INTEGER NOT NULL,
    temporal_stability TEXT NOT NULL CHECK(temporal_stability IN ('STABLE', 'EVOLVING'))
) STRICT;

-- 4. QUADRO DE TAREFAS ATIVAS (Kanban Swarm)
CREATE TABLE IF NOT EXISTS kanban_tasks (
    task_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK(status IN ('BACKLOG', 'TODO', 'IN_PROGRESS', 'REVIEW', 'DONE', 'BLOQUEADO')),
    assigned_agent TEXT,
    priority TEXT NOT NULL CHECK(priority IN ('LOW', 'MEDIUM', 'HIGH', 'CRITICAL')),
    last_updated INTEGER NOT NULL
) STRICT;

-- 5. SESSÕES ATIVAS E CONTINUIDADE COGNITIVA (CCP)
CREATE TABLE IF NOT EXISTS session_states (
    session_id TEXT PRIMARY KEY,
    current_task TEXT NOT NULL,
    recorded_discoveries TEXT, -- JSON estruturado em String
    recorded_decisions TEXT,   -- JSON estruturado em String
    last_active INTEGER NOT NULL
) STRICT;

-- 6. RAZÃO DE FINOPS, TOKENOMETRIA E LATÊNCIAS (Substitui cost_attribution.json e feedback.json)
CREATE TABLE IF NOT EXISTS telemetry_costs (
    event_id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL,
    tokens_input INTEGER NOT NULL,
    tokens_output INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp INTEGER NOT NULL
) STRICT;

-- 7. HEATMAP E LOGS DE EXECUÇÃO DE TOOLS (Substitui tool-calls.log, heatmap.json e stats.json)
CREATE TABLE IF NOT EXISTS tool_activity (
    event_id TEXT PRIMARY KEY,
    tool_name TEXT NOT NULL,
    file_path TEXT,
    execution_status TEXT NOT NULL CHECK(execution_status IN ('SUCCESS', 'FAILED')),
    timestamp INTEGER NOT NULL
) STRICT;
```

---

## 5. EIXO 4: O REATOR RUST (`souls_mcp_server.rs`)
Para que o Souls MC ganhe as garras de execução e suporte à nova nomenclatura, o arquivo `src-tauri/src/bin/souls_mcp_server.rs` deve ser cirurgicamente adaptado.

### 5.1. Registro de Novas Ferramentas Semânticas no `tools/list`
Adicionar os esquemas de metadados das três ferramentas cognitivas na lista de exposição JSON do MCP, amarrados ao padrão **Zero-Brand** (`souls_*`) [1021, 1086]:
*   `souls_sub_agent`: Gerencia registros de diários e mensagens dos agentes em lote.
*   `souls_handoff`: Cria e consulta o histórico do Context Ledger.
*   `souls_knowledge`: Registra gotchas de TDD e vacinações de compilação.

### 5.2. Despacho das Chamadas de Escrita e Leitura (`handle_tool_call`)
Substituir o comportamento de stub genérico destas rotas por invocações dinâmicas ancoradas no SQLite local, redirecionadas para o reator de persistência seguro.

*   **As Duas Lógicas de Conexão (Leitura vs. Escrita):**
    No código em Rust, as funções que realizam consultas (`SELECT`) devem manter a abertura segura em modo exclusivo de leitura, evitando overheads e locks desnecessários:
    ```rust
    // Lógica para Leituras (Otimizada e sem locks)
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    ```
    Já para as ferramentas de escrita controlada (`souls_sub_agent`, `souls_handoff` e `souls_knowledge`), as funções internas do Rust devem abrir o banco with as seguintes flags de mutação e criação segura:
    ```rust
    // Lógica para Gravações Determinísticas parametrizadas
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    // Impor busy_timeout de 5000ms para aguardar locks de concorrência graciosamente
    conn.execute("PRAGMA busy_timeout = 5000;", [])?;
    ```

---

## 6. EIXO 5: BLINDAGEM DE CONCORRÊNCIA E PERSISTÊNCIA (MPSC WRITER)
Operações simultâneas de gravação disparadas por subagentes em lote no mesmo banco geram deadlocks e abortamentos ruidosos [156, 157]. Para resolver este gargalo com elegância, **as ferramentas MCP do Souls MC não devem escrever diretamente no arquivo SQLite**. 

*   O servidor `souls_mcp_server` deve instanciar um canal assíncrono do tipo **`tokio::sync::mpsc::bounded(100)`** no arranque do daemon [157].
*   Toda solicitação de gravação gerada pelas ferramentas semânticas (`souls_sub_agent`, `souls_handoff`, `souls_knowledge`) é envelopada em uma estrutura de dados de evento de escrita e empurrada para a fila RAM através do transmissor (`Sender`).
*   Uma única thread dedicada em background (criada via `tokio::task::spawn_blocking` e monitorada por um loop contínuo de recepção `Receiver`) consome essa fila MPSC sequencialmente, processa as transações de disco um-a-um, e executa os commits físicos de forma limpa.

Isso garante latência de barramento sub-milissegundo para os subagentes e elimina por completo os picos térmicos de I/O em disco SSD NVMe [155, 157].

---

## 7. DEFINITION OF DONE (DoD) DO REBOOT
A branch `feature/souls-rebranding-and-state-db` só poderá sofrer merge na main após a IDE validar a seguinte esteira:
1.  [ ] **Compilação:** O comando `cargo check` no workspace deve retornar **Exit Code 0** [1065, 1080].
2.  [ ] **Clippy:** O comando `cargo clippy --all-targets -- -D warnings` deve retornar **Green/Zero Warnings** [355].
3.  [ ] **Boot:** O script `Z:\souls_mc\boot.ps1` deve inicializar o sistema de bandejas sem travar ou acusar arquivos não encontrados [1050].
4.  [ ] **Sanidade Stdio (ADR-003):** Executar `./target/debug/souls_mcp_server.exe < NUL` e verificar que **exatos 0 bytes** são despejados no `stdout` [1001].
5.  [ ] **Conexão:** O cliente MCP da IDE deve conectar com sucesso à porta `http://127.0.0.1:3001/` em menos de 1ms de latência [User's previous prompt].

Trabalhe com rigor e sem pressa! A forja nos chama! 🦅⚙️🍷
