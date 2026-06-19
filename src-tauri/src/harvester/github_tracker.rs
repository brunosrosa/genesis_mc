use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use super::community::{CommunityMetaPayload, CommunityPrMeta, FetchError, RateLimiter};
use super::git::CloneError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitoxideCloneOutcome {
    pub cache_path: PathBuf,
    pub head_short_sha: String,
}

#[derive(Debug, Error)]
pub enum GithubTrackerError {
    #[error("GITHUB_PAT ausente no ambiente local/.env")]
    MissingGithubToken,

    #[error("URL GitHub inválida: {0}")]
    InvalidGithubUrl(String),

    #[error("Falha ao configurar cliente GitHub: {0}")]
    ClientConfig(String),

    #[error("Falha de rede GitHub: {0}")]
    Network(String),

    #[error("GitHub retornou recurso inexistente")]
    NotFound,

    #[error("GitHub retornou limite/bloqueio")]
    RateLimit,

    #[error("Payload GitHub inválido: {0}")]
    InvalidResponse(String),

    #[error("Falha gitoxide: {0}")]
    Gitoxide(String),

    #[error("Falha de I/O local: {0}")]
    Io(String),
}

#[derive(Debug, Deserialize)]
struct GithubRepoPayload {
    #[serde(default)]
    default_branch: String,
    #[serde(default)]
    stargazers_count: u64,
    #[serde(default)]
    forks_count: u64,
    #[serde(default)]
    open_issues_count: u32,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    license: Option<GithubLicensePayload>,
}

