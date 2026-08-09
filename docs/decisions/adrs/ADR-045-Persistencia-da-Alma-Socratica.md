---
id: "ADR-045"
title: "ADR-045-Persistencia-da-Alma-Socratica"
version: 2.0
status: Aprovado
epic: "Cognicao / Persistencia Socratica + Souls State V5"
amends: ["ADR-029", "ADR-040", "ADR-041", "ADR-043", "ADR-044", "ADR-011", "ADR-014"]
description: "Marco 3.9 Fase E (Emendado Marco 5.9): institui a persistência relacional do ThinkingEngine socrático no SQLite WAL STRICT e formaliza a auditoria HITL via interrupção socrática no chat/CLI em conformidade com o Pragmatismo de Interface."
mathematical_anchors: ["O(1)_adjacency_reconstruction", "ON_DELETE_CASCADE_subgraph_purge", "last_write_wins_merge"]
physical_paths: ["src-tauri\\src\\cognition\\thinking\\persistence.rs", "src-tauri\\src\\cognition\\thinking\\ops.rs", "src-tauri\\src\\cognition\\thinking\\analytics.rs", "src-tauri\\src\\bin\\souls_mcp_server.rs", ".souls_data\\souls_state.db"]
pr: "https://github.com/brunosrosa/souls_mc/pull/21 (commits cumulativos)"
test_coverage: "39/39 verdes em <0.2s (Fast Pass Marcha Rapida)"
---

# ADR-045: Persistência da Alma Socrática e Auditoria HITL via Chat/CLI

## Status

**Aprovado (Versão 2.0 / Marco 5.9).** Emenda cumulativa das ADRs 011 (HITL), 014 (Fricção Produtiva), 029 (Visão Cognitiva O(1)), 040 (Souls State), 041 (Servername Soberano `souls_mcp`), 043 (Observabilidade Sensorial) e 044 (Enjaulamento Wasmtime). Quita a amnésia de raciocínio do `ThinkingEngine` via SQLite WAL STRICT e estabelece a auditoria socrática interativa no chat/CLI ativo sob a Regra de Pragmatismo de Interface.

## Contexto Técnico e Desafio Operacional

O `ThinkingEngine` (PRD-032 §3) canibalizou o `ultrafast-mcp-sequential-thinking` com sucesso mas operava **estritamente in-RAM**:

```rust
pub struct ThinkingEngine {
    session_id: String,
    main_thread: Vec<ThoughtData>,
    branches: HashMap<BranchId, Vec<ThoughtId>>,
    hard_limit: u32,
    hitl_authorized: bool,
}
```

Esta escolha arquitetural previa que o estado era **descartável** (sessão = subagente; teardown = heap freed). Porém, o gateway `souls_mcp` evoluiu para hospedar 3 ferramentas canônicas que **exigem** o histórico da sessão:

1. **`export_session`** — Reconstruir a árvore relacional para auditoria humana socrática (HITL via CLI/Chat) ou para apresentação a outro agente.
2. **`analyze_session`** — Computar métricas comportamentais (taxa de revisão, fan-out médio, latência média) para FinOps cognitivo.
3. **`merge_sessions`** — Fundir dois fluxos de raciocínio concorrentes sob consistência eventual, especialmente em workflows multi-agente onde o `swarm_dispatcher` paraleliza pensamentos e precisa reconciliar divergências.

Sem persistência, estas 3 tools só podem operar enquanto o `ThinkingEngine` está vivo em memória — uma janela de ~segundos durante o dispatch MCP. **A amnésia é estrutural**, não acidental.

## Decisões de Engenharia e Arquitetura

### 1. Governança Humana e Auditabilidade HITL Interativa (Pragmatismo de Interface)
*   **Regra de Pragmatismo de Interface:** Em concordância com as ADRs 011 e 014, a auditoria de sessões socráticas contorna telas visuais e telas gráficas enquanto a Milestone 4 (Frontend Canvas) estiver inativa.
*   **Interrupção no Chat Active:** Toda revisão de hipóteses socráticas de alto risco dispara a exportação da árvore via `export_session(format="markdown")` diretamente na janela de chat/stdout ativo.
*   A runtime Rust/Tokio realiza a pausa assíncrona da execução, interroga o operador em modo CLI socrático e aguarda a autorização textual humana antes de aplicar o rebase no disco físico.

### 2. Souls State V5 — Schema Relacional Socrático

A migração **V3 → V5** é **idempotente** sob o padrão já estabelecido em `observability::ops::migrate_v2_to_v3`:

