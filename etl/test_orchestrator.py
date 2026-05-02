"""
test_orchestrator.py — T-09: Testes de Integração do ETL Orchestrator.

Valida:
  - init_db + schema completo (3 tabelas)
  - create_run_log + finalize_run_log (ciclo de vida do run)
  - process_repo com mocks das 3 fases (sem rede)
  - dry_run não grava no SQLite
  - finalize_run_log → status COMPLETED / PARTIAL / FAILED
"""
from __future__ import annotations

import asyncio
import sqlite3
from pathlib import Path
from unittest.mock import AsyncMock, patch

import pytest

from etl.db import create_run_log, finalize_run_log, init_db
from etl.models import (
    LenteOutput,
    RepoContext,
    RepoHeuristic,
    SwarmResult,
)
from etl.etl_orchestrator import RepoCatalogEntry, process_repo


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def mem_conn() -> sqlite3.Connection:
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    init_db(conn)
    yield conn
    conn.close()


def _mock_entry(repo_id: str = "owner/repo-mock") -> RepoCatalogEntry:
    return RepoCatalogEntry(
        row_index=2,
        repo_id=repo_id,
        repo_url=f"https://github.com/{repo_id}",
        nome_projeto="repo-mock",
        lote_id="LOTE_TEST",
    )


def _mock_ctx() -> RepoContext:
    return RepoContext(
        primary_language="Rust",
        domain_hint="cli-tool",
        summary="A fast CLI tool written in Rust.",
        has_rust_components=True,
        has_wasm_targets=False,
        estimated_complexity="MED",
    )


def _mock_swarm() -> SwarmResult:
    return SwarmResult(
        lente_a=LenteOutput(raw_analysis="Excelente DX.", score_parcial=8.0, flags=["has_docs"]),
        lente_b=LenteOutput(raw_analysis="Arquitetura modular.", score_parcial=9.0, flags=["has_tokio", "ffi_friendly"]),
        lente_c=LenteOutput(raw_analysis="CI sólido, MIT license.", score_parcial=7.5, flags=["good_ci"]),
        lentes_disponiveis=3,
    )


def _mock_heuristic(repo_id: str = "owner/repo-mock") -> RepoHeuristic:
    return RepoHeuristic(
        repo_id=repo_id,
        repo_url=f"https://github.com/{repo_id}",
        lote_id="LOTE_TEST",
        nome_projeto="repo-mock",
        primary_language="Rust",
        domain_hint="cli-tool",
        summary_kimi="A fast CLI tool.",
        score_total=8.3,
        score_arquitetura=9.0,
        score_rust_potential=8.6,
        score_bare_metal=8.85,
        score_wasm_compat=8.7,
        score_latencia=8.1,
        score_manutencao=7.975,
        executive_verdict="Excelente candidato PLANO_B — Rust nativo, CLI com boa DX.",
        classificacao_terminal="PLANO_B",
        categoria_arquitetural="ffi-bindable",
        justificativa="Score=8.3 | Flags: has_docs, has_tokio, ffi_friendly, good_ci",
        etl_phase_completed=3,
    )


# ---------------------------------------------------------------------------
# T-09-A: Schema completo (3 tabelas criadas)
# ---------------------------------------------------------------------------

def test_init_db_creates_three_tables(mem_conn: sqlite3.Connection):
    """Valida que init_db() cria repo_heuristics, etl_run_log e etl_errors."""
    tables = {
        row[0]
        for row in mem_conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table'"
        ).fetchall()
    }
    assert "repo_heuristics" in tables
    assert "etl_run_log" in tables
    assert "etl_errors" in tables


# ---------------------------------------------------------------------------
# T-09-B: Ciclo de vida do run_log (RUNNING → COMPLETED)
# ---------------------------------------------------------------------------

def test_run_log_lifecycle(mem_conn: sqlite3.Connection):
    """Valida create_run_log + finalize_run_log → status COMPLETED."""
    run_id = "run-lifecycle-test"
    create_run_log(mem_conn, run_id, "LOTE_TEST", repos_total=3)

    row = mem_conn.execute(
        "SELECT status, repos_total FROM etl_run_log WHERE run_id=?", (run_id,)
    ).fetchone()
    assert row["status"] == "RUNNING"
    assert row["repos_total"] == 3

    finalize_run_log(mem_conn, run_id, repos_ok=3, repos_erro=0)

    row = mem_conn.execute(
        "SELECT status, repos_ok, repos_erro FROM etl_run_log WHERE run_id=?", (run_id,)
    ).fetchone()
    assert row["status"] == "COMPLETED"
    assert row["repos_ok"] == 3
    assert row["repos_erro"] == 0


