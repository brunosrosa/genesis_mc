use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{info, warn};
use url::Url;

use souls_mc_lib::telemetry::{enable_virtual_terminal, init_cli_tracing, parse_log_level_from_env};

const README_CHAR_LIMIT: usize = 3_000;
const MAX_RESUMO_CHARS: usize = 800;

const ALLOWED_CATEGORIA_ARQUITETURAL: [&str; 47] = [
    "AI_Research - Foundation_Model",
    "CanvasUI - Core_Pattern",
    "CanvasUI - Domain_App",
    "CanvasUI - Ops_Dashboard",
    "CanvasUI - Terminal_Workspace",
    "Comms_Social - Platform_Client",
    "Domain_App - Self_Hosted",
    "Infraestrutura_Core - Concurrency_OS",
    "Infraestrutura_Core - Data_Pipeline",
    "Infraestrutura_Core - Data_Serialization",
    "Infraestrutura_Core - Hardware_Ops",
    "Knowledge_Extraction - Doc_Parsing",
    "Knowledge_Extraction - Generic",
    "Knowledge_Extraction - Multimedia_Parsing",
    "Knowledge_Extraction - Semantic_Mining",
    "Knowledge_Extraction - Web_Scraping",
    "Memoria_RAG - Graph_Store",
    "Memoria_RAG - Relational_Episodic",
    "Memoria_RAG - Vector_Store",
    "Model_Serving - Edge_Deployment",
    "Model_Serving - Inference_Engine",
    "Model_Serving - Resource_Scheduler",
    "Model_Serving - Training_FineTuning",
    "Orquestracao_Agentes - Dev_Framework",
    "Orquestracao_Agentes - OS_Runtime",
    "Orquestracao_Agentes - Simulation_Environment",
    "Orquestracao_Agentes - Skill_Library",
    "Orquestracao_Agentes - Specialized_Worker",
    "Orquestracao_Agentes - Workflow_DAG",
    "Roteamento_FinOps - API_Gateway",
    "Roteamento_FinOps - Cost_Analytics",
    "Roteamento_FinOps - Network_Tunnel",
    "Roteamento_FinOps - Prompt_Caching",
    "Seguranca_Sandbox - Auth_Crypto",
    "Seguranca_Sandbox - MicroVM_Container",
    "Seguranca_Sandbox - Privacy_Governance",
    "Seguranca_Sandbox - Runtime_Isolation",
    "Tooling_Dev - CLI_Utilities",
    "Tooling_Dev - Knowledge_Curation",
    "Tooling_Dev - MCP_Bridging",
    "Tooling_Dev - Observability_Eval",
    "Tooling_Dev - Prompt_Knowledge",
    "UILibrary - Animation_Graphics",
    "UILibrary - Component_System",
    "UILibrary - Generative_UI",
    "UILibrary - Terminal_TUI",
    "Outros - Uncategorized",
];

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BatedorOut {
    pub proposta_original_resumo: String,
    pub categoria_arquitetural: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTriageRepo {
    pub project_name: String,
    pub repo_url: String,
}

trait TriageLlmClient: Send + Sync {
    fn triage<'a>(
        &'a self,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<BatedorOut, String>> + Send + 'a>>;
}

trait ReadmeFetcher: Send + Sync {
    fn fetch_readme_truncated<'a>(
        &'a self,
        repo_url: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

struct GithubReadmeFetcher;

impl ReadmeFetcher for GithubReadmeFetcher {
    fn fetch_readme_truncated<'a>(
        &'a self,
        repo_url: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            fetch_readme_truncated(repo_url).await
        })
    }
}

trait BatedorRepoStore: Send + Sync {
    fn fetch_pending_triage_repos<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PendingTriageRepo>, String>> + Send + 'a>>;

