use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};
use url::Url;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use souls_mc_lib::telemetry::{enable_virtual_terminal, init_cli_tracing, parse_log_level_from_env};

type RepoStoreFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubResponse {
    NotModified,
    NewRelease {
        tag: Option<String>,
        etag: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRepoCtx {
    pub project_name: String,
    pub repo_url: String,
    pub repo_analised_version: String,
    pub ultima_versao_online: String,
    pub etag: Option<String>,
}

trait GithubClient: Send + Sync {
    fn latest_release_tag<'a>(
        &'a self,
        repo_url: &'a str,
        etag: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<GithubResponse, String>> + Send + 'a>>;
}

trait RepoStore: Send + Sync {
    fn fetch_pending_repos<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PendingRepoCtx>, String>> + Send + 'a>>;

    fn persist_release_resolution<'a>(
        &'a self,
        project_name: &'a str,
        repo_url: &'a str,
        latest: &'a str,
        etag: Option<&'a str>,
        status: &'a str,
    ) -> RepoStoreFuture<'a>;
}

struct ReqwestGithubClient {
    http: Client,
    api_base: String,
    allow_host_override: bool,
    github_pat: String,
    policy: RetryPolicy,
    jitter_state: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
struct RetryPolicy {
    max_attempts: u32,
    jitter_min_ms: u64,
    jitter_max_ms: u64,
    backoff_base_ms: u64,
}

struct SqliteRepoStore {
    db_path: PathBuf,
}

impl SqliteRepoStore {
    fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

fn is_invalid_version_seed(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("main")
        || trimmed.eq_ignore_ascii_case("master")
        || trimmed.eq_ignore_ascii_case("unknown")
}

fn normalize_version(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.starts_with('v') || s.starts_with('V') {
        s = s[1..].to_string();
    }
    if let Some(stripped) = s.strip_prefix("release-") {
        s = stripped.to_string();
    }
    s.trim().to_string()
}

fn has_drift(repo_analised_version: &str, github_latest: &str) -> bool {
    let local = normalize_version(repo_analised_version);
    let remote = normalize_version(github_latest);
    !(remote.is_empty() || (!local.is_empty() && local == remote))
}

fn try_extract_project_name_from_repo_url(repo_url: &str) -> Option<String> {
    let url = Url::parse(repo_url).ok()?;
    if !url.host_str()?.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let mut parts = url.path().trim_matches('/').split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

impl RepoStore for SqliteRepoStore {
    fn fetch_pending_repos<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PendingRepoCtx>, String>> + Send + 'a>> {
        let db_path = self.db_path.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || -> Result<Vec<PendingRepoCtx>, String> {
                let conn = Connection::open(&db_path)
                    .map_err(|e| format!("Guardião: falha ao abrir SQLite em {}: {}", db_path.display(), e))?;
                
                let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN etag TEXT", []);
                
                let mut stmt = conn
                    .prepare(
                        "SELECT project_name, repo_url, 
                                COALESCE(repo_analised_version, ''), 
                                COALESCE(ultima_versao_online, ''), 
                                etag
                         FROM repositorios
                         WHERE status_processamento = 'PENDENTE'",
                    )
                    .map_err(|e| format!("Guardião: erro na query: {e}"))?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok(PendingRepoCtx {
                            project_name: row.get(0)?,
                            repo_url: row.get(1)?,
                            repo_analised_version: row.get(2)?,
                            ultima_versao_online: row.get(3)?,
                            etag: row.get(4)?,
                        })
                    })
                    .map_err(|e| format!("Guardião: erro na query: {e}"))?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(|e| e.to_string())?);
                }
                Ok(out)
            })
            .await
            .map_err(|e| format!("Join error: {e}"))?
        })
    }

    fn persist_release_resolution<'a>(
        &'a self,
        project_name: &'a str,
        repo_url: &'a str,
        latest: &'a str,
        etag: Option<&'a str>,
        status: &'a str,
    ) -> RepoStoreFuture<'a> {
        let db_path = self.db_path.clone();
        let project_name = project_name.trim().to_string();
        let repo_url = repo_url.trim().to_string();
        let latest = latest.trim().to_string();
        let etag = etag.map(|s| s.trim().to_string());
        let status = status.trim().to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || -> Result<(), String> {
                let conn = Connection::open(&db_path)
                    .map_err(|e| format!("Guardião: falha ao abrir SQLite em {}: {}", db_path.display(), e))?;
                
                let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN etag TEXT", []);
                
                let repo_key = if !project_name.is_empty() {
                    project_name
                } else {
                    try_extract_project_name_from_repo_url(&repo_url).unwrap_or_default()
                };
                if repo_key.is_empty() {
                    return Err(format!(
                        "Guardião: não foi possível derivar project_name para persistir versão (repo_url={repo_url})"
                    ));
                }

                if !latest.is_empty() && is_invalid_version_seed(&latest) {
                    return Err(format!(
                        "Guardião: versão resolvida inválida para persistência no SQLite: '{}'",
                        latest
                    ));
                }

                let updated_rows = conn
                    .execute(
                        "UPDATE repositorios
                         SET ultima_versao_online = ?1,
                             repo_version = ?1,
                             repo_analised_version = CASE
                                 WHEN repo_analised_version IS NULL THEN ''
                                 WHEN lower(trim(repo_analised_version)) IN ('main','master','unknown') THEN ''
                                 ELSE trim(repo_analised_version)
                             END,
                             etag = ?2,
                             status_processamento = ?3,
                             retry_count = 0
                         WHERE project_name = ?4 OR repo_url = ?5",
                        params![latest, etag, status, repo_key, repo_url],
                    )
                    .map_err(|e| format!("Guardião: falha ao persistir versão no SQLite: {e}"))?;
                
                if updated_rows == 0 {
                    return Err(format!(
                        "Guardião: nenhuma linha em repositorios foi atualizada para project_name='{}' repo_url='{}'",
                        repo_key, repo_url
                    ));
                }
                Ok(())
            })
            .await
            .map_err(|e| format!("Join error: {e}"))?
        })
    }
}

