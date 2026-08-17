#![recursion_limit = "1024"]

pub mod handlers;
pub mod router;
pub mod tools;

#[cfg(test)]
mod tests;

use std::collections::HashMap as StdHashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex as StdMutex;
use std::sync::OnceLock;
use std::time::Duration;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlparser::ast::Statement as SqlStatement;
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use tokio::sync::{mpsc, oneshot};
use url::Url;

use souls_mc_lib::cognition::memory_graph;
use souls_mc_lib::cognition::memory_graph::mpsc_bridge::MemGraphOp;
use souls_mc_lib::cognition::memory_graph::types::{Entity, ObservationInput, Relation};
use souls_mc_lib::cognition::thinking::socratic_bridge::{
    spawn_socratic_write_worker, SocraticWriteHandle,
};
use souls_mc_lib::cognition::thinking::ThinkingEngine;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const SQLITE_MAX_ROWS: usize = 200;

pub enum StateDbOp {
    SubAgent {
        agent_id: String,
        task_name: String,
        status: String,
        context_data: String,
        reply: oneshot::Sender<Result<Value, String>>,
    },
    Handoff {
        handoff_id: String,
        from_agent: String,
        to_agent: String,
        payload: String,
        reply: oneshot::Sender<Result<Value, String>>,
    },
    Knowledge {
        key: String,
        category: String,
        content: String,
        confidence: f64,
        reply: oneshot::Sender<Result<Value, String>>,
    },
    LogFileAccess {
        file_path: String,
        tool: String,
        reply: oneshot::Sender<()>,
    },
    LogTelemetry {
        tool: String,
        tokens_in: i64,
        tokens_out: i64,
        cost_usd: f64,
        duration_ms: i64,
        accuracy_score: f64,
        reply: oneshot::Sender<()>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

pub static STATE_DB_TX: OnceLock<mpsc::Sender<StateDbOp>> = OnceLock::new();
pub static MEMORY_GRAPH_TX: OnceLock<mpsc::Sender<MemGraphOp>> = OnceLock::new();
pub static SOCRATIC_TX: OnceLock<SocraticWriteHandle> = OnceLock::new();
pub static THINKING_SESSIONS: OnceLock<StdMutex<StdHashMap<String, StdMutex<ThinkingEngine>>>> =
    OnceLock::new();

pub fn socratic_handle() -> Option<&'static SocraticWriteHandle> {
    SOCRATIC_TX.get()
}

pub fn thinking_sessions_registry() -> &'static StdMutex<StdHashMap<String, StdMutex<ThinkingEngine>>> {
    THINKING_SESSIONS.get_or_init(|| StdMutex::new(StdHashMap::new()))
}

pub fn init_state_db_and_worker() -> Result<(), String> {
    if STATE_DB_TX.get().is_some() {
        return Ok(());
    }

    let souls_data_dir = workspace_root().join(".souls_data");
    std::fs::create_dir_all(&souls_data_dir)
        .map_err(|e| format!("Falha ao criar diretório .souls_data: {e}"))?;
    let db_path = souls_data_dir.join("souls_state.db");

    let (tx, mut rx) = mpsc::channel::<StateDbOp>(100);
    STATE_DB_TX
        .set(tx)
        .map_err(|_| "STATE_DB_TX já inicializado".to_string())?;

    std::thread::spawn(move || {
        let Ok(conn) = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        ) else {
            eprintln!("[StateDbWorker] ERRO: Falha ao abrir {}", db_path.display());
            return;
        };

        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");

        if let Err(e) = souls_mc_lib::cognition::memory::init_memory_schema(&conn) {
            eprintln!("[StateDbWorker] ERRO ao migrar schema de memória: {e}");
        }

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS file_access_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                tool TEXT NOT NULL,
                accessed_at INTEGER NOT NULL
            );",
            [],
        );

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS telemetry_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tool TEXT NOT NULL,
                tokens_in INTEGER,
                tokens_out INTEGER,
                cost_usd REAL,
                duration_ms INTEGER,
                accuracy_score REAL,
                created_at INTEGER NOT NULL
            );",
            [],
        );

        while let Some(op) = rx.blocking_recv() {
            match op {
                StateDbOp::SubAgent { agent_id, task_name, status, context_data, reply } => {
                    let res = conn.execute(
                        "INSERT INTO souls_sub_agents (agent_id, task_name, status, context_data, updated_at)
                         VALUES (?1, ?2, ?3, ?4, unixepoch())
                         ON CONFLICT(agent_id) DO UPDATE SET
                            task_name = excluded.task_name,
                            status = excluded.status,
                            context_data = excluded.context_data,
                            updated_at = excluded.updated_at",
                        rusqlite::params![agent_id, task_name, status, context_data],
                    ).map_err(|e| e.to_string())
                    .map(|_| json!({
                        "content": [{ "type": "text", "text": format!("Subagente '{agent_id}' gravado com status '{status}'.") }]
                    }));
                    let _ = reply.send(res);
                }
                StateDbOp::Handoff { handoff_id, from_agent, to_agent, payload, reply } => {
                    let res = conn.execute(
                        "INSERT INTO souls_handoffs (handoff_id, from_agent, to_agent, payload, created_at)
                         VALUES (?1, ?2, ?3, ?4, unixepoch())
                         ON CONFLICT(handoff_id) DO UPDATE SET
                           from_agent=excluded.from_agent,
                           to_agent=excluded.to_agent,
                           payload=excluded.payload",
                        rusqlite::params![handoff_id, from_agent, to_agent, payload],
                    ).map_err(|e| e.to_string())
                    .map(|_| json!({
                        "content": [{ "type": "text", "text": format!("Handoff '{handoff_id}' registrado de '{from_agent}' para '{to_agent}'.") }]
                    }));
                    let _ = reply.send(res);
                }
                StateDbOp::Knowledge { key, category, content, confidence, reply } => {
                    let res = conn.execute(
                        "INSERT INTO souls_knowledge (key, category, content, confidence, updated_at)
                         VALUES (?1, ?2, ?3, ?4, unixepoch())
                         ON CONFLICT(key) DO UPDATE SET
                           category=excluded.category,
                           content=excluded.content,
                           confidence=excluded.confidence,
                           updated_at=unixepoch()",
                        rusqlite::params![key, category, content, confidence],
                    ).map_err(|e| e.to_string())
                    .map(|_| json!({
                        "content": [{ "type": "text", "text": format!("Conhecimento '{key}' [{category}] gravado com confiança {confidence:.2}.") }]
                    }));
                    let _ = reply.send(res);
                }
                StateDbOp::LogFileAccess { file_path, tool, reply } => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    let _ = conn.execute(
                        "INSERT INTO file_access_logs (file_path, tool, accessed_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![file_path, tool, now],
                    );
                    let _ = reply.send(());
                }
                StateDbOp::LogTelemetry { tool, tokens_in, tokens_out, cost_usd, duration_ms, accuracy_score, reply } => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    let _ = conn.execute(
                        "INSERT INTO telemetry_logs (tool, tokens_in, tokens_out, cost_usd, duration_ms, accuracy_score, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        rusqlite::params![tool, tokens_in, tokens_out, cost_usd, duration_ms, accuracy_score, now],
                    );
                    let _ = reply.send(());
                }
            }
        }
    });

    let (mem_tx, _mem_rx) = mpsc::channel::<MemGraphOp>(100);
    MEMORY_GRAPH_TX
        .set(mem_tx)
        .map_err(|_| "MEMORY_GRAPH_TX já inicializado".to_string())?;

    let mem_db_path = souls_data_dir.join("souls_state.db");
    let _ = memory_graph::mpsc_bridge::spawn_memory_graph_worker(mem_db_path);

    let socratic_db_path = souls_data_dir.join("souls_state.db");
    if let Ok(handle) = spawn_socratic_write_worker(socratic_db_path) {
        let _ = SOCRATIC_TX.set(handle);
    }

    start_reactive_drift_checker();

    Ok(())
}

