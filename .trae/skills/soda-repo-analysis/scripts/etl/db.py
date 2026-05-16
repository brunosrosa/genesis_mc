"""
db.py — Camada de Persistência bare-metal do ETL Orchestrator.

Lei Anti-SDC:
  - Toda gravação usa BEGIN IMMEDIATE + COMMIT atômico.
  - Falha em COMMIT → ROLLBACK imediato + registro em etl_errors.
  - PROIBIDO: sqlite3 fora de bloco try/except explícito.
  - PROIBIDO: SELECT * em colunas TEXT longas (lente_a/b/c, etc.).
"""
from __future__ import annotations

import logging
import sqlite3
from datetime import datetime, timezone
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from etl.models import RepoHeuristic

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# DDL — Schema Canônico SODA V3
# ---------------------------------------------------------------------------
_DDL = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS repo_heuristics (
    -- PK / FK / Controle
    repo_id                TEXT PRIMARY KEY,
    etl_phase_completed    INTEGER DEFAULT 0,
    processado_em          TEXT,
    etl_run_id             TEXT,

    -- As 45 Colunas Canônicas
    project_name           TEXT,
    declared_description   TEXT,
    repo_url               TEXT NOT NULL,
    score_final            REAL,
    score_fit_geral_soda   REAL,
    score_philosophical_fit REAL,
    score_bare_metal_fit   REAL,
    score_architectural_extractability REAL,
    score_operability      REAL,
    score_creep_risk       REAL,
    entropy_risk           TEXT,
    design_misuse_risk     TEXT,
    intrinsic_ethics_risk  TEXT,
    horizonte_extracao     TEXT,
    justificativa_decisao  TEXT,
    categoria_arquitetural TEXT,
    categoria_nuance_tecnica TEXT,
    classificacao_terminal TEXT,
    stack_base             TEXT,
    tipo_integracao        TEXT,
    integracao_papel_exato TEXT,
    must_components        TEXT,
    proposta_original_resumo TEXT,
    lente_a_sentido_ux     TEXT,
    lente_b_estrutura_arq  TEXT,
    lente_c_realidade_ops  TEXT,
    executive_verdict      TEXT,
    ouro_a_extrair         TEXT,
    deep_pattern           TEXT,
    acao_de_canibalizacao  TEXT,
    transplantable_core    TEXT,
    logic_math_heuristic   TEXT,
    risco_principal        TEXT,
    risco_linha_vermelha   TEXT,
    observacoes            TEXT,
    real_structural_problem TEXT,
    bare_metal_fit         TEXT,
    discipline_dependency  TEXT,
    extractability_level   TEXT,
    operability_level      TEXT,
    where_ai_should_not_enter TEXT,
    do_not_absorb          TEXT,
    data_ultima_analise    TEXT,
    analise_origem         TEXT,
    lote_id                TEXT,

    FOREIGN KEY (etl_run_id) REFERENCES etl_run_log(run_id)
);

CREATE TABLE IF NOT EXISTS etl_run_log (
    run_id          TEXT PRIMARY KEY,
    iniciado_em     TEXT NOT NULL,
    finalizado_em   TEXT,
    lote_processado TEXT NOT NULL,
    repos_total     INTEGER DEFAULT 0,
    repos_ok        INTEGER DEFAULT 0,
    repos_erro      INTEGER DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'RUNNING'
        CHECK(status IN ('RUNNING','COMPLETED','FAILED','PARTIAL')),
    erro_ultimo     TEXT
);

CREATE TABLE IF NOT EXISTS etl_errors (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     TEXT NOT NULL,
    repo_id    TEXT NOT NULL,
    fase       INTEGER NOT NULL,
    erro_tipo  TEXT NOT NULL,
    erro_msg   TEXT,
    timestamp  TEXT NOT NULL
);
"""

_INSERT_HEURISTIC = """
INSERT OR REPLACE INTO repo_heuristics (
    repo_id, etl_phase_completed, processado_em, etl_run_id,
    project_name, declared_description, repo_url, score_final, score_fit_geral_soda,
    score_philosophical_fit, score_bare_metal_fit, score_architectural_extractability,
    score_operability, score_creep_risk, entropy_risk, design_misuse_risk,
    intrinsic_ethics_risk, horizonte_extracao, justificativa_decisao, categoria_arquitetural,
    categoria_nuance_tecnica, classificacao_terminal, stack_base, tipo_integracao,
    integracao_papel_exato, must_components, proposta_original_resumo, lente_a_sentido_ux,
    lente_b_estrutura_arq, lente_c_realidade_ops, executive_verdict, ouro_a_extrair,
    deep_pattern, acao_de_canibalizacao, transplantable_core, logic_math_heuristic,
    risco_principal, risco_linha_vermelha, observacoes, real_structural_problem,
    bare_metal_fit, discipline_dependency, extractability_level, operability_level,
    where_ai_should_not_enter, do_not_absorb, data_ultima_analise, analise_origem, lote_id
) VALUES (
    :repo_id, :etl_phase_completed, :processado_em, :etl_run_id,
    :project_name, :declared_description, :repo_url, :score_final, :score_fit_geral_soda,
    :score_philosophical_fit, :score_bare_metal_fit, :score_architectural_extractability,
    :score_operability, :score_creep_risk, :entropy_risk, :design_misuse_risk,
    :intrinsic_ethics_risk, :horizonte_extracao, :justificativa_decisao, :categoria_arquitetural,
    :categoria_nuance_tecnica, :classificacao_terminal, :stack_base, :tipo_integracao,
    :integracao_papel_exato, :must_components, :proposta_original_resumo, :lente_a_sentido_ux,
    :lente_b_estrutura_arq, :lente_c_realidade_ops, :executive_verdict, :ouro_a_extrair,
    :deep_pattern, :acao_de_canibalizacao, :transplantable_core, :logic_math_heuristic,
    :risco_principal, :risco_linha_vermelha, :observacoes, :real_structural_problem,
    :bare_metal_fit, :discipline_dependency, :extractability_level, :operability_level,
    :where_ai_should_not_enter, :do_not_absorb, :data_ultima_analise, :analise_origem, :lote_id
)
"""

_INSERT_ERROR = """
INSERT INTO etl_errors (run_id, repo_id, fase, erro_tipo, erro_msg, timestamp)
VALUES (:run_id, :repo_id, :fase, :erro_tipo, :erro_msg, :timestamp)
"""

_UPDATE_RUN_LOG_FINAL = """
UPDATE etl_run_log
SET finalizado_em=:finalizado_em, status=:status,
    repos_ok=:repos_ok, repos_erro=:repos_erro, erro_ultimo=:erro_ultimo
