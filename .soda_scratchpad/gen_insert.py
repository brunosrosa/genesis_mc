import csv
with open('docs/architecture/DATABASE_SCHEMA_DIC.csv', 'r', encoding='utf-8') as f:
    reader = csv.DictReader(f)
    cols = [row['Nome da Coluna'].strip() for row in reader if row['Tabela de Destino (SQLite)'] == 'repo_heuristics']
    
    print('INSERT OR REPLACE INTO repo_heuristics (')
    print('    ' + ', '.join(cols))
    print(') VALUES (')
    print('    ' + ', '.join(f'?{i+1}' for i in range(len(cols))))
    print(')')
    
    for i, c in enumerate(cols):
        val = f'payload.{c}'
        if c == 'executive_verdict' or c == 'acao_de_canibalizacao':
            val = f'format!(\\"{{:?}}\\", payload.{c})'
        print(f'                {val},')
