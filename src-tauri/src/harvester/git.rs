use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;
use url::Url;
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
#[cfg(not(target_os = "windows"))]
use tokio::time::timeout;
use super::ramdisk::RamdiskHandle;
use super::github_tracker;
#[cfg(not(target_os = "windows"))]
use super::sandbox::kill_process_tree_by_pid;
use tracing::{info, warn};
#[cfg(target_os = "windows")]
use serde::Deserialize;
#[cfg(target_os = "windows")]
use std::io::Cursor;
#[cfg(target_os = "windows")]
use zip::ZipArchive;

#[cfg(target_os = "windows")]
use super::projfs::ProjectedRepoSnapshot;

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

#[cfg(target_os = "windows")]
fn github_auth_header_value() -> Option<HeaderValue> {
    let token = std::env::var("GITHUB_PAT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| read_local_env_var("GITHUB_PAT"))
        .filter(|s| !s.is_empty())?;
    HeaderValue::from_str(&format!("Bearer {}", token)).ok()
}

#[cfg(target_os = "windows")]
fn read_local_env_var(key: &str) -> Option<String> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()?.join(".env"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env"),
    ];
    for candidate in candidates {
        let Ok(content) = std::fs::read_to_string(candidate) else {
            continue;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((name, value)) = trimmed.split_once('=') else {
                continue;
            };
            if name.trim() == key {
                return Some(
                    value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string(),
                )
                .filter(|value| !value.is_empty());
            }
        }
    }
    None
}

pub struct BloblessCloner;

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
struct GitHubRepoMetadata {
    default_branch: String,
}

impl BloblessCloner {
    #[cfg(target_os = "windows")]
    fn is_retryable_github_zip_error(reason: &str) -> bool {
        let lower = reason.to_ascii_lowercase();
        lower.contains("error decoding response body")
            || lower.contains("could not find eocd")
            || lower.contains("invalid zip")
            || lower.contains("snapshot não-zip")
    }

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

    #[cfg(not(target_os = "windows"))]
    async fn detach_git_metadata(dest: &Path) -> Result<(), CloneError> {
        let git_dir = dest.join(".git");
        let metadata = match tokio::fs::symlink_metadata(&git_dir).await {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                return Err(CloneError::NetworkError {
                    reason: format!("Falha ao inspecionar metadata git do clone: {}", e),
                });
            }
        };

