"""
production_run_v2.py — Tiro Real PILOT_01 com Tríade SODA Completa.

Lente C: z-ai/glm-5.1 (67.0% agentic capacity, 43.4 Coding).
Usa LOTE_PILOT_01 com INSERT OR REPLACE — atualiza registros existentes
com dados da Lente C previamente ausentes.

Após COMMIT atômico no SQLite, imprime os dados prontos para injeção
no Google Sheets via MCP (Fase 4 delegada ao agente orquestrador).
"""
from __future__ import annotations

import asyncio
import json
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

import os

from etl.db import atomic_write, create_run_log, finalize_run_log, init_db
from etl.phases import phase1_kimi, phase2_swarm, phase3_validate

logging.basicConfig(
    stream=sys.stdout,
    level=logging.INFO,
    format="[%(asctime)s] %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger(__name__)

LOTE_ID = "LOTE_PILOT_01"
BATCH = [
    {"repo_id": "BurntSushi/ripgrep", "repo_url": "https://github.com/BurntSushi/ripgrep", "nome_projeto": "ripgrep"},
    {"repo_id": "serde-rs/serde",     "repo_url": "https://github.com/serde-rs/serde",     "nome_projeto": "serde"},
    {"repo_id": "dtolnay/anyhow",     "repo_url": "https://github.com/dtolnay/anyhow",     "nome_projeto": "anyhow"},
]
VAULT_PATH = Path(__file__).parent.parent / "soda_heuristic_vault.db"

logger.info("[CONFIG] LENS_OPS = %s", os.getenv("OPENROUTER_HEAVY_MODEL_LENS_OPS"))


async def run_production() -> int:
    run_id = str(uuid.uuid4())
    logger.info("[PROD] run_id=%s lote=%s", run_id, LOTE_ID)

    conn = sqlite3.connect(str(VAULT_PATH))
    conn.row_factory = sqlite3.Row
    init_db(conn)
    create_run_log(conn, run_id, LOTE_ID, repos_total=len(BATCH))

    repos_ok, repos_erro = 0, 0

    for entry in BATCH:
        repo_id, repo_url, nome = entry["repo_id"], entry["repo_url"], entry["nome_projeto"]
        logger.info("\n%s\n[PROD] %s\n%s", "="*68, repo_id, "="*68)

        try:
            ctx   = await phase1_kimi(repo_url=repo_url, conn=conn, run_id=run_id, repo_id=repo_id)
            swarm = await phase2_swarm(ctx=ctx, repo_url=repo_url, conn=conn, run_id=run_id, repo_id=repo_id)

            logger.info("[FASE-2] lentes=%d/3  A=%-4s B=%-4s C=%-4s",
                        swarm.lentes_disponiveis,
                        f"{swarm.lente_a.score_parcial:.1f}" if swarm.lente_a else "N/A",
                        f"{swarm.lente_b.score_parcial:.1f}" if swarm.lente_b else "N/A",
                        f"{swarm.lente_c.score_parcial:.1f}" if swarm.lente_c else "N/A")

            if swarm.lentes_disponiveis == 0:
                raise RuntimeError("Todas as Lentes falharam")

            heuristic = await phase3_validate(
                swarm=swarm, ctx=ctx,
                repo_url=repo_url, repo_id=repo_id,
                lote_id=LOTE_ID, nome_projeto=nome,
                conn=conn, run_id=run_id,
            )
            logger.info("[FASE-3] score=%.2f → %s | %s",
                        heuristic.score_total, heuristic.classificacao_terminal, heuristic.executive_verdict[:80])

            atomic_write(conn, heuristic, run_id)
            repos_ok += 1

        except Exception as exc:
            logger.error("[ERRO] %s: %s", repo_id, exc)
            repos_erro += 1

    finalize_run_log(conn, run_id, repos_ok, repos_erro)

    # -----------------------------------------------------------------------
    # Fase 4 — Dump dos dados prontos para injeção no Sheets via MCP
    # O agente orquestrador (Antigravity) lê este JSON e chama o MCP.
    # -----------------------------------------------------------------------
    rows = conn.execute(
        "SELECT lote_id, nome_projeto, repo_url, score_total, "
        "classificacao_terminal, 'PROCESSADO' AS status, categoria_arquitetural, "
        "score_arquitetura, score_rust_potential, score_bare_metal, "
        "score_wasm_compat, score_latencia, score_manutencao, "
        "substr(executive_verdict, 1, 400) AS executive_verdict, "
        "primary_language, domain_hint, has_rust_components, has_wasm_targets, "
        "lente_a_sentido_ux IS NOT NULL AS lente_a_ok, "
        "lente_b_estrutura_arq IS NOT NULL AS lente_b_ok, "
        "lente_c_realidade_ops IS NOT NULL AS lente_c_ok "
        "FROM repo_heuristics WHERE lote_id=? ORDER BY score_total DESC",
        (LOTE_ID,),
    ).fetchall()

    payload = [dict(r) for r in rows]
    print("\n[FASE-4-PAYLOAD-JSON]")
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    print("[/FASE-4-PAYLOAD-JSON]")

    conn.close()
    logger.info("[PROD] CONCLUIDO — OK=%d ERRO=%d", repos_ok, repos_erro)
    return 0 if repos_erro == 0 else 1


if __name__ == "__main__":
    sys.exit(asyncio.run(run_production()))
