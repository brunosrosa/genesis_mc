//! DDL canônico do `souls_graph` (V2).
//!
//! Idempotente: `CREATE TABLE/VIRTUAL TABLE/TRIGGER/INDEX IF NOT EXISTS`.
//! Pode ser executado em cold start de qualquer banco V1 ou V2 sem efeito
//! colateral. A transação atômica é responsabilidade do caller (`ops.rs`).

/// DDL puro do schema V2 (observations + FTS5 + triggers + índices).
/// Não abre conexão — só retorna o SQL para o caller executar dentro
/// de uma transação.
pub const V2_SCHEMA_DDL: &str = "
CREATE TABLE IF NOT EXISTS observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_name TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(entity_name) REFERENCES entities(name) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_observations_entity ON observations(entity_name);
CREATE INDEX IF NOT EXISTS idx_observations_created ON observations(created_at);

CREATE VIRTUAL TABLE IF NOT EXISTS observations_fts USING fts5(
    entity_name,
    content,
    content='observations',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS observations_ai AFTER INSERT ON observations BEGIN
    INSERT INTO observations_fts(rowid, entity_name, content)
    VALUES (new.id, new.entity_name, new.content);
END;

CREATE TRIGGER IF NOT EXISTS observations_ad AFTER DELETE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, rowid, entity_name, content)
    VALUES('delete', old.id, old.entity_name, old.content);
END;

CREATE TRIGGER IF NOT EXISTS observations_au AFTER UPDATE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, rowid, entity_name, content)
    VALUES('delete', old.id, old.entity_name, old.content);
    INSERT INTO observations_fts(rowid, entity_name, content)
    VALUES (new.id, new.entity_name, new.content);
END;
";

/// Lê `PRAGMA user_version` (V1 = estado atual; V2 = pós-migração).
/// Usa a forma universal `query_row` (compatível com qualquer versão de rusqlite).
pub fn read_user_version(conn: &rusqlite::Connection) -> rusqlite::Result<i64> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
}

/// Crava `PRAGMA user_version` no valor informado.
pub fn write_user_version(conn: &rusqlite::Connection, version: i64) -> rusqlite::Result<()> {
    conn.execute(&format!("PRAGMA user_version = {version}"), [])
        .map(|_| ())
}
