use crate::finops::phase1_5::package_assembler::Phase2Payloads;
use reqwest::{Client, StatusCode};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

const STATUS_OK: &str = "F2_OK";
const STATUS_ERR: &str = "ERRO_F2";
const DEFAULT_OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const BLOB_10_CANON_MARKER: &str = "=== BLOB_10_CANON_CONTEXT ===";

type LensFuture<'a> = Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

#[cfg(not(test))]
const LENS_TIMEOUT: Duration = Duration::from_secs(180);
#[cfg(test)]
const LENS_TIMEOUT: Duration = Duration::from_millis(220);

#[cfg(not(test))]
const OPENROUTER_HTTP_TIMEOUT: Duration = Duration::from_secs(120);
#[cfg(test)]
const OPENROUTER_HTTP_TIMEOUT: Duration = Duration::from_secs(3);

const LENS_MAX_TOKENS: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwarmDebate {
    pub repo_id: String,
    pub lente_a: String,
    pub lente_b: String,
    pub lente_c: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LensKind {
    ProductUx,
    Architecture,
    Operations,
}

impl LensKind {
    fn lens_id(self) -> &'static str {
        match self {
            Self::ProductUx => "LensA_ProductUX",
            Self::Architecture => "LensB_Architecture",
            Self::Operations => "LensC_Operations",
        }
    }

    fn model_env_key(self) -> &'static str {
        match self {
            Self::ProductUx => "OPENROUTER_HEAVY_MODEL_LENS_PROD_UX",
            Self::Architecture => "OPENROUTER_HEAVY_MODEL_LENS_ARQ",
            Self::Operations => "OPENROUTER_HEAVY_MODEL_LENS_OPS",
        }
    }

    fn system_prompt(self) -> &'static str {
        match self {
            Self::ProductUx => "RESPONDA OBRIGATORIAMENTE EM PORTUGUES (PT-BR) NO FORMATO JSON PURO, SEM MARGINALIZAÇÃO OU BLOCOS DE CÓDIGO (COMO ```json). Atue como LensA_ProductUX. Analise a inovacao e o valor real de produto. Qual e o 'UAU moment', o 'refreshness' da UX ou o fluxo de trabalho genial que amplia as capacidades do usuario? Avalie estrategicamente como essa solucao se encaixa ou cria novos Canvas no SODA. Valide se essa entrega respeita nossas leis de neuro-inclusao (mitigacao de Flow-Debt, Zero Layout Shift). Se o payload contiver repo_kind=SkillLibrary ou repo_kind=ContentRepo, trate como curadoria de conhecimento: avalie clareza, utilidade operacional, reusabilidade como skill e se ha exemplos determinísticos (formatos de saída bem definidos) para virar ferramenta no SODA. Responda estritamente em JSON com as chaves: lens_id, repo_id, model_used, bullets. E TERMINANTEMENTE PROIBIDO deixar o array 'bullets' vazio. Divida o seu raciocinio em 3 a 5 pontos factuais dentro do array 'bullets'.\n\nExemplo de formato de saída esperado:\n{\n  \"lens_id\": \"LensA_ProductUX\",\n  \"repo_id\": \"owner/repo\",\n  \"model_used\": \"nome-do-modelo\",\n  \"bullets\": [\n    \"A interface reduz a carga cognitiva ao unificar as ações em um único hub.\",\n    \"Presença de um fluxo de feedback tátil e sonoro inovador para ações críticas.\",\n    \"Otimizado para evitar Layout Shift durante o carregamento assíncrono de dados.\"\n  ]\n}",
            Self::Architecture => "RESPONDA OBRIGATORIAMENTE EM PORTUGUES (PT-BR) NO FORMATO JSON PURO, SEM MARGINALIZAÇÃO OU BLOCOS DE CÓDIGO (COMO ```json). Atue como LensB_Architecture. Isole a 'alma matematica' e o nucleo transplantavel. A logica e transmutavel e agnostica (recompilavel dinamicamente via CubeCL/Burn)? Avalie a viabilidade do codigo sobreviver ao nosso 'Treino de Gravidade' (limite da RTX 2060m) sem depender de interpretadores presos a arquitetura (Node.js/JVM). Se o payload contiver repo_kind=SkillLibrary ou repo_kind=ContentRepo, avalie extraibilidade como biblioteca de habilidades: taxonomia, consistencia de formato, existencia de contratos de I/O (inputs/outputs), facilidade de normalizar para JSON/gramatica e o custo de manutencao do catalogo. Responda estritamente em JSON com as chaves: lens_id, repo_id, model_used, bullets. E TERMINANTEMENTE PROIBIDO deixar o array 'bullets' vazio. Divida o seu raciocinio em 3 a 5 pontos factuais dentro do array 'bullets'.\n\nExemplo de formato de saída esperado:\n{\n  \"lens_id\": \"LensB_Architecture\",\n  \"repo_id\": \"owner/repo\",\n  \"model_used\": \"nome-do-modelo\",\n  \"bullets\": [\n    \"Código estruturado puramente em Rust com tokio, facilitando compilação zero-dependency.\",\n    \"Gargalo de I/O isolado em threads dedicadas fora do event loop assíncrono.\",\n    \"Ausência de interpretadores pesados, consumindo apenas 50MB de VRAM estática.\"\n  ]\n}",
            Self::Operations => "RESPONDA OBRIGATORIAMENTE EM PORTUGUES (PT-BR) NO FORMATO JSON PURO, SEM MARGINALIZAÇÃO OU BLOCOS DE CÓDIGO (COMO ```json). Atue como o Auditor Pessimista (FinOps e HardwareOps). Qual a real taxa de entropia? O sistema gera custos em nuvem ou Rate Limits perigosos? Liste o lixo toxico da stack original. O sistema 'fala' quando tem dor (observabilidade) e falha graciosamente? Se o payload contiver repo_kind=SkillLibrary ou repo_kind=ContentRepo, avalie riscos de prompt-injection, licenca, drift temporal (links mortos), custo de curadoria, e se o conteudo incentiva stack proibida. Responda estritamente em JSON com as chaves: lens_id, repo_id, model_used, bullets. E TERMINANTEMENTE PROIBIDO deixar o array 'bullets' vazio. Divida o seu raciocinio em 3 a 5 pontos factuais dentro do array 'bullets'.\n\nExemplo de formato de saída esperado:\n{\n  \"lens_id\": \"LensC_Operations\",\n  \"repo_id\": \"owner/repo\",\n  \"model_used\": \"nome-do-modelo\",\n  \"bullets\": [\n    \"Uso ineficiente de chamadas consecutivas à API sem mecanismos de caching.\",\n    \"A dependência de bibliotecas de terceiros sem auditoria de segurança ativa.\",\n    \"Falta de telemetria estruturada de erros e alertas de rate limit.\"\n  ]\n}",
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Phase2Error {
    #[error("Repositorio invalido: {0}")]
    InvalidRepoId(String),
    #[error("Falha ao buscar payloads da F1 (Destilador FinOps): {0}")]
    PayloadFetchError(String),
    #[error("Falha de configuracao da F2 (Enxame Cognitivo): {0}")]
    ConfigError(String),
    #[error("Pacote ausente ou vazio: {0}")]
    EmptyPackage(String),
    #[error("Falha na lente {lens}: {message}")]
    LensExecutionError { lens: String, message: String },
    #[error("Falha ao persistir debates: {0}")]
    PersistError(String),
    #[error("Repositorio marcado como erro na F2 (Enxame Cognitivo): {0}")]
    Phase2Aborted(String),
}

pub trait DebateStore: Send + Sync {
    fn fetch_phase2_payloads(&self, repo_id: &str) -> Result<Phase2Payloads, String>;
    fn persist_debate(&self, repo_id: &str, debate: &SwarmDebate) -> Result<(), String>;
    fn mark_phase2_error(&self, repo_id: &str) -> Result<(), String>;
}

pub trait LensInvoker: Send + Sync {
    fn invoke<'a>(
        &'a self,
        lens: LensKind,
        repo_id: &'a str,
        payload: &'a str,
        model_override: Option<&'a str>,
    ) -> LensFuture<'a>;

    fn primary_model(&self, _lens: LensKind) -> Option<&str> {
        None
    }

    fn ops2_model(&self) -> Option<&str> {
        None
    }

    fn default_model(&self) -> Option<&str> {
        None
    }

    fn last_resort_model(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LensDebatePayload {
    #[serde(default)]
    pub lens_id: String,
    #[serde(default)]
    pub repo_id: String,
    #[serde(default)]
    pub model_used: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

pub struct CognitiveSwarmDispatcher<S, I> {
    store: S,
    invoker: I,
}

impl<S, I> CognitiveSwarmDispatcher<S, I>
where
    S: DebateStore,
    I: LensInvoker,
{
    pub fn new(store: S, invoker: I) -> Self {
        Self { store, invoker }
    }

    pub async fn dispatch_swarm(&self, repo_id: &str) -> Result<(), Phase2Error> {
        if repo_id.trim().is_empty() {
            return Err(Phase2Error::InvalidRepoId(repo_id.to_string()));
        }

        let payloads = self
            .store
            .fetch_phase2_payloads(repo_id)
            .map_err(Phase2Error::PayloadFetchError)?;

        Self::validate_payload("package_a", &payloads.package_a)?;
        Self::validate_payload("package_b", &payloads.package_b)?;
        Self::validate_payload("package_c", &payloads.package_c)?;

        let (lens_a, lens_b, lens_c) = tokio::join!(
            self.execute_lens(LensKind::ProductUx, repo_id, &payloads.package_a),
            self.execute_lens(LensKind::Architecture, repo_id, &payloads.package_b),
            self.execute_lens(LensKind::Operations, repo_id, &payloads.package_c),
        );

        let lens_a_failed = lens_a.is_err();
        let lens_b_failed = lens_b.is_err();
        let lens_c_failed = lens_c.is_err();

        // Se TODAS falharem, aí marcamos erro global e abortamos
        if lens_a_failed && lens_b_failed && lens_c_failed {
            let root_error = lens_a.err().unwrap_or_else(|| {
                Phase2Error::EmptyPackage("Todas as lentes do enxame falharam".to_string())
            });
            self.store
                .mark_phase2_error(repo_id)
                .map_err(Phase2Error::Phase2Aborted)?;
            return Err(root_error);
        }

        let lente_a = match lens_a {
            Ok(val) => val,
            Err(e) => {
                tracing::error!(repo_id = %repo_id, error = %e, "Lente ProductUx falhou");
                serde_json::json!({
                    "lens_id": "LensA_ProductUX",
                    "repo_id": repo_id,
                    "model_used": "unknown",
                    "bullets": [format!("Lente ProductUx falhou: {}", e)]
                }).to_string()
            }
        };

        let lente_b = match lens_b {
            Ok(val) => val,
            Err(e) => {
                tracing::error!(repo_id = %repo_id, error = %e, "Lente Architecture falhou");
                serde_json::json!({
                    "lens_id": "LensB_Architecture",
                    "repo_id": repo_id,
                    "model_used": "unknown",
                    "bullets": [format!("Lente Architecture falhou: {}", e)]
                }).to_string()
            }
        };

        let lente_c = match lens_c {
            Ok(val) => val,
            Err(e) => {
                tracing::error!(repo_id = %repo_id, error = %e, "Lente Operations falhou");
                serde_json::json!({
                    "lens_id": "LensC_Operations",
                    "repo_id": repo_id,
                    "model_used": "unknown",
                    "bullets": [format!("Lente Operations falhou: {}", e)]
                }).to_string()
            }
        };

        let debate = SwarmDebate {
            repo_id: repo_id.to_string(),
            lente_a,
            lente_b,
            lente_c,
        };

        self.store
            .persist_debate(repo_id, &debate)
            .map_err(Phase2Error::PersistError)?;

        Ok(())
    }

    fn validate_payload(package_name: &str, payload: &str) -> Result<(), Phase2Error> {
        if payload.trim().is_empty() {
            return Err(Phase2Error::EmptyPackage(package_name.to_string()));
        }
        Ok(())
    }

    async fn execute_lens(
        &self,
        lens: LensKind,
        repo_id: &str,
        payload: &str,
    ) -> Result<String, Phase2Error> {
        let mut errors: Vec<String> = Vec::new();
        let mut cascade: Vec<(&'static str, Option<&str>)> = Vec::new();

        let primary = self.invoker.primary_model(lens);
        let ops2 = self.invoker.ops2_model();
        let default = self.invoker.default_model();
        let last_resort = self.invoker.last_resort_model();

        cascade.push(("primario", primary));
        if lens == LensKind::Operations {
            cascade.push(("ops2", ops2));
        }
        cascade.push(("default", default));
        cascade.push(("last_resort", last_resort));

        for (stage, model_opt) in cascade {
            let Some(model) = model_opt else { continue };

            let max_attempts = if stage == "primario" { 3 } else { 1 };

            for attempt in 1..=max_attempts {
                let res = tokio::time::timeout(
                    LENS_TIMEOUT,
                    self.invoker.invoke(lens, repo_id, payload, Some(model)),
                )
                .await;

                match res {
                    Ok(Ok(result)) => return Ok(result),
                    Ok(Err(err)) => {
                        let msg = format!(
                            "stage={}, attempt {} => {}",
                            stage,
                            attempt,
                            truncate_for_log(&err, 1400)
                        );
                        errors.push(msg.clone());
                        tracing::warn!(
                            lens_id = lens.lens_id(),
                            repo_id = repo_id,
                            model_used = model,
                            stage = stage,
                            attempt = attempt,
                            error = %msg,
                            "Falha no passo da cascata FinOps"
                        );
                    }
                    Err(_) => {
                        let msg = format!(
                            "stage={}, attempt {} => timeout_ms={}",
                            stage,
                            attempt,
                            LENS_TIMEOUT.as_millis()
                        );
                        errors.push(msg.clone());
                        tracing::warn!(
                            lens_id = lens.lens_id(),
                            repo_id = repo_id,
                            model_used = model,
                            stage = stage,
                            attempt = attempt,
                            error = %msg,
                            "Timeout atingido na execução da lente"
                        );
                    }
                };
            }
        }

        if errors.is_empty() {
            errors.push("Nenhum modelo configurado na cascata FinOps (todos valores None)".to_string());
        }

        Err(Phase2Error::LensExecutionError {
            lens: lens.lens_id().to_string(),
            message: errors.join(" | "),
        })
    }
}

#[derive(Debug, Clone)]
struct HttpLensConfig {
    base_url: String,
    api_key: String,
    model: String,
}

pub struct HttpLensInvoker {
    client: Client,
    claude: HttpLensConfig,
    deepseek: HttpLensConfig,
    glm: HttpLensConfig,
    ops2: HttpLensConfig,
    default: HttpLensConfig,
    last_resort: HttpLensConfig,
}

impl HttpLensInvoker {
    pub fn from_env() -> Result<Self, Phase2Error> {
        Self::from_openrouter_env()
    }

    pub fn from_openrouter_env() -> Result<Self, Phase2Error> {
        let api_key = get_first_env(&[
            "OPENROUTER_API_HEAVY_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
        ])
        .ok_or_else(|| {
            Phase2Error::ConfigError(
                "OPENROUTER_API_HEAVY_KEY/OPENROUTER_API_FAST_KEY/OPENROUTER_API_FREE_KEY ausente"
                    .to_string(),
            )
        })?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .map(|base| format!("{}/chat/completions", base.trim_end_matches('/')))
            .or_else(|_| std::env::var("OPENROUTER_BASE_URL"))
            .unwrap_or_else(|_| DEFAULT_OPENROUTER_URL.to_string());

        Ok(Self {
            client: Client::builder()
                .timeout(OPENROUTER_HTTP_TIMEOUT)
                .build()
                .map_err(|e| Phase2Error::ConfigError(format!("Falha ao construir reqwest client: {}", e)))?,
            claude: HttpLensConfig {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: std::env::var(LensKind::ProductUx.model_env_key()).map_err(|_| {
                    Phase2Error::ConfigError(
                        "OPENROUTER_HEAVY_MODEL_LENS_PROD_UX ausente".to_string(),
                    )
                })?,
            },
            deepseek: HttpLensConfig {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: std::env::var(LensKind::Architecture.model_env_key()).map_err(|_| {
                    Phase2Error::ConfigError(
                        "OPENROUTER_HEAVY_MODEL_LENS_ARQ ausente".to_string(),
                    )
                })?,
            },
            glm: HttpLensConfig {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: std::env::var(LensKind::Operations.model_env_key()).map_err(|_| {
                    Phase2Error::ConfigError(
                        "OPENROUTER_HEAVY_MODEL_LENS_OPS ausente".to_string(),
                    )
                })?,
            },
            ops2: HttpLensConfig {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: std::env::var("OPENROUTER_HEAVY_MODEL_LENS_OPS_2")
                    .unwrap_or_else(|_| "google/gemini-2.5-flash".to_string()),
            },
            default: HttpLensConfig {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: std::env::var("OPENROUTER_DEFAULT_MODEL")
                    .unwrap_or_else(|_| "google/gemini-2.5-flash".to_string()),
            },
            last_resort: HttpLensConfig {
                base_url,
                api_key,
                model: std::env::var("OPENROUTER_HEAVY_MODEL_LAST_RESORCE").map_err(|_| {
                    Phase2Error::ConfigError(
                        "OPENROUTER_HEAVY_MODEL_LAST_RESORCE ausente".to_string(),
                    )
                })?,
            },
        })
    }

    #[cfg(test)]
    fn with_configs(claude: HttpLensConfig, deepseek: HttpLensConfig, glm: HttpLensConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(OPENROUTER_HTTP_TIMEOUT)
                .build()
                .expect("reqwest client"),
            claude,
            deepseek,
            glm,
            ops2: HttpLensConfig {
                base_url: DEFAULT_OPENROUTER_URL.to_string(),
                api_key: "test".to_string(),
                model: "google/gemini-2.5-flash".to_string(),
            },
            default: HttpLensConfig {
                base_url: DEFAULT_OPENROUTER_URL.to_string(),
                api_key: "test".to_string(),
                model: "google/gemini-2.5-flash".to_string(),
            },
            last_resort: HttpLensConfig {
                base_url: DEFAULT_OPENROUTER_URL.to_string(),
                api_key: "test".to_string(),
                model: "anthropic/claude-opus-4.7".to_string(),
            },
        }
    }

    fn config_for(&self, lens: LensKind) -> &HttpLensConfig {
        match lens {
            LensKind::ProductUx => &self.claude,
            LensKind::Architecture => &self.deepseek,
            LensKind::Operations => &self.glm,
        }
    }
}

impl LensInvoker for HttpLensInvoker {
    fn invoke<'a>(
        &'a self,
        lens: LensKind,
        repo_id: &'a str,
        payload: &'a str,
        model_override: Option<&'a str>,
    ) -> LensFuture<'a> {
        Box::pin(async move {
            let config = self.config_for(lens);
            let model_used = model_override.unwrap_or(&config.model);
            let mut user_prefix = format!("repo_id={}\n", repo_id);
            if payload_looks_like_knowledge_repo(payload) {
                user_prefix.push_str("ALERTA: Este é um repositório de Conhecimento/Metodologia (stack_base desconhecida ou conteúdo sem stack). Ignore exigências de código fonte/AVX2/Bare-Metal. Avalie prompts, padrões teóricos, metodologia e artefatos textuais a serem canibalizados.\n");
            }
            let mut system_prompt = lens.system_prompt().to_string();
            let mut reasoning_effort: Option<String> = None;
            system_prompt.push_str("\nVocê deve retornar EXCLUSIVAMENTE um JSON válido. Não utilize blocos de código Markdown (```json).");
            if model_used.contains("deepseek-v4-pro") {
                reasoning_effort = Some("xhigh".to_string());
            }

            let response_format = if model_used.contains("google/") {
                None
            } else {
                Some(ChatResponseFormat {
                    kind: "json_object".to_string(),
                })
            };

            let body = ChatCompletionsRequest {
                model: model_used.to_string(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: ChatMessageContent::Text(system_prompt),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: build_cached_payload_content(&format!("{}{}", user_prefix, payload)),
                    },
                ],
                max_tokens: LENS_MAX_TOKENS,
                temperature: 0.0,
                response_format,
                reasoning_effort,
            };

            #[cfg(not(test))]
            {
                use tinyrand::RandRange;
                let jitter_ms = crate::telemetry::dynamic_wyrand().next_range(0..2500);
                tokio::time::sleep(tokio::time::Duration::from_millis(jitter_ms)).await;
            }

            tracing::info!(
                lens_id = lens.lens_id(),
                repo_id = repo_id,
                model_used = model_used,
                max_tokens = LENS_MAX_TOKENS,
                response_format_enabled = !model_used.contains("google/"),
                reasoning_effort_enabled = model_used.contains("deepseek-v4-pro"),
                base_url = %config.base_url,
                "F2: enviando request para OpenRouter"
            );

            let response = self
                .client
                .post(&config.base_url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        tracing::warn!(
                            lens_id = lens.lens_id(),
                            repo_id = repo_id,
                            model_used = model_used,
                            timeout_ms = OPENROUTER_HTTP_TIMEOUT.as_millis(),
                            "Timeout HTTP ao chamar OpenRouter"
                        );
                    }
                    e.to_string()
                })?;

            let status = response.status();
            let raw_response = response.text().await.map_err(|e| e.to_string())?;
            if status != StatusCode::OK {
                tracing::error!(
                    lens_id = lens.lens_id(),
                    repo_id = repo_id,
                    model_used = model_used,
                    status = status.as_u16(),
                    response_body = %truncate_for_log(&raw_response, 6000),
                    "OpenRouter retornou status != 200; abortando parse"
                );
                return Err(format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    truncate_for_log(&raw_response, 6000)
                ));
            }

            let parsed: serde_json::Value = serde_json::from_str(&raw_response).map_err(|e| {
                format!(
                    "Resposta HTTP invalida da lente {}: {}",
                    lens.lens_id(),
                    e
                )
            })?;
            let usage = extract_openrouter_usage(&parsed);
            let content = extract_chat_message_content_from_parsed(&parsed);
            if content.trim().is_empty() {
                return Err(format!("Resposta vazia da lente {}", lens.lens_id()));
            }
            let normalized = normalize_lens_payload(lens, repo_id, model_used, &content)?;
            Ok(inject_usage_into_lens_payload(&normalized, usage))
        })
    }

    fn last_resort_model(&self) -> Option<&str> {
        Some(self.last_resort.model.as_str())
    }

    fn ops2_model(&self) -> Option<&str> {
        Some(self.ops2.model.as_str())
    }

    fn default_model(&self) -> Option<&str> {
        Some(self.default.model.as_str())
    }

    fn primary_model(&self, lens: LensKind) -> Option<&str> {
        Some(self.config_for(lens).model.as_str())
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionsRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: usize,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ChatResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: ChatMessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ChatMessageContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Debug, Serialize)]
struct ChatContentPart {
    #[serde(rename = "type")]
    kind: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<ChatCacheControl>,
}

