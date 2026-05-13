use std::path::{Path, PathBuf};
use thiserror::Error;
use sysinfo::System;
use tracing::warn;

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

    #[error("Platform not supported")]
    UnsupportedPlatform,
}

#[derive(Debug)]
pub struct RamdiskHandle {
    path: PathBuf,
    is_mock: bool,
    #[cfg(target_os = "windows")]
    drive_letter: Option<char>,
}

impl RamdiskHandle {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for RamdiskHandle {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for RamdiskHandle {
    fn drop(&mut self) {
        if self.is_mock {
            if self.path.exists() {
                if let Err(e) = std::fs::remove_dir_all(&self.path) {
                    warn!(path = %self.path.display(), error = %e, "Falha ao limpar diretório mock do Ramdisk");
                }
            }
            return;
        }

        // RAII INCONDICIONAL: A thread de desmontagem é JOIN'ed.
        // O Drop BLOQUEIA até que o Ramdisk seja desmontado.
        // Isto é INTENCIONAL — o Drop roda em destruição de escopo,
        // não dentro do Event Loop do Tokio. Garantia do PRD-001 §6.1:
        // "Não existe cenário onde o Ramdisk sobrevive ao fim do escopo."
        #[cfg(target_os = "windows")]
        {
            if let Some(letter) = self.drive_letter {
                let handle = std::thread::spawn(move || {
                    let status = std::process::Command::new("imdisk.exe")
                        .arg("-D")
                        .arg("-m")
                        .arg(format!("{}:", letter))
                        .status();
                    if let Err(e) = status {
                        eprintln!("[SODA] CRITICAL: Falha ao desmontar Ramdisk {}:\\ — {}", letter, e);
                    }
                });
                if let Err(e) = handle.join() {
                    eprintln!("[SODA] CRITICAL: Thread de desmontagem do Ramdisk entrou em panic: {:?}", e);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let path = self.path.clone();
            let handle = std::thread::spawn(move || {
                let status = std::process::Command::new("umount")
                    .arg(&path)
                    .status();
                if let Err(e) = status {
                    eprintln!("[SODA] CRITICAL: Falha ao desmontar Ramdisk {} — {}", path.display(), e);
                }
            });
            if let Err(e) = handle.join() {
                eprintln!("[SODA] CRITICAL: Thread de desmontagem do Ramdisk entrou em panic: {:?}", e);
            }
        }
    }
}

pub struct RamdiskAllocator;

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

        // 2. Mock mode detection
        let is_mock = if std::env::var("SODA_REAL_RAMDISK").is_ok() {
            false
        } else {
            cfg!(test) || std::env::var("SODA_MOCK_RAMDISK").is_ok()
        };

        if is_mock {
            let unique_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let mock_path = std::env::temp_dir().join(format!("soda_mock_ramdisk_{}", unique_id));
            std::fs::create_dir_all(&mock_path).map_err(|e| RamdiskError::AllocationFailed {
                reason: format!("Falha ao criar diretório mock: {}", e),
            })?;

            return Ok(RamdiskHandle {
                path: mock_path,
                is_mock: true,
                #[cfg(target_os = "windows")]
                drive_letter: None,
            });
        }

        // 3. Real OS allocation
        #[cfg(target_os = "windows")]
        {
            // PT-3: Verify imdisk via spawn_blocking para não bloquear o Event Loop
            let imdisk_installed = tokio::task::spawn_blocking(which_imdisk)
                .await
                .map_err(|e| RamdiskError::AllocationFailed {
                    reason: format!("Falha no spawn_blocking para verificar imdisk: {}", e),
                })?;
            if !imdisk_installed {
                return Err(RamdiskError::AllocationFailed {
                    reason: "imdisk.exe não encontrado no PATH ou em System32. Por favor, instale o ImDisk Virtual Disk Driver.".to_string(),
                });
            }

            let drive_letter = find_free_drive_letter().ok_or_else(|| {
                RamdiskError::AllocationFailed {
                    reason: "Nenhuma letra de unidade disponível encontrada (D-Z)".to_string(),
                }
            })?;

            // PT-3: Run imdisk via tokio::process::Command
            let output = tokio::process::Command::new("imdisk.exe")
                .arg("-a")
                .arg("-s")
                .arg(format!("{}M", tamanho_mb))
                .arg("-m")
                .arg(format!("{}:", drive_letter))
                .arg("-p")
                .arg("/fs:ntfs /q /y")
                .output()
                .await
                .map_err(|e| RamdiskError::AllocationFailed {
                    reason: format!("Falha ao executar imdisk: {}", e),
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(RamdiskError::AllocationFailed {
                    reason: format!("imdisk falhou com erro: {}", stderr.trim()),
                });
            }

            let path = PathBuf::from(format!("{}:\\", drive_letter));
            Ok(RamdiskHandle {
                path,
                is_mock: false,
                drive_letter: Some(drive_letter),
            })
        }

        #[cfg(target_os = "linux")]
        {
            let unique_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let mount_path = PathBuf::from(format!("/mnt/soda_ramdisk_{}", unique_id));
            std::fs::create_dir_all(&mount_path).map_err(|e| RamdiskError::AllocationFailed {
                reason: format!("Falha ao criar ponto de montagem: {}", e),
            })?;

            let output = tokio::process::Command::new("mount")
                .arg("-t")
                .arg("tmpfs")
                .arg("-o")
                .arg(format!("size={}M", tamanho_mb))
                .arg("soda_ramdisk")
                .arg(&mount_path)
                .output()
                .await
                .map_err(|e| RamdiskError::AllocationFailed {
                    reason: format!("Falha ao executar mount: {}", e),
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(RamdiskError::AllocationFailed {
                    reason: format!("mount falhou com erro: {}", stderr.trim()),
                });
            }

            Ok(RamdiskHandle {
                path: mount_path,
                is_mock: false,
            })
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(RamdiskError::UnsupportedPlatform)
        }
    }
}

#[cfg(target_os = "windows")]
fn which_imdisk() -> bool {
    if std::process::Command::new("where")
        .arg("imdisk.exe")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }
    // Check standard locations
    std::path::Path::new("C:\\Windows\\System32\\imdisk.exe").exists()
        || std::path::Path::new("C:\\Windows\\SysWOW64\\imdisk.exe").exists()
}

#[cfg(target_os = "windows")]
fn find_free_drive_letter() -> Option<char> {
    for c in ('D'..='Z').rev() {
        let path = format!("{}:\\", c);
        if !std::path::Path::new(&path).exists() {
            return Some(c);
        }
    }
    None
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
    async fn test_drop_unmounts() {
        let path = {
            let handle = RamdiskAllocator::allocate(64).await.unwrap();
            let p = handle.path().to_path_buf();
            assert!(p.exists());
            p
        };
        // Ao dropar o handle, o ponto de montagem deve desaparecer
        assert!(!path.exists(), "Ponto de montagem deveria ter sido desmontado/excluído");
    }

    #[tokio::test]
    async fn test_no_ssd_fallback() {
        // Se a alocação falhar, não deve criar nenhum fallback no disco local
        let result = RamdiskAllocator::allocate(1_000_000_000).await;
        assert!(result.is_err());
        // Garante que não foi criado nenhum diretório temporário no workspace de fallback
        assert!(!Path::new("R:\\").exists() && !Path::new("/mnt/soda_ramdisk").exists());
    }
}
