# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "google-auth",
#     "google-api-python-client",
# ]
# ///

import json
import os
import sys
import re
from google.oauth2 import service_account
from googleapiclient.discovery import build

SPREADSHEET_ID = "1jPmO29ZR240nq2YW4iwHvjKcd5X299z4Kv7AEI7oUbg"
TAB_NAME = "MASTER_SOLUTIONS_v3"
SERVICE_ACCOUNT_FILE = r"C:\Users\rosas\.keys\soda-sheets-service-account.json"

def normalize_score(val):
    if not val:
        return ""
    val = str(val).strip()
    if not val:
        return ""
    
    # Remove sufixos como "/10"
    val = val.split('/')[0].strip()
    
    # Tenta extrair apenas o numero (pode ser int ou float)
    match = re.search(r"(\d+(\.\d+)?)", val)
    if match:
        try:
            num = float(match.group(1))
            return f"{num:.1f}"
        except ValueError:
            return ""
    return ""

def main():
    try:
        credentials = service_account.Credentials.from_service_account_file(
            SERVICE_ACCOUNT_FILE, 
            scopes=['https://www.googleapis.com/auth/spreadsheets']
        )
        service = build('sheets', 'v4', credentials=credentials)

        # 1. Ler Coluna D (score) e Linha 1
        print("[*] Lendo dados da planilha...")
        ranges = [f"{TAB_NAME}!D2:D1000", f"{TAB_NAME}!S1:X1"]
        result = service.spreadsheets().values().batchGet(
            spreadsheetId=SPREADSHEET_ID,
            ranges=ranges
        ).execute()

        value_ranges = result.get('valueRanges', [])
        col_d_values = value_ranges[0].get('values', [])
        
        # 2. Preparar atualizacao da Coluna D
        normalized_d = []
        for row in col_d_values:
            val = row[0] if row else ""
            normalized_d.append([normalize_score(val)])
        
        # 3. Preparar novos cabecalhos
        headers = [
            "score_philosophical_fit",
            "score_bare_metal_fit",
            "score_architectural_extractability",
            "score_operability",
            "score_creep_risk",
            "score_fit_geral_soda"
        ]

        # 4. Construir Batch Update
        data_to_update = [
            {
                'range': f"{TAB_NAME}!D2:D{1 + len(normalized_d)}",
                'values': normalized_d
            },
            {
                'range': f"{TAB_NAME}!S1:X1",
                'values': [headers]
            }
        ]

        body = {
            'valueInputOption': 'USER_ENTERED',
            'data': data_to_update
        }

        print("[*] Enviando atualizacao em lote (Normalizacao + Expansao)...")
        service.spreadsheets().values().batchUpdate(
            spreadsheetId=SPREADSHEET_ID,
            body=body
        ).execute()

        print("[SUCESSO] Normalizacao numerica e expansao de schema concluidas.")
        sys.exit(0)

    except Exception as e:
        print(f"[ERRO FATAL] {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