WHERE run_id=:run_id
"""


# ---------------------------------------------------------------------------
# API Pública
# ---------------------------------------------------------------------------

def init_db(conn: sqlite3.Connection) -> None:
    """Cria as 3 tabelas canônicas se não existirem. Idempotente.
    AVISO: Se a tabela repo_heuristics já existir com esquema antigo, 
    isto não fará ALTER TABLE. Recomenda-se recriar o DB em caso de schema break.
    """
    conn.executescript(_DDL)
    conn.commit()
    logger.info("[DB] Schema inicializado (WAL + FK ON)")


def create_run_log(conn: sqlite3.Connection, run_id: str, lote_id: str, repos_total: int) -> None:
    """Abre um novo registro de execução com status RUNNING."""
    now = _utcnow()
    try:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute(
            "INSERT INTO etl_run_log (run_id, iniciado_em, lote_processado, repos_total, status) "
            "VALUES (?, ?, ?, ?, 'RUNNING')",
            (run_id, now, lote_id, repos_total),
        )
        conn.commit()
        logger.info("[DB] run_log criado: %s", run_id)
    except sqlite3.Error:
        conn.rollback()
        raise


def atomic_write(conn: sqlite3.Connection, heuristic: "RepoHeuristic", run_id: str) -> None:
    """
    Grava um RepoHeuristic no vault de forma atômica.
    Falha → ROLLBACK + log_error(fase=3) + re-raise.
    """
    payload = heuristic.model_dump()
    
    # Adicionamos metadados de controle interno no payload para o dicionário do INSERT
    import hashlib
    payload["repo_id"] = hashlib.md5(payload["repo_url"].encode("utf-8")).hexdigest()
    payload["etl_phase_completed"] = 3
    payload["processado_em"] = _utcnow()
    payload["etl_run_id"] = run_id
    
    # Proteção VRAM: trunca colunas TEXT longas antes de gravar
    for col in ("lente_a_sentido_ux", "lente_b_estrutura_arq", "lente_c_realidade_ops"):
        if payload.get(col):
            payload[col] = payload[col][:800]
    if payload.get("executive_verdict"):
        payload["executive_verdict"] = payload["executive_verdict"][:400]
    if payload.get("justificativa_decisao"):
        payload["justificativa_decisao"] = payload["justificativa_decisao"][:600]

    try:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute(_INSERT_HEURISTIC, payload)
        conn.commit()
        logger.info("[DB] atomic_write OK: %s", payload["repo_id"])
    except sqlite3.Error as exc:
        conn.rollback()
        log_error(conn, run_id, payload["repo_id"], fase=3, exc=exc)
        raise


def log_error(
    conn: sqlite3.Connection,
    run_id: str,
    repo_id: str,
    fase: int,
    exc: Exception,
) -> None:
    """Registra uma falha isolada sem abortar o lote."""
    try:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute(
            _INSERT_ERROR,
            {
                "run_id": run_id,
                "repo_id": repo_id,
                "fase": fase,
                "erro_tipo": type(exc).__name__,
                "erro_msg": str(exc)[:400],
                "timestamp": _utcnow(),
            },
        )
        conn.commit()
    except sqlite3.Error:
        conn.rollback()
        logger.exception("[DB] Falha ao gravar etl_errors — repo=%s fase=%d", repo_id, fase)


def finalize_run_log(
    conn: sqlite3.Connection,
    run_id: str,
    repos_ok: int,
    repos_erro: int,
    ultimo_erro: str | None = None,
) -> None:
    """Fecha o run_log com status final."""
    status = "COMPLETED" if repos_erro == 0 else ("FAILED" if repos_ok == 0 else "PARTIAL")
    try:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute(
            _UPDATE_RUN_LOG_FINAL,
            {
                "run_id": run_id,
                "finalizado_em": _utcnow(),
                "status": status,
                "repos_ok": repos_ok,
                "repos_erro": repos_erro,
                "erro_ultimo": ultimo_erro[:400] if ultimo_erro else None,
            },
        )
        conn.commit()
        logger.info("[DB] run_log finalizado: %s → %s", run_id, status)
    except sqlite3.Error:
        conn.rollback()
        raise


# ---------------------------------------------------------------------------
# Helpers Internos
# ---------------------------------------------------------------------------

def _utcnow() -> str:
    return datetime.now(timezone.utc).isoformat()
