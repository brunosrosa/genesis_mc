use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io;
use std::path::{Path, PathBuf};
use tracing::error;
use url::Url;

use genesis_mc_lib::cognition::synthesizer::ArchitecturalCategory;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BatedorResult {
    proposta_original_resumo: String,
    categoria_arquitetural: ArchitecturalCategory,
    status_atualizacao: String,
    status_fase: String,
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn vault_db_path() -> io::Result<PathBuf> {
    Ok(workspace_root()?.join(".soda_data").join("soda_heuristic_vault.db"))
}

fn repo_id_from_repo_url(repo_url: &str) -> Result<String, String> {
    let url = Url::parse(repo_url).map_err(|e| format!("repo_url inválida: {e}"))?;
    let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
    if url.host_str() != Some("github.com") && !allow_host_override {
        return Err("repo_url não é GitHub (host != github.com)".to_string());
    }
    let mut segments = url
        .path_segments()
        .ok_or_else(|| "repo_url sem path segments".to_string())?;
    let owner = segments
        .next()
        .ok_or_else(|| "repo_url sem owner".to_string())?
        .to_string();
    let repo = segments
        .next()
        .ok_or_else(|| "repo_url sem repo".to_string())?
        .to_string();
    if owner.trim().is_empty() || repo.trim().is_empty() {
        return Err("repo_url inválida (owner/repo vazio)".to_string());
    }
    Ok(format!("{owner}/{repo}"))
}

fn load_blob10_soda_canon_context(repo_id: &str) -> Result<String, String> {
    let db_path = vault_db_path().map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    let result: Result<String, _> = conn.query_row(
        "SELECT cast(payload_blob as text) FROM artefatos_brutos WHERE artifact_type = 'blob_10_soda_canon_context' AND repo_id = ?1 LIMIT 1",
        [repo_id],
        |row| row.get(0),
    );
    result.map_err(|e| format!("Canon context ausente (blob_10): {e}"))
}

fn truncate_readme_3000(input: &str) -> String {
    for (count, (idx, _)) in input.char_indices().enumerate() {
        if count == 3000 {
            return input[..idx].to_string();
        }
    }
    input.to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct BatedorOutputRaw {
    proposta_original_resumo: String,
    categoria_arquitetural: String,
}

fn strict_object(properties: Vec<(String, Value)>, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties.into_iter().collect::<serde_json::Map<String, Value>>(),
        "required": required
    })
}

fn response_format_for_batedor() -> Value {
    let schema = strict_object(
        vec![
            (
                "proposta_original_resumo".to_string(),
                json!({ "type": "string" }),
            ),
            (
                "categoria_arquitetural".to_string(),
                json!({ "type": "string" }),
            ),
        ],
        vec!["proposta_original_resumo", "categoria_arquitetural"],
    );
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "soda_f_minus_0_5_batedor",
            "strict": true,
            "schema": schema
        }
    })
}

fn build_batedor_prompt(repo_url: &str, readme_trunc: &str, blob10: &str) -> String {
    format!(
        "SODA_PHASE=-0.5\nrepo_url={}\n\nREADME_TRUNC_3000:\n{}\n\nBLOB_10_CANON_CONTEXT:\n{}\n\nTarefa: Responda APENAS com JSON estrito contendo:\n- proposta_original_resumo: string (curto, técnico, neutro)\n- categoria_arquitetural: um dentre [CanvasUI, UILibrary, Memoria_RAG, Roteamento_FinOps, Orquestracao_Agentes, Model_Serving, Knowledge_Extraction, Seguranca_Sandbox, Infraestrutura_Core, Tooling_Dev]\n",
        repo_url, readme_trunc, blob10
    )
}

struct OpenRouterClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenRouterClient {
    fn new() -> Result<Self, String> {
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
        let api_key = std::env::var("OPENROUTER_API_HEAVY_KEY")
            .ok()
            .map(|v| v.trim().trim_matches('"').to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "Missing OPENROUTER_API_HEAVY_KEY".to_string())?;
        let model = std::env::var("OPENROUTER_BATEDOR_MODEL")
            .ok()
            .map(|v| v.trim().trim_matches('"').to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "deepseek/deepseek-v4".to_string());
        Ok(Self {
            client: Client::new(),
            base_url,
            api_key,
            model,
        })
    }

    fn openrouter_body_for_batedor(&self, prompt: &str) -> Value {
        json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "Responda SOMENTE com JSON válido (sem markdown, sem texto extra)."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.0,
            "max_tokens": 512,
            "response_format": response_format_for_batedor()
        })
    }

    fn extract_openrouter_content(json: &Value) -> Option<String> {
        let content = json
            .get("choices")?
            .as_array()?
            .first()?
            .get("message")?
            .get("content")?;
        match content {
            Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
            _ => None,
        }
    }

    async fn run_batedor(&self, prompt: &str) -> Result<BatedorResult, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = self.openrouter_body_for_batedor(prompt);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Erro de rede: {e}"))?;

        let status = resp.status();
        let raw = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {}: {}", status.as_u16(), raw));
        }
        let envelope: Value =
            serde_json::from_str(&raw).map_err(|e| format!("Envelope JSON inválido do OpenRouter: {e}"))?;
        let content =
            Self::extract_openrouter_content(&envelope).ok_or_else(|| "Resposta vazia do OpenRouter".to_string())?;
        parse_batedor_output(&content)
    }
}