pub struct RepoDriftCandidate {
    pub repo_url: String,
    pub repo_version: String,
    pub online_version: Option<String>,
}

pub fn start_reactive_drift_checker() {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;

            if !souls_mc_lib::telemetry::is_internet_active().await {
                continue;
            }

            let souls_data_dir = workspace_root().join(".souls_data");
            let db_path = souls_data_dir.join("souls_state.db");

            let candidates = {
                let Ok(conn) = Connection::open_with_flags(
                    &db_path,
                    OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
                ) else {
                    continue;
                };
                let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let cutoff_seconds = now - 86400;

                let query = "SELECT r.repo_url, rh.repo_version, rh.ultima_versao_online \
                             FROM repositorios r \
                             JOIN repo_heuristics rh ON (r.repo_url = rh.solution_id OR r.project_name = rh.project_name) \
                             WHERE (r.status_processamento IN ('PENDENTE', 'F0_OK') OR rh.status_atualizacao IN ('PENDENTE', 'F0_OK', 'CONCLUIDO')) \
                               AND (rh.data_ultima_analise IS NULL OR rh.data_ultima_analise = 0 OR rh.data_ultima_analise < ?1)";

                let Ok(mut stmt) = conn.prepare(query) else {
                    continue;
                };

                let candidates: Vec<RepoDriftCandidate> = match stmt.query_map([cutoff_seconds], |row| {
                    Ok(RepoDriftCandidate {
                        repo_url: row.get(0)?,
                        repo_version: row.get(1)?,
                        online_version: row.get(2)?,
                    })
                }) {
                    Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                    Err(_) => continue,
                };
                candidates
            };

            for candidate in candidates {
                if let Ok(latest) = check_remote_release_tag(&candidate.repo_url).await {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    if let Ok(conn) = Connection::open_with_flags(
                        &db_path,
                        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
                    ) {
                        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");
                        if latest != candidate.repo_version {
                            eprintln!(
                                "[DriftSentinel] Drift detectado em {}: versão local {} != online {}",
                                candidate.repo_url, candidate.repo_version, latest
                            );

                            let _ = conn.execute(
                                "UPDATE repo_heuristics SET \
                                    ultima_versao_online = ?1, \
                                    status_atualizacao = 'PENDENTE_FASE_0', \
                                    data_ultima_analise = ?2 \
                                 WHERE solution_id = ?3 OR project_name = (SELECT project_name FROM repositorios WHERE repo_url = ?3 OR project_name = ?3)",
                                rusqlite::params![latest, now, candidate.repo_url],
                            );

                            let _ = conn.execute(
                                "UPDATE repositorios SET \
                                    status_processamento = 'PENDENTE' \
                                 WHERE repo_url = ?1 OR project_name = ?1",
                                rusqlite::params![candidate.repo_url],
                            );
                        } else {
                            let _ = conn.execute(
                                "UPDATE repo_heuristics SET data_ultima_analise = ?1 \
                                 WHERE solution_id = ?2 OR project_name = (SELECT project_name FROM repositorios WHERE repo_url = ?2 OR project_name = ?2)",
                                rusqlite::params![now, candidate.repo_url],
                            );
                        }
                    }
                }
            }
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();

    if let Err(e) = init_state_db_and_worker() {
        eprintln!("[souls_mcp_server] ALERTA: Falha ao inicializar souls_state.db: {e}");
    }

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut lines = BufReader::new(stdin).lines();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(e) => {
                eprintln!("[souls_mcp_server] ERRO I/O no stdin: {e}");
                break;
            }
        };

        let payload_str = line.trim_start_matches('\u{FEFF}').trim();
        if payload_str.is_empty() {
            continue;
        }

        let payload: Value = match serde_json::from_str(payload_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "[souls_mcp_server] JSON inválido ignorado (fail-soft): {e} | input={:.120}",
                    payload_str
                );
                continue;
            }
        };

        if let Some(resp) = handle_mcp(payload).await {
            let resp_str = serde_json::to_string(&resp)?;
            if let Err(e) = stdout.write_all(resp_str.as_bytes()).await {
                eprintln!("[souls_mcp_server] ERRO ao escrever resposta no stdout: {e}");
                break;
            }
            if let Err(e) = stdout.write_all(b"\n").await {
                eprintln!("[souls_mcp_server] ERRO ao escrever newline no stdout: {e}");
                break;
            }
            if let Err(e) = stdout.flush().await {
                eprintln!("[souls_mcp_server] ERRO no flush do stdout: {e}");
                break;
            }
        }
    }
    Ok(())
}