```sql
-- socratic_sessions: aggregate root de cada sessão de raciocínio.
CREATE TABLE IF NOT EXISTS socratic_sessions (
    session_id  TEXT PRIMARY KEY STRICT,  -- UUIDv4 simples
    created_at  INTEGER NOT NULL,          -- epoch seconds
    metadata    TEXT NOT NULL DEFAULT '{}' -- JSON blob (tags, task_name, etc.)
) STRICT;

-- socratic_thoughts: cada pensamento é uma linha com parentesco.
-- parent_thought_id é NULLABLE (Tese raiz não tem pai).
-- ON DELETE CASCADE: apagar a sessão purga TODO o grafo dela.
CREATE TABLE IF NOT EXISTS socratic_thoughts (
    thought_id        TEXT PRIMARY KEY STRICT,
    session_id        TEXT NOT NULL,
    branch_id         TEXT NOT NULL DEFAULT 'main',
    parent_thought_id TEXT,
    thought_type      TEXT NOT NULL,  -- 'regular'|'revision'|'branching'
    content           TEXT NOT NULL,
    step_number       INTEGER NOT NULL,
    duration_ms       INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL,
    FOREIGN KEY(session_id)        REFERENCES socratic_sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY(parent_thought_id) REFERENCES socratic_thoughts(thought_id) ON DELETE SET NULL
) STRICT;

-- Índices físicos para acelerar busca de parentesco sintático na RAM Host.
CREATE INDEX IF NOT EXISTS idx_thoughts_session
    ON socratic_thoughts(session_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_branch
    ON socratic_thoughts(branch_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_parent
    ON socratic_thoughts(parent_thought_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_session_step
    ON socratic_thoughts(session_id, step_number);
```

**Lei de Idempotência:** `migrate_v3_to_v5` testa `PRAGMA user_version < 5`; se já é 5, retorna `Ok(())` sem alocar transação.

**Lei do STRICT:** todas as colunas declaram `STRICT` para que o SQLite rejeite coercion silenciosa de tipo (Pessimismo da Razão).

### 3. Reconstrução de Árvore O(1) na RAM Host (não BFS no SQLite)

O `export_session` lê **uma única query** que devolve TODAS as linhas de uma sessão ordenadas por `(branch_id, step_number)`, e reconstrói a árvore de adjacência em **HashMap<ThoughtId, NodeIndex>** na RAM em tempo $O(N)$.

### 4. Análise Comportamental — Fórmulas Fechadas

`analyze_session` computa três métricas que resumem o **FinOps cognitivo** de uma sessão:

$$
\text{revision\_rate} = \frac{|\{t \in T : t.\text{type} = \text{revision}\}|}{|T|}
$$

$$
\text{branching\_factor} = \frac{1}{|B|} \sum_{b \in B} \text{child\_count}(b), \quad B = \text{conjunto de branches}
$$

$$
\text{latency\_mean\_ms} = \frac{1}{|T|} \sum_{t \in T} t.\text{duration\_ms}
$$

### 5. Merge Atômico Last-Write-Wins (Consistência Eventual)

`merge_sessions(source_session_id, target_session_id)` opera sob o **padrão CRDT simplificado** com transação `BEGIN EXCLUSIVE` no `souls_state.db`.

### 6. Três Tools Canônicas sob o Teto 32/120 (ADR-041)

| Tool | Nome | Descrição (≤120 chars) |
|------|------|------------------------|
| Export | `export_session` | "Exporta a árvore relacional de pensamentos socráticos de uma sessão em formato estruturado (JSON/Markdown)." |
| Análise | `analyze_session` | "Processa as métricas comportamentais e de revisão de hipóteses socráticas de uma sessão na RAM." |
| Fusão | `merge_sessions` | "Executa a fusão atômica de ramificações e fluxos de raciocínio concorrentes sob consistência eventual." |

## Consequências

### Positivas
* **Amnésia de raciocínio eliminada.** Sessões persistem em `.souls_data/souls_state.db`; crash do gateway não perde contexto socrático.
* **Auditoria HITL pragmática via Chat.** `export_session` em Markdown é ejetado no chat ativo para o Arquiteto revisar e aprovar interativamente via CLI.
* **FinOps Cognitivo mensurável.** `analyze_session` expõe métricas que alimentam o ParetoBandit.

### Negativas (Aceitas sob Pessimismo da Razão)
* **Custo de I/O por pensamento.** ~50µs por INSERT sob WAL. Aceitável (pensamentos são infrequentes).
* **`last-write-wins` pode perder edições em sessões paralelas.** Aceitável sob consistência eventual.