#[derive(Debug, Serialize)]
struct ChatCacheControl {
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatResponseFormat {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Default, Clone, Copy)]
struct OpenRouterUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
}

fn extract_openrouter_usage(parsed: &serde_json::Value) -> OpenRouterUsage {
    let usage = &parsed["usage"];
    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total_tokens = usage
        .get("total_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(prompt_tokens.saturating_add(completion_tokens));
    let total_cost_usd = usage
        .get("total_cost")
        .or_else(|| usage.get("cost"))
        .or_else(|| usage.get("estimated_cost"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    OpenRouterUsage {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        total_cost_usd,
    }
}

fn extract_chat_message_content_from_parsed(parsed: &serde_json::Value) -> String {
    let content = parsed
        .get("choices")
        .and_then(|choices| choices.as_array())
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"));
    content.map(flatten_chat_content).unwrap_or_default()
}

fn build_cached_payload_content(payload: &str) -> ChatMessageContent {
    let Some((before, after)) = payload.split_once(BLOB_10_CANON_MARKER) else {
        return ChatMessageContent::Text(payload.to_string());
    };

    let mut parts = Vec::new();
    if !before.trim().is_empty() {
        parts.push(ChatContentPart {
            kind: "text".to_string(),
            text: before.to_string(),
            cache_control: None,
        });
    }

    let canon_block = format!("{BLOB_10_CANON_MARKER}{after}");
    parts.push(ChatContentPart {
        kind: "text".to_string(),
        text: canon_block,
        cache_control: Some(ChatCacheControl {
            kind: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
        }),
    });

    ChatMessageContent::Parts(parts)
}

fn inject_usage_into_lens_payload(json_payload: &str, usage: OpenRouterUsage) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(json_payload) else {
        return json_payload.to_string();
    };
    let Some(obj) = value.as_object_mut() else {
        return json_payload.to_string();
    };
    obj.insert("prompt_tokens".to_string(), serde_json::json!(usage.prompt_tokens));
    obj.insert(
        "completion_tokens".to_string(),
        serde_json::json!(usage.completion_tokens),
    );
    obj.insert("total_tokens".to_string(), serde_json::json!(usage.total_tokens));
    obj.insert(
        "total_cost_usd".to_string(),
        serde_json::json!(usage.total_cost_usd),
    );
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| json_payload.to_string())
}

fn flatten_chat_content(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(items) => items
            .iter()
            .map(flatten_chat_content)
            .filter(|chunk| !chunk.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Object(map) => ["text", "content", "value"]
            .iter()
            .find_map(|key| map.get(*key))
            .map(flatten_chat_content)
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub struct SqliteDebateStore {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
struct TableColumn {
    name: String,
    declared_type: String,
    not_null: bool,
    default_value: Option<String>,
}

pub fn ensure_phase2_schema(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS debates_enxame (
            repo_id TEXT PRIMARY KEY,
            lens_a_json TEXT NOT NULL,
            lens_b_json TEXT NOT NULL,
            lens_c_json TEXT NOT NULL,
            model_used TEXT NOT NULL,
            phase_status TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("Falha ao criar debates_enxame: {}", e))?;

    let columns = describe_table(conn, "debates_enxame")?;
    let column_names: Vec<String> = columns.iter().map(|col| col.name.clone()).collect();

    if !column_names.iter().any(|col| col == "lens_a_json") {
        conn.execute(
            "ALTER TABLE debates_enxame ADD COLUMN lens_a_json TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| format!("Falha ao migrar coluna lens_a_json: {}", e))?;
    }
    if !column_names.iter().any(|col| col == "lens_b_json") {
        conn.execute(
            "ALTER TABLE debates_enxame ADD COLUMN lens_b_json TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| format!("Falha ao migrar coluna lens_b_json: {}", e))?;
    }
    if !column_names.iter().any(|col| col == "lens_c_json") {
        conn.execute(
            "ALTER TABLE debates_enxame ADD COLUMN lens_c_json TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| format!("Falha ao migrar coluna lens_c_json: {}", e))?;
    }
    if !column_names.iter().any(|col| col == "phase_status") {
        conn.execute(
            "ALTER TABLE debates_enxame ADD COLUMN phase_status TEXT NOT NULL DEFAULT 'PENDING'",
            [],
        )
        .map_err(|e| format!("Falha ao migrar coluna phase_status: {}", e))?;
    }
    if !column_names.iter().any(|col| col == "model_used") {
        conn.execute(
            "ALTER TABLE debates_enxame ADD COLUMN model_used TEXT NOT NULL DEFAULT '{}'",
            [],
        )
        .map_err(|e| format!("Falha ao migrar coluna model_used: {}", e))?;
    }

    if column_names.iter().any(|col| col == "lente_a") {
        conn.execute(
            "UPDATE debates_enxame
             SET lens_a_json = CASE
                 WHEN lens_a_json = '' THEN COALESCE(lente_a, '')
                 ELSE lens_a_json
             END",
            [],
        )
        .map_err(|e| format!("Falha ao migrar dados de lente_a para lens_a_json: {}", e))?;
    }
    if column_names.iter().any(|col| col == "lente_b") {
        conn.execute(
            "UPDATE debates_enxame
             SET lens_b_json = CASE
                 WHEN lens_b_json = '' THEN COALESCE(lente_b, '')
                 ELSE lens_b_json
             END",
            [],
        )
        .map_err(|e| format!("Falha ao migrar dados de lente_b para lens_b_json: {}", e))?;
    }
    if column_names.iter().any(|col| col == "lente_c") {
        conn.execute(
            "UPDATE debates_enxame
             SET lens_c_json = CASE
                 WHEN lens_c_json = '' THEN COALESCE(lente_c, '')
                 ELSE lens_c_json
             END",
            [],
        )
        .map_err(|e| format!("Falha ao migrar dados de lente_c para lens_c_json: {}", e))?;
    }

    Ok(())
}

impl SqliteDebateStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn ensure_schema_locked(conn: &Connection) -> Result<(), String> {
        ensure_phase2_schema(conn)
    }

    fn fetch_package(conn: &Connection, repo_id: &str, name: &str) -> Result<String, String> {
        conn.query_row(
            "SELECT payload_package
             FROM pacotes_destilados
             WHERE repo_id = ?1 AND package_name = ?2
             LIMIT 1",
            params![repo_id, name],
            |row| row.get(0),
        )
        .map_err(|e| format!("Pacote {} ausente para {}: {}", name, repo_id, e))
    }
}

fn describe_table(conn: &Connection, table_name: &str) -> Result<Vec<TableColumn>, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table_name))
        .map_err(|e| format!("Falha ao preparar PRAGMA de {}: {}", table_name, e))?;

    let mapped = stmt
        .query_map([], |row| {
        Ok(TableColumn {
            name: row.get(1)?,
            declared_type: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            not_null: row.get::<_, i64>(3)? != 0,
            default_value: row.get(4)?,
        })
    })
    .map_err(|e| format!("Falha ao ler schema de {}: {}", table_name, e))?;

    mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Falha ao coletar schema de {}: {}", table_name, e))
}

fn default_value_for_column(column: &TableColumn) -> Value {
    match column.name.as_str() {
        "contexto_oraculo_soda" => Value::Text("blob_10_soda_canon_context".to_string()),
        "lens_a_json" | "lens_b_json" | "lens_c_json" | "phase_status" | "model_used" => {
            Value::Text(String::new())
        }
        _ if column.declared_type.to_ascii_uppercase().contains("INT") => Value::Integer(0),
        _ if column.declared_type.to_ascii_uppercase().contains("REAL")
            || column.declared_type.to_ascii_uppercase().contains("FLOA")
            || column.declared_type.to_ascii_uppercase().contains("DOUB") =>
        {
            Value::Real(0.0)
        }
        _ => Value::Text(String::new()),
    }
}

impl DebateStore for SqliteDebateStore {
    fn fetch_phase2_payloads(&self, repo_id: &str) -> Result<Phase2Payloads, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Falha ao adquirir lock do SQLite: {}", e))?;

        Ok(Phase2Payloads {
            package_a: Self::fetch_package(&conn, repo_id, "A")?,
            package_b: Self::fetch_package(&conn, repo_id, "B")?,
            package_c: Self::fetch_package(&conn, repo_id, "C")?,
        })
    }

    fn persist_debate(&self, repo_id: &str, debate: &SwarmDebate) -> Result<(), String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| format!("Falha ao adquirir lock do SQLite: {}", e))?;

        Self::ensure_schema_locked(&conn)?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Falha ao abrir transacao SQLite: {}", e))?;

        let model_used_summary = build_model_used_summary(debate)?;
        let updated = tx
            .execute(
                "UPDATE debates_enxame
                 SET lens_a_json = ?2,
                     lens_b_json = ?3,
                     lens_c_json = ?4,
                     model_used = ?5,
                     phase_status = ?6
                 WHERE repo_id = ?1",
                params![
                    repo_id,
                    debate.lente_a,
                    debate.lente_b,
                    debate.lente_c,
                    model_used_summary,
                    STATUS_OK
                ],
            )
            .map_err(|e| format!("Falha ao atualizar debates_enxame: {}", e))?;

        if updated == 0 {
            let columns = describe_table(&tx, "debates_enxame")?;
            let mut insert_columns = Vec::new();
            let mut insert_values = Vec::new();

            for column in columns {
                let value = match column.name.as_str() {
                    "repo_id" => Some(Value::Text(repo_id.to_string())),
                    "lens_a_json" => Some(Value::Text(debate.lente_a.clone())),
                    "lens_b_json" => Some(Value::Text(debate.lente_b.clone())),
                    "lens_c_json" => Some(Value::Text(debate.lente_c.clone())),
                    "model_used" => Some(Value::Text(model_used_summary.clone())),
                    "phase_status" => Some(Value::Text(STATUS_OK.to_string())),
                    _ if column.not_null && column.default_value.is_none() => {
                        Some(default_value_for_column(&column))
                    }
                    _ => None,
                };

                if let Some(value) = value {
                    insert_columns.push(column.name);
                    insert_values.push(value);
                }
            }

            let placeholders = (1..=insert_columns.len())
                .map(|idx| format!("?{}", idx))
                .collect::<Vec<_>>()
                .join(", ");
            tx.execute(
                &format!(
                    "INSERT INTO debates_enxame ({}) VALUES ({})",
                    insert_columns.join(", "),
                    placeholders
                ),
                params_from_iter(insert_values.iter()),
            )
            .map_err(|e| format!("Falha ao inserir debates_enxame: {}", e))?;
        }

        tx.execute(
            "UPDATE repositorios
             SET status_processamento = ?1
             WHERE project_name = ?2",
            params![STATUS_OK, repo_id],
        )
        .map_err(|e| format!("Falha ao atualizar status do repositorio: {}", e))?;

        tx.commit()
            .map_err(|e| format!("Falha ao finalizar transacao SQLite: {}", e))
    }

    fn mark_phase2_error(&self, repo_id: &str) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Falha ao adquirir lock do SQLite: {}", e))?;

        Self::ensure_schema_locked(&conn)?;
        conn.execute(
            "UPDATE repositorios
             SET status_processamento = ?1
             WHERE project_name = ?2",
            params![STATUS_ERR, repo_id],
        )
        .map_err(|e| format!("Falha ao marcar ERRO_FASE_2: {}", e))?;
        Ok(())
    }
}

