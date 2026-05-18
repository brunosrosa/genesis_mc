use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use url::Url;
use sha2::{Digest, Sha256};
use tokio::time::timeout;
use super::ramdisk::RamdiskHandle;
use super::sandbox::kill_process_tree_by_pid;
use tracing::info;

#[derive(Debug)]
pub struct RepoPath(pub(crate) PathBuf);

impl AsRef<Path> for RepoPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Deref for RepoPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CloneError {
    #[error("Network error: {reason}")]
    NetworkError { reason: String },

    #[error("Repository not found: {url}")]
    RepositoryNotFound { url: String },

    #[error("Git binary not found in PATH")]
    GitNotInstalled,

    #[error("Ramdisk is full or write failed at: {path}")]
    RamdiskFull { path: String },

    #[error("Clone operation timed out")]
    Timeout,
}

pub struct BloblessCloner;

impl BloblessCloner {
    fn repo_workspace_destination(workspace: &RamdiskHandle, repo_url: &Url) -> PathBuf {
        let mut segments = repo_url
            .path_segments()
            .map(|parts| parts.collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.trim_end_matches(".git").to_string())
            .collect::<Vec<_>>();

        if segments.len() >= 2 {
            let repo = segments.pop().unwrap_or_else(|| "repo".to_string());
            let owner = segments.pop().unwrap_or_else(|| "owner".to_string());
            return workspace.path().join("repos").join(owner).join(repo);
        }

        let mut hasher = Sha256::new();
        hasher.update(repo_url.as_str().as_bytes());
        let hash_result = hasher.finalize();
        let hex_string = format!("{:x}", hash_result);
        let truncated_hash = &hex_string[..12];
        workspace.path().join("repos").join(format!("repo_{}", truncated_hash))
    }

