import json

def get_next_batch(file_path, lote_id="Lote_19_Novos", status="TRIAGEM", limit=5):
    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    rows = data['valueRanges'][0]['values']
    header = rows[0]
    
    batch = []
    # Começa da linha 2 (índice 1) para pular o cabeçalho
    for i, row in enumerate(rows[1:], start=2):
        if len(row) > 4:
            row_lote = row[2] if len(row) > 2 else ""
            row_status = row[4] if len(row) > 4 else ""
            
            if row_lote == lote_id and row_status == status:
                batch.append({
                    "row_index": i,
                    "project_name": row[0],
                    "repo_url": row[1]
                })
                if len(batch) >= limit:
                    break
    return batch

if __name__ == "__main__":
    file_path = r"C:\Users\rosas\.gemini\antigravity\brain\35d6519c-3121-48f5-b9a2-eaa86600338b\.system_generated\steps\5\output.txt"
    batch = get_next_batch(file_path)
    print(json.dumps(batch, indent=2))
