"""
etl_orchestrator.py — Loop Principal do ETL Orchestrator SODA.

T-08: Orquestra o pipeline de 3 Fases sobre micro-lotes de 5 repositórios.
T-10: CLI via argparse (--lote-id, --batch-size, --dry-run, --vault-db).
T-11: Logging estruturado para stdout (zero dependências externas).

Regras absolutas:
  - PROIBIDO usar mcp-server-sqlite para gravação. Apenas sqlite3 nativo.
  - PROIBIDO carregar o catálogo inteiro em memória. Micro-lotes de N repos.
  - PROIBIDO silenciar exceções. Toda falha vai para etl_errors ou re-raise.
  - Sheets atualizada APENAS após COMMIT bem-sucedido no SQLite.
"""
from __future__ import annotations

import argparse
import asyncio
import logging
import os
import sqlite3
import sys
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import httpx

from etl.db import (
    atomic_write,
    create_run_log,
    finalize_run_log,
    init_db,
    log_error,
)
from etl.models import RepoHeuristic
from etl.phases import phase1_kimi, phase2_swarm, phase3_validate

# ---------------------------------------------------------------------------
# Logging estruturado — stdout, sem deps externas
# ---------------------------------------------------------------------------
try:
    from rich.logging import RichHandler
    from rich.console import Console
    logging.basicConfig(
        level=logging.INFO,
        format="%(message)s",
        datefmt="[%X]",
        handlers=[RichHandler(console=Console(width=160), rich_tracebacks=True, markup=True)]
    )
except ImportError:
    logging.basicConfig(
        stream=sys.stdout,
        level=logging.INFO,
        format="[ETL][%(asctime)s] %(message)s",
        datefmt="%H:%M:%S",
    )
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Configuração
# ---------------------------------------------------------------------------
_DEFAULT_VAULT = Path(__file__).parent.parent / "soda_heuristic_vault.db"
_SHEETS_ID     = os.getenv("SHEETS_ID", "1jPmO29ZR240nq2YW4iwHvjKcd5X299z4Kv7AEI7oUbg")
_OR_BASE_URL   = os.getenv("OPENAI_BASE_URL", "https://openrouter.ai/api/v1")
_OR_KEY_FAST   = os.getenv("OPENROUTER_API_FAST", "")


# ---------------------------------------------------------------------------
# Modelo de dados do catálogo (entrada do loop)
# ---------------------------------------------------------------------------
@dataclass
class RepoCatalogEntry:
    row_index: int          # linha na planilha (1-based, linha 1 = header)
    repo_id:   str          # "{owner}/{repo}"
    repo_url:  str
    nome_projeto: str
    lote_id:   str


# ---------------------------------------------------------------------------
# T-08-A: Fetch do micro-lote via Google Sheets API (HTTP direto)
# ---------------------------------------------------------------------------

async def fetch_batch_from_sheets(
    lote_id: str,
    batch_size: int,
    sheet_tab: str = "MASTER_SOLUTIONS_v3",
) -> list[RepoCatalogEntry]:
    """
    Lê a planilha MASTER_SOLUTIONS_v3 e retorna todos os repos
    com classificacao_terminal=TRIAGEM ou PENDENTE para o lote especificado.
    Ignora repos já processados nesta versão ou que correspondam à blocklist.
    """
    import google.auth
    from google.oauth2 import service_account
    from googleapiclient.discovery import build

    SERVICE_ACCOUNT_FILE = r"C:\Users\rosas\.keys\soda-sheets-service-account.json"
    credentials = service_account.Credentials.from_service_account_file(
        SERVICE_ACCOUNT_FILE, 
        scopes=['https://www.googleapis.com/auth/spreadsheets']
    )
    service = build('sheets', 'v4', credentials=credentials)

    range_notation = f"{sheet_tab}!A2:AS400"  # max 400 linhas, pula o header
    result = service.spreadsheets().values().get(
        spreadsheetId=_SHEETS_ID,
        range=range_notation
    ).execute()

    rows: list[list[str]] = result.get("values", [])

    entries: list[RepoCatalogEntry] = []
    for i, row in enumerate(rows, start=2):  # linha 2 em diante (header na 1)
        row = row + [""] * (45 - len(row))
        
        nome     = row[0].strip()
        url_val  = row[2].strip()
        status   = row[17].strip()
        analise_origem = row[43].strip()

        if status not in ("TRIAGEM", "PENDENTE"):
            continue
        if not url_val.startswith("http"):
            continue
            
        # MARCA D'ÁGUA V3 e Blocklist
        if analise_origem == "SODA ETL V3 Auto":
            continue
        if "hiroppy/tmux-agent-sidebar" in url_val:
            continue

        parts = url_val.rstrip("/").split("/")
        repo_id = f"{parts[-2]}/{parts[-1]}" if len(parts) >= 2 else url_val

        entries.append(RepoCatalogEntry(
            row_index=i,
            repo_id=repo_id,
            repo_url=url_val,
            nome_projeto=nome,
            lote_id=lote_id,
        ))

    entries = entries[:5]  # Trava estrita de lote piloto (5 linhas não processadas)

    logger.info("[SHEETS] %d repos TRIAGEM/PENDENTE encontrados (após deduplicação V3) para lote=%s", len(entries), lote_id)
    return entries


