use std::ops::Deref;
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;
use sha2::{Digest, Sha256};
use tracing::info;
use super::ramdisk::RamdiskHandle;

#[derive(Debug)]
pub struct RepoPath(pub PathBuf);

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

    async fn run_git_clone(repo_url: &Url, dest: &Path) -> Result<String, CloneError> {
        info!("Iniciando git clone blobless com submodulos para {}", dest.display());
        
        // Verifica se o git está instalado
        let check_git = tokio::process::Command::new("git")
            .arg("--version")
            .output()
            .await;
        if check_git.is_err() {
            return Err(CloneError::GitNotInstalled);
        }

        let clone_status = tokio::process::Command::new("git")
            .args([
                "clone",
                "--filter=blob:none",
                "--recurse-submodules",
                repo_url.as_str(),
                &dest.to_string_lossy(),
            ])
            .status()
            .await
            .map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao rodar git clone: {}", e),
            })?;

        if !clone_status.success() {
            return Err(CloneError::RepositoryNotFound {
                url: repo_url.to_string(),
            });
        }

        // Obtém o short SHA do HEAD
        let sha_output = tokio::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(dest)
            .output()
            .await
            .map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao ler short SHA do clone: {}", e),
            })?;

        if !sha_output.status.success() {
            let stderr = String::from_utf8_lossy(&sha_output.stderr).to_string();
            return Err(CloneError::NetworkError {
                reason: format!("Erro ao obter short SHA do commit HEAD: {}", stderr),
            });
        }

        let sha = String::from_utf8_lossy(&sha_output.stdout).trim().to_string();
        Ok(sha)
    }

    pub async fn clone(
        repo_url: &Url,
        ramdisk: &mut RamdiskHandle,
    ) -> Result<RepoPath, CloneError> {
        let dest = Self::repo_workspace_destination(ramdisk, repo_url);
        info!(
            url = %repo_url,
            workspace = %ramdisk.path().display(),
            dest = %dest.display(),
            "Preparando workspace efemero do clone"
        );
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

        let clone_result = Self::run_git_clone(repo_url, &dest).await;
        match clone_result {
            Ok(sha) => {
                // Grava as versões do repositório
                let _ = tokio::fs::write(dest.join(".souls_repo_version"), &sha).await;
                let _ = tokio::fs::write(dest.join(".souls_ultima_versao_online"), &sha).await;
                Ok(RepoPath(dest))
            }
            Err(e) => {
                // Cleanup em caso de falha para evitar lixo parcial
                let _ = tokio::fs::remove_dir_all(&dest).await;
                Err(e)
            }
        }
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
        if std::env::var("SOULS_RUN_GITHUB_NETWORK_TESTS")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }
        let _guard = get_test_mutex().lock().await;
        let mut ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let repo_url = Url::parse("https://github.com/octocat/Spoon-Knife").unwrap();
        
        let repo_path = BloblessCloner::clone(&repo_url, &mut ramdisk).await.unwrap();
        assert!(repo_path.exists());
        // PRD-032: O diretório .git DEVE continuar existindo fisicamente (motor de hidratação)
        assert!(repo_path.join(".git").exists());
    }

    #[tokio::test]
    async fn test_clone_repo_not_found() {
        let _guard = get_test_mutex().lock().await;
        let mut ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        // Uma URL inválida que cause falha no git clone
        let repo_url = Url::parse("https://github.com/invalid_owner_12345/invalid_repo_12345").unwrap();
        
        let err = BloblessCloner::clone(&repo_url, &mut ramdisk).await.unwrap_err();
        assert!(
            matches!(err, CloneError::RepositoryNotFound { .. }),
            "Deveria falhar com RepositoryNotFound, mas retornou {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_clone_stays_in_ramdisk() {
        if std::env::var("SOULS_RUN_GITHUB_NETWORK_TESTS")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }
        let _guard = get_test_mutex().lock().await;
        let mut ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let repo_url = Url::parse("https://github.com/octocat/Spoon-Knife").unwrap();
        
        let repo_path = BloblessCloner::clone(&repo_url, &mut ramdisk).await.unwrap();
        assert!(
            repo_path.starts_with(ramdisk.path()),
            "O clone deve residir estritamente dentro do workspace efêmero"
        );
    }
}
