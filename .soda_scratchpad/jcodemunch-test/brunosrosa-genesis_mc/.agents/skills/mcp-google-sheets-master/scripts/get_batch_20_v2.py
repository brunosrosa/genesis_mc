import json

def generate_payload():
    batch = [
        # 1. recursive-llm (Row 207)
        {
            "row": 207,
            "score": "7.1",
            "classificacao": "KEEP_UNDER_OBSERVATION",
            "categoria": "Roteamento/LLM",
            "scores_granulares": ["8.5", "5.0", "9.0", "7.0", "6.0", "7.1"],
            "canon_v3": [
                "Implementação Python de RLM (Recursive Language Models) para contextos gigantes.",
                "Gestão de contexto unbounded via REPL e recursão de peeking/searching.",
                "Recursive Language Model (RLM) pattern com particionamento adaptativo.",
                "Lógica de recursão e particionamento de contexto via LLM.",
                "Heurística de exploração recursiva e particionamento dinâmico.",
                "ALTO (Lógica portável para Rust)",
                "BAIXO (Dependência de interpretador Python)",
                "Onde a IA se perde na profundidade da recursão sem circuit breaker.",
                "MEDIO (Exige REPL isolado)",
                "MEDIO (REPL restrito ajuda, mas recursão infinita é risco)",
                "Fobia de Runtimes (Python/LiteLLM)",
                "REPL Executor, Prompt Builder, Parser",
                "LiteLLM (viola dogma bare-metal), RestrictedPython",
                "Vazamento de contexto ou execução de código malicioso no REPL",
                "BAIXO"
            ]
        },
        # 2. agent-os (Row 208)
        {
            "row": 208,
            "score": "9.3",
            "classificacao": "INTEGRATE_AS_COMPONENT",
            "categoria": "Orquestração/Loop",
            "scores_granulares": ["9.5", "9.0", "9.5", "9.0", "3.0", "9.3"],
            "canon_v3": [
                "Sistema leve para alinhamento de agentes via padrões e specs dinâmicas.",
                "Desalinhamento de agentes e exaustão de contexto por falta de padrões.",
                "Standards Injection (Pattern-Matching) e Discovery autônomo.",
                "Framework de injeção de padrões baseado em triggers de contexto.",
                "Heurística de descoberta e deploy de padrões arquiteturais.",
                "EXTREMO (Agnóstico e focado em specs)",
                "EXTREMO (Lógica pura de orquestração)",
                "Decisões arquiteturais fundamentais que exigem soberania humana.",
                "EXTREMO (Baseado em arquivos markdown/docs)",
                "BAIXO (Zero-trust por design)",
                "Metodologia SODA (Standards-Driven)",
                "Standards Discoverer, Deployer, Spec Shaper",
                "N/A (Agnóstico)",
                "Injeção de padrões obsoletos ou conflitantes",
                "EXTREMO"
            ]
        },
        # 3. bmalph (Row 209)
        {
            "row": 209,
            "score": "6.8",
            "classificacao": "ABSORB_CONCEPT",
            "categoria": "Orquestração/Loop",
            "scores_granulares": ["9.0", "4.0", "8.5", "7.5", "5.0", "6.8"],
            "canon_v3": [
                "Bundle de automação de desenvolvimento (BMAD + Ralph) para loops TDD.",
                "Lentidão no ciclo Planejamento -> Implementação autônoma.",
                "Autonomous TDD Loop com transição de artefatos de planejamento.",
                "Máquina de estados para loop de implementação com circuit breakers.",
                "Heurística de circuit breaker para evitar Ralph Loops (stagnation).",
                "ALTO (Lógica de loop é portável)",
                "MEDIO (Dependência de Bash e Node.js)",
                "Refatorações estruturais massivas sem supervisão (Blast Radius).",
                "ALTO (CLI driven)",
                "MEDIO (Circuit breakers ajudam)",
                "Bash/Node.js persistent runtimes",
                "BMAD Agents, Ralph Loop, Platform Drivers",
                "Node.js, Bash, jq",
                "Commits automáticos quebrados ou loops infinitos de custo",
                "ALTO"
            ]
        },
        # 4. OpenSpec (Row 210)
        {
            "row": 210,
            "score": "9.4",
            "classificacao": "INTEGRATE_AS_COMPONENT",
            "categoria": "Orquestração/Loop",
            "scores_granulares": ["9.5", "9.5", "9.0", "9.5", "2.0", "9.4"],
            "canon_v3": [
                "Workflow guiado por artefatos para alinhamento humano-IA em camadas.",
                "Vibe coding e perda de rastreabilidade de requisitos em chats longos.",
                "Artifact-Guided Workflow (Proposal -> Spec -> Design -> Tasks).",
                "Divulgação progressiva de contexto via subpastas e artefatos atômicos.",
                "Heurística de validação de readiness antes da implementação.",
                "EXTREMO (Puro design de processo)",
                "EXTREMO (Zero runtime dependência)",
                "Mudanças de 'Constitution' e regras globais do projeto.",
                "EXTREMO (Simples e baseado em Markdown)",
                "BAIXO (Fricção cognitiva estruturada)",
                "Spec-Driven Development (SDD) Dogma",
                "Opsx Proposer, Task Generator, Archiver",
                "Node.js (CLI), mas o core é Markdown",
                "Fragmentação de specs em projetos massivos",
                "EXTREMO"
            ]
        },
        # 5. spec-kit (Row 211)
        {
            "row": 211,
            "score": "8.8",
            "classificacao": "INTEGRATE_AS_COMPONENT",
            "categoria": "Orquestração/Loop",
            "scores_granulares": ["9.5", "8.0", "8.5", "8.5", "4.0", "8.8"],
            "canon_v3": [
                "Toolkit para tornar especificações executáveis (Spec-Driven Development).",
                "Tradução falha entre requisitos de negócio e tarefas técnicas.",
                "Executable Specification Pattern e Constitution-based governance.",
                "Transformação de grafos de requisitos em sequências de tarefas TDD.",
                "Heurística de governança baseada em constituição (principals).",
                "ALTO (Conceitos SDD fundamentais)",
                "ALTO (Focado em CLI e automação)",
                "Definição da Constituição e regras de governança core.",
                "ALTO (CLI robusto)",
                "MEDIO (Focado em qualidade via TDD)",
                "Constitution-Driven Governance",
                "Specify CLI, Task Engine, Constitution Guard",
                "Python (specify-cli), uv/pipx",
                "Over-engineering de specs simples",
                "ALTO"
            ]
        }
    ]

    ranges = {}
    spreadsheet_id = "1jPmO29ZR240nq2YW4iwHvjKcd5X299z4Kv7AEI7oUbg"
    sheet = "MASTER_SOLUTIONS_v3"

    for p in batch:
        row = p["row"]
        # Coluna D (Score Geral)
        ranges[f"D{row}"] = [[p["score"]]]
        # Coluna E (Classificação)
        ranges[f"E{row}"] = [[p["classificacao"]]]
        # Coluna G (Categoria)
        ranges[f"G{row}"] = [[p["categoria"]]]
        # Colunas S-X (Scores Granulares)
        ranges[f"S{row}:X{row}"] = [p["scores_granulares"]]
        # Colunas Y-AM (Canônicos V3)
        ranges[f"Y{row}:AM{row}"] = [p["canon_v3"]]

    return {
        "spreadsheet_id": spreadsheet_id,
        "sheet": sheet,
        "ranges": ranges
    }

if __name__ == "__main__":
    payload = generate_payload()
    print(json.dumps(payload, indent=2))