#[derive(Debug, Deserialize)]
struct GithubLicensePayload {
    #[serde(default)]
    spdx_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchIssueResponse {
    total_count: u32,
    #[serde(default)]
    items: Vec<GithubPullRequestPayload>,
}

#[derive(Debug, Deserialize)]
struct GithubPullRequestPayload {
    number: u64,
    state: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct GithubCommitPayload {
    sha: String,
    commit: GithubCommitInnerPayload,
}

#[derive(Debug, Deserialize)]
struct GithubCommitInnerPayload {
    author: GithubCommitAuthorPayload,
}

#[derive(Debug, Deserialize)]
struct GithubCommitAuthorPayload {
    date: DateTime<Utc>,
}

pub async fn fetch_community_meta(
    repo_url: &Url,
    limiter: &RateLimiter,
    api_base: Option<&str>,
) -> Result<CommunityMetaPayload, GithubTrackerError> {
    limiter.check().await;
    let token = required_github_token()?;
    let (owner, repo) = github_owner_repo(repo_url)?;
    let crab = build_octocrab(&token, api_base)?;

    let repo_route = format!("/repos/{owner}/{repo}");
    let repo_payload: GithubRepoPayload = github_get(&crab, &repo_route).await?;

    let default_branch = if repo_payload.default_branch.trim().is_empty() {
        "main".to_string()
    } else {
        repo_payload.default_branch.trim().to_string()
    };

    let open_prs_route = format!("/search/issues?q=repo:{owner}/{repo}+is:pr+is:open&per_page=1");
    let recent_prs_route =
        format!("/search/issues?q=repo:{owner}/{repo}+is:pr&sort=updated&order=desc&per_page=5");
    let commits_route = format!("/repos/{owner}/{repo}/commits?sha={default_branch}&per_page=1");

    let open_prs: SearchIssueResponse = github_get(&crab, &open_prs_route).await?;
    let recent_prs: SearchIssueResponse = github_get(&crab, &recent_prs_route).await?;
    let commits: Vec<GithubCommitPayload> = github_get(&crab, &commits_route).await?;
    let last_commit = commits.into_iter().next();

    let licenca = repo_payload
        .license
        .as_ref()
        .and_then(|license| {
            license
                .spdx_id
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("NOASSERTION"))
                .or_else(|| {
                    license
                        .name
                        .as_ref()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
        })
        .unwrap_or_else(|| "UNKNOWN".to_string());

    Ok(CommunityMetaPayload {
        stars_count: repo_payload.stargazers_count,
        forks_count: repo_payload.forks_count,
        open_issues_count: repo_payload.open_issues_count,
        open_prs_count: open_prs.total_count,
        licenca,
        description: repo_payload
            .description
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        full_name: repo_payload
            .full_name
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        name: repo_payload
            .name
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        last_commit_sha: last_commit.as_ref().map(|commit| commit.sha.clone()),
        last_commit_date: last_commit.map(|commit| commit.commit.author.date),
        recent_prs: recent_prs
            .items
            .into_iter()
            .map(|pr| CommunityPrMeta {
                number: pr.number,
                state: pr.state,
                updated_at: pr.updated_at,
            })
            .collect(),
    })
}

pub async fn fetch_community_meta_for_owner_repo(
    owner_repo: &str,
    limiter: &RateLimiter,
    api_base: Option<&str>,
) -> Result<CommunityMetaPayload, GithubTrackerError> {
    let normalized = normalize_owner_repo(owner_repo)?;
    let repo_url = Url::parse(&format!("https://github.com/{normalized}"))
        .map_err(|_| GithubTrackerError::InvalidGithubUrl(normalized.clone()))?;
    fetch_community_meta(&repo_url, limiter, api_base).await
}

pub async fn clone_or_fetch_to_workspace(
    repo_url: &Url,
    workspace_dest: &Path,
) -> Result<GitoxideCloneOutcome, CloneError> {
    let token = required_github_token().map_err(map_tracker_to_clone_error)?;
    let repo_url_string = repo_url.to_string();
    let workspace_dest = workspace_dest.to_path_buf();
    tokio::task::spawn_blocking(move || {
        clone_or_fetch_to_workspace_blocking(&repo_url_string, &workspace_dest, &token)
    })
    .await
    .map_err(|e| CloneError::NetworkError {
        reason: format!("Falha ao aguardar worker gitoxide: {}", e),
    })?
}

pub fn map_tracker_to_fetch_error(err: GithubTrackerError) -> FetchError {
    match err {
        GithubTrackerError::MissingGithubToken => {
            FetchError::Network("Missing GITHUB_PAT in .env".to_string())
        }
        GithubTrackerError::InvalidGithubUrl(reason) => FetchError::UnsupportedSource(reason),
        GithubTrackerError::NotFound => FetchError::NotFound,
        GithubTrackerError::RateLimit => FetchError::RateLimit,
        GithubTrackerError::InvalidResponse(reason) => FetchError::InvalidResponse(reason),
        GithubTrackerError::Network(reason)
        | GithubTrackerError::ClientConfig(reason)
        | GithubTrackerError::Gitoxide(reason)
        | GithubTrackerError::Io(reason) => FetchError::Network(reason),
    }
}

fn map_tracker_to_clone_error(err: GithubTrackerError) -> CloneError {
    match err {
        GithubTrackerError::InvalidGithubUrl(reason)
        | GithubTrackerError::ClientConfig(reason)
        | GithubTrackerError::Network(reason)
        | GithubTrackerError::Gitoxide(reason)
        | GithubTrackerError::Io(reason) => CloneError::NetworkError { reason },
        GithubTrackerError::NotFound => CloneError::RepositoryNotFound {
            url: "github".to_string(),
        },
        GithubTrackerError::RateLimit => CloneError::NetworkError {
            reason: "GitHub rate limit/bloqueio".to_string(),
        },
        GithubTrackerError::MissingGithubToken => CloneError::NetworkError {
            reason: "Missing GITHUB_PAT in .env".to_string(),
        },
        GithubTrackerError::InvalidResponse(reason) => CloneError::NetworkError { reason },
    }
}

fn clone_or_fetch_to_workspace_blocking(
    repo_url: &str,
    workspace_dest: &Path,
    _token: &str,
) -> Result<GitoxideCloneOutcome, CloneError> {
    static GIX_INTERRUPTS: Once = Once::new();
    GIX_INTERRUPTS.call_once(|| unsafe {
        let _ = gix::interrupt::init_handler(1, || {});
    });

    let repo_url = Url::parse(repo_url).map_err(|e| CloneError::NetworkError {
        reason: format!("URL inválida para clone gitoxide: {}", e),
    })?;
    let (owner, repo) = github_owner_repo(&repo_url).map_err(map_tracker_to_clone_error)?;
    let scratchpad_root = workspace_root()
        .join(".soda_scratchpad")
        .join("gitoxide_cache")
        .join(owner)
        .join(repo);
    let parent = scratchpad_root.parent().ok_or_else(|| CloneError::NetworkError {
        reason: format!("Destino scratchpad inválido: {}", scratchpad_root.display()),
    })?;
    std::fs::create_dir_all(parent).map_err(|e| CloneError::NetworkError {
        reason: format!("Falha ao preparar diretório do cache gitoxide: {}", e),
    })?;

    let nonce = format!("{}_{}", std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
    let staging_path = parent.join(format!(
        ".{}_stage_{}",
        scratchpad_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("repo"),
        nonce
    ));

    if staging_path.exists() {
        std::fs::remove_dir_all(&staging_path).map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao limpar staging gitoxide anterior: {}", e),
        })?;
    }
    std::fs::create_dir_all(&staging_path).map_err(|e| CloneError::NetworkError {
        reason: format!("Falha ao criar staging gitoxide: {}", e),
    })?;

    let url = gix::url::parse(repo_url.as_str().into()).map_err(|e| CloneError::NetworkError {
        reason: format!("Falha ao parsear URL para gitoxide: {}", e),
    })?;
    let mut prepare_clone =
        gix::prepare_clone(url, &staging_path).map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao preparar clone gitoxide: {}", e),
        })?;
    let (mut prepare_checkout, _) = prepare_clone
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| CloneError::NetworkError {
            reason: format!("Falha no fetch gitoxide: {}", e),
        })?;
    let (repo, _) = prepare_checkout
        .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| CloneError::NetworkError {
            reason: format!("Falha no checkout gitoxide: {}", e),
        })?;
    let head_short_sha = repo
        .head_id()
        .map(|id| id.to_string())
        .map(|full| full.chars().take(7).collect::<String>())
        .unwrap_or_else(|_| "UNKNOWN".to_string());
    drop(repo);

    if scratchpad_root.exists() {
        std::fs::remove_dir_all(&scratchpad_root).map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao substituir cache gitoxide antigo: {}", e),
        })?;
    }
    std::fs::rename(&staging_path, &scratchpad_root).map_err(|e| CloneError::NetworkError {
        reason: format!("Falha ao promover clone gitoxide atômico: {}", e),
    })?;

    if workspace_dest.exists() {
        std::fs::remove_dir_all(workspace_dest).map_err(|e| CloneError::NetworkError {
            reason: format!("Falha ao limpar destino do workspace: {}", e),
        })?;
    }
    copy_tree_without_git(&scratchpad_root, workspace_dest).map_err(|e| CloneError::NetworkError {
        reason: e,
    })?;

    Ok(GitoxideCloneOutcome {
        cache_path: scratchpad_root,
        head_short_sha,
    })
}

