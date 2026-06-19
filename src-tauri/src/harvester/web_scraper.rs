use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use html_to_markdown_rs::convert;
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use serde::Deserialize;
use thiserror::Error;
use url::Url;

const ETHICAL_BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 SODA-Harvester/1.0";
const MIN_USEFUL_MARKDOWN_CHARS: usize = 200;
const DOMAIN_JITTER_WINDOW: Duration = Duration::from_secs(12);
const FIRECRAWL_TIMEOUT_MS: u64 = 30_000;

static DOMAIN_REQUEST_TIMES: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScrapeDiagnostics {
    pub route: &'static str,
    pub markdown: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WebScraperError {
    #[error("URL inválida para scraping: {url}")]
    InvalidUrl { url: String },

    #[error("Falha ao montar cliente HTTP: {reason}")]
    ClientBuild { reason: String },

    #[error("HTTP {status} ao buscar {url}")]
    HttpStatus { url: String, status: u16 },

    #[error("Falha de rede ao buscar {url}: {reason}")]
    Network { url: String, reason: String },

    #[error("Falha ao ler corpo HTTP de {url}: {reason}")]
    ReadBody { url: String, reason: String },

    #[error("Falha ao converter HTML em markdown para {url}: {reason}")]
    HtmlToMarkdown { url: String, reason: String },

    #[error("Conteúdo insuficiente em {url} pela rota {route}: {reason}")]
    SuspiciousContent {
        url: String,
        route: &'static str,
        reason: String,
    },

    #[error("Falha no Firecrawl para {url}: {reason}")]
    Firecrawl { url: String, reason: String },

    #[error("Tentativa dupla falhou em {url}. rota_a={route_a}; rota_b={route_b}")]
    GuaranteedFetchFailed {
        url: String,
        route_a: String,
        route_b: String,
    },
}

#[derive(Debug, Deserialize)]
struct FirecrawlResponse {
    success: bool,
    #[serde(default)]
    data: Option<FirecrawlData>,
    #[serde(default)]
    warning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FirecrawlData {
    #[serde(default)]
    markdown: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    html: Option<String>,
}

pub async fn fetch_markdown_with_guarantee(url: &str) -> Result<String, WebScraperError> {
    Ok(fetch_markdown_with_diagnostics(url).await?.markdown)
}

pub(crate) async fn fetch_markdown_with_diagnostics(
    url: &str,
) -> Result<ScrapeDiagnostics, WebScraperError> {
    let parsed_url = Url::parse(url).map_err(|_| WebScraperError::InvalidUrl {
        url: url.to_string(),
    })?;
    apply_domain_jitter(&parsed_url).await;

    match fetch_via_reqwest(&parsed_url).await {
        Ok(markdown) => Ok(ScrapeDiagnostics {
            route: "reqwest",
            markdown,
        }),
        Err(route_a_error) => {
            let route_b_error = match try_firecrawl_fallback(&parsed_url).await {
                Ok(diagnostics) => return Ok(diagnostics),
                Err(err) => err,
            };
            Err(WebScraperError::GuaranteedFetchFailed {
                url: parsed_url.to_string(),
                route_a: route_a_error.to_string(),
                route_b: route_b_error.to_string(),
            })
        }
    }
}

fn domain_request_times() -> &'static Mutex<HashMap<String, Instant>> {
    DOMAIN_REQUEST_TIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn apply_domain_jitter(url: &Url) {
    let Some(domain) = url.domain().map(|value| value.to_ascii_lowercase()) else {
        return;
    };

    let should_jitter = {
        let now = Instant::now();
        let mut guard = match domain_request_times().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let should_jitter = guard
            .get(&domain)
            .map(|last_seen| now.saturating_duration_since(*last_seen) <= DOMAIN_JITTER_WINDOW)
            .unwrap_or(false);
        guard.insert(domain, now);
        should_jitter
    };

    if should_jitter {
        let jitter_secs = rand::thread_rng().gen_range(2..=5);
        tokio::time::sleep(Duration::from_secs(jitter_secs)).await;
    }
}

async fn fetch_via_reqwest(url: &Url) -> Result<String, WebScraperError> {
    let client = build_client()?;
    let response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|err| WebScraperError::Network {
            url: url.to_string(),
            reason: err.to_string(),
        })?;

    let status = response.status();
    if status.as_u16() == 403 {
        return Err(WebScraperError::HttpStatus {
            url: url.to_string(),
            status: 403,
        });
    }
    if !status.is_success() {
        return Err(WebScraperError::HttpStatus {
            url: url.to_string(),
            status: status.as_u16(),
        });
    }

    let html = response
        .text()
        .await
        .map_err(|err| WebScraperError::ReadBody {
            url: url.to_string(),
            reason: err.to_string(),
        })?;
    let markdown = html_to_markdown(url, &html)?;
    ensure_useful_markdown(url, "reqwest", markdown)
}

async fn try_firecrawl_fallback(url: &Url) -> Result<ScrapeDiagnostics, WebScraperError> {
    let api_key = std::env::var("FIRECRAWL_API_KEY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| WebScraperError::Firecrawl {
            url: url.to_string(),
            reason: "FIRECRAWL_API_KEY ausente; rota de garantia indisponível".to_string(),
        })?;

    fetch_via_firecrawl(url, &api_key)
        .await
        .map(|markdown| ScrapeDiagnostics {
            route: "firecrawl",
            markdown,
        })
}

