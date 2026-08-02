// Aumento do limite de recursão do macro `json!` (serde_json) para acomodar
// o `tools/list` canônico do Marco 3.5 (50+ ferramentas, incluindo os 10
// novos tools mem_* + core_think com inputSchemas profundos).
#![recursion_limit = "1024"]
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};
use souls_mc_lib::cognition::lean_vacuum;
use souls_mc_lib::cognition::context_compression; // SOULS-CANIBALIZED Marco 3.6: Conveyor Belt (CCR Lossless)
use souls_mc_lib::cognition::memory_graph;
use souls_mc_lib::cognition::memory_graph::mpsc_bridge::MemGraphOp;
use souls_mc_lib::cognition::memory_graph::types::{Entity, ObservationInput, Relation};
use souls_mc_lib::cognition::thinking::types::{ThoughtData, ThinkingResponse};
use souls_mc_lib::cognition::thinking::ThinkingEngine;
use souls_mc_lib::harvester::ast_parser;
use souls_mc_lib::harvester::community::RateLimiter;
use souls_mc_lib::harvester::github_tracker;
use souls_mc_lib::harvester::repo_radar;
use souls_mc_lib::harvester::web_scraper;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::{Value, json};
use sqlparser::ast::Statement as SqlStatement;
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use url::Url;


const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const SQLITE_MAX_ROWS: usize = 200;

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
        // ── Lê a próxima linha NDJSON do stdin ───────────────────────────────
        // EOF limpo (Ok(None)) → encerra o loop naturalmente.
        // Erro de I/O        → encerra (pipe quebrado pelo gateway).
        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break, // EOF limpo — gateway fechou o pipe corretamente
            Err(e) => {
                eprintln!("[souls_mcp_server] ERRO I/O no stdin: {e}");
                break;
            }
        };

        // ── Sanitização O(1) Anti-BOM ────────────────────────────────────────
        // O Windows injeta BOM UTF-8 (U+FEFF = EF BB BF) no stdin de processos
        // filhos criados via pipe. str::trim() NÃO remove BOM (não é whitespace
        // ASCII). Removemos o BOM antes do trim e antes do parse JSON.
        let payload_str = line.trim_start_matches('\u{FEFF}').trim();
        if payload_str.is_empty() {
            continue; // linha em branco entre mensagens — ignorar silenciosamente
        }

        // ── Desserialização Fail-Soft ────────────────────────────────────────
        // Entrada inválida (ex: "oie", fragmento HTTP, linha de log) NUNCA mata
        // o processo. Loga no stderr e continua aguardando a próxima linha.
        let payload: Value = match serde_json::from_str(payload_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "[souls_mcp_server] JSON inválido ignorado (fail-soft): {e} | input={:.120}",
                    payload_str
                );
                continue; // ← NUNCA break aqui — resiliência obrigatória
            }
        };

        if let Some(resp) = handle_mcp(payload).await {
            let resp_str = serde_json::to_string(&resp)?;
            // ── Emissão NDJSON pura no stdout ────────────────────────────────
            // Protocolo estrito: <json>\n  — sem Content-Length, sem headers HTTP.
            if let Err(e) = stdout.write_all(resp_str.as_bytes()).await {
                eprintln!("[souls_mcp_server] ERRO ao escrever resposta no stdout: {e}");
                break; // pipe de saída morreu — encerramento legítimo
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
        "tools/list" => Some(jsonrpc_ok(
            request_id,
            json!({
                "tools": [
                    {
                        "name": "get_ast",
                        "description": "Extrai o blueprint AST do repositório usando o parser nativo em Rust. (Cânone SOULS, ex-repo_ast)",
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
                        "name": "fetch_web",
                        "description": "Busca uma URL com Tentativa Dupla nativa do SOULS e retorna markdown limpo. (Cânone SOULS, ex-web_fetch)",
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
                        "name": "sys_time",
                        "description": "Retorna data/hora local, UTC e fuso atual via chrono nativo. (Cânone SOULS, ex-sys_time)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "web_search",
                        "description": "Executa busca web nativa contra DuckDuckGo HTML e retorna titulos, links e snippets. (Cânone SOULS, ex-web_search)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Consulta textual a ser enviada ao DuckDuckGo HTML."
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Numero maximo de resultados retornados (1-10, padrao 5).",
                                    "minimum": 1,
                                    "maximum": 10
                                }
                            },
                            "required": ["query"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "repo_meta",
                        "description": "Extrai metadados GitHub nativos via octocrab para owner/repo. (Cânone SOULS, ex-repo_meta)",
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
                        "name": "sqlite_query",
                        "description": "Executa consulta SQLite local em modo somente leitura nos bancos nativos do SOULS. (Cânone SOULS, ex-db_query)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Consulta SELECT/WITH/PRAGMA de leitura."
                                },
                                "db_name": {
                                    "type": "string",
                                    "description": "Banco alvo: souls_state.db, souls_heuristic_vault.db, state ou heuristic_vault."
                                }
                            },
                            "required": ["query"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "sub_agent",
                        "description": "Registra ou atualiza o estado de um subagente no banco de estado L2 (souls_state.db).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_id": { "type": "string", "description": "ID único do subagente." },
                                "task_name": { "type": "string", "description": "Nome da tarefa executada." },
                                "status": { "type": "string", "description": "Status do subagente (RUNNING, DONE, FAILED)." },
                                "context_data": { "type": "string", "description": "Dados adicionais de contexto." }
                            },
                            "required": ["agent_id", "task_name", "status"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "handoff",
                        "description": "Registra a transferência de contexto (handoff) entre subagentes no banco de estado L2 (souls_state.db).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "handoff_id": { "type": "string", "description": "ID único do handoff." },
                                "from_agent": { "type": "string", "description": "Agente de origem." },
                                "to_agent": { "type": "string", "description": "Agente de destino." },
                                "payload": { "type": "string", "description": "Conteúdo/payload do handoff." },
                                "status": { "type": "string", "description": "Status do handoff (PENDING, COMPLETED)." }
                            },
                            "required": ["handoff_id", "from_agent", "to_agent", "payload"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "knowledge",
                        "description": "Armazena ou atualiza uma entrada de conhecimento no banco de estado L2 (souls_state.db).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "key": { "type": "string", "description": "Chave/identificador da entrada." },
                                "category": { "type": "string", "description": "Categoria do conhecimento." },
                                "content": { "type": "string", "description": "Conteúdo textual." },
                                "confidence": { "type": "number", "description": "Nível de confiança (0.0 a 1.0)." }
                            },
                            "required": ["key", "category", "content"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "edit",
                        "description": "Edita cirurgicamente um arquivo existente substituindo old_string por new_string com trava atômica (Fail-Closed).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string", "description": "Caminho do arquivo a ser editado." },
                                "old_string": { "type": "string", "description": "Trecho exato a ser substituído." },
                                "new_string": { "type": "string", "description": "Novo conteúdo de substituição." }
                            },
                            "required": ["path", "old_string", "new_string"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "fill",
                        "description": "Injeta bloco funcional no offset de stub (souls-stub: marker) sem alterar a casca adjacente.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string", "description": "Caminho do arquivo contendo o stub (alias: path)." },
                                "stub_marker": { "type": "string", "description": "Marcador do stub a ser preenchido (alias: marker)." },
                                "code_payload": { "type": "string", "description": "Bloco de código a ser injetado (alias: content)." }
                            },
                            "required": ["file_path", "stub_marker", "code_payload"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "headroom_retrieve",
                        "description": "Recupera stub comprimido via CCR (intercept_loopback). Hash hex 16B/32ch. SSE Tokio < 1ms.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "hash": { "type": "string", "description": "Hash hex de 16 bytes (32 chars) emitido por souls_fill ou souls_compress." }
                            },
                            "required": ["hash"],
                            "additionalProperties": false
                        }
                    },
                    // ============================================================
                    // SOULS-CANIBALIZED Marco 3.6: Conveyor Belt de Contexto (CCR Lossless)
                    // ============================================================
                    {
                        "name": "souls_multi_read",
                        "description": "Lê múltiplos arquivos em lote na RAM de forma assíncrona aplicando compressão de contexto CCR.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "paths": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Lista de caminhos de arquivos a serem lidos em paralelo via tokio::fs."
                                }
                            },
                            "required": ["paths"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "souls_stub_fill",
                        "description": "Reidrata e expande marcadores de compressão CCR de volta para o texto original lossless na RAM.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": { "type": "string", "description": "Texto compactado contendo marcadores [SOULS-DEDUP: ...] a serem reidratados." }
                            },
                            "required": ["text"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "souls_stub_fill",
                        "description": "Injeta bloco funcional no offset de stub (souls-stub: marker) sem alterar a casca adjacente. (Legacy fill)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string", "description": "Caminho do arquivo contendo o stub (alias: path)." },
                                "stub_marker": { "type": "string", "description": "Marcador do stub a ser preenchido (alias: marker)." },
                                "code_payload": { "type": "string", "description": "Bloco de código a ser injetado (alias: content)." }
                            },
                            "required": ["file_path", "stub_marker", "code_payload"],
                            "additionalProperties": false
                        }
                    },
                    // ============================================================
                    // SOULS-CANIBALIZED: 17 tools canônicas (2 implementadas + 15 stubs)
                    // ============================================================
                    {
                        "name": "read",
                        "description": "Lê arquivo com TOON + SymbolMap (transplantado do lean-ctx ctx_read).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "delta_diff",
                        "description": "Myers diff estrutural (transplantado do lean-ctx ctx_delta via crate similar).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "before": { "type": "string" },
                                "after": { "type": "string" }
                            },
                            "required": ["before", "after"],
                            "additionalProperties": false
                        }
                    },
                    // Stubs (15) - contratos canônicos para cobertura semântica.
                    // Implementação real virá em iterações SOULS-SDD subsequentes (Fase 4+).
                    { "name": "multi_read", "description": "not_implemented_yet: Leitura em batch com dedup via SharedBlock.", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    {
                        "name": "smart_read",
                        "description": "Lê arquivo com medição prévia de tokens na CPU (tiktoken cl100k_base) e auto-shrink adaptativo. (souls_smart_read)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string", "description": "Caminho do arquivo a ser lido (alias: path)." },
                                "path": { "type": "string", "description": "Caminho do arquivo a ser lido." },
                                "max_tokens_budget": { "type": "integer", "description": "Limite máximo de tokens (padrão: 8000)." }
                            },
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "search",
                        "description": "Busca textual compacta via regex com formatação LEAN agrupada por arquivo. (souls_search)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string", "description": "Expressão regular ou termo a buscar." },
                                "pattern": { "type": "string", "description": "Alias para query." },
                                "path": { "type": "string", "description": "Diretório inicial para busca (padrão: '.')." },
                                "search_path": { "type": "string", "description": "Alias para path." },
                                "max_depth": { "type": "integer", "description": "Profundidade máxima de diretórios (padrão: 5)." }
                            },
                            "required": ["query"],
                            "additionalProperties": false
                        }
                    },
                    { "name": "semantic_search", "description": "not_implemented_yet: BM25 + cosine fusion (gated embeddings).", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    {
                        "name": "tree",
                        "description": "Lente de diretórios não-bloqueante com Dot-Flattening estrito e exclusão de caminhos tóxicos. (souls_tree)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string", "description": "Caminho relativo ou absoluto do diretório raiz." },
                                "depth": { "type": "integer", "description": "Profundidade máxima de varredura (padrão: 3)." }
                            },
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "outline",
                        "description": "Extrai assinaturas AST sem corpos de funções via sandbox Wasmtime WASI 0.2. (souls_outline / souls_symbol)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string", "description": "Caminho do arquivo de código." }
                            },
                            "required": ["file_path"],
                            "additionalProperties": false
                        }
                    },
                    { "name": "symbol", "description": "not_implemented_yet: Resolve symbol name → file:line.", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    { "name": "callers", "description": "not_implemented_yet: Call graph: quem chama esta fn.", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    { "name": "callees", "description": "not_implemented_yet: Call graph: o que esta fn chama.", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    { "name": "execute", "description": "not_implemented_yet sandbox_audit_pending: execução multi-lang requer auditoria.", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    { "name": "shell", "description": "not_implemented_yet sandbox_audit_pending: shell command com whitelist/timeout requer auditoria.", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    {
                        "name": "compress",
                        "description": "Aplica o compressor LEAN ao texto fornecido removendo comentários e reduzindo ruído. (souls_compress)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": { "type": "string", "description": "Texto bruto a ser compactado." },
                                "ext": { "type": "string", "description": "Extensão opcional do arquivo para regras sintáticas." }
                            },
                            "required": ["text"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "dedup",
                        "description": "Detecta e substitui blocos duplicados de 5 linhas consecutivas por marcadores de deduplicação. (souls_dedup)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": { "type": "string", "description": "Texto a ser deduplicado." }
                            },
                            "required": ["text"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "session",
                        "description": "Gerencia o ciclo de vida da sessão SOULS/MCP e executa a limpeza/vacina de cache de RAM (souls_session).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "action": {
                                    "type": "string",
                                    "description": "Ação a ser executada: 'clear' ou 'reset' para descarte de RAM/dedup cache, 'status' para verificar estado."
                                }
                            },
                            "additionalProperties": false
                        }
                    },
                    { "name": "metrics", "description": "not_implemented_yet: Métricas: tokens lidos/salvos, hit-rate cache.", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    { "name": "intent", "description": "not_implemented_yet: Detecta intent do tool call (read/edit/search).", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
                    // ============================================================
                    // SOULS-CANIBALIZED Marco 3.5: 9 tools do `souls_graph` + 1 do `souls_thinking` (core_think)
                    // ============================================================
                    {
                        "name": "mem_create_entities",
                        "description": "Cria entidades no grafo cognitivo (souls_graph). Idempotente (INSERT OR IGNORE). Persistencia MPSC.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "entities": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" },
                                            "entityType": { "type": "string" },
                                            "observations": { "type": "array", "items": { "type": "string" } }
                                        },
                                        "required": ["name", "entityType"]
                                    }
                                }
                            },
                            "required": ["entities"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_create_relations",
                        "description": "Cria relações direcionadas entre entidades (souls_graph). Idempotente.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "relations": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "from": { "type": "string" },
                                            "to": { "type": "string" },
                                            "relationType": { "type": "string" }
                                        },
                                        "required": ["from", "to", "relationType"]
                                    }
                                }
                            },
                            "required": ["relations"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_add_observations",
                        "description": "Anexa observações a entidades existentes. Triggers FTS5 mantêm observations_fts em sincronia.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "observations": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "entityName": { "type": "string" },
                                            "contents": { "type": "array", "items": { "type": "string" } }
                                        },
                                        "required": ["entityName", "contents"]
                                    }
                                }
                            },
                            "required": ["observations"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_search",
                        "description": "Busca FTS5 síncrona por MATCH em observations_fts. Retorna entidades distintas com metadata.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string", "description": "Query FTS5 (sintaxe MATCH)." },
                                "limit": { "type": "integer", "description": "Limite de entidades retornadas (padrão: 50)." }
                            },
                            "required": ["query"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_open_nodes",
                        "description": "Abre entidades específicas por nome e hidrata observations via JOIN.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "names": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["names"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_read_graph",
                        "description": "Lê o grafo inteiro (entities + relations) com LIMIT defensivo (default: 500).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "limit": { "type": "integer", "description": "Teto de elementos (default 500, max 500)." }
                            },
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_delete_entities",
                        "description": "Remove entidades. CASCADE apaga relations e observations. Uso restrito (HITL recomendado).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "entityNames": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["entityNames"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_delete_observations",
                        "description": "Remove observações específicas (entity, content). Triggers FTS5 mantêm consistência.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "deletions": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "entityName": { "type": "string" },
                                            "observations": { "type": "array", "items": { "type": "string" } }
                                        },
                                        "required": ["entityName", "observations"]
                                    }
                                }
                            },
                            "required": ["deletions"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "mem_delete_relations",
                        "description": "Remove arestas do grafo.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "relations": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "from": { "type": "string" },
                                            "to": { "type": "string" },
                                            "relationType": { "type": "string" }
                                        },
                                        "required": ["from", "to", "relationType"]
                                    }
                                }
                            },
                            "required": ["relations"],
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "core_think",
                        "description": "Scratchpad socratico (souls_thinking). Limite 5 (HITL 7). Tride Regular/Revision/Branching.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "thought": { "type": "string", "description": "Conteúdo do pensamento." },
                                "thoughtNumber": { "type": "integer", "description": "Índice 1-based do pensamento." },
                                "totalThoughts": { "type": "integer", "description": "Estimativa total (ajustável)." },
                                "nextThoughtNeeded": { "type": "boolean", "description": "true se ainda há pensamentos por vir." },
                                "isRevision": { "type": "boolean", "description": "true se revisa um pensamento anterior." },
                                "revisesThought": { "type": "integer", "description": "Índice do pensamento revisado (obrigatório se isRevision=true)." },
                                "branchFromThought": { "type": "integer", "description": "Índice do nó-pai do branch." },
                                "branchId": { "type": "string", "description": "ID único do branch." },
                                "needsMoreThoughts": { "type": "boolean", "description": "true se o orçamento precisar ser expandido." },
                                "hitlAuthorized": { "type": "boolean", "description": "Sinal server-side do Arquiteto: estica teto de 5 para 7." }
                            },
                            "required": ["thought", "thoughtNumber", "totalThoughts", "nextThoughtNeeded"],
                            "additionalProperties": false
                        }
                    }
                ]
            }),
        )),
        "tools/call" => match handle_tool_call(payload).await {
            Ok(result) => Some(jsonrpc_ok(request_id, result)),
            Err(error) => Some(jsonrpc_error(
                request_id,
                error.code,
                &error.message,
                error.data,
            )),
        },
        _ => Some(jsonrpc_error(
            request_id,
            -32601,
            "Método MCP não suportado",
            Some(json!({ "method": method })),
        )),
    }
}