fn xorshift64star(state: u64) -> u64 {
    let mut x = state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    x.wrapping_mul(2685821657736338717u64)
}

fn jitter_ms(state: &AtomicU64, min_ms: u64, max_ms: u64) -> u64 {
    if max_ms <= min_ms {
        return min_ms;
    }
    let cur = state.load(Ordering::Relaxed);
    let next = xorshift64star(cur.wrapping_add(0x9E3779B97F4A7C15));
    state.store(next, Ordering::Relaxed);
    min_ms + (next % (max_ms - min_ms + 1))
}

fn backoff_delay_ms(base_ms: u64, attempt_index_1based: u32) -> u64 {
    let shift = attempt_index_1based.saturating_sub(1).min(16);
    base_ms.saturating_mul(1u64 << shift)
}

impl ReqwestGithubClient {
    fn new() -> Result<Self, String> {
        Self::new_with_policy(RetryPolicy {
            max_attempts: 3,
            jitter_min_ms: 50,
            jitter_max_ms: 150,
            backoff_base_ms: 100,
        })
    }

    fn new_with_policy(policy: RetryPolicy) -> Result<Self, String> {
        let api_base = std::env::var("SODA_GITHUB_API_BASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://api.github.com".to_string());
        let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
        let github_pat = std::env::var("GITHUB_PAT")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "Missing GITHUB_PAT".to_string())?;
        let http = Client::builder()
            .user_agent("f-minus-1-guardian/1.0")
            .build()
            .map_err(|e| format!("Falha ao criar client HTTP: {e}"))?;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xD1B54A32D192ED03);
        Ok(Self {
            http,
            api_base,
            allow_host_override,
            github_pat,
            policy,
            jitter_state: AtomicU64::new(seed),
        })
    }

    async fn get_with_retries_and_etag(&self, endpoint: &str, etag: Option<&str>) -> Result<reqwest::Response, String> {
        for attempt in 1..=self.policy.max_attempts {
            let sleep_ms = jitter_ms(
                &self.jitter_state,
                self.policy.jitter_min_ms,
                self.policy.jitter_max_ms,
            );
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

            let mut req = self
                .http
                .get(endpoint)
                .bearer_auth(&self.github_pat)
                .header("User-Agent", "f-minus-1-guardian/1.0");

            if let Some(e) = etag {
                req = req.header("If-None-Match", e);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| format!("Falha HTTP GitHub: {e}"))?;

            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status == reqwest::StatusCode::FORBIDDEN {
                if attempt < self.policy.max_attempts {
                    let backoff_ms = backoff_delay_ms(self.policy.backoff_base_ms, attempt);
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    continue;
                }
                return Err(format!(
                    "GitHub rate limit (status {}) após {} tentativas",
                    status, attempt
                ));
            }

            return Ok(resp);
        }

        Err("Falha inesperada: loop de tentativas terminou sem retorno".to_string())
    }
}

#[derive(Deserialize)]
struct GithubReleaseResponse {
    tag_name: Option<String>,
}

#[derive(Deserialize)]
struct GithubRepoResponse {
    default_branch: Option<String>,
}

