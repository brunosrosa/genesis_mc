use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use genesis_mc_lib::harvester::ast_parser;
use genesis_mc_lib::harvester::community::RateLimiter;
use genesis_mc_lib::harvester::github_tracker;
use genesis_mc_lib::harvester::web_scraper;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};

const MCP_SESSION_ID_HEADER: &str = "Mcp-Session-Id";
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const SQLITE_MAX_ROWS: usize = 200;

#[derive(Clone)]
struct AppState {
    session_seed: Arc<AtomicU64>,
}

impl AppState {
    fn new() -> Self {
        Self {
            session_seed: Arc::new(AtomicU64::new(1)),
        }
    }

    fn next_session_id(&self) -> String {
        let id = self.session_seed.fetch_add(1, Ordering::Relaxed);
        format!("soda-native-ast-{id}")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listen = parse_listen_addr(std::env::args().skip(1))?;
    let state = AppState::new();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/mcp", post(handle_mcp).get(method_not_allowed))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn parse_listen_addr<I>(mut args: I) -> Result<SocketAddr, String>
where
    I: Iterator<Item = String>,
{
    let mut listen = "127.0.0.1:3002".to_string();
    while let Some(arg) = args.next() {
        if arg == "--listen" {
            let Some(value) = args.next() else {
                return Err("Parâmetro --listen sem valor".to_string());
            };
            listen = value;
        }
    }
    listen
        .parse::<SocketAddr>()
        .map_err(|e| format!("Endereço inválido para --listen: {e}"))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn method_not_allowed() -> impl IntoResponse {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(json!({
            "error": "Use POST /mcp para requisições MCP"
        })),
    )
}

async fn handle_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let request_id = payload.get("id").cloned().unwrap_or(Value::Null);
    let method = payload
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    match method {
        "initialize" => {
            let session_id = state.next_session_id();
            jsonrpc_ok(
                Some(&session_id),
                request_id,
                json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "soda-native-ast",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )
        }
        "notifications/initialized" => {
            let session_id = existing_session_id(&headers);
            let mut response_headers = HeaderMap::new();
            if let Some(session_id) = session_id {
                if let Ok(value) = HeaderValue::from_str(&session_id) {
                    response_headers.insert(MCP_SESSION_ID_HEADER, value);
                }
            }
            (StatusCode::ACCEPTED, response_headers, Json(json!({}))).into_response()
        }
        "tools/list" => {
            let session_id = existing_session_id(&headers);
            jsonrpc_ok(
                session_id.as_deref(),
                request_id,
                json!({
                    "tools": [
                        {
                            "name": "soda_get_ast",
                            "description": "Extrai o blueprint AST do repositório usando o parser nativo em Rust.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "repo_path": {
                                        "type": "string",
                                        "description": "Caminho absoluto do diretório do repositório."
                                    }
                                },
                                "required": ["repo_path"],
                                "additionalProperties": false
                            }
                        },
                        {
                            "name": "soda_fetch_web",
                            "description": "Busca uma URL com Tentativa Dupla nativa do SODA e retorna markdown limpo.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "url": {
                                        "type": "string",
                                        "description": "URL absoluta a ser buscada com reqwest + fallback robusto."
                                    }
                                },
                                "required": ["url"],
                                "additionalProperties": false
                            }
                        },
                        {
                            "name": "soda_github_meta",
                            "description": "Extrai metadados GitHub nativos via octocrab para owner/repo.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "owner_repo": {
                                        "type": "string",
                                        "description": "Identificador owner/repo do repositório GitHub."
                                    }
                                },
                                "required": ["owner_repo"],
                                "additionalProperties": false
                            }
                        },
                        {
                            "name": "soda_sqlite_query",
                            "description": "Executa consulta SQLite local em modo somente leitura nos bancos nativos do SODA.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": {
                                        "type": "string",
                                        "description": "Consulta SELECT/WITH/PRAGMA de leitura."
                                    },
                                    "db_name": {
                                        "type": "string",
                                        "description": "Banco alvo: soda_state.db, soda_heuristic_vault.db, state ou heuristic_vault."
                                    }
                                },
                                "required": ["query"],
                                "additionalProperties": false
                            }
                        }
                    ]
                }),
            )
        }
        "tools/call" => {
            let session_id = existing_session_id(&headers);
            match handle_tool_call(payload).await {
                Ok(result) => jsonrpc_ok(session_id.as_deref(), request_id, result),
                Err(error) => jsonrpc_error(
                    session_id.as_deref(),
                    request_id,
                    error.code,
                    &error.message,
                    error.data,
                ),
            }
        }
        _ => {
            let session_id = existing_session_id(&headers);
            jsonrpc_error(
                session_id.as_deref(),
                request_id,
                -32601,
                "Método MCP não suportado",
                Some(json!({ "method": method })),
            )
        }
    }
}