fn get_first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| std::env::var(key).ok().filter(|value| !value.trim().is_empty()))
}

fn payload_looks_like_knowledge_repo(payload: &str) -> bool {
    let lower = payload.to_ascii_lowercase();
    lower.contains("stack_base: unknown")
        || lower.contains("stack_base: n/a")
        || lower.contains("\nstack_base: unknown")
        || lower.contains("\nstack_base: n/a")
        || lower.contains("\nstack_base: \n")
        || lower.contains("repo_kind=skilllibrary")
        || lower.contains("repo_kind=contentrepo")
}

fn normalize_lens_payload(
    lens: LensKind,
    repo_id: &str,
    model_used: &str,
    raw: &str,
) -> Result<String, String> {
    let candidate = extract_first_json_object(raw).unwrap_or_else(|| raw.trim().to_string());
    let mut parsed: LensDebatePayload = match serde_json::from_str(&candidate) {
        Ok(parsed) => parsed,
        Err(json_err) => extract_bullets_fallback(raw)
            .map(|bullets| LensDebatePayload {
                lens_id: lens.lens_id().to_string(),
                repo_id: repo_id.to_string(),
                model_used: model_used.to_string(),
                bullets,
            })
            .ok_or_else(|| {
                format!(
                    "JSON invalido da lente {}: {} | raw_snippet={}",
                    lens.lens_id(),
                    json_err,
                    truncate_for_log(&candidate, 900)
                )
            })?,
    };

    if parsed.lens_id.trim().is_empty() {
        parsed.lens_id = lens.lens_id().to_string();
    }
    if parsed.repo_id.trim().is_empty() {
        parsed.repo_id = repo_id.to_string();
    }
    parsed.model_used = model_used.to_string();

    parsed.bullets = parsed
        .bullets
        .into_iter()
        .map(|bullet| bullet.trim().to_string())
        .filter(|bullet| !bullet.is_empty())
        .collect();

    if parsed.bullets.is_empty() {
        if let Some(bullets) = extract_bullets_fallback(raw) {
            parsed.bullets = bullets;
        }
    }

    if parsed.bullets.len() < 3 || parsed.bullets.len() > 5 {
        return Err(format!(
            "Mini-JSON invalido da lente {}: esperado 3 a 5 bullets, recebido {}",
            lens.lens_id(),
            parsed.bullets.len()
        ));
    }

    serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("Falha ao serializar JSON canonico da lente {}: {}", lens.lens_id(), e))
}

