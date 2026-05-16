import json

# Dados consolidados das análises
data_updates = {
    "202": { # NVIDIA / NemoClaw
        "score": 3.2,
        "classificacao": "ABSORB_PARTIALLY",
        "categoria": "Segurança/Sandbox",
        "scores": [4.0, 2.0, 4.5, 3.0, 2.5, 3.2], # S até X
        "details": [
            "Pilha de referência para assistentes OpenClaw seguros.",
            "Acoplamento rígido ao Docker e runtime Node.js.",
            "Sandbox de privilégio mínimo via Landlock/eBPF.",
            "Lógicas de sandboxing Landlock e políticas de rede.",
            "Algoritmos de validação SSRF e isolamento de processos.",
            "High", "Low", "Orquestração de containers e gestão de certificados.",
            "Medium", "Low", "Node.js, Docker, K3s.", "Landlock, seccomp, netns.",
            "Runtime Node.js, Docker Desktop integrations.", "High", "Low"
        ]
    },
    "203": { # NVIDIA / NeMo-Agent-Toolkit
        "score": 3.5,
        "classificacao": "ABSORB_CONCEPT",
        "categoria": "Roteamento/LLM",
        "scores": [4.5, 2.5, 4.0, 3.5, 3.0, 3.5],
        "details": [
            "Ferramentas de inteligência e otimização para agentes.",
            "Camada pesada de abstração Python adicionando latência.",
            "Speculative branching e priorização de nós em grafos.",
            "Heurísticas de otimização de latência e controle de cache.",
            "Algoritmos de speculative execution para grafos de agentes.",
            "Medium", "Low", "Camada de instrumentação LangSmith e telemetria pesada.",
            "Medium", "Medium", "Python 3.11+, LangChain.", "Agent Performance Primitives (APP).",
            "Bindings LangChain/CrewAI pesados.", "Medium", "Low"
        ]
    },
    "204": { # ankitvgupta / mail-app
        "score": 3.2,
        "classificacao": "USE_AS_INSPIRATION_ONLY",
        "categoria": "Interface/UI - Primitivas & Estética",
        "scores": [4.8, 1.5, 3.5, 4.0, 2.0, 3.2],
        "details": [
            "Cliente de e-mail AI-native focado em automação total.",
            "Monolito Electron impossível de fragmentar para o core SODA.",
            "Priority memory e aprendizado de estilo via few-shot.",
            "Lógica de triagem e rascunhos em background.",
            "Heurística de classificação de prioridade (high/medium/low).",
            "Low", "Low", "Renderização de e-mails via WebView e gestão de OAuth.",
            "High", "Low", "Electron, React, TypeScript.", "Priority memory system.",
            "Estrutura Electron e componentes React.", "Medium", "Low"
        ]
    },
    "205": { # knowsuchagency / mcp2cli
        "score": 3.8,
        "classificacao": "ABSORB_PARTIALLY",
        "categoria": "Technical Infrastructure",
        "scores": [5.0, 3.0, 5.0, 4.5, 1.5, 3.8],
        "details": [
            "Interface CLI dinâmica para MCP/OpenAPI com economia de tokens.",
            "Dependência de interpretador Python para execução dinâmica.",
            "Dynamic CLI generation via API introspection.",
            "Formato TOON e lógica de ranking de ferramentas por uso.",
            "Abstração de esquemas OpenAPI/MCP para CLI-first.",
            "High", "Medium", "Gestão de tokens OAuth e persistência de cache local.",
            "High", "Low", "Python, uv.", "Formatos TOON, Bake mode.",
            "OAuth state management complexo.", "Low", "Low"
        ]
    },
    "206": { # fullstackwebdev / rlm_repl
        "score": 3.7,
        "classificacao": "ABSORB_PARTIALLY",
        "categoria": "Memória/RAG",
        "scores": [4.9, 3.0, 4.8, 4.0, 2.0, 3.7],
        "details": [
            "Implementação de RLMs para processar contextos infinitos.",
            "Exigência de REPL Python externo para segurança.",
            "Recursive context scaling via environment interaction.",
            "Algoritmo de decomposição de contexto e terminação.",
            "Recursive Language Models (RLM) protocol logic.",
            "High", "Medium", "Execução de código arbitrário sem sandbox isolado.",
            "High", "Medium", "Python, LLM REPL loop.", "RLM base class, Sandbox.",
            "Recursive depth sem controle de custo.", "Medium", "Medium"
        ]
    }
}

payload = {}

for row, data in data_updates.items():
    # Coluna D (score), E (classif), G (categoria)
    # Mas o range é D:AM.
    # D: 3, E: 4, F: 5, G: 6, ...
    # S: 18, ... AM: 38
    
    # Vamos criar uma linha completa de D até AM (36 colunas)
    # D (idx 3 na planilha, mas no range D:AM o D é 0)
    # E (idx 4 na planilha, no range é 1)
    # F (idx 5, no range é 2) -> horizonte_extracao (não mudar, ou manter vazio?)
    # G (idx 6, no range é 3)
    
    # S (idx 18, no range é 15)
    
    row_data = [""] * 36 # D (0) até AM (35)
    row_data[0] = data["score"]
    row_data[1] = data["classificacao"]
    row_data[3] = data["categoria"] # G é o 4º elemento (D, E, F, G)
    
    # Scores (S-X) começam no índice 15 (S é o 19º elemento da planilha, index 18)
    # Range D:AM -> D is 0. S is index 15.
    for i, s in enumerate(data["scores"]):
        row_data[15 + i] = s
        
    # Details (Y-AM) começam no índice 21 (Y é o 25º elemento, index 24)
    for i, d in enumerate(data["details"]):
        row_data[21 + i] = d
        
    payload[f"D{row}:AM{row}"] = [row_data]

print(json.dumps(payload))