impl GithubClient for ReqwestGithubClient {
    fn latest_release_tag<'a>(
        &'a self,
        repo_url: &'a str,
        etag: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<GithubResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let url = Url::parse(repo_url).map_err(|e| format!("repo_url inválida: {e}"))?;
            if url.host_str() != Some("github.com") && !self.allow_host_override {
                return Ok(GithubResponse::NewRelease { tag: None, etag: None });
            }
            let mut segments = url
                .path_segments()
                .map(|parts| parts.collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter()
                .filter(|segment| !segment.is_empty())
                .map(|segment| segment.trim_end_matches(".git").to_string())
                .collect::<Vec<_>>();
            if segments.len() < 2 {
                return Ok(GithubResponse::NewRelease { tag: None, etag: None });
            }
            let repo = segments.pop().unwrap();
            let owner = segments.pop().unwrap();

            let base = self.api_base.trim_end_matches('/');
            let repo_endpoint = format!("{base}/repos/{owner}/{repo}");
            let release_endpoint = format!("{repo_endpoint}/releases/latest");

            let release_resp = self.get_with_retries_and_etag(&release_endpoint, etag).await?;
            let release_status = release_resp.status();
            if release_status == reqwest::StatusCode::NOT_MODIFIED {
                return Ok(GithubResponse::NotModified);
            }
            if release_status.is_success() {
                let etag_val = release_resp
                    .headers()
                    .get("ETag")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());
                let parsed = release_resp
                    .json::<GithubReleaseResponse>()
                    .await
                    .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
                if let Some(tag) = parsed
                    .tag_name
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                {
                    return Ok(GithubResponse::NewRelease {
                        tag: Some(tag),
                        etag: etag_val,
                    });
                }
            } else if release_status != reqwest::StatusCode::NOT_FOUND {
                return Err(format!("GitHub retornou status {}", release_status));
            }

            let repo_resp = self.get_with_retries_and_etag(&repo_endpoint, None).await?;
            let repo_status = repo_resp.status();
            if repo_status == reqwest::StatusCode::NOT_FOUND {
                return Ok(GithubResponse::NewRelease { tag: None, etag: None });
            }
            if !repo_status.is_success() {
                return Err(format!("GitHub retornou status {}", repo_status));
            }
            let repo_meta = repo_resp
                .json::<GithubRepoResponse>()
                .await
                .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;

            let tags_endpoint = format!("{repo_endpoint}/tags?per_page=1");
            let tags_resp = self.get_with_retries_and_etag(&tags_endpoint, etag).await?;
            let tags_status = tags_resp.status();
            if tags_status == reqwest::StatusCode::NOT_MODIFIED {
                return Ok(GithubResponse::NotModified);
            }
            if tags_status.is_success() {
                let etag_val = tags_resp
                    .headers()
                    .get("ETag")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());
                let tags = tags_resp
                    .json::<Vec<Value>>()
                    .await
                    .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
                if let Some(tag) = tags
                    .first()
                    .and_then(|t| t.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|v| !v.is_empty())
                {
                    return Ok(GithubResponse::NewRelease {
                        tag: Some(tag),
                        etag: etag_val,
                    });
                }
            } else if tags_status != reqwest::StatusCode::NOT_FOUND {
                return Err(format!("GitHub retornou status {}", tags_status));
            }

            if let Some(branch) = repo_meta
                .default_branch
                .as_deref()
                .map(|b| b.trim())
                .filter(|b| !b.is_empty())
            {
                let commit_endpoint = format!("{repo_endpoint}/commits/{branch}");
                let commit_resp = self.get_with_retries_and_etag(&commit_endpoint, etag).await?;
                let commit_status = commit_resp.status();
                if commit_status == reqwest::StatusCode::NOT_MODIFIED {
                    return Ok(GithubResponse::NotModified);
                }
                if !commit_status.is_success() {
                    return Err(format!("GitHub retornou status {}", commit_status));
                }
                let etag_val = commit_resp
                    .headers()
                    .get("ETag")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());
                let commit = commit_resp
                    .json::<Value>()
                    .await
                    .map_err(|e| format!("Falha ao parsear JSON GitHub: {e}"))?;
                let sha = commit
                    .get("sha")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "GitHub: resposta de commit sem sha".to_string())?;
                let short = sha.chars().take(7).collect::<String>();
                return Ok(GithubResponse::NewRelease {
                    tag: Some(short),
                    etag: etag_val,
                });
            }

            Ok(GithubResponse::NewRelease { tag: None, etag: None })
        })
    }
}