fn extract_first_json_object(raw: &str) -> Option<String> {
    let stripped = raw
        .trim()
        .strip_prefix("```json")
        .map(str::trim)
        .unwrap_or(raw)
        .strip_prefix("```")
        .map(str::trim)
        .unwrap_or(raw)
        .strip_suffix("```")
        .map(str::trim)
        .unwrap_or(raw);

    let start = stripped.find('{')?;
    let end = stripped.rfind('}')?;
    Some(stripped[start..=end].to_string())
}

fn truncate_for_log(value: &str, max_len: usize) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= max_len {
        return trimmed.to_string();
    }
    trimmed.chars().take(max_len).collect::<String>()
}

fn build_model_used_summary(debate: &SwarmDebate) -> Result<String, String> {
    let lens_a: LensDebatePayload = serde_json::from_str(&debate.lente_a)
        .map_err(|e| format!("Falha ao ler model_used da lente A: {}", e))?;
    let lens_b: LensDebatePayload = serde_json::from_str(&debate.lente_b)
        .map_err(|e| format!("Falha ao ler model_used da lente B: {}", e))?;
    let lens_c: LensDebatePayload = serde_json::from_str(&debate.lente_c)
        .map_err(|e| format!("Falha ao ler model_used da lente C: {}", e))?;

    serde_json::to_string_pretty(&serde_json::json!({
        "lens_a": lens_a.model_used,
        "lens_b": lens_b.model_used,
        "lens_c": lens_c.model_used,
    }))
    .map_err(|e| format!("Falha ao serializar coluna model_used: {}", e))
}