struct GithubReadmeClient {
    http: Client,
    api_base: String,
    allow_host_override: bool,
    github_pat: String,
}

impl GithubReadmeClient {
    fn new() -> Result<Self, String> {
        let api_base = std::env::var("SODA_GITHUB_API_BASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "https://api.github.com".to_string());
        let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
        let github_pat = std::env::var("GITHUB_PAT")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "Missing GITHUB_PAT".to_string())?;
        let http = Client::builder()
            .user_agent("f-minus-0.5-batedor/1.0")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            http,
            api_base,
            allow_host_override,
            github_pat,
        })
    }

    async fn fetch_readme_raw(&self, repo_url: &str) -> Result<String, String> {
        let url = Url::parse(repo_url).map_err(|e| format!("repo_url inválida: {e}"))?;
        if url.host_str() != Some("github.com") && !self.allow_host_override {
            return Err("repo_url não é GitHub (host != github.com)".to_string());
        }
        let repo_id = repo_id_from_repo_url(repo_url)?;
        let endpoint = format!(
            "{}/repos/{}/readme",
            self.api_base.trim_end_matches('/'),
            repo_id
        );

        let resp = self
            .http
            .get(&endpoint)
            .bearer_auth(&self.github_pat)
            .header("Accept", "application/vnd.github.raw")
            .send()
            .await
            .map_err(|e| format!("Falha HTTP GitHub: {e}"))?;

        let status = resp.status();
        let raw = resp.text().await.map_err(|e| e.to_string())?;
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(format!("GitHub rate limit (status {})", status.as_u16()));
        }
        if !status.is_success() {
            return Err(format!("GitHub HTTP {}: {}", status.as_u16(), raw));
        }
        Ok(raw)
    }
}

fn parse_batedor_output(raw_json: &str) -> Result<BatedorResult, String> {
    let parsed: BatedorOutputRaw = serde_json::from_str(raw_json).map_err(|e| e.to_string())?;
    let categoria = ArchitecturalCategory::parse_strict(&parsed.categoria_arquitetural)?;
    if categoria == ArchitecturalCategory::Unspecified {
        return Err("categoria_arquitetural invalida: ''".to_string());
    }
    Ok(BatedorResult {
        proposta_original_resumo: parsed.proposta_original_resumo,
        categoria_arquitetural: categoria,
        status_atualizacao: "TRIAGEM_CONCLUIDA".to_string(),
        status_fase: "FASE_-0.5_BATEDOR_OK".to_string(),
    })
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let repo_url = std::env::var("REPO_URL").map_err(|_| io::Error::other("Missing REPO_URL"))?;
    let repo_id = repo_id_from_repo_url(&repo_url).map_err(io::Error::other)?;

    let github = GithubReadmeClient::new().map_err(io::Error::other)?;
    let readme = github.fetch_readme_raw(&repo_url).await.map_err(io::Error::other)?;
    let readme_trunc = truncate_readme_3000(&readme);

    let blob10 = match load_blob10_soda_canon_context(&repo_id) {
        Ok(v) => v,
        Err(e) => {
            error!(error = %e, repo_id = %repo_id, "Falha ao carregar blob_10");
            return Err(io::Error::other(e));
        }
    };

    let prompt = build_batedor_prompt(&repo_url, &readme_trunc, &blob10);
    let llm = OpenRouterClient::new().map_err(io::Error::other)?;
    let out = llm.run_batedor(&prompt).await.map_err(io::Error::other)?;

    let payload = json!({
        "proposta_original_resumo": out.proposta_original_resumo,
        "categoria_arquitetural": out.categoria_arquitetural.as_str(),
        "status_atualizacao": out.status_atualizacao,
        "status_fase": out.status_fase
    });
    println!("{}", serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_readme_to_3000_chars() {
        let input = "a".repeat(4000);
        let out = truncate_readme_3000(&input);
        assert_eq!(out.len(), 3000);

        let small = "b".repeat(12);
        let out2 = truncate_readme_3000(&small);
        assert_eq!(out2, small);
    }

    #[test]
    fn rejects_category_outside_10_enums() {
        let raw = r#"{
            "proposta_original_resumo": "x",
            "categoria_arquitetural": "Inventada"
        }"#;

        let err = parse_batedor_output(raw).err().unwrap();
        assert!(err.contains("categoria_arquitetural"));
    }

    #[test]
    fn success_updates_statuses_to_triagem_concluida_and_fase_meio_ok() {
        let raw = r#"{
            "proposta_original_resumo": "Resumo",
            "categoria_arquitetural": "CanvasUI"
        }"#;
        let out = parse_batedor_output(raw).unwrap();
        assert_eq!(out.status_atualizacao, "TRIAGEM_CONCLUIDA");
        assert_eq!(out.status_fase, "FASE_-0.5_BATEDOR_OK");
        assert_eq!(out.categoria_arquitetural, ArchitecturalCategory::CanvasUi);
        assert_eq!(out.proposta_original_resumo, "Resumo");
    }
}
