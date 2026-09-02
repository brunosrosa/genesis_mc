// SOULS V6 — Core Engine: Maestro de Amarração Tardia / Late-Binding MCP (ADR-041)
//
// Bootstrap inicial expõe apenas as 6 ferramentas basais para manter o prompt da IDE leve.
// Injeção dinâmica sob demanda via `souls_summon_tool` e Garbage Collection (GC)
// atômico de esquemas após 10 minutos de inatividade.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};
use serde_json::{json, Value};

pub const DEFAULT_SCHEMA_IDLE_TTL: Duration = Duration::from_secs(600); // 10 minutos
pub const BASE_TOOL_NAMES: &[&str] = &[
    "export_session",
    "analyze_session",
    "symbol",
    "repo_heatmap",
    "execute",
    "souls_summon_tool",
];

static GLOBAL_LATE_BINDING_ROUTER: OnceLock<LateBindingRouter> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct LateBindingRouter {
    base_tools: HashSet<String>,
    active_tools: Arc<RwLock<HashMap<String, (Value, Instant)>>>,
    master_catalog: Arc<HashMap<String, Value>>,
}

impl LateBindingRouter {
    /// Obtém a referência singleton global do `LateBindingRouter`.
    pub fn global() -> &'static Self {
        GLOBAL_LATE_BINDING_ROUTER.get_or_init(|| {
            Self::new_with_base_tools(Self::build_default_catalog())
        })
    }

    /// Inicializa a instância global com catálogo mestre explícito (se ainda não inicializada).
    pub fn init_global_with_catalog(catalog: Vec<Value>) -> &'static Self {
        GLOBAL_LATE_BINDING_ROUTER.get_or_init(|| {
            Self::new_with_base_tools(catalog)
        })
    }

    /// Constrói catálogo padrão com as 6 ferramentas basais.
    pub fn build_default_catalog() -> Vec<Value> {
        vec![
            json!({
                "name": "export_session",
                "description": "Exporta a sessão socrática ativa para JSON estruturado.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": { "type": "string" }
                    },
                    "required": ["session_id"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "analyze_session",
                "description": "Processa métricas comportamentais da sessão socrática na RAM.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": { "type": "string" }
                    },
                    "required": ["session_id"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "symbol",
                "description": "Localização e extração cirúrgica de símbolos no workspace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "repo_heatmap",
                "description": "Mapeia os arquivos mais aquecidos do repositório via Langevin Decay.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer" }
                    },
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "execute",
                "description": "Execução isolada de comandos em Shadow Workspace sob jaula LPAC.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" }
                    },
                    "required": ["command"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "souls_summon_tool",
                "description": "Vincula dinamicamente esquemas JSON-RPC de ferramentas adicionais no roteador MCP sob demanda.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tool_name": {
                            "type": "string",
                            "description": "Nome canônico da ferramenta MCP a ser summonada (ex: 'handoff', 'souls_handoff', 'semantic_search')."
                        }
                    },
                    "required": ["tool_name"],
                    "additionalProperties": false
                }
            }),
        ]
    }

    /// Cria uma nova instância configurada estritamente com as 6 ferramentas basais.
    pub fn new_with_base_tools(catalog: Vec<Value>) -> Self {
        let mut master_map = HashMap::with_capacity(catalog.len() + 8);
        for item in catalog {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                master_map.insert(name.to_string(), item.clone());
                // Também indexa versão com prefixo souls_ se aplicável
                if !name.starts_with("souls_") {
                    master_map.insert(format!("souls_{}", name), item);
                }
            }
        }

        // Garante que o schema de souls_summon_tool esteja no catálogo mestre
        if !master_map.contains_key("souls_summon_tool") {
            let summon_schema = json!({
                "name": "souls_summon_tool",
                "description": "Vincula dinamicamente esquemas JSON-RPC de ferramentas adicionais no roteador MCP sob demanda.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tool_name": {
                            "type": "string",
                            "description": "Nome canônico da ferramenta MCP a ser summonada (ex: 'handoff', 'souls_handoff', 'semantic_search')."
                        }
                    },
                    "required": ["tool_name"],
                    "additionalProperties": false
                }
            });
            master_map.insert("souls_summon_tool".to_string(), summon_schema.clone());
            master_map.insert("summon_tool".to_string(), summon_schema);
        }

        let mut base_set = HashSet::new();
        let mut active_map = HashMap::new();
        let now = Instant::now();

        for &base_name in BASE_TOOL_NAMES {
            base_set.insert(base_name.to_string());
            if let Some(schema) = master_map.get(base_name) {
                active_map.insert(base_name.to_string(), (schema.clone(), now));
            } else if base_name == "souls_summon_tool" {
                let s = master_map.get("souls_summon_tool").unwrap().clone();
                active_map.insert("souls_summon_tool".to_string(), (s, now));
            }
        }

        Self {
            base_tools: base_set,
            active_tools: Arc::new(RwLock::new(active_map)),
            master_catalog: Arc::new(master_map),
        }
    }

    /// Normaliza nomes de ferramentas para busca no catálogo mestre.
    fn normalize_name(tool_name: &str) -> &str {
        let trimmed = tool_name.trim();
        if let Some(stripped) = trimmed.strip_prefix("souls_") {
            stripped
        } else if let Some(stripped) = trimmed.strip_prefix("ctx_") {
            stripped
        } else {
            trimmed
        }
    }

    /// Sumona e injeta ativamente um esquema JSON-RPC no roteador MCP.
    pub fn summon(&self, tool_name: &str) -> Result<Value, String> {
        let trimmed = tool_name.trim();
        let normalized = Self::normalize_name(trimmed);

        let schema = self.master_catalog
            .get(trimmed)
            .or_else(|| self.master_catalog.get(normalized))
            .or_else(|| self.master_catalog.get(&format!("souls_{}", normalized)))
            .cloned()
            .unwrap_or_else(|| {
                json!({
                    "name": normalized,
                    "description": format!("Garra MCP '{}' vinculada dinamicamente via late-binding.", normalized),
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "additionalProperties": true
                    }
                })
            });


        let canonical_name = schema.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(normalized)
            .to_string();

        let mut active = self.active_tools.write().map_err(|e| e.to_string())?;
        active.insert(canonical_name.clone(), (schema.clone(), Instant::now()));

        Ok(json!({
            "status": "summoned",
            "tool_name": canonical_name,
            "schema": schema,
            "message": format!("Garra '{}' summonada com sucesso e vinculada dinamicamente ao roteador MCP.", canonical_name)
        }))
    }

    /// Atualiza o carimbo de acesso da ferramenta para evitar expurgo precoce pelo GC.
    pub fn touch(&self, tool_name: &str) {
        let trimmed = tool_name.trim();
        let normalized = Self::normalize_name(trimmed);
        if let Ok(mut active) = self.active_tools.write() {
            if let Some(entry) = active.get_mut(trimmed) {
                entry.1 = Instant::now();
            } else if let Some(entry) = active.get_mut(normalized) {
                entry.1 = Instant::now();
            }
        }
    }

    /// Executa o expurgo atômico (GC) de ferramentas dinamicamente injetadas que excederam o TTL de ociosidade.
    /// Ferramentas basais NUNCA são expurgadas.
    pub fn evict_idle(&self, timeout: Duration) -> usize {
        let Ok(mut active) = self.active_tools.write() else {
            return 0;
        };

        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (name, (_, last_touched)) in active.iter() {
            if !self.base_tools.contains(name) && now.duration_since(*last_touched) >= timeout {
                to_remove.push(name.clone());
            }
        }

        let count = to_remove.len();
        for name in to_remove {
            active.remove(&name);
        }
        count
    }

    /// Retorna a lista de ferramentas ativas serializada em JSON conforme especificação MCP `tools/list`.
    pub fn list_active_tools(&self) -> Value {
        let active = match self.active_tools.read() {
            Ok(guard) => guard,
            Err(_) => return json!({ "tools": [] }),
        };

        let mut tools = Vec::with_capacity(active.len());
        for (_, (schema, _)) in active.iter() {
            tools.push(schema.clone());
        }

        // Ordenação determinística por nome para estabilidade de snapshot
        tools.sort_by(|a, b| {
            let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
            name_a.cmp(name_b)
        });

        json!({ "tools": tools })
    }

    /// Verifica se uma ferramenta específica está ativa no momento.
    pub fn is_active(&self, tool_name: &str) -> bool {
        let trimmed = tool_name.trim();
        let normalized = Self::normalize_name(trimmed);
        if let Ok(active) = self.active_tools.read() {
            active.contains_key(trimmed) || active.contains_key(normalized)
        } else {
            false
        }
    }

    /// Retorna a quantidade de ferramentas ativas.
    pub fn active_count(&self) -> usize {
        self.active_tools.read().map(|g| g.len()).unwrap_or(0)
    }

    /// Retorna se uma ferramenta faz parte do grupo basal estrito.
    pub fn is_base_tool(&self, name: &str) -> bool {
        self.base_tools.contains(name)
    }
}
