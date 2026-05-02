# Tasks: ETL Orchestrator — `etl_orchestrator.py`
**Status:** AGUARDANDO AUTORIZAÇÃO DE MUTAÇÃO  
**Versão:** 1.0.0  
**Design ref:** `docs/design_etl_orchestrator.md` v0.2.0  
**Data:** 2026-05-01

---

## Fase 0 — Scaffold e Ambiente

- [x] **T-00** — Criar estrutura de diretórios e arquivos de scaffold
  - **Ação:** Criar `etl/` com `__init__.py`, `etl_orchestrator.py`, `models.py`, `db.py`, `phases.py`, `test_phases.py`
  - **Método:** `write_to_file` + estrutura de pacote Python mínima
  - **Critério de Sucesso:** `python -c "import etl"` executa sem erros

- [x] **T-01** — Criar `db.py` — Inicialização do `soda_heuristic_vault.db`
  - **Ação:** Implementar `init_db(conn: sqlite3.Connection) -> None` com `CREATE TABLE IF NOT EXISTS` das 3 tabelas (`repo_heuristics`, `etl_run_log`, `etl_errors`) conforme schema canônico do design v0.2.0
  - **Método:** `sqlite3` nativo. `conn.execute()` + `conn.commit()`. **ZERO** ORM, **ZERO** migration framework
  - **Snippet base:**
    ```python
    def init_db(conn: sqlite3.Connection) -> None:
        conn.executescript("""
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;
        CREATE TABLE IF NOT EXISTS repo_heuristics ( ... );
        CREATE TABLE IF NOT EXISTS etl_run_log ( ... );
        CREATE TABLE IF NOT EXISTS etl_errors ( ... );
        """)
        conn.commit()
    ```
  - **Critério de Sucesso:** `python -c "import sqlite3, etl.db; conn=sqlite3.connect(':memory:'); etl.db.init_db(conn)"` → Exit Code 0

---

## Fase 1 — Modelos Pydantic (Contratos de Dados)

- [x] **T-02** — Criar `models.py` — Contratos Pydantic das 3 Fases
  - **Ação:** Declarar `RepoContext`, `LenteOutput`, `SwarmResult`, `RepoHeuristic` conforme especificado no design v0.2.0
  - **Regras críticas:**
    - `RepoHeuristic.classificacao_terminal`: `Literal["STACK_CORE", "PLANO_B", "RADAR", "DESCARTE"]`
    - `RepoHeuristic.executive_verdict`: `Field(max_length=400)`
    - `RepoHeuristic.justificativa`: `Field(max_length=600)`
    - Campos `lente_a/b/c`: `str | None` (tolerância a falha de Lente)
  - **Critério de Sucesso:** `python -m pytest etl/test_phases.py::test_models_contract -v` → PASSED

---

## Fase 2 — TDD Red/Green (Escrever Testes Antes do Código)

- [x] **T-03** — Escrever testes unitários em `test_phases.py` **ANTES** da implementação (Red)
  - **Ação:** Escrever os seguintes testes que **devem falhar** (Exit Code != 0) antes da implementação:
    - `test_phase3_classification_rules()` — valida que score 8.5 → STACK_CORE, 6.5 → PLANO_B, 4.0 → RADAR, 3.9 → DESCARTE
    - `test_atomic_write_rollback()` — simula falha de `COMMIT` e valida `ROLLBACK` + registro em `etl_errors`
    - `test_swarm_partial_failure()` — valida que falha de 1 Lente não aborta o lote (lente=None, lentes_disponiveis=2)
    - `test_executive_verdict_max_length()` — valida truncagem do `executive_verdict` em 400 chars
  - **Método:** `pytest` + `unittest.mock` para mockar chamadas OpenRouter
  - **Critério Red:** `python -m pytest etl/test_phases.py -v` → **4 FAILED** (comportamento esperado nesta etapa)

---

## Fase 3 — Implementação por Fase do Pipeline

- [x] **T-04** — Implementar `phase1_kimi()` em `phases.py`
  - **Ação:** Chamada HTTP via `httpx.AsyncClient` ao OpenRouter (`kimi-k2`) com timeout de 30s. Retorna `RepoContext`.
  - **Fallback:** `except (httpx.TimeoutException, ValidationError)` → preenche `domain_hint="unknown"`, `estimated_complexity="MED"`. Registra em `etl_errors(fase=1)`.
  - **Snippet base:**
    ```python
    async def phase1_kimi(repo_url: str, conn: sqlite3.Connection, run_id: str) -> RepoContext:
        async with httpx.AsyncClient(timeout=30.0) as client:
            resp = await client.post(OPENROUTER_URL, json={
                "model": "moonshotai/kimi-k2",
                "messages": [{"role": "user", "content": KIMI_PROMPT.format(url=repo_url)}],
                "response_format": {"type": "json_object"}
            }, headers=OPENROUTER_HEADERS)
        return RepoContext.model_validate_json(resp.text)
    ```
  - **Critério de Sucesso:** `test_phase1_kimi_fallback()` → PASSED