struct RpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

async fn handle_tool_call(payload: Value) -> Result<Value, RpcError> {
    let params = payload
        .get("params")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto params".to_string(),
            data: None,
        })?;
    let tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem campo name".to_string(),
            data: None,
        })?;

    match tool_name {
        "soda_get_ast" => run_soda_get_ast(params).await,
        "soda_fetch_web" => run_soda_fetch_web(params).await,
        "soda_github_meta" => run_soda_github_meta(params).await,
        "soda_sqlite_query" => run_soda_sqlite_query(params).await,
        other => Err(RpcError {
            code: -32601,
            message: "Ferramenta MCP desconhecida".to_string(),
            data: Some(json!({ "tool_name": other })),
        }),
    }
}

async fn run_soda_get_ast(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let repo_path_raw = arguments
        .get("repo_path")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento repo_path é obrigatório".to_string(),
            data: Some(json!({ "required": "repo_path" })),
        })?;

    let repo_path = PathBuf::from(repo_path_raw);
    validate_repo_path(&repo_path)?;

    let repo_path_for_task = repo_path.clone();
    let artifacts = tokio::task::spawn_blocking(move || {
        ast_parser::extract_repository_outline_native(
            &repo_path_for_task,
            400_000,
            120_000,
            32_000,
        )
    })
    .await
    .map_err(|e| RpcError {
        code: -32001,
        message: "Falha ao aguardar parser AST nativo".to_string(),
        data: Some(json!({ "reason": e.to_string() })),
    })?
    .map_err(|e| RpcError {
        code: -32002,
        message: "Falha ao extrair AST do repositório".to_string(),
        data: Some(json!({
            "repo_path": repo_path_raw,
            "reason": e.to_string()
        })),
    })?;

    let repo_outline = String::from_utf8(artifacts.repo_outline_blob).map_err(|e| RpcError {
        code: -32003,
        message: "blob_04_repo_outline inválido em UTF-8".to_string(),
        data: Some(json!({ "reason": e.to_string() })),
    })?;
    let architecture_map =
        String::from_utf8(artifacts.architecture_map_blob).map_err(|e| RpcError {
            code: -32004,
            message: "blob_05_architecture_map inválido em UTF-8".to_string(),
            data: Some(json!({ "reason": e.to_string() })),
        })?;
    let health_report = String::from_utf8(artifacts.health_report_blob).map_err(|e| RpcError {
        code: -32005,
        message: "blob_08_health_report inválido em UTF-8".to_string(),
        data: Some(json!({ "reason": e.to_string() })),
    })?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": repo_outline
            }
        ],
        "structuredContent": {
            "repo_path": repo_path_raw,
            "repo_outline": repo_outline,
            "architecture_map": architecture_map,
            "health_report": health_report
        },
        "isError": false
    }))
}

async fn run_soda_fetch_web(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let url = arguments
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento url é obrigatório".to_string(),
            data: Some(json!({ "required": "url" })),
        })?;

    let markdown = web_scraper::fetch_markdown_with_guarantee(url)
        .await
        .map_err(|e| RpcError {
            code: -32020,
            message: "Falha ao buscar conteúdo web com garantia".to_string(),
            data: Some(json!({
                "url": url,
                "reason": e.to_string()
            })),
        })?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": {
            "url": url,
            "markdown": markdown
        },
        "isError": false
    }))
}

