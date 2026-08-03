---
id: "ADR-045"
title: "ADR-045-Persistencia-da-Alma-Socratica"
version: 1.0
status: Aprovado
epic: "Cognicao / Persistencia Socratica + Souls State V5"
amends: ["ADR-029", "ADR-040", "ADR-041", "ADR-043", "ADR-044"]
description: "Marco 3.9 Fase E: institui a persistencia relacional do ThinkingEngine socratico no SQLite WAL STRICT, quita a amnésia de raciocínio do gateway souls_mcp e adiciona as 3 tools canônicas export_session/analyze_session/merge_sessions sob o teto 32/120."
mathematical_anchors: ["O(1)_adjacency_reconstruction", "ON_DELETE_CASCADE_subgraph_purge", "last_write_wins_merge"]
physical_paths: ["src-tauri\\src\\cognition\\thinking\\persistence.rs", "src-tauri\\src\\cognition\\thinking\\ops.rs", "src-tauri\\src\\cognition\\thinking\\analytics.rs", "src-tauri\\src\\bin\\souls_mcp_server.rs", ".souls_data\\souls_state.db"]
pr: "https://github.com/brunosrosa/souls_mc/pull/21 (commits cumulativos)"
test_coverage: "39/39 verdes em <0.2s (Fast Pass Marcha Rapida)"
---

# ADR-045: Persistência da Alma Socrática e Quitação da Amnésia de Raciocínio

## Status

**Aprovado (Marco 3.9 Fase E).** Emenda cumulativa das ADRs 029 (Visão Cognitiva O(1)), 040 (Souls State), 041 (Servername Soberano `souls_mcp`), 043 (Observabilidade Sensorial) e 044 (Enjaulamento Wasmtime). Quita o **déficit de memória de raciocínio** do `ThinkingEngine` (até então in-RAM-only) instituindo a **persistência relacional SQLite WAL STRICT** de árvores socráticas, e expõe 3 tools MCP canônicas para reconstrução, análise e fusão atômica.

## Contexto Técnico e Desafio Operacional

O `ThinkingEngine` (PRD-032 §3) canibalizou o `ultrafast-mcp-sequential-thinking` com sucesso mas opera **estritamente in-RAM**:

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

1. **`export_session`** — Reconstruir a árvore relacional para auditoria humana (HITL) ou para apresentação a outro agente.
2. **`analyze_session`** — Computar métricas comportamentais (taxa de revisão, fan-out médio, latência média) para FinOps cognitivo.
3. **`merge_sessions`** — Fundir dois fluxos de raciocínio concorrentes sob consistência eventual, especialmente em workflows multi-agente onde o `swarm_dispatcher` paraleliza pensamentos e precisa reconciliar divergências.

Sem persistência, estas 3 tools só podem operar enquanto o `ThinkingEngine` está vivo em memória — uma janela de ~segundos durante o dispatch MCP. **A amnésia é estrutural**, não acidental.

## Decisões de Engenharia e Arquitetura

### 1. Souls State V5 — Schema Relacional Socrático

A migração **V3 → V5** (pulamos V4 intencionalmente; o número 4 é reservado para o Marco 4.x do swarm reativo) é **idempotente** sob o padrão já estabelecido em `observability::ops::migrate_v2_to_v3`:

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

**Lei do STRICT:** todas as 11 colunas do novo schema declaram `STRICT` para que o SQLite rejeite coercion silenciosa de tipo (princípio do Pessimismo da Razão do `user_profile`).

### 2. Reconstrução de Árvore O(1) na RAM Host (não BFS no SQLite)

O `export_session` lê **uma única query** que devolve TODAS as linhas de uma sessão ordenadas por `(branch_id, step_number)`, e reconstrói a árvore de adjacência em **HashMap<ThoughtId, NodeIndex>** na RAM. A justificativa é tripla:

1. **Latência:** Para uma sessão típica de ≤ 7 pensamentos (5 padrão + 2 HITL), a query retorna ≤ 7 linhas; o overhead de múltiplas queries round-trip seria pior.
2. **Previsibilidade:** O loop de reconstrução é puro, sem side effects; falha não corrompe estado.
3. **Determinismo:** A estrutura de adjacência reconstruída é canônica (mesma entrada → mesma árvore), o que é uma propriedade essencial para o test `test_export_session_formatting`.

A query canônica é:

```sql
SELECT thought_id, branch_id, parent_thought_id, thought_type, content,
       step_number, duration_ms
FROM socratic_thoughts
WHERE session_id = ?1
ORDER BY branch_id, step_number ASC;
```

### 3. Análise Comportamental — Fórmulas Fechadas

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