- [x] **T-05** — Implementar `phase2_swarm()` em `phases.py` — Map-Reduce Socrático
  - **Ação:** `asyncio.gather(lente_a(ctx), lente_b(ctx), lente_c(ctx), return_exceptions=True)` com timeout global de 60s. Cada Lente é uma corotina `async def` separada chamando o modelo OpenRouter correspondente.
  - **Modelos:**
    - `lente_a()` → `anthropic/claude-opus-4.7`
    - `lente_b()` → `deepseek/deepseek-v4-pro`
    - `lente_c()` → `zhipuai/glm-5`
  - **Tratamento de falha:** Se `isinstance(result, Exception)` → `log_error(conn, run_id, repo_id, fase=2, exc)` + preenche `lente=None`
  - **Critério de Sucesso:** `test_swarm_partial_failure()` → PASSED (Green)

- [x] **T-06** — Implementar `phase3_validate()` em `phases.py` — Síntese Pydantic AI
  - **Ação:** Agrega os 3 `LenteOutput` (ou 2 em caso de falha isolada). Calcula `score_total` como média ponderada dos `score_parcial` disponíveis. Aplica regras de classificação. Gera `executive_verdict` via prompt de síntese ao OpenRouter (modelo leve: `google/gemini-flash-1.5`).
  - **Regras de classificação:**
    ```python
    if score_total >= 8.5: classificacao = "STACK_CORE"
    elif score_total >= 6.5: classificacao = "PLANO_B"
    elif score_total >= 4.0: classificacao = "RADAR"
    else: classificacao = "DESCARTE"
    ```
  - **Critério de Sucesso:** `test_phase3_classification_rules()` + `test_executive_verdict_max_length()` → PASSED (Green)

- [x] **T-07** — Implementar `atomic_write()` em `db.py`
  - **Ação:** `BEGIN IMMEDIATE` → `INSERT OR REPLACE INTO repo_heuristics` → `COMMIT`. On failure: `ROLLBACK` + `log_error()`.
  - **Lei Anti-SDC:** Nunca usar `conn.execute()` fora de um bloco `with conn:` ou sem `try/except` explícito.
  - **Snippet base:**
    ```python
    def atomic_write(conn: sqlite3.Connection, heuristic: RepoHeuristic, run_id: str) -> None:
        try:
            conn.execute("BEGIN IMMEDIATE")
            conn.execute(INSERT_HEURISTIC_SQL, heuristic.model_dump())
            conn.commit()
        except sqlite3.Error as exc:
            conn.rollback()
            log_error(conn, run_id, heuristic.repo_id, fase=3, exc=exc)
            raise
    ```
  - **Critério de Sucesso:** `test_atomic_write_rollback()` → PASSED (Green)

---

## Fase 4 — Loop Principal e Integração

- [x] **T-08** — Implementar `main()` em `etl_orchestrator.py`
  - **Ação:** Orquestrar o loop de micro-lotes. Fluxo:
    ```
    conn = sqlite3.connect(VAULT_DB_PATH)
    init_db(conn)
    run_id = create_run_log(conn, lote_id)
    batch = fetch_batch_from_sheets(lote_id, n=5)    # Via httpx + Google Sheets API
    for repo in batch:
        ctx  = await phase1_kimi(repo.url, conn, run_id)
        swarm = await phase2_swarm(ctx, conn, run_id, repo.id)
        result = await phase3_validate(swarm, ctx, conn, run_id, repo.id)
        atomic_write(conn, result, run_id)
    finalize_run_log(conn, run_id)
    update_sheets_status(batch)                      # batch_update_cells → PROCESSADO
    ```
  - **Critério de Sucesso:** Execução manual com 1 repo mock retorna Exit Code 0 e linha gravada no `soda_heuristic_vault.db`

- [x] **T-09** — Validação E2E via MCP (auditoria passiva)
  - **Ação:** Após T-08 com repo mock, auditar o vault via ferramenta MCP:
    ```sql
    SELECT repo_id, score_total, classificacao_terminal,
           substr(executive_verdict, 1, 100),
           substr(lente_a_sentido_ux, 1, 80)
    FROM repo_heuristics ORDER BY processado_em DESC LIMIT 1;
    ```
  - **Critério de Sucesso:** Registro visível via `sqlite_read_query` com todas as colunas preenchidas → **EXIT CODE 0 CONFIRMADO**

---

## Fase 5 — Hardening

- [x] **T-10** — Adicionar `--lote-id` e `--dry-run` como argumentos CLI (`argparse`)
  - `--dry-run`: executa Fases 1-3 mas não grava no SQLite nem atualiza Sheets

- [x] **T-11** — Adicionar logging estruturado (`logging.basicConfig` → stdout)
  - Formato: `[ETL][FASE-N][repo_id] mensagem` — sem deps externas

---

> ⚠️ **AGUARDANDO AUTORIZAÇÃO DO ARQUITETO**  
> Digite **"Iniciar Mutação"** para liberar a Fase de Implementação (T-00 em diante).
> O TDD Red será executado **primeiro** (T-03), antes de qualquer código de produção.
