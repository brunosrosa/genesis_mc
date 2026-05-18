use std::path::{Path, PathBuf};
use sysinfo::System;
use tempfile::{Builder, TempDir};
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tracing::warn;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_TEMPORARY};

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
struct RamdiskGuard {
    temp_dir: Option<TempDir>,
}

#[derive(Debug)]
pub struct RamdiskHandle {
    _guard: RamdiskGuard,
    mount_path: PathBuf,
}

impl RamdiskHandle {
    pub fn path(&self) -> &Path {
        &self.mount_path
    }

    #[cfg(target_os = "windows")]
    pub async fn prime_temp_cache(&self, root: &Path) -> Result<(), RamdiskError> {
        let root = root.to_path_buf();
        let root_for_mark = root.clone();
        tokio::task::spawn_blocking(move || mark_tree_temporary(&root_for_mark))
            .await
            .map_err(|e| RamdiskError::AllocationFailed {
                reason: format!("Falha ao aguardar marcacao temporaria da arvore: {}", e),
            })?
    }
}

impl AsRef<Path> for RamdiskHandle {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl RamdiskGuard {
    fn new(temp_dir: TempDir) -> Self {
        Self { temp_dir: Some(temp_dir) }
    }
}

impl Drop for RamdiskGuard {
    fn drop(&mut self) {
        let _ = self.temp_dir.take();
    }
}

#[cfg(target_os = "windows")]
fn to_wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
fn mark_path_temporary(path: &Path) -> Result<(), RamdiskError> {
    let metadata = std::fs::metadata(path).map_err(|e| RamdiskError::AllocationFailed {
        reason: format!("Falha ao ler metadata de '{}': {}", path.display(), e),
    })?;
    if metadata.is_dir() {
        return Ok(());
    }
    let desired_attributes = metadata.file_attributes() | FILE_ATTRIBUTE_TEMPORARY;
    let wide_path = to_wide_path(path);
    let ok = unsafe { SetFileAttributesW(wide_path.as_ptr(), desired_attributes) };
    if ok == 0 {
        return Err(RamdiskError::AllocationFailed {
            reason: format!(
                "SetFileAttributesW falhou ao marcar '{}' como temporario",
                path.display()
            ),
        });
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn mark_tree_temporary(root: &Path) -> Result<(), RamdiskError> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        mark_path_temporary(&path)?;
        if path.is_dir() {
            let entries = std::fs::read_dir(&path).map_err(|e| RamdiskError::AllocationFailed {
                reason: format!("Falha ao percorrer '{}': {}", path.display(), e),
            })?;
            for entry in entries {
                let entry = entry.map_err(|e| RamdiskError::AllocationFailed {
                    reason: format!("Falha ao ler entrada em '{}': {}", path.display(), e),
                })?;
                stack.push(entry.path());
            }
        }
    }
    Ok(())
}

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

pub struct RamdiskAllocator;

impl RamdiskAllocator {
    fn allocate_temp_workspace() -> Result<RamdiskHandle, RamdiskError> {
        let temp_dir = Builder::new()
            .prefix("soda_shadow_workspace_")
            .tempdir()
            .map_err(|e| RamdiskError::AllocationFailed {
                reason: format!("Falha ao criar workspace temporario via tempfile: {}", e),
            })?;
        let mount_path = temp_dir.path().to_path_buf();
        let handle = RamdiskHandle {
            _guard: RamdiskGuard::new(temp_dir),
            mount_path,
        };
        Ok(handle)
    }

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

        let handle = Self::allocate_temp_workspace()?;

        wait_until_writable(handle.path()).await?;

        Ok(handle)
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
    async fn test_allocator_creates_writable_workspace() {
        let handle = RamdiskAllocator::allocate(64).await.unwrap();
        let marker = handle.path().join("allocator_probe.txt");
        std::fs::write(&marker, "ok").unwrap();
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ok");
    }
}