fn copy_tree_without_git(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("Falha ao criar destino do workspace '{}': {}", dest.display(), e))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("Falha ao listar '{}': {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Falha ao ler entrada em '{}': {}", src.display(), e))?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().eq(".git") {
            continue;
        }
        let dest_path = dest.join(&name);
        let metadata = entry
            .metadata()
            .map_err(|e| format!("Falha ao ler metadata '{}': {}", path.display(), e))?;
        if metadata.is_dir() {
            copy_tree_without_git(&path, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("Falha ao criar diretório pai '{}': {}", parent.display(), e)
                })?;
            }
            std::fs::copy(&path, &dest_path).map_err(|e| {
                format!(
                    "Falha ao copiar '{}' para '{}': {}",
                    path.display(),
                    dest_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

fn build_octocrab(
    token: &str,
    api_base: Option<&str>,
) -> Result<octocrab::Octocrab, GithubTrackerError> {
    let mut builder = octocrab::Octocrab::builder()
        .personal_token(token.to_string())
        .set_connect_timeout(Some(Duration::from_secs(10)));
    if let Some(base) = api_base {
        builder = builder
            .base_uri(base)
            .map_err(|e| GithubTrackerError::ClientConfig(e.to_string()))?;
    }
    builder
        .build()
        .map_err(|e| GithubTrackerError::ClientConfig(e.to_string()))
}

async fn github_get<T>(crab: &octocrab::Octocrab, route: &str) -> Result<T, GithubTrackerError>
where
    T: for<'de> Deserialize<'de>,
{
    crab.get(route, None::<&()>)
        .await
        .map_err(map_octocrab_error)
}

fn map_octocrab_error(err: octocrab::Error) -> GithubTrackerError {
    let message = err.to_string();
    if message.contains("404") {
        GithubTrackerError::NotFound
    } else if message.contains("403") || message.to_ascii_lowercase().contains("rate") {
        GithubTrackerError::RateLimit
    } else {
        GithubTrackerError::Network(message)
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

pub fn normalize_owner_repo(owner_repo: &str) -> Result<String, GithubTrackerError> {
    let trimmed = owner_repo.trim().trim_matches('/');
    let mut segments = trimmed
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() != 2 {
        return Err(GithubTrackerError::InvalidGithubUrl(owner_repo.trim().to_string()));
    }
    let repo = segments.pop().unwrap_or_default();
    let owner = segments.pop().unwrap_or_default();
    Ok(format!("{owner}/{repo}"))
}

fn required_github_token() -> Result<String, GithubTrackerError> {
    std::env::var("GITHUB_PAT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| read_local_env_var("GITHUB_PAT"))
        .ok_or(GithubTrackerError::MissingGithubToken)
}

fn read_local_env_var(key: &str) -> Option<String> {
    let candidates = [
        workspace_root().join(".env"),
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

fn github_owner_repo(repo_url: &Url) -> Result<(String, String), GithubTrackerError> {
    let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
    if repo_url.host_str() != Some("github.com") && !allow_host_override {
        return Err(GithubTrackerError::InvalidGithubUrl(repo_url.to_string()));
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
        return Err(GithubTrackerError::InvalidGithubUrl(repo_url.to_string()));
    }

    let repo = segments.pop().unwrap_or_else(|| "repo".to_string());
    let owner = segments.pop().unwrap_or_else(|| "owner".to_string());
    Ok((owner, repo))
}