async fn run_soda_github_meta(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let owner_repo = arguments
        .get("owner_repo")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento owner_repo é obrigatório".to_string(),
            data: Some(json!({ "required": "owner_repo" })),
        })?;

    let normalized_owner_repo =
        github_tracker::normalize_owner_repo(owner_repo).map_err(|e| RpcError {
            code: -32602,
            message: "owner_repo inválido".to_string(),
            data: Some(json!({
                "owner_repo": owner_repo,
                "reason": e.to_string()
            })),
        })?;

    let limiter = RateLimiter;
    let meta = github_tracker::fetch_community_meta_for_owner_repo(
        &normalized_owner_repo,
        &limiter,
        std::env::var("SODA_GITHUB_API_BASE_URL").ok().as_deref(),
    )
    .await
    .map_err(|e| {
        let (code, message) = match e {
            github_tracker::GithubTrackerError::MissingGithubToken => {
                (-32030, "GITHUB_TOKEN ausente para consulta GitHub")
            }
            github_tracker::GithubTrackerError::NotFound => {
                (-32031, "Repositório GitHub não encontrado")
            }
            github_tracker::GithubTrackerError::RateLimit => {
                (-32032, "GitHub bloqueou ou limitou a consulta")
            }
            github_tracker::GithubTrackerError::InvalidGithubUrl(_) => {
                (-32602, "owner_repo inválido")
            }
            _ => (-32033, "Falha ao consultar metadados GitHub")
        };
        RpcError {
            code,
            message: message.to_string(),
            data: Some(json!({
                "owner_repo": normalized_owner_repo,
                "reason": e.to_string()
            })),
        }
    })?;

    let markdown = format_github_meta_markdown(&normalized_owner_repo, &meta);

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": {
            "owner_repo": normalized_owner_repo,
            "stars": meta.stars_count,
            "forks": meta.forks_count,
            "open_issues": meta.open_issues_count,
            "open_prs": meta.open_prs_count,
            "license": meta.licenca,
            "last_commit_sha": meta.last_commit_sha,
            "last_commit_date": meta.last_commit_date,
            "recent_prs": meta.recent_prs
        },
        "isError": false
    }))
}

fn format_github_meta_markdown(
    owner_repo: &str,
    meta: &genesis_mc_lib::harvester::community::CommunityMetaPayload,
) -> String {
    let mut out = String::new();
    out.push_str("# GitHub Meta\n\n");
    out.push_str(&format!("- Repository: `{}`\n", owner_repo));
    out.push_str(&format!("- Stars: `{}`\n", meta.stars_count));
    out.push_str(&format!("- Forks: `{}`\n", meta.forks_count));
    out.push_str(&format!("- Open Issues: `{}`\n", meta.open_issues_count));
    out.push_str(&format!("- Open PRs: `{}`\n", meta.open_prs_count));
    out.push_str(&format!("- License: `{}`\n", meta.licenca));
    if let Some(description) = meta.description.as_deref() {
        out.push_str(&format!("- Description: {}\n", description));
    }
    if let Some(last_commit_sha) = meta.last_commit_sha.as_deref() {
        out.push_str(&format!("- Last Commit SHA: `{}`\n", last_commit_sha));
    }
    if let Some(last_commit_date) = meta.last_commit_date.as_ref() {
        out.push_str(&format!("- Last Commit Date: `{}`\n", last_commit_date));
    }
    out.push_str("\n## Recent PRs\n");
    if meta.recent_prs.is_empty() {
        out.push_str("- `<none>`\n");
    } else {
        for pr in &meta.recent_prs {
            out.push_str(&format!(
                "- `#{}`
 `{}` updated `{}`\n",
                pr.number, pr.state, pr.updated_at
            ));
        }
    }
    out
}

async fn run_soda_sqlite_query(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento query é obrigatório".to_string(),
            data: Some(json!({ "required": "query" })),
        })?;

    validate_sqlite_query(query)?;

    let db_name = arguments
        .get("db_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("soda_state.db");
    let db_path = resolve_sqlite_db_path(db_name)?;

    let query_owned = query.to_string();
    let db_name_owned = db_name.to_string();
    let db_path_for_task = db_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        execute_sqlite_read_only_query(&db_name_owned, &db_path_for_task, &query_owned)
    })
    .await
    .map_err(|e| RpcError {
        code: -32040,
        message: "Falha ao aguardar worker SQLite nativo".to_string(),
        data: Some(json!({ "reason": e.to_string() })),
    })??;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": result.markdown
            }
        ],
        "structuredContent": {
            "db_name": result.db_name,
            "db_path": result.db_path,
            "query": query,
            "columns": result.columns,
            "rows": result.rows,
            "truncated": result.truncated
        },
        "isError": false
    }))
}