    async fn directory_has_files(path: &Path) -> Result<bool, CloneError> {
        let mut entries = tokio::fs::read_dir(path).await.map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao inspecionar cache local do repositório: {}", e),
        })?;
        Ok(entries.next_entry().await.map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao iterar cache local do repositório: {}", e),
        })?.is_some())
    }

    pub async fn clone(
        repo_url: &Url,
        ramdisk: &RamdiskHandle,
    ) -> Result<RepoPath, CloneError> {
        // 1. Verificação preliminar e assíncrona se o Git está instalado
        match tokio::process::Command::new("git")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
        {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CloneError::GitNotInstalled);
            }
            Err(e) => {
                return Err(CloneError::NetworkError {
                    reason: format!("Erro ao tentar verificar presença do git no sistema: {}", e),
                });
            }
        }

        // 2. Workspace efemero com hint de cache temporario nativo para esta execucao.
        let dest = Self::repo_workspace_destination(ramdisk, repo_url);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao preparar diretório pai do workspace do repositório: {}", e),
            })?;
        }

        if tokio::fs::try_exists(&dest).await.map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao verificar existência do workspace do repositório: {}", e),
        })? && Self::directory_has_files(&dest).await? {
            info!(path = %dest.display(), url = %repo_url, "Workspace efêmero do repositório já está preparado; pulando clone");
            return Ok(RepoPath(dest));
        }

        // 3. Spawning do comando git clone de forma assíncrona com as flags otimizadas do SODA
        let mut child = match tokio::process::Command::new("git")
            .arg("clone")
            .arg("--filter=blob:none")
            .arg("--single-branch")
            .arg("--no-tags")
            .arg("--quiet")
            .arg(repo_url.as_str())
            .arg(&dest)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CloneError::GitNotInstalled);
            }
            Err(e) => {
                return Err(CloneError::NetworkError {
                    reason: format!("Falha ao spawnar processo git clone: {}", e),
                });
            }
        };

        // Extrai o stream do stderr para leitura concorrente (evita mover child)
        let mut stderr_stream = child.stderr.take().ok_or_else(|| {
            CloneError::NetworkError {
                reason: "Não foi possível capturar stderr do processo Git".to_string(),
            }
        })?;

        // 4. Executar concorrentemente wait() e leitura de stderr com Timeout de 600s
        let mut stderr_buffer = Vec::new();
        let run_fut = async {
            use tokio::io::AsyncReadExt;
            let status = child.wait().await;
            let _ = stderr_stream.read_to_end(&mut stderr_buffer).await;
            status
        };

        let wait_result = timeout(Duration::from_secs(600), run_fut).await;

        if wait_result.is_err() {
            // Timeout expirou! Matamos o processo de forma assíncrona
            if let Some(pid) = child.id() {
                kill_process_tree_by_pid(pid).await;
            } else {
                let _ = child.kill().await;
            }
            // PT-3: Limpa o diretório parcial sem bloquear o Event Loop
            let _ = tokio::fs::remove_dir_all(&dest).await;
            return Err(CloneError::Timeout);
        }

        let status = match wait_result.unwrap() {
            Ok(s) => s,
            Err(e) => {
                // PT-3: Limpeza assíncrona em caso de erro de I/O
                let _ = tokio::fs::remove_dir_all(&dest).await;
                return Err(CloneError::NetworkError {
                    reason: format!("Erro de I/O ao aguardar término do processo git: {}", e),
                });
            }
        };

        // 5. Analisar o resultado do processo
        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr_buffer);
            
            // PT-3: Limpa o diretório de clone parcial sem bloquear o Event Loop
            let _ = tokio::fs::remove_dir_all(&dest).await;

            let stderr_lower = stderr.to_lowercase();
            if stderr_lower.contains("repository") && stderr_lower.contains("not found") {
                return Err(CloneError::RepositoryNotFound {
                    url: repo_url.as_str().to_string(),
                });
            } else if stderr_lower.contains("no space left") || stderr_lower.contains("disk full") {
                return Err(CloneError::RamdiskFull {
                    path: dest.display().to_string(),
                });
            } else if stderr_lower.contains("could not resolve host") || stderr_lower.contains("connection refused") {
                return Err(CloneError::NetworkError {
                    reason: format!("Erro de rede/resolução de DNS: {}", stderr.trim()),
                });
            } else {
                return Err(CloneError::NetworkError {
                    reason: format!("Git falhou com código {:?}: {}", status.code(), stderr.trim()),
                });
            }
        }

        #[cfg(target_os = "windows")]
        ramdisk
            .prime_temp_cache(&dest)
            .await
            .map_err(|e| CloneError::NetworkError {
                reason: format!(
                    "Falha ao aplicar as flags temporarias do Cache Manager no workspace: {}",
                    e
                ),
            })?;

        Ok(RepoPath(dest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::ramdisk::RamdiskAllocator;
    use std::sync::OnceLock;

    static TEST_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

    fn get_test_mutex() -> &'static tokio::sync::Mutex<()> {
        TEST_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    #[tokio::test]
    async fn test_clone_success() {
        let _guard = get_test_mutex().lock().await;
        let ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        // Repositório minúsculo e estável para teste rápido
        let repo_url = Url::parse("https://github.com/octocat/Spoon-Knife").unwrap();
        
        let repo_path = BloblessCloner::clone(&repo_url, &ramdisk).await.unwrap();
        assert!(repo_path.exists());
        assert!(repo_path.join(".git").exists());
    }

    #[tokio::test]
    async fn test_clone_repo_not_found() {
        let _guard = get_test_mutex().lock().await;
        let ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let repo_url = Url::parse("https://github.com/octocat/this-repo-should-not-exist-ever-soda").unwrap();
        
        let err = BloblessCloner::clone(&repo_url, &ramdisk).await.unwrap_err();
        assert!(
            matches!(err, CloneError::RepositoryNotFound { .. }),
            "Deveria falhar com RepositoryNotFound, mas retornou {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_clone_stays_in_ramdisk() {
        let _guard = get_test_mutex().lock().await;
        let ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let repo_url = Url::parse("https://github.com/octocat/Spoon-Knife").unwrap();
        
        let repo_path = BloblessCloner::clone(&repo_url, &ramdisk).await.unwrap();
        assert!(
            repo_path.starts_with(ramdisk.path()),
            "O clone deve residir estritamente dentro do workspace efêmero"
        );
    }

    #[tokio::test]
    async fn test_cleanup_on_failure() {
        let _guard = get_test_mutex().lock().await;
        let ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let repo_url = Url::parse("https://github.com/octocat/this-repo-should-not-exist-ever-soda").unwrap();
        
        let _ = BloblessCloner::clone(&repo_url, &ramdisk).await;
        
        let repo_root = ramdisk.path().join("repos");
        if repo_root.exists() {
            let entries = std::fs::read_dir(repo_root).unwrap().count();
            assert_eq!(entries, 0, "O cache não deve preservar diretórios parciais após falha");
        }
    }

    #[tokio::test]
    async fn test_git_not_installed() {
        let _guard = get_test_mutex().lock().await;
        // Usamos uma técnica de adulteração do PATH temporária para simular ausência do git
        let old_path = std::env::var_os("PATH");
        std::env::set_var("PATH", ""); // Limpa o PATH para o git não ser encontrado
        
        let ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let repo_url = Url::parse("https://github.com/octocat/Spoon-Knife").unwrap();
        
        let err = BloblessCloner::clone(&repo_url, &ramdisk).await.unwrap_err();
        
        // Restaura o PATH
        if let Some(path) = old_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
        
        assert_eq!(err, CloneError::GitNotInstalled);
    }
}
