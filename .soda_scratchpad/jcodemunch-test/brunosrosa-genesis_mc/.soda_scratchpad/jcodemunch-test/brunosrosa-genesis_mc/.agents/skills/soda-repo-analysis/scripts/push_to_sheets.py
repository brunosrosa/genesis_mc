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
import google.auth
from google.oauth2 import service_account
from googleapiclient.discovery import build

PAYLOAD_PATH = ".agents/tmp/etl_payload.json"
SPREADSHEET_ID = "1jPmO29ZR240nq2YW4iwHvjKcd5X299z4Kv7AEI7oUbg"
RANGE_NAME = "MASTER_SOLUTIONS_v3!A:R"

def main():
    if not os.path.exists(PAYLOAD_PATH):
        print(f"[ERRO] Arquivo de payload nao encontrado no caminho: {PAYLOAD_PATH}", file=sys.stderr)
        sys.exit(1)

    try:
        with open(PAYLOAD_PATH, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except Exception as e:
        print(f"[ERRO] Falha ao ler ou aplicar parser no JSON local: {e}", file=sys.stderr)
        sys.exit(1)

    if not isinstance(data, list) or len(data) == 0:
        print("[ERRO] Estrutura Invalida: O payload deve ser uma lista (Array de Arrays) contendo o cabecalho.", file=sys.stderr)
        sys.exit(1)

    print(f"[*] Preparando injecao de {len(data)} linhas no Google Sheets usando Service Account...")

    try:
        # Resolve Service Account Explicitamente
        SERVICE_ACCOUNT_FILE = r"C:\Users\rosas\.keys\soda-sheets-service-account.json"
        credentials = service_account.Credentials.from_service_account_file(
            SERVICE_ACCOUNT_FILE, 
            scopes=['https://www.googleapis.com/auth/spreadsheets']
        )
        service = build('sheets', 'v4', credentials=credentials)

        body = {
            'values': data
        }

        # Transacao atômica de insercao
        result = service.spreadsheets().values().append(
            spreadsheetId=SPREADSHEET_ID,
            range=RANGE_NAME,
            valueInputOption="USER_ENTERED",
            insertDataOption="INSERT_ROWS",
            body=body
        ).execute()

        updates = result.get('updates', {})
        print(f"[SUCESSO] {updates.get('updatedCells', 0)} celulas atualizadas na planilha.")
        
        # Expurgacao toxica do payload em caso de sucesso (Garbage Collection)
        os.remove(PAYLOAD_PATH)
        print(f"[*] Garbage Collection: Payload temporario deletado em {PAYLOAD_PATH}")
        
    except Exception as e:
        print(f"[ERRO FATAL] Falha na API do Google Sheets: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