#[derive(Debug)]
struct SqliteQueryOutput {
    db_name: String,
    db_path: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    truncated: bool,
    markdown: String,
}

fn execute_sqlite_read_only_query(
    db_name: &str,
    db_path: &Path,
    query: &str,
) -> Result<SqliteQueryOutput, RpcError> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| RpcError {
        code: -32041,
        message: "Falha ao abrir banco SQLite em modo somente leitura".to_string(),
        data: Some(json!({
            "db_name": db_name,
            "db_path": db_path.display().to_string(),
            "reason": e.to_string()
        })),
    })?;

    let mut stmt = conn.prepare(query).map_err(|e| RpcError {
        code: -32042,
        message: "Falha sintática ou semântica ao preparar query SQLite".to_string(),
        data: Some(json!({
            "db_name": db_name,
            "reason": e.to_string()
        })),
    })?;

    let columns = stmt
        .column_names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return Err(RpcError {
            code: -32602,
            message: "A query não retornou colunas; apenas SELECT/WITH/PRAGMA informacional são permitidos".to_string(),
            data: Some(json!({ "query": query })),
        });
    }

    let mut rows = Vec::<Vec<String>>::new();
    let mut query_rows = stmt.query([]).map_err(|e| RpcError {
        code: -32043,
        message: "Falha ao executar query SQLite".to_string(),
        data: Some(json!({
            "db_name": db_name,
            "reason": e.to_string()
        })),
    })?;

    let mut truncated = false;
    while let Some(row) = query_rows.next().map_err(|e| RpcError {
        code: -32044,
        message: "Falha ao iterar linhas SQLite".to_string(),
        data: Some(json!({
            "db_name": db_name,
            "reason": e.to_string()
        })),
    })? {
        if rows.len() >= SQLITE_MAX_ROWS {
            truncated = true;
            break;
        }
        rows.push(extract_sqlite_row(row, columns.len())?);
    }

    let markdown = format_sqlite_result_markdown(
        db_name,
        &db_path.display().to_string(),
        query,
        &columns,
        &rows,
        truncated,
    );

    Ok(SqliteQueryOutput {
        db_name: db_name.to_string(),
        db_path: db_path.display().to_string(),
        columns,
        rows,
        truncated,
        markdown,
    })
}

fn extract_sqlite_row(row: &rusqlite::Row<'_>, column_count: usize) -> Result<Vec<String>, RpcError> {
    let mut values = Vec::with_capacity(column_count);
    for index in 0..column_count {
        let value = row.get_ref(index).map_err(|e| RpcError {
            code: -32045,
            message: "Falha ao ler célula SQLite".to_string(),
            data: Some(json!({
                "column_index": index,
                "reason": e.to_string()
            })),
        })?;
        values.push(sqlite_value_to_string(value));
    }
    Ok(values)
}

fn sqlite_value_to_string(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(v) => v.to_string(),
        ValueRef::Real(v) => v.to_string(),
        ValueRef::Text(v) => String::from_utf8_lossy(v).to_string(),
        ValueRef::Blob(v) => format!("<blob:{} bytes>", v.len()),
    }
}

fn format_sqlite_result_markdown(
    db_name: &str,
    db_path: &str,
    query: &str,
    columns: &[String],
    rows: &[Vec<String>],
    truncated: bool,
) -> String {
    let mut out = String::new();
    out.push_str("# SQLite Query\n\n");
    out.push_str(&format!("- Database: `{}`\n", db_name));
    out.push_str(&format!("- Path: `{}`\n", db_path));
    out.push_str(&format!("- Rows: `{}`\n", rows.len()));
    out.push_str(&format!("- Truncated: `{}`\n\n", truncated));
    out.push_str("```sql\n");
    out.push_str(query.trim());
    out.push_str("\n```\n\n");

    if columns.is_empty() {
        out.push_str("_No columns returned._\n");
        return out;
    }

    out.push('|');
    for column in columns {
        out.push(' ');
        out.push_str(&escape_markdown_cell(column));
        out.push(' ');
        out.push('|');
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
            out.push(' ');
            out.push_str(&escape_markdown_cell(cell));
            out.push(' ');
            out.push('|');
        }
        out.push('\n');
    }

    if rows.is_empty() {
        out.push_str("| _empty_ |\n");
    }
    if truncated {
        out.push_str("\n_Note: resultado truncado em 200 linhas._\n");
    }
    out
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br/>")
}

