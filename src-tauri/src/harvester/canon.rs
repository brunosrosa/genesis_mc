use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::info;

use super::persist::ArtifactBlob;

const BLOB_10_TYPE: &str = "blob_10_soda_canon_context";
pub const CANON_GLOBAL_REPO_ID: &str = "__SODA_CANON_GLOBAL__";
const CANON_NOTEBOOK_TITLE: &str = "SODA Canon V3 - Base Cristalizada";
const CANON_CACHE_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;
const CANON_QUERY_TIMEOUT_SECS: u64 = 180;
const CANON_MAX_CHARS: usize = 8_000;
const CANON_QUERY_PROMPT: &str = "Forneca o contexto canonico SODA aplicavel a Fase 1 do Harvester Genesis MC. Foque em extracao local-first, fail-fast, cache, truncagem, observabilidade, sidecars, persistencia SQLite e higiene de workspace. Responda em texto objetivo, sem markdown, com no maximo 8000 caracteres.";

#[derive(Error, Debug)]
pub enum CanonError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("NotebookLM CLI failed: {0}")]
    Cli(String),
    #[error("NotebookLM authentication expired; run `nlm login` before rerunning the harvester")]
    AuthenticationExpired,
    #[error("Canonical notebook not found: {0}")]
    NotebookNotFound(String),
    #[error("Invalid NotebookLM JSON payload: {0}")]
    Parse(String),
    #[error("Canonical context query returned empty content")]
    EmptyPayload,
}

#[derive(Debug, Clone)]
struct CanonCacheEntry {
    repo_id: String,
    payload_blob: Vec<u8>,
    timestamp_extracao: i64,
}

pub struct SodaCanonExtractor;

impl SodaCanonExtractor {
    pub async fn extract(
        repo_id: &str,
        conn: Arc<Mutex<Connection>>,
    ) -> Result<ArtifactBlob, CanonError> {
        if let Some(entry) = Self::load_cache(repo_id, Arc::clone(&conn)).await? {
            if Self::is_fresh(entry.timestamp_extracao)? {
                if entry.repo_id == CANON_GLOBAL_REPO_ID {
                    Self::persist_blob(repo_id, entry.payload_blob.clone(), Arc::clone(&conn)).await?;
                }
                info!(repo_id = %repo_id, "blob_10_soda_canon_context servido do cache SQLite");
                return Ok(ArtifactBlob {
                    artifact_type: BLOB_10_TYPE.to_string(),
                    payload_blob: entry.payload_blob,
                });
            }
        }

        let notebook_id = Self::resolve_notebook_id().await?;
        let payload_text = Self::query_canon_context(&notebook_id).await?;
        if payload_text.trim().is_empty() {
            return Err(CanonError::EmptyPayload);
        }

        let payload_blob = truncate_chars(&payload_text, CANON_MAX_CHARS).into_bytes();
        Self::persist_blob(repo_id, payload_blob.clone(), Arc::clone(&conn)).await?;
        Self::persist_blob(CANON_GLOBAL_REPO_ID, payload_blob.clone(), conn).await?;

        Ok(ArtifactBlob {
            artifact_type: BLOB_10_TYPE.to_string(),
            payload_blob,
        })
    }

    async fn load_cache(
        repo_id: &str,
        conn: Arc<Mutex<Connection>>,
    ) -> Result<Option<CanonCacheEntry>, CanonError> {
        let repo_id = repo_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| CanonError::Storage(format!("Falha ao adquirir lock do SQLite: {}", e)))?;

            let mut stmt = conn
                .prepare(
                    "SELECT repo_id, payload_blob, timestamp_extracao
                     FROM artefatos_brutos
                     WHERE artifact_type = ?1
                       AND repo_id IN (?2, ?3)
                     ORDER BY CASE WHEN repo_id = ?2 THEN 0 ELSE 1 END, timestamp_extracao DESC
                     LIMIT 1",
                )
                .map_err(|e| CanonError::Storage(format!("Falha ao preparar query do cache canonico: {}", e)))?;