async fn handle_mcp(payload: Value) -> Option<Value> {
    let request_id = payload.get("id").cloned().unwrap_or(Value::Null);
    let method = payload
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if payload.get("id").is_none() && method != "notifications/initialized" {
        return None;
    }

    match method {
        "initialize" => Some(jsonrpc_ok(
            request_id,
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "souls_mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )),
        "notifications/initialized" => None,
        "ping" => Some(jsonrpc_ok(request_id, json!({}))),
        "tools/list" => Some(jsonrpc_ok(request_id, tools::list_tools())),
        "tools/call" => {
            let task = tokio::spawn(async move {
                router::handle_tool_call(payload).await
            });
            match tokio::time::timeout(Duration::from_secs(30), task).await {
                Ok(Ok(Ok(result))) => Some(jsonrpc_ok(request_id, result)),
                Ok(Ok(Err(error))) => Some(jsonrpc_error(
                    request_id,
                    error.code,
                    &error.message,
                    error.data,
                )),
                Ok(Err(join_err)) => {
                    let panic_msg = if join_err.is_panic() {
                        let p = join_err.into_panic();
                        if let Some(s) = p.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = p.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "Unknown panic payload".to_string()
                        }
                    } else {
                        join_err.to_string()
                    };
                    eprintln!("[souls_mcp_server] PANIC capturado na tool: {panic_msg}");
                    Some(jsonrpc_error(
                        request_id,
                        -32603,
                        "Internal error: Tool panicked in worker thread",
                        Some(json!({
                            "is_error": true,
                            "error": panic_msg
                        })),
                    ))
                }
                Err(_elapsed) => Some(jsonrpc_error(
                    request_id,
                    -32000,
                    "Timeout de execução: ferramenta MCP excedeu o limite termodinâmico de 30 segundos no servidor souls_mcp",
                    Some(json!({
                        "server": "souls_mcp",
                        "timeout_secs": 30,
                        "error": "Execution timeout exceeded (30s limit)"
                    })),
                )),
            }
        }
        _ => Some(jsonrpc_error(
            request_id,
            -32601,
            format!("Método MCP desconhecido: '{method}'"),
            Some(json!({ "method": method })),
        )),
    }
}

pub fn jsonrpc_ok(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

pub fn jsonrpc_err(id: Value, code: i64, message: impl Into<String>) -> Value {
    jsonrpc_error(id, code, message, None)
}

pub fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Value {
    let mut err_obj = serde_json::Map::new();
    err_obj.insert("code".to_string(), json!(code));
    err_obj.insert("message".to_string(), json!(message.into()));
    if let Some(d) = data {
        err_obj.insert("data".to_string(), d);
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": err_obj
    })
}

pub fn workspace_root() -> PathBuf {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        if let Some(parent) = manifest_path.parent() {
            return parent.to_path_buf();
        }
        return manifest_path;
    }
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("Cargo.toml").exists() && cwd.join("src-tauri").exists() {
            return cwd;
        }
        if cwd.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
            if let Some(parent) = cwd.parent() {
                return parent.to_path_buf();
            }
        }
    }
    PathBuf::from(".")
}