fn resolve_sqlite_db_path(db_name: &str) -> Result<PathBuf, RpcError> {
    let normalized = db_name.trim().to_ascii_lowercase();
    let file_name = match normalized.as_str() {
        "" | "state" | "soda_state" | "soda_state.db" => "soda_state.db",
        "vault" | "heuristic_vault" | "soda_heuristic_vault" | "soda_heuristic_vault.db" => {
            "soda_heuristic_vault.db"
        }
        other => {
            return Err(RpcError {
                code: -32602,
                message: "db_name inválido; use soda_state.db ou soda_heuristic_vault.db".to_string(),
                data: Some(json!({ "db_name": other })),
            })
        }
    };

    let path = workspace_root().join(".soda_data").join(file_name);
    if !path.exists() {
        return Err(RpcError {
            code: -32046,
            message: "Arquivo SQLite solicitado não existe".to_string(),
            data: Some(json!({
                "db_name": file_name,
                "db_path": path.display().to_string()
            })),
        });
    }
    Ok(path)
}

fn validate_sqlite_query(query: &str) -> Result<(), RpcError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(RpcError {
            code: -32602,
            message: "Query vazia".to_string(),
            data: None,
        });
    }

    let normalized = trimmed.trim_end_matches(';').trim();
    if normalized.contains(';') {
        return Err(RpcError {
            code: -32602,
            message: "Apenas uma única query é permitida".to_string(),
            data: Some(json!({ "query": query })),
        });
    }

    let lower = normalized.to_ascii_lowercase();
    for forbidden in [
        "insert",
        "update",
        "delete",
        "drop",
        "alter",
        "create",
        "replace",
        "truncate",
        "attach",
        "detach",
        "vacuum",
        "reindex",
        "analyze",
        "begin",
        "commit",
        "rollback",
    ] {
        if lower.split_whitespace().any(|token| token == forbidden) {
            return Err(RpcError {
                code: -32602,
                message: "Query destrutiva ou mutável bloqueada pelo cofre SQLite".to_string(),
                data: Some(json!({
                    "blocked_token": forbidden
                })),
            });
        }
    }

    let first_token = lower
        .split_whitespace()
        .next()
        .unwrap_or_default();
    let is_allowed = matches!(first_token, "select" | "with" | "pragma");
    if !is_allowed {
        return Err(RpcError {
            code: -32602,
            message: "Somente SELECT, WITH e PRAGMA informacional são permitidos".to_string(),
            data: Some(json!({ "first_token": first_token })),
        });
    }

    if first_token == "pragma" && lower.contains('=') {
        return Err(RpcError {
            code: -32602,
            message: "PRAGMA mutável bloqueado; apenas PRAGMA informacional é permitido".to_string(),
            data: Some(json!({ "query": query })),
        });
    }

    Ok(())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn validate_repo_path(repo_path: &Path) -> Result<(), RpcError> {
    if !repo_path.exists() {
        return Err(RpcError {
            code: -32010,
            message: "Diretório do repositório não existe".to_string(),
            data: Some(json!({ "repo_path": repo_path.display().to_string() })),
        });
    }
    if !repo_path.is_dir() {
        return Err(RpcError {
            code: -32011,
            message: "repo_path não aponta para um diretório".to_string(),
            data: Some(json!({ "repo_path": repo_path.display().to_string() })),
        });
    }
    Ok(())
}

fn existing_session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(MCP_SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

fn jsonrpc_ok(session_id: Option<&str>, request_id: Value, result: Value) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    if let Some(session_id) = session_id {
        if let Ok(value) = HeaderValue::from_str(session_id) {
            headers.insert(MCP_SESSION_ID_HEADER, value);
        }
    }
    (
        StatusCode::OK,
        headers,
        Json(json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": result
        })),
    )
        .into_response()
}

fn jsonrpc_error(
    session_id: Option<&str>,
    request_id: Value,
    code: i64,
    message: &str,
    data: Option<Value>,
) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    if let Some(session_id) = session_id {
        if let Ok(value) = HeaderValue::from_str(session_id) {
            headers.insert(MCP_SESSION_ID_HEADER, value);
        }
    }
    (
        StatusCode::OK,
        headers,
        Json(json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {
                "code": code,
                "message": message,
                "data": data
            }
        })),
    )
        .into_response()
}