        info!(path = %git_dir.display(), "Removendo metadata Git do workspace efemero antes dos sidecars");
        if metadata.is_dir() {
            tokio::fs::remove_dir_all(&git_dir).await.map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao remover diretório .git do workspace efêmero: {}", e),
            })?;
        } else {
            tokio::fs::remove_file(&git_dir).await.map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao remover arquivo .git do workspace efêmero: {}", e),
            })?;
        }
        info!(path = %git_dir.display(), "Metadata Git removida do workspace efemero");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn github_owner_repo(repo_url: &Url) -> Result<(String, String), CloneError> {
        let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
        if repo_url.host_str() != Some("github.com") && !allow_host_override {
            return Err(CloneError::NetworkError {
                reason: format!(
                    "Modo ProjFS em memória exige repositório GitHub; host recebido='{}'",
                    repo_url.host_str().unwrap_or("<none>")
                ),
            });
        }

        let mut segments = repo_url
            .path_segments()
            .map(|parts| parts.collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.trim_end_matches(".git").to_string())
            .collect::<Vec<_>>();

        if segments.len() < 2 {
            return Err(CloneError::NetworkError {
                reason: format!("URL do repositório GitHub inválida para ProjFS: {}", repo_url),
            });
        }

        let repo = segments.pop().unwrap_or_else(|| "repo".to_string());
        let owner = segments.pop().unwrap_or_else(|| "owner".to_string());
        Ok((owner, repo))
    }

    #[cfg(target_os = "windows")]
    async fn fetch_github_archive_bytes(
        repo_url: &Url,
    ) -> Result<(Vec<u8>, String, String), CloneError> {
        fn is_probably_zip(bytes: &[u8]) -> bool {
            bytes.starts_with(b"PK\x03\x04")
                || bytes.starts_with(b"PK\x05\x06")
                || bytes.starts_with(b"PK\x07\x08")
        }

        fn preview_bytes_lossy(bytes: &[u8]) -> String {
            let head_len = bytes.len().min(240);
            let head = &bytes[..head_len];
            let mut s = String::from_utf8_lossy(head).to_string();
            s = s.replace(['\r', '\n'], " ");
            if bytes.len() > head_len {
                s.push_str(" …");
            }
            s
        }

        async fn download_zip_bytes(
            client: &reqwest::Client,
            url: &str,
        ) -> Result<Vec<u8>, CloneError> {
            let resp = client.get(url).send().await.map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao baixar snapshot compactado do GitHub: {}", e),
            })?;
            let status = resp.status();
            let resp = resp.error_for_status().map_err(|e| CloneError::NetworkError {
                reason: format!("GitHub respondeu erro ao baixar snapshot compactado: {}", e),
            })?;
            let bytes = resp.bytes().await.map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao ler bytes do snapshot GitHub: {}", e),
            })?;
            let raw = bytes.to_vec();
            if !is_probably_zip(&raw) {
                return Err(CloneError::NetworkError {
                    reason: format!(
                        "ProjFS: snapshot não-ZIP recebido (status={}) bytes={} preview='{}' url={}",
                        status,
                        raw.len(),
                        preview_bytes_lossy(&raw),
                        url
                    ),
                });
            }
            Ok(raw)
        }

        let (owner, repo) = Self::github_owner_repo(repo_url)?;
        let github_api_base = std::env::var("SODA_GITHUB_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.github.com".to_string());
        let mut headers = HeaderMap::new();
        if let Some(value) = github_auth_header_value() {
            headers.insert(AUTHORIZATION, value);
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("genesis-mc-harvester-projfs/1.0")
            .default_headers(headers)
            .build()
            .map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao criar cliente HTTP para ProjFS: {}", e),
            })?;

        let metadata_url = format!("{}/repos/{owner}/{repo}", github_api_base.trim_end_matches('/'));
        info!(url = %metadata_url, "ProjFS: consultando metadados do repositório GitHub");
        let metadata_response = client
            .get(&metadata_url)
            .send()
            .await
            .map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao consultar metadados do GitHub: {}", e),
            })?;
        if metadata_response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(CloneError::RepositoryNotFound {
                url: repo_url.as_str().to_string(),
            });
        }
        let metadata_response = metadata_response.error_for_status().map_err(|e| CloneError::NetworkError {
            reason: format!("GitHub respondeu erro ao consultar metadados do repositório: {}", e),
        })?;
        let metadata = metadata_response
            .json::<GitHubRepoMetadata>()
            .await
            .map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao decodificar metadados do GitHub: {}", e),
            })?;

        #[derive(Deserialize)]
        struct GithubRelease {
            tag_name: Option<String>,
        }

        #[derive(Deserialize)]
        struct GithubCommit {
            sha: String,
        }

        let release_url = format!(
            "{}/repos/{owner}/{repo}/releases/latest",
            github_api_base.trim_end_matches('/')
        );
        info!(url = %release_url, "ProjFS: consultando release mais recente do repositório");
        let release_tag = match client.get(&release_url).send().await {
            Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => None,
            Ok(resp) if resp.status().is_success() => match resp.json::<GithubRelease>().await {
                Ok(release) => release.tag_name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                Err(_) => None,
            },
            _ => None,
        };

        let mut candidate_branches = vec![metadata.default_branch.clone()];
        if metadata.default_branch.eq_ignore_ascii_case("main") {
            candidate_branches.push("master".to_string());
        } else if metadata.default_branch.eq_ignore_ascii_case("master") {
            candidate_branches.push("main".to_string());
        }

        let mut selected_branch: Option<String> = None;
        let mut head_sha: Option<String> = None;
        for branch in candidate_branches.iter() {
            let commits_url = format!(
                "{}/repos/{owner}/{repo}/commits?sha={}&per_page=1",
                github_api_base.trim_end_matches('/'),
                branch
            );
            info!(url = %commits_url, "ProjFS: consultando SHA do commit HEAD");
            let commits_resp = client.get(&commits_url).send().await.map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao consultar commits do GitHub: {}", e),
            })?;
            if commits_resp.status() == reqwest::StatusCode::NOT_FOUND {
                continue;
            }
            let commits_resp = commits_resp.error_for_status().map_err(|e| CloneError::NetworkError {
                reason: format!("GitHub respondeu erro ao consultar commits: {}", e),
            })?;
            let commits = commits_resp.json::<Vec<GithubCommit>>().await.map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao decodificar commits do GitHub: {}", e),
            })?;
            let sha = commits
                .first()
                .map(|c| c.sha.trim().to_string())
                .filter(|s| !s.is_empty());
            if let Some(sha) = sha {
                selected_branch = Some(branch.clone());
                head_sha = Some(sha);
                break;
            }
        }

        let selected_branch = selected_branch.ok_or_else(|| CloneError::NetworkError {
            reason: "GitHub não retornou commits válidos (main/master); impossível extrair SHA".to_string(),
        })?;
        let head_sha = head_sha.ok_or_else(|| CloneError::NetworkError {
            reason: "GitHub não retornou SHA válido; impossível extrair SHA".to_string(),
        })?;
        let short_sha = head_sha.chars().take(7).collect::<String>();
        let repo_version = release_tag.clone().unwrap_or_else(|| short_sha.clone());
        let ultima_versao_online = release_tag.unwrap_or(short_sha);

        let api_archive_url = format!(
            "{}/repos/{owner}/{repo}/zipball/{}",
            github_api_base.trim_end_matches('/'),
            selected_branch
        );
        info!(
            url = %api_archive_url,
            default_branch = %metadata.default_branch,
            selected_branch = %selected_branch,
            "ProjFS: baixando snapshot compactado do repositório"
        );

        let bytes = match download_zip_bytes(&client, &api_archive_url).await {
            Ok(bytes) => bytes,
            Err(e) => {
                let codeload_url = format!(
                    "https://codeload.github.com/{owner}/{repo}/zip/refs/heads/{selected_branch}"
                );
                warn!(
                    url = %repo_url,
                    error = %e,
                    fallback_url = %codeload_url,
                    "ProjFS: fallback para codeload (ZIP) após falha no endpoint zipball"
                );
                download_zip_bytes(&client, &codeload_url).await?
            }
        };
        info!(archive_bytes = bytes.len(), "ProjFS: snapshot ZIP recebido em memória");
        Ok((bytes, repo_version, ultima_versao_online))
    }

    #[cfg(target_os = "windows")]
    fn build_projfs_snapshot(archive_bytes: Vec<u8>) -> Result<ProjectedRepoSnapshot, CloneError> {
        let cursor = Cursor::new(archive_bytes);
        let mut zip = ZipArchive::new(cursor).map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao abrir arquivo ZIP do GitHub em memória: {}", e),
        })?;

        let mut files = Vec::new();
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao ler entrada {index} do ZIP do GitHub: {}", e),
            })?;
            if entry.is_dir() {
                continue;
            }

            let raw_name = entry.name().replace('\\', "/");
            let mut parts = raw_name.split('/').filter(|part| !part.is_empty());
            let _archive_root = parts.next();
            let relative = parts.collect::<Vec<_>>();
            if relative.is_empty() {
                continue;
            }

            let relative_path = PathBuf::from(relative.join("/"));
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            std::io::Read::read_to_end(&mut entry, &mut buffer).map_err(|e| CloneError::NetworkError {
                reason: format!("Falha ao descompactar '{}' do snapshot GitHub: {}", raw_name, e),
            })?;
            files.push((relative_path, buffer));
        }

        ProjectedRepoSnapshot::from_files(files).map_err(|reason| CloneError::NetworkError { reason })
    }

    #[cfg(target_os = "windows")]
    async fn git_clone_fallback(repo_url: &Url, dest: &Path) -> Result<(), CloneError> {
        github_tracker::clone_or_fetch_to_workspace(repo_url, dest)
            .await
            .map(|_| ())
    }

    pub async fn clone(
        repo_url: &Url,
        ramdisk: &mut RamdiskHandle,
    ) -> Result<RepoPath, CloneError> {
        // 2. Workspace efemero com hint de cache temporario nativo para esta execucao.
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

        #[cfg(target_os = "windows")]
        {
            let archive_started = Instant::now();
            let max_attempts: u32 = 4;
            let mut last_error: Option<CloneError> = None;
            let mut last_repo_version: Option<String> = None;
            let mut last_ultima_versao_online: Option<String> = None;

            for attempt in 1..=max_attempts {
                let (archive_bytes, repo_version, ultima_versao_online) =
                    match Self::fetch_github_archive_bytes(repo_url).await {
                        Ok(value) => value,
                        Err(e) => {
                            let retry = matches!(&e, CloneError::NetworkError { reason } if Self::is_retryable_github_zip_error(reason));
                            if attempt < max_attempts && retry {
                                warn!(
                                    url = %repo_url,
                                    attempt,
                                    error = %e,
                                    "ProjFS: falha transitória ao baixar ZIP; aplicando retry"
                                );
                                let backoff_ms = (2_000_u64.saturating_mul(1u64 << (attempt.saturating_sub(1).min(6))))
                                    .min(120_000);
                                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                                last_error = Some(e);
                                continue;
                            }
                            return Err(e);
                        }
                    };
                last_repo_version = Some(repo_version.clone());
                last_ultima_versao_online = Some(ultima_versao_online.clone());

                let snapshot = match tokio::task::spawn_blocking(move || Self::build_projfs_snapshot(archive_bytes))
                    .await
                    .map_err(|e| CloneError::NetworkError {
                        reason: format!("Falha ao aguardar montagem do snapshot ProjFS: {}", e),
                    }) {
                    Ok(Ok(snapshot)) => snapshot,
                    Ok(Err(e)) => {
                        let retry = matches!(&e, CloneError::NetworkError { reason } if Self::is_retryable_github_zip_error(reason));
                        if attempt < max_attempts && retry {
                            warn!(
                                url = %repo_url,
                                attempt,
                                error = %e,
                                "ProjFS: snapshot ZIP inválido/corrompido; aplicando retry"
                            );
                            let backoff_ms = (2_000_u64.saturating_mul(1u64 << (attempt.saturating_sub(1).min(6))))
                                .min(120_000);
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            last_error = Some(e);
                            continue;
                        }
                        return Err(e);
                    }
                    Err(e) => return Err(e),
                };

                let (projected_files, projected_bytes) = ramdisk
                    .mount_projected_repo(&dest, snapshot)
                    .map_err(|e| CloneError::NetworkError {
                        reason: format!("Falha ao iniciar projeção ProjFS do repositório: {}", e),
                    })?;
                tokio::fs::write(dest.join(".soda_repo_version"), repo_version)
                    .await
                    .map_err(|e| CloneError::NetworkError {
                        reason: format!("Falha ao persistir repo_version no workspace ProjFS: {}", e),
                    })?;
                tokio::fs::write(dest.join(".soda_ultima_versao_online"), ultima_versao_online)
                    .await
                    .map_err(|e| CloneError::NetworkError {
                        reason: format!(
                            "Falha ao persistir ultima_versao_online no workspace ProjFS: {}",
                            e
                        ),
                    })?;
                info!(
                    url = %repo_url,
                    dest = %dest.display(),
                    projected_files,
                    projected_bytes,
                    elapsed_ms = archive_started.elapsed().as_millis(),
                    "Clone virtual via ProjFS concluido"
                );
                return Ok(RepoPath(dest));
            }

            if let Some(err) = last_error.clone() {
                let should_try_git = matches!(
                    &err,
                    CloneError::NetworkError { reason } if Self::is_retryable_github_zip_error(reason)
                );
                if should_try_git {
                    warn!(
                        url = %repo_url,
                        error = %err,
                        "ProjFS: retries esgotados; aplicando fallback para git clone (main/master)"
                    );
                    let repo_version = last_repo_version.unwrap_or_default();
                    let ultima_versao_online = last_ultima_versao_online.unwrap_or_default();
                    Self::git_clone_fallback(repo_url, &dest).await?;
                    if !repo_version.is_empty() {
                        tokio::fs::write(dest.join(".soda_repo_version"), repo_version)
                            .await
                            .map_err(|e| CloneError::NetworkError {
                                reason: format!("Falha ao persistir repo_version no workspace: {}", e),
                            })?;
                    }
                    if !ultima_versao_online.is_empty() {
                        tokio::fs::write(dest.join(".soda_ultima_versao_online"), ultima_versao_online)
                            .await
                            .map_err(|e| CloneError::NetworkError {
                                reason: format!("Falha ao persistir ultima_versao_online no workspace: {}", e),
                            })?;
                    }
                    return Ok(RepoPath(dest));
                }
                return Err(err);
            }
            return Err(CloneError::NetworkError {
                reason: "ProjFS: falha inesperada (loop de retries encerrou sem erro registrado)".to_string(),
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            let clone_started = Instant::now();
            info!(url = %repo_url, dest = %dest.display(), "Iniciando clone gitoxide nativo");
            let outcome = github_tracker::clone_or_fetch_to_workspace(repo_url, &dest).await?;
            let _ = tokio::fs::write(dest.join(".soda_repo_version"), &outcome.head_short_sha).await;
            let _ = tokio::fs::write(dest.join(".soda_ultima_versao_online"), &outcome.head_short_sha).await;
            Self::detach_git_metadata(&dest).await?;
            info!(
                url = %repo_url,
                dest = %dest.display(),
                cache = %outcome.cache_path.display(),
                elapsed_ms = clone_started.elapsed().as_millis(),
                "Clone gitoxide concluido"
            );
            return Ok(RepoPath(dest));
        }

        #[allow(unreachable_code)]
        Err(CloneError::NetworkError {
            reason: "Caminho de clone inalcançável".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::ramdisk::RamdiskAllocator;
    use mockito::Server;
    use std::sync::OnceLock;

    static TEST_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

    fn get_test_mutex() -> &'static tokio::sync::Mutex<()> {
        TEST_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    #[tokio::test]
    async fn test_clone_success() {
        if std::env::var("SODA_RUN_GITHUB_NETWORK_TESTS")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }
        let _guard = get_test_mutex().lock().await;
        let mut ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        // Repositório minúsculo e estável para teste rápido
        let repo_url = Url::parse("https://github.com/octocat/Spoon-Knife").unwrap();
        
        let repo_path = BloblessCloner::clone(&repo_url, &mut ramdisk).await.unwrap();
        assert!(repo_path.exists());
        assert!(!repo_path.join(".git").exists());
    }

    #[tokio::test]
    async fn test_clone_repo_not_found() {
        let _guard = get_test_mutex().lock().await;
        let mut ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let mut server = Server::new_async().await;
        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        let _m = server
            .mock("GET", "/repos/octocat/repo")
            .with_status(404)
            .with_body("Repository not found")
            .create_async()
            .await;
        let repo_url = Url::parse(&format!("{}/octocat/repo", server.url())).unwrap();
        
        let err = BloblessCloner::clone(&repo_url, &mut ramdisk).await.unwrap_err();
        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
        assert!(
            matches!(err, CloneError::RepositoryNotFound { .. }),
            "Deveria falhar com RepositoryNotFound, mas retornou {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_clone_stays_in_ramdisk() {
        if std::env::var("SODA_RUN_GITHUB_NETWORK_TESTS")
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

    #[tokio::test]
    async fn test_cleanup_on_failure() {
        let _guard = get_test_mutex().lock().await;
        let mut ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let mut server = Server::new_async().await;
        std::env::set_var("SODA_GITHUB_API_BASE_URL", server.url());
        let _m = server
            .mock("GET", "/repos/octocat/repo")
            .with_status(404)
            .with_body("Repository not found")
            .create_async()
            .await;
        let repo_url = Url::parse(&format!("{}/octocat/repo", server.url())).unwrap();
        
        let _ = BloblessCloner::clone(&repo_url, &mut ramdisk).await;
        std::env::remove_var("SODA_GITHUB_API_BASE_URL");
        
        let repo_root = ramdisk.path().join("repos");
        if !repo_root.exists() {
            return;
        }
        for owner in std::fs::read_dir(&repo_root).unwrap() {
            let owner = owner.unwrap();
            if owner.path().is_dir() {
                let entries = std::fs::read_dir(owner.path()).unwrap().count();
                assert_eq!(
                    entries, 0,
                    "O cache não deve preservar repositórios parciais após falha"
                );
            }
        }
    }

    #[cfg_attr(target_os = "windows", ignore = "ProjFS em memória não usa o binário git no Windows")]
    #[tokio::test]
    async fn test_git_not_installed() {
        let _guard = get_test_mutex().lock().await;
        // Usamos uma técnica de adulteração do PATH temporária para simular ausência do git
        let old_path = std::env::var_os("PATH");
        std::env::set_var("PATH", ""); // Limpa o PATH para o git não ser encontrado
        
        let mut ramdisk = RamdiskAllocator::allocate(64).await.unwrap();
        let repo_url = Url::parse("https://github.com/octocat/Spoon-Knife").unwrap();
        
        let err = BloblessCloner::clone(&repo_url, &mut ramdisk).await.unwrap_err();
        
        // Restaura o PATH
        if let Some(path) = old_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
        
        assert_eq!(err, CloneError::GitNotInstalled);
    }
}