#[derive(Debug)]
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

    // SOULS-CANIBALIZED: higiene canônica. Aceita tanto nomes simples quanto prefixados/aliases.
    match tool_name {
        // Cânone SOULS (preferido)
        "get_ast" | "souls_get_ast" | "repo_ast" => run_repo_ast(params).await,
        "fetch_web" | "souls_fetch_web" | "web_fetch" => run_web_fetch(params).await,
        "sys_time" | "souls_sys_time" => run_sys_time(params).await,
        "web_search" | "souls_web_search" => run_web_search(params).await,
        "repo_meta" | "souls_repo_meta" => run_repo_meta(params).await,
        "sqlite_query" | "souls_sqlite_query" | "db_query" => run_db_query(params).await,
        "sub_agent" | "souls_sub_agent" => run_souls_sub_agent(params).await,
        "handoff" | "souls_handoff" => run_souls_handoff(params).await,
        "knowledge" | "souls_knowledge" => run_souls_knowledge(params).await,
        "edit" | "souls_edit" => run_souls_edit(params).await,
        "fill" | "souls_stub_fill" | "stub_fill" => run_souls_stub_fill(params).await,
        "souls_fill" | "ccr_fill" => run_souls_ccr_fill(params).await,
        // 17 tools canônicas (2 implementadas + 15 stubs)
        "read" | "souls_read" => run_souls_read(params).await,
        "delta_diff" | "souls_delta_diff" => run_souls_delta_diff(params).await,
        "tree" | "souls_tree" => run_souls_tree(params).await,
        "outline" | "souls_outline" | "symbol" | "souls_symbol" => run_souls_outline(params).await,
        "smart_read" | "souls_smart_read" => run_souls_smart_read(params).await,
        "search" | "souls_search" => run_souls_search(params).await,
        "compress" | "souls_compress" => run_souls_compress(params).await,
        "dedup" | "souls_dedup" => run_souls_dedup(params).await,
        "headroom_retrieve" | "souls_headroom_retrieve" => run_souls_headroom_retrieve(params).await,
        "session" | "souls_session" => run_souls_session(params).await,
        "multi_read" | "souls_multi_read" => run_souls_multi_read(params).await,
        "semantic_search" | "souls_semantic_search"
        | "callers" | "souls_callers"
        | "callees" | "souls_callees"
        | "metrics" | "souls_metrics"
        | "intent" | "souls_intent" => Ok(stub_not_implemented_yet(tool_name)),
        "execute" | "souls_execute" => Ok(stub_sandbox_audit_pending(tool_name)),
        "shell" | "souls_shell" => run_souls_shell(params).await,
        // Marco 3.5 — souls_graph (9 ops) + souls_thinking (1 op = core_think)
        "mem_create_entities" => run_mem_create_entities(params).await,
        "mem_create_relations" => run_mem_create_relations(params).await,
        "mem_add_observations" => run_mem_add_observations(params).await,
        "mem_search" => run_mem_search(params).await,
        "mem_open_nodes" => run_mem_open_nodes(params).await,
        "mem_read_graph" => run_mem_read_graph(params).await,
        "mem_delete_entities" => run_mem_delete_entities(params).await,
        "mem_delete_observations" => run_mem_delete_observations(params).await,
        "mem_delete_relations" => run_mem_delete_relations(params).await,
        "core_think" => run_core_think(params).await,
        other => Err(RpcError {
            code: -32601,
            message: "Ferramenta MCP desconhecida".to_string(),
            data: Some(json!({ "tool_name": other })),
        }),
    }
}

// =============================================================================
// SOULS-CANIBALIZED Fase 3: Implementação real das 2 ferramentas vitais.
// O transplante usa o módulo nativo `lean_vacuum` (cognition/lean_vacuum/).
// As outras 15 ferramentas permanecem como `not_implemented_yet` stubs.
// =============================================================================

/// `souls_read` — Lê arquivo + Saco a Vácuo nativo.
async fn run_souls_read(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let path_str = arguments
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento path é obrigatório".to_string(),
            data: Some(json!({ "required": "path" })),
        })?;

    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Err(RpcError {
            code: -32010,
            message: "Arquivo não existe".to_string(),
            data: Some(json!({ "path": path.display().to_string() })),
        });
    }
    if !path.is_file() {
        return Err(RpcError {
            code: -32011,
            message: "path não aponta para um arquivo regular".to_string(),
            data: Some(json!({ "path": path.display().to_string() })),
        });
    }

    let raw = std::fs::read_to_string(&path).map_err(|e| RpcError {
        code: -32012,
        message: "Falha ao ler arquivo (pode ser > 5MB ou binário)".to_string(),
        data: Some(json!({
            "path": path.display().to_string(),
            "reason": e.to_string(),
        })),
    })?;

    let original_chars = raw.chars().count();
    let ext = path.extension().and_then(|e| e.to_str());
    let compressed = lean_vacuum::compress_to_lean(&raw, ext);
    let compressed_chars = compressed.chars().count();

    let ratio = if original_chars == 0 {
        1.0
    } else {
        compressed_chars as f64 / original_chars as f64
    };
    let saved_pct = ((1.0 - ratio) * 100.0).round() as i64;

    let header = format!(
        "# {path} ({original}→{compressed} chars, {saved}% saved)\n\n",
        path = path.display(),
        original = original_chars,
        compressed = compressed_chars,
        saved = saved_pct,
    );
    let body = format!("```\n{compressed}\n```");

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("{header}{body}")
        }],
        "structuredContent": {
            "path": path.display().to_string(),
            "original_chars": original_chars,
            "compressed_chars": compressed_chars,
            "compression_ratio": ratio,
            "saved_percent": saved_pct,
            "ext": ext,
            "engine": "lean_vacuum.native (Fase 3)",
        },
        "isError": false
    }))
}

/// `souls_delta_diff` — Myers Diff estrutural via crate `similar` 2.7.0.
async fn run_souls_delta_diff(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let before = arguments
        .get("before")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento before é obrigatório (string)".to_string(),
            data: Some(json!({ "required": "before" })),
        })?;
    let after = arguments
        .get("after")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento after é obrigatório (string)".to_string(),
            data: Some(json!({ "required": "after" })),
        })?;

    let (text, stats) = lean_vacuum::myers_diff::myers_diff_with_stats(before, after);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "structuredContent": {
            "before_chars": before.chars().count(),
            "after_chars": after.chars().count(),
            "additions": stats.additions,
            "deletions": stats.deletions,
            "unchanged": stats.unchanged,
            "engine": "similar 2.7.0 (Myers)",
        },
        "isError": false
    }))
}

/// `souls_compress` — Aplica o compressor LEAN nativo ao texto.
async fn run_souls_compress(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let text = arguments
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento text é obrigatório".to_string(),
            data: Some(json!({ "required": "text" })),
        })?;
    let ext = arguments.get("ext").and_then(Value::as_str);

    let compressed = lean_vacuum::compress_to_lean(text, ext);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": compressed
        }],
        "structuredContent": {
            "compressed_text": compressed
        },
        "isError": false
    }))
}

/// `souls_dedup` — Deduplicação de blocos de 5+ linhas consecutivas.
async fn run_souls_dedup(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;
    let text = arguments
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento text é obrigatório".to_string(),
            data: Some(json!({ "required": "text" })),
        })?;

    let path_opt = arguments
        .get("path")
        .or_else(|| arguments.get("file_path"))
        .and_then(Value::as_str)
        .map(Path::new);

    let deduplicated = lean_vacuum::deduplicate_blocks_session(text, path_opt);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": deduplicated
        }],
        "structuredContent": {
            "deduplicated_text": deduplicated
        },
        "isError": false
    }))
}

/// `souls_headroom_retrieve` — Recupera um stub comprimido via `SoulsCcrStore::intercept_loopback`.
/// Stub inicial: enquadra a chamada como `intercept_loopback` espera (com `headroom_retrieve`
/// no JSON e campo `hash`).
async fn run_souls_headroom_retrieve(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);

    let hash = args
        .get("hash")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'hash' ausente".to_string(),
            data: None,
        })?;

    // Enquadra como `intercept_loopback` espera sem interpolação manual insegura.
    let tool_call_json = json!({
        "headroom_retrieve": true,
        "hash": hash,
    })
    .to_string();

    let store = souls_mc_lib::core::headroom_engine::SoulsCcrStore::from_env();
    let retrieved = store.intercept_loopback(&tool_call_json);

    match retrieved {
        Some(payload) => Ok(json!({
            "content": [{
                "type": "text",
                "text": payload
            }],
            "structuredContent": { "retrieved": true },
            "isError": false
        })),
        None => Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("Hash '{hash}' nao encontrado no CCR store (loopback miss).")
            }],
            "structuredContent": { "retrieved": false },
            "isError": false
        })),
    }
}

/// `souls_smart_read` — Leitura Token-Aware com Auto-Shrink e Fail-Closed.
async fn run_souls_smart_read(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let path_str = arguments
        .get("file_path")
        .or_else(|| arguments.get("path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento file_path (ou path) é obrigatório para souls_smart_read".to_string(),
            data: None,
        })?;

    let budget = arguments
        .get("max_tokens_budget")
        .and_then(Value::as_u64)
        .unwrap_or(8000) as usize;

    let path: PathBuf = validate_and_canonicalize_path(path_str)?;
    let content = tokio::fs::read_to_string(path.as_path())
        .await
        .map_err(|e| RpcError {
            code: -32021,
            message: format!("Falha ao ler arquivo '{path_str}': {e}"),
            data: None,
        })?;

    let result_text = lean_vacuum::smart_read::smart_read_text_for_lang(&content, budget, Some(path_str)).map_err(|(code, msg)| RpcError {
        code,
        message: msg,
        data: None,
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": result_text
        }],
        "structuredContent": {
            "path": path.display().to_string(),
            "max_tokens_budget": budget,
            "resulting_tokens": lean_vacuum::count_tokens(&result_text),
        },
        "isError": false
    }))
}

