"""
phase4_sheets_loader.py — Fase 4: Load para Google Sheets.

Mapeia um RepoHeuristic (SQLite) para a matriz canônica de EXATAMENTE 42
colunas da aba MASTER_SOLUTIONS. Cada posição é declarada explicitamente.
Colunas sem dados recebem string vazia "".

Schema de 42 colunas (ordem estrita):
 01 project_name              09 score_operability         17 classificacao_terminal     25 executive_verdict          33 transplantable_core
 02 declared_description      10 score_creep_risk          18 stack_base                 26 ouro_a_extrair             34 logic_math_heuristic
 03 repo_url                  11 entropy_risk              19 tipo_integracao            27 deep_pattern               35 discipline_dependency
 04 score_final               12 design_misuse_risk        20 must_components            28 acao_de_canibalizacao       36 extractability_level
 05 score_fit_geral_soda      13 intrinsic_ethics_risk     21 proposta_original_resumo   29 risco_principal             37 operability_level
 06 score_philosophical_fit   14 horizonte_extracao        22 lente_a_sentido_ux         30 observacoes                38 where_ai_should_not_enter
 07 score_bare_metal_fit      15 justificativa_decisao     23 lente_b_estrutura_arq      31 real_structural_problem    39 do_not_absorb
 08 score_architectural_extractability  16 categoria_arquitetural  24 lente_c_realidade_ops  32 bare_metal_fit             40 data_ultima_analise
                                                                                                                          41 analise_origem
                                                                                                                          42 lote_id
"""
from __future__ import annotations

import sqlite3
from typing import Any


# ---------------------------------------------------------------------------
# Constantes
# ---------------------------------------------------------------------------
SHEET_COLS = 45         # Lei dura: nunca altere sem aprovação HITL
SPREADSHEET_ID = ""     # Injetado em tempo de execução pelo chamador


def _v(val: Any, max_len: int = 0) -> str:
    """Converte qualquer valor para string segura para injeção no Sheets."""
    if val is None or val == "" or val == 0.0 and isinstance(val, float) and max_len == 0:
        return ""
    if isinstance(val, (int, float)):
        return val
    s = str(val).strip()
    if max_len > 0:
        s = s[:max_len]
    return s


def build_row_45(heuristic: Any) -> list[Any]:
    """
    Constrói o array de EXATAMENTE 45 elementos a partir do Pydantic RepoHeuristic ou dict.
    Cada índice corresponde à coluna canônica da MASTER_SOLUTIONS_v3.
    """
    r = heuristic if isinstance(heuristic, dict) else heuristic.model_dump()
    
    row = [
        # 01 project_name
        _v(r.get("project_name")),
        # 02 declared_description
        _v(r.get("declared_description"), max_len=300),
        # 03 repo_url
        _v(r.get("repo_url")),
        # 04 score_final
        r.get("score_final", 0.0),
        # 05 score_fit_geral_soda
        r.get("score_fit_geral_soda", 0.0),
        # 06 score_philosophical_fit
        r.get("score_philosophical_fit", 0.0),
        # 07 score_bare_metal_fit
        r.get("score_bare_metal_fit", 0.0),
        # 08 score_architectural_extractability
        r.get("score_architectural_extractability", 0.0),
        # 09 score_operability
        r.get("score_operability", 0.0),
        # 10 score_creep_risk
        r.get("score_creep_risk", 0.0),
        # 11 entropy_risk
        _v(r.get("entropy_risk")),
        # 12 design_misuse_risk
        _v(r.get("design_misuse_risk")),
        # 13 intrinsic_ethics_risk
        _v(r.get("intrinsic_ethics_risk")),
        # 14 horizonte_extracao
        _v(r.get("horizonte_extracao")),
        # 15 justificativa_decisao
        _v(r.get("justificativa_decisao"), max_len=400),
        # 16 categoria_arquitetural
        _v(r.get("categoria_arquitetural")),
        # 17 categoria_nuance_tecnica
        _v(r.get("categoria_nuance_tecnica")),
        # 18 classificacao_terminal
        _v(r.get("classificacao_terminal")),
        # 19 stack_base
        _v(r.get("stack_base")),
        # 20 tipo_integracao
        _v(r.get("tipo_integracao")),
        # 21 integracao_papel_exato
        _v(r.get("integracao_papel_exato")),
        # 22 must_components
        _v(r.get("must_components")),
        # 23 proposta_original_resumo
        _v(r.get("proposta_original_resumo")),
        # 24 lente_a_sentido_ux
        _v(r.get("lente_a_sentido_ux"), max_len=600),
        # 25 lente_b_estrutura_arq
        _v(r.get("lente_b_estrutura_arq"), max_len=600),
        # 26 lente_c_realidade_ops
        _v(r.get("lente_c_realidade_ops"), max_len=600),
        # 27 executive_verdict
        _v(r.get("executive_verdict"), max_len=400),
        # 28 ouro_a_extrair
        _v(r.get("ouro_a_extrair")),
        # 29 deep_pattern
        _v(r.get("deep_pattern")),
        # 30 acao_de_canibalizacao
        _v(r.get("acao_de_canibalizacao")),
        # 31 transplantable_core
        _v(r.get("transplantable_core")),
        # 32 logic_math_heuristic
        _v(r.get("logic_math_heuristic")),
        # 33 risco_principal
        _v(r.get("risco_principal")),
        # 34 risco_linha_vermelha
        _v(r.get("risco_linha_vermelha")),
        # 35 observacoes
        _v(r.get("observacoes")),
        # 36 real_structural_problem
        _v(r.get("real_structural_problem")),
        # 37 bare_metal_fit
        _v(r.get("bare_metal_fit")),
        # 38 discipline_dependency
        _v(r.get("discipline_dependency")),
        # 39 extractability_level
        _v(r.get("extractability_level")),
        # 40 operability_level
        _v(r.get("operability_level")),
        # 41 where_ai_should_not_enter
        _v(r.get("where_ai_should_not_enter")),
        # 42 do_not_absorb
        _v(r.get("do_not_absorb")),
        # 43 data_ultima_analise
        _v(r.get("data_ultima_analise")),
        # 44 analise_origem
        _v(r.get("analise_origem")),
        # 45 lote_id
        _v(r.get("lote_id")),
    ]

    # Fail-Fast: blindagem do schema V3
    assert len(row) == SHEET_COLS, f"Array Misalignment detectado: {len(row)} colunas (esperado {SHEET_COLS})."
    return row


