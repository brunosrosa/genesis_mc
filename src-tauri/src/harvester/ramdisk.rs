use std::path::{Path, PathBuf};
use std::time::Instant;
use sysinfo::System;
use thiserror::Error;
use tokio::time::sleep;
use tokio::time::Duration;
use tracing::{info, warn};
#[cfg(not(target_os = "windows"))]
use tempfile::{Builder, TempDir};
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_TEMPORARY};
#[cfg(target_os = "windows")]
use windows_projfs::ProjectedFileSystem;

#[cfg(target_os = "windows")]
use super::projfs::{mount_projected_repo, ProjectedRepoSnapshot};

const RAMDISK_READY_RETRIES: u32 = 20;
const RAMDISK_READY_DELAY_MS: u64 = 150;
#[cfg(not(target_os = "windows"))]
const CLEANUP_RETRIES: u32 = 20;
#[cfg(not(target_os = "windows"))]
const CLEANUP_DELAY_MS: u64 = 250;

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

#[cfg(target_os = "windows")]
struct MountedProjection {
    root_path: PathBuf,
    projection: ProjectedFileSystem,
}

struct RamdiskGuard {
    #[cfg(not(target_os = "windows"))]
    temp_dir: Option<TempDir>,
    #[cfg(target_os = "windows")]
    projection_handles: Vec<MountedProjection>,
    #[cfg(target_os = "windows")]
    workspace_root: PathBuf,
    #[cfg(target_os = "windows")]
    skip_drop_cleanup: bool,
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

    pub async fn cleanup(mut self) -> Result<(), RamdiskError> {
        let mount_path = self.mount_path.clone();
        #[cfg(target_os = "windows")]
        {
            let cleanup_started = Instant::now();
            let projected_roots = self._guard.release_projections();
            self._guard.skip_drop_cleanup = true;
            info!(
                path = %mount_path.display(),
                projected_roots = projected_roots.len(),
                "RamdiskHandle: iniciando teardown ProjFS"
            );

            for projection in projected_roots {
                let root = projection.root_path.clone();
                let _ = spawn_detached_delete_process(&root);
                drop(projection.projection);
                info!(
                    path = %root.display(),
                    elapsed_ms = cleanup_started.elapsed().as_millis(),
                    "RamdiskHandle: virtualization root delegada para delecao externa"
                );
            }

            let _ = spawn_detached_delete_process(&mount_path);
            self._guard.workspace_root = mount_path.clone();
            info!(
                path = %mount_path.display(),
                elapsed_ms = cleanup_started.elapsed().as_millis(),
                "RamdiskHandle: cleanup explicito concluido com delecao externa não-bloqueante"
            );
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Some(temp_dir) = self._guard.temp_dir.take() {
                let persisted_path = temp_dir.keep();
                info!(path = %mount_path.display(), "RamdiskHandle: iniciando cleanup explicito do workspace efemero");
                remove_dir_all_with_retries(persisted_path.clone()).await?;
                info!(path = %mount_path.display(), "RamdiskHandle: cleanup explicito concluido");
            }

            Ok(())
        }
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

    #[cfg(target_os = "windows")]
    pub fn mount_projected_repo(
        &mut self,
        repo_root: &Path,
        snapshot: ProjectedRepoSnapshot,
    ) -> Result<(usize, usize), RamdiskError> {
        let mounted = mount_projected_repo(repo_root, snapshot).map_err(|reason| {
            RamdiskError::AllocationFailed { reason }
        })?;
        let file_count = mounted.file_count;
        let total_bytes = mounted.total_bytes;
        self._guard.projection_handles.push(MountedProjection {
            root_path: repo_root.to_path_buf(),
            projection: mounted.projection,
        });
        Ok((file_count, total_bytes))
    }
}

impl AsRef<Path> for RamdiskHandle {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl RamdiskGuard {
    #[cfg(not(target_os = "windows"))]
    fn new(temp_dir: TempDir) -> Self {
        Self { temp_dir: Some(temp_dir) }
    }

    #[cfg(target_os = "windows")]
    fn new(workspace_root: PathBuf) -> Self {
        Self {
            projection_handles: Vec::new(),
            workspace_root,
            skip_drop_cleanup: false,
        }
    }

    #[cfg(target_os = "windows")]
    fn release_projections(&mut self) -> Vec<MountedProjection> {
        std::mem::take(&mut self.projection_handles)
    }
}

impl std::fmt::Debug for RamdiskGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(target_os = "windows")]
        {
            f.debug_struct("RamdiskGuard")
                .field("projection_handles_len", &self.projection_handles.len())
                .field("workspace_root", &self.workspace_root)
                .finish()
        }

        #[cfg(not(target_os = "windows"))]
        {
            f.debug_struct("RamdiskGuard")
                .field("temp_dir_present", &self.temp_dir.is_some())
                .finish()
        }
    }
}