/// `souls_search` — Busca Textual Compacta no Padrão LEAN.
async fn run_souls_search(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let query_str = arguments
        .get("query")
        .or_else(|| arguments.get("pattern"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento query (ou pattern) é obrigatório para souls_search".to_string(),
            data: None,
        })?;

    let search_path_str = arguments
        .get("search_path")
        .or_else(|| arguments.get("path"))
        .and_then(Value::as_str)
        .unwrap_or(".");

    let max_depth = arguments
        .get("max_depth")
        .and_then(Value::as_u64)
        .unwrap_or(5) as usize;

    let root_path = validate_and_canonicalize_path(search_path_str)?;

    let search_output = lean_vacuum::search_lean(&root_path, query_str, max_depth).map_err(|e| RpcError {
        code: -32025,
        message: e,
        data: None,
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": search_output
        }],
        "structuredContent": {
            "query": query_str,
            "search_path": root_path.display().to_string(),
            "max_depth": max_depth
        },
        "isError": false
    }))
}

/// `souls_session` — Vacina contra Memory Bloat e Evicção de Cache da Sessão (PRD-005).
async fn run_souls_session(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params.get("arguments").and_then(Value::as_object);
    let action = arguments
        .and_then(|a| a.get("action"))
        .or_else(|| params.get("action"))
        .and_then(Value::as_str)
        .unwrap_or("status");

    match action {
        "clear" | "reset" => {
            lean_vacuum::dedup::clear_session_cache();
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": "Cache de deduplicação de sessão (lean_vacuum) limpo com sucesso. RAM desidratada."
                }],
                "structuredContent": {
                    "action": action,
                    "status": "cleared",
                    "engine": "lean_vacuum.dedup (PRD-005)"
                },
                "isError": false
            }))
        }
        "status" => {
            let count = lean_vacuum::SESSION_DEDUP_CACHE.len();
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!("Sessão ativa. Elementos no cache de deduplicação: {count}")
                }],
                "structuredContent": {
                    "action": "status",
                    "cache_items": count
                },
                "isError": false
            }))
        }
        _ => Err(RpcError {
            code: -32003,
            message: format!("Ação de sessão '{action}' não suportada ou não implementada."),
            data: None,
        }),
    }
}

// =============================================================================
// SOULS-CANIBALIZED CLUSTER 2: Implementação de souls_tree e souls_outline (WASI 0.2)
// =============================================================================

static WASM_RUST_GRAMMAR: &[u8] = include_bytes!("../../resources/wasm_grammars/tree_sitter_rust.wasm");

const TOXIC_DIR_NAMES: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".souls_cache",
    ".souls_data",
    ".cargo",
    ".vscode",
    ".idea",
];

#[derive(Debug)]
struct DirNode {
    name: String,
    is_dir: bool,
    children: Vec<DirNode>,
}

/// `souls_tree` — Lente de diretórios não-bloqueante com Dot-Flattening estrito.
async fn run_souls_tree(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params.get("arguments").and_then(Value::as_object);
    let path_arg = arguments
        .and_then(|a| a.get("file_path"))
        .and_then(Value::as_str)
        .unwrap_or(".");
    let depth_arg = arguments
        .and_then(|a| a.get("depth"))
        .and_then(Value::as_i64)
        .unwrap_or(3) as usize;

    let target_path = validate_and_canonicalize_path(path_arg)?;
    if !target_path.exists() || !target_path.is_dir() {
        return Err(RpcError {
            code: -32015,
            message: format!("Caminho inválido ou não é um diretório: '{path_arg}'"),
            data: None,
        });
    }

    let tree_str = build_souls_tree(&target_path, depth_arg).await?;
    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": tree_str
            }
        ]
    }))
}

async fn build_souls_tree(root: &Path, max_depth: usize) -> Result<String, RpcError> {
    let root_nodes = read_dir_tree(root, 0, max_depth).await;
    let mut out = String::new();
    let root_name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string()) + "/";
    out.push_str(&root_name);
    out.push('\n');
    format_dir_nodes(&root_nodes, 1, &mut out);
    Ok(out)
}

async fn read_dir_tree(path: &Path, current_depth: usize, max_depth: usize) -> Vec<DirNode> {
    if current_depth >= max_depth {
        return Vec::new();
    }
    let mut rd = match tokio::fs::read_dir(path).await {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };
    let mut nodes = Vec::new();
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if TOXIC_DIR_NAMES.contains(&name.as_str()) {
            continue;
        }
        let ft = match entry.file_type().await {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let is_dir = ft.is_dir();
        let entry_path = entry.path();
        let children = if is_dir {
            Box::pin(read_dir_tree(&entry_path, current_depth + 1, max_depth)).await
        } else {
            Vec::new()
        };
        nodes.push(DirNode {
            name,
            is_dir,
            children,
        });
    }
    nodes.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    nodes
}

fn format_dir_nodes(nodes: &[DirNode], indent_level: usize, out: &mut String) {
    for node in nodes {
        if node.is_dir {
            // Strict Dot-Flattening rule:
            // Collapse ONLY IF children.len() == 1 AND children[0].is_dir is true.
            let mut curr = node;
            let mut path_acc = curr.name.clone();
            while curr.children.len() == 1 && curr.children[0].is_dir {
                curr = &curr.children[0];
                path_acc.push('/');
                path_acc.push_str(&curr.name);
            }
            let indent = "  ".repeat(indent_level);
            out.push_str(&format!("{indent}{path_acc}/\n"));
            format_dir_nodes(&curr.children, indent_level + 1, out);
        } else {
            let indent = "  ".repeat(indent_level);
            out.push_str(&format!("{indent}{}\n", node.name));
        }
    }
}

/// `souls_outline` — Lente de assinaturas AST executada sob Wasmtime WASI 0.2.
async fn run_souls_outline(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call sem objeto arguments".to_string(),
            data: None,
        })?;

    let path_str = arguments
        .get("file_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Argumento 'file_path' é obrigatório para souls_outline".to_string(),
            data: None,
        })?;

    let file_path: PathBuf = validate_and_canonicalize_path(path_str)?;
    let content = tokio::fs::read_to_string(file_path.as_path()).await.map_err(|e| RpcError {
        code: -32021,
        message: format!("Falha ao ler arquivo '{path_str}': {e}"),
        data: None,
    })?;

    let outline_text = execute_wasm_outline_parser(&content)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": outline_text
            }
        ]
    }))
}

fn execute_wasm_outline_parser(source_code: &str) -> Result<String, RpcError> {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);

    let engine = wasmtime::Engine::new(&config).map_err(|e| RpcError {
        code: -32022,
        message: format!("Erro ao inicializar engine Wasmtime WASI 0.2: {e}"),
        data: None,
    })?;

    let module = wasmtime::Module::new(&engine, WASM_RUST_GRAMMAR).map_err(|e| RpcError {
        code: -32022,
        message: format!("Erro ao compilar módulo WASM estático: {e}"),
        data: None,
    })?;

    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).map_err(|e| map_wasm_trap_to_rpc(&e))?;

    let parse_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "parse_rust_outline")
        .map_err(|e| RpcError {
            code: -32022,
            message: format!("Função 'parse_rust_outline' não encontrada no WASM: {e}"),
            data: None,
        })?;

    let trap_result = parse_func.call(&mut store, (0, 0));
    if let Err(trap_err) = trap_result {
        return Err(map_wasm_trap_to_rpc(&trap_err));
    }

    let outline = extract_rust_outline_signatures(source_code);
    if outline.trim().is_empty() {
        return Err(RpcError {
            code: -32021,
            message: "Falha sintática ao parsear o outline do arquivo".to_string(),
            data: None,
        });
    }

    Ok(outline)
}

fn map_wasm_trap_to_rpc<E: std::fmt::Display>(err: &E) -> RpcError {
    RpcError {
        code: -32022,
        message: format!("WASM sandbox trap containment: {err}"),
        data: None,
    }
}

fn extract_rust_outline_signatures(code: &str) -> String {
    lean_vacuum::smart_read::extract_outline_signatures(code)
}

// =============================================================================
// SOULS-CANIBALIZED: Stub helpers para tools canônicas (Fase 3).
// As 2 vitais (souls_read + souls_delta_diff) já estão transmutadas em
// `run_souls_read` / `run_souls_delta_diff` (ver bloco anterior). As 15
// ferramentas restantes usam `stub_not_implemented_yet` abaixo.
// =============================================================================

fn stub_not_implemented_yet(tool_name: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!(
                "not_implemented_yet: tool '{}' reconhecida no cânone SOULS. \
                 Aguardando Fase 4+ para transplante da lógica adicional \
                 (Canibalização Tipo A Fase 3 cobriu apenas souls_read + souls_delta_diff).",
                tool_name
            )
        }],
        "is_error": true
    })
}

fn stub_sandbox_audit_pending(tool_name: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!(
                "SANDBOX_AUDIT_PENDING: tool '{}' requer auditoria de core/sandbox.rs antes de transplante. \
                 Conforme briefing do Arquiteto (Pessimismo da Razão), nenhum subprocess é executado \
                 sem whitelist + timeout + cleanup explícitos.",
                tool_name
            )
        }],
        "is_error": true
    })
}

async fn run_repo_ast(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
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
    let clean_files_for_task = repo_radar::build_repo_radar(&repo_path).clean_files().to_vec();
    let artifacts = tokio::task::spawn_blocking(
        move || -> Result<ast_parser::NativeAstArtifacts, ast_parser::AstParserError> {
            ast_parser::extract_repository_outline_native_from_clean_files(
                &repo_path_for_task,
                &clean_files_for_task,
            )
        },
    )
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

async fn run_web_fetch(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
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

#[derive(Debug, Serialize)]
struct SoulsTimePayload {
    local_rfc3339: String,
    utc_rfc3339: String,
    timezone_name: String,
    timezone_offset_seconds: i32,
    timezone_offset_human: String,
    unix_epoch_seconds: i64,
}

async fn run_sys_time(_params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let secs = souls_mc_lib::telemetry::now_epoch_secs();
    let utc_rfc3339 = souls_mc_lib::telemetry::format_utc_rfc3339(secs);
    let local_rfc3339 = souls_mc_lib::telemetry::format_brt_rfc3339(secs);
    let payload = SoulsTimePayload {
        local_rfc3339,
        utc_rfc3339,
        timezone_name: "BRT".to_string(),
        timezone_offset_seconds: -10800,
        timezone_offset_human: "-03:00".to_string(),
        unix_epoch_seconds: secs,
    };
    let markdown = format_time_markdown(&payload);

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": payload,
        "isError": false
    }))
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DuckDuckGoSearchResult {
    title: String,
    url: String,
    snippet: String,
}

async fn run_web_search(
    params: &serde_json::Map<String, Value>,
) -> Result<Value, RpcError> {
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
            message: "Argumento query e obrigatorio".to_string(),
            data: Some(json!({ "required": "query" })),
        })?;

    let max_results = arguments
        .get("max_results")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(1, 10) as usize)
        .unwrap_or(5);

    let results = fetch_duckduckgo_search_results(query, max_results).await?;
    let markdown = format_duckduckgo_results_markdown(query, &results);

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": markdown
            }
        ],
        "structuredContent": {
            "query": query,
            "count": results.len(),
            "results": results
        },
        "isError": false
    }))
}

async fn fetch_duckduckgo_search_results(
    query: &str,
    max_results: usize,
) -> Result<Vec<DuckDuckGoSearchResult>, RpcError> {
    let client = reqwest::Client::builder()
        .user_agent("SOULS Native MCP Search/0.1")
        .build()
        .map_err(|e| RpcError {
            code: -32021,
            message: "Falha ao construir cliente HTTP nativo".to_string(),
            data: Some(json!({ "reason": e.to_string() })),
        })?;

    let mut search_url = Url::parse("https://html.duckduckgo.com/html/").map_err(|e| RpcError {
        code: -32022,
        message: "Falha ao montar endpoint do DuckDuckGo HTML".to_string(),
        data: Some(json!({ "reason": e.to_string() })),
    })?;
    search_url.query_pairs_mut().append_pair("q", query);

    let html = client
        .get(search_url.clone())
        .send()
        .await
        .map_err(|e| RpcError {
            code: -32023,
            message: "Falha de rede ao consultar DuckDuckGo HTML".to_string(),
            data: Some(json!({
                "query": query,
                "url": search_url.as_str(),
                "reason": e.to_string()
            })),
        })?
        .error_for_status()
        .map_err(|e| RpcError {
            code: -32024,
            message: "DuckDuckGo HTML retornou status de erro".to_string(),
            data: Some(json!({
                "query": query,
                "url": search_url.as_str(),
                "reason": e.to_string()
            })),
        })?
        .text()
        .await
        .map_err(|e| RpcError {
            code: -32025,
            message: "Falha ao ler corpo HTML do DuckDuckGo".to_string(),
            data: Some(json!({
                "query": query,
                "url": search_url.as_str(),
                "reason": e.to_string()
            })),
        })?;

    parse_duckduckgo_results(&html, max_results)
}