            stmt.query_row(params![BLOB_10_TYPE, repo_id, CANON_GLOBAL_REPO_ID], |row| {
                Ok(CanonCacheEntry {
                    repo_id: row.get(0)?,
                    payload_blob: row.get(1)?,
                    timestamp_extracao: row.get(2)?,
                })
            })
            .optional()
            .map_err(|e| CanonError::Storage(format!("Falha ao consultar cache canonico: {}", e)))
        })
        .await
        .map_err(|e| CanonError::Storage(format!("Falha ao aguardar leitura do cache canonico: {}", e)))?
    }

    async fn persist_blob(
        repo_id: &str,
        payload_blob: Vec<u8>,
        conn: Arc<Mutex<Connection>>,
    ) -> Result<(), CanonError> {
        let repo_id = repo_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| CanonError::Storage(format!("Falha ao adquirir lock do SQLite: {}", e)))?;
            let now = now_epoch_secs()?;
            conn.execute(
                "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(repo_id, artifact_type) DO UPDATE SET
                    payload_blob = excluded.payload_blob,
                    timestamp_extracao = excluded.timestamp_extracao",
                params![repo_id, BLOB_10_TYPE, payload_blob, now],
            )
            .map_err(|e| CanonError::Storage(format!("Falha ao persistir blob_10 no SQLite: {}", e)))?;
            Ok(())
        })
        .await
        .map_err(|e| CanonError::Storage(format!("Falha ao aguardar persistencia do blob_10: {}", e)))?
    }

    async fn resolve_notebook_id() -> Result<String, CanonError> {
        let stdout = Self::run_nlm(&["notebook", "list", "--json"]).await?;
        let json: Value = serde_json::from_slice(&stdout)
            .map_err(|e| CanonError::Parse(format!("Falha ao decodificar notebook list: {}", e)))?;

        extract_notebook_entries(&json)
            .into_iter()
            .find(|entry| entry.title.eq_ignore_ascii_case(CANON_NOTEBOOK_TITLE))
            .map(|entry| entry.id)
            .ok_or_else(|| CanonError::NotebookNotFound(CANON_NOTEBOOK_TITLE.to_string()))
    }

    async fn query_canon_context(notebook_id: &str) -> Result<String, CanonError> {
        let stdout = Self::run_nlm(&[
            "query",
            "notebook",
            notebook_id,
            CANON_QUERY_PROMPT,
            "--json",
            "--timeout",
            "180",
        ])
        .await?;
        let json: Value = serde_json::from_slice(&stdout)
            .map_err(|e| CanonError::Parse(format!("Falha ao decodificar resposta do NotebookLM: {}", e)))?;
        extract_query_answer(&json).ok_or_else(|| CanonError::Parse("Nao foi possivel localizar a resposta textual do NotebookLM".to_string()))
    }

    async fn run_nlm(args: &[&str]) -> Result<Vec<u8>, CanonError> {
        let mut full_args = vec!["--from".to_string(), "notebooklm-mcp-cli".to_string(), "nlm".to_string()];
        full_args.extend(args.iter().map(|arg| (*arg).to_string()));

        let mut command = Command::new(resolve_uvx_path());
        command.args(&full_args);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let child = command
            .spawn()
            .map_err(|e| CanonError::Cli(format!("Falha ao iniciar NotebookLM CLI: {}", e)))?;

        let output = timeout(Duration::from_secs(CANON_QUERY_TIMEOUT_SECS), child.wait_with_output())
            .await
            .map_err(|_| CanonError::Cli(format!("NotebookLM CLI excedeu {}s", CANON_QUERY_TIMEOUT_SECS)))?
            .map_err(|e| CanonError::Cli(format!("Falha ao aguardar NotebookLM CLI: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let combined = [stderr.as_str(), stdout.as_str()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" | ");
            if combined.to_lowercase().contains("authentication expired") {
                return Err(CanonError::AuthenticationExpired);
            }
            return Err(CanonError::Cli(combined));
        }

        if output.stdout.iter().all(|byte| byte.is_ascii_whitespace()) {
            return Err(CanonError::EmptyPayload);
        }

        Ok(output.stdout)
    }

    fn is_fresh(timestamp_extracao: i64) -> Result<bool, CanonError> {
        Ok(now_epoch_secs()? - timestamp_extracao <= CANON_CACHE_MAX_AGE_SECS)
    }
}

#[derive(Debug, Clone)]
struct NotebookEntry {
    id: String,
    title: String,
}

fn extract_notebook_entries(value: &Value) -> Vec<NotebookEntry> {
    match value {
        Value::Array(items) => items.iter().flat_map(extract_notebook_entries).collect(),
        Value::Object(map) => {
            if let (Some(id), Some(title)) = (
                map.get("id").and_then(Value::as_str),
                map.get("title")
                    .and_then(Value::as_str)
                    .or_else(|| map.get("name").and_then(Value::as_str)),
            ) {
                return vec![NotebookEntry {
                    id: id.trim().to_string(),
                    title: title.trim().to_string(),
                }];
            }

            ["notebooks", "items", "results", "data"]
                .into_iter()
                .filter_map(|key| map.get(key))
                .flat_map(extract_notebook_entries)
                .collect()
        }
        _ => Vec::new(),
    }
}

fn extract_query_answer(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Array(items) => items.iter().find_map(extract_query_answer),
        Value::Object(map) => {
            for key in ["answer", "response", "text", "content", "result", "message"] {
                if let Some(found) = map.get(key).and_then(extract_query_answer) {
                    return Some(found);
                }
            }
            map.values().find_map(extract_query_answer)
        }
        _ => None,
    }
}

fn resolve_uvx_path() -> PathBuf {
    if let Some(value) = env::var_os("SODA_UV_PATH") {
        if let Some(candidate) = resolve_configured_path(&value.to_string_lossy()) {
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    let executable_names = if cfg!(target_os = "windows") {
        vec!["uvx.exe", "uvx.cmd", "uvx.bat", "uvx"]
    } else {
        vec!["uvx"]
    };

    if let Some(path_var) = env::var_os("PATH") {
        for path_entry in env::split_paths(&path_var) {
            for executable_name in &executable_names {
                let candidate = path_entry.join(executable_name);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    if cfg!(target_os = "windows") {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            let base = PathBuf::from(local_app_data);
            for candidate in [
                base.join("Microsoft")
                    .join("WinGet")
                    .join("Packages")
                    .join("astral-sh.uv_Microsoft.Winget.Source_8wekyb3d8bbwe")
                    .join("uvx.exe"),
                base.join("Programs").join("uv").join("uvx.exe"),
            ] {
                if candidate.is_file() {
                    return candidate;
                }
            }
        }

        if let Some(app_data) = env::var_os("APPDATA") {
            let candidate = PathBuf::from(app_data).join("uv").join("uvx.exe");
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    PathBuf::from("uvx")
}

fn resolve_configured_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        Some(candidate)
    } else {
        let relative_candidate = candidate.clone();
        Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .map(|path| path.join(&relative_candidate))
                .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(candidate)),
        )
    }
}

fn now_epoch_secs() -> Result<i64, CanonError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CanonError::Storage(format!("Falha ao calcular timestamp atual: {}", e)))?
        .as_secs() as i64)
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
}
