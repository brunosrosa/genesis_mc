import csv
import sqlite3
import re

def forge_db():
    conn = sqlite3.connect('.soda_data/soda_heuristic_vault.db')
    cursor = conn.cursor()

    # Drop existing tables
    cursor.execute("SELECT name, type FROM sqlite_master WHERE type='table' OR type='view'")
    for row in cursor.fetchall():
        name = row[0]
        obj_type = row[1]
        if name != 'sqlite_sequence':
            cursor.execute(f"DROP {obj_type.upper()} IF EXISTS {name}")
            print(f"Dropped {obj_type} {name}")
    
    tables = {}
    with open('docs/architecture/DATABASE_SCHEMA_DIC.csv', 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        for row in reader:
            t_name = row.get('Tabela de Destino (SQLite)', '').strip()
            if not t_name or t_name.startswith('VIEW:'):
                continue
            
            c_name = row['Nome da Coluna'].strip()
            c_type = row['Tipo SQL (SQLite)'].strip()
            c_req = row['Obrigatoriedade'].strip()
            c_key = row['Chave / Relacionamento'].strip()
            
            if t_name not in tables:
                tables[t_name] = []
            
            col_def = f"{c_name} {c_type}"
            if c_req == 'NOT NULL':
                col_def += " NOT NULL"
            
            if c_key and c_key != '-':
                if 'PRIMARY KEY' in c_key:
                    if '(AUTOINCREMENT)' in c_key:
                        col_def = col_def.replace(' NOT NULL', '')
                        col_def += " PRIMARY KEY AUTOINCREMENT"
                    else:
                        col_def += " PRIMARY KEY"
                if 'UNIQUE' in c_key:
                    col_def += " UNIQUE"
                if 'FOREIGN KEY' in c_key:
                    match = re.search(r'FOREIGN KEY \((.*?)\)', c_key)
                    if match:
                        fk_target = match.group(1)
                        if '.' in fk_target:
                            ref_table, ref_col = fk_target.split('.')
                            col_def += f" REFERENCES {ref_table}({ref_col})"
                        else:
                            if fk_target == 'repositorios':
                                col_def += f" REFERENCES repositorios(project_name)"
                            else:
                                col_def += f" REFERENCES {fk_target}"
            
            tables[t_name].append(col_def)

    for t_name, columns in tables.items():
        cols_str = ",\n    ".join(columns)
        create_sql = f"CREATE TABLE IF NOT EXISTS {t_name} (\n    {cols_str}\n);"
        print(f"Creating table {t_name}...")
        cursor.execute(create_sql)
        
    view_matrix = """
    CREATE VIEW IF NOT EXISTS action_matrix AS
    SELECT project_name, acao_de_canibalizacao, transplantable_core, score_architectural_priority, score_absorption_readiness
    FROM repo_heuristics
    WHERE classificacao_terminal IN ('STACK_CORE_PLANO_A', 'INTEGRATE_AS_COMPONENT')
    """
    
    view_quarantine = """
    CREATE VIEW IF NOT EXISTS quarantine_radar AS
    SELECT project_name, design_misuse_risk, entropy_risk, intrinsic_ethics_risk, risco_principal
    FROM repo_heuristics
    WHERE design_misuse_risk IN ('HIGH', 'CRITICAL')
       OR entropy_risk IN ('HIGH', 'CRITICAL')
       OR intrinsic_ethics_risk IN ('HIGH', 'CRITICAL')
    """
    
    view_topology = """
    CREATE VIEW IF NOT EXISTS soda_graph_topology AS
    SELECT project_name, stack_base, architectural_topology, capability_nature_primary
    FROM repo_heuristics
    """
    
    cursor.execute(view_matrix)
    print("Created VIEW action_matrix")
    cursor.execute(view_quarantine)
    print("Created VIEW quarantine_radar")
    cursor.execute(view_topology)
    print("Created VIEW soda_graph_topology")

    conn.commit()
    conn.close()
    print("Database forged successfully.")

if __name__ == '__main__':
    forge_db()