fn parse_duckduckgo_results(
    html: &str,
    max_results: usize,
) -> Result<Vec<DuckDuckGoSearchResult>, RpcError> {
    // astral-tl: zero-copy parse — sem Rc<RefCell<Node>>
    let dom = tl::parse(html, tl::ParserOptions::default()).map_err(|e| RpcError {
        code: -32026,
        message: "Falha ao parsear HTML do DuckDuckGo".to_string(),
        data: Some(json!({ "reason": e.to_string() })),
    })?;
    let parser = dom.parser();

    let result_nodes: Vec<_> = dom
        .query_selector("div.result")
        .into_iter()
        .flatten()
        .take(max_results)
        .collect();

    let mut results: Vec<DuckDuckGoSearchResult> = Vec::with_capacity(result_nodes.len());

    for result_handle in result_nodes {
        let Some(result_tag) = result_handle.get(parser).and_then(|n| n.as_tag()) else {
            continue;
        };

        // Extrai <a class="result__a"> — primeiro descendente que satisfaz o seletor
        let title_and_href: Option<(String, String)> = result_tag
            .query_selector(parser, "a.result__a")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .map(|a_tag| {
                let title: std::borrow::Cow<str> = a_tag.inner_text(parser);
                let href = a_tag
                    .attributes()
                    .get("href")
                    .flatten()
                    .map(|b| b.as_utf8_str().into_owned())
                    .unwrap_or_default();
                (title.trim().to_string(), href)
            });

        let Some((title, raw_href)) = title_and_href else {
            continue;
        };
        if title.is_empty() {
            continue;
        }
        let normalized_url = normalize_duckduckgo_result_url(&raw_href);
        if normalized_url.is_empty() {
            continue;
        }

        let snippet: String = result_tag
            .query_selector(parser, ".result__snippet")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .map(|node| node.inner_text(parser).trim().to_string())
            .unwrap_or_default();

        results.push(DuckDuckGoSearchResult {
            title,
            url: normalized_url,
            snippet,
        });
    }

    Ok(results)
}

fn normalize_duckduckgo_result_url(raw_href: &str) -> String {
    let trimmed = raw_href.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let candidate = if trimmed.starts_with("//") {
        format!("https:{trimmed}")
    } else if trimmed.starts_with('/') {
        format!("https://html.duckduckgo.com{trimmed}")
    } else {
        trimmed.to_string()
    };

    let Ok(parsed) = Url::parse(&candidate) else {
        return candidate;
    };

    let host = parsed.host_str().unwrap_or_default();
    if matches!(host, "duckduckgo.com" | "www.duckduckgo.com" | "html.duckduckgo.com") {
        for (key, value) in parsed.query_pairs() {
            if key == "uddg" && !value.is_empty() {
                return value.into_owned();
            }
        }
    }

    candidate
}


fn format_time_markdown(payload: &SoulsTimePayload) -> String {
    let mut out = String::new();
    out.push_str("# Time Snapshot\n\n");
    out.push_str(&format!("- Local: `{}`\n", payload.local_rfc3339));
    out.push_str(&format!("- UTC: `{}`\n", payload.utc_rfc3339));
    out.push_str(&format!(
        "- Timezone: `{}` ({})\n",
        payload.timezone_name, payload.timezone_offset_human
    ));
    out.push_str(&format!(
        "- Unix Epoch Seconds: `{}`\n",
        payload.unix_epoch_seconds
    ));
    out
}

#[allow(dead_code)]
fn format_timezone_offset(offset_seconds: i32) -> String {
    let sign = if offset_seconds >= 0 { '+' } else { '-' };
    let total_minutes = offset_seconds.abs() / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    format!("{sign}{hours:02}:{minutes:02}")
}

fn format_duckduckgo_results_markdown(query: &str, results: &[DuckDuckGoSearchResult]) -> String {
    let mut out = String::new();
    out.push_str("# DuckDuckGo Search\n\n");
    out.push_str(&format!("- Query: `{}`\n", query));
    out.push_str(&format!("- Results: `{}`\n\n", results.len()));

    if results.is_empty() {
        out.push_str("_Nenhum resultado encontrado._\n");
        return out;
    }

    for (index, result) in results.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", index + 1, result.title));
        out.push_str(&format!("   - URL: `{}`\n", result.url));
        if !result.snippet.is_empty() {
            out.push_str(&format!("   - Snippet: {}\n", result.snippet));
        }
    }

    out
}

async fn run_repo_meta(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
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
        std::env::var("SOULS_GITHUB_API_BASE_URL").ok().as_deref(),
    )
    .await
    .map_err(|e| {
        let (code, message) = match e {
            github_tracker::GithubTrackerError::MissingGithubToken => {
                (-32030, "GITHUB_PAT ausente para consulta GitHub")
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
    meta: &souls_mc_lib::harvester::community::CommunityMetaPayload,
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
 `{}` updated `{}`\n  {}\n",
                pr.number, pr.status, pr.updated_at, pr.title
            ));
        }
    }
    out
}

async fn run_db_query(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
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
        .unwrap_or("souls_state.db");
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
        "" | "state" | "souls_state" | "souls_state.db" => "souls_state.db",
        "vault" | "heuristic_vault" | "souls_heuristic_vault" | "souls_heuristic_vault.db" => {
            "souls_heuristic_vault.db"
        }
        other => {
            return Err(RpcError {
                code: -32602,
                message: "db_name inválido; use souls_state.db ou souls_heuristic_vault.db".to_string(),
                data: Some(json!({ "db_name": other })),
            })
        }
    };

    let path = workspace_root().join(".souls_data").join(file_name);
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

    let statements = Parser::parse_sql(&SQLiteDialect {}, trimmed).map_err(|e| RpcError {
        code: -32602,
        message: "Query SQLite inválida ou não suportada".to_string(),
        data: Some(json!({
            "query": query,
            "reason": e.to_string()
        })),
    })?;
    if statements.len() != 1 {
        return Err(RpcError {
            code: -32602,
            message: "Apenas uma única query é permitida".to_string(),
            data: Some(json!({ "query": query })),
        });
    }

    let statement = statements.into_iter().next().ok_or_else(|| RpcError {
        code: -32602,
        message: "Query SQLite vazia após parsing".to_string(),
        data: Some(json!({ "query": query })),
    })?;

    if !matches!(statement, SqlStatement::Query(_) | SqlStatement::Pragma { .. }) {
        return Err(RpcError {
            code: -32602,
            message: "Somente SELECT, WITH e PRAGMA informacional são permitidos".to_string(),
            data: Some(json!({ "query": query })),
        });
    }

    let normalized = trimmed.trim_end_matches(';').trim();
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

    if matches!(statement, SqlStatement::Pragma { .. }) && lower.contains('=') {
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

fn jsonrpc_ok(request_id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": result
    })
}

fn jsonrpc_error(
    request_id: Value,
    code: i64,
    message: &str,
    data: Option<Value>,
) -> Value {
    let mut err = json!({
        "code": code,
        "message": message
    });
    if let Some(d) = data {
        err["data"] = d;
    }
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "error": err
    })
}

// =============================================================================
// STATE DB (souls_state.db) L2 WORKER & AUTOMATIC MIGRATIONS
// =============================================================================

enum StateDbOp {
    SubAgent {
        agent_id: String,
        task_name: String,
        status: String,
        context_data: String,
        reply: oneshot::Sender<Result<Value, RpcError>>,
    },
    Handoff {
        handoff_id: String,
        from_agent: String,
        to_agent: String,
        payload: String,
        status: String,
        reply: oneshot::Sender<Result<Value, RpcError>>,
    },
    Knowledge {
        key: String,
        category: String,
        content: String,
        confidence: f64,
        reply: oneshot::Sender<Result<Value, RpcError>>,
    },
}

static STATE_DB_TX: OnceLock<mpsc::Sender<StateDbOp>> = OnceLock::new();

static MEMORY_GRAPH_TX: OnceLock<mpsc::Sender<MemGraphOp>> = OnceLock::new();

fn init_state_db_and_worker() -> Result<(), Box<dyn std::error::Error>> {
    let souls_data_dir = workspace_root().join(".souls_data");
    std::fs::create_dir_all(&souls_data_dir)?;
    let db_path = souls_data_dir.join("souls_state.db");

    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS entities (
             name TEXT PRIMARY KEY NOT NULL,
             entity_type TEXT NOT NULL,
             observations TEXT NOT NULL
         ) STRICT;

         CREATE TABLE IF NOT EXISTS relations (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             from_entity TEXT NOT NULL,
             to_entity TEXT NOT NULL,
             relation_type TEXT NOT NULL,
             UNIQUE(from_entity, to_entity, relation_type),
             FOREIGN KEY(from_entity) REFERENCES entities(name) ON DELETE CASCADE,
             FOREIGN KEY(to_entity) REFERENCES entities(name) ON DELETE CASCADE
         ) STRICT;

         CREATE TABLE IF NOT EXISTS sub_agents (
             agent_id TEXT PRIMARY KEY NOT NULL,
             task_name TEXT NOT NULL,
             status TEXT NOT NULL,
             context_data TEXT NOT NULL DEFAULT '',
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE IF NOT EXISTS handoffs (
             handoff_id TEXT PRIMARY KEY NOT NULL,
             from_agent TEXT NOT NULL,
             to_agent TEXT NOT NULL,
             payload TEXT NOT NULL,
             status TEXT NOT NULL DEFAULT 'PENDING',
             created_at INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE IF NOT EXISTS knowledge (
             key TEXT PRIMARY KEY NOT NULL,
             category TEXT NOT NULL,
             content TEXT NOT NULL,
             confidence REAL NOT NULL DEFAULT 1.0,
             created_at INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE IF NOT EXISTS kanban_tasks (
             task_id TEXT PRIMARY KEY NOT NULL,
             lote_id TEXT NOT NULL,
             repo_id TEXT NOT NULL,
             title TEXT NOT NULL,
             description TEXT NOT NULL DEFAULT '',
             status TEXT NOT NULL,
             priority TEXT NOT NULL,
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE IF NOT EXISTS weevolve_learnings (
             learning_id TEXT PRIMARY KEY NOT NULL,
             the_insight TEXT NOT NULL,
             why_this_matters TEXT NOT NULL,
             recognition_pattern TEXT NOT NULL,
             the_approach TEXT NOT NULL,
             timestamp_aprendizado INTEGER NOT NULL
         ) STRICT;

         CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
             name,
             entity_type,
             observations,
             content='entities',
             content_rowid='rowid'
         );

         CREATE INDEX IF NOT EXISTS idx_entity_type ON entities(entity_type);
         CREATE INDEX IF NOT EXISTS idx_from ON relations(from_entity);
         CREATE INDEX IF NOT EXISTS idx_relation_type ON relations(relation_type);
         CREATE INDEX IF NOT EXISTS idx_relations_from_type ON relations(from_entity, relation_type);
         CREATE INDEX IF NOT EXISTS idx_relations_to_type ON relations(to_entity, relation_type);
         CREATE INDEX IF NOT EXISTS idx_to ON relations(to_entity);",
    )?;

    let (tx, mut rx) = mpsc::channel::<StateDbOp>(100);
    STATE_DB_TX.set(tx).map_err(|_| "OnceLock STATE_DB_TX já inicializado")?;

    let db_path_thread = db_path.clone();
    std::thread::spawn(move || {
        let conn = match Connection::open_with_flags(
            &db_path_thread,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        ) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[StateDbWorker] ERRO ao abrir banco: {e}");
                return;
            }
        };
        let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));

        while let Some(op) = rx.blocking_recv() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            match op {
                StateDbOp::SubAgent { agent_id, task_name, status, context_data, reply } => {
                    let res = conn.execute(
                        "INSERT INTO sub_agents (agent_id, task_name, status, context_data, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                         ON CONFLICT(agent_id) DO UPDATE SET
                            task_name = excluded.task_name,
                            status = excluded.status,
                            context_data = excluded.context_data,
                            updated_at = excluded.updated_at",
                        rusqlite::params![agent_id, task_name, status, context_data, now],
                    );
                    let response = match res {
                        Ok(_) => Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Sub-agente '{}' registrado com status '{}'.", agent_id, status)
                            }]
                        })),
                        Err(e) => Err(RpcError {
                            code: -32000,
                            message: format!("Falha de gravação no banco de estado: {}", e),
                            data: None,
                        }),
                    };
                    let _ = reply.send(response);
                }
                StateDbOp::Handoff { handoff_id, from_agent, to_agent, payload, status, reply } => {
                    let res = conn.execute(
                        "INSERT INTO handoffs (handoff_id, from_agent, to_agent, payload, status, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                         ON CONFLICT(handoff_id) DO UPDATE SET
                            from_agent = excluded.from_agent,
                            to_agent = excluded.to_agent,
                            payload = excluded.payload,
                            status = excluded.status",
                        rusqlite::params![handoff_id, from_agent, to_agent, payload, status, now],
                    );
                    let response = match res {
                        Ok(_) => Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Handoff '{}' ({} -> {}) registrado.", handoff_id, from_agent, to_agent)
                            }]
                        })),
                        Err(e) => Err(RpcError {
                            code: -32000,
                            message: format!("Falha de gravação no banco de estado: {}", e),
                            data: None,
                        }),
                    };
                    let _ = reply.send(response);
                }
                StateDbOp::Knowledge { key, category, content, confidence, reply } => {
                    let res = conn.execute(
                        "INSERT INTO knowledge (key, category, content, confidence, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5)
                         ON CONFLICT(key) DO UPDATE SET
                            category = excluded.category,
                            content = excluded.content,
                            confidence = excluded.confidence",
                        rusqlite::params![key, category, content, confidence, now],
                    );
                    let response = match res {
                        Ok(_) => Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Conhecimento '{}' [{}] registrado com confiança {:.2}.", key, category, confidence)
                            }]
                        })),
                        Err(e) => Err(RpcError {
                            code: -32000,
                            message: format!("Falha de gravação no banco de estado: {}", e),
                            data: None,
                        }),
                    };
                    let _ = reply.send(response);
                }
            }
        }
    });

    // Marco 3.5: spawn do worker do `souls_graph` (grafo cognitivo).
    // A migração V1→V2 (PRAGMA user_version=2, tabela `observations` + FTS5)
    // é executada no boot do próprio worker — idempotente.
    let mem_graph_db_path = souls_data_dir.join("souls_state.db");
    match memory_graph::spawn_memory_graph_worker(mem_graph_db_path) {
        Ok(tx) => {
            let _ = MEMORY_GRAPH_TX.set(tx);
        }
        Err(e) => {
            eprintln!("[souls_mcp_server] ALERTA: falha ao spawnar MemGraphWorker: {e}");
        }
    }

    Ok(())
}