fn extract_bullets_fallback(raw: &str) -> Option<Vec<String>> {
    let bullets: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            let normalized = line
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim_start_matches("+ ")
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .trim_start_matches(". ")
                .trim_start_matches(") ")
                .trim();
            let looks_like_bullet = line.starts_with("- ")
                || line.starts_with("* ")
                || line.starts_with("+ ")
                || line
                    .chars()
                    .next()
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false);
            (looks_like_bullet && !normalized.is_empty()).then(|| normalized.to_string())
        })
        .collect();

    (3..=5).contains(&bullets.len()).then_some(bullets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    #[derive(Clone)]
    struct RecordingLensInvoker {
        delays_ms: HashMap<LensKind, u64>,
        payloads: Arc<Mutex<Vec<(LensKind, String)>>>,
        attempts: Arc<Mutex<HashMap<LensKind, usize>>>,
    }

    impl RecordingLensInvoker {
        fn new(
            delays_ms: HashMap<LensKind, u64>,
            _results: HashMap<LensKind, Result<String, String>>,
        ) -> Self {
            Self {
                delays_ms,
                payloads: Arc::new(Mutex::new(Vec::new())),
                attempts: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn payload_for(&self, lens: LensKind) -> String {
            self.payloads
                .lock()
                .expect("payload mutex poisoned")
                .iter()
                .find(|(recorded_lens, _)| *recorded_lens == lens)
                .map(|(_, payload)| payload.clone())
                .expect("payload not recorded")
        }

    }

    impl LensInvoker for RecordingLensInvoker {
        fn invoke<'a>(
            &'a self,
            lens: LensKind,
            _repo_id: &'a str,
            payload: &'a str,
            model_override: Option<&'a str>,
        ) -> LensFuture<'a> {
            Box::pin(async move {
                self.payloads
                    .lock()
                    .expect("payload mutex poisoned")
                    .push((lens, payload.to_string()));
                *self
                    .attempts
                    .lock()
                    .expect("attempt mutex poisoned")
                    .entry(lens)
                    .or_insert(0) += 1;

                if let Some(delay_ms) = self.delays_ms.get(&lens) {
                    sleep(Duration::from_millis(*delay_ms)).await;
                }

                Ok(default_json(lens, model_override.unwrap_or("mock-model")))
            })
        }

        fn ops2_model(&self) -> Option<&str> {
            Some("mock-ops2-model")
        }

        fn default_model(&self) -> Option<&str> {
            Some("mock-default-model")
        }

        fn last_resort_model(&self) -> Option<&str> {
            Some("mock-last-resort-model")
        }

        fn primary_model(&self, _lens: LensKind) -> Option<&str> {
            Some("mock-primary-model")
        }
    }

    #[derive(Clone)]
    struct MemoryStore {
        payloads: Phase2Payloads,
        persisted: Arc<AtomicUsize>,
        errors: Arc<AtomicUsize>,
    }

    impl MemoryStore {
        fn new(payloads: Phase2Payloads) -> Self {
            Self {
                payloads,
                persisted: Arc::new(AtomicUsize::new(0)),
                errors: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl DebateStore for MemoryStore {
        fn fetch_phase2_payloads(&self, _repo_id: &str) -> Result<Phase2Payloads, String> {
            Ok(self.payloads.clone())
        }

        fn persist_debate(&self, _repo_id: &str, _debate: &SwarmDebate) -> Result<(), String> {
            self.persisted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn mark_phase2_error(&self, _repo_id: &str) -> Result<(), String> {
            self.errors.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn sample_payloads() -> Phase2Payloads {
        Phase2Payloads {
            package_a: "PACKAGE_A_ONLY\nblob_10_soda_canon_context".to_string(),
            package_b: "PACKAGE_B_ONLY\nblob_10_soda_canon_context".to_string(),
            package_c: "PACKAGE_C_ONLY\nblob_10_soda_canon_context".to_string(),
        }
    }

    fn default_json(lens: LensKind, model_used: &str) -> String {
        format!(
            "{{\"lens_id\":\"{}\",\"repo_id\":\"repo\",\"model_used\":\"{}\",\"bullets\":[\"ok-1\",\"ok-2\",\"ok-3\"]}}",
            lens.lens_id(),
            model_used
        )
    }

    fn create_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute(
            "CREATE TABLE repositorios (
                project_name TEXT PRIMARY KEY,
                status_processamento TEXT NOT NULL
            )",
            [],
        )
        .expect("create repositorios");
        conn.execute(
            "CREATE TABLE pacotes_destilados (
                package_id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id TEXT NOT NULL,
                package_name TEXT NOT NULL,
                payload_package TEXT NOT NULL,
                timestamp_empacotamento INTEGER NOT NULL,
                UNIQUE(repo_id, package_name)
            )",
            [],
        )
        .expect("create pacotes_destilados");
        conn.execute(
            "INSERT INTO repositorios (project_name, status_processamento) VALUES (?1, ?2)",
            params!["repo/test", "FASE_1_5_OK"],
        )
        .expect("insert repo");
        conn.execute(
            "INSERT INTO pacotes_destilados (repo_id, package_name, payload_package, timestamp_empacotamento)
             VALUES (?1, ?2, ?3, 1)",
            params!["repo/test", "A", "PACKAGE_A_ONLY\nblob_10_soda_canon_context"],
        )
        .expect("insert package A");
        conn.execute(
            "INSERT INTO pacotes_destilados (repo_id, package_name, payload_package, timestamp_empacotamento)
             VALUES (?1, ?2, ?3, 1)",
            params!["repo/test", "B", "PACKAGE_B_ONLY\nblob_10_soda_canon_context"],
        )
        .expect("insert package B");
        conn.execute(
            "INSERT INTO pacotes_destilados (repo_id, package_name, payload_package, timestamp_empacotamento)
             VALUES (?1, ?2, ?3, 1)",
            params!["repo/test", "C", "PACKAGE_C_ONLY\nblob_10_soda_canon_context"],
        )
        .expect("insert package C");
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_ensure_phase2_schema_migrates_legacy_columns() {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute(
            "CREATE TABLE debates_enxame (
                repo_id TEXT PRIMARY KEY,
                lente_a TEXT,
                lente_b TEXT,
                lente_c TEXT
            )",
            [],
        )
        .expect("create legacy debates_enxame");
        conn.execute(
            "INSERT INTO debates_enxame (repo_id, lente_a, lente_b, lente_c)
             VALUES (?1, ?2, ?3, ?4)",
            params!["repo/test", "{\"a\":1}", "{\"b\":1}", "{\"c\":1}"],
        )
        .expect("insert legacy row");

        ensure_phase2_schema(&conn).expect("migrate schema");

        let (a, b, c, model_used, status): (String, String, String, String, String) = conn
            .query_row(
                "SELECT lens_a_json, lens_b_json, lens_c_json, model_used, phase_status
                 FROM debates_enxame
                 WHERE repo_id = ?1",
                params!["repo/test"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .expect("query migrated row");

        assert_eq!(a, "{\"a\":1}");
        assert_eq!(b, "{\"b\":1}");
        assert_eq!(c, "{\"c\":1}");
        assert_eq!(model_used, "{}");
        assert_eq!(status, "PENDING");
    }

    #[test]
    fn test_normalize_lens_payload_extracts_json_from_code_fence() {
        let raw = "```json\n{\"lens_id\":\"LensA_ProductUX\",\"repo_id\":\"repo/test\",\"model_used\":\"google/gemini-3.5-flash\",\"bullets\":[\"a\",\"b\",\"c\"]}\n```";
        let normalized = normalize_lens_payload(
            LensKind::ProductUx,
            "repo/test",
            "google/gemini-3.5-flash",
            raw,
        )
        .expect("normalized json");
        let parsed: LensDebatePayload = serde_json::from_str(&normalized).expect("parsed canonical json");

        assert_eq!(parsed.lens_id, "LensA_ProductUX");
        assert_eq!(parsed.repo_id, "repo/test");
        assert_eq!(parsed.model_used, "google/gemini-3.5-flash");
        assert_eq!(parsed.bullets.len(), 3);
    }

    #[test]
    fn test_normalize_lens_payload_falls_back_to_plain_bullets() {
        let raw = "1. Primeiro achado\n2. Segundo achado\n3. Terceiro achado";
        let normalized = normalize_lens_payload(
            LensKind::ProductUx,
            "repo/test",
            "google/gemini-3.5-flash",
            raw,
        )
        .expect("normalized fallback");
        let parsed: LensDebatePayload = serde_json::from_str(&normalized).expect("parsed fallback json");

        assert_eq!(parsed.lens_id, "LensA_ProductUX");
        assert_eq!(parsed.repo_id, "repo/test");
        assert_eq!(parsed.model_used, "google/gemini-3.5-flash");
        assert_eq!(parsed.bullets.len(), 3);
    }

    #[test]
    fn test_normalize_lens_payload_strips_think_block_before_json() {
        let raw = "<think>cadeia privada</think>\n{\"lens_id\":\"LensA_ProductUX\",\"repo_id\":\"repo/test\",\"bullets\":[\"achado 1\",\"achado 2\",\"achado 3\"]}\ntexto residual";
        let normalized = normalize_lens_payload(
            LensKind::ProductUx,
            "repo/test",
            "google/gemini-3.5-flash",
            raw,
        )
        .expect("normalized think json");
        let parsed: LensDebatePayload = serde_json::from_str(&normalized).expect("parsed think json");

        assert_eq!(parsed.model_used, "google/gemini-3.5-flash");
        assert_eq!(parsed.bullets.len(), 3);
    }

    #[test]
    fn test_normalize_lens_payload_salvages_bullets_from_recommendation_when_empty() {
        let raw = "{\"lens_id\":\"LensA_ProductUX\",\"repo_id\":\"repo/test\",\"bullets\":[]}\n1. Ponto um\n2. Ponto dois\n3. Ponto tres";
        let normalized = normalize_lens_payload(
            LensKind::ProductUx,
            "repo/test",
            "google/gemini-3.5-flash",
            raw,
        )
        .expect("salvage bullets");
        let parsed: LensDebatePayload = serde_json::from_str(&normalized).expect("parsed salvage json");
        assert_eq!(parsed.bullets.len(), 3);
    }

    #[test]
    fn test_from_env_missing_var_returns_config_error() {
        let env_keys = [
            "OPENROUTER_API_HEAVY_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
            "OPENROUTER_HEAVY_MODEL_LENS_PROD_UX",
            "OPENROUTER_HEAVY_MODEL_LENS_ARQ",
            "OPENROUTER_HEAVY_MODEL_LENS_OPS",
            "OPENROUTER_HEAVY_MODEL_LAST_RESORCE",
        ];
        let previous_values: Vec<(String, Option<String>)> = env_keys
            .iter()
            .map(|key| ((*key).to_string(), std::env::var(key).ok()))
            .collect();

        for key in &env_keys {
            std::env::remove_var(key);
        }

        let result = HttpLensInvoker::from_env();

        for (key, value) in previous_values {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }

        assert!(matches!(
            result,
            Err(Phase2Error::ConfigError(_))
        ));
    }

    #[test]
    fn test_from_openrouter_env_missing_key_returns_config_error() {
        let env_keys = [
            "OPENROUTER_API_HEAVY_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
            "OPENROUTER_HEAVY_MODEL_LENS_PROD_UX",
            "OPENROUTER_HEAVY_MODEL_LENS_ARQ",
            "OPENROUTER_HEAVY_MODEL_LENS_OPS",
            "OPENROUTER_HEAVY_MODEL_LAST_RESORCE",
        ];
        let previous_values: Vec<(String, Option<String>)> = env_keys
            .iter()
            .map(|key| ((*key).to_string(), std::env::var(key).ok()))
            .collect();

        for key in &env_keys {
            std::env::remove_var(key);
        }

        let result = HttpLensInvoker::from_openrouter_env();

        for (key, value) in previous_values {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }

        assert!(matches!(result, Err(Phase2Error::ConfigError(_))));
    }

    #[tokio::test]
    async fn test_parallel_dispatch_uses_total_time_of_slowest_lens() {
        let store = MemoryStore::new(sample_payloads());
        let invoker = RecordingLensInvoker::new(
            HashMap::from([
                (LensKind::ProductUx, 180),
                (LensKind::Architecture, 120),
                (LensKind::Operations, 150),
            ]),
            HashMap::from([
                (
                    LensKind::ProductUx,
                    Ok(default_json(LensKind::ProductUx, "google/gemini-3.5-flash")),
                ),
                (
                    LensKind::Architecture,
                    Ok(default_json(LensKind::Architecture, "deepseek/deepseek-v4-pro")),
                ),
                (
                    LensKind::Operations,
                    Ok(default_json(LensKind::Operations, "z-ai/glm-5.1")),
                ),
            ]),
        );
        let dispatcher = CognitiveSwarmDispatcher::new(store, invoker);

        let started = Instant::now();
        let result = dispatcher.dispatch_swarm("repo/test").await;
        let elapsed = started.elapsed();

        assert!(result.is_ok());
        assert!(elapsed < Duration::from_millis(320), "tempo parece sequencial: {:?}", elapsed);
        assert!(elapsed >= Duration::from_millis(180), "tempo nao refletiu a lente mais lenta: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_timeout_does_not_hang_join_and_marks_phase2_error() {
        let store = MemoryStore::new(sample_payloads());
        let store_errors = Arc::clone(&store.errors);
        let store_persisted = Arc::clone(&store.persisted);
        let invoker = RecordingLensInvoker::new(
            HashMap::from([(LensKind::Architecture, 800)]),
            HashMap::new(),
        );
        let dispatcher = CognitiveSwarmDispatcher::new(store, invoker);

        let started = Instant::now();
        let result = dispatcher.dispatch_swarm("repo/test").await;
        let elapsed = started.elapsed();

        assert!(result.is_ok(), "Falha parcial nao deve abortar o debate global: {:?}", result);
        assert!(elapsed < Duration::from_secs(2), "timeout nao abortou em tempo util: {:?}", elapsed);
        assert_eq!(store_persisted.load(Ordering::SeqCst), 1);
        assert_eq!(store_errors.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_package_isolation_routes_only_its_own_payload_to_each_lens() {
        let store = MemoryStore::new(sample_payloads());
        let invoker = RecordingLensInvoker::new(HashMap::new(), HashMap::new());
        let inspector = invoker.clone();
        let dispatcher = CognitiveSwarmDispatcher::new(store, invoker);

        dispatcher
            .dispatch_swarm("repo/test")
            .await
            .expect("dispatcher should succeed");

        let payload_a = inspector.payload_for(LensKind::ProductUx);
        let payload_b = inspector.payload_for(LensKind::Architecture);
        let payload_c = inspector.payload_for(LensKind::Operations);

        assert!(payload_a.contains("PACKAGE_A_ONLY"));
        assert!(!payload_a.contains("PACKAGE_B_ONLY"));
        assert!(!payload_a.contains("PACKAGE_C_ONLY"));

        assert!(payload_b.contains("PACKAGE_B_ONLY"));
        assert!(!payload_b.contains("PACKAGE_A_ONLY"));
        assert!(!payload_b.contains("PACKAGE_C_ONLY"));

        assert!(payload_c.contains("PACKAGE_C_ONLY"));
        assert!(!payload_c.contains("PACKAGE_A_ONLY"));
        assert!(!payload_c.contains("PACKAGE_B_ONLY"));
    }

    #[tokio::test]
    async fn test_fail_fast_aborts_after_third_429_and_last_resort_and_keeps_debate_table_empty() {
        let mut claude_server = Server::new_async().await;
        let mut deepseek_server = Server::new_async().await;
        let mut glm_server = Server::new_async().await;

        let claude_mock = claude_server
            .mock("POST", "/claude")
            .match_body(Matcher::Regex("PACKAGE_A_ONLY".to_string()))
            .with_status(429)
            .with_body(r#"{"error":"rate limit"}"#)
            .expect(5)
            .create();

        let deepseek_mock = deepseek_server
            .mock("POST", "/deepseek")
            .match_body(Matcher::Regex("PACKAGE_B_ONLY".to_string()))
            .with_status(429)
            .with_body(r#"{"error":"rate limit"}"#)
            .expect(5)
            .create();

        let glm_mock = glm_server
            .mock("POST", "/glm")
            .match_body(Matcher::Regex("PACKAGE_C_ONLY".to_string()))
            .with_status(429)
            .with_body(r#"{"error":"rate limit"}"#)
            .expect(6)
            .create();

        let store_conn = create_test_db();
        let store = SqliteDebateStore::new(Arc::clone(&store_conn));
        let invoker = HttpLensInvoker::with_configs(
            HttpLensConfig {
                base_url: format!("{}/claude", claude_server.url()),
                api_key: "test".to_string(),
                model: "google/gemini-3.5-flash".to_string(),
            },
            HttpLensConfig {
                base_url: format!("{}/deepseek", deepseek_server.url()),
                api_key: "test".to_string(),
                model: "deepseek/deepseek-v4-pro".to_string(),
            },
            HttpLensConfig {
                base_url: format!("{}/glm", glm_server.url()),
                api_key: "test".to_string(),
                model: "z-ai/glm-5.1".to_string(),
            },
        );
        let dispatcher = CognitiveSwarmDispatcher::new(store, invoker);

        let result = dispatcher.dispatch_swarm("repo/test").await;

        claude_mock.assert();
        deepseek_mock.assert();
        glm_mock.assert();

        assert!(matches!(
            &result,
            Err(Phase2Error::LensExecutionError { ref lens, .. }) if lens == "LensA_ProductUX"
        ));
        let error_message = match result {
            Err(Phase2Error::LensExecutionError { message, .. }) => message,
            other => panic!("resultado inesperado: {:?}", other),
        };
        assert!(error_message.contains("stage="));

        let conn = store_conn.lock().expect("sqlite lock poisoned");
        let debates_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM debates_enxame", [], |row| row.get(0))
            .expect("count debates");
        let status: String = conn
            .query_row(
                "SELECT status_processamento FROM repositorios WHERE project_name = ?1",
                params!["repo/test"],
                |row| row.get(0),
            )
            .expect("query repo status");

        assert_eq!(debates_count, 0);
        assert_eq!(status, STATUS_ERR);
    }

}