A **taxa de revisão** indica o quanto o agente se autocorrige (sinal de qualidade); o **fator de ramificação** indica exploração (sinal de profundidade); a **latência média** indica custo (sinal FinOps direto).

### 4. Merge Atômico Last-Write-Wins (Consistência Eventual)

`merge_sessions(source_session_id, target_session_id)` opera sob o **padrão CRDT simplificado**:

1. **Transação BEGIN EXCLUSIVE** no `souls_state.db` (write lock global).
2. Para cada pensamento de `source`: INSERT OR IGNORE em `target` com `thought_id` regenerado (evita colisão de PK). Se já existe, **last-write-wins** baseado em `created_at`.
3. **Remap de `parent_thought_id`:** se o pai de um pensamento movido estava em `source`, reescreve para apontar para o equivalente em `target` (busca por `(step_number, branch_id)`).
4. **COMMIT atômico.** Falha em qualquer ponto = ROLLBACK automático.

A garantia é **eventual consistency** sob a hipótese de que os `created_at` dos pensamentos são monotonicamente ordenados (garantido pelo relógio do Tokio).

### 5. Três Tools Canônicas sob o Teto 32/120 (ADR-041)

| Tool | Nome | Descrição (≤120 chars) |
|------|------|------------------------|
| Export | `export_session` | "Exporta a árvore relacional de pensamentos socráticos de uma sessão em formato estruturado (JSON/Markdown)." |
| Análise | `analyze_session` | "Processa as métricas comportamentais e de revisão de hipóteses socráticas de uma sessão na RAM." |
| Fusão | `merge_sessions` | "Executa a fusão atômica de ramificações e fluxos de raciocínio concorrentes sob consistência eventual." |

Cada tool aceita os 3 aliases canônicos (`name | souls_name | ctx_name`) no dispatcher, mantendo compatibilidade de transição.

## Equações de Validação

### Footprint de Memória

$$
\text{Memória}_{V5} \approx |T| \times 256 \text{ bytes} + \text{overhead SQLite WAL}
$$

Para 7 pensamentos, $\approx 1.8\text{ KB}$. Cabe em L1 do CPU.

### Custo de Reconstrução

$$
T_{\text{reconstruct}}(n) = \mathcal{O}(n), \quad n = \text{|T|}
$$

O loop de adjacência é linear; não há BFS, não há recursão profunda (depth ≤ 5 por causa do disjuntor cognitivo).

## Testes de Homologação (TDD Fast Pass < 0.2s)

| # | Teste | Validação |
|---|-------|-----------|
| 1 | `test_database_migration_v5` | Migração idempotente; FK rejeita inserção inválida. |
| 2 | `test_export_session_formatting` | Tese→Antítese→Síntese; JSON e Markdown respeitam indentação. |
| 3 | `test_analyze_session_metrics` | Taxa de revisão, fan-out, latência média calculados corretamente. |
| 4 | `test_merge_sessions_atomic_last_write_wins` | 2 branches concorrentes fundidas; ponteiros reconciliados. |

## Consequências

### Positivas

* **Amnésia de raciocínio eliminada.** Sessões persistem em `.souls_data/souls_state.db`; crash do gateway não perde contexto socrático.
* **Auditoria HITL viável.** `export_session` em Markdown é legível para o Arquiteto revisar antes de merge.
* **FinOps Cognitivo mensurável.** `analyze_session` expõe métricas que alimentam o ParetoBandit.
* **Merge multi-agente funcional.** O `swarm_dispatcher` pode paralelizar pensamentos e reconciliar divergências.

### Negativas (Aceitas sob Pessimismo da Razão)

* **Custo de I/O por pensamento.** ~50µs por INSERT sob WAL. Aceitável (pensamentos são infrequentes).
* **Esquema precisa de migração manual** se uma coluna for adicionada no futuro. Mitigado por `IF NOT EXISTS` em todas as DDLs.
* **`last-write-wins` pode perder edições em sessões paralelas.** Aceitável sob consistência eventual; para ACID estrito seria necessário um CRDT mais sofisticado (Marco 4.x).

## Referências Cruzadas

- **ADR-029** (Visão Cognitiva O(1)) — A reconstrução O(n) em RAM é justificada por n ≤ 7.
- **ADR-040** (Souls State) — `migrate_v3_to_v5` segue o mesmo padrão de `migrate_v2_to_v3`.
- **ADR-041** (Servername Soberano) — Teto 32/120 respeitado.
- **PRD-032** §3 (Socratic Disjuntor) — `ThinkingEngine` é a fonte da verdade para o disjuntor; persistência é projeção.
