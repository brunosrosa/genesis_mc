use serde_json::{json, Value};

pub fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "analyze_session",
                "description": "Processa as métricas comportamentais e de revisão de hipóteses socráticas de uma sessão na RAM.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": { "type": "string", "description": "UUID da sessão socrática a analisar." }
                    },
                    "required": ["session_id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "callees",
                "description": "Mapeia quais funções e structs são consumidos internamente pelo símbolo interrogado.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Nome do símbolo do qual se deseja saber os consumidos." }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                }
            },
            {
                "name": "callers",
                "description": "Lista os nós do grafo de dependências que invocam um determinado símbolo no workspace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Nome do símbolo do qual se deseja saber os chamadores." }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                }
            },
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
            {
                "name": "edit",
                "description": "Aplica edições cirúrgicas baseadas em casamento exato de blocos (Search and Replace) com proteção de travamento.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo a ser editado." },
                        "old_string": { "type": "string", "description": "Bloco exato a ser procurado (match exato e contextual)." },
                        "new_string": { "type": "string", "description": "Novo bloco de substituição." },
                        "verify_ast": { "type": "boolean", "description": "Se true, ativa a Válvula de Recusa Sintática (Wasmtime WASI 0.2) com rollback atômico." }
                    },
                    "required": ["path", "old_string", "new_string"],
                    "additionalProperties": false
                }
            },
            {
                "name": "execute",
                "description": "Execução isolada de comandos em Shadow Workspace sob jaula LPAC Win11 com Job Objects.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "Comando ou caminho do executável a rodar no sandbox." },
                        "args": { "type": "array", "items": { "type": "string" }, "description": "Argumentos do comando." },
                        "workspace_path": { "type": "string", "description": "Diretório do shadow workspace isolado." },
                        "timeout_secs": { "type": "integer", "description": "Timeout limite em segundos." }
                    },
                    "required": ["command"],
                    "additionalProperties": false
                }
            },
            {
                "name": "export_session",
                "description": "Exporta a árvore relacional de pensamentos socráticos de uma sessão em formato estruturado (JSON/Markdown).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": { "type": "string", "description": "UUID da sessão socrática a exportar." },
                        "format": { "type": "string", "enum": ["json", "markdown"], "description": "Formato de saída desejado." }
                    },
                    "required": ["session_id", "format"],
                    "additionalProperties": false
                }
            },
            {
                "name": "feedback",
                "description": "Dumps FinOps de telemetria, latencia e eficiencia de token E3 a partir de logs locais de execucao.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            },
            {
                "name": "fetch_web",
                "description": "Busca URL com fallback duplo, retorna markdown limpo. Aliases: fetch_web | souls_fetch_web | web_fetch.",
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
                "name": "fill",
                "description": "Reidrata e expande marcadores de compressão CCR de volta para o texto original lossless na RAM.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Texto compactado contendo marcadores de compressão CCR a serem reidratados." },
                        "hash": { "type": "string", "description": "Hash hex de 64 bits para resgate direto do bloco na RAM." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "get_ast",
                "description": "Extrai o blueprint AST via tree-sitter nativo. Aliases: get_ast | souls_get_ast | repo_ast.",
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
            {
                "name": "heatmap",
                "description": "Mapeia dinamicamente os caminhos quentes de acesso a arquivos locais na RAM Host usando Langevin decay.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Numero maximo de entradas retornadas (padrao 50).", "minimum": 1, "maximum": 500 },
                        "lambda": { "type": "number", "description": "Constante de decaimento Langevin (padrao 0.05)." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "intent",
                "description": "Avalia a ambiguidade, risco relacional e consistência de memória de um prompt antes do disparo de inferência.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string" },
                        "session_id": { "type": "string" },
                        "memory_window": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["prompt"],
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
                "name": "merge_sessions",
                "description": "Executa a fusão atômica de ramificações e fluxos de raciocínio concorrentes sob consistência eventual.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_session_id": { "type": "string", "description": "UUID da sessão fonte (será lida)." },
                        "target_session_id": { "type": "string", "description": "UUID da sessão alvo (receberá as inserções)." }
                    },
                    "required": ["source_session_id", "target_session_id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "metrics",
                "description": "Consolida telemetria FinOps, tráfego de tokens, hit-rate do cache L2 e microdólares via telemetry_logs.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            },
            {
                "name": "multi_read",
                "description": "Lê múltiplos arquivos em lote na RAM de forma assíncrona aplicando compressão de contexto CCR lossless.",
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
                "name": "outline",
                "description": "Extrai assinaturas AST sem corpos de funções via sandbox Wasmtime WASI 0.2. (souls_outline)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Caminho do arquivo de código." }
                    },
                    "required": ["file_path"],
                    "additionalProperties": false
                }
            },
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
                "name": "replace",
                "description": "Substitui blocos textuais extensos sob verificação sintática e com rollback atômico em caso de falha de TDD.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo a ser modificado." },
                        "old_string": { "type": "string", "description": "Bloco exato a ser procurado (match exato e contextual)." },
                        "new_string": { "type": "string", "description": "Novo bloco de substituição." },
                        "verify_ast": { "type": "boolean", "description": "Se true (default para replace), ativa a Válvula de Recusa Sintática." }
                    },
                    "required": ["path", "old_string", "new_string"],
                    "additionalProperties": false
                }
            },
            {
                "name": "repo_heatmap",
                "description": "Calcula o ranking de calor (Frecency) dos arquivos do monorepo baseando-se em modificacoes e acessos.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_path": { "type": "string", "description": "Raiz do monorepo (padrao: workspace atual)." },
                        "limit": { "type": "integer", "description": "Numero maximo de entradas retornadas (padrao 50).", "minimum": 1, "maximum": 500, "default": 50 },
                        "lambda": { "type": "number", "description": "Constante de decaimento exponencial (padrao 0.0001, meia-vida ~1h55min).", "default": 0.0001 }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "repo_impact",
                "description": "Analisa o raio de impacto (Blast Radius) de alteracoes de arquivos via travessia reversa de dependencias.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Caminho do arquivo-alvo (relativo ao repo ou absoluto)." },
                        "max_depth": { "type": "integer", "description": "Profundidade maxima do BFS reverso (1..=10, padrao 3).", "minimum": 1, "maximum": 10, "default": 3 }
                    },
                    "required": ["file_path"],
                    "additionalProperties": false
                }
            },
            {
                "name": "repo_meta",
                "description": "Extrai metadados GitHub via octocrab para owner/repo. Aliases: repo_meta | souls_repo_meta.",
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
                "name": "routes",
                "description": "Mapeia os contratos de endpoints ativos e a reatividade de comunicacao entre Tauri Rust e Svelte 5.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_path": { "type": "string", "description": "Raiz do monorepo (padrao: workspace atual)." }
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
            {
                "name": "semantic_search",
                "description": "Busca híbrida local (FTS5 + LanceDB) usando fusão RRF de baixa latência e sanitização ontológica LadybugDB na RAM.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Consulta textual para busca híbrida." },
                        "limit": { "type": "integer", "description": "Número máximo de resultados (padrão: 5)." },
                        "stability_filter": { "type": "string", "description": "Filtro opcional de estabilidade ('STABLE' ou 'EVOLVING')." },
                        "valid_from": { "type": "integer", "description": "Timestamp Unix Epoch mínimo para valid_from." },
                        "valid_to": { "type": "integer", "description": "Timestamp Unix Epoch máximo para valid_to." },
                        "db_path": { "type": "string", "description": "Caminho opcional do banco SQLite (souls_state.db)." },
                        "vector_db_path": { "type": "string", "description": "Caminho opcional da base LanceDB." }
                    },
                    "required": ["query"],
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
            {
                "name": "shell",
                "description": "Executa comandos de sistema assincronamente via Tokio com compressão e poda de logs de terminal.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            },
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
                "name": "sqlite_query",
                "description": "Consulta SQLite read-only nos bancos nativos. Aliases: sqlite_query | souls_sqlite_query | db_query.",
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
                "name": "stub_fill",
                "description": "Preenche stubs de código demarcados em arquivos locais substituindo-os pelo código fornecido.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo contendo o stub." },
                        "stub_marker": { "type": "string", "description": "Marcador textual do stub a ser preenchido." },
                        "code_payload": { "type": "string", "description": "Código substituto a ser inserido no arquivo." }
                    },
                    "required": ["path", "stub_marker", "code_payload"],
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
                "name": "souls_summon_tool",
                "description": "Vincula dinamicamente esquemas JSON-RPC de garras adicionais no roteador MCP sob demanda.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tool_name": { "type": "string", "description": "Nome da garra MCP a ser summonada dinamicamente." }
                    },
                    "required": ["tool_name"],
                    "additionalProperties": false
                }
            },
            {
                "name": "symbol",
                "description": "Resolve localizacao fisica (file:line:col) via WalkDir+Regex+AST Wasmtime. (souls_symbol)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Nome do simbolo a ser resolvido (identificador valido)." },
                        "path": { "type": "string", "description": "Workspace root (opcional, default = '.')." }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                }
            },
            {
                "name": "sys_time",
                "description": "Retorna data/hora local, UTC e fuso atual via chrono nativo. Aliases: sys_time | souls_sys_time.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            },
            {
                "name": "thinking",
                "description": "Invoca o espaço de raciocínio socrático em múltiplos passos com auto-correção e ramificações de hipóteses.",
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
            },
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
                "name": "web_search",
                "description": "Busca web DuckDuckGo HTML, retorna titulos, links e snippets. Aliases: web_search | souls_web_search.",
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
            }
        ]
    })
}
