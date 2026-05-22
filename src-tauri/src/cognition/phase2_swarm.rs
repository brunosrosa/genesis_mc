use crate::cognition::sgr_synthesizer::SwarmDebate;
use crate::finops::phase1_5::package_assembler::Phase2Payloads;
use reqwest::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use thiserror::Error;

const STATUS_OK: &str = "FASE_2_OK";
const STATUS_ERR: &str = "ERRO_FASE_2";

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
            Self::ProductUx => "Atue como LensA_ProductUX. Foque em neuro-inclusao, mitigacao de Flow-Debt e valor de produto. Responda apenas em mini-JSON com 3 a 5 bullets.",
            Self::Architecture => "Atue como LensB_Architecture. Foque em alma matematica, extraibilidade O(1) e sobrevivencia bare-metal na RTX 2060m. Responda apenas em mini-JSON com 3 a 5 bullets.",
            Self::Operations => "Atue como LensC_Operations. Audite lixo toxico, entropia temporal e risco FinOps. Responda apenas em mini-JSON com 3 a 5 bullets.",
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
                max_tokens: 350,
                temperature: 0.1,
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
        })
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionsRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: usize,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
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

impl SqliteDebateStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn ensure_schema_locked(conn: &Connection) -> Result<(), String> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS debates_enxame (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id TEXT NOT NULL UNIQUE,
                lens_a_json TEXT NOT NULL,
                lens_b_json TEXT NOT NULL,
                lens_c_json TEXT NOT NULL,
                phase_status TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .map_err(|e| format!("Falha ao criar debates_enxame: {}", e))?;
        Ok(())
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

        tx.execute(
            "INSERT INTO debates_enxame
                (repo_id, lens_a_json, lens_b_json, lens_c_json, phase_status)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(repo_id) DO UPDATE SET
                lens_a_json = excluded.lens_a_json,
                lens_b_json = excluded.lens_b_json,
                lens_c_json = excluded.lens_c_json,
                phase_status = excluded.phase_status,
                created_at = datetime('now')",
            params![repo_id, debate.lente_a, debate.lente_b, debate.lente_c, STATUS_OK],
        )
        .map_err(|e| format!("Falha ao persistir debates_enxame: {}", e))?;

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
