use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use url::Url;
use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct CommunityMetaPayload {
    pub open_issues_count: u32,
    pub open_prs_count: u32,
    pub last_commit_date: Option<DateTime<Utc>>,
    pub last_release_date: Option<DateTime<Utc>>,
}

impl CommunityMetaPayload {
    pub fn empty() -> Self {
        Self::default()
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
    /// Versão interna que permite injetar a URL base da API para testes.
    async fn fetch_internal(
        repo_url: &Url,
        limiter: &RateLimiter,
        api_base: Option<&str>,
    ) -> Result<CommunityMetaPayload, FetchError> {
        limiter.check().await;
        
        let client_result = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("SODA-Harvester/1.0")
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

        let resp = client.get(&api_url).send().await.map_err(|e| {
            if e.is_timeout() {
                FetchError::Timeout
            } else {
                FetchError::Network(e.to_string())
            }
        })?;

        let status = resp.status();
        if status.is_success() {
            #[derive(Deserialize)]
            struct GithubRepo {
                open_issues_count: u32,
                pushed_at: Option<DateTime<Utc>>,
            }

            let repo = resp
                .json::<GithubRepo>()
                .await
                .map_err(|e| FetchError::InvalidResponse(e.to_string()))?;

            Ok(CommunityMetaPayload {
                open_issues_count: repo.open_issues_count,
                open_prs_count: 0,
                last_commit_date: repo.pushed_at,
                last_release_date: None,
            })
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

    pub async fn fetch(repo_url: &Url, limiter: &RateLimiter) -> Result<CommunityMetaPayload, FetchError> {
        Self::fetch_internal(repo_url, limiter, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_fetch_success() {
        let mut server = Server::new_async().await;
        let mock_body = r#"{"open_issues_count": 42, "pushed_at": "2023-10-27T10:00:00Z"}"#;
        
        let _m = server.mock("GET", "/repos/owner/repo")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_body)
            .create_async().await;

        let url = Url::parse("https://github.com/owner/repo").unwrap();
        let limiter = RateLimiter;
        
        let result = CommunityMetaFetcher::fetch_internal(&url, &limiter, Some(&server.url())).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert_eq!(payload.open_issues_count, 42);
        assert!(payload.last_commit_date.is_some());
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
        // URL válida para o parser mas que falha na conexão
        let url = Url::parse("https://github.com/nonexistent/domain").unwrap();
        let limiter = RateLimiter;
        
        // Usamos fetch normal para garantir que ele tenta conectar e falha
        let result = CommunityMetaFetcher::fetch(&url, &limiter).await;
        assert!(matches!(result, Err(FetchError::Network(_)) | Err(FetchError::Timeout)));
    }
}
