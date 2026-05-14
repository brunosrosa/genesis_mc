import csv

rust_types = {
    'INTEGER': 'i64',
    'REAL': 'f64',
    'TEXT': 'String'
}

with open('docs/architecture/DATABASE_SCHEMA_DIC.csv', 'r', encoding='utf-8') as f:
    reader = csv.DictReader(f)
    cols = []
    types = []
    reqs = []
    for row in reader:
        if row['Tabela de Destino (SQLite)'] == 'repo_heuristics':
            col = row['Nome da Coluna'].strip()
            t = rust_types[row['Tipo SQL (SQLite)'].strip()]
            req = row['Obrigatoriedade'].strip()
            cols.append(col)
            types.append(t)
            reqs.append(req)

with open('gen_code.rs', 'w', encoding='utf-8') as out:
    out.write("#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]\n")
    out.write("pub struct SgrPayload {\n")
    for col, t, req in zip(cols, types, reqs):
        if col == 'executive_verdict': t = 'TerminalClassification'
        elif col == 'acao_de_canibalizacao': t = 'CannibalizationAction'
        elif col == 'classificacao_terminal': t = 'String'
        
        if req != 'NOT NULL': t = f'Option<{t}>'
        out.write(f"    #[serde(default)]\n")
        out.write(f"    pub {col}: {t},\n")
    out.write("}\n\n")

    out.write("INSERT OR REPLACE INTO repo_heuristics (\n")
    out.write("    " + ", ".join(cols) + "\n")
    out.write(") VALUES (\n")
    out.write("    " + ", ".join(f"?{i+1}" for i in range(len(cols))) + "\n")
    out.write(")\n\n")

    out.write("rusqlite::params![\n")
    for col in cols:
        val = f"payload.{col}"
        if col == 'executive_verdict' or col == 'acao_de_canibalizacao':
            val = f"format!(\\\"{{:?}}\\\", payload.{col})"
        out.write(f"    {val},\n")
    out.write("]\n\n")
    
    out.write("let batch_payload = vec![\n")
    out.write("    json!(vec![\n")
    for col in cols:
        val = f"payload.{col}"
        if col == 'executive_verdict' or col == 'acao_de_canibalizacao':
            val = f"format!(\\\"{{:?}}\\\", payload.{col})"
        out.write(f"        json!({val}),\n")
    out.write("    ])\n")
    out.write("];\n")
