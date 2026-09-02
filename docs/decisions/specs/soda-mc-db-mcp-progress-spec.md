# ESPECIFICAÇÃO TÉCNICA E CADERNO DE TDD: MARCO 5.10.0 (TASKS 142 & 143)

## 💾 1. Task 142 — Saneamento Relacional do FrankenSQLite

### 1.1 Racional do Design
Durante o avanço frenético do pipeline de ETL Cognitivo V3 e a consolidação das 85 colunas de controle da Matriz Mestre, diversas colunas de pontuação e status de sincronização foram renomeadas. Como consequência direta, as views relacionais `quarantine_radar` e `action_matrix` tornaram-se inconsistentes perante as tabelas reais do banco `souls_state.db`. 
Esta especificação repara as fendas relacionais, reinstaura a integridade referencial de chave estrangeira (`FOREIGN KEY`) entre a tabela de subcomponentes (`deep_components`) e a tabela principal de repositórios, e normaliza as VIEWS utilizando os novos Enums e colunas estáveis.

### 1.2 DDL de Saneamento e Migração (State DB v6)
A migração elevará o `PRAGMA user_version` para `6`. As alterações são aplicadas de forma estritamente idempotente:

```sql
-- Ativação de chaves estrangeiras por sessão
PRAGMA foreign_keys = ON;

-- 1. Recriação da tabela de subcomponentes com integridade referencial estrita
CREATE TABLE IF NOT EXISTS deep_components (
    component_id TEXT PRIMARY KEY STRICT,
    solution_id TEXT NOT NULL,
    solution_name TEXT NOT NULL,
    component_name TEXT NOT NULL,
    component_group TEXT NOT NULL,
    analysis_status TEXT NOT NULL,
    analysis_date INTEGER NOT NULL,
    analyst TEXT NOT NULL,
    component_version_of_analysis TEXT NOT NULL,
    FOREIGN KEY(solution_id) REFERENCES repositorios(repo_url) ON DELETE CASCADE
);

-- Indexadores para consultas O(1) de subcomponentes
CREATE INDEX IF NOT EXISTS idx_deep_comp_solution ON deep_components(solution_id);

-- 2. Correção e Normalização da VIEW: quarantine_radar
-- Filtra todos os repositórios embargados ou com status de embargo ativo que exigem atenção imediata (embargo_status = 1)
DROP VIEW IF EXISTS quarantine_radar;
CREATE VIEW quarantine_radar AS
SELECT 
    r.project_name,
    r.repo_url,
    rh.status_atualizacao,
    rh.status_fase,
    rh.classificacao_terminal,
    rh.risco_principal,
    rh.risco_linha_vermelha
FROM repositorios r
JOIN repo_heuristics rh ON r.repo_url = rh.solution_id
WHERE rh.embargo_status = 1 
   OR rh.status_atualizacao = 'REJEITADO_DESCARTE'
   OR rh.classificacao_terminal = 'REJECT';

-- 3. Correção e Normalização da VIEW: action_matrix
-- Filtra os repositórios aprovados para canibalização, ordenando-os para ataque imediato
DROP VIEW IF EXISTS action_matrix;
CREATE VIEW action_matrix AS
SELECT 
    r.project_name,
    r.repo_url,
    rh.classificacao_terminal,
    rh.status_atualizacao,
    rh.status_fase,
    rh.ouro_a_extrair,
    rh.score_final
FROM repositorios r
JOIN repo_heuristics rh ON r.repo_url = rh.solution_id
WHERE rh.classificacao_terminal IN ('STACK_CORE_PLANO_A1', 'STACK_CORE_PLANO_A2', 'INTEGRATE_AS_COMPONENT', 'ABSORB_PARTIALLY')
  AND rh.status_atualizacao != 'REJEITADO_DESCARTE'
ORDER BY rh.score_final DESC;
```

---

## ⚙️ 2. Task 143 — Progress Notifications no souls_mcp_server

### 2.1 Racional de UX Sensorial (JSON-RPC + Stderr)
Para aniquilar a asfixia de feedback em comandos lentos (como varreduras AST pesadas ou testes do compilador no metal), o `souls_mcp_server` implementará o protocolo de **Progress Notifications** (especificação oficial do MCP 1.0.0). 
Toda e qualquer telemetria, erros e mensagens estritas de progresso continuarão isoladas no canal **`stderr`** via `eprintln!`, mantendo o canal **`stdout`** 100% puro para as respostas de protocolo JSON-RPC estruturadas.

### 2.2 O Protocolo de Mensagens
Quando o cliente MCP envia um `progressToken` opcional na chamada de ferramenta via metadados (`_meta`), o servidor em Rust despacha notificações periódicas de progresso assíncronas do tipo `notifications/progress` no `stdout`:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progressToken": "task_142_migration",
    "progress": 50.0,
    "total": 100.0
  }
}
```

### 2.3 Implementação em Rust (Helper Util)
Criamos uma macro/helper resiliente no `souls_mcp_server` para despachar esses eventos com segurança e sem travamento:

```rust
use serde_json::json;

pub fn report_mcp_progress(token: &str, progress: f64, total: f64) {
    // 1. Emite telemetria sensorial limpa para o stderr (visível no terminal / logs)
    eprintln!("[PROGRESS] {} => {:.1}% / {:.1}%", token, progress, total);

    // 2. Formata e envia a notificação JSON-RPC estrita no stdout
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": {
            "progressToken": token,
            "progress": progress,
            "total": total
        }
    });

    if let Ok(serialized) = serde_json::to_string(&notification) {
        println!("{}", serialized);
    }
}
```

---

## 🚦 3. Caderno de Testes TDD (DoD GREEN)

Escreveremos e rodaremos os seguintes testes sob `cargo test --bin souls_mcp_server`:

1. `test_database_migration_v6_schema`: Prova a criação e a integridade das tabelas e VIEWS normalizadas na versão 6 do State DB.
2. `test_quarantine_radar_filtering`: Popula registros em quarentena e valida se a VIEW expõe os itens de embargo de forma cirúrgica.
3. `test_action_matrix_ordering`: Popula dados de scores cruzados e assevera que a VIEW de ação ordena corretamente decrescente por `score_final`.
4. `test_mcp_progress_rpc_serialization`: Valida que o helper de progresso gera uma serialização JSON-RPC idêntica à especificação oficial do MCP 1.0.0, sem violar as travas do stdio.
