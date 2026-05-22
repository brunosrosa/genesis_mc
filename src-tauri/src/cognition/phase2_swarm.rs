use crate::cognition::sgr_synthesizer::SwarmDebate;
use crate::finops::phase1_5::package_assembler::Phase2Payloads;
use reqwest::Client;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use thiserror::Error;

const STATUS_OK: &str = "FASE_2_OK";
const STATUS_ERR: &str = "ERRO_FASE_2";
const DEFAULT_OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

type LensFuture<'a> = Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

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

    fn model_name(self) -> &'static str {
        match self {
            Self::ProductUx => "claude-opus-4.7",
            Self::Architecture => "deepseek-v4-pro",
            Self::Operations => "glm-5.1",
        }
    }

    fn system_prompt(self) -> &'static str {
        match self {
            Self::ProductUx => "Atue como LensA_ProductUX. Foque em neuro-inclusao, mitigacao de Flow-Debt e valor de produto. Responda estritamente em JSON com as chaves lens_id, repo_id, bullets, risk_level, recommendation. bullets deve conter de 3 a 5 itens curtos e factuais. Nenhuma prosa fora do JSON.",
            Self::Architecture => "Atue como LensB_Architecture. Foque em alma matematica, extraibilidade O(1) e sobrevivencia bare-metal na RTX 2060m. Responda estritamente em JSON com as chaves lens_id, repo_id, bullets, risk_level, recommendation. bullets deve conter de 3 a 5 itens curtos e factuais. Nenhuma prosa fora do JSON.",
            Self::Operations => "Atue como LensC_Operations. Audite lixo toxico, entropia temporal e risco FinOps. Responda estritamente em JSON com as chaves lens_id, repo_id, bullets, risk_level, recommendation. bullets deve conter de 3 a 5 itens curtos e factuais. Nenhuma prosa fora do JSON.",
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Phase2Error {
    #[error("Repositorio invalido: {0}")]
    InvalidRepoId(String),
    #[error("Falha ao buscar payloads da Fase 1.5: {0}")]
    PayloadFetchError(String),
    #[error("Falha de configuracao da Fase 2: {0}")]
    ConfigError(String),
    #[error("Pacote ausente ou vazio: {0}")]
    EmptyPackage(String),
    #[error("Falha na lente {lens}: {message}")]
    LensExecutionError { lens: String, message: String },
    #[error("Falha ao persistir debates: {0}")]
    PersistError(String),
    #[error("Repositorio marcado como erro na Fase 2: {0}")]
    Phase2Aborted(String),
}

pub trait DebateStore: Send + Sync {
    fn fetch_phase2_payloads(&self, repo_id: &str) -> Result<Phase2Payloads, String>;
    fn persist_debate(&self, repo_id: &str, debate: &SwarmDebate) -> Result<(), String>;
    fn mark_phase2_error(&self, repo_id: &str) -> Result<(), String>;
}

pub trait LensInvoker: Send + Sync {
    fn invoke<'a>(&'a self, lens: LensKind, repo_id: &'a str, payload: &'a str) -> LensFuture<'a>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LensDebatePayload {
    #[serde(default)]
    pub lens_id: String,
    #[serde(default)]
    pub repo_id: String,
    #[serde(default)]
    pub bullets: Vec<String>,
    #[serde(default)]
    pub risk_level: String,
    #[serde(default)]
    pub recommendation: String,
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

        let debate = match (lens_a, lens_b, lens_c) {
            (Ok(lente_a), Ok(lente_b), Ok(lente_c)) => SwarmDebate {
                repo_id: repo_id.to_string(),
                lente_a,
                lente_b,
                lente_c,
            },
            (a_res, b_res, c_res) => {
                let root_error = a_res
                    .err()
                    .or_else(|| b_res.err())
                    .or_else(|| c_res.err())
                    .expect("at least one lens must have failed");
                self.store
                    .mark_phase2_error(repo_id)
                    .map_err(Phase2Error::Phase2Aborted)?;
                return Err(root_error);
            }
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
        let mut last_error = String::new();

        for _attempt in 1..=3 {
            match self.invoker.invoke(lens, repo_id, payload).await {
                Ok(result) => return Ok(result),
                Err(err) => last_error = err,
            }
        }

        Err(Phase2Error::LensExecutionError {
            lens: lens.lens_id().to_string(),
            message: last_error,
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
}

impl HttpLensInvoker {
    pub fn from_env() -> Result<Self, Phase2Error> {
        Ok(Self {
            client: Client::new(),
            claude: HttpLensConfig {
                base_url: std::env::var("PHASE2_CLAUDE_URL")
                    .map_err(|_| Phase2Error::ConfigError("PHASE2_CLAUDE_URL ausente".to_string()))?,
                api_key: std::env::var("PHASE2_CLAUDE_API_KEY")
                    .map_err(|_| Phase2Error::ConfigError("PHASE2_CLAUDE_API_KEY ausente".to_string()))?,
                model: LensKind::ProductUx.model_name().to_string(),
            },
            deepseek: HttpLensConfig {
                base_url: std::env::var("PHASE2_DEEPSEEK_URL")
                    .map_err(|_| Phase2Error::ConfigError("PHASE2_DEEPSEEK_URL ausente".to_string()))?,
                api_key: std::env::var("PHASE2_DEEPSEEK_API_KEY")
                    .map_err(|_| Phase2Error::ConfigError("PHASE2_DEEPSEEK_API_KEY ausente".to_string()))?,
                model: LensKind::Architecture.model_name().to_string(),
            },
            glm: HttpLensConfig {
                base_url: std::env::var("PHASE2_GLM_URL")
                    .map_err(|_| Phase2Error::ConfigError("PHASE2_GLM_URL ausente".to_string()))?,
                api_key: std::env::var("PHASE2_GLM_API_KEY")
                    .map_err(|_| Phase2Error::ConfigError("PHASE2_GLM_API_KEY ausente".to_string()))?,
                model: LensKind::Operations.model_name().to_string(),
            },
        })
    }

    pub fn from_openrouter_env() -> Result<Self, Phase2Error> {
        let api_key = get_first_env(&[
            "OPENROUTER_API_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
        ])
        .ok_or_else(|| {
            Phase2Error::ConfigError(
                "OPENROUTER_API_KEY/OPENROUTER_API_FAST_KEY/OPENROUTER_API_FREE_KEY ausente".to_string(),
            )
        })?;
        let base_url = std::env::var("OPENROUTER_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_OPENROUTER_URL.to_string());

        Ok(Self {
            client: Client::new(),
            claude: HttpLensConfig {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: std::env::var("PHASE2_LENS_A_MODEL")
                    .unwrap_or_else(|_| "anthropic/claude-3.5-sonnet".to_string()),
            },
            deepseek: HttpLensConfig {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: std::env::var("PHASE2_LENS_B_MODEL")
                    .unwrap_or_else(|_| "deepseek/deepseek-chat".to_string()),
            },
            glm: HttpLensConfig {
                base_url,
                api_key,
                model: std::env::var("PHASE2_LENS_C_MODEL")
                    .unwrap_or_else(|_| "google/gemini-2.5-flash".to_string()),
            },
        })
    }

    #[cfg(test)]
    fn with_configs(claude: HttpLensConfig, deepseek: HttpLensConfig, glm: HttpLensConfig) -> Self {
        Self {
            client: Client::new(),
            claude,
            deepseek,
            glm,
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
    fn invoke<'a>(&'a self, lens: LensKind, repo_id: &'a str, payload: &'a str) -> LensFuture<'a> {
        Box::pin(async move {
            let config = self.config_for(lens);
            let body = ChatCompletionsRequest {
                model: config.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: lens.system_prompt().to_string(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: format!("repo_id={}\n{}", repo_id, payload),
                    },
                ],
                max_tokens: 700,
                temperature: 0.0,
                response_format: ChatResponseFormat {
                    kind: "json_object".to_string(),
                },
            };

            let response = self
                .client
                .post(&config.base_url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(format!("HTTP {}: {}", status.as_u16(), body));
            }

            let parsed: ChatCompletionsResponse = response.json().await.map_err(|e| e.to_string())?;
            parsed
                .choices
                .first()
                .and_then(|choice| choice.message.content.clone())
                .filter(|content| !content.trim().is_empty())
                .ok_or_else(|| format!("Resposta vazia da lente {}", lens.lens_id()))
                .and_then(|content| normalize_lens_payload(lens, repo_id, &content))
        })
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionsRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: usize,
    temperature: f32,
    response_format: ChatResponseFormat,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatResponseFormat {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
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
        "lens_a_json" | "lens_b_json" | "lens_c_json" | "phase_status" => Value::Text(String::new()),
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

        let updated = tx
            .execute(
                "UPDATE debates_enxame
                 SET lens_a_json = ?2,
                     lens_b_json = ?3,
                     lens_c_json = ?4,
                     phase_status = ?5
                 WHERE repo_id = ?1",
                params![repo_id, debate.lente_a, debate.lente_b, debate.lente_c, STATUS_OK],
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

fn normalize_lens_payload(lens: LensKind, repo_id: &str, raw: &str) -> Result<String, String> {
    let candidate = extract_json_object(raw).unwrap_or_else(|| raw.trim().to_string());
    let mut parsed: LensDebatePayload = match serde_json::from_str(&candidate) {
        Ok(parsed) => parsed,
        Err(json_err) => extract_bullets_fallback(raw)
            .map(|bullets| LensDebatePayload {
                lens_id: lens.lens_id().to_string(),
                repo_id: repo_id.to_string(),
                bullets,
                risk_level: "medium".to_string(),
                recommendation: "manual-review".to_string(),
            })
            .ok_or_else(|| format!("JSON invalido da lente {}: {}", lens.lens_id(), json_err))?,
    };

    if parsed.lens_id.trim().is_empty() {
        parsed.lens_id = lens.lens_id().to_string();
    }
    if parsed.repo_id.trim().is_empty() {
        parsed.repo_id = repo_id.to_string();
    }

    parsed.bullets = parsed
        .bullets
        .into_iter()
        .map(|bullet| bullet.trim().to_string())
        .filter(|bullet| !bullet.is_empty())
        .collect();

    if parsed.bullets.len() < 3 || parsed.bullets.len() > 5 {
        return Err(format!(
            "Mini-JSON invalido da lente {}: esperado 3 a 5 bullets, recebido {}",
            lens.lens_id(),
            parsed.bullets.len()
        ));
    }

    if parsed.risk_level.trim().is_empty() {
        parsed.risk_level = "medium".to_string();
    }
    if parsed.recommendation.trim().is_empty() {
        parsed.recommendation = "refine".to_string();
    }

    serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("Falha ao serializar JSON canonico da lente {}: {}", lens.lens_id(), e))
}

fn extract_json_object(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    (end >= start).then(|| trimmed[start..=end].to_string())
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
        results: HashMap<LensKind, Result<String, String>>,
        payloads: Arc<Mutex<Vec<(LensKind, String)>>>,
        attempts: Arc<Mutex<HashMap<LensKind, usize>>>,
    }

    impl RecordingLensInvoker {
        fn new(
            delays_ms: HashMap<LensKind, u64>,
            results: HashMap<LensKind, Result<String, String>>,
        ) -> Self {
            Self {
                delays_ms,
                results,
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
        fn invoke<'a>(&'a self, lens: LensKind, _repo_id: &'a str, payload: &'a str) -> LensFuture<'a> {
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

                self.results
                    .get(&lens)
                    .cloned()
                    .unwrap_or_else(|| Ok(default_json(lens)))
            })
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

    fn default_json(lens: LensKind) -> String {
        format!(
            "{{\"lens_id\":\"{}\",\"repo_id\":\"repo\",\"bullets\":[\"ok-1\",\"ok-2\",\"ok-3\"],\"risk_level\":\"low\",\"recommendation\":\"keep\"}}",
            lens.lens_id()
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

        let (a, b, c, status): (String, String, String, String) = conn
            .query_row(
                "SELECT lens_a_json, lens_b_json, lens_c_json, phase_status
                 FROM debates_enxame
                 WHERE repo_id = ?1",
                params!["repo/test"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("query migrated row");

        assert_eq!(a, "{\"a\":1}");
        assert_eq!(b, "{\"b\":1}");
        assert_eq!(c, "{\"c\":1}");
        assert_eq!(status, "PENDING");
    }

    #[test]
    fn test_normalize_lens_payload_extracts_json_from_code_fence() {
        let raw = "```json\n{\"lens_id\":\"LensA_ProductUX\",\"repo_id\":\"repo/test\",\"bullets\":[\"a\",\"b\",\"c\"],\"risk_level\":\"low\",\"recommendation\":\"keep\"}\n```";
        let normalized =
            normalize_lens_payload(LensKind::ProductUx, "repo/test", raw).expect("normalized json");
        let parsed: LensDebatePayload = serde_json::from_str(&normalized).expect("parsed canonical json");

        assert_eq!(parsed.lens_id, "LensA_ProductUX");
        assert_eq!(parsed.repo_id, "repo/test");
        assert_eq!(parsed.bullets.len(), 3);
    }

    #[test]
    fn test_normalize_lens_payload_falls_back_to_plain_bullets() {
        let raw = "1. Primeiro achado\n2. Segundo achado\n3. Terceiro achado";
        let normalized =
            normalize_lens_payload(LensKind::ProductUx, "repo/test", raw).expect("normalized fallback");
        let parsed: LensDebatePayload = serde_json::from_str(&normalized).expect("parsed fallback json");

        assert_eq!(parsed.lens_id, "LensA_ProductUX");
        assert_eq!(parsed.repo_id, "repo/test");
        assert_eq!(parsed.bullets.len(), 3);
        assert_eq!(parsed.recommendation, "manual-review");
    }

    #[test]
    fn test_from_env_missing_var_returns_config_error() {
        let env_keys = [
            "PHASE2_CLAUDE_URL",
            "PHASE2_CLAUDE_API_KEY",
            "PHASE2_DEEPSEEK_URL",
            "PHASE2_DEEPSEEK_API_KEY",
            "PHASE2_GLM_URL",
            "PHASE2_GLM_API_KEY",
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
            Err(Phase2Error::ConfigError(message)) if message == "PHASE2_CLAUDE_URL ausente"
        ));
    }

    #[test]
    fn test_from_openrouter_env_missing_key_returns_config_error() {
        let env_keys = [
            "OPENROUTER_API_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
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
                (LensKind::ProductUx, Ok(default_json(LensKind::ProductUx))),
                (LensKind::Architecture, Ok(default_json(LensKind::Architecture))),
                (LensKind::Operations, Ok(default_json(LensKind::Operations))),
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
    async fn test_fail_fast_aborts_after_third_429_and_keeps_debate_table_empty() {
        let mut claude_server = Server::new_async().await;
        let mut deepseek_server = Server::new_async().await;
        let mut glm_server = Server::new_async().await;

        let claude_mock = claude_server
            .mock("POST", "/claude")
            .match_body(Matcher::Regex("PACKAGE_A_ONLY".to_string()))
            .with_status(429)
            .with_body(r#"{"error":"rate limit"}"#)
            .expect(3)
            .create();

        let deepseek_mock = deepseek_server
            .mock("POST", "/deepseek")
            .match_body(Matcher::Regex("PACKAGE_B_ONLY".to_string()))
            .with_status(200)
            .with_body(success_body(LensKind::Architecture))
            .expect(1)
            .create();

        let glm_mock = glm_server
            .mock("POST", "/glm")
            .match_body(Matcher::Regex("PACKAGE_C_ONLY".to_string()))
            .with_status(200)
            .with_body(success_body(LensKind::Operations))
            .expect(1)
            .create();

        let store_conn = create_test_db();
        let store = SqliteDebateStore::new(Arc::clone(&store_conn));
        let invoker = HttpLensInvoker::with_configs(
            HttpLensConfig {
                base_url: format!("{}/claude", claude_server.url()),
                api_key: "test".to_string(),
                model: "claude-opus-4.7".to_string(),
            },
            HttpLensConfig {
                base_url: format!("{}/deepseek", deepseek_server.url()),
                api_key: "test".to_string(),
                model: "deepseek-v4-pro".to_string(),
            },
            HttpLensConfig {
                base_url: format!("{}/glm", glm_server.url()),
                api_key: "test".to_string(),
                model: "glm-5.1".to_string(),
            },
        );
        let dispatcher = CognitiveSwarmDispatcher::new(store, invoker);

        let result = dispatcher.dispatch_swarm("repo/test").await;

        claude_mock.assert();
        deepseek_mock.assert();
        glm_mock.assert();

        assert!(matches!(
            result,
            Err(Phase2Error::LensExecutionError { ref lens, .. }) if lens == "LensA_ProductUX"
        ));

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

    fn success_body(lens: LensKind) -> String {
        serde_json::json!({
            "choices": [{
                "message": {
                    "content": default_json(lens)
                }
            }]
        })
        .to_string()
    }
}
