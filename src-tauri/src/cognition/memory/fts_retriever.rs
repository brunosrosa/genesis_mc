use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Estrutura para representar uma correspondência léxica retornada pelo FTS5.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LexicalMatch {
    pub observation_id: String,
    pub content: String,
    pub file_path: String,
    pub raw_score: f64,
}

/// Recuperador léxico síncrono que consulta o índice virtual FTS5 `observations_fts` no SQLite.
#[derive(Debug, Clone)]
pub struct FtsRetriever {
    pub db_path: PathBuf,
}

impl FtsRetriever {
    /// Cria uma nova instância do `FtsRetriever` apontando para o arquivo de banco SQLite.
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    /// Executa a busca léxica síncrona usando o score BM25 nativo do SQLite FTS5.
    pub fn search_lexical(&self, query: &str, limit: usize) -> Result<Vec<LexicalMatch>, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Erro ao abrir banco SQLite em '{:?}': {}", self.db_path, e))?;
        Self::search_lexical_with_conn(&conn, query, limit)
    }

    /// Executa a busca léxica reutilizando uma conexão SQLite existente.
    pub fn search_lexical_with_conn(
        conn: &Connection,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LexicalMatch>, String> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Sanitização básica da query para evitar erros de sintaxe FTS5
        let sanitized_query = query
            .replace(['"', '\'', '*'], "")
            .trim()
            .to_string();

        if sanitized_query.is_empty() {
            return Ok(Vec::new());
        }

        let fts_query = format!("\"{}\"*", sanitized_query);

        // Tenta fazer o JOIN com a tabela `observations` para obter metadata estendido.
        let sql_join = "
            SELECT 
                COALESCE(NULLIF(o.observation_id, ''), CAST(observations_fts.rowid AS TEXT)) AS obs_id,
                observations_fts.content,
                COALESCE(o.file_path, '') AS fpath,
                bm25(observations_fts) AS score
            FROM observations_fts
            LEFT JOIN observations o ON observations_fts.rowid = o.id
            WHERE observations_fts MATCH ?1
            ORDER BY score ASC
            LIMIT ?2
        ";

        let mut matches = Vec::new();

        if let Ok(mut stmt) = conn.prepare(sql_join) {
            let rows = stmt.query_map([&fts_query, &limit.to_string()], |row| {
                Ok(LexicalMatch {
                    observation_id: row.get(0)?,
                    content: row.get(1)?,
                    file_path: row.get(2)?,
                    raw_score: row.get(3)?,
                })
            });

            if let Ok(rows) = rows {
                for r in rows.flatten() {
                    matches.push(r);
                }
                return Ok(matches);
            }
        }

        // Fallback: consulta direta na tabela virtual `observations_fts` caso o JOIN falhe
        let sql_direct = "
            SELECT 
                CAST(rowid AS TEXT) AS obs_id,
                content,
                '' AS fpath,
                bm25(observations_fts) AS score
            FROM observations_fts
            WHERE observations_fts MATCH ?1
            ORDER BY score ASC
            LIMIT ?2
        ";

        let mut stmt = conn
            .prepare(sql_direct)
            .map_err(|e| format!("Erro ao preparar SQL FTS5: {}", e))?;

        let rows = stmt
            .query_map([&fts_query, &limit.to_string()], |row| {
                Ok(LexicalMatch {
                    observation_id: row.get(0)?,
                    content: row.get(1)?,
                    file_path: row.get(2)?,
                    raw_score: row.get(3)?,
                })
            })
            .map_err(|e| format!("Erro ao executar consulta FTS5: {}", e))?;

        for r in rows {
            let m = r.map_err(|e| format!("Erro ao ler linha FTS5: {}", e))?;
            matches.push(m);
        }

        Ok(matches)
    }
}