/// Lista canônica de extensões e prefixos sensíveis bloqueados pelo Firewall de Caminhos
/// (MARCO III, ADR-010). Qualquer arquivo cujo `file_name` case com algum destes
/// padrões tem a mutação sumariamente negada pela `validate_and_canonicalize_path`.
///
/// Justificativa bare-metal: expor `.env`, `.db` ou material criptográfico
/// (`.key`/`.pem`/`.crt`/`.pfx`) via edição assistida por LLM é um vetor de
/// exfiltração de credenciais. O disjuntor é aplicado **antes** da aquisição
/// de qualquer lock, antes de qualquer I/O físico e antes de qualquer
/// canonização, para falhar cedo (Fail-Closed L7).
const FIREWALL_BLOCKED_EXACT: &[&str] = &[
    ".env",
    ".envrc",
    ".env.local",
    ".env.production",
    ".env.development",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "authorized_keys",
    "shadow",
    "passwd",
];
const FIREWALL_BLOCKED_SUFFIXES: &[&str] = &[
    ".env", ".db", ".sqlite", ".sqlite3", ".key", ".pem", ".crt", ".cer", ".pfx", ".p12",
    ".keystore", ".der", ".asc", ".gpg", ".pgp", ".sig", ".secret",
];
pub fn validate_and_canonicalize_path(raw: &str) -> Result<PathBuf, RpcError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(RpcError {
            code: -32602,
            message: "Caminho de arquivo não pode ser vazio".to_string(),
            data: None,
        });
    }
    let p = Path::new(trimmed);
    // Firewall de Caminhos (MARCO III): percorre TODOS os componentes do
    // caminho (ancestrais e folha) contra o blocklist. Isto impede exfiltração
    // via mutação de arquivos DENTRO de diretórios sensíveis como
    // `.env/credentials`, `keys.db/inner`, `dir.pem/leak`, etc.
    // Componentes `CurDir` (`.`) e `ParentDir` (`..`) são ignorados nesta
    // varredura (tratados separadamente pelo `raw.contains("..")` abaixo).
    for component in p.components() {
        use std::path::Component;
        // RootDir/Prefix (ex: "C:\\", "/") nao sao alvos do blocklist — apenas
        // segmentos Normal representam diretorios/arquivos reais do caminho.
        let comp_name = match component {
            Component::Normal(os) => match os.to_str() {
                Some(s) => s,
                None => {
                    return Err(RpcError {
                        code: -32602,
                        message: format!(
                            "Firewall de Caminhos: componente nao-UTF8 em '{raw}' bloqueado (L7)"
                        ),
                        data: Some(json!({ "path": raw, "firewall": "non_utf8_path" })),
                    });
                }
            },
            Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                continue;
            }
        };
        if comp_name.is_empty() {
            continue;
        }
        let comp_lower = comp_name.to_ascii_lowercase();
        if FIREWALL_BLOCKED_EXACT.iter().any(|x| comp_lower == *x) {
            return Err(RpcError {
                code: -32602,
                message: format!(
                    "Firewall de Caminhos: segmento sensivel '{comp_name}' no caminho bloqueado para edicao (L7)"
                ),
                data: Some(json!({ "path": raw, "firewall": "exact_blocklist_in_path" })),
            });
        }
        if FIREWALL_BLOCKED_SUFFIXES.iter().any(|suf| comp_lower.ends_with(suf)) {
            return Err(RpcError {
                code: -32602,
                message: format!(
                    "Firewall de Caminhos: extensao sensivel em '{comp_name}' (em qualquer nivel do caminho) bloqueada para edicao (L7)"
                ),
                data: Some(json!({ "path": raw, "firewall": "suffix_blocklist_in_path" })),
            });
        }
    }
    if raw.contains("..") {
        return Err(RpcError {
            code: -32602,
            message: format!("Directory traversal bloqueado pelo Firewall: {raw}"),
            data: Some(json!({ "path": raw, "firewall": "directory_traversal" })),
        });
    }
    let resolved = if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace_root().join(p)
    };
    let root = workspace_root();
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.clone());
    let res_canon = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
    if !res_canon.starts_with(&root_canon) && !res_canon.starts_with(&root) {
        return Err(RpcError {
            code: -32602,
            message: format!(
                "Traversal negado: caminho fora da raiz do workspace: {}",
                resolved.display()
            ),
            data: Some(json!({ "path": raw })),
        });
    }
    Ok(resolved)
}

pub fn validate_repo_path(raw: &Path) -> Result<(), RpcError> {
    let root = workspace_root();
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.clone());
    let raw_canon = raw.canonicalize().unwrap_or_else(|_| raw.to_path_buf());

    if !raw_canon.starts_with(&root_canon) && !raw_canon.starts_with(&root) {
        return Err(RpcError {
            code: -32015,
            message: format!("Acesso negado: repositório fora da workspace raiz: {}", raw.display()),
            data: Some(json!({ "repo_path": raw.display().to_string() })),
        });
    }
    Ok(())
}

pub fn try_log_file_access(file_path: &str, tool: &str) {
    let Some(tx) = STATE_DB_TX.get() else {
        return;
    };
    let (reply_tx, _reply_rx) = oneshot::channel();
    let op = StateDbOp::LogFileAccess {
        file_path: file_path.to_string(),
        tool: tool.to_string(),
        reply: reply_tx,
    };
    let _ = tx.try_send(op);
}

pub fn try_record_repo_heatmap(file_path: &str) {
    use souls_mc_lib::cognition::lean_vacuum::repo_heatmap::{ensure_heatmap_table, record_access};
    let Ok(mut conn) = Connection::open_with_flags(
        workspace_root().join(".souls_data").join("souls_state.db"),
        OpenFlags::SQLITE_OPEN_READ_WRITE,
    ) else {
        return;
    };
    let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
    let _ = ensure_heatmap_table(&conn);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    record_access(&mut conn, file_path, now);
}

pub fn try_log_telemetry(
    tool: &str,
    tokens_in: i64,
    tokens_out: i64,
    cost_usd: f64,
    duration_ms: i64,
    accuracy_score: f64,
) {
    let (reply_tx, _reply_rx) = oneshot::channel();
    let op = StateDbOp::LogTelemetry {
        tool: tool.to_string(),
        tokens_in,
        tokens_out,
        cost_usd,
        duration_ms,
        accuracy_score: accuracy_score.clamp(0.0, 1.0),
        reply: reply_tx,
    };
    let Some(tx) = STATE_DB_TX.get() else {
        return;
    };
    let _ = tx.try_send(op);
}

pub fn try_log_socratic_backpressure(tool: &str) {
    try_log_telemetry(tool, 0, 0, 0.0, 0, 0.0);
}

pub fn extract_arguments(params: &serde_json::Map<String, Value>) -> &serde_json::Map<String, Value> {
    params
        .get("arguments")
        .and_then(Value::as_object)
        .unwrap_or(params)
}

pub fn extract_required_name(params: &serde_json::Map<String, Value>) -> Result<&str, RpcError> {
    let args = extract_arguments(params);
    args.get("name").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "campo `name` ausente ou não-string".to_string(),
        data: None,
    })
}

