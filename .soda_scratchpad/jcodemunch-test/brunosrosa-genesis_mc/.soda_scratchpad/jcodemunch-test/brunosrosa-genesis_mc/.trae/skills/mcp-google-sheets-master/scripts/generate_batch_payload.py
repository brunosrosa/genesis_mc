import json

projects = [
    {"row": 207, "name": "ysz / recursive-llm", "score": "8.5", "class": "INTEGRATE_AS_COMPONENT", "cat": "Roteamento/LLM", "scores": ["9", "7", "9", "8", "3", "8.5"]},
    {"row": 208, "name": "buildermethods / agent-os", "score": "9.0", "class": "INTEGRATE_AS_COMPONENT", "cat": "SDD/Spec", "scores": ["9", "9", "9", "9", "2", "9.0"]},
    {"row": 209, "name": "LarsCowe / bmalph", "score": "6.0", "class": "KEEP_UNDER_OBSERVATION", "cat": "Performance", "scores": ["6", "8", "5", "6", "5", "6.0"]},
    {"row": 210, "name": "Fission-AI / OpenSpec", "score": "9.5", "class": "INTEGRATE_AS_COMPONENT", "cat": "SDD/Spec", "scores": ["10", "9", "10", "9", "1", "9.5"]},
    {"row": 211, "name": "github / spec-kit", "score": "7.0", "class": "KEEP_UNDER_OBSERVATION", "cat": "SDD/Spec", "scores": ["7", "7", "7", "7", "4", "7.0"]},
    {"row": 212, "name": "gotalab / cc-sdd", "score": "9.8", "class": "INTEGRATE_AS_COMPONENT", "cat": "Orquestração/Loop", "scores": ["10", "10", "10", "9", "1", "9.8"]},
    {"row": 213, "name": "timescale / pg_textsearch", "score": "7.5", "class": "ABSORB_CONCEPT", "cat": "Memória/RAG", "scores": ["7", "6", "8", "8", "3", "7.5"]},
    {"row": 214, "name": "glassflow / clickhouse-etl", "score": "5.0", "class": "REJECT", "cat": "ETL Pesado", "scores": ["4", "2", "6", "5", "8", "5.0"]},
    {"row": 215, "name": "multigres / multigres-operator", "score": "4.0", "class": "REJECT", "cat": "Infra/K8s", "scores": ["3", "1", "5", "4", "9", "4.0"]},
    {"row": 216, "name": "DinobaseHQ / dinobase", "score": "6.5", "class": "KEEP_UNDER_OBSERVATION", "cat": "Memória/RAG", "scores": ["7", "5", "7", "6", "5", "6.5"]},
    {"row": 217, "name": "agberohq / keeper", "score": "6.0", "class": "KEEP_UNDER_OBSERVATION", "cat": "Segurança/Sandbox", "scores": ["6", "6", "6", "6", "6", "6.0"]},
    {"row": 218, "name": "rvitorper / go-bt", "score": "7.0", "class": "USE_AS_INSPIRATION_ONLY", "cat": "Orquestração/Loop", "scores": ["8", "6", "7", "7", "4", "7.0"]},
    {"row": 219, "name": "RyanCodrai / turbovec", "score": "9.2", "class": "INTEGRATE_AS_COMPONENT", "cat": "Memória/RAG", "scores": ["9", "10", "9", "9", "2", "9.2"]},
    {"row": 220, "name": "kitfunso / hippo-memory", "score": "8.8", "class": "ABSORB_PARTIALLY", "cat": "Memória/RAG", "scores": ["10", "7", "9", "8", "2", "8.8"]},
    {"row": 221, "name": "Luce-Org / luce-megakernel", "score": "6.5", "class": "KEEP_UNDER_OBSERVATION", "cat": "Technical Infrastructure", "scores": ["6", "7", "6", "6", "5", "6.5"]},
    {"row": 222, "name": "hauntsaninja / git_bayesect", "score": "7.2", "class": "USE_AS_INSPIRATION_ONLY", "cat": "Orquestração/Loop", "scores": ["8", "6", "8", "7", "3", "7.2"]},
    {"row": 223, "name": "afshinm / zerobox", "score": "8.0", "class": "INTEGRATE_AS_COMPONENT", "cat": "Segurança/Sandbox", "scores": ["8", "9", "8", "8", "3", "8.0"]},
    {"row": 224, "name": "yannick-cw / korb", "score": "5.0", "class": "KEEP_UNDER_OBSERVATION", "cat": "TBD", "scores": ["5", "5", "5", "5", "5", "5.0"]},
    {"row": 225, "name": "skrun-dev / skrun", "score": "6.0", "class": "KEEP_UNDER_OBSERVATION", "cat": "Orquestração/Loop", "scores": ["6", "6", "6", "6", "5", "6.0"]},
    {"row": 226, "name": "razvandimescu / numa", "score": "7.5", "class": "ABSORB_CONCEPT", "cat": "Technical Infrastructure", "scores": ["8", "8", "7", "7", "2", "7.5"]}
]

ranges = {}
for p in projects:
    # Col D (4), E (5), F (6), G (7) -> score, class, horizonte, cat
    # horizonte is index 5, we'll keep it as is or set default
    row_num = p['row']
    ranges[f"MASTER_SOLUTIONS_v3!D{row_num}:G{row_num}"] = [
        [p['score'], p['class'], "CURTO_PRAZO", p['cat']]
    ]
    
    # Col S (19) to AM (39) -> 21 columns
    # We'll fill the first 6 with scores, and the rest with placeholders/analysis
    s_to_am = p['scores'] + ["N/A"] * 15
    ranges[f"MASTER_SOLUTIONS_v3!S{row_num}:AM{row_num}"] = [s_to_am]

print(json.dumps(ranges))