    fn persist_triage_result<'a>(
        &'a self,
        project_name: &'a str,
        repo_url: &'a str,
        proposta_resumo: &'a str,
        categoria: &'a str,
        new_status: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

struct SqliteBatedorRepoStore {
    db_path: PathBuf,
}

impl SqliteBatedorRepoStore {
    fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    input.chars().take(max_chars).collect()
}

fn try_extract_owner_repo_from_repo_url(repo_url: &str) -> Option<(String, String)> {
    let url = Url::parse(repo_url).ok()?;
    if !url.host_str()?.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let mut parts = url.path().trim_matches('/').split('/');
    let owner = parts.next()?.trim().to_string();
    let repo = parts.next()?.trim().to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

fn sanitize_env_scalar(raw: &str) -> String {
    let mut v = raw.trim().trim_matches('"').trim_matches('\'').trim().to_string();
    if let Some(hash) = v.find('#') {
        v.truncate(hash);
        v = v.trim().to_string();
    }
    if let Some(first) = v.split_whitespace().next() {
        first.trim().to_string()
    } else {
        String::new()
    }
}

fn validate_batedor_out(out: &BatedorOut) -> Result<(), String> {
    let resumo = out.proposta_original_resumo.trim();
    if resumo.is_empty() {
        return Err("proposta_original_resumo vazio".to_string());
    }
    if resumo.chars().count() > MAX_RESUMO_CHARS {
        return Err(format!(
            "proposta_original_resumo excede limite de {} chars",
            MAX_RESUMO_CHARS
        ));
    }
    let cat = out.categoria_arquitetural.trim();
    if cat.is_empty() {
        return Err("categoria_arquitetural inválida (vazia)".to_string());
    }
    if !ALLOWED_CATEGORIA_ARQUITETURAL.iter().any(|v| v == &cat) {
        return Err("categoria_arquitetural inválida (fora do ENUM)".to_string());
    }
    Ok(())
}

impl BatedorRepoStore for SqliteBatedorRepoStore {
    fn fetch_pending_triage_repos<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PendingTriageRepo>, String>> + Send + 'a>> {
        let db_path = self.db_path.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || -> Result<Vec<PendingTriageRepo>, String> {
                let conn = Connection::open(&db_path)
                    .map_err(|e| format!("Batedor: falha ao abrir SQLite: {}", e))?;
                
                let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN proposta_original_resumo TEXT", []);
                let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN categoria_arquitetural TEXT", []);

                let mut stmt = conn
                    .prepare(
                        "SELECT project_name, repo_url
                         FROM repositorios
                         WHERE status_processamento IN ('PENDENTE_TRIAGEM', 'INICIAR_TRIAGEM')",
                    )
                    .map_err(|e| format!("Batedor: erro na query: {e}"))?;
                
                let rows = stmt
                    .query_map([], |row| {
                        Ok(PendingTriageRepo {
                            project_name: row.get(0)?,
                            repo_url: row.get(1)?,
                        })
                    })
                    .map_err(|e| format!("Batedor: erro na query: {e}"))?;
                
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

    fn persist_triage_result<'a>(
        &'a self,
        project_name: &'a str,
        repo_url: &'a str,
        proposta_resumo: &'a str,
        categoria: &'a str,
        new_status: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        let db_path = self.db_path.clone();
        let project_name = project_name.trim().to_string();
        let repo_url = repo_url.trim().to_string();
        let proposta_resumo = proposta_resumo.trim().to_string();
        let categoria = categoria.trim().to_string();
        let new_status = new_status.trim().to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || -> Result<(), String> {
                let conn = Connection::open(&db_path)
                    .map_err(|e| format!("Batedor: falha ao abrir SQLite: {}", e))?;
                
                let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN proposta_original_resumo TEXT", []);
                let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN categoria_arquitetural TEXT", []);

                let repo_key = if !project_name.is_empty() {
                    project_name
                } else {
                    try_extract_owner_repo_from_repo_url(&repo_url)
                        .map(|(o, r)| format!("{o}/{r}"))
                        .unwrap_or_default()
                };

                let updated_rows = conn
                    .execute(
                        "UPDATE repositorios
                         SET proposta_original_resumo = ?1,
                             categoria_arquitetural = ?2,
                             status_processamento = ?3,
                             retry_count = 0
                         WHERE project_name = ?4 OR repo_url = ?5",
                        params![proposta_resumo, categoria, new_status, repo_key, repo_url],
                    )
                    .map_err(|e| format!("Batedor: falha ao persistir triagem no SQLite: {e}"))?;

                if updated_rows == 0 {
                    return Err(format!(
                        "Batedor: nenhuma linha atualizada para project_name='{}' repo_url='{}'",
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

// =========================================================
// WATERFALL ROUTING CLIENT WITH CIRCUIT BREAKER (FINOPS)
// =========================================================

struct WaterfallRoutingClient {
    client: Client,
    google_api_key: Option<String>,
    google_model: String,
    openrouter_free_key: Option<String>,
    openrouter_free_model: String,
    openrouter_fast_key: Option<String>,
    openrouter_fast_model: String,
    openrouter_base_url: String,
}

impl WaterfallRoutingClient {
    fn new() -> Result<Self, String> {
        let google_api_key = std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_FREE_KEY"))
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty());

        let google_model = std::env::var("GOOGLE_MODEL_FAST")
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "gemini-flash-latest".to_string());

        let openrouter_free_key = std::env::var("OPENROUTER_API_FREE_KEY")
            .or_else(|_| std::env::var("OPENROUTER_API_FAST_KEY"))
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty());

        let openrouter_free_model = std::env::var("OPENROUTER_FREE_MODEL")
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "openrouter/free".to_string());

        let openrouter_fast_key = std::env::var("OPENROUTER_API_FAST_KEY")
            .or_else(|_| std::env::var("OPENROUTER_API_HEAVY_KEY"))
            .or_else(|_| std::env::var("OPENROUTER_API_FREE_KEY"))
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty());

        let openrouter_fast_model = std::env::var("OPENROUTER_DEFAULT_MODEL")
            .or_else(|_| std::env::var("OPENROUTER_BATEDOR_MODEL"))
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "deepseek/deepseek-v4-flash".to_string());

        let openrouter_base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|v| sanitize_env_scalar(&v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());

        Ok(Self {
            client: Client::new(),
            google_api_key,
            google_model,
            openrouter_free_key,
            openrouter_free_model,
            openrouter_fast_key,
            openrouter_fast_model,
            openrouter_base_url,
        })
    }

    async fn try_google_native(&self, prompt: &str) -> Result<BatedorOut, String> {
        let Some(key) = &self.google_api_key else {
            return Err("Google API Key ausente".to_string());
        };
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.google_model, key
        );

        let allowed = ALLOWED_CATEGORIA_ARQUITETURAL.to_vec();
        let body = json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }],
            "generationConfig": {
                "temperature": 0.0,
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "OBJECT",
                    "properties": {
                        "proposta_original_resumo": {
                            "type": "STRING",
                            "description": "Resumo técnico de 1 frase"
                        },
                        "categoria_arquitetural": {
                            "type": "STRING",
                            "enum": allowed
                        }
                    },
                    "required": ["proposta_original_resumo", "categoria_arquitetural"]
                }
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(Duration::from_secs(35))
            .send()
            .await
            .map_err(|e| format!("Google API HTTP erro: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("Google API HTTP {}", status.as_u16()));
        }

        let val: Value = resp
            .json()
            .await
            .map_err(|e| format!("Google API JSON parse erro: {e}"))?;

        let text = val
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Google API: resposta vazia".to_string())?;

        let parsed: BatedorOut = serde_json::from_str(text)
            .map_err(|e| format!("Google API JSON estruturado inválido: {e}"))?;
        validate_batedor_out(&parsed)?;
        Ok(parsed)
    }

