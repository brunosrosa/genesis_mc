use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use url::Url;
use thiserror::Error;
use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct CommunityMetaPayload {
    pub open_issues_count: u32,
    pub open_prs_count: u32,
    pub licenca: String,
    pub last_commit_sha: Option<String>,
    pub last_commit_date: Option<DateTime<Utc>>,
    pub recent_prs: Vec<CommunityPrMeta>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CommunityPrMeta {
    pub number: u64,
    pub state: String,
    pub updated_at: DateTime<Utc>,
}

impl CommunityMetaPayload {
    pub fn empty() -> Self {
        Self {
            licenca: "UNKNOWN".to_string(),
            ..Self::default()
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("API Limit exceeded")]
    RateLimit,
    #[error("Not Found")]
    NotFound,
    #[error("Timeout")]
    Timeout,
    #[error("Unsupported repository source: {0}")]
    UnsupportedSource(String),
    #[error("Invalid response payload: {0}")]
    InvalidResponse(String),
}

pub struct RateLimiter;

impl RateLimiter {
    pub async fn check(&self) {
        // Simples delay ou check de token bucket no futuro
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub struct CommunityMetaFetcher;

impl CommunityMetaFetcher {
    fn github_auth_header_value() -> Option<HeaderValue> {
        let token = std::env::var("GITHUB_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::env::var("GITHUB_PAT")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })?;
        HeaderValue::from_str(&format!("Bearer {}", token)).ok()
    }

    async fn fetch_json<T: for<'de> Deserialize<'de>>(
        client: &reqwest::Client,
        limiter: &RateLimiter,
        url: &str,
    ) -> Result<T, FetchError> {
        limiter.check().await;
        let resp = client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                FetchError::Timeout
            } else {
                FetchError::Network(e.to_string())
            }
        })?;

        let status = resp.status();
        if status.is_success() {
            resp.json::<T>()
                .await
                .map_err(|e| FetchError::InvalidResponse(e.to_string()))
        } else if status.as_u16() == 404 {
            Err(FetchError::NotFound)
        } else if status.as_u16() == 403 {
            Err(FetchError::RateLimit)
        } else {
            Err(FetchError::Network(format!(
                "GitHub API retornou HTTP {}",
                status
            )))
        }
    }

    /// Versão interna que permite injetar a URL base da API para testes.
    async fn fetch_internal(
        repo_url: &Url,
        limiter: &RateLimiter,
        api_base: Option<&str>,
    ) -> Result<CommunityMetaPayload, FetchError> {
        let mut headers = HeaderMap::new();
        if let Some(value) = Self::github_auth_header_value() {
            headers.insert(AUTHORIZATION, value);
        }
        let client_result = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("SODA-Harvester/1.0")
            .default_headers(headers)
            .build();
        let client = client_result.map_err(|e| FetchError::Network(e.to_string()))?;

        // Mapeamento de URL GitHub para API
        let api_url = if repo_url.host_str() == Some("github.com") {
            let path = repo_url.path().trim_start_matches('/');
            let base = api_base.unwrap_or("https://api.github.com");
            format!("{}/repos/{}", base, path)
        } else {
            return Err(FetchError::UnsupportedSource(repo_url.to_string()));
        };

        #[derive(Deserialize)]
        struct GithubRepo {
            default_branch: String,
            #[serde(default)]
            license: Option<GithubLicense>,
        }

        #[derive(Deserialize)]
        struct GithubLicense {
            spdx_id: Option<String>,
            name: Option<String>,
        }

        #[derive(Deserialize)]
        struct SearchIssueResponse {
            total_count: u32,
            #[serde(default)]
            items: Vec<GithubPullRequest>,
        }

        #[derive(Deserialize)]
        struct GithubCommit {
            sha: String,
            commit: GithubCommitInner,
        }

        #[derive(Deserialize)]
        struct GithubCommitInner {
            author: GithubCommitAuthor,
        }

        #[derive(Deserialize)]
        struct GithubCommitAuthor {
            date: DateTime<Utc>,
        }

        #[derive(Deserialize)]
        struct GithubPullRequest {
            number: u64,
            state: String,
            updated_at: DateTime<Utc>,
        }

        let repo = Self::fetch_json::<GithubRepo>(&client, limiter, &api_url).await?;
        let licenca = repo
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
        let issues_url = format!(
            "{}/search/issues?q=repo:{}+is:issue+is:open&per_page=1",
            api_base.unwrap_or("https://api.github.com"),
            repo_url.path().trim_start_matches('/')
        );
        let open_prs_url = format!(
            "{}/search/issues?q=repo:{}+is:pr+is:open&per_page=1",
            api_base.unwrap_or("https://api.github.com"),
            repo_url.path().trim_start_matches('/')
        );
        let recent_prs_url = format!(
            "{}/search/issues?q=repo:{}+is:pr&sort=updated&order=desc&per_page=5",
            api_base.unwrap_or("https://api.github.com"),
            repo_url.path().trim_start_matches('/')
        );
        let commits_url = format!(
            "{}/repos/{}/commits?sha={}&per_page=1",
            api_base.unwrap_or("https://api.github.com"),
            repo_url.path().trim_start_matches('/'),
            repo.default_branch
        );

        let open_issues = Self::fetch_json::<SearchIssueResponse>(&client, limiter, &issues_url).await?;
        let open_prs = Self::fetch_json::<SearchIssueResponse>(&client, limiter, &open_prs_url).await?;
        let recent_prs = Self::fetch_json::<SearchIssueResponse>(&client, limiter, &recent_prs_url).await?;
        let commits = Self::fetch_json::<Vec<GithubCommit>>(&client, limiter, &commits_url).await?;
        let last_commit = commits.into_iter().next();

        Ok(CommunityMetaPayload {
            open_issues_count: open_issues.total_count,
            open_prs_count: open_prs.total_count,
            licenca,
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

    pub async fn fetch(repo_url: &Url, limiter: &RateLimiter) -> Result<CommunityMetaPayload, FetchError> {
        Self::fetch_internal(repo_url, limiter, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::net::TcpListener;

    #[tokio::test]
    async fn test_fetch_success() {
        let mut server = Server::new_async().await;
        let _repo = server.mock("GET", "/repos/owner/repo")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"default_branch":"main"}"#)
            .create_async().await;
        let _issues = server.mock("GET", "/search/issues")
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "repo:owner/repo+is:issue+is:open".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"total_count":42,"items":[]}"#)
            .create_async().await;
        let _open_prs = server.mock("GET", "/search/issues")
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "repo:owner/repo+is:pr+is:open".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"total_count":7,"items":[]}"#)
            .create_async().await;
        let _recent_prs = server.mock("GET", "/search/issues")
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "repo:owner/repo+is:pr".into()))
            .match_query(mockito::Matcher::UrlEncoded("sort".into(), "updated".into()))
            .match_query(mockito::Matcher::UrlEncoded("order".into(), "desc".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "5".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"total_count":7,"items":[{"number":101,"state":"open","updated_at":"2023-10-28T10:00:00Z"}]}"#)
            .create_async().await;
        let _commits = server.mock("GET", "/repos/owner/repo/commits")
            .match_query(mockito::Matcher::UrlEncoded("sha".into(), "main".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"sha":"abc123","commit":{"author":{"date":"2023-10-27T10:00:00Z"}}}]"#)
            .create_async().await;

        let url = Url::parse("https://github.com/owner/repo").unwrap();
        let limiter = RateLimiter;
        
        let result = CommunityMetaFetcher::fetch_internal(&url, &limiter, Some(&server.url())).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert_eq!(payload.open_issues_count, 42);
        assert_eq!(payload.open_prs_count, 7);
        assert_eq!(payload.last_commit_sha.as_deref(), Some("abc123"));
        assert!(payload.last_commit_date.is_some());
        assert_eq!(payload.recent_prs.len(), 1);
        assert_eq!(payload.recent_prs[0].number, 101);
    }

    #[tokio::test]
    async fn test_fetch_404_fails_closed() {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/repos/owner/repo")
            .with_status(404)
            .create_async().await;

        let url = Url::parse("https://github.com/owner/repo").unwrap();
        let limiter = RateLimiter;
        
        let result = CommunityMetaFetcher::fetch_internal(&url, &limiter, Some(&server.url())).await;
        assert_eq!(result, Err(FetchError::NotFound));
    }

    #[tokio::test]
    async fn test_fetch_rate_limit_fails_closed() {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/repos/owner/repo")
            .with_status(403)
            .create_async().await;

        let url = Url::parse("https://github.com/owner/repo").unwrap();
        let limiter = RateLimiter;
        
        let result = CommunityMetaFetcher::fetch_internal(&url, &limiter, Some(&server.url())).await;
        assert_eq!(result, Err(FetchError::RateLimit));
    }

    #[tokio::test]
    async fn test_fetch_connection_error_fails_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let api_base = format!("http://{}", addr);
        let url = Url::parse("https://github.com/owner/repo").unwrap();
        let limiter = RateLimiter;

        let result = CommunityMetaFetcher::fetch_internal(&url, &limiter, Some(&api_base)).await;
        assert!(matches!(result, Err(FetchError::Network(_)) | Err(FetchError::Timeout)));
    }
}