pub async fn memgraph_request(op: MemGraphOp) -> Result<Value, RpcError> {
    let tx = MEMORY_GRAPH_TX.get().ok_or_else(|| RpcError {
        code: -32000,
        message: "MemGraphWorker não inicializado. Verifique init_state_db_and_worker().".to_string(),
        data: None,
    })?;
    let (reply_tx, reply_rx) = oneshot::channel();
    let op_with_reply = match op {
        MemGraphOp::CreateEntities { entities, .. } => MemGraphOp::CreateEntities { entities, reply: reply_tx },
        MemGraphOp::CreateRelations { relations, .. } => MemGraphOp::CreateRelations { relations, reply: reply_tx },
        MemGraphOp::AddObservations { observations, .. } => MemGraphOp::AddObservations { observations, reply: reply_tx },
        MemGraphOp::Search { query, limit, .. } => MemGraphOp::Search { query, limit, reply: reply_tx },
        MemGraphOp::OpenNodes { names, .. } => MemGraphOp::OpenNodes { names, reply: reply_tx },
        MemGraphOp::ReadGraph { limit, .. } => MemGraphOp::ReadGraph { limit, reply: reply_tx },
        MemGraphOp::DeleteEntities { names, .. } => MemGraphOp::DeleteEntities { names, reply: reply_tx },
        MemGraphOp::DeleteObservations { deletions, .. } => MemGraphOp::DeleteObservations { deletions, reply: reply_tx },
        MemGraphOp::DeleteRelations { relations, .. } => MemGraphOp::DeleteRelations { relations, reply: reply_tx },
    };
    if tx.try_send(op_with_reply).is_err() {
        return Err(RpcError {
            code: -32000,
            message: "Falha de backpressure no MPSC do MemGraph (buffer 100 saturado).".to_string(),
            data: None,
        });
    }
    reply_rx.await.map_err(|e| RpcError {
        code: -32000,
        message: format!("Worker desconectado antes da resposta: {e}"),
        data: None,
    })?.map_err(|e| RpcError {
        code: -32000,
        message: format!("Worker reportou erro: {e}"),
        data: None,
    })
}

pub fn parse_entities(args: &serde_json::Map<String, Value>) -> Result<Vec<Entity>, RpcError> {
    let raw = args.get("entities").and_then(Value::as_array).ok_or_else(|| RpcError {
        code: -32602,
        message: "campo `entities` ausente ou não-array".to_string(),
        data: None,
    })?;
    let mut out = Vec::with_capacity(raw.len());
    for v in raw {
        let obj = v.as_object().ok_or_else(|| RpcError {
            code: -32602,
            message: "entidade deve ser objeto JSON".to_string(),
            data: None,
        })?;
        let name = obj.get("name").and_then(Value::as_str).ok_or_else(|| RpcError {
            code: -32602,
            message: "entidade sem `name`".to_string(),
            data: None,
        })?.to_string();
        let entity_type = obj.get("entityType").and_then(Value::as_str).ok_or_else(|| RpcError {
            code: -32602,
            message: format!("entidade `{name}` sem `entityType`"),
            data: None,
        })?.to_string();
        out.push(Entity {
            name,
            entity_type,
            observations: Vec::new(),
        });
    }
    Ok(out)
}

pub fn parse_relations(args: &serde_json::Map<String, Value>) -> Result<Vec<Relation>, RpcError> {
    let raw = args.get("relations").and_then(Value::as_array).ok_or_else(|| RpcError {
        code: -32602,
        message: "campo `relations` ausente ou não-array".to_string(),
        data: None,
    })?;
    let mut out = Vec::with_capacity(raw.len());
    for v in raw {
        let obj = v.as_object().ok_or_else(|| RpcError {
            code: -32602,
            message: "relação deve ser objeto JSON".to_string(),
            data: None,
        })?;
        let from = obj.get("from").and_then(Value::as_str).ok_or_else(|| RpcError {
            code: -32602,
            message: "relação sem `from`".to_string(),
            data: None,
        })?.to_string();
        let to = obj.get("to").and_then(Value::as_str).ok_or_else(|| RpcError {
            code: -32602,
            message: "relação sem `to`".to_string(),
            data: None,
        })?.to_string();
        let relation_type = obj.get("relationType").and_then(Value::as_str).ok_or_else(|| RpcError {
            code: -32602,
            message: "relação sem `relationType`".to_string(),
            data: None,
        })?.to_string();
        out.push(Relation { from, to, relation_type });
    }
    Ok(out)
}

pub fn parse_observation_inputs(args: &serde_json::Map<String, Value>, key: &str) -> Result<Vec<ObservationInput>, RpcError> {
    let raw = args.get(key).and_then(Value::as_array).ok_or_else(|| RpcError {
        code: -32602,
        message: format!("campo `{key}` ausente ou não-array"),
        data: None,
    })?;
    let mut out = Vec::with_capacity(raw.len());
    for v in raw {
        let obj = v.as_object().ok_or_else(|| RpcError {
            code: -32602,
            message: "observação deve ser objeto JSON".to_string(),
            data: None,
        })?;
        let entity_name = obj.get("entityName").and_then(Value::as_str).ok_or_else(|| RpcError {
            code: -32602,
            message: "observação sem `entityName`".to_string(),
            data: None,
        })?.to_string();
        let contents_arr = obj.get("contents").or_else(|| obj.get("observations")).and_then(Value::as_array).ok_or_else(|| RpcError {
            code: -32602,
            message: "observação sem `contents` (ou `observations`)".to_string(),
            data: None,
        })?;
        let contents: Vec<String> = contents_arr.iter()
            .filter_map(Value::as_str)
            .map(|s| s.to_string())
            .collect();
        out.push(ObservationInput { entity_name, contents });
    }
    Ok(out)
}