# ---------------------------------------------------------------------------
# Lógica de UPSERT Destrutivo
# ---------------------------------------------------------------------------

async def upsert_batch_to_sheets(
    spreadsheet_id: str,
    sheet_tab: str,
    heuristics: list[Any],
) -> None:
    """
    Executa UPSERT destrutivo (Update in-place) para uma lista de repositórios.
    A Fase 4 NÃO usa 'Append'. Ela lê a coluna C (repo_url) e sobrescreve a linha.
    """
    import logging
    import google.auth
    from google.oauth2 import service_account
    from googleapiclient.discovery import build

    logger = logging.getLogger(__name__)

    if not heuristics:
        return

    SERVICE_ACCOUNT_FILE = r"C:\Users\rosas\.keys\soda-sheets-service-account.json"
    credentials = service_account.Credentials.from_service_account_file(
        SERVICE_ACCOUNT_FILE, 
        scopes=['https://www.googleapis.com/auth/spreadsheets']
    )
    service = build('sheets', 'v4', credentials=credentials)
    
    # 1. Lê a coluna C (repo_url) para encontrar as linhas (limitado a 1000 para performance)
    read_range = f"{sheet_tab}!C:C"
    result = service.spreadsheets().values().get(
        spreadsheetId=spreadsheet_id,
        range=read_range
    ).execute()
        
    data = result.get("values", [])
    urls_in_sheet = [r[0].strip() if r else "" for r in data]

    # Mapeamento: url -> numero da linha (1-based)
    url_to_row = {url: i+1 for i, url in enumerate(urls_in_sheet) if url}

    value_ranges = []
    
    for h in heuristics:
        r = h if isinstance(h, dict) else h.model_dump()
        repo_url = r.get("repo_url", "").strip()
        
        row_data = build_row_45(r)
        
        if repo_url in url_to_row:
            row_idx = url_to_row[repo_url]
            range_to_update = f"{sheet_tab}!A{row_idx}:AS{row_idx}"
            value_ranges.append({
                "range": range_to_update,
                "values": [row_data]
            })
        else:
            logger.warning(f"[FASE-4] UPSERT pulou repo: {repo_url} não encontrado na Coluna C.")

    if not value_ranges:
        return

    # Executa o batchUpdate com as linhas sobrescritas
    payload = {
        "valueInputOption": "USER_ENTERED",
        "data": value_ranges,
    }

    resp = service.spreadsheets().values().batchUpdate(
        spreadsheetId=spreadsheet_id,
        body=payload
    ).execute()

    logger.info(f"[FASE-4] UPSERT destrutivo concluído para {len(value_ranges)} repositórios.")

