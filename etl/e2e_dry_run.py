"""
e2e_dry_run.py â€” Tiro de Teste E2E (Auditoria de Telemetria).

Executa as 3 Fases do pipeline com um repo real sem gravar no SQLite.
Carrega o .env local via python-dotenv antes de qualquer importaÃ§Ã£o de phases.

Uso:
  uvx --with pydantic --with httpx --with python-dotenv python etl/e2e_dry_run.py
"""
from __future__ import annotations

import asyncio
import json
import logging
import sqlite3
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Carrega .env ANTES de importar phases (que lÃª os os.getenv no topo)
# ---------------------------------------------------------------------------
try:
    from dotenv import load_dotenv
    load_dotenv(Path(__file__).parent.parent / ".env", override=True)
    print("[E2E] .env carregado.")
except ImportError:
    print("[E2E] AVISO: python-dotenv ausente â€” usando variÃ¡veis de ambiente do shell.")

import os
print(f"[E2E] OPENROUTER_FAST_MODEL  = {os.getenv('OPENROUTER_FAST_MODEL', '<nÃ£o definido>')}")
print(f"[E2E] OPENROUTER_FORMATTER   = {os.getenv('OPENROUTER_FORMATTER_MODEL', '<nÃ£o definido>')}")
print(f"[E2E] LENS_UX  = {os.getenv('OPENROUTER_HEAVY_MODEL_LENS_UX', '<nÃ£o definido>')}")
print(f"[E2E] LENS_ARQ = {os.getenv('OPENROUTER_HEAVY_MODEL_LENS_ARQ', '<nÃ£o definido>')}")
print(f"[E2E] LENS_OPS = {os.getenv('OPENROUTER_HEAVY_MODEL_LENS_OPS', '<nÃ£o definido>')}")
print(f"[E2E] OPENAI_BASE_URL = {os.getenv('OPENAI_BASE_URL', '<nÃ£o definido>')}")
key_fast  = os.getenv("OPENROUTER_API_FAST", "")
key_heavy = os.getenv("OPENROUTER_API_HEAVY", "")
print(f"[E2E] API_FAST  presente: {'SIM (' + key_fast[:8] + '...)' if key_fast else 'NÃƒO'}")
print(f"[E2E] API_HEAVY presente: {'SIM (' + key_heavy[:8] + '...)' if key_heavy else 'NÃƒO'}")
print("-" * 72)

from etl.db import init_db
from etl.phases import phase1_kimi, phase2_swarm, phase3_validate

# ---------------------------------------------------------------------------
# ConfiguraÃ§Ã£o do repo de teste (Lote_17 â€” repositÃ³rio real conhecido)
# ---------------------------------------------------------------------------
TEST_REPO = {
    "repo_id":     "tokio-rs/tokio",
    "repo_url":    "https://github.com/tokio-rs/tokio",
    "nome_projeto": "tokio",
    "lote_id":     "LOTE_E2E_TEST",
}

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
logging.basicConfig(
    stream=sys.stdout,
    level=logging.INFO,
    format="%(message)s",
)
logger = logging.getLogger(__name__)