    async fn try_openrouter(&self, api_key: Option<&str>, model: &str, prompt: &str) -> Result<BatedorOut, String> {
        let Some(key) = api_key else {
            return Err("OpenRouter API Key ausente".to_string());
        };
        let url = format!("{}/chat/completions", self.openrouter_base_url.trim_end_matches('/'));
        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": "Responda SOMENTE com JSON válido."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.0,
            "response_format": response_format_for_batedor()
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(key)
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(Duration::from_secs(35))
            .send()
            .await
            .map_err(|e| format!("OpenRouter HTTP erro: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("OpenRouter HTTP {}", status.as_u16()));
        }

        let val: Value = resp
            .json()
            .await
            .map_err(|e| format!("OpenRouter JSON parse erro: {e}"))?;

        let content = extract_openrouter_content(&val)
            .ok_or_else(|| "OpenRouter: resposta vazia".to_string())?;

        let parsed: BatedorOut = serde_json::from_str(&content)
            .map_err(|e| format!("OpenRouter JSON inválido: {e}"))?;
        validate_batedor_out(&parsed)?;
        Ok(parsed)
    }
}

fn response_format_for_batedor() -> Value {
    fn strict_object(properties: serde_json::Map<String, Value>, required: Vec<&'static str>) -> Value {
        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": required,
            "additionalProperties": false
        })
    }

    let mut props = serde_json::Map::new();
    props.insert(
        "proposta_original_resumo".to_string(),
        json!({ "type": "string", "minLength": 10, "maxLength": MAX_RESUMO_CHARS }),
    );
    props.insert(
        "categoria_arquitetural".to_string(),
        json!({
            "type": "string",
            "enum": ALLOWED_CATEGORIA_ARQUITETURAL.to_vec()
        }),
    );

    let schema = strict_object(props, vec!["proposta_original_resumo", "categoria_arquitetural"]);
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "soda_batedor_triage_v1",
            "strict": true,
            "schema": schema
        }
    })
}

