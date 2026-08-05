use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Semaphore;

#[derive(Error, Debug, Clone)]
pub enum CascadeError {
    #[error("Payload invalido ou vazio")]
    InvalidInput,
    #[error("Modelo gratuito indisponivel (HTTP {status})")]
    FreeTierUnavailable { status: u16 },
    #[error("Modelo pago falhou (HTTP {status}): {message}")]
    PaidFallbackFailed { status: u16, message: String },
    #[error("Timeout na requisicao: {0}")]
    RequestTimeout(String),
    #[error("Erro de rede: {0}")]
    NetworkError(String),
}

const MAX_OUTPUT_TOKENS: usize = 3_000;

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_reasoning: Option<bool>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: MessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Serialize)]
struct ContentPart {
    #[serde(rename = "type")]
    kind: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<CacheControl>,
}

#[derive(Debug, Serialize)]
struct CacheControl {
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

pub struct CloudCascade {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    semaphore: Arc<Semaphore>,
    canon_context: Arc<String>,
}

impl CloudCascade {
    pub fn new() -> Result<Self, CascadeError> {
        let api_key = [
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
            "OPENROUTER_API_HEAVY_KEY",
        ]
        .into_iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|value| value.trim().trim_matches('"').to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CascadeError::NetworkError(
                "OPENROUTER_API_FAST_KEY/OPENROUTER_API_FREE_KEY/OPENROUTER_API_HEAVY_KEY not set"
                    .to_string(),
            )
        })?;

        let canon_context = Arc::new(load_souls_canon_manifest());

        Ok(CloudCascade {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .map_err(|e| CascadeError::NetworkError(e.to_string()))?,
            base_url: openrouter_chat_completions_url(),
            semaphore: Arc::new(Semaphore::new(2)),
            canon_context,
        })
    }

    #[cfg(test)]
    pub fn with_url(base_url: &str) -> Self {
        CloudCascade {
            api_key: "test_key".to_string(),
            client: reqwest::Client::new(),
            base_url: format!("{}/api/v1/chat/completions", base_url.trim_end_matches('/')),
            semaphore: Arc::new(Semaphore::new(2)),
            canon_context: Arc::new(load_souls_canon_manifest()),
        }
    }

    pub async fn cascade_distill(
        &self,
        payload: &str,
        system_prompt: &str,
    ) -> Result<String, CascadeError> {
        if payload.trim().is_empty() {
            return Err(CascadeError::InvalidInput);
        }

        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| CascadeError::NetworkError(format!("Semaphore error: {}", e)))?;

        let result = self
            .call_openrouter(payload, system_prompt, &free_model_name())
            .await;

        match result {
            Ok(essence) => Ok(essence),
            Err(CascadeError::FreeTierUnavailable { .. }) => {
                self.call_openrouter(payload, system_prompt, &paid_model_name())
                    .await
            }
            Err(err) => Err(err),
        }
    }

    async fn call_openrouter(
        &self,
        payload: &str,
        system_prompt: &str,
        model: &str,
    ) -> Result<String, CascadeError> {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: MessageContent::Parts(vec![ContentPart {
                    kind: "text".to_string(),
                    text: format!("=== SOULS CANON CONTEXT ===\n{}", self.canon_context),
                    cache_control: Some(CacheControl {
                        kind: "ephemeral".to_string(),
                        ttl: None,
                    }),
                }]),
            },
            Message {
                role: "user".to_string(),
                content: MessageContent::Text(format!(
                    "{}\n\n=== CONTEÚDO DO ARTEFATO ===\n{}",
                    system_prompt, payload
                )),
            },
        ];

        let req_body = OpenRouterRequest {
            model: model.to_string(),
            messages,
            temperature: 0.0,
            max_tokens: MAX_OUTPUT_TOKENS,
            include_reasoning: Some(false),
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req_body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    CascadeError::RequestTimeout(e.to_string())
                } else {
                    CascadeError::NetworkError(e.to_string())
                }
            })?;

        let status = response.status();
        if status.is_success() {
            let body: OpenRouterResponse =
                response
                    .json()
                    .await
                    .map_err(|e| CascadeError::PaidFallbackFailed {
                        status: status.as_u16(),
                        message: format!("Failed to parse JSON response: {}", e),
                    })?;

            let choice = body.choices.into_iter().next().ok_or_else(|| {
                CascadeError::PaidFallbackFailed {
                    status: status.as_u16(),
                    message: "Response contained no choices".to_string(),
                }
            })?;

            Ok(choice.message.content)
        } else if status.as_u16() == 429 || status.is_server_error() {
            Err(CascadeError::FreeTierUnavailable {
                status: status.as_u16(),
            })
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(CascadeError::PaidFallbackFailed {
                status: status.as_u16(),
                message: format!("HTTP {}: {}", status.as_u16(), error_text),
            })
        }
    }
}

