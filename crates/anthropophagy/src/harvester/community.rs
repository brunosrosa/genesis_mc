use serde::{Deserialize, Serialize};
use url::Url;
use thiserror::Error;
use std::time::Duration;

use super::github_tracker;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CommunityMetaPayload {
    pub extracted_at: String,
    pub stars_count: u64,
    pub forks_count: u64,
    pub open_issues_count: u32,
    pub open_prs_count: u32,
    pub licenca: String,
    pub description: Option<String>,
    pub full_name: Option<String>,
    pub name: Option<String>,
    pub last_commit_sha: Option<String>,
    pub last_commit_date: Option<String>,
    pub top_open_issues: Vec<CommunityIssueMeta>,
    pub recent_prs: Vec<CommunityPrMeta>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CommunityIssueMeta {
    pub number: u64,
    pub title: String,
    pub labels: Vec<String>,
    pub comments: u64,
    pub reactions: u64,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CommunityPrMeta {
    pub number: u64,
    pub title: String,
    pub status: String,
    pub updated_at: String,
}

impl CommunityMetaPayload {
    pub fn empty() -> Self {
        Self {
            extracted_at: souls_mc_lib::telemetry::now_utc_rfc3339(),
            licenca: "UNKNOWN".to_string(),
            ..Self::default()
        }
    }
}

impl Default for CommunityMetaPayload {
    fn default() -> Self {
        Self {
            extracted_at: souls_mc_lib::telemetry::now_utc_rfc3339(),
            stars_count: 0,
            forks_count: 0,
            open_issues_count: 0,
            open_prs_count: 0,
            licenca: "UNKNOWN".to_string(),
            description: None,
            full_name: None,
            name: None,
            last_commit_sha: None,
            last_commit_date: None,
            top_open_issues: Vec::new(),
            recent_prs: Vec::new(),
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
    /// Versão interna que permite injetar a URL base da API para testes.
    async fn fetch_internal(
        repo_url: &Url,
        limiter: &RateLimiter,
        api_base: Option<&str>,
    ) -> Result<CommunityMetaPayload, FetchError> {
        github_tracker::fetch_community_meta(repo_url, limiter, api_base)
            .await
            .map_err(github_tracker::map_tracker_to_fetch_error)
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
            .with_body(
                r#"{
                    "default_branch":"main",
                    "full_name":"owner/repo",
                    "open_issues_count":42,
                    "stargazers_count":99,
                    "forks_count":13,
                    "license":{"spdx_id":"MIT"},
                    "description":"demo repo",
                    "name":"repo"
                }"#,
            )
            .create_async().await;
        let _open_prs = server.mock("GET", "/search/issues")
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "repo:owner/repo+is:pr+is:open".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"total_count":7,"items":[]}"#)
            .create_async().await;
        let _top_issues = server.mock("GET", "/search/issues")
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "repo:owner/repo+is:issue+is:open+sort:interactions-desc".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "7".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "total_count": 1,
                "items": [
                    {
                        "number": 77,
                        "title": "pain: complex setup",
                        "comments": 31,
                        "reactions": { "total_count": 12 },
                        "labels": [{ "name": "bug" }, { "name": "build" }],
                        "updated_at": "2023-10-20T10:00:00Z"
                    }
                ]
            }"#)
            .create_async().await;
        let _recent_prs = server.mock("GET", "/repos/owner/repo/pulls")
            .match_query(mockito::Matcher::UrlEncoded("state".into(), "all".into()))
            .match_query(mockito::Matcher::UrlEncoded("sort".into(), "updated".into()))
            .match_query(mockito::Matcher::UrlEncoded("direction".into(), "desc".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "5".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"number":101,"title":"refactor: cleanup","state":"open","updated_at":"2023-10-28T10:00:00Z","merged_at":null,"draft":false}]"#)
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
        assert_eq!(payload.stars_count, 99);
        assert_eq!(payload.forks_count, 13);
        assert_eq!(payload.licenca, "MIT");
        assert_eq!(payload.description.as_deref(), Some("demo repo"));
        assert_eq!(payload.name.as_deref(), Some("repo"));
        assert_eq!(payload.last_commit_sha.as_deref(), Some("abc123"));
        assert!(payload.last_commit_date.is_some());
        assert_eq!(payload.top_open_issues.len(), 1);
        assert_eq!(payload.top_open_issues[0].number, 77);
        assert!(payload.top_open_issues[0].labels.contains(&"bug".to_string()));
        assert_eq!(payload.recent_prs.len(), 1);
        assert_eq!(payload.recent_prs[0].number, 101);
        assert_eq!(payload.recent_prs[0].status, "open");
    }

    #[tokio::test]
    async fn test_fetch_uses_canonical_full_name_for_search_routes() {
        let mut server = Server::new_async().await;
        let _repo = server.mock("GET", "/repos/owner/repo")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"default_branch":"main","full_name":"firecrawl/firecrawl"}"#)
            .create_async().await;
        let _open_prs = server.mock("GET", "/search/issues")
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "repo:firecrawl/firecrawl+is:pr+is:open".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"total_count":5,"items":[]}"#)
            .create_async().await;
        let _top_issues = server.mock("GET", "/search/issues")
            .match_query(mockito::Matcher::UrlEncoded("q".into(), "repo:firecrawl/firecrawl+is:issue+is:open+sort:interactions-desc".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "7".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"total_count":5,"items":[]}"#)
            .create_async().await;
        let _commits = server.mock("GET", "/repos/owner/repo/commits")
            .match_query(mockito::Matcher::UrlEncoded("sha".into(), "main".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"sha":"abc123","commit":{"author":{"date":"2023-10-27T10:00:00Z"}}}]"#)
            .create_async().await;
        let _pulls = server.mock("GET", "/repos/firecrawl/firecrawl/pulls")
            .match_query(mockito::Matcher::UrlEncoded("state".into(), "all".into()))
            .match_query(mockito::Matcher::UrlEncoded("sort".into(), "updated".into()))
            .match_query(mockito::Matcher::UrlEncoded("direction".into(), "desc".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "5".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async().await;
        let _canonical_commits = server.mock("GET", "/repos/firecrawl/firecrawl/commits")
            .match_query(mockito::Matcher::UrlEncoded("sha".into(), "main".into()))
            .match_query(mockito::Matcher::UrlEncoded("per_page".into(), "1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"sha":"def456","commit":{"author":{"date":"2023-10-29T10:00:00Z"}}}]"#)
            .create_async().await;

        let url = Url::parse("https://github.com/owner/repo").unwrap();
        let limiter = RateLimiter;

        let result = CommunityMetaFetcher::fetch_internal(&url, &limiter, Some(&server.url())).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert_eq!(payload.open_prs_count, 5);
        assert_eq!(payload.full_name.as_deref(), Some("firecrawl/firecrawl"));
    }

    #[test]
    fn test_map_tracker_not_found_to_fetch_error() {
        let result = github_tracker::map_tracker_to_fetch_error(github_tracker::GithubTrackerError::NotFound);
        assert_eq!(result, FetchError::NotFound);
    }

    #[test]
    fn test_map_tracker_rate_limit_to_fetch_error() {
        let result = github_tracker::map_tracker_to_fetch_error(github_tracker::GithubTrackerError::RateLimit);
        assert_eq!(result, FetchError::RateLimit);
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
