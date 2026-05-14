import sqlite3
import json

conn = sqlite3.connect('.soda_data/soda_heuristic_vault.db')
cur = conn.cursor()
cur.execute("SELECT name, sql FROM sqlite_master WHERE type='table' OR type='view'")
schema = {row[0]: row[1] for row in cur.fetchall()}
print(json.dumps(schema, indent=2))
