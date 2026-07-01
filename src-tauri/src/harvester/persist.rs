use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{params, Connection};
use thiserror::Error;

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
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().map_err(|e| HarvesterError::StorageError(e.to_string()))?;

            conn.busy_timeout(Duration::from_millis(5000))
                .map_err(|e| HarvesterError::StorageError(e.to_string()))?;

            let tx = conn.transaction().map_err(|e| HarvesterError::StorageError(e.to_string()))?;

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| HarvesterError::StorageError(e.to_string()))?
                .as_secs() as i64;

            for blob in blobs {
                tx.execute(
                    "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(repo_id, artifact_type) DO UPDATE SET
                        payload_blob = excluded.payload_blob,
                        timestamp_extracao = excluded.timestamp_extracao",
                    params![repo_id, blob.artifact_type, blob.payload_blob, now],
                ).map_err(|e| HarvesterError::StorageError(e.to_string()))?;
            }

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
                payload_blob BLOB NOT NULL,
                timestamp_extracao INTEGER NOT NULL,
                UNIQUE(repo_id, artifact_type)
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
    async fn test_persist_upserts_existing_blob() {
        let conn = Arc::new(Mutex::new(setup_db()));
        let first_pass = vec![
            ArtifactBlob { artifact_type: "DUP".to_string(), payload_blob: b"first".to_vec() },
        ];
        BlobNormalizer::persist("repo_fail".to_string(), first_pass, conn.clone()).await.unwrap();

        let second_pass = vec![
            ArtifactBlob { artifact_type: "DUP".to_string(), payload_blob: b"second".to_vec() },
        ];
        let result = BlobNormalizer::persist("repo_fail".to_string(), second_pass, conn.clone()).await;
        assert!(result.is_ok());

        let conn_locked = conn.lock().unwrap();
        let count: i64 = conn_locked
            .query_row("SELECT count(*) FROM artefatos_brutos", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let payload: Vec<u8> = conn_locked
            .query_row(
                "SELECT payload_blob FROM artefatos_brutos WHERE repo_id = ?1 AND artifact_type = ?2",
                ["repo_fail", "DUP"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(payload, b"second".to_vec());
    }

    #[tokio::test]
    async fn test_persist_preserves_unrelated_blob_types_for_same_repo() {
        let conn = Arc::new(Mutex::new(setup_db()));
        let first_pass = vec![
            ArtifactBlob { artifact_type: "blob_03_test_intent".to_string(), payload_blob: b"tests".to_vec() },
            ArtifactBlob { artifact_type: "blob_03_domain_mechanics".to_string(), payload_blob: b"legacy-old".to_vec() },
        ];
        BlobNormalizer::persist("repo_goose".to_string(), first_pass, conn.clone()).await.unwrap();

        let second_pass = vec![
            ArtifactBlob { artifact_type: "blob_03_test_intent".to_string(), payload_blob: b"tests-new".to_vec() },
            ArtifactBlob { artifact_type: "blob_11_ux_contracts".to_string(), payload_blob: b"ux-new".to_vec() },
        ];
        BlobNormalizer::persist("repo_goose".to_string(), second_pass, conn.clone()).await.unwrap();

        let conn_locked = conn.lock().unwrap();
        let count: i64 = conn_locked
            .query_row("SELECT count(*) FROM artefatos_brutos WHERE repo_id = ?1", ["repo_goose"], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);

        let preserved_count: i64 = conn_locked
            .query_row(
                "SELECT count(*) FROM artefatos_brutos WHERE repo_id = ?1 AND artifact_type = ?2",
                ["repo_goose", "blob_03_domain_mechanics"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved_count, 1);
    }
}