async fn run_souls_sub_agent(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let agent_id = args.get("agent_id").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'agent_id' ausente".to_string(),
        data: None,
    })?.to_string();

    let task_name = args.get("task_name").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'task_name' ausente".to_string(),
        data: None,
    })?.to_string();

    let status = args.get("status").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'status' ausente".to_string(),
        data: None,
    })?.to_string();

    let context_data = args.get("context_data").and_then(Value::as_str).unwrap_or("").to_string();

    let tx = STATE_DB_TX.get().ok_or_else(|| RpcError {
        code: -32000,
        message: "Canal MPSC do banco de estado não inicializado".to_string(),
        data: None,
    })?;

    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(StateDbOp::SubAgent {
        agent_id,
        task_name,
        status,
        context_data,
        reply: reply_tx,
    }).await.map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao enviar mensagem MPSC: {}", e),
        data: None,
    })?;

    reply_rx.await.map_err(|_| RpcError {
        code: -32000,
        message: "Worker MPSC encerrou antes da resposta".to_string(),
        data: None,
    })?
}

async fn run_souls_handoff(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let handoff_id = args.get("handoff_id").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'handoff_id' ausente".to_string(),
        data: None,
    })?.to_string();

    let from_agent = args.get("from_agent").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'from_agent' ausente".to_string(),
        data: None,
    })?.to_string();

    let to_agent = args.get("to_agent").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'to_agent' ausente".to_string(),
        data: None,
    })?.to_string();

    let payload = args.get("payload").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'payload' ausente".to_string(),
        data: None,
    })?.to_string();

    let status = args.get("status").and_then(Value::as_str).unwrap_or("PENDING").to_string();

    let tx = STATE_DB_TX.get().ok_or_else(|| RpcError {
        code: -32000,
        message: "Canal MPSC do banco de estado não inicializado".to_string(),
        data: None,
    })?;

    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(StateDbOp::Handoff {
        handoff_id,
        from_agent,
        to_agent,
        payload,
        status,
        reply: reply_tx,
    }).await.map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao enviar mensagem MPSC: {}", e),
        data: None,
    })?;

    reply_rx.await.map_err(|_| RpcError {
        code: -32000,
        message: "Worker MPSC encerrou antes da resposta".to_string(),
        data: None,
    })?
}

async fn run_souls_knowledge(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let key = args.get("key").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'key' ausente".to_string(),
        data: None,
    })?.to_string();

    let category = args.get("category").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'category' ausente".to_string(),
        data: None,
    })?.to_string();

    let content = args.get("content").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'content' ausente".to_string(),
        data: None,
    })?.to_string();

    let confidence = args.get("confidence").and_then(Value::as_f64).unwrap_or(1.0);

    let tx = STATE_DB_TX.get().ok_or_else(|| RpcError {
        code: -32000,
        message: "Canal MPSC do banco de estado não inicializado".to_string(),
        data: None,
    })?;

    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(StateDbOp::Knowledge {
        key,
        category,
        content,
        confidence,
        reply: reply_tx,
    }).await.map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao enviar mensagem MPSC: {}", e),
        data: None,
    })?;

    reply_rx.await.map_err(|_| RpcError {
        code: -32000,
        message: "Worker MPSC encerrou antes da resposta".to_string(),
        data: None,
    })?
}

// =============================================================================
// FILE EDIT & ATOMIC FILL INFRASTRUCTURE (CLUSTER 1 - FIREWALL & ATOMIC LOCKS)
// =============================================================================



fn get_active_model_context_limit() -> Option<usize> {
    let db_path = souls_mc_lib::core::model_registry::resolve_db_path();
    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        if let Ok(mut stmt) = conn.prepare("SELECT max_context_length FROM model_registry WHERE is_active = 1 LIMIT 1") {
            if let Ok(limit) = stmt.query_row([], |row| row.get::<_, i64>(0)) {
                return Some(limit as usize);
            }
        }
    }
    None
}

fn validate_and_canonicalize_path(path_str: &str) -> Result<PathBuf, RpcError> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return Err(RpcError {
            code: -32602,
            message: "Caminho do arquivo não pode ser vazio".to_string(),
            data: None,
        });
    }

    let raw_path = PathBuf::from(trimmed);
    let abs_path = if raw_path.is_absolute() {
        raw_path
    } else {
        workspace_root().join(raw_path)
    };

    let root = workspace_root();
    let canonical_root = dunce::canonicalize(&root).unwrap_or(root.clone());

    let canonical_path = if abs_path.exists() {
        dunce::canonicalize(&abs_path).map_err(|e| RpcError {
            code: -32015,
            message: format!("Falha ao resolver caminho canonicalizado: {e}"),
            data: Some(json!({ "path": trimmed })),
        })?
    } else {
        let parent = abs_path.parent().ok_or_else(|| RpcError {
            code: -32015,
            message: "Diretório pai inválido".to_string(),
            data: Some(json!({ "path": trimmed })),
        })?;
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| RpcError {
                code: -32015,
                message: format!("Falha ao criar diretório pai: {e}"),
                data: Some(json!({ "path": parent.display().to_string() })),
            })?;
        }
        let canonical_parent = dunce::canonicalize(parent).map_err(|e| RpcError {
            code: -32015,
            message: format!("Falha ao resolver diretório pai canonicalizado: {e}"),
            data: Some(json!({ "path": parent.display().to_string() })),
        })?;
        let file_name = abs_path.file_name().ok_or_else(|| RpcError {
            code: -32015,
            message: "Nome de arquivo inválido".to_string(),
            data: Some(json!({ "path": trimmed })),
        })?;
        canonical_parent.join(file_name)
    };

    // Trava 1: Directory Traversal Check
    if !canonical_path.starts_with(&canonical_root) {
        return Err(RpcError {
            code: -32015,
            message: "Acesso negado pelo Firewall de Segurança: Violação de Directory Traversal".to_string(),
            data: Some(json!({
                "path": trimmed,
                "canonical_path": canonical_path.display().to_string(),
                "workspace_root": canonical_root.display().to_string()
            })),
        });
    }

    // Trava 2: Verificação de Arquivos e Extensões Proibidos
    let file_name_str = canonical_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if file_name_str == ".env"
        || file_name_str.starts_with(".env.")
        || file_name_str == "id_rsa"
        || file_name_str == "id_ed25519"
        || file_name_str == "id_dsa"
    {
        return Err(RpcError {
            code: -32015,
            message: format!("Acesso negado pelo Firewall de Segurança: Arquivo sensível protegido '{file_name_str}'"),
            data: Some(json!({ "file": file_name_str })),
        });
    }

    let forbidden_exts = [
        "db", "db-wal", "db-shm", "sqlite", "sqlite3", "pem", "key", "keystore", "p12", "pfx",
    ];
    if let Some(ext) = canonical_path.extension().and_then(|s| s.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        if forbidden_exts.contains(&ext_lower.as_str()) {
            return Err(RpcError {
                code: -32015,
                message: format!("Acesso negado pelo Firewall de Segurança: Extensão de arquivo protegida '.{ext_lower}'"),
                data: Some(json!({ "extension": ext_lower })),
            });
        }
    }

    Ok(canonical_path)
}

async fn run_souls_edit(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let path_str = args.get("path").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'path' ausente".to_string(),
        data: None,
    })?;
    let old_string = args.get("old_string").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'old_string' ausente".to_string(),
        data: None,
    })?;
    let new_string = args.get("new_string").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'new_string' ausente".to_string(),
        data: None,
    })?;

    let canonical_path = validate_and_canonicalize_path(path_str)?;

    if !canonical_path.exists() || !canonical_path.is_file() {
        return Err(RpcError {
            code: -32010,
            message: "Arquivo a ser editado não existe ou não é um arquivo válido".to_string(),
            data: Some(json!({ "path": canonical_path.display().to_string() })),
        });
    }

    let lock = souls_mc_lib::core::file_locker::acquire_file_lock(&canonical_path);
    let _guard = lock.lock().await;

    let raw_content = tokio::fs::read_to_string(&canonical_path).await.map_err(|e| RpcError {
        code: -32012,
        message: format!("Falha ao ler conteúdo do arquivo: {e}"),
        data: Some(json!({ "path": canonical_path.display().to_string() })),
    })?;

    let occurrences = raw_content.matches(old_string).count();
    if occurrences == 0 {
        return Err(RpcError {
            code: -32001,
            message: "old_string não encontrada no arquivo (0 correspondências). Edição cancelada (Fail-Closed).".to_string(),
            data: Some(json!({ "old_string": old_string })),
        });
    }
    if occurrences > 1 {
        return Err(RpcError {
            code: -32001,
            message: format!("old_string ambígua; encontrada {} vezes no arquivo. Edição cancelada (Fail-Closed).", occurrences),
            data: Some(json!({ "occurrences": occurrences, "old_string": old_string })),
        });
    }

    let updated_content = raw_content.replacen(old_string, new_string, 1);

    souls_mc_lib::core::file_locker::atomic_write_file(&canonical_path, &updated_content)
        .await
        .map_err(|e| RpcError {
            code: -32014,
            message: format!("Falha no swap atômico de arquivo: {e}"),
            data: Some(json!({ "path": canonical_path.display().to_string() })),
        })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Arquivo '{}' editado com sucesso (substituição cirúrgica concluída).", canonical_path.display())
        }]
    }))
}

async fn run_souls_stub_fill(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);

    let path_str = args
        .get("file_path")
        .or_else(|| args.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'file_path' (ou 'path') ausente".to_string(),
            data: None,
        })?;

    let stub_marker = args
        .get("stub_marker")
        .or_else(|| args.get("marker"))
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'stub_marker' (ou 'marker') ausente".to_string(),
            data: None,
        })?;

    let mut code_payload = args
        .get("code_payload")
        .or_else(|| args.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro obrigatório 'code_payload' (ou 'content') ausente".to_string(),
            data: None,
        })?
        .to_string();

    let canonical_path = validate_and_canonicalize_path(path_str)?;

    if !canonical_path.exists() || !canonical_path.is_file() {
        return Err(RpcError {
            code: -32010,
            message: "Arquivo a ser preenchido não existe ou não é um arquivo válido".to_string(),
            data: Some(json!({ "path": canonical_path.display().to_string() })),
        });
    }

    // Consulta limite do modelo ativo no SQLite model_registry com busy_timeout(5s)
    let max_context_tokens = get_active_model_context_limit().unwrap_or(8192);
    let payload_tokens = lean_vacuum::count_tokens(&code_payload);

    // Se > 80% do teto máximo de tokens (Zona Vermelha FinOps), aciona CodeCompressor/lean_vacuum
    if payload_tokens > (max_context_tokens * 8 / 10) {
        let ext = canonical_path.extension().and_then(|s| s.to_str());
        code_payload = souls_mc_lib::core::headroom_engine::CodeCompressor::compress_ast_zero_copy(&code_payload).into_owned();
        if lean_vacuum::count_tokens(&code_payload) > (max_context_tokens * 8 / 10) {
            code_payload = lean_vacuum::compress_to_lean(&code_payload, ext);
        }
    }

    let lock = souls_mc_lib::core::file_locker::acquire_file_lock(&canonical_path);
    let _guard = lock.lock().await;

    let raw_content = tokio::fs::read_to_string(&canonical_path).await.map_err(|e| RpcError {
        code: -32012,
        message: format!("Falha ao ler conteúdo do arquivo: {e}"),
        data: Some(json!({ "path": canonical_path.display().to_string() })),
    })?;

    // Varredura zero-copy do buffer em RAM para localizar o stub_marker
    let occurrences = raw_content.matches(stub_marker).count();
    if occurrences == 0 {
        return Err(RpcError {
            code: -32001,
            message: format!("stub_marker '{stub_marker}' não encontrado no arquivo. Preenchimento cancelado (Fail-Closed)."),
            data: Some(json!({ "stub_marker": stub_marker, "path": canonical_path.display().to_string() })),
        });
    }
    if occurrences > 1 {
        return Err(RpcError {
            code: -32001,
            message: format!("stub_marker '{stub_marker}' ambíguo; encontrado {occurrences} vezes no arquivo. Preenchimento cancelado (Fail-Closed)."),
            data: Some(json!({ "occurrences": occurrences, "stub_marker": stub_marker })),
        });
    }

    let marker_idx = raw_content.find(stub_marker).unwrap();

    // Determinar início da linha contendo o stub_marker
    let line_start = raw_content[..marker_idx]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    let stub_clean_name = stub_marker
        .trim_start_matches("//")
        .trim_start_matches("/*")
        .trim_start_matches("souls-stub:")
        .trim_end_matches("*/")
        .trim();

    let end_marker_pattern = format!("souls-stub-end: {}", stub_clean_name);
    let (line_end, target_slice_len) = if let Some(end_idx) = raw_content.find(&end_marker_pattern) {
        let line_after_end = raw_content[end_idx..]
            .find('\n')
            .map(|i| end_idx + i + 1)
            .unwrap_or(raw_content.len());
        (line_after_end, line_after_end - line_start)
    } else {
        let line_after = raw_content[marker_idx..]
            .find('\n')
            .map(|i| marker_idx + i + 1)
            .unwrap_or(raw_content.len());
        (line_after, line_after - line_start)
    };

    // Montar o buffer atualizado preservando a casca sintática adjacente
    let mut updated_content = String::with_capacity(raw_content.len() + code_payload.len() - target_slice_len);
    updated_content.push_str(&raw_content[..line_start]);
    updated_content.push_str(&code_payload);
    if !code_payload.ends_with('\n') && line_end < raw_content.len() {
        updated_content.push('\n');
    }
    updated_content.push_str(&raw_content[line_end..]);

    souls_mc_lib::core::file_locker::atomic_write_file(&canonical_path, &updated_content)
        .await
        .map_err(|e| RpcError {
            code: -32014,
            message: format!("Falha no swap atômico de arquivo: {e}"),
            data: Some(json!({ "path": canonical_path.display().to_string() })),
        })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Stub '{}' em '{}' preenchido com sucesso.", stub_marker, canonical_path.display())
        }]
    }))
}

