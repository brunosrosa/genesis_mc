import json

def find_triagem_lote19(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    values = data['valueRanges'][0]['values']
    header = values[0]
    
    # Índices das colunas
    col_project = header.index("project_name ") if "project_name " in header else 0
    col_url = header.index("repo_url ") if "repo_url " in header else 1
    col_lote = header.index("lote_id ") if "lote_id " in header else 2
    col_classif = header.index("classificacao_terminal") if "classificacao_terminal" in header else 4
    
    candidates = []
    # values[0] é o cabeçalho (Linha 1 da planilha)
    # values[1] é o dado 1 (Linha 2 da planilha)
    
    for i, row in enumerate(values):
        if i == 0: continue # Skip header
        
        # Garantir que a linha tenha colunas suficientes
        if len(row) > max(col_lote, col_classif):
            lote = row[col_lote].strip()
            classif = row[col_classif].strip()
            
            if lote == "Lote_19_Novos" and classif == "TRIAGEM":
                candidates.append({
                    "row_index": i + 1, # 1-indexed (Linha da planilha)
                    "project_name": row[col_project].strip(),
                    "repo_url": row[col_url].strip()
                })
                
                if len(candidates) >= 5:
                    break
    
    return candidates

if __name__ == "__main__":
    path = r"C:\Users\rosas\.gemini\antigravity\brain\b0f6b3d6-da45-4bd1-bca0-223e08c25707\.system_generated\steps\10\output.txt"
    results = find_triagem_lote19(path)
    print(json.dumps(results, indent=2))