fn extract_openrouter_content(json_val: &Value) -> Option<String> {
    let choices = json_val.get("choices")?.as_array()?;
    let first = choices.first()?;
    let message = first.get("message")?;
    let content = message.get("content")?;

    match content {
        Value::String(s) => Some(s.trim().to_string()),
        Value::Array(parts) => {
            let mut out = String::new();
            for part in parts {
                if let Some(t) = part
                    .as_str()
                    .or_else(|| part.get("text").and_then(|v| v.as_str()))
                    .or_else(|| part.get("content").and_then(|v| v.as_str()))
                {
                    let t = t.trim();
                    if t.is_empty() {
                        continue;
                    }
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(t);
                }
            }
            Some(out).filter(|s| !s.trim().is_empty())
        }
        _ => None,
    }
}

impl TriageLlmClient for WaterfallRoutingClient {
    fn triage<'a>(
        &'a self,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<BatedorOut, String>> + Send + 'a>> {
        Box::pin(async move {
            // ROTA 1: Google API Nativa (gemini-flash-latest)
            match self.try_google_native(prompt).await {
                Ok(out) => {
                    info!("Waterfall Routing: Rota 1 (Google Native) OK");
                    return Ok(out);
                }
                Err(e) => {
                    warn!(error = %e, "Circuit Breaker: Rota 1 (Google Native) falhou; tentando Rota 2 (OpenRouter Free)");
                }
            }

            // ROTA 2: OpenRouter Free (openrouter/free)
            match self.try_openrouter(self.openrouter_free_key.as_deref(), &self.openrouter_free_model, prompt).await {
                Ok(out) => {
                    info!("Waterfall Routing: Rota 2 (OpenRouter Free) OK");
                    return Ok(out);
                }
                Err(e) => {
                    warn!(error = %e, "Circuit Breaker: Rota 2 (OpenRouter Free) falhou; tentando Rota 3 (OpenRouter Fast Fallback)");
                }
            }

            // ROTA 3: OpenRouter Fast Fallback (deepseek-v4-flash)
            match self.try_openrouter(self.openrouter_fast_key.as_deref(), &self.openrouter_fast_model, prompt).await {
                Ok(out) => {
                    info!("Waterfall Routing: Rota 3 (OpenRouter Fast Fallback) OK");
                    return Ok(out);
                }
                Err(e) => {
                    warn!(error = %e, "Circuit Breaker: Rota 3 (OpenRouter Fast Fallback) falhou");
                    Err(format!("Todas as 3 rotas da cascata FinOps falharam: {e}"))
                }
            }
        })
    }
}