def test_run_log_partial_status(mem_conn: sqlite3.Connection):
    """Valida que repos_erro > 0 mas repos_ok > 0 gera status PARTIAL."""
    run_id = "run-partial-test"
    create_run_log(mem_conn, run_id, "LOTE_TEST", repos_total=5)
    finalize_run_log(mem_conn, run_id, repos_ok=3, repos_erro=2, ultimo_erro="Falha em owner/x")

    row = mem_conn.execute(
        "SELECT status FROM etl_run_log WHERE run_id=?", (run_id,)
    ).fetchone()
    assert row["status"] == "PARTIAL"


def test_run_log_failed_status(mem_conn: sqlite3.Connection):
    """Valida que repos_ok == 0 gera status FAILED."""
    run_id = "run-failed-test"
    create_run_log(mem_conn, run_id, "LOTE_TEST", repos_total=2)
    finalize_run_log(mem_conn, run_id, repos_ok=0, repos_erro=2)

    row = mem_conn.execute(
        "SELECT status FROM etl_run_log WHERE run_id=?", (run_id,)
    ).fetchone()
    assert row["status"] == "FAILED"


# ---------------------------------------------------------------------------
# T-09-C: process_repo com mocks das 3 fases (sem rede)
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
async def test_process_repo_success(mem_conn: sqlite3.Connection):
    """
    Valida que process_repo() grava o RepoHeuristic no banco quando as 3 fases
    retornam mock com sucesso.
    """
    run_id = "run-process-test"
    create_run_log(mem_conn, run_id, "LOTE_TEST", repos_total=1)
    entry = _mock_entry()

    with (
        patch("etl.etl_orchestrator.phase1_kimi",      new_callable=AsyncMock, return_value=_mock_ctx()),
        patch("etl.etl_orchestrator.phase2_swarm",     new_callable=AsyncMock, return_value=_mock_swarm()),
        patch("etl.etl_orchestrator.phase3_validate",  new_callable=AsyncMock, return_value=_mock_heuristic()),
    ):
        result = await process_repo(entry, mem_conn, run_id, dry_run=False)

    assert result is not None
    assert result.classificacao_terminal == "PLANO_B"

    # Confirma que o dado foi gravado no vault
    row = mem_conn.execute(
        "SELECT repo_id, classificacao_terminal, score_total "
        "FROM repo_heuristics WHERE repo_id=?",
        (entry.repo_id,),
    ).fetchone()
    assert row is not None
    assert row["classificacao_terminal"] == "PLANO_B"


@pytest.mark.asyncio
async def test_process_repo_dry_run_nao_grava(mem_conn: sqlite3.Connection):
    """
    Valida que dry_run=True não grava nenhum dado no SQLite.
    """
    run_id = "run-dryrun-test"
    create_run_log(mem_conn, run_id, "LOTE_TEST", repos_total=1)
    entry = _mock_entry(repo_id="owner/repo-dry")

    with (
        patch("etl.etl_orchestrator.phase1_kimi",      new_callable=AsyncMock, return_value=_mock_ctx()),
        patch("etl.etl_orchestrator.phase2_swarm",     new_callable=AsyncMock, return_value=_mock_swarm()),
        patch("etl.etl_orchestrator.phase3_validate",  new_callable=AsyncMock, return_value=_mock_heuristic("owner/repo-dry")),
    ):
        result = await process_repo(entry, mem_conn, run_id, dry_run=True)

    assert result is not None  # pipeline rodou

    # Confirma ZERO gravação no vault
    row = mem_conn.execute(
        "SELECT repo_id FROM repo_heuristics WHERE repo_id=?",
        ("owner/repo-dry",),
    ).fetchone()
    assert row is None, "dry_run não deveria gravar no SQLite"


@pytest.mark.asyncio
async def test_process_repo_fase1_falha_continua(mem_conn: sqlite3.Connection):
    """
    Valida que falha na Fase 1 não aborta o processo — usa contexto fallback.
    phase1_kimi retorna RepoContext(domain_hint='unknown') mesmo em falha.
    """
    run_id = "run-fase1-fail-test"
    create_run_log(mem_conn, run_id, "LOTE_TEST", repos_total=1)
    entry = _mock_entry(repo_id="owner/repo-fallback")

    ctx_fallback = RepoContext(
        primary_language="unknown",
        domain_hint="unknown",
        summary="[Fase 1 falhou — contexto parcial]",
        estimated_complexity="MED",
    )

    with (
        patch("etl.etl_orchestrator.phase1_kimi",     new_callable=AsyncMock, return_value=ctx_fallback),
        patch("etl.etl_orchestrator.phase2_swarm",    new_callable=AsyncMock, return_value=_mock_swarm()),
        patch("etl.etl_orchestrator.phase3_validate", new_callable=AsyncMock,
              return_value=_mock_heuristic("owner/repo-fallback")),
    ):
        result = await process_repo(entry, mem_conn, run_id, dry_run=False)

    assert result is not None, "Fallback de Fase 1 não deveria retornar None"
