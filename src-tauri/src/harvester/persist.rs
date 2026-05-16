use rusqlite::{params, Connection};
use thiserror::Error;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactBlob {
    pub artifact_type: String,
    pub payload_blob: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum HarvesterError {
    #[error("Storage error: {0}")]
    StorageError(String),
}

pub struct BlobNormalizer;

impl BlobNormalizer {
    pub async fn persist(
        repo_id: String,
        blobs: Vec<ArtifactBlob>,
        conn: Arc<Mutex<Connection>>,
    ) -> Result<(), HarvesterError> {
        // PT-3: I/O do SQLite é síncrono, delegamos para spawn_blocking
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().map_err(|e| HarvesterError::StorageError(e.to_string()))?;
            
            // Início da Transação Atômica
            let tx = conn.transaction().map_err(|e| HarvesterError::StorageError(e.to_string()))?;

            // Estrangula a redundância (Idempotência)
            tx.execute("DELETE FROM artefatos_brutos WHERE repo_id = ?1", params![&repo_id])
                .map_err(|e| HarvesterError::StorageError(e.to_string()))?;

            for blob in blobs {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                    
                // PT-BLOB-1: Inserção individual de artefatos
                tx.execute(
                    "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
                    params![repo_id, blob.artifact_type, blob.payload_blob, now],
                ).map_err(|e| HarvesterError::StorageError(e.to_string()))?;
            }

            // Commit da Transação
            tx.commit().map_err(|e| HarvesterError::StorageError(e.to_string()))
        })
        .await
        .map_err(|e| HarvesterError::StorageError(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE artefatos_brutos (
                id INTEGER PRIMARY KEY,
                repo_id TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                payload_blob BLOB NOT NULL
            )",
            [],
        ).unwrap();
        conn
    }

    #[tokio::test]
    async fn test_persist_multiple_blobs_success() {
        let conn = Arc::new(Mutex::new(setup_db()));
        let blobs = vec![
            ArtifactBlob { artifact_type: "AST".to_string(), payload_blob: b"ast_data".to_vec() },
            ArtifactBlob { artifact_type: "Manifest".to_string(), payload_blob: b"manifest_data".to_vec() },
        ];

        let result = BlobNormalizer::persist("repo_1".to_string(), blobs, conn.clone()).await;
        assert!(result.is_ok());

        // Verificar persistência
        let conn_locked = conn.lock().unwrap();
        let mut stmt = conn_locked.prepare("SELECT count(*) FROM artefatos_brutos").unwrap();
        let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_persist_rollback_on_failure() {
        let conn = Arc::new(Mutex::new(setup_db()));
        
        // Forçar erro via constraint de UNIQUE
        {
            let conn_locked = conn.lock().unwrap();
            conn_locked.execute("CREATE UNIQUE INDEX idx_unique_type ON artefatos_brutos(artifact_type)", []).unwrap();
        }

        let blobs = vec![
            ArtifactBlob { artifact_type: "DUP".to_string(), payload_blob: b"first".to_vec() },
            ArtifactBlob { artifact_type: "DUP".to_string(), payload_blob: b"second".to_vec() },
        ];

        let result = BlobNormalizer::persist("repo_fail".to_string(), blobs, conn.clone()).await;
        assert!(result.is_err());

        // PT-BLOB-1 & Transação: Provar que o ROLLBACK funcionou (banco deve estar vazio)
        let conn_locked = conn.lock().unwrap();
        let mut stmt = conn_locked.prepare("SELECT count(*) FROM artefatos_brutos").unwrap();
        let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
    }
}
