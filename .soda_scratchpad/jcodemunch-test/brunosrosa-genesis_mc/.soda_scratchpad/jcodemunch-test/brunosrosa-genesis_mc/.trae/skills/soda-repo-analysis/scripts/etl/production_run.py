"""
production_run.py — Tiro Real de Produção: Lote PILOT_01.

Processa 3 repositórios canônicos do ecossistema SODA/Rust com gravação
atômica no soda_heuristic_vault.db (Lei Anti-SDC: BEGIN IMMEDIATE / COMMIT).

Uso:
  $env:PYTHONPATH="."; $env:PYTHONUTF8="1"
  uvx --with pydantic --with httpx --with python-dotenv python etl/production_run.py
"""
from __future__ import annotations

import asyncio
import logging
import sqlite3
import sys
import uuid
from pathlib import Path

try:
    from dotenv import load_dotenv
    load_dotenv(Path(__file__).parent.parent / ".env", override=True)
except ImportError:
    pass

from etl.db import atomic_write, create_run_log, finalize_run_log, init_db
from etl.phases import phase1_kimi, phase2_swarm, phase3_validate

logging.basicConfig(
    stream=sys.stdout,
    level=logging.INFO,
    format="[%(asctime)s] %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Micro-lote PILOT_01 — 3 repositórios canônicos do ecossistema SODA
# ---------------------------------------------------------------------------
LOTE_ID = "LOTE_PILOT_01"
BATCH = [
    {
        "repo_id":      "BurntSushi/ripgrep",
        "repo_url":     "https://github.com/BurntSushi/ripgrep",
        "nome_projeto": "ripgrep",
    },
    {
        "repo_id":      "serde-rs/serde",
        "repo_url":     "https://github.com/serde-rs/serde",
        "nome_projeto": "serde",
    },
    {
        "repo_id":      "dtolnay/anyhow",
        "repo_url":     "https://github.com/dtolnay/anyhow",
        "nome_projeto": "anyhow",
    },
]

VAULT_PATH = Path(__file__).parent.parent / "soda_heuristic_vault.db"


async def run_production() -> int:
    run_id = str(uuid.uuid4())
    logger.info("[PROD] Iniciando — run_id=%s lote=%s vault=%s", run_id, LOTE_ID, VAULT_PATH)

    conn = sqlite3.connect(str(VAULT_PATH))
    conn.row_factory = sqlite3.Row
    init_db(conn)

    create_run_log(conn, run_id, LOTE_ID, repos_total=len(BATCH))

    repos_ok = 0
    repos_erro = 0
    ultimo_erro: str | None = None

    for entry in BATCH:
        repo_id  = entry["repo_id"]
        repo_url = entry["repo_url"]
        nome     = entry["nome_projeto"]

        logger.info("\n%s\n[PROD] Processando: %s\n%s", "="*68, repo_id, "="*68)

        try:
            # Fase 1
            ctx = await phase1_kimi(repo_url=repo_url, conn=conn, run_id=run_id, repo_id=repo_id)
            logger.info("[FASE-1] lang=%-12s domain=%-15s complexity=%s",
                        ctx.primary_language, ctx.domain_hint, ctx.estimated_complexity)

            # Fase 2
            swarm = await phase2_swarm(ctx=ctx, repo_url=repo_url, conn=conn, run_id=run_id, repo_id=repo_id)
            logger.info("[FASE-2] lentes=%d/3  scores: A=%-4s B=%-4s C=%-4s",
                        swarm.lentes_disponiveis,
                        f"{swarm.lente_a.score_parcial:.1f}" if swarm.lente_a else "N/A",
                        f"{swarm.lente_b.score_parcial:.1f}" if swarm.lente_b else "N/A",
                        f"{swarm.lente_c.score_parcial:.1f}" if swarm.lente_c else "N/A")

            if swarm.lentes_disponiveis == 0:
                raise RuntimeError("Todas as 3 Lentes falharam — dados insuficientes")

            # Fase 3
            heuristic = await phase3_validate(
                swarm=swarm, ctx=ctx,
                repo_url=repo_url, repo_id=repo_id,
                lote_id=LOTE_ID, nome_projeto=nome,
                conn=conn, run_id=run_id,
            )
            logger.info("[FASE-3] score=%.2f → %-12s categoria=%s",
                        heuristic.score_total, heuristic.classificacao_terminal,
                        heuristic.categoria_arquitetural)
            logger.info("[FASE-3] verdict: %s", heuristic.executive_verdict)

            # Gravacao atomica — BEGIN IMMEDIATE / COMMIT
            atomic_write(conn, heuristic, run_id)
            logger.info("[DB] Gravado atomicamente no vault: %s", repo_id)

            repos_ok += 1

        except Exception as exc:
            logger.error("[ERRO] %s: %s", repo_id, exc)
            repos_erro += 1
            ultimo_erro = str(exc)[:400]

    finalize_run_log(conn, run_id, repos_ok, repos_erro, ultimo_erro)
    conn.close()

    logger.info("\n%s", "="*68)
    logger.info("[PROD] CONCLUIDO — OK=%d  ERRO=%d  run_id=%s", repos_ok, repos_erro, run_id)
    logger.info("[PROD] Vault: %s", VAULT_PATH)
    logger.info("%s\n", "="*68)

    return 0 if repos_erro == 0 else 1


if __name__ == "__main__":
    sys.exit(asyncio.run(run_production()))
