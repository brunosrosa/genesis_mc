import sqlite3
import sys

db_path = "soda_heuristic_vault.db"
try:
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
    tables = cursor.fetchall()
    print(f"Tables in {db_path}: {[t[0] for t in tables]}")
    
    # Check for progress or logs
    for table in [t[0] for t in tables]:
        if 'log' in table.lower() or 'progress' in table.lower() or 'heuristic' in table.lower():
            cursor.execute(f"SELECT count(*) FROM {table}")
            count = cursor.fetchone()[0]
            print(f"Table {table} has {count} rows.")
            # Sample V3 data
            if count > 0:
                cursor.execute(f"PRAGMA table_info({table})")
                cols = [c[1] for c in cursor.fetchall()]
                print(f"Columns in {table}: {cols}")
                cursor.execute(f"SELECT * FROM {table} LIMIT 2")
                print(f"Sample data from {table}: {cursor.fetchall()}")
    
    conn.close()
except Exception as e:
    print(f"Error: {e}")