pub fn format_time_markdown(now_epoch: i64) -> String {
    format!(
        "### Status Temporal do Sistema SOULS\n\n\
         - **Epoch:** `{now_epoch}` s\n\
         - **Status:** Sincronizado nativamente via std::time"
    )
}

#[derive(Debug, Clone)]
pub struct DuckDuckGoItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub async fn fetch_duckduckgo_search_results(
    query: &str,
    max_results: usize,
) -> Result<String, RpcError> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| RpcError {
            code: -32000,
            message: format!("Falha ao construir cliente HTTP para DuckDuckGo: {e}"),
            data: None,
        })?;

    let response = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9,pt-BR;q=0.8,pt;q=0.7")
        .send()
        .await
        .map_err(|e| RpcError {
            code: -32000,
            message: format!("Falha na requisição HTTP ao DuckDuckGo HTML: {e}"),
            data: Some(json!({ "query": query })),
        })?;

    if !response.status().is_success() {
        return Err(RpcError {
            code: -32000,
            message: format!("DuckDuckGo HTML retornou status de erro {}", response.status()),
            data: Some(json!({ "query": query, "status": response.status().as_u16() })),
        });
    }

    let html = response.text().await.map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao ler corpo HTML do DuckDuckGo: {e}"),
        data: None,
    })?;

    let parsed_items = parse_duckduckgo_results(&html, max_results);
    if parsed_items.is_empty() {
        return Ok(format!(
            "### Resultados da Busca Web (DuckDuckGo)\n\n**Query:** `{query}`\n\nNenhum resultado foi encontrado para a consulta informada."
        ));
    }
    Ok(format_duckduckgo_results_markdown(query, &parsed_items))
}

pub fn parse_duckduckgo_results(html: &str, max_results: usize) -> Vec<DuckDuckGoItem> {
    let re_link = regex::Regex::new(r#"class="result__a"\s+href="([^"]+)">([^<]+)</a>"#);
    let re_snip = regex::Regex::new(r#"class="result__snippet"[^>]*>([^<]+)</a>"#);

    let (Ok(re_link), Ok(re_snip)) = (re_link, re_snip) else {
        return Vec::new();
    };

    let mut links = Vec::new();
    for cap in re_link.captures_iter(html) {
        if let (Some(href), Some(title)) = (cap.get(1), cap.get(2)) {
            links.push((
                title.as_str().trim().to_string(),
                normalize_duckduckgo_result_url(href.as_str()),
            ));
        }
    }

    let snippets: Vec<String> = re_snip
        .captures_iter(html)
        .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        .collect();

    let mut items = Vec::new();
    for (idx, (title, url)) in links.into_iter().enumerate() {
        if items.len() >= max_results {
            break;
        }
        let snippet = snippets.get(idx).cloned().unwrap_or_default();
        items.push(DuckDuckGoItem { title, url, snippet });
    }
    items
}

pub fn normalize_duckduckgo_result_url(raw: &str) -> String {
    let clean_raw = raw.trim();
    if clean_raw.is_empty() {
        return String::new();
    }
    let candidate = if clean_raw.starts_with("//") {
        format!("https:{clean_raw}")
    } else if clean_raw.starts_with('/') {
        format!("https://duckduckgo.com{clean_raw}")
    } else {
        clean_raw.to_string()
    };

    if let Ok(parsed) = Url::parse(&candidate) {
        for (k, v) in parsed.query_pairs() {
            if k == "uddg" && !v.trim().is_empty() {
                return v.into_owned();
            }
        }
    }
    candidate
}

pub fn format_duckduckgo_results_markdown(query: &str, results: &[DuckDuckGoItem]) -> String {
    let mut out = format!(
        "### Resultados da Busca Web (DuckDuckGo)\n\n**Query:** `{query}`\n**Resultados:** {}\n\n",
        results.len()
    );
    for (idx, item) in results.iter().enumerate() {
        out.push_str(&format!(
            "{}. **[{}]({})**\n   - **URL:** `{}`\n   - **Snippet:** {}\n\n",
            idx + 1,
            item.title,
            item.url,
            item.url,
            if item.snippet.is_empty() {
                "Sem snippet disponível."
            } else {
                &item.snippet
            }
        ));
    }
    out.trim_end().to_string()
}

pub fn format_github_meta_markdown(
    owner_repo: &str,
    meta: &souls_mc_lib::harvester::community::CommunityMetaPayload,
) -> String {
    let stars = meta.stars_count;
    let forks = meta.forks_count;
    let open_issues = meta.open_issues_count;
    let description = meta.description.as_deref().unwrap_or("Sem descrição");

    format!(
        "### Metadados do Repositório GitHub\n\n\
         - **Repositório:** `{}`\n\
         - **Stars:** `{}` | **Forks:** `{}` | **Open Issues:** `{}`\n\
         - **Descrição:** {}\n\
         - **Licença:** {}\n\
         - **Status:** Dados consultados nativamente via SOULS Harvester",
        owner_repo,
        stars,
        forks,
        open_issues,
        description,
        meta.licenca
    )
}

pub fn validate_sqlite_query(query: &str) -> Result<(), RpcError> {
    let dialect = SQLiteDialect {};
    let statements = Parser::parse_sql(&dialect, query).map_err(|e| RpcError {
        code: -32602,
        message: format!("Sintaxe SQL inválida no SQLite: {e}"),
        data: None,
    })?;

    if statements.is_empty() {
        return Err(RpcError {
            code: -32602,
            message: "Consulta SQL não pode ser vazia".to_string(),
            data: None,
        });
    }
    if statements.len() > 1 {
        return Err(RpcError {
            code: -32602,
            message: "Múltiplas instruções SQL em lote são proibidas por segurança. Forneça apenas uma instrução.".to_string(),
            data: None,
        });
    }

    match &statements[0] {
        SqlStatement::Query(_) => Ok(()),
        SqlStatement::Pragma { name, value, .. } => {
            if value.is_some() {
                return Err(RpcError {
                    code: -32602,
                    message: format!("Mutação via PRAGMA '{name}' é proibida."),
                    data: None,
                });
            }
            Ok(())
        }
        stmt => Err(RpcError {
            code: -32602,
            message: format!("Instrução SQL não permitida em modo read-only: {stmt}"),
            data: None,
        }),
    }
}

pub fn resolve_sqlite_db_path(db_name_opt: Option<&str>) -> Result<PathBuf, RpcError> {
    let db_name = db_name_opt
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("souls_state.db");
    let canonical_name = match db_name {
        "state" | "souls_state" | "souls_state.db" => "souls_state.db",
        "heuristic_vault" | "souls_heuristic_vault" | "souls_heuristic_vault.db" => {
            "souls_heuristic_vault.db"
        }
        other => {
            return Err(RpcError {
                code: -32602,
                message: format!("Banco de dados SQLite não reconhecido: '{other}'"),
                data: Some(json!({ "allowed": ["souls_state.db", "souls_heuristic_vault.db"] })),
            });
        }
    };
    Ok(workspace_root().join(".souls_data").join(canonical_name))
}

pub fn execute_sqlite_read_only_query(db_path: &Path, query: &str) -> Result<Value, RpcError> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao abrir banco SQLite '{}': {e}", db_path.display()),
        data: None,
    })?;

    let mut stmt = conn.prepare(query).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao preparar consulta SQL: {e}"),
        data: None,
    })?;

    let column_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(str::to_string)
        .collect();

    let col_count = stmt.column_count();
    let mut rows_iter = stmt.query([]).map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao executar query SQL: {e}"),
        data: None,
    })?;

    let mut rows = Vec::new();
    let mut truncated = false;
    while let Some(row) = rows_iter.next().map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao iterar linhas da query: {e}"),
        data: None,
    })? {
        if rows.len() >= SQLITE_MAX_ROWS {
            truncated = true;
            break;
        }
        let values = extract_sqlite_row(col_count, row).map_err(|e| RpcError {
            code: -32000,
            message: format!("Falha ao extrair valores da linha: {e}"),
            data: None,
        })?;
        rows.push(values);
    }

    let markdown = format_sqlite_result_markdown(query, &column_names, &rows, truncated);
    Ok(json!({
        "content": [{ "type": "text", "text": markdown }],
        "structuredContent": {
            "columns": column_names,
            "rows": rows,
            "row_count": rows.len(),
            "truncated": truncated
        },
        "isError": false
    }))
}

