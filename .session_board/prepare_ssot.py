# coding=utf-8

with open('gen_code.rs', 'r', encoding='utf-8') as f:
    text = f.read()

sql_insert = text.split('INSERT OR REPLACE INTO repo_heuristics (')[1].split(' rusqlite::params!')[0].strip()
sql_insert = 'INSERT OR REPLACE INTO repo_heuristics (\n' + sql_insert

rusqlite_params = text.split('rusqlite::params!')[1].split('let batch_payload')[0].strip()
rusqlite_params = 'rusqlite::params!' + rusqlite_params

batch_payload = text.split('let batch_payload = ')[1].strip().rstrip(';')

replacement1 = f'''        conn.execute(
            "{sql_insert}",
            {rusqlite_params},
        ).map_err(|e| format!("Falha ao executar INSERT repo_heuristics: {{}}", e))?;'''

with open('rep1.txt', 'w', encoding='utf-8') as out:
    out.write(replacement1)

replacement2 = f'''    fn prepare_batch_payload(_repo_id: &str, payload: SgrPayload) -> Value {{
        let batch_payload = {batch_payload};
        // Formato correto esperado pelo MCP batch_update_cells (dict)
        json!({{
            "A2:CD2": batch_payload
        }})
    }}'''

with open('rep2.txt', 'w', encoding='utf-8') as out:
    out.write(replacement2)