fn load_souls_canon_manifest() -> String {
    let paths = [
        "Z:\\souls_mc\\docs\\SOULS_CANON_MANIFEST.md",
        "docs/SOULS_CANON_MANIFEST.md",
        "../docs/SOULS_CANON_MANIFEST.md",
    ];

    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if !content.trim().is_empty() {
                return content;
            }
        }
    }

    "SOULS Canon Manifest unavailable in filesystem.".to_string()
}

fn openrouter_chat_completions_url() -> String {
    let base = std::env::var("OPENAI_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
    format!("{base}/chat/completions")
}

fn free_model_name() -> String {
    std::env::var("OPENROUTER_FREE_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "openrouter/free".to_string())
}

fn paid_model_name() -> String {
    std::env::var("OPENROUTER_DEFAULT_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "deepseek/deepseek-v4-flash".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Mock, Server};

    fn create_openrouter_success_mock(server: &mut Server) -> Mock {
        let response_body = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Distilled essence: test summary"
                }
            }]
        });

        server
            .mock("POST", "/api/v1/chat/completions")
            .with_status(200)
            .with_body(serde_json::to_string(&response_body).unwrap())
            .create()
    }

    fn create_rate_limit_mock(server: &mut Server) -> Mock {
        server
            .mock("POST", "/api/v1/chat/completions")
            .with_status(429)
            .with_body(r#"{"error": "Rate limit exceeded"}"#)
            .create()
    }

    fn create_server_error_mock(server: &mut Server) -> Mock {
        server
            .mock("POST", "/api/v1/chat/completions")
            .with_status(400)
            .with_body(r#"{"error": "Bad request"}"#)
            .create()
    }

    #[tokio::test]
    async fn test_free_tier_success() {
        let mut server = Server::new_async().await;
        let m = create_openrouter_success_mock(&mut server);

        let cascade = CloudCascade::with_url(&server.url());

        let result = cascade
            .cascade_distill("word ".repeat(100).as_str(), "Distil this")
            .await;

        m.assert();
        if result.is_err() {
            eprintln!("ERROR: {:?}", result.as_ref().err());
        }
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Distilled essence"));
    }

    #[tokio::test]
    async fn test_free_tier_rate_limit_then_paid_fallback() {
        let mut server = Server::new_async().await;

        let free_mock = create_rate_limit_mock(&mut server);
        let paid_mock = create_openrouter_success_mock(&mut server);

        let cascade = CloudCascade::with_url(&server.url());

        let result = cascade
            .cascade_distill("word ".repeat(100).as_str(), "Distil this")
            .await;

        free_mock.assert();
        paid_mock.assert();
        if result.is_err() {
            eprintln!("ERROR: {:?}", result.as_ref().err());
        }
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Distilled essence"));
    }

    #[tokio::test]
    async fn test_paid_tier_also_fails() {
        let mut server = Server::new_async().await;

        let free_mock = create_rate_limit_mock(&mut server);
        let paid_mock = create_server_error_mock(&mut server);

        let cascade = CloudCascade::with_url(&server.url());

        let result = cascade
            .cascade_distill("word ".repeat(100).as_str(), "Distil this")
            .await;

        free_mock.assert();
        paid_mock.assert();
        if result.is_err() {
            eprintln!("ERROR: {:?}", result.as_ref().err());
        }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CascadeError::PaidFallbackFailed { .. }));
    }

    #[tokio::test]
    async fn test_invalid_input_returns_error() {
        let server = Server::new_async().await;
        let cascade = CloudCascade::with_url(&server.url());

        let result = cascade.cascade_distill("", "Distil this").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CascadeError::InvalidInput));
    }

    #[tokio::test]
    async fn test_whitespace_only_input_returns_error() {
        let server = Server::new_async().await;
        let cascade = CloudCascade::with_url(&server.url());

        let result = cascade.cascade_distill("   \n\t  ", "Distil this").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CascadeError::InvalidInput));
    }
}