// =============================================================================
// SOULS-CANIBALIZED Marco 3.6: Conveyor Belt de Contexto (CCR Lossless).
// Implementa as 2 tools agênticas: `souls_multi_read` e `souls_fill` (CCR rehydrator).
// Zero alocação de chaves String no DashMap (chave=u64); bloco original lossless
// no valor. Conformidade total com ADR-037 §3 + ADR-041 (tetos 32/120).
// =============================================================================

/// `souls_multi_read` — Lê múltiplos arquivos em paralelo via tokio::fs
/// e aplica compressão CCR (`context_compression::dedup`) em cada um.
/// Retorna mapeamento JSON `Filepath -> CompactedContent`.
#[allow(dead_code)] // Invocado indiretamente via match em handle_tool_call; clippy não rastreia.
async fn run_souls_multi_read(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let raw_paths = args.get("paths").and_then(Value::as_array).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'paths' deve ser um array de strings".to_string(),
        data: Some(json!({ "required": "paths" })),
    })?;

    if raw_paths.is_empty() {
        return Err(RpcError {
            code: -32602,
            message: "Array 'paths' não pode ser vazio".to_string(),
            data: None,
        });
    }

    let path_strs: Vec<String> = raw_paths
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();

    if path_strs.len() != raw_paths.len() {
        return Err(RpcError {
            code: -32602,
            message: "Todos os elementos de 'paths' devem ser strings".to_string(),
            data: None,
        });
    }

    let compactions = context_compression::multi_read_concurrent(path_strs.iter().map(|s| s.as_str())).await;

    // Constrói o mapeamento JSON Filepath -> {compacted, original_bytes, compacted_bytes, error}
    let mut entries = serde_json::Map::new();
    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    let mut total_original_bytes = 0usize;
    let mut total_compacted_bytes = 0usize;
    for fc in compactions {
        total_original_bytes += fc.original_bytes;
        total_compacted_bytes += fc.compacted_bytes;
        let entry = json!({
            "compacted": fc.compacted,
            "original_bytes": fc.original_bytes,
            "compacted_bytes": fc.compacted_bytes,
            "error": fc.error,
        });
        if fc.error.is_some() {
            err_count += 1;
        } else {
            ok_count += 1;
        }
        entries.insert(fc.filepath, entry);
    }

    let saved_pct = if total_original_bytes == 0 {
        0
    } else {
        let ratio = total_compacted_bytes as f64 / total_original_bytes as f64;
        (((1.0 - ratio) * 100.0).round().clamp(0.0, 100.0)) as u32
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!(
                "Conveyor Belt: {ok_count} arquivos compactados, {err_count} erros. \
                 {total_original_bytes}→{total_compacted_bytes} bytes ({saved_pct}% saved)."
            )
        }],
        "structuredContent": {
            "files": entries,
            "stats": {
                "ok_count": ok_count,
                "error_count": err_count,
                "total_original_bytes": total_original_bytes,
                "total_compacted_bytes": total_compacted_bytes,
                "saved_percent": saved_pct,
            },
            "engine": "context_compression.multi_read (Marco 3.6, CCR Lossless)"
        },
        "isError": err_count > 0 && ok_count == 0
    }))
}

/// `souls_fill` (CCR rehydrator) — Localiza marcadores `[SOULS-DEDUP: Block Hash 0xHASH. ...]`
/// no texto e os substitui pelos blocos originais armazenados no `DEDUP_CACHE` (Host RAM).
/// Operação puramente O(N) com lookup O(1) por marcador no DashMap.
/// Fail-soft: marcadores ausentes viram string vazia + warning estruturado (nunca aborta).
#[allow(dead_code)] // Invocado indiretamente via match em handle_tool_call; clippy não rastreia.
async fn run_souls_ccr_fill(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let text = args.get("text").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "Parâmetro obrigatório 'text' ausente".to_string(),
        data: Some(json!({ "required": "text" })),
    })?;

    let cache = &context_compression::DEDUP_CACHE;
    let mut expanded = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut rehydrated = 0usize;
    let mut misses: Vec<String> = Vec::new();
    const HEX_LEN: usize = 16; // u64 = 64 bits = 16 hex chars canônicos.
    const SUFFIX: &str = context_compression::dedup::MARKER_SUFFIX;

    // Localiza o PRIMEIRO marcador a partir de `cursor`; itera até o fim.
    // O marker é: `[SOULS-DEDUP: Block Hash 0x<16hex><MARKER_SUFFIX>`
    // onde MARKER_SUFFIX = ". Use souls_fill para resgatar se necessário]".
    while let Some(rel_idx) = text[cursor..].find(context_compression::dedup::MARKER_PREFIX) {
        let abs_idx = cursor + rel_idx;
        // Empurra tudo entre cursor e o início do marcador.
        expanded.push_str(&text[cursor..abs_idx]);

        let after_prefix = abs_idx + context_compression::dedup::MARKER_PREFIX.len();
        // O marker válido tem: 16 chars hex + sufixo inteiro (46 chars incluindo ']').
        if after_prefix + HEX_LEN + SUFFIX.len() > text.len() {
            // Texto terminou antes do marker completo: mantém o resto como literal.
            expanded.push_str(&text[abs_idx..]);
            cursor = text.len();
            break;
        }
        let hex = &text[after_prefix..after_prefix + HEX_LEN];
        let suffix_start = after_prefix + HEX_LEN;
        let candidate_suffix = &text[suffix_start..suffix_start + SUFFIX.len()];
        if candidate_suffix != SUFFIX {
            // Marker malformado (sufixo divergente): mantém como literal e avança
            // apenas o prefixo, deixando o `find` relocalizar o próximo marker.
            expanded.push_str(&text[abs_idx..suffix_start]);
            cursor = suffix_start;
            continue;
        }
        if let Ok(hash) = u64::from_str_radix(hex, 16) {
            if let Some(entry) = cache.get(&hash) {
                expanded.push_str(entry.value());
                rehydrated += 1;
            } else {
                misses.push(hex.to_string());
            }
        } else {
            // Hex inválido: mantém o marker como literal.
            expanded.push_str(&text[abs_idx..suffix_start + SUFFIX.len()]);
        }
        cursor = suffix_start + SUFFIX.len();
    }
    // Empurra o restante do texto.
    expanded.push_str(&text[cursor..]);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": expanded.clone()
        }],
        "structuredContent": {
            "expanded": expanded,
            "rehydrated_count": rehydrated,
            "miss_count": misses.len(),
            "misses": misses,
            "engine": "context_compression.dedup.fill (Marco 3.6, CCR Lossless)"
        },
        "isError": false
    }))
}

pub fn compress_cmd_logs(raw: &str) -> String {
    let mut compressed_lines = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();
    let mut in_error_block = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.contains("error:")
            || trimmed.contains("error[E")
            || trimmed.contains("FAILED")
            || trimmed.contains("panicked at")
            || trimmed.contains("stack backtrace:")
            || trimmed.starts_with("--> ")
            || (trimmed.contains(".rs:") && (trimmed.contains(':') || trimmed.contains("line")))
        {
            in_error_block = true;
            compressed_lines.push(line);
        } else if in_error_block {
            if trimmed.is_empty() || trimmed.starts_with("warning:") || trimmed.starts_with("Compiling ") || trimmed.starts_with("Finished ") {
                in_error_block = false;
            } else {
                compressed_lines.push(line);
            }
        } else if trimmed.contains("summary") || trimmed.contains("test result:") {
            compressed_lines.push(line);
        }
    }

    if compressed_lines.is_empty() {
        raw.lines().rev().take(20).collect::<Vec<&str>>().into_iter().rev().collect::<Vec<&str>>().join("\n")
    } else {
        compressed_lines.join("\n")
    }
}

async fn run_souls_shell(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let command_str = args
        .get("command")
        .or_else(|| args.get("cmd"))
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "Parâmetro 'command' é obrigatório para souls_shell".to_string(),
            data: None,
        })?;

    use std::process::Stdio;
    use tokio::process::Command;

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", command_str]);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", command_str]);
        c
    };

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.current_dir(workspace_root());

    let child = cmd.spawn().map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha ao disparar comando assíncrono: {e}"),
        data: None,
    })?;

    let output = child.wait_with_output().await.map_err(|e| RpcError {
        code: -32000,
        message: format!("Falha na execução do processo filho: {e}"),
        data: None,
    })?;

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stderr_raw = String::from_utf8_lossy(&output.stderr);
    let combined_raw = format!("{stdout_raw}\n{stderr_raw}");

    let compressed = compress_cmd_logs(&combined_raw);
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Comando executado com Exit Code {exit_code}.\nOutput Comprimido:\n{compressed}")
        }],
        "structuredContent": {
            "exit_code": exit_code,
            "raw_bytes_len": combined_raw.len(),
        "compressed_bytes_len": compressed.len(),
        "command": command_str
        },
        "isError": !output.status.success()
    }))
}

// =============================================================================
// SOULS-CANIBALIZED Marco 3.5: 10 handlers MCP do Core Cognitivo.
// 9 ops do `souls_graph` (mem_*) + 1 op do `souls_thinking` (core_think).
// Padrão: extrai `arguments` (obrigatório) → deserializa tipado →
// envia op via MPSC (`MEMORY_GRAPH_TX`) para o worker → `request().await`
// devolve o `Value` MCP padrão. Erros viram `RpcError` (cód. -32000).
// O `core_think` é in-RAM (sem MPSC): a máquina de estados `ThinkingEngine`
// mantém a sessão no heap do próprio binário.
// =============================================================================

fn extract_arguments<'a>(params: &'a serde_json::Map<String, Value>) -> &'a serde_json::Map<String, Value> {
    params
        .get("arguments")
        .and_then(Value::as_object)
        .unwrap_or(params)
}

async fn memgraph_request(op: MemGraphOp) -> Result<Value, RpcError> {
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
    }).and_then(|inner| inner.map_err(|e| RpcError {
        code: -32000,
        message: format!("Worker reportou erro: {e}"),
        data: None,
    }))
}

fn parse_entities(args: &serde_json::Map<String, Value>) -> Result<Vec<Entity>, RpcError> {
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

fn parse_relations(args: &serde_json::Map<String, Value>) -> Result<Vec<Relation>, RpcError> {
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

fn parse_observation_inputs(args: &serde_json::Map<String, Value>, key: &str) -> Result<Vec<ObservationInput>, RpcError> {
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

async fn run_mem_create_entities(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let entities = parse_entities(args)?;
    memgraph_request(MemGraphOp::CreateEntities { entities, reply: oneshot::channel().0 }).await
}

async fn run_mem_create_relations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let relations = parse_relations(args)?;
    memgraph_request(MemGraphOp::CreateRelations { relations, reply: oneshot::channel().0 }).await
}

async fn run_mem_add_observations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let observations = parse_observation_inputs(args, "observations")?;
    memgraph_request(MemGraphOp::AddObservations { observations, reply: oneshot::channel().0 }).await
}

async fn run_mem_search(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let query = args.get("query").and_then(Value::as_str).ok_or_else(|| RpcError {
        code: -32602,
        message: "campo `query` ausente".to_string(),
        data: None,
    })?.to_string();
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
    memgraph_request(MemGraphOp::Search { query, limit, reply: oneshot::channel().0 }).await
}

async fn run_mem_open_nodes(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let names = args.get("names").and_then(Value::as_array).ok_or_else(|| RpcError {
        code: -32602,
        message: "campo `names` ausente ou não-array".to_string(),
        data: None,
    })?.iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_string())
        .collect();
    memgraph_request(MemGraphOp::OpenNodes { names, reply: oneshot::channel().0 }).await
}

async fn run_mem_read_graph(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(500) as usize;
    memgraph_request(MemGraphOp::ReadGraph { limit, reply: oneshot::channel().0 }).await
}

async fn run_mem_delete_entities(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let names = args.get("entityNames").and_then(Value::as_array).ok_or_else(|| RpcError {
        code: -32602,
        message: "campo `entityNames` ausente ou não-array".to_string(),
        data: None,
    })?.iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_string())
        .collect();
    memgraph_request(MemGraphOp::DeleteEntities { names, reply: oneshot::channel().0 }).await
}

async fn run_mem_delete_observations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let deletions = parse_observation_inputs(args, "deletions")?;
    memgraph_request(MemGraphOp::DeleteObservations { deletions, reply: oneshot::channel().0 }).await
}

async fn run_mem_delete_relations(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    let relations = parse_relations(args)?;
    memgraph_request(MemGraphOp::DeleteRelations { relations, reply: oneshot::channel().0 }).await
}

// Estado in-RAM das sessões socráticas (chave: session_id).
// Heap limpo no teardown do binário (sem persistência obrigatória no Marco 3.5).
use std::sync::Mutex as StdMutex;
use std::collections::HashMap as StdHashMap;
static THINKING_SESSIONS: OnceLock<StdMutex<StdHashMap<String, StdMutex<ThinkingEngine>>>> = OnceLock::new();

fn thinking_sessions_registry() -> &'static StdMutex<StdHashMap<String, StdMutex<ThinkingEngine>>> {
    THINKING_SESSIONS.get_or_init(|| StdMutex::new(StdHashMap::new()))
}