async def run_e2e() -> None:
    repo_id     = TEST_REPO["repo_id"]
    repo_url    = TEST_REPO["repo_url"]
    nome        = TEST_REPO["nome_projeto"]
    lote_id     = TEST_REPO["lote_id"]
    run_id      = "e2e-dry-run-001"

    # Banco in-memory para auditoria (sem persistÃªncia)
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    init_db(conn)

    # -----------------------------------------------------------------
    print(f"\n{'='*72}")
    print(f"[E2E] REPOSITÃ“RIO ALVO: {repo_url}")
    print(f"{'='*72}\n")

    # FASE 1 â€” Kimi K2.5
    print("[E2E][FASE-1] Disparando Kimi K2.5 (timeout=30s)...")
    ctx = await phase1_kimi(repo_url=repo_url, conn=conn, run_id=run_id, repo_id=repo_id)
    print(f"[E2E][FASE-1] âœ“ Resultado:")
    print(f"  primary_language     : {ctx.primary_language}")
    print(f"  domain_hint          : {ctx.domain_hint}")
    print(f"  estimated_complexity : {ctx.estimated_complexity}")
    print(f"  has_rust_components  : {ctx.has_rust_components}")
    print(f"  has_wasm_targets     : {ctx.has_wasm_targets}")
    print(f"  summary              : {ctx.summary[:120]}...")

    # FASE 2 â€” Enxame SocrÃ¡tico (3 Lentes em paralelo)
    print(f"\n[E2E][FASE-2] Disparando Enxame SocrÃ¡tico (asyncio.gather â€” timeout=60s)...")
    print(f"  â†’ Lente A (UX/Produto): {os.getenv('OPENROUTER_HEAVY_MODEL_LENS_UX')}")
    print(f"  â†’ Lente B (Arq):        {os.getenv('OPENROUTER_HEAVY_MODEL_LENS_ARQ')}")
    print(f"  â†’ Lente C (Ops):        {os.getenv('OPENROUTER_HEAVY_MODEL_LENS_OPS')}")

    swarm = await phase2_swarm(
        ctx=ctx,
        repo_url=repo_url,
        conn=conn,
        run_id=run_id,
        repo_id=repo_id,
    )
    print(f"\n[E2E][FASE-2] âœ“ Resultado ({swarm.lentes_disponiveis}/3 lentes disponÃ­veis):")

    if swarm.lente_a:
        print(f"\n  [Lente A â€” UX/Produto]")
        print(f"    score_parcial : {swarm.lente_a.score_parcial}")
        print(f"    flags         : {swarm.lente_a.flags}")
        print(f"    raw_analysis  : {swarm.lente_a.raw_analysis[:200]}...")

    if swarm.lente_b:
        print(f"\n  [Lente B â€” Arquitetura]")
        print(f"    score_parcial : {swarm.lente_b.score_parcial}")
        print(f"    flags         : {swarm.lente_b.flags}")
        print(f"    raw_analysis  : {swarm.lente_b.raw_analysis[:200]}...")

    if swarm.lente_c:
        print(f"\n  [Lente C â€” OperaÃ§Ã£o]")
        print(f"    score_parcial : {swarm.lente_c.score_parcial}")
        print(f"    flags         : {swarm.lente_c.flags}")
        print(f"    raw_analysis  : {swarm.lente_c.raw_analysis[:200]}...")

    if swarm.lentes_disponiveis == 0:
        print("[E2E][FASE-2] âœ— FALHA TOTAL â€” nenhuma lente retornou dados.")
        conn.close()
        return

    # FASE 3 â€” SÃ­ntese Pydantic AI
    print(f"\n[E2E][FASE-3] Sintetizando via Pydantic AI (formatter={os.getenv('OPENROUTER_FORMATTER_MODEL')})...")
    heuristic = await phase3_validate(
        swarm=swarm,
        ctx=ctx,
        repo_url=repo_url,
        repo_id=repo_id,
        lote_id=lote_id,
        nome_projeto=nome,
        conn=conn,
        run_id=run_id,
    )

    print(f"\n[E2E][FASE-3] âœ“ RepoHeuristic Final:")
    print(f"  repo_id                : {heuristic.repo_id}")
    print(f"  score_total            : {heuristic.score_total}")
    print(f"  score_arquitetura      : {heuristic.score_arquitetura}")
    print(f"  score_rust_potential   : {heuristic.score_rust_potential}")
    print(f"  score_bare_metal       : {heuristic.score_bare_metal}")
    print(f"  score_wasm_compat      : {heuristic.score_wasm_compat}")
    print(f"  score_latencia         : {heuristic.score_latencia}")
    print(f"  score_manutencao       : {heuristic.score_manutencao}")
    print(f"  classificacao_terminal : {heuristic.classificacao_terminal}")
    print(f"  categoria_arquitetural : {heuristic.categoria_arquitetural}")
    print(f"  executive_verdict      : {heuristic.executive_verdict}")
    print(f"  justificativa          : {heuristic.justificativa[:120]}...")

    # Auditoria de erros registrados durante o E2E
    erros = conn.execute("SELECT COUNT(*) FROM etl_errors").fetchone()[0]

    print(f"\n{'='*72}")
    print(f"[E2E] DRY-RUN CONCLUÃDO â€” Lentes: {swarm.lentes_disponiveis}/3 | Erros registrados: {erros}")
    print(f"[E2E] NENHUM DADO GRAVADO NO VAULT (dry-run confirmado)")
    print(f"{'='*72}\n")

    conn.close()


if __name__ == "__main__":
    asyncio.run(run_e2e())