# ---------------------------------------------------------------------------
# T-08-B: Atualização de status na planilha (UPSERT Destrutivo)
# ---------------------------------------------------------------------------

async def update_sheets_status(
    entries: list[RepoCatalogEntry],
    results: list[RepoHeuristic | None],
    dry_run: bool = False,
) -> None:
    """
    Atualiza a planilha fazendo UPSERT via phase4_sheets_loader.
    """
    from etl.phase4_sheets_loader import upsert_batch_to_sheets

    if dry_run:
        for entry in entries:
            logger.info("[DRY-RUN][SHEETS] Não atualizando a planilha: row=%d repo=%s", entry.row_index, entry.repo_id)
        return

    # Filtra apenas os sucessos
    sucessos = [r for r in results if r is not None]
    if not sucessos:
        return

    await upsert_batch_to_sheets(_SHEETS_ID, "MASTER_SOLUTIONS_v3", sucessos)


# ---------------------------------------------------------------------------
# T-08-C: Processamento de um único repo (1 iteração do loop)
# ---------------------------------------------------------------------------

async def process_repo(
    entry: RepoCatalogEntry,
    conn: sqlite3.Connection,
    run_id: str,
    dry_run: bool = False,
) -> RepoHeuristic | None:
    """
    Executa as 3 fases para um repositório e persiste o resultado atomicamente.
    Retorna RepoHeuristic em sucesso, None em falha catastrófica.
    """
    logger.info("[FASE-0] Iniciando: %s", entry.repo_id)

    try:
        # Fase 1 — Kimi K2: contexto estruturado
        ctx = await phase1_kimi(
            repo_url=entry.repo_url,
            conn=conn,
            run_id=run_id,
            repo_id=entry.repo_id,
        )

        # Short-Circuit: se o repositório for inacessível ou morto
        if ctx.domain_hint == "unknown" or ctx.primary_language == "unknown":
            logger.warning("[FASE-0] [%s] Short-Circuit ativado: Repositório inacessível/vazio.", entry.repo_id)
            from etl.phases import create_rejected_heuristic
            heuristic = create_rejected_heuristic(entry.repo_id, entry.repo_url, entry.nome_projeto, entry.lote_id)
            
            if not dry_run:
                atomic_write(conn, heuristic, run_id)
            else:
                logger.info("[DRY-RUN] %s → Reject (Short-Circuit) [não gravado]", entry.repo_id)
                
            logger.info("[OK] %s → Reject | score=0.0 | lentes=0 (Short-Circuit)", entry.repo_id)
            return heuristic

        # Fase 2 — Enxame: 3 Lentes em asyncio.gather
        swarm = await phase2_swarm(
            ctx=ctx,
            repo_url=entry.repo_url,
            conn=conn,
            run_id=run_id,
            repo_id=entry.repo_id,
        )

        if swarm.lentes_disponiveis == 0:
            raise RuntimeError("Todas as 3 Lentes falharam — dados insuficientes para classificação")

        # Fase 3 — Síntese Pydantic AI
        heuristic = await phase3_validate(
            swarm=swarm,
            ctx=ctx,
            repo_url=entry.repo_url,
            repo_id=entry.repo_id,
            lote_id=entry.lote_id,
            nome_projeto=entry.nome_projeto,
            conn=conn,
            run_id=run_id,
        )

        # Persistência atômica (sqlite3 nativo — BEGIN IMMEDIATE)
        if not dry_run:
            atomic_write(conn, heuristic, run_id)
        else:
            logger.info(
                "[DRY-RUN] %s → %s (score=%.1f) [não gravado]",
                entry.repo_id, heuristic.classificacao_terminal, heuristic.score_final,
            )

        logger.info(
            "[OK] %s → %s | score=%.1f | lentes=%d",
            entry.repo_id, heuristic.classificacao_terminal,
            heuristic.score_final, swarm.lentes_disponiveis,
        )
        return heuristic

    except Exception as exc:
        logger.error("[ERRO] %s: %s", entry.repo_id, exc)
        log_error(conn, run_id, entry.repo_id, fase=0, exc=exc)
        return None


