"""
test_phases.py — T-03: TDD Red Phase

Estes testes DEVEM FALHAR antes da implementação de produção (phases.py vazio).
Execute: pytest etl/test_phases.py -v
Resultado esperado na Red Phase: 4 FAILED.
"""
from __future__ import annotations

import sqlite3
from unittest.mock import MagicMock, patch

import pytest

# ---------------------------------------------------------------------------
# Imports dos módulos alvo — falhará se os módulos não existirem
# ---------------------------------------------------------------------------
from etl.models import (
    LenteOutput,
    RepoHeuristic,
    SwarmResult,
    classificar,
)
from etl.db import (
    atomic_write,
    init_db,
    log_error,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def mem_conn() -> sqlite3.Connection:
    """Banco in-memory isolado por teste. Sem efeitos colaterais."""
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    init_db(conn)
    yield conn
    conn.close()


def _make_heuristic(**overrides) -> RepoHeuristic:
    """Factory de RepoHeuristic com valores-padrão seguros para testes."""
    defaults = dict(
        repo_id="owner/repo-test",
        repo_url="https://github.com/owner/repo-test",
        lote_id="LOTE_TEST",
        nome_projeto="repo-test",
        score_total=7.0,
        classificacao_terminal="PLANO_B",
        categoria_arquitetural="cli-tool",
        justificativa="Teste unitário",
        executive_verdict="Veredito de teste",
    )
    defaults.update(overrides)
    return RepoHeuristic(**defaults)


# ---------------------------------------------------------------------------
# T-03-A: Regras de classificação terminal
# ---------------------------------------------------------------------------

def test_phase3_classification_rules():
    """
    Valida que classificar() aplica os limiares canônicos SODA V3 corretamente.
    RED: falhará se classificar() não estiver implementada ou retornar valores errados.
    """
    assert classificar(10.0) == "STACK_CORE"
    assert classificar(8.5)  == "STACK_CORE"
    assert classificar(8.49) == "PLANO_B"
    assert classificar(6.5)  == "PLANO_B"
    assert classificar(6.49) == "RADAR"
    assert classificar(4.0)  == "RADAR"
    assert classificar(3.99) == "DESCARTE"
    assert classificar(0.0)  == "DESCARTE"


# ---------------------------------------------------------------------------
# T-03-B: atomic_write + ROLLBACK em falha de COMMIT
# ---------------------------------------------------------------------------

def test_atomic_write_rollback():
    """
    Valida que atomic_write() faz ROLLBACK e relança a exceção ao falhar no INSERT.
    Python 3.14: sqlite3.Connection.execute é C-extension read-only — não aceita
    patch.object. Solução: subclasse com override de execute() para injetar falha.
    RED: falharia se atomic_write() não capturasse sqlite3.Error ou não fizesse ROLLBACK.
    """

    class FaultyConnection(sqlite3.Connection):
        """Subclasse que falha na 2ª chamada a execute() (INSERT OR REPLACE)."""

        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self._call_count = 0

        def execute(self, sql, *args, **kwargs):  # type: ignore[override]
            self._call_count += 1
            if self._call_count == 2:
                raise sqlite3.OperationalError("forced failure")
            return super().execute(sql, *args, **kwargs)

    # Cria conexão faulty in-memory e inicializa o schema
    conn = FaultyConnection(":memory:")
    conn.row_factory = sqlite3.Row
    # init_db usa executescript (não execute) → não é interceptado, schema OK
    init_db(conn)

    heuristic = _make_heuristic()
    run_id = "run-rollback-test"

    with pytest.raises(sqlite3.OperationalError, match="forced failure"):
        atomic_write(conn, heuristic, run_id)

    # Reseta o contador e verifica que ROLLBACK foi efetivo
    conn._call_count = -999  # nunca mais dispara a falha
    count_after = sqlite3.Connection.execute(
        conn, "SELECT COUNT(*) FROM repo_heuristics"
    ).fetchone()[0]
    assert count_after == 0, "ROLLBACK falhou — dado não deveria estar gravado"
    conn.close()



# ---------------------------------------------------------------------------
# T-03-C: Falha parcial do Enxame (1 Lente → None, lotes não abortados)
# ---------------------------------------------------------------------------

def test_swarm_partial_failure():
    """
    Valida que SwarmResult aceita lente_c=None e contabiliza lentes_disponiveis=2.
    RED: falhará se SwarmResult não aceitar None para lente_c.
    """
    lente_a = LenteOutput(raw_analysis="Análise UX sólida.", score_parcial=7.5, flags=["has_docs"])
    lente_b = LenteOutput(raw_analysis="Arquitetura modular.", score_parcial=8.0, flags=["has_tokio"])

    swarm = SwarmResult(
        lente_a=lente_a,
        lente_b=lente_b,
        lente_c=None,  # GLM-5 falhou — não aborta o lote
        lentes_disponiveis=2,
    )

    assert swarm.lente_c is None
    assert swarm.lentes_disponiveis == 2
    assert swarm.lente_a.score_parcial == 7.5
    assert swarm.lente_b.flags == ["has_tokio"]

    # Classificação com dados parciais ainda deve funcionar
    score_medio = (lente_a.score_parcial + lente_b.score_parcial) / 2  # 7.75
    classificacao = classificar(score_medio)
    assert classificacao == "PLANO_B"


# ---------------------------------------------------------------------------
# T-03-D: executive_verdict truncado a 400 chars em atomic_write
# ---------------------------------------------------------------------------

def test_executive_verdict_max_length(mem_conn: sqlite3.Connection):
    """
    Valida que atomic_write() trunca executive_verdict a 400 chars antes de gravar.
    Usa model_construct() para bypass da validação Pydantic (simula dado já 'sujo'
    que chega do OpenRouter antes da truncagem em db.atomic_write).
    RED: falharia se atomic_write() não aplicasse truncagem em exec time.
    """
    verdict_longo = "X" * 600  # 600 chars — excede o limite de 400

    # model_construct() bypassa os validators do Pydantic intencionalmente
    # para simular dado bruto do OpenRouter antes de atomic_write processar.
    heuristic = RepoHeuristic.model_construct(
        repo_id="owner/repo-truncate",
        repo_url="https://github.com/owner/repo-truncate",
        lote_id="LOTE_TEST",
        nome_projeto="repo-truncate",
        score_total=7.0,
        classificacao_terminal="PLANO_B",
        categoria_arquitetural="cli-tool",
        justificativa="Teste de truncagem",
        executive_verdict=verdict_longo,  # 600 chars intencionais
        etl_phase_completed=3,
    )
    run_id = "run-truncate-test"

    # Cria o run_log antes de gravar (FK obrigatória)
    mem_conn.execute(
        "INSERT INTO etl_run_log (run_id, iniciado_em, lote_processado, status) "
        "VALUES (?, ?, ?, 'RUNNING')",
        (run_id, "2026-05-01T00:00:00+00:00", "LOTE_TEST"),
    )
    mem_conn.commit()

    atomic_write(mem_conn, heuristic, run_id)

    row = mem_conn.execute(
        "SELECT executive_verdict FROM repo_heuristics WHERE repo_id=?",
        (heuristic.repo_id,),
    ).fetchone()

    assert row is not None
    assert len(row["executive_verdict"]) == 400, (
        f"Esperado 400 chars, obtido {len(row['executive_verdict'])}"
    )
