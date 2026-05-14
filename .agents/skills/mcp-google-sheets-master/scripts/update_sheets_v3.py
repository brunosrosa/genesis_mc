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

SPREADSHEET_ID = "1jPmO29ZR240nq2YW4iwHvjKcd5X299z4Kv7AEI7oUbg"
SERVICE_ACCOUNT_FILE = r"C:\Users\rosas\.keys\soda-sheets-service-account.json"

def update_ranges(updates_dict):
    try:
        credentials = service_account.Credentials.from_service_account_file(
            SERVICE_ACCOUNT_FILE, 
            scopes=['https://www.googleapis.com/auth/spreadsheets']
        )
        service = build('sheets', 'v4', credentials=credentials)

        data = []
        for range_name, values in updates_dict.items():
            data.append({
                'range': f"MASTER_SOLUTIONS_v3!{range_name}",
                'values': values
            })

        body = {
            'valueInputOption': 'USER_ENTERED',
            'data': data
        }

        result = service.spreadsheets().values().batchUpdate(
            spreadsheetId=SPREADSHEET_ID,
            body=body
        ).execute()

        print(f"[SUCESSO] {result.get('totalUpdatedCells', 0)} celulas atualizadas.")
        
    except Exception as e:
        print(f"[ERRO] {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    # O payload gerado anteriormente
    updates = {
        "D202:AM202": [[3.2, "ABSORB_PARTIALLY", "", "Segurança/Sandbox", "", "", "", "", "", "", "", "", "", "", "", 4.0, 2.0, 4.5, 3.0, 2.5, 3.2, "Pilha de referência para assistentes OpenClaw seguros.", "Acoplamento rígido ao Docker e runtime Node.js.", "Sandbox de privilégio mínimo via Landlock/eBPF.", "Lógicas de sandboxing Landlock e políticas de rede.", "Algoritmos de validação SSRF e isolamento de processos.", "High", "Low", "Orquestração de containers e gestão de certificados.", "Medium", "Low", "Node.js, Docker, K3s.", "Landlock, seccomp, netns.", "Runtime Node.js, Docker Desktop integrations.", "High", "Low"]],
        "D203:AM203": [[3.5, "ABSORB_CONCEPT", "", "Roteamento/LLM", "", "", "", "", "", "", "", "", "", "", "", 4.5, 2.5, 4.0, 3.5, 3.0, 3.5, "Ferramentas de inteligência e otimização para agentes.", "Camada pesada de abstração Python adicionando latência.", "Speculative branching e priorização de nós em grafos.", "Heurísticas de otimização de latência e controle de cache.", "Algoritmos de speculative execution para grafos de agentes.", "Medium", "Low", "Camada de instrumentação LangSmith e telemetria pesada.", "Medium", "Medium", "Python 3.11+, LangChain.", "Agent Performance Primitives (APP).", "Bindings LangChain/CrewAI pesados.", "Medium", "Low"]],
        "D204:AM204": [[3.2, "USE_AS_INSPIRATION_ONLY", "", "Interface/UI - Primitivas & Estética", "", "", "", "", "", "", "", "", "", "", "", 4.8, 1.5, 3.5, 4.0, 2.0, 3.2, "Cliente de e-mail AI-native focado em automação total.", "Monolito Electron impossível de fragmentar para o core SODA.", "Priority memory e aprendizado de estilo via few-shot.", "Lógica de triagem e rascunhos em background.", "Heurística de classificação de prioridade (high/medium/low).", "Low", "Low", "Renderização de e-mails via WebView e gestão de OAuth.", "High", "Low", "Electron, React, TypeScript.", "Priority memory system.", "Estrutura Electron e componentes React.", "Medium", "Low"]],
        "D205:AM205": [[3.8, "ABSORB_PARTIALLY", "", "Technical Infrastructure", "", "", "", "", "", "", "", "", "", "", "", 5.0, 3.0, 5.0, 4.5, 1.5, 3.8, "Interface CLI dinâmica para MCP/OpenAPI com economia de tokens.", "Dependência de interpretador Python para execução dinâmica.", "Dynamic CLI generation via API introspection.", "Formato TOON e lógica de ranking de ferramentas por uso.", "Abstração de esquemas OpenAPI/MCP para CLI-first.", "High", "Medium", "Gestão de tokens OAuth e persistência de cache local.", "High", "Low", "Python, uv.", "Formatos TOON, Bake mode.", "OAuth state management complexo.", "Low", "Low"]],
        "D206:AM206": [[3.7, "ABSORB_PARTIALLY", "", "Memória/RAG", "", "", "", "", "", "", "", "", "", "", "", 4.9, 3.0, 4.8, 4.0, 2.0, 3.7, "Implementação de RLMs para processar contextos infinitos.", "Exigência de REPL Python externo para segurança.", "Recursive context scaling via environment interaction.", "Algoritmo de decomposição de contexto e terminação.", "Recursive Language Models (RLM) protocol logic.", "High", "Medium", "Execução de código arbitrário sem sandbox isolado.", "High", "Medium", "Python, LLM REPL loop.", "RLM base class, Sandbox.", "Recursive depth sem controle de custo.", "Medium", "Medium"]]
    }
    update_ranges(updates)