impl Drop for RamdiskGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            if self.skip_drop_cleanup {
                info!("RamdiskGuard: cleanup ja delegado externamente; Drop nao repetira a remocao");
                return;
            }
            let dropped = self.projection_handles.len();
            let workspace_root = self.workspace_root.clone();
            let projected_roots = self.release_projections();
            info!(projections = dropped, "RamdiskGuard: projeções ProjFS descartadas via Drop");
            for projection in projected_roots {
                let root = projection.root_path;
                drop(projection.projection);
                match remove_dir_all_via_powershell(&root) {
                    Ok(()) => {
                        info!(path = %root.display(), "RamdiskGuard: virtualization root removida via Drop");
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => {
                        warn!(
                            path = %root.display(),
                            error = %e,
                            "RamdiskGuard: falha ao remover virtualization root via Drop"
                        );
                    }
                }
            }
            match remove_dir_all_via_powershell(&workspace_root) {
                Ok(()) => {
                    info!(path = %workspace_root.display(), "RamdiskGuard: workspace ProjFS descartado via Drop");
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    warn!(
                        path = %workspace_root.display(),
                        error = %e,
                        "RamdiskGuard: falha ao remover workspace ProjFS via Drop"
                    );
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        if let Some(temp_dir) = self.temp_dir.take() {
            let mount_path = temp_dir.path().to_path_buf();
            drop(temp_dir);
            info!(path = %mount_path.display(), "RamdiskGuard: workspace efemero descartado via Drop");
        }
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

#[cfg(target_os = "windows")]
fn remove_dir_all_via_powershell(path: &Path) -> std::io::Result<()> {
    let escaped = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$p='{escaped}'; if (Test-Path -LiteralPath $p) {{ Remove-Item -LiteralPath $p -Recurse -Force -ErrorAction Stop }}"
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "PowerShell Remove-Item falhou para '{}'",
            path.display()
        )))
    }
}

#[cfg(target_os = "windows")]
fn spawn_detached_delete_process(path: &Path) -> Result<(), RamdiskError> {
    let escaped = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$p='{escaped}'; \
         for ($i = 0; $i -lt 240; $i++) {{ \
            if (-not (Test-Path -LiteralPath $p)) {{ exit 0 }} \
            try {{ \
                Remove-Item -LiteralPath $p -Recurse -Force -ErrorAction Stop; \
                if (-not (Test-Path -LiteralPath $p)) {{ exit 0 }} \
            }} catch {{}} \
            Start-Sleep -Milliseconds 500; \
         }} \
         exit 0"
    );
    std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| RamdiskError::AllocationFailed {
            reason: format!(
                "Falha ao iniciar delecao externa para '{}': {}",
                path.display(),
                e
            ),
        })
}

#[cfg(not(target_os = "windows"))]
async fn remove_dir_all_with_retries(path: PathBuf) -> Result<(), RamdiskError> {
    tokio::task::spawn_blocking(move || {
        let mut last_error = None;
        for attempt in 1..=CLEANUP_RETRIES {
            match std::fs::remove_dir_all(&path) {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => {
                    #[cfg(target_os = "windows")]
                    let error = if e.raw_os_error() == Some(369) {
                        match remove_dir_all_via_powershell(&path) {
                            Ok(()) => return Ok(()),
                            Err(shell_err) => shell_err,
                        }
                    } else {
                        e
                    };
                    #[cfg(not(target_os = "windows"))]
                    let error = e;
                    last_error = Some(error);
                    std::thread::sleep(std::time::Duration::from_millis(CLEANUP_DELAY_MS));
                    warn!(
                        path = %path.display(),
                        attempt,
                        retries = CLEANUP_RETRIES,
                        "RamdiskHandle: cleanup ainda bloqueado; aguardando fechamento de handles"
                    );
                }
            }
        }

        let error = last_error.unwrap_or_else(|| {
            std::io::Error::other("cleanup falhou sem erro do sistema operacional")
        });
        Err(RamdiskError::AllocationFailed {
            reason: format!(
                "Falha ao remover workspace temporario '{}' durante cleanup explicito: {}",
                path.display(),
                error
            ),
        })
    })
    .await
    .map_err(|e| RamdiskError::AllocationFailed {
        reason: format!("Falha ao aguardar cleanup explicito do workspace: {}", e),
    })?
}

pub struct RamdiskAllocator;

impl RamdiskAllocator {
    #[cfg(target_os = "windows")]
    fn allocate_projfs_workspace() -> Result<RamdiskHandle, RamdiskError> {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| RamdiskError::AllocationFailed {
                reason: "Não foi possível resolver a raiz do workspace para o ProjFS".to_string(),
            })?;
        let base_root = project_root
            .join(".soda_scratchpad")
            .join("projfs_workspaces");

        std::fs::create_dir_all(&base_root).map_err(|e| RamdiskError::AllocationFailed {
            reason: format!("Falha ao preparar diretório raiz do ProjFS '{}': {}", base_root.display(), e),
        })?;

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mount_path = base_root.join(format!(
            "soda_projfs_workspace_{}_{}",
            std::process::id(),
            nonce
        ));

        std::fs::create_dir_all(&mount_path).map_err(|e| RamdiskError::AllocationFailed {
            reason: format!("Falha ao criar workspace base do ProjFS '{}': {}", mount_path.display(), e),
        })?;

        Ok(RamdiskHandle {
            _guard: RamdiskGuard::new(mount_path.clone()),
            mount_path,
        })
    }

    #[cfg(not(target_os = "windows"))]
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

        #[cfg(target_os = "windows")]
        let handle = Self::allocate_projfs_workspace()?;
        #[cfg(not(target_os = "windows"))]
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