fn build_client() -> Result<reqwest::Client, WebScraperError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(USER_AGENT, HeaderValue::from_static(ETHICAL_BROWSER_USER_AGENT));

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|err| WebScraperError::ClientBuild {
            reason: err.to_string(),
        })
}

async fn fetch_via_firecrawl(url: &Url, api_key: &str) -> Result<String, WebScraperError> {
    let client = build_client()?;
    let endpoint = std::env::var("FIRECRAWL_API_BASE_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.firecrawl.dev/v0/scrape".to_string());
    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "url": url.as_str(),
            "pageOptions": {
                "onlyMainContent": true,
                "includeHtml": false,
                "includeRawHtml": false,
                "waitFor": 1500
            },
            "timeout": FIRECRAWL_TIMEOUT_MS
        }))
        .send()
        .await
        .map_err(|err| WebScraperError::Firecrawl {
            url: url.to_string(),
            reason: err.to_string(),
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(WebScraperError::Firecrawl {
            url: url.to_string(),
            reason: format!("HTTP {}", status.as_u16()),
        });
    }

    let payload = response
        .json::<FirecrawlResponse>()
        .await
        .map_err(|err| WebScraperError::Firecrawl {
            url: url.to_string(),
            reason: format!("payload inválido: {}", err),
        })?;

    if !payload.success {
        return Err(WebScraperError::Firecrawl {
            url: url.to_string(),
            reason: payload
                .warning
                .unwrap_or_else(|| "Firecrawl retornou success=false".to_string()),
        });
    }

    let data = payload.data.ok_or_else(|| WebScraperError::Firecrawl {
        url: url.to_string(),
        reason: "resposta sem campo data".to_string(),
    })?;

    let markdown = data
        .markdown
        .map(|value| normalize_markdown(&value))
        .filter(|value| !value.is_empty())
        .or_else(|| {
            data.content
                .map(|value| normalize_markdown(&value))
                .filter(|value| !value.is_empty())
        })
        .or_else(|| {
            data.html
                .as_deref()
                .and_then(|html| html_to_markdown(url, html).ok())
                .filter(|value| !value.is_empty())
        })
        .ok_or_else(|| WebScraperError::Firecrawl {
            url: url.to_string(),
            reason: "resposta sem markdown utilizável".to_string(),
        })?;

    ensure_useful_markdown(url, "firecrawl", markdown)
}

fn html_to_markdown(url: &Url, html: &str) -> Result<String, WebScraperError> {
    convert(html, None)
        .map(|value| normalize_markdown(&value))
        .map_err(|err| WebScraperError::HtmlToMarkdown {
            url: url.to_string(),
            reason: err.to_string(),
        })
}

fn ensure_useful_markdown(
    url: &Url,
    route: &'static str,
    markdown: String,
) -> Result<String, WebScraperError> {
    let normalized = normalize_markdown(&markdown);
    if is_suspicious_content(&normalized) {
        return Err(WebScraperError::SuspiciousContent {
            url: url.to_string(),
            route,
            reason: format!(
                "conteúdo limpo abaixo do limiar ou com assinatura de bloqueio/SPA (chars={})",
                normalized.chars().count()
            ),
        });
    }
    Ok(normalized)
}

fn normalize_markdown(input: &str) -> String {
    input
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}

fn is_suspicious_content(markdown: &str) -> bool {
    let trimmed = markdown.trim();
    if trimmed.chars().count() < MIN_USEFUL_MARKDOWN_CHARS {
        return true;
    }

    let lower = trimmed.to_ascii_lowercase();
    [
        "enable javascript",
        "javascript is required",
        "please turn javascript on",
        "checking your browser",
        "attention required",
        "cloudflare",
        "verify you are human",
        "access denied",
        "just a moment",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