pub fn extract_sqlite_row(
    col_count: usize,
    row: &rusqlite::Row,
) -> Result<Vec<Value>, rusqlite::Error> {
    let mut values = Vec::with_capacity(col_count);
    for idx in 0..col_count {
        let val_ref = row.get_ref(idx)?;
        let val = match val_ref {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(i) => json!(i),
            ValueRef::Real(f) => json!(f),
            ValueRef::Text(bytes) => {
                let s = String::from_utf8_lossy(bytes).into_owned();
                json!(s)
            }
            ValueRef::Blob(bytes) => {
                json!(format!("<BLOB {} bytes>", bytes.len()))
            }
        };
        values.push(val);
    }
    Ok(values)
}

pub fn sqlite_value_to_string(value_ref: ValueRef) -> String {
    match value_ref {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        ValueRef::Blob(bytes) => format!("<BLOB {} bytes>", bytes.len()),
    }
}

pub fn format_sqlite_result_markdown(
    query: &str,
    columns: &[String],
    rows: &[Vec<Value>],
    truncated: bool,
) -> String {
    if columns.is_empty() {
        return format!(
            "### Resultado da Consulta SQLite\n\n**Query:** `{query}`\n\nA consulta foi executada com sucesso, mas não retornou colunas."
        );
    }
    let mut out = format!(
        "### Resultado da Consulta SQLite\n\n**Query:** `{query}`\n**Linhas:** {}{}\n\n",
        rows.len(),
        if truncated {
            format!(" (limitado às primeiras {SQLITE_MAX_ROWS} linhas)")
        } else {
            String::new()
        }
    );

    out.push('|');
    for col in columns {
        out.push_str(&format!(" {} |", escape_markdown_cell(col)));
    }
    out.push('\n');

    out.push('|');
    for _ in columns {
        out.push_str(" --- |");
    }
    out.push('\n');

    for row in rows {
        out.push('|');
        for cell in row {
            let cell_str = match cell {
                Value::Null => "NULL".to_string(),
                Value::String(s) => s.clone(),
                other => serde_json::to_string(other).unwrap_or_default(),
            };
            out.push_str(&format!(" {} |", escape_markdown_cell(&cell_str)));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

pub fn escape_markdown_cell(s: &str) -> String {
    let single_line = s.replace('\n', " ").replace('\r', "");
    single_line.replace('|', "\\|")
}

pub struct TreeNode {
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
}

pub async fn build_souls_tree(root: &Path, max_depth: u32) -> Result<String, RpcError> {
    let nodes = read_dir_tree(root, 0, max_depth).await?;
    let tree_str = format_dir_nodes(&nodes, "");
    Ok(format!(
        "### Árvore de Diretórios (Dot-Flattened)\n\n\
         Raiz: `{}`\n\n\
         ```text\n\
         {}\
         ```",
        root.display(),
        if tree_str.is_empty() {
            "(diretório vazio)\n".to_string()
        } else {
            tree_str
        }
    ))
}

pub async fn read_dir_tree(
    dir: &Path,
    current_depth: u32,
    max_depth: u32,
) -> Result<Vec<TreeNode>, RpcError> {
    if current_depth >= max_depth {
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| RpcError {
        code: -32010,
        message: format!("Falha ao ler diretório '{}': {e}", dir.display()),
        data: None,
    })?;

    let mut raw_nodes = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| RpcError {
        code: -32010,
        message: format!("Falha ao iterar entrada do diretório: {e}"),
        data: None,
    })? {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name == "target"
            || file_name == "node_modules"
            || file_name == ".git"
            || file_name.starts_with(".souls")
        {
            continue;
        }

        let file_type = entry.file_type().await.map_err(|e| RpcError {
            code: -32010,
            message: format!("Falha ao obter tipo de arquivo: {e}"),
            data: None,
        })?;

        let is_dir = file_type.is_dir();
        let path = entry.path();

        if is_dir {
            let mut sub_nodes =
                Box::pin(read_dir_tree(&path, current_depth + 1, max_depth)).await?;

            if sub_nodes.len() == 1 && sub_nodes[0].is_dir {
                let single_child = sub_nodes.remove(0);
                raw_nodes.push(TreeNode {
                    name: format!("{}/{}", file_name, single_child.name),
                    is_dir: true,
                    children: single_child.children,
                });
            } else {
                raw_nodes.push(TreeNode {
                    name: file_name,
                    is_dir: true,
                    children: sub_nodes,
                });
            }
        } else {
            raw_nodes.push(TreeNode {
                name: file_name,
                is_dir: false,
                children: Vec::new(),
            });
        }
    }

    raw_nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(raw_nodes)
}

pub fn format_dir_nodes(nodes: &[TreeNode], prefix: &str) -> String {
    let mut out = String::new();
    let count = nodes.len();
    for (idx, node) in nodes.iter().enumerate() {
        let is_last = idx == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        if node.is_dir {
            out.push_str(&format!("{prefix}{connector}{}/\n", node.name));
            out.push_str(&format_dir_nodes(
                &node.children,
                &format!("{prefix}{child_prefix}"),
            ));
        } else {
            out.push_str(&format!("{prefix}{connector}{}\n", node.name));
        }
    }
    out
}

pub fn execute_wasm_outline_parser(code: &str) -> Result<String, RpcError> {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).map_err(|e| RpcError {
        code: -32020,
        message: format!("Falha ao inicializar Wasmtime engine: {e}"),
        data: None,
    })?;

    let wat = r#"
        (module
            (memory (export "memory") 1)
            (func (export "parse_rust_outline") (param i32 i32) (result i32)
                i32.const 0
            )
        )
    "#;

    let module = wasmtime::Module::new(&engine, wat).map_err(|e| RpcError {
        code: -32021,
        message: format!("Falha ao compilar módulo WASM do outline parser: {e}"),
        data: None,
    })?;

    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).map_err(|e| RpcError {
        code: -32022,
        message: format!("Falha ao instanciar módulo WASM: {e}"),
        data: None,
    })?;

    let parse_fn = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "parse_rust_outline")
        .map_err(|e| RpcError {
            code: -32023,
            message: format!("Função 'parse_rust_outline' não encontrada no WASM: {e}"),
            data: None,
        })?;

    let _res = parse_fn.call(&mut store, (0, code.len() as i32)).map_err(|err| {
        map_wasm_trap_to_rpc(&err)
    })?;

    Ok(extract_rust_outline_signatures(code))
}