async fn fetch_readme_truncated(repo_url: &str) -> Result<String, String> {
    let (owner, repo) = try_extract_owner_repo_from_repo_url(repo_url)
        .ok_or_else(|| "repo_url não é GitHub (esperado https://github.com/<owner>/<repo>)".to_string())?;

    let api_base = std::env::var("SODA_GITHUB_API_BASE_URL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://api.github.com".to_string());
    let url = format!("{}/repos/{}/{}/readme", api_base.trim_end_matches('/'), owner, repo);

    let client = Client::new();
    let mut req = client
        .get(&url)
        .header("User-Agent", "soda-batedor")
        .header("Accept", "application/vnd.github.raw")
        .timeout(Duration::from_secs(25));
    if let Ok(token) = std::env::var("GITHUB_PAT") {
        let token = token.trim().trim_matches('"').to_string();
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("GitHub README HTTP falhou: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub README HTTP {} (repo={owner}/{repo})", resp.status().as_u16()));
    }
    let text = resp.text().await.map_err(|e| format!("GitHub README body falhou: {e}"))?;
    Ok(truncate_chars(&text, README_CHAR_LIMIT))
}

fn build_prompt(readme_trunc: &str) -> String {
    let allowed = ALLOWED_CATEGORIA_ARQUITETURAL
        .to_vec()
        .join(", ");
    let mut out = String::new();
    out.push_str("Tarefa: Resuma o repositório em 1 frase técnica, neutra e desidratada e classifique-o.\n");
    out.push_str("Responda SOMENTE com JSON válido, seguindo o schema fornecido.\n");
    out.push_str("Regras:\n");
    out.push_str("- proposta_original_resumo: 1 frase técnica, neutra, desidratada (até 800 chars).\n");
    out.push_str("- categoria_arquitetural: escolha EXATA dentre: ");
    out.push_str(&allowed);
    out.push_str(".\n");
    out.push_str("\n\nREADME (primeiros 3000 chars):\n");
    out.push_str(readme_trunc);
    out
}

struct BatedorEngine<L: TriageLlmClient, R: BatedorRepoStore, F: ReadmeFetcher> {
    llm: Arc<L>,
    repo_store: Arc<R>,
    readme_fetcher: Arc<F>,
}

