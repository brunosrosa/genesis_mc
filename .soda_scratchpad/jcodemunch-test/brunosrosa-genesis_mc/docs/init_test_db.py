"""
init_test_db.py — Script Efêmero de Validação de Infra
Protocolo BMAD: Fase de Mutação
Destruição obrigatória após Exit Code 0 via MCP.
"""
import sqlite3
import sys
import os

DB_PATH = os.path.join(os.path.dirname(__file__), "genesis.db")

def main() -> int:
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()

        cursor.execute("""
            CREATE TABLE IF NOT EXISTS teste_etl_memoria (
                id      INTEGER PRIMARY KEY,
                projeto TEXT NOT NULL,
                status  TEXT NOT NULL
            )
        """)

        # Idempotente: apaga canário anterior para evitar duplicata
        cursor.execute(
            "DELETE FROM teste_etl_memoria WHERE projeto = ?",
            ("Antigravity_SQLite_Test",)
        )

        cursor.execute(
            "INSERT INTO teste_etl_memoria (projeto, status) VALUES (?, ?)",
            ("Antigravity_SQLite_Test", "Concluído")
        )

        conn.commit()
        conn.close()

        print("[OK] Tabela 'teste_etl_memoria' criada e linha canária inserida.")
        print(f"[OK] Banco: {DB_PATH}")
        return 0

    except sqlite3.Error as e:
        print(f"[ERRO] SQLite: {e}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    sys.exit(main())