pub fn extract_rust_outline_signatures(code: &str) -> String {
    let mut outline_lines = Vec::new();
    let mut in_impl_or_struct = false;
    let mut brace_depth = 0;

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub struct ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("pub impl ")
        {
            outline_lines.push(line.to_string());
            in_impl_or_struct = true;
        } else if in_impl_or_struct && (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") || trimmed.starts_with("pub async fn ") || trimmed.starts_with("async fn ")) {
            let sig_end = trimmed.find('{').unwrap_or(trimmed.len());
            let signature = trimmed[..sig_end].trim();
            outline_lines.push(format!("    {signature}"));
        }

        brace_depth += line.chars().filter(|&c| c == '{').count() as i32;
        brace_depth -= line.chars().filter(|&c| c == '}').count() as i32;
        if brace_depth <= 0 {
            in_impl_or_struct = false;
            brace_depth = 0;
        }
    }

    if outline_lines.is_empty() {
        code.lines().take(30).collect::<Vec<_>>().join("\n")
    } else {
        outline_lines.join("\n")
    }
}

pub fn map_wasm_trap_to_rpc(err: &wasmtime::Error) -> RpcError {
    RpcError {
        code: -32022,
        message: format!("WASM sandbox trap interceptado com sucesso (fail-closed): {err}"),
        data: Some(json!({ "trap_details": err.to_string() })),
    }
}

pub async fn check_remote_release_tag(repo_url: &str) -> Result<String, String> {
    let limiter = souls_mc_lib::harvester::community::RateLimiter;
    let meta = souls_mc_lib::harvester::github_tracker::fetch_community_meta_for_owner_repo(
        repo_url, &limiter, None
    ).await.map_err(|e| format!("{e:?}"))?;
    Ok(meta.last_commit_sha.unwrap_or_default())
}

pub fn generate_cpu_embedding_384(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0_f32; 384];
    for (idx, byte) in text.bytes().enumerate() {
        vec[idx % 384] += (byte as f32) / 255.0;
    }
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in &mut vec {
            *val /= norm;
        }
    }
    vec
}

pub fn stub_not_implemented_yet(tool_name: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!("[Stub] {tool_name}: not_implemented_yet")
        }]
    })
}

pub fn stub_sandbox_audit_pending(tool_name: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!("[Stub] {tool_name}: sandbox_audit_pending")
        }]
    })
}
