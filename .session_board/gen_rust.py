import csv

rust_types = {
    'INTEGER': 'i64',
    'REAL': 'f64',
    'TEXT': 'String'
}

print("#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]")
print("pub struct SgrPayload {")

with open('docs/architecture/DATABASE_SCHEMA_DIC.csv', 'r', encoding='utf-8') as f:
    reader = csv.DictReader(f)
    for row in reader:
        if row['Tabela de Destino (SQLite)'] == 'repo_heuristics':
            col = row['Nome da Coluna'].strip()
            t = rust_types[row['Tipo SQL (SQLite)'].strip()]
            req = row['Obrigatoriedade'].strip()
            
            # Use specific types for some columns
            if col == 'executive_verdict':
                t = 'TerminalClassification'
            elif col == 'acao_de_canibalizacao':
                t = 'CannibalizationAction'
            
            if req != 'NOT NULL':
                t = f"Option<{t}>"
            
            print(f"    #[serde(default)]")
            print(f"    pub {col}: {t},")
print("}")
