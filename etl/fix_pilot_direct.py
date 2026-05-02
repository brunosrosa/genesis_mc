"""
fix_pilot_direct.py — Corrige o LOTE_PILOT_01 diretamente via Google Sheets API.
"""
import sqlite3
import sys
import os
from pathlib import Path
from google.oauth2 import service_account
from googleapiclient.discovery import build

sys.path.insert(0, str(Path(__file__).parent.parent))
from etl.phase4_sheets_loader import build_row_42, load_rows_for_lote, SHEET_COLS

try:
    from dotenv import load_dotenv
    load_dotenv(Path(__file__).parent.parent / ".env", override=True)
except ImportError:
    pass

VAULT_PATH = Path(__file__).parent.parent / "soda_heuristic_vault.db"
LOTE_ID    = "LOTE_PILOT_01"
SPREADSHEET_ID = os.getenv("GOOGLE_SHEETS_ID", "1jPmO29ZR240nq2YW4iwHvjKcd5X299z4Kv7AEI7oUbg")
CREDENTIALS_FILE = os.getenv("GOOGLE_APPLICATION_CREDENTIALS")

# Mapeamento
SHEET_ROW_MAP = {
    "serde-rs/serde":     371,
    "dtolnay/anyhow":     372,
    "BurntSushi/ripgrep": 373,
}

def col_letter(n: int) -> str:
    result = ""
    while n > 0:
        n, rem = divmod(n - 1, 26)
        result = chr(65 + rem) + result
    return result

def main():
    conn = sqlite3.connect(str(VAULT_PATH))
    conn.row_factory = sqlite3.Row
    rows_42 = load_rows_for_lote(conn, LOTE_ID)
    conn_rows = conn.execute(
        "SELECT repo_id FROM repo_heuristics WHERE lote_id=? ORDER BY score_total DESC",
        (LOTE_ID,),
    ).fetchall()
    
    col_end = col_letter(SHEET_COLS)
    update_data = []

    for row_42, meta in zip(rows_42, conn_rows):
        repo_id = meta["repo_id"]
        sheet_row = SHEET_ROW_MAP.get(repo_id)
        if sheet_row is None:
            continue
        range_str = f"MASTER_SOLUTIONS_v3!A{sheet_row}:{col_end}{sheet_row}"
        update_data.append({
            "range": range_str,
            "values": [row_42]
        })

    conn.close()

    if not CREDENTIALS_FILE or not os.path.exists(CREDENTIALS_FILE):
        print(f"[ERRO] Credenciais não encontradas em: {CREDENTIALS_FILE}")
        sys.exit(1)

    print("[FIX] Autenticando na API do Google Sheets...")
    creds = service_account.Credentials.from_service_account_file(
        CREDENTIALS_FILE, scopes=["https://www.googleapis.com/auth/spreadsheets"]
    )
    service = build('sheets', 'v4', credentials=creds)
    
    body = {
        "valueInputOption": "USER_ENTERED",
        "data": update_data
    }
    
    print("[FIX] Executando batchUpdate...")
    result = service.spreadsheets().values().batchUpdate(
        spreadsheetId=SPREADSHEET_ID, body=body
    ).execute()
    
    print(f"[FIX] SUCESSO: {result.get('totalUpdatedCells')} células atualizadas.")

if __name__ == "__main__":
    main()
