use thiserror::Error;
use serde::{Deserialize, Serialize};

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
    max_tokens: usize,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
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
}

impl CloudCascade {
    pub fn new() -> Result<Self, CascadeError> {
        let api_key = ["OPENROUTER_API_FAST_KEY", "OPENROUTER_API_FREE_KEY", "OPENROUTER_API_HEAVY_KEY"]
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

        Ok(CloudCascade {
            api_key,
            client: reqwest::Client::new(),
            base_url: openrouter_chat_completions_url(),
        })
    }

    #[cfg(test)]
    pub fn with_url(base_url: &str) -> Self {
        CloudCascade {
            api_key: "test_key".to_string(),
            client: reqwest::Client::new(),
            base_url: format!("{}/api/v1/chat/completions", base_url.trim_end_matches('/')),
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

        let result = self
            .call_openrouter(payload, system_prompt, &free_model_name())
            .await;

        match result {
            Ok(essence) => Ok(essence),
            Err(CascadeError::FreeTierUnavailable { status }) if status == 429 || status == 503 => {
                tracing::info!(
                    "CloudCascade: Free tier unavailable ({}), switching to paid",
                    status
                );
                self.call_openrouter(payload, system_prompt, &paid_model_name()).await
            }
            Err(e) => Err(e),
        }
    }

    async fn call_openrouter(
        &self,
        payload: &str,
        system_prompt: &str,
        model: &str,
    ) -> Result<String, CascadeError> {
        let request = OpenRouterRequest {
            model: model.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: payload.to_string(),
                },
            ],
            max_tokens: MAX_OUTPUT_TOKENS,
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| CascadeError::NetworkError(e.to_string()))?;

        let status = response.status();

        if status.as_u16() == 200 {
            let body: OpenRouterResponse = response
                .json()
                .await
                .map_err(|e| CascadeError::NetworkError(e.to_string()))?;

            body.choices
                .first()
                .map(|c| c.message.content.clone())
                .ok_or_else(|| CascadeError::NetworkError("Empty response".to_string()))
        } else if status.as_u16() == 429 || status.as_u16() == 503 {
            Err(CascadeError::FreeTierUnavailable {
                status: status.as_u16(),
            })
        } else {
            Err(CascadeError::PaidFallbackFailed {
                status: status.as_u16(),
                message: format!("HTTP {}", status.as_u16()),
            })
        }
    }
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
    use mockito::{Server, Mock};

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
            .with_status(500)
            .with_body(r#"{"error": "Internal server error"}"#)
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

        let result = cascade
            .cascade_distill("", "Distil this")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CascadeError::InvalidInput));
    }

    #[tokio::test]
    async fn test_whitespace_only_input_returns_error() {
        let server = Server::new_async().await;
        let cascade = CloudCascade::with_url(&server.url());

        let result = cascade
            .cascade_distill("   \n\t  ", "Distil this")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CascadeError::InvalidInput));
    }
}