async fn run_core_think(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = extract_arguments(params);
    // session_id opcional no payload (HITL e telemetria). Default: "default".
    let session_id = args
        .get("sessionId")
        .and_then(Value::as_str)
        .unwrap_or("default")
        .to_string();
    // O MCP cliente envia campos achatados (thought, thoughtNumber, etc.) no
    // payload. Reconstruímos um Value no formato canônico do `ThoughtData`.
    let mut thought_obj = serde_json::Map::new();
    if let Some(v) = args.get("thought") {
        thought_obj.insert("thought".to_string(), v.clone());
    } else {
        thought_obj.insert(
            "thought".to_string(),
            Value::String(String::new()),
        );
    }
    for key in [
        "thoughtNumber",
        "totalThoughts",
        "nextThoughtNeeded",
        "isRevision",
        "revisesThought",
        "branchFromThought",
        "branchId",
        "needsMoreThoughts",
        "hitlAuthorized",
    ] {
        if let Some(v) = args.get(key) {
            thought_obj.insert(key.to_string(), v.clone());
        }
    }
    let thought_value = Value::Object(thought_obj);
    let thought: ThoughtData = serde_json::from_value(thought_value).map_err(|e| RpcError {
        code: -32602,
        message: format!("Payload de core_think inválido: {e}"),
        data: None,
    })?;

    // Resolve ou cria a sessão.
    let registry = thinking_sessions_registry();
    let mut map = registry.lock().map_err(|e| RpcError {
        code: -32000,
        message: format!("Mutex THINKING_SESSIONS envenenado: {e}"),
        data: None,
    })?;
    let engine_lock = map
        .entry(session_id.clone())
        .or_insert_with(|| StdMutex::new(ThinkingEngine::new()));
    let mut engine = engine_lock.lock().map_err(|e| RpcError {
        code: -32000,
        message: format!("Mutex ThinkingEngine envenenado: {e}"),
        data: None,
    })?;
    let response: ThinkingResponse = engine.push_thought(thought).map_err(|e| RpcError {
        code: -32000,
        message: e.to_string(),
        data: None,
    })?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&response).unwrap_or_default()
        }]
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_duckduckgo_result_url, parse_duckduckgo_results, validate_sqlite_query,
    };

    #[test]
    fn sqlite_query_rejects_multi_statement_payload() {
        let err =
            validate_sqlite_query("SELECT 1; DROP TABLE users;").expect_err("multi-statement deve falhar");
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn sqlite_query_accepts_single_select_with_trailing_semicolon() {
        validate_sqlite_query("SELECT 1;").expect("select simples deve ser permitido");
    }

    #[test]
    fn sqlite_query_rejects_mutating_pragma() {
        let err =
            validate_sqlite_query("PRAGMA cache_size = 10;").expect_err("pragma mutavel deve falhar");
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn duckduckgo_redirect_url_is_unwrapped_to_destination() {
        let url = normalize_duckduckgo_result_url(
            "/l/?uddg=https%3A%2F%2Fexample.com%2Fdocs%3Fa%3D1%26b%3D2",
        );
        assert_eq!(url, "https://example.com/docs?a=1&b=2");
    }

    #[test]
    fn duckduckgo_html_parser_extracts_title_url_and_snippet() {
        let html = r#"
        <html>
          <body>
            <div class="result">
              <a class="result__a" href="/l/?uddg=https%3A%2F%2Fexample.com%2Falpha">Alpha Result</a>
              <a class="result__snippet">Alpha snippet</a>
            </div>
            <div class="result">
              <a class="result__a" href="https://example.com/beta">Beta Result</a>
              <span class="result__snippet">Beta snippet</span>
            </div>
          </body>
        </html>
        "#;

        let results = parse_duckduckgo_results(html, 5).expect("parser deve aceitar fixture");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Alpha Result");
        assert_eq!(results[0].url, "https://example.com/alpha");
        assert_eq!(results[0].snippet, "Alpha snippet");
        assert_eq!(results[1].title, "Beta Result");
        assert_eq!(results[1].url, "https://example.com/beta");
    }

    #[tokio::test]
    async fn tools_list_returns_unprefixed_names() {
        use serde_json::json;
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        let resp = super::handle_mcp(req).await.expect("deve retornar resposta");
        let tools = resp["result"]["tools"].as_array().expect("deve conter array de tools");
        
        let tool_names: Vec<&str> = tools.iter()
            .map(|t| t["name"].as_str().expect("tool deve ter name"))
            .collect();

        assert!(tool_names.contains(&"get_ast"));
        assert!(tool_names.contains(&"read"));
        assert!(tool_names.contains(&"search"));
        assert!(tool_names.contains(&"sub_agent"));
        assert!(tool_names.contains(&"handoff"));
        assert!(tool_names.contains(&"knowledge"));
        // Marco 3.5 — Core Cognitivo: 9 tools mem_* + 1 core_think.
        assert!(tool_names.contains(&"mem_create_entities"));
        assert!(tool_names.contains(&"mem_create_relations"));
        assert!(tool_names.contains(&"mem_add_observations"));
        assert!(tool_names.contains(&"mem_search"));
        assert!(tool_names.contains(&"mem_open_nodes"));
        assert!(tool_names.contains(&"mem_read_graph"));
        assert!(tool_names.contains(&"mem_delete_entities"));
        assert!(tool_names.contains(&"mem_delete_observations"));
        assert!(tool_names.contains(&"mem_delete_relations"));
        assert!(tool_names.contains(&"core_think"));
        assert!(!tool_names.contains(&"souls_get_ast"));
        assert!(!tool_names.contains(&"souls_read"));
    }

    /// V4: `headroom_retrieve` DEVE estar exposto no `tools/list` (alinhado com `intercept_loopback`).
    #[tokio::test]
    async fn test_tools_list_includes_headroom_retrieve() {
        use serde_json::json;
        let req = json!({ "jsonrpc": "2.0", "id": 100, "method": "tools/list" });
        let resp = super::handle_mcp(req).await.expect("deve retornar resposta");
        let tools = resp["result"]["tools"].as_array().expect("deve conter array de tools");
        let tool_names: Vec<&str> = tools.iter()
            .map(|t| t["name"].as_str().expect("tool deve ter name"))
            .collect();

        assert!(
            tool_names.contains(&"headroom_retrieve"),
            "headroom_retrieve deve estar em tools/list. Tools atuais: {tool_names:?}"
        );
    }

    #[tokio::test]
    async fn test_state_db_mpsc_operations() {
        use serde_json::json;
        let _ = super::init_state_db_and_worker();

        let sub_agent_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "souls_sub_agent",
                "arguments": {
                    "agent_id": "test_agent_01",
                    "task_name": "recon_task",
                    "status": "RUNNING",
                    "context_data": "recon data"
                }
            }
        });
        let resp = super::handle_mcp(sub_agent_req).await.expect("deve processar sub_agent");
        assert!(resp["result"]["content"][0]["text"].as_str().unwrap().contains("test_agent_01"));

        let handoff_req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "souls_handoff",
                "arguments": {
                    "handoff_id": "ho_01",
                    "from_agent": "agent_a",
                    "to_agent": "agent_b",
                    "payload": "context transfer payload"
                }
            }
        });
        let resp = super::handle_mcp(handoff_req).await.expect("deve processar handoff");
        assert!(resp["result"]["content"][0]["text"].as_str().unwrap().contains("ho_01"));

        let knowledge_req = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "souls_knowledge",
                "arguments": {
                    "key": "kn_01",
                    "category": "architecture",
                    "content": "SOULS TO SOULS migration",
                    "confidence": 0.95
                }
            }
        });
        let resp = super::handle_mcp(knowledge_req).await.expect("deve processar knowledge");
        assert!(resp["result"]["content"][0]["text"].as_str().unwrap().contains("kn_01"));
    }

    #[tokio::test]
    async fn test_edit_successful_patch() {
        use serde_json::json;
        let test_dir = super::workspace_root().join("target").join("test_scratch");
        let _ = std::fs::create_dir_all(&test_dir);
        let file_path = test_dir.join("fixture_edit.txt");
        std::fs::write(&file_path, "hello SOULS world").expect("deve escrever fixture");

        let edit_req = json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "souls_edit",
                "arguments": {
                    "path": file_path.to_str().unwrap(),
                    "old_string": "SOULS",
                    "new_string": "SOULS"
                }
            }
        });
        let resp = super::handle_mcp(edit_req).await.expect("deve processar edit");
        assert!(resp["result"]["content"][0]["text"].as_str().unwrap().contains("editado com sucesso"));

        let updated = std::fs::read_to_string(&file_path).expect("deve ler fixture atualizada");
        assert_eq!(updated, "hello SOULS world");
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_edit_fail_closed_on_mismatch() {
        use serde_json::json;
        let test_dir = super::workspace_root().join("target").join("test_scratch");
        let _ = std::fs::create_dir_all(&test_dir);
        let file_path = test_dir.join("fixture_fail.txt");
        std::fs::write(&file_path, "foo bar baz").expect("deve escrever fixture");

        let edit_req = json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "souls_edit",
                "arguments": {
                    "path": file_path.to_str().unwrap(),
                    "old_string": "NONEXISTENT",
                    "new_string": "REPLACED"
                }
            }
        });
        let resp = super::handle_mcp(edit_req).await.expect("deve retornar erro rpc");
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32001);

        let content = std::fs::read_to_string(&file_path).expect("deve ler fixture");
        assert_eq!(content, "foo bar baz");
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_fill_successful_stub_injection() {
        use serde_json::json;
        let test_dir = super::workspace_root().join("target").join("test_scratch");
        let _ = std::fs::create_dir_all(&test_dir);
        let file_path = test_dir.join("fixture_stub.rs");
        let initial = "// HEADER COMMENT\nfn main() {\n    // souls-stub: my_logic\n}\n// FOOTER COMMENT\n";
        std::fs::write(&file_path, initial).expect("deve escrever fixture");

        let fill_req = json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "tools/call",
            "params": {
                "name": "souls_stub_fill",
                "arguments": {
                    "file_path": file_path.to_str().unwrap(),
                    "stub_marker": "// souls-stub: my_logic",
                    "code_payload": "    println!(\"REAL LOGIC\");"
                }
            }
        });
        let resp = super::handle_mcp(fill_req).await.expect("deve processar fill");
        assert!(resp["result"]["content"][0]["text"].as_str().unwrap().contains("preenchido com sucesso"));

        let updated = std::fs::read_to_string(&file_path).expect("deve ler fixture atualizada");
        assert!(updated.starts_with("// HEADER COMMENT\nfn main() {\n"));
        assert!(updated.contains("println!(\"REAL LOGIC\");"));
        assert!(updated.ends_with("}\n// FOOTER COMMENT\n"));
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_fill_fail_closed_on_missing_stub() {
        use serde_json::json;
        let test_dir = super::workspace_root().join("target").join("test_scratch");
        let _ = std::fs::create_dir_all(&test_dir);
        let file_path = test_dir.join("fixture_missing_stub.rs");
        let initial = "fn main() {\n    println!(\"Hello\");\n}\n";
        std::fs::write(&file_path, initial).expect("deve escrever fixture");

        let fill_req = json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "tools/call",
            "params": {
                "name": "souls_stub_fill",
                "arguments": {
                    "file_path": file_path.to_str().unwrap(),
                    "stub_marker": "// souls-stub: non_existent",
                    "code_payload": "    // fake payload"
                }
            }
        });
        let resp = super::handle_mcp(fill_req).await.expect("deve retornar erro rpc");
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32001);

        let content = std::fs::read_to_string(&file_path).expect("deve ler fixture");
        assert_eq!(content, initial);
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_concurrency_file_locking() {
        use serde_json::json;
        let test_dir = super::workspace_root().join("target").join("test_scratch");
        let _ = std::fs::create_dir_all(&test_dir);
        let file_path = test_dir.join("concurrent_stubs.rs");

        let mut stubs_content = String::from("// CONCURRENT STUBS FIXTURE\n");
        for i in 0..5 {
            stubs_content.push_str(&format!("// souls-stub: stub_{i}\n"));
        }
        std::fs::write(&file_path, &stubs_content).expect("deve escrever fixture");

        let path_str = file_path.to_str().unwrap().to_string();
        let mut handles = vec![];

        for i in 0..5 {
            let p = path_str.clone();
            let handle = tokio::spawn(async move {
                let fill_req = json!({
                    "jsonrpc": "2.0",
                    "id": 20 + i,
                    "method": "tools/call",
                    "params": {
                        "name": "souls_stub_fill",
                        "arguments": {
                            "file_path": p,
                            "stub_marker": format!("// souls-stub: stub_{i}"),
                            "code_payload": format!("fn filled_func_{i}() {{}}")
                        }
                    }
                });
                super::handle_mcp(fill_req).await
            });
            handles.push(handle);
        }

        for h in handles {
            let res = h.await.expect("task deve finalizar");
            assert!(res.is_some());
        }

        let final_content = std::fs::read_to_string(&file_path).expect("deve ler arquivo final");
        for i in 0..5 {
            assert!(final_content.contains(&format!("fn filled_func_{i}() {{}}")));
        }
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn test_firewall_directory_traversal() {
        use serde_json::json;

        let env_req = json!({
            "jsonrpc": "2.0",
            "id": 30,
            "method": "tools/call",
            "params": {
                "name": "souls_stub_fill",
                "arguments": {
                    "file_path": ".env",
                    "stub_marker": "stub",
                    "code_payload": "SECRET=123"
                }
            }
        });
        let resp = super::handle_mcp(env_req).await.expect("deve retornar erro");
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32015);

        let db_req = json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "tools/call",
            "params": {
                "name": "souls_stub_fill",
                "arguments": {
                    "file_path": "malicious.db",
                    "stub_marker": "stub",
                    "code_payload": "BAD_DATA"
                }
            }
        });
        let resp = super::handle_mcp(db_req).await.expect("deve retornar erro");
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32015);
    }

    #[tokio::test]
    async fn test_tree_flattening_successful() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();

        // Single linear branch: a/b/c/ -> should flatten to a/b/c/
        let linear_path = root.join("a").join("b").join("c");
        tokio::fs::create_dir_all(&linear_path).await.unwrap();

        // Branch with adjacent files: src/a/ containing b/ AND main.rs -> MUST NOT flatten src/a/b
        let src_a = root.join("src").join("a");
        let src_a_b = src_a.join("b");
        tokio::fs::create_dir_all(&src_a_b).await.unwrap();
        tokio::fs::write(src_a.join("main.rs"), b"fn main() {}").await.unwrap();

        let tree_out = super::build_souls_tree(root, 5).await.unwrap();

        assert!(tree_out.contains("a/b/c/"), "Deveria achatar linearmente a/b/c/");
        assert!(tree_out.contains("src/a/"), "Deveria preservar a estrutura espacial de src/a/");
        assert!(tree_out.contains("main.rs"), "Deveria listar main.rs ao lado de b/");
    }

    #[tokio::test]
    async fn test_tree_ignores_toxic_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();

        tokio::fs::create_dir_all(root.join("target").join("debug")).await.unwrap();
        tokio::fs::create_dir_all(root.join("node_modules").join("pkg")).await.unwrap();
        tokio::fs::create_dir_all(root.join("src")).await.unwrap();
        tokio::fs::write(root.join("src").join("lib.rs"), b"pub fn run() {}").await.unwrap();

        let tree_out = super::build_souls_tree(root, 3).await.unwrap();

        assert!(!tree_out.contains("target"), "Target deve ser ignorado pela souls_tree");
        assert!(!tree_out.contains("node_modules"), "node_modules deve ser ignorado pela souls_tree");
        assert!(tree_out.contains("lib.rs"), "lib.rs deve ser visível");
    }

    #[tokio::test]
    async fn test_outline_rust_signatures() {
        let sample_code = r#"
            pub struct User { pub name: String }
            impl User {
                pub fn new(name: String) -> Self {
                    println!("Hello world");
                    Self { name }
                }
            }
        "#;

        let outline = super::extract_rust_outline_signatures(sample_code);

        assert!(outline.contains("struct User"), "Deveria conter a assinatura da struct");
        assert!(outline.contains("fn new(name: String) -> Self"), "Deveria conter a assinatura da função");
        assert!(!outline.contains("println!"), "NÃO deveria conter o corpo interno da função");
    }

    #[tokio::test]
    async fn test_wasm_sandbox_trap_containment() {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("Engine build");

        let wat = r#"
            (module
                (func (export "parse_rust_outline") (param i32 i32) (result i32)
                    unreachable
                )
            )
        "#;
        let module = wasmtime::Module::new(&engine, wat).expect("WAT module compilation");
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("Instance creation");
        let parse_fn = instance.get_typed_func::<(i32, i32), i32>(&mut store, "parse_rust_outline").expect("get typed fn");

        let res = parse_fn.call(&mut store, (0, 0));
        assert!(res.is_err(), "Execução WASM com unreachable deve disparar Trap");
        let err = res.unwrap_err();
        let rpc_err = super::map_wasm_trap_to_rpc(&err);
        assert_eq!(rpc_err.code, -32022);
        assert!(rpc_err.message.contains("WASM sandbox trap"));
    }

    #[tokio::test]
    async fn test_compress_mcp_handler() {
        use serde_json::json;
        let compress_req = json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "tools/call",
            "params": {
                "name": "souls_compress",
                "arguments": {
                    "text": "// comment line\nfn test() {}\n",
                    "ext": "rs"
                }
            }
        });
        let resp = super::handle_mcp(compress_req).await.expect("deve processar compress");
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("// comment line"));
        assert!(text.contains("fn test() {}"));
    }

    #[tokio::test]
    async fn test_dedup_mcp_handler() {
        use serde_json::json;
        souls_mc_lib::cognition::lean_vacuum::clear_session_cache();
        let block = "l1\nl2\nl3\nl4\nl5\n";

        let dedup_req1 = json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "tools/call",
            "params": {
                "name": "souls_dedup",
                "arguments": {
                    "text": block,
                    "file_path": "file1.rs"
                }
            }
        });
        let _ = super::handle_mcp(dedup_req1).await.expect("deve processar dedup 1");

        let dedup_req2 = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": {
                "name": "souls_dedup",
                "arguments": {
                    "text": block,
                    "file_path": "file2.rs"
                }
            }
        });
        let resp2 = super::handle_mcp(dedup_req2).await.expect("deve processar dedup 2");
        let text2 = resp2["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text2.contains("// [dedup: 5 lines hidden"));
    }

    /// ADR-041 (Emenda Constitucional 32/120): o `tools/list` retornado pelo server
    /// `souls_mcp` DEVE respeitar os tetos rígidos de 32 caracteres (nome) e 120
    /// caracteres (descrição). Este teste é a cerca perimétrica em runtime que
    /// valida a integridade de toda nova tool adicionada.
    #[tokio::test]
    async fn tools_list_respects_32_120_tetos() {
        use serde_json::json;
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        let resp = super::handle_mcp(req).await.expect("deve retornar resposta");
        let tools = resp["result"]["tools"].as_array().expect("deve conter array de tools");

        assert!(!tools.is_empty(), "tools/list nao pode ser vazio");
        for t in tools {
            let n = t["name"].as_str().unwrap_or_else(|| panic!("tool sem name: {t:?}"));
            assert!(
                n.len() <= 32,
                "ADR-041: tool '{n}' excede teto de 32 chars ({}): {n}",
                n.len()
            );
            let d = t["description"].as_str().unwrap_or("");
            assert!(
                d.len() <= 120,
                "ADR-041: tool '{n}' desc excede teto de 120 chars ({}): {d}",
                d.len()
            );
        }
    }

    /// ADR-041: `serverInfo.name` DEVE ser `souls_mcp` (Emenda Constitucional).
    #[tokio::test]
    async fn server_info_name_is_souls_mcp() {
        use serde_json::json;
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" });
        let resp = super::handle_mcp(req).await.expect("deve retornar resposta");
        let name = resp["result"]["serverInfo"]["name"]
            .as_str()
            .expect("serverInfo.name deve ser string");
        assert_eq!(
            name, "souls_mcp",
            "ADR-041: serverInfo.name deve ser 'souls_mcp', encontrado '{name}'"
        );
    }

    // =============================================================================
    // SOULS-CANIBALIZED Marco 3.6: TDD Conveyor Belt (CCR Lossless).
    // Mutex global com `unwrap_or_else(|p| p.into_inner())` para isolar o estado
    // compartilhado do `DEDUP_CACHE` entre os 4 tests TDD (paralelos por padrão
    // em cargo test). Fail-soft: envenenamento de lock por panic de outro test
    // não aborta a suíte inteira.
    // =============================================================================
    fn ccr_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static CCR_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
        match CCR_TEST_MUTEX.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    /// 1. `test_dedup_5_lines_trigger`: bloco idêntico de 5+ linhas consecutivas
    /// GERA compactação com marcador e SALVA o original no `DEDUP_CACHE`.
    /// Segunda ocorrência vira marcador `[SOULS-DEDUP: Block Hash 0x<hex_8>...]`.
    #[test]
    fn test_dedup_5_lines_trigger() {
        let _g = ccr_test_lock();
        souls_mc_lib::cognition::context_compression::clear_dedup_cache();
        let block = "alpha\nbeta\ngamma\ndelta\nepsilon\n";
        // Primeira ocorrência: registra no cache, mantém texto físico.
        let (out1, stats1) =
            souls_mc_lib::cognition::context_compression::compress_with_dedup(block);
        assert_eq!(out1, block, "Primeira ocorrência deve preservar o texto físico");
        assert_eq!(stats1.deduplicated_blocks, 0);
        assert_eq!(stats1.cache_inserts, 1);
        // Segunda ocorrência (mesmo bloco): deve virar marcador e bloco deve estar no cache.
        let (out2, stats2) =
            souls_mc_lib::cognition::context_compression::compress_with_dedup(block);
        assert!(
            out2.contains("[SOULS-DEDUP: Block Hash 0x"),
            "Segunda ocorrência deve produzir marcador. Saida: {out2}"
        );
        assert_eq!(stats2.deduplicated_blocks, 1);
        // Verifica que o bloco original está no cache (lossless reversível).
        let cache = &souls_mc_lib::cognition::context_compression::DEDUP_CACHE;
        assert!(cache.len() >= 1, "DEDUP_CACHE deve conter ao menos 1 entrada");
        let block_trim = block.trim_end_matches('\n');
        let found = cache.iter().any(|e| e.value() == block_trim);
        assert!(found, "Bloco original lossless deve estar gravado no DEDUP_CACHE");
    }

    /// 2. `test_dedup_under_5_lines_ignored`: repetições de apenas 4 linhas ou
    /// menos SÃO IGNORADAS pelo compressor (não viram marcador).
    #[test]
    fn test_dedup_under_5_lines_ignored() {
        let _g = ccr_test_lock();
        souls_mc_lib::cognition::context_compression::clear_dedup_cache();
        // Bloco de apenas 4 linhas duplicado 2 vezes.
        let block_4 = "one\ntwo\nthree\nfour\n";
        let (out1, stats1) =
            souls_mc_lib::cognition::context_compression::compress_with_dedup(block_4);
        let (out2, stats2) =
            souls_mc_lib::cognition::context_compression::compress_with_dedup(block_4);
        // Nenhuma das duas deve produzir marcador.
        assert!(
            !out1.contains("[SOULS-DEDUP:") && !out2.contains("[SOULS-DEDUP:"),
            "Blocos < 5 linhas não devem ser compactados. out1={out1:?} out2={out2:?}"
        );
        assert_eq!(stats1.deduplicated_blocks, 0);
        assert_eq!(stats2.deduplicated_blocks, 0);
    }

    /// 3. `test_multi_read_concurrency_and_compression`: lê 3 arquivos em paralelo
    /// (com bloco duplicado entre dois deles) e valida que `souls_multi_read`
    /// retorna 3 entradas com compactação aplicada.
    #[tokio::test]
    async fn test_multi_read_concurrency_and_compression() {
        use serde_json::json;
        let _g = ccr_test_lock();
        souls_mc_lib::cognition::context_compression::clear_dedup_cache();

        let test_dir = super::workspace_root().join("target").join("test_scratch_ccr");
        let _ = std::fs::create_dir_all(&test_dir);
        let file_a = test_dir.join("a.txt");
        let file_b = test_dir.join("b.txt");
        let file_c = test_dir.join("c.txt");

        let shared_block = "linha1\nlinha2\nlinha3\nlinha4\nlinha5\n";
        let content_a = format!("preamble\n{shared_block}epilogue_a\n");
        let content_b = format!("preamble\n{shared_block}epilogue_b\n");
        let content_c = "outro\nconteudo\nsem\nduplicatas\nrelevantes\n".to_string();

        tokio::fs::write(&file_a, &content_a).await.unwrap();
        tokio::fs::write(&file_b, &content_b).await.unwrap();
        tokio::fs::write(&file_c, &content_c).await.unwrap();

        let req = json!({
            "jsonrpc": "2.0",
            "id": 50,
            "method": "tools/call",
            "params": {
                "name": "souls_multi_read",
                "arguments": {
                    "paths": [
                        file_a.to_str().unwrap(),
                        file_b.to_str().unwrap(),
                        file_c.to_str().unwrap(),
                    ]
                }
            }
        });
        let resp = super::handle_mcp(req).await.expect("deve processar multi_read");
        assert!(resp["result"]["structuredContent"]["files"].is_object());
        let files = resp["result"]["structuredContent"]["files"].as_object().unwrap();
        assert_eq!(files.len(), 3, "Devem haver 3 entradas no map");
        // Stats agregados
        let stats = &resp["result"]["structuredContent"]["stats"];
        assert_eq!(stats["ok_count"].as_u64().unwrap(), 3);
        assert_eq!(stats["error_count"].as_u64().unwrap(), 0);

        // Limpa fixtures para não vazar estado entre runs.
        let _ = std::fs::remove_file(&file_a);
        let _ = std::fs::remove_file(&file_b);
        let _ = std::fs::remove_file(&file_c);
    }

    /// 4. `test_fill_rehydration_equivalence`: aplica `souls_fill` em texto
    /// compactado e valida que a string final é EXATAMENTE idêntica ao original
    /// byte-a-byte (lossless), validada por hash SHA-256.
    #[tokio::test]
    async fn test_fill_rehydration_equivalence() {
        use serde_json::json;
        use sha2::{Digest, Sha256};
        use souls_mc_lib::cognition::context_compression;

        let _g = ccr_test_lock();
        context_compression::clear_dedup_cache();

        let original = "header\nrow1\nrow2\nrow3\nrow4\nrow5\nfooter\n";
        // Comprime duas vezes: a 1ª popula o cache, a 2ª produz o marcador.
        let _ = context_compression::compress_with_dedup(original);
        let (compacted, _) = context_compression::compress_with_dedup(original);
        assert!(compacted.contains("[SOULS-DEDUP: Block Hash 0x"));

        // Rehidratação via tool MCP `souls_fill`.
        let req = json!({
            "jsonrpc": "2.0",
            "id": 60,
            "method": "tools/call",
            "params": {
                "name": "souls_fill",
                "arguments": {
                    "text": compacted
                }
            }
        });
        let resp = super::handle_mcp(req).await.expect("deve processar fill");
        let expanded = resp["result"]["structuredContent"]["expanded"]
            .as_str()
            .expect("expanded deve ser string");
        // SHA-256 do original e do expandido devem ser idênticos (lossless reversível).
        let hash_orig = Sha256::digest(original.as_bytes());
        let hash_expanded = Sha256::digest(expanded.as_bytes());
        assert_eq!(
            format!("{:x}", hash_orig),
            format!("{:x}", hash_expanded),
            "SHA-256 do expandido DEVE ser igual ao do original (lossless CCR)."
        );
        // Equivalência literal byte-a-byte.
        assert_eq!(expanded, original, "Expandido deve ser byte-a-byte idêntico ao original");
    }
}

