"""
fix_pilot.py — Corrige as 3 linhas corrompidas do LOTE_PILOT_01 no Sheets.

Lê do soda_heuristic_vault.db, constrói a matriz canônica de 42 colunas
via phase4_sheets_loader.build_row_42() e imprime o payload JSON para
o agente orquestrador injetar via MCP batch_update_cells.

Linhas alvo (inseridas anteriormente com schema errado):
  371 → serde-rs/serde
  372 → dtolnay/anyhow
  373 → BurntSushi/ripgrep

Uso:
  python etl/fix_pilot.py
"""
import json
import sqlite3
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from etl.phase4_sheets_loader import build_row_42, load_rows_for_lote, SHEET_COLS

VAULT_PATH = Path(__file__).parent.parent / "soda_heuristic_vault.db"
LOTE_ID    = "LOTE_PILOT_01"

# Mapeamento: posição na lista (order by score_total DESC) → linha na planilha
# serde(9.71)=371, anyhow(9.20)=372, ripgrep(8.49)=373
SHEET_ROW_MAP = {
    "serde-rs/serde":     371,
    "dtolnay/anyhow":     372,
    "BurntSushi/ripgrep": 373,
}

conn = sqlite3.connect(str(VAULT_PATH))
conn.row_factory = sqlite3.Row

# Valida o invariante 42 antes de qualquer coisa
rows_42 = load_rows_for_lote(conn, LOTE_ID)
conn_rows = conn.execute(
    "SELECT repo_id FROM repo_heuristics WHERE lote_id=? ORDER BY score_total DESC",
    (LOTE_ID,),
).fetchall()

print(f"\n[FIX] Validando invariante de 42 colunas para {len(rows_42)} registros...")
for i, (row, meta) in enumerate(zip(rows_42, conn_rows)):
    assert len(row) == SHEET_COLS, f"FALHA invariante em {meta['repo_id']}: {len(row)} cols"
    print(f"  [{i+1}] {meta['repo_id']}: {len(row)} colunas OK")

# Monta o payload para batch_update_cells
col_end = chr(ord('A') + SHEET_COLS - 1)  # 'A' + 41 = 'P'... na verdade AP = A+P
# Coluna AP = col 42. Letra dupla: A=1 ... Z=26, AA=27 ... AP=42
def col_letter(n: int) -> str:
    """Converte número 1-based para letra(s) de coluna do Sheets (A..Z, AA..AZ...)."""
    result = ""
    while n > 0:
        n, rem = divmod(n - 1, 26)
        result = chr(65 + rem) + result
    return result

col_start = "A"
col_end   = col_letter(SHEET_COLS)  # deve ser "AP"

print(f"\n[FIX] Intervalo de colunas: {col_start}..{col_end} ({SHEET_COLS} cols)")

ranges_payload = {}
for row_42, meta in zip(rows_42, conn_rows):
    repo_id  = meta["repo_id"]
    sheet_row = SHEET_ROW_MAP.get(repo_id)
    if sheet_row is None:
        print(f"  AVISO: {repo_id} não está no mapa de linhas — pulando")
        continue
    range_key = f"MASTER_SOLUTIONS!{col_start}{sheet_row}:{col_end}{sheet_row}"
    ranges_payload[range_key] = [row_42]
    print(f"  {range_key}  →  {repo_id}")

print("\n[FIX] === PAYLOAD batch_update_cells ===")
print(json.dumps(ranges_payload, ensure_ascii=False, indent=2, default=str))
print("[FIX] === FIM DO PAYLOAD ===\n")

# Verificação final
total_cols = sum(len(v[0]) for v in ranges_payload.values())
expected   = SHEET_COLS * len(ranges_payload)
print(f"[FIX] Total células: {total_cols} (esperado {expected}) — {'OK' if total_cols == expected else 'FALHA'}")

conn.close()
