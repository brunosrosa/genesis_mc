use std::path::Path;
use sysinfo::System;
use tempfile::{Builder, TempDir};
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tracing::warn;

const RAMDISK_READY_RETRIES: u32 = 20;
const RAMDISK_READY_DELAY_MS: u64 = 150;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RamdiskError {
    #[error("Insufficient memory. Available: {available_mb} MB, requested: {requested_mb} MB (with 2048 MB safety margin)")]
    InsufficientMemory {
        available_mb: u64,
        requested_mb: u32,
    },

    #[error("Allocation failed: {reason}")]
    AllocationFailed {
        reason: String,
    },

}

#[derive(Debug)]
pub struct RamdiskHandle {
    temp_dir: TempDir,
}

impl RamdiskHandle {
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

impl AsRef<Path> for RamdiskHandle {
    fn as_ref(&self) -> &Path {
        self.temp_dir.path()
    }
}

pub struct RamdiskAllocator;

async fn wait_until_writable(path: &Path) -> Result<(), RamdiskError> {
    let probe_file = path.join(format!(
        ".soda_ramdisk_probe_{}_tmp",
        std::process::id()
    ));
    let mut last_error = String::from("ramdisk ainda nao respondeu ao health check");

    for _ in 0..RAMDISK_READY_RETRIES {
        if tokio::fs::metadata(path).await.is_ok() {
            match tokio::fs::write(&probe_file, b"SODA_READY").await {
                Ok(()) => {
                    if let Err(e) = tokio::fs::remove_file(&probe_file).await {
                        warn!(path = %probe_file.display(), error = %e, "Falha ao remover probe file do health check do ramdisk");
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }
        }

        sleep(Duration::from_millis(RAMDISK_READY_DELAY_MS)).await;
    }

    Err(RamdiskError::AllocationFailed {
        reason: format!(
            "Ramdisk montado, mas nao ficou gravavel apos {} tentativas: {}",
            RAMDISK_READY_RETRIES, last_error
        ),
    })
}

impl RamdiskAllocator {
    pub async fn allocate(tamanho_mb: u32) -> Result<RamdiskHandle, RamdiskError> {
        // 1. Memory Check
        let mut sys = System::new();
        sys.refresh_memory();
        let available_bytes = sys.available_memory();
        let available_mb = available_bytes / 1024 / 1024;

        let margem_seguranca_mb = 2048;
        let required_mb = tamanho_mb as u64 + margem_seguranca_mb;

        if available_mb < required_mb {
            return Err(RamdiskError::InsufficientMemory {
                available_mb,
                requested_mb: tamanho_mb,
            });
        }

        let temp_dir = Builder::new()
            .prefix("soda_shadow_workspace_")
            .tempdir()
            .map_err(|e| RamdiskError::AllocationFailed {
                reason: format!("Falha ao criar workspace temporario via tempfile: {}", e),
            })?;

        wait_until_writable(temp_dir.path()).await?;

        Ok(RamdiskHandle { temp_dir })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alloc_success() {
        let handle = RamdiskAllocator::allocate(64).await.unwrap();
        let path = handle.path();
        assert!(path.exists(), "Ponto de montagem deve existir");

        let test_file = path.join("test.txt");
        std::fs::write(&test_file, "SODA").unwrap();
        assert_eq!(std::fs::read_to_string(&test_file).unwrap(), "SODA");
    }

    #[tokio::test]
    async fn test_alloc_insufficient_memory() {
        // Solicita 1 Petabyte (1_000_000_000 MB) para forçar InsufficientMemory
        let err = RamdiskAllocator::allocate(1_000_000_000).await.unwrap_err();
        assert!(
            matches!(err, RamdiskError::InsufficientMemory { .. }),
            "Deveria falhar por memória insuficiente"
        );
    }

    #[tokio::test]
    async fn test_drop_cleans_temp_workspace() {
        let path = {
            let handle = RamdiskAllocator::allocate(64).await.unwrap();
            let p = handle.path().to_path_buf();
            assert!(p.exists());
            p
        };
        assert!(!path.exists(), "Workspace temporario deveria ser removido pelo Drop do TempDir");
    }

    #[tokio::test]
    async fn test_wait_until_writable_accepts_real_writes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = wait_until_writable(temp_dir.path()).await;
        assert!(result.is_ok(), "Health check deve aprovar diretório gravavel");
    }

    #[tokio::test]
    async fn test_allocator_creates_workspace_inside_system_temp() {
        let handle = RamdiskAllocator::allocate(64).await.unwrap();
        assert!(
            handle.path().starts_with(std::env::temp_dir()),
            "Workspace deve nascer dentro do diretório temporario do host"
        );
    }
}