impl<L: TriageLlmClient + 'static, R: BatedorRepoStore + 'static, F: ReadmeFetcher + 'static> BatedorEngine<L, R, F> {
    async fn run_once(&self) -> Result<(), String> {
        let pending = self.repo_store.fetch_pending_triage_repos().await?;
        if pending.is_empty() {
            info!("Batedor: nenhum repositório pendente de triagem encontrado no SQLite.");
            return Ok(());
        }

        let max_parallel = std::env::var("SODA_BATEDOR_PARALLEL")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(3)
            .max(1);
        let semaphore = Arc::new(Semaphore::new(max_parallel));

        let mut join_set = JoinSet::new();

        for ctx in pending {
            let sem = Arc::clone(&semaphore);
            let llm = Arc::clone(&self.llm);
            let fetcher = Arc::clone(&self.readme_fetcher);
            join_set.spawn(async move {
                let _permit = sem.acquire_owned().await.unwrap();
                let readme_res = fetcher.fetch_readme_truncated(&ctx.repo_url).await;
                let readme = match readme_res {
                    Ok(r) => r,
                    Err(e) => return (ctx, Err(e)),
                };
                let prompt = build_prompt(&readme);
                let triage_res = llm.triage(&prompt).await;
                (ctx, triage_res)
            });
        }

        let mut processed = 0usize;

        while let Some(out) = join_set.join_next().await {
            match out {
                Ok((ctx, Ok(triage))) => {
                    self.repo_store
                        .persist_triage_result(
                            &ctx.project_name,
                            &ctx.repo_url,
                            &triage.proposta_original_resumo,
                            &triage.categoria_arquitetural,
                            "PENDENTE_HARVESTER",
                        )
                        .await?;
                    processed += 1;
                    info!(repo_url = %ctx.repo_url, "Batedor: triagem concluída -> PENDENTE_HARVESTER");
                }
                Ok((ctx, Err(e))) => {
                    warn!(repo_url = %ctx.repo_url, error = %e, "Batedor: falha na triagem do repo");
                }
                Err(e) => {
                    warn!(error = ?e, "Batedor: falha de JoinHandle");
                }
            }
        }

        info!(processed, "Batedor: rodada concluída");
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

    let db_path = root_dir.join(".souls_data").join("souls_heuristic_vault.db");
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let llm = Arc::new(WaterfallRoutingClient::new().map_err(io::Error::other)?);
    let repo_store = Arc::new(SqliteBatedorRepoStore::new(db_path));
    let readme_fetcher = Arc::new(GithubReadmeFetcher);

    let engine = BatedorEngine { llm, repo_store, readme_fetcher };
    engine.run_once().await.map_err(io::Error::other)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    struct MockTriageClient {
        resumo: String,
        categoria: String,
        should_fail_route_1: bool,
    }

    impl TriageLlmClient for MockTriageClient {
        fn triage<'a>(
            &'a self,
            _prompt: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<BatedorOut, String>> + Send + 'a>> {
            let resumo = self.resumo.clone();
            let categoria = self.categoria.clone();
            let should_fail = self.should_fail_route_1;
            Box::pin(async move {
                if should_fail {
                    Err("Simulação de falha Circuit Breaker Rota 1 (429)".to_string())
                } else {
                    Ok(BatedorOut {
                        proposta_original_resumo: resumo,
                        categoria_arquitetural: categoria,
                    })
                }
            })
        }
    }

    struct MockReadmeFetcher;

    impl ReadmeFetcher for MockReadmeFetcher {
        fn fetch_readme_truncated<'a>(
            &'a self,
            _repo_url: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
            Box::pin(async move {
                Ok("# Acme Widget\nFerramenta CLI incrível para automação.".to_string())
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
                proposta_original_resumo TEXT,
                categoria_arquitetural TEXT
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[tokio::test]
    async fn test_sqlite_batedor_transition_to_pendente_harvester() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count
            ) VALUES ('acme/widget', 'L1', 'https://github.com/acme/widget', 'v1.0.0', 'v1.0.0', 'v1.0.0', 'UUID-1', 'PENDENTE_TRIAGEM', 5)",
            [],
        )
        .unwrap();
        drop(conn);

        let store = Arc::new(SqliteBatedorRepoStore::new(tmp.path().to_path_buf()));
        let llm = Arc::new(MockTriageClient {
            resumo: "Ferramenta de IA para triagem de repositórios.".to_string(),
            categoria: "Tooling_Dev - CLI_Utilities".to_string(),
            should_fail_route_1: false,
        });
        let readme_fetcher = Arc::new(MockReadmeFetcher);

        let engine = BatedorEngine {
            llm,
            repo_store: store.clone(),
            readme_fetcher,
        };

        engine.run_once().await.unwrap();

        let conn = Connection::open(tmp.path()).unwrap();
        let (status, resumo, cat, retry): (String, Option<String>, Option<String>, i32) = conn
            .query_row(
                "SELECT status_processamento, proposta_original_resumo, categoria_arquitetural, retry_count 
                 FROM repositorios WHERE project_name = 'acme/widget'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();

        assert_eq!(status, "PENDENTE_HARVESTER");
        assert_eq!(resumo, Some("Ferramenta de IA para triagem de repositórios.".to_string()));
        assert_eq!(cat, Some("Tooling_Dev - CLI_Utilities".to_string()));
        assert_eq!(retry, 0);
    }

    #[test]
    fn truncation_is_deterministic() {
        let big = "a".repeat(10_000);
        let out = truncate_chars(&big, 3_000);
        assert_eq!(out.len(), 3_000);
    }

    #[test]
    fn enum_validation_rejects_outside_catalog() {
        let out = BatedorOut {
            proposta_original_resumo: "Ferramenta que faz X.".to_string(),
            categoria_arquitetural: "QualquerCoisa".to_string(),
        };
        assert!(validate_batedor_out(&out).is_err());
    }

    #[test]
    fn json_validation_requires_fields_and_types() {
        let ok = r#"{"proposta_original_resumo":"Ferramenta CLI para triagem.","categoria_arquitetural":"Tooling_Dev - CLI_Utilities"}"#;
        let parsed: BatedorOut = serde_json::from_str(ok).unwrap();
        assert!(validate_batedor_out(&parsed).is_ok());
    }
}