struct Guardian<G: GithubClient, R: RepoStore> {
    github: Arc<G>,
    repo_store: Arc<R>,
}

impl<G: GithubClient + 'static, R: RepoStore + 'static> Guardian<G, R> {
    async fn run_once(&self) -> Result<(), String> {
        let pending = self.repo_store.fetch_pending_repos().await?;
        if pending.is_empty() {
            info!("Guardião: nenhum repositório PENDENTE encontrado no SQLite.");
            return Ok(());
        }

        let max_parallel = std::env::var("SODA_GUARDIAN_GITHUB_PARALLEL")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(2)
            .max(1);
        let semaphore = Arc::new(Semaphore::new(max_parallel));
        
        let mut join_set = JoinSet::new();
        
        for ctx in pending {
            let sem = Arc::clone(&semaphore);
            let github = Arc::clone(&self.github);
            join_set.spawn(async move {
                let _permit = sem.acquire_owned().await.unwrap();
                let etag_opt = ctx.etag.as_deref();
                let res = github.latest_release_tag(&ctx.repo_url, etag_opt).await;
                (ctx, res)
            });
        }

        let mut drifted = 0usize;
        let mut updated = 0usize;

        while let Some(out) = join_set.join_next().await {
            match out {
                Ok((ctx, res)) => {
                    match res {
                        Ok(GithubResponse::NotModified) => {
                            let is_new_link = ctx.repo_analised_version.trim().is_empty() 
                                || is_invalid_version_seed(&ctx.repo_analised_version);
                            let status = if is_new_link {
                                "INICIAR_TRIAGEM"
                            } else {
                                "FASE_-1_OK"
                            };
                            info!(repo_url = %ctx.repo_url, status, "Guardião: ETag matched (304 Not Modified)");
                            self.repo_store
                                .persist_release_resolution(
                                    &ctx.project_name,
                                    &ctx.repo_url,
                                    &ctx.ultima_versao_online,
                                    ctx.etag.as_deref(),
                                    status
                                )
                                .await?;
                            updated += 1;
                        }
                        Ok(GithubResponse::NewRelease { tag, etag }) => {
                            let tag_val = tag.unwrap_or_default().trim().to_string();
                            if tag_val.is_empty() {
                                warn!(repo_url = %ctx.repo_url, "Guardião: tag resolvida vazia");
                                continue;
                            }
                            
                            let is_new_link = ctx.repo_analised_version.trim().is_empty() 
                                || is_invalid_version_seed(&ctx.repo_analised_version);
                            
                            let drift = !is_new_link && has_drift(&ctx.repo_analised_version, &tag_val);
                            
                            let status = if is_new_link || drift {
                                "INICIAR_TRIAGEM"
                            } else {
                                "FASE_-1_OK"
                            };

                            if drift {
                                drifted += 1;
                            }

                            info!(repo_url = %ctx.repo_url, latest = %tag_val, status, "Guardião: nova versão resolvida");
                            
                            self.repo_store
                                .persist_release_resolution(
                                    &ctx.project_name,
                                    &ctx.repo_url,
                                    &tag_val,
                                    etag.as_deref(),
                                    status
                                )
                                .await?;
                            updated += 1;
                        }
                        Err(e) => {
                            warn!(
                                repo_url = %ctx.repo_url,
                                error = %e,
                                "Guardião: falha ao consultar GitHub; pulando"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(error = ?e, "Guardião: falha de JoinHandle");
                }
            }
        }

        info!(
            drifted,
            updated,
            "Guardião: rodada concluída"
        );
        Ok(())
    }
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    #[cfg(windows)]
    let _ = enable_ansi_support::enable_ansi_support();
    enable_virtual_terminal();
    let level = parse_log_level_from_env();
    init_cli_tracing(level);

    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let guardian = Guardian {
        github: Arc::new(ReqwestGithubClient::new().map_err(io::Error::other)?),
        repo_store: Arc::new(SqliteRepoStore::new(db_path)),
    };

    guardian.run_once().await.map_err(io::Error::other)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    struct MockGithub {
        tag: Option<String>,
        etag: Option<String>,
        not_modified: bool,
    }

    impl GithubClient for MockGithub {
        fn latest_release_tag<'a>(
            &'a self,
            _repo_url: &'a str,
            _etag: Option<&'a str>,
        ) -> Pin<Box<dyn Future<Output = Result<GithubResponse, String>> + Send + 'a>> {
            let not_modified = self.not_modified;
            let tag = self.tag.clone();
            let etag = self.etag.clone();
            Box::pin(async move {
                if not_modified {
                    Ok(GithubResponse::NotModified)
                } else {
                    Ok(GithubResponse::NewRelease { tag, etag })
                }
            })
        }
    }

    fn setup_test_db(db_path: &Path) -> Connection {
        let conn = Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE repositorios (
                project_name TEXT PRIMARY KEY,
                lote_id TEXT NOT NULL,
                repo_url TEXT NOT NULL UNIQUE,
                repo_analised_version TEXT,
                repo_version TEXT,
                ultima_versao_online TEXT,
                soda_universal_uuid TEXT NOT NULL UNIQUE,
                status_processamento TEXT NOT NULL,
                timestamp_fase_1 INTEGER,
                timestamp_fase_3 INTEGER,
                retry_count INTEGER NOT NULL,
                etag TEXT
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[tokio::test]
    async fn test_sqlite_outbox_transition_new_link_to_iniciar_triagem() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count
            ) VALUES ('acme/widget', 'L1', 'https://github.com/acme/widget', '', '', '', 'UUID-1', 'PENDENTE', 3)",
            [],
        )
        .unwrap();
        drop(conn);

        let store = Arc::new(SqliteRepoStore::new(tmp.path().to_path_buf()));
        let github = Arc::new(MockGithub {
            tag: Some("v1.0.0".to_string()),
            etag: Some("etag-123".to_string()),
            not_modified: false,
        });

        let guardian = Guardian {
            github,
            repo_store: store.clone(),
        };

        guardian.run_once().await.unwrap();

        let conn = Connection::open(tmp.path()).unwrap();
        let (status, ver, online, etag, retry): (String, String, String, Option<String>, i32) = conn
            .query_row(
                "SELECT status_processamento, repo_version, ultima_versao_online, etag, retry_count 
                 FROM repositorios WHERE project_name = 'acme/widget'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();

        assert_eq!(status, "INICIAR_TRIAGEM");
        assert_eq!(ver, "v1.0.0");
        assert_eq!(online, "v1.0.0");
        assert_eq!(etag, Some("etag-123".to_string()));
        assert_eq!(retry, 0); // Law 2: reset retry_count to 0
    }

    #[tokio::test]
    async fn test_sqlite_outbox_transition_drift_to_iniciar_triagem() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count, etag
            ) VALUES ('acme/widget', 'L1', 'https://github.com/acme/widget', 'v1.0.0', 'v1.0.0', 'v1.0.0', 'UUID-1', 'PENDENTE', 2, 'etag-old')",
            [],
        )
        .unwrap();
        drop(conn);

        let store = Arc::new(SqliteRepoStore::new(tmp.path().to_path_buf()));
        let github = Arc::new(MockGithub {
            tag: Some("v2.0.0".to_string()),
            etag: Some("etag-new".to_string()),
            not_modified: false,
        });

        let guardian = Guardian {
            github,
            repo_store: store.clone(),
        };

        guardian.run_once().await.unwrap();

        let conn = Connection::open(tmp.path()).unwrap();
        let (status, ver, online, etag, retry): (String, String, String, Option<String>, i32) = conn
            .query_row(
                "SELECT status_processamento, repo_version, ultima_versao_online, etag, retry_count 
                 FROM repositorios WHERE project_name = 'acme/widget'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();

        assert_eq!(status, "INICIAR_TRIAGEM");
        assert_eq!(ver, "v2.0.0");
        assert_eq!(online, "v2.0.0");
        assert_eq!(etag, Some("etag-new".to_string()));
        assert_eq!(retry, 0);
    }

    #[tokio::test]
    async fn test_sqlite_outbox_no_drift_transition_to_fase_1_ok() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count, etag
            ) VALUES ('acme/widget', 'L1', 'https://github.com/acme/widget', 'v1.0.0', 'v1.0.0', 'v1.0.0', 'UUID-1', 'PENDENTE', 1, 'etag-ok')",
            [],
        )
        .unwrap();
        drop(conn);

        let store = Arc::new(SqliteRepoStore::new(tmp.path().to_path_buf()));
        let github = Arc::new(MockGithub {
            tag: Some("v1.0.0".to_string()),
            etag: Some("etag-ok".to_string()),
            not_modified: true,
        });

        let guardian = Guardian {
            github,
            repo_store: store.clone(),
        };

        guardian.run_once().await.unwrap();

        let conn = Connection::open(tmp.path()).unwrap();
        let (status, retry): (String, i32) = conn
            .query_row(
                "SELECT status_processamento, retry_count 
                 FROM repositorios WHERE project_name = 'acme/widget'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        assert_eq!(status, "FASE_-1_OK");
        assert_eq!(retry, 0);
    }
}