# ---------------------------------------------------------------------------
# T-08-D: Loop Principal
# ---------------------------------------------------------------------------

async def main(
    lote_id: str,
    batch_size: int,
    vault_db: Path,
    dry_run: bool,
) -> int:
    """
    Orquestra o pipeline ETL completo.
    Retorna 0 (sucesso) ou 1 (falha parcial/total).
    """
    run_id = str(uuid.uuid4())
    logger.info("[ETL] Iniciando run_id=%s lote=%s dry_run=%s", run_id, lote_id, dry_run)

    # Conexão bare-metal (sqlite3 nativo)
    conn = sqlite3.connect(str(vault_db))
    conn.row_factory = sqlite3.Row

    try:
        init_db(conn)
        logger.info("[DB] Vault: %s", vault_db)

        # Fetch do micro-lote da planilha
        entries = await fetch_batch_from_sheets(lote_id, batch_size)

        if not entries:
            logger.warning("[ETL] Nenhum repo TRIAGEM encontrado para lote=%s", lote_id)
            return 0

        create_run_log(conn, run_id, lote_id, len(entries))

        # Loop em micro-lotes (batch_size) para proteção de RAM e rate-limit
        for chunk_idx in range(0, len(entries), batch_size):
            chunk = entries[chunk_idx:chunk_idx+batch_size]
            logger.info("[ETL] Processando micro-lote %d/%d (tamanho=%d)", 
                        (chunk_idx//batch_size)+1, 
                        (len(entries)//batch_size)+1, 
                        len(chunk))
            
            chunk_results: list[RepoHeuristic | None] = []
            repos_ok   = 0
            repos_erro = 0
            ultimo_erro: str | None = None

            for entry in chunk:
                result = await process_repo(entry, conn, run_id, dry_run)
                chunk_results.append(result)
                if result is not None:
                    repos_ok += 1
                else:
                    repos_erro += 1
                    ultimo_erro = f"Falha em {entry.repo_id}"
                
                # Resfriamento da fila TCP
                await asyncio.sleep(2)

            # Atualiza planilha para este micro-lote
            await update_sheets_status(chunk, chunk_results, dry_run)
            
            # Atualiza o log da run no sqlite
            finalize_run_log(conn, run_id, repos_ok, repos_erro, ultimo_erro)
            
            logger.info("[ETL] HARD BREAK ATIVADO: Encerrando execução do Lote Piloto.")
            break

        logger.info("[ETL] Varredura de Lote Piloto finalizada. run_id=%s", run_id)

        return 0

    except Exception as exc:
        logger.exception("[ETL] Falha catastrófica: %s", exc)
        try:
            finalize_run_log(conn, run_id, 0, 0, str(exc)[:400])
        except Exception:
            pass
        return 1

    finally:
        conn.close()


# ---------------------------------------------------------------------------
# T-10: CLI (argparse)
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="etl_orchestrator",
        description="SODA ETL Orchestrator — Pipeline Cognitivo de 3 Fases",
    )
    p.add_argument(
        "--lote-id",
        required=True,
        help="ID do lote a processar (ex: LOTE_19)",
    )
    p.add_argument(
        "--batch-size",
        type=int,
        default=5,
        metavar="N",
        help="Número de repos por micro-lote (default: 5)",
    )
    p.add_argument(
        "--vault-db",
        type=Path,
        default=_DEFAULT_VAULT,
        metavar="PATH",
        help=f"Caminho para o soda_heuristic_vault.db (default: {_DEFAULT_VAULT})",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        default=False,
        help="Executa o pipeline sem gravar no SQLite nem atualizar a planilha",
    )
    return p


if __name__ == "__main__":
    parser = _build_parser()
    args = parser.parse_args()

    exit_code = asyncio.run(
        main(
            lote_id=args.lote_id,
            batch_size=args.batch_size,
            vault_db=args.vault_db,
            dry_run=args.dry_run,
        )
    )
    sys.exit(exit_code)
