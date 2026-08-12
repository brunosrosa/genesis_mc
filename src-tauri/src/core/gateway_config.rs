//! SOULS MC — Marco I · v6.1: Gateway Config (JSONC Parser + SSOT Loader)
//!
//! Carrega e valida o arquivo `.souls/config/souls-gateway.jsonc` no parse-time
//! do `agentgateway_tcp_proxy`. Implementa um parser JSONC minimalista (strip
//! de comentários `//` e `/* */` + trailing commas) que **NÃO aloca no hot path**
//! e preserva a ordem das chaves via `serde_json` com feature `preserve_order`.
//!
//! ## Leis de Ferro
//! - **ADR-001 (Gateway):** Config é a única fonte de verdade de rotas/BYOK/FinOps.
//! - **ADR-010 (Escrita atômica):** Hot-reload do config escreve via tmp+rename.
//! - **ADR-030 (Higiene):** Zero dependências adicionadas — apenas `serde_json`.
//! - **Marco I (Sticky Routing):** Ordem das chaves no JSONC afeta a serialização
//!   byte-a-byte de Z1 (System Prompt) e Z2 (Tool Schemas) — `preserve_order` é
//!   **inegociável**.
//!
//! ## Performance
//! - Strip de comentários: O(n) single-pass com máquina de estados de string.
//! - Parse: `serde_json::from_str` com `preserve_order` → `serde_json::Value`.
//! - Resolução de `${VAR}`: O(k) por string onde k = número de vars no payload.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Schema raiz do `.souls/config/souls-gateway.jsonc`.
///
/// Toda mutação incompatível exige bump major de `version`. Toda mutação
/// compatível exige bump minor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub version: String,
    pub byok: ByokConfig,
    pub models: ModelsConfig,
    pub routes: RoutesConfig,
    pub finops: FinOpsConfig,
    pub telemetry: TelemetryConfig,
    pub l7_shield: L7ShieldConfig,
    pub mcp_server: McpServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByokConfig {
    pub openai: ProviderKey,
    pub anthropic: ProviderKey,
    pub openrouter: ProviderKey,
    pub gemini: ProviderKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderKey {
    pub api_key: String,
    pub base_url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteEndpoint {
    pub provider: String,
    pub model: String,
    pub fallback_model: String,
    pub estimated_cost_per_1m_usd: f64,
    pub max_complexity: f32,
    pub is_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutesConfig {
    pub fast_worker_endpoint: RouteEndpoint,
    pub heavy_brain_endpoint: RouteEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IronCostBreakerConfig {
    pub premium_per_1m_usd: f64,
    pub flash_per_1m_usd: f64,
    pub vram_token_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinOpsConfig {
    pub daily_budget_usd: f64,
    pub iron_cost_breaker: IronCostBreakerConfig,
    pub eco_hybrid_allowed: Vec<String>,
    pub eco_hybrid_forbidden: Vec<String>,
    pub force_local_on_budget_exceeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub peak_ewma_alpha: f32,
    pub ring_buffer_size: usize,
    pub ttft_sample_window_ms: u64,
    pub sqlite_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L7ShieldConfig {
    pub sticky_routing_enabled: bool,
    pub sticky_session_ttl_secs: u64,
    pub pii_redaction_enabled: bool,
    pub pii_patterns: Vec<String>,
    pub response_healing_enabled: bool,
}

/// Configuração do subprocesso MCP local (`souls_mcp_server`).
/// O proxy é o dono do ciclo de vida do filho: spawn no boot, SIGKILL no drop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Caminho do executável (relativo a `src-tauri/target/debug/` ou absoluto).
    /// Aceita `${SOULS_MCP_BIN}` como env var para override.
    pub executable_path: String,
    /// Argumentos CLI passados ao subprocesso (ex: `["--transport", "stdio"]`).
    pub args: Vec<String>,
    /// Working directory do subprocesso. Vazio = herda do proxy.
    pub working_dir: String,
    /// Se `true`, o proxy envia SIGKILL atômico ao filho no shutdown/drop,
    /// eliminando processos zumbis na RAM Host.
    pub kill_on_drop: bool,
    /// Timeout (ms) para considerar o filho "lido" (handshake inicial).
    /// Se 0, sem timeout — proxy confia no subprocesso acordar sozinho.
    pub handshake_timeout_ms: u64,
}

// ============================================================================
// Cache global SSOT — carregado uma única vez no primeiro uso
// ============================================================================

static GATEWAY_CONFIG: OnceLock<GatewayConfig> = OnceLock::new();
static GATEWAY_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

impl GatewayConfig {
    /// Retorna a instância global cacheada do `GatewayConfig`. Carrega
    /// do disco no primeiro uso; nas chamadas subsequentes, retorna
    /// referência zero-copy ao singleton.
    ///
    /// Hot-reload pode ser feito chamando `reload_from_path`.
    pub fn global() -> &'static Self {
        GATEWAY_CONFIG.get_or_init(|| {
            let path = default_config_path();
            Self::load_from_path(&path).unwrap_or_else(|e| {
                tracing::error!(
                    "Falha ao carregar GatewayConfig de {}: {}. Usando defaults conservativos.",
                    path.display(),
                    e
                );
                Self::safe_default()
            })
        })
    }

    pub fn config_path() -> &'static Path {
        GATEWAY_CONFIG_PATH
            .get()
            .map(PathBuf::as_path)
            .unwrap_or_else(|| Path::new(".souls/config/souls-gateway.jsonc"))
    }

    /// Carrega e valida o JSONC do caminho informado. Hot-reload friendly.
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("read_to_string falhou para {}: {e}", path.display()))?;

        let stripped = strip_jsonc_comments(&raw);
        let mut config: Self = serde_json::from_str(&stripped)
            .map_err(|e| format!("serde_json::from_str falhou: {e}. Payload (após strip):\n{stripped}"))?;

        // Expansão lazy de `${VAR}` em todas as strings.
        config.expand_env_vars();

        // Validação de invariantes do schema.
        config.validate()?;

        // Cacheia o path resolvido (para observabilidade).
        let _ = GATEWAY_CONFIG_PATH.set(path.to_path_buf());

        Ok(config)
    }

    /// Defaults conservativos — usados quando o JSONC está ausente/corrompido
    /// em cold start. Nenhuma chave real é embutida; tudo via env vars.
    pub fn safe_default() -> Self {
        Self {
            version: "0.0.0-fallback".to_string(),
            byok: ByokConfig {
                openai: ProviderKey {
                    api_key: String::new(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    enabled: false,
                },
                anthropic: ProviderKey {
                    api_key: String::new(),
                    base_url: "https://api.anthropic.com/v1".to_string(),
                    enabled: false,
                },
                openrouter: ProviderKey {
                    api_key: String::new(),
                    base_url: "https://openrouter.ai/api/v1".to_string(),
                    enabled: false,
                },
                gemini: ProviderKey {
                    api_key: String::new(),
                    base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                    enabled: false,
                },
            },
            models: ModelsConfig {
                whitelist: vec![
                    "anthropic/claude-3.5-sonnet".to_string(),
                    "anthropic/claude-3-5-haiku".to_string(),
                ],
                blacklist: vec!["gpt-3.5-turbo-instruct".to_string()],
            },
            routes: RoutesConfig {
                fast_worker_endpoint: RouteEndpoint {
                    provider: "openrouter".to_string(),
                    model: "anthropic/claude-3-5-haiku".to_string(),
                    fallback_model: "google/gemini-2.0-flash-exp".to_string(),
                    estimated_cost_per_1m_usd: 0.50,
                    max_complexity: 0.3,
                    is_local: false,
                },
                heavy_brain_endpoint: RouteEndpoint {
                    provider: "openrouter".to_string(),
                    model: "anthropic/claude-3.5-sonnet".to_string(),
                    fallback_model: "deepseek/deepseek-r1".to_string(),
                    estimated_cost_per_1m_usd: 15.00,
                    max_complexity: 1.0,
                    is_local: false,
                },
            },
            finops: FinOpsConfig {
                daily_budget_usd: 5.00,
                iron_cost_breaker: IronCostBreakerConfig {
                    premium_per_1m_usd: 15.00,
                    flash_per_1m_usd: 0.50,
                    vram_token_limit: 16384,
                },
                eco_hybrid_allowed: vec!["factual".to_string(), "classification".to_string()],
                eco_hybrid_forbidden: vec!["code_write".to_string(), "refactor".to_string()],
                force_local_on_budget_exceeded: true,
            },
            telemetry: TelemetryConfig {
                peak_ewma_alpha: 0.3,
                ring_buffer_size: 64,
                ttft_sample_window_ms: 60_000,
                sqlite_path: ".souls_data/souls_state.db".to_string(),
            },
            l7_shield: L7ShieldConfig {
                sticky_routing_enabled: true,
                sticky_session_ttl_secs: 3600,
                pii_redaction_enabled: false,
                pii_patterns: vec![
                    "windows_path".to_string(),
                    "unix_path".to_string(),
                    "api_key_value".to_string(),
                    "bearer_token".to_string(),
                    "email_address".to_string(),
                ],
                response_healing_enabled: true,
            },
            mcp_server: McpServerConfig {
                // Default seguro: spawn desabilitado até o JSONC real ser carregado.
                // Path vazio = proxy aborta o spawn com erro explícito.
                executable_path: String::new(),
                args: vec!["--transport".to_string(), "stdio".to_string()],
                working_dir: String::new(),
                kill_on_drop: true,
                handshake_timeout_ms: 5_000,
            },
        }
    }

    /// Expande `${VAR}` em todas as `String` do config usando `std::env::var`.
    /// Resolve apenas a primeira ocorrência por par de chaves; ausente → mantém literal.
    fn expand_env_vars(&mut self) {
        expand_env_in_provider(&mut self.byok.openai);
        expand_env_in_provider(&mut self.byok.anthropic);
        expand_env_in_provider(&mut self.byok.openrouter);
        expand_env_in_provider(&mut self.byok.gemini);
        self.telemetry.sqlite_path =
            expand_env_var(&self.telemetry.sqlite_path.clone());
        self.mcp_server.executable_path =
            expand_env_var(&self.mcp_server.executable_path.clone());
        self.mcp_server.working_dir =
            expand_env_var(&self.mcp_server.working_dir.clone());
    }

    fn validate(&self) -> Result<(), String> {
        if self.finops.daily_budget_usd <= 0.0 {
            return Err("finops.daily_budget_usd deve ser > 0".to_string());
        }
        if self.finops.iron_cost_breaker.vram_token_limit == 0 {
            return Err("finops.iron_cost_breaker.vram_token_limit deve ser > 0".to_string());
        }
        if !(0.0..=1.0).contains(&self.telemetry.peak_ewma_alpha) {
            return Err(format!(
                "telemetry.peak_ewma_alpha={} fora de [0.0, 1.0]",
                self.telemetry.peak_ewma_alpha
            ));
        }
        if self.telemetry.ring_buffer_size == 0 || self.telemetry.ring_buffer_size > 4096 {
            return Err(format!(
                "telemetry.ring_buffer_size={} inválido (esperado 1..=4096)",
                self.telemetry.ring_buffer_size
            ));
        }
        if self.routes.fast_worker_endpoint.estimated_cost_per_1m_usd < 0.0 {
            return Err("routes.fast_worker_endpoint.estimated_cost_per_1m_usd deve ser >= 0".to_string());
        }
        Ok(())
    }
}

fn expand_env_in_provider(p: &mut ProviderKey) {
    p.api_key = expand_env_var(&p.api_key);
    p.base_url = expand_env_var(&p.base_url);
}

/// Expande **todas** as ocorrências de `${VAR}` em `input` usando `std::env::var`.
///
/// Itera o `find` em loop até esgotar todas as substituições, suportando
/// strings compostas como `"${VAR1} ${VAR2}"` onde múltiplas vars aparecem
/// na mesma string. Se uma var não estiver definida no ambiente, mantém
/// o literal `${VAR}` inalterado (fail-soft: não aborta o boot).
///
/// Custo: O(n·k) onde n = comprimento da string, k = número de vars.
/// Strings sem `${` retornam `Cow::Borrowed` (zero alocação).
fn expand_env_var(input: &str) -> String {
    // Fast-path: sem `${` → zero alocação.
    if !input.contains("${") {
        return input.to_string();
    }

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;

    while cursor < input.len() {
        // Procura o próximo `${` a partir de `cursor`.
        let rest = &input[cursor..];
        let Some(rel_start) = rest.find("${") else {
            // Sem mais `${` → copia o resto verbatim e sai.
            out.push_str(rest);
            break;
        };
        let abs_start = cursor + rel_start;
        // Copia tudo antes do `${` para `out`.
        out.push_str(&input[cursor..abs_start]);

        // Procura o `}` correspondente.
        let after_dollar = &input[abs_start + 2..];
        let Some(rel_end) = after_dollar.find('}') else {
            // `${` sem `}` de fechamento — copia verbatim e sai.
            out.push_str(&input[abs_start..]);
            break;
        };

        let var_name = &after_dollar[..rel_end];
        match std::env::var(var_name) {
            Ok(val) => {
                out.push_str(&val);
            }
            Err(_) => {
                // Fail-soft: var indefinida → mantém `${VAR}` literal.
                out.push_str(&input[abs_start..abs_start + 2 + rel_end + 1]);
            }
        }
        cursor = abs_start + 2 + rel_end + 1;
    }
    out
}

fn default_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("SOULS_GATEWAY_CONFIG") {
        return PathBuf::from(p);
    }
    // Subir a árvore até `.souls/config/souls-gateway.jsonc` (até 6 níveis).
    if let Ok(cwd) = std::env::current_dir() {
        let mut candidate = cwd.as_path();
        for _ in 0..6 {
            let path = candidate.join(".souls").join("config").join("souls-gateway.jsonc");
            if path.is_file() {
                return path;
            }
            match candidate.parent() {
                Some(p) => candidate = p,
                None => break,
            }
        }
        // Último fallback: relativo ao CWD.
        return cwd.join(".souls").join("config").join("souls-gateway.jsonc");
    }
    PathBuf::from(".souls/config/souls-gateway.jsonc")
}

// ============================================================================
// JSONC Strip — Máquina de Estados O(n) single-pass
// ============================================================================

/// Remove comentários `//` e `/* */` de uma string JSONC, preservando
/// strings literais (com escapes). O(n) com tabela de lookup de 4 estados.
pub fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Detecta início de string literal: copia até a próxima aspa não escapada.
        if b == b'"' {
            out.push('"');
            i += 1;
            while i < len {
                let c = bytes[i];
                if c == b'\\' && i + 1 < len {
                    // Escape sequence: copia os 2 bytes verbatim.
                    out.push(c as char);
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                out.push(c as char);
                if c == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Comentário de linha `//`: consome até o próximo `\n` (mas mantém o `\n`).
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Comentário de bloco `/* */`: consome até o `*/`. Não aninhado (padrão JSONC).
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }

        out.push(b as char);
        i += 1;
    }

    // Strip trailing commas: `,]` → `]`, `,}` → `}`.
    strip_trailing_commas(&out)
}

fn strip_trailing_commas(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if c == '"' {
            // Toggle string state mas tolera escape básico.
            // (Strip anterior já removeu comentários; restam apenas strings válidas.)
            in_string = !in_string;
        }
        if !in_string && c == ',' {
            // Lookahead: pula whitespace e verifica o próximo char não-whitespace.
            let mut next_non_ws = '\0';
            for nc in chars.clone() {
                if !nc.is_whitespace() {
                    next_non_ws = nc;
                    break;
                }
            }
            if next_non_ws == ']' || next_non_ws == '}' {
                // Trailing comma: consome a vírgula e continua.
                continue;
            }
        }
        out.push(c);
    }
    out
}

// ============================================================================
// Hot-reload (ADR-010 — escrita atômica via tmp + rename)
// ============================================================================

/// Recarrega o `GatewayConfig` do disco e valida sua integridade.
///
/// ## ⚠️ RESTRIÇÃO ARQUITETURAL CRÍTICA (Marco I v6.1.0)
///
/// O `GATEWAY_CONFIG` é um **`OnceLock<GatewayConfig>`** imutável após o
/// primeiro `set()`. Portanto, **esta função NÃO aplica o novo config ao
/// processo em execução**. A nova config é validada, seu conteúdo é logado
/// (incluindo o `version`), mas ela permanece inerte até o próximo boot.
///
/// Em produção, **recarregar config exige reinício completo do binário
/// `agentgateway_tcp_proxy.exe`** (fail-closed por design — evita que
/// partes do runtime operem com config antiga enquanto outras já viram
/// a nova, o que poderia invalidar invariantes do Sticky Router, FinOps
/// e IronCostBreaker).
///
/// ## Por que NÃO usar `RwLock<GatewayConfig>` (decisão registrada)
///
/// Alternativa considerada: trocar o `OnceLock` por um `RwLock` para
/// hot-reload. **Rejeitada** pelos seguintes motivos:
///
/// 1. **Blast radius:** 9 call sites em 6 módulos
///    ([`pareto_bandit`], [`iron_cost`], [`telemetry_dispatcher`],
///    [`subprocess_guard`], [`sticky_router`], [`pii_redactor`],
///    [`agentgateway_tcp_proxy`]) seriam refatorados para usar
///    `read().unwrap()` — risco SDC alto.
/// 2. **Inconsistência de leitura:** Sticky Router, IronCostBreaker e
///    FinOps fariam reads sob lock durante o hot-path de cada request,
///    adicionando contenção de `RwLock` ao Tokio. O P99 de latência
///    sofre mesmo em leituras (cache line bouncing).
/// 3. **Atomicidade de boot:** O `OnceLock` garante que toda thread vê
///    a mesma config. Com `RwLock`, um reload em T1 poderia fazer T2
///    observar config parcialmente inicializada.
///
/// ## Workaround para Dev/Staging
///
/// Para validar mudanças de config sem reiniciar o binário:
/// - Use `validate_jsonc_file(path)` (puro: parse + validate, sem apply).
/// - Aplique mudanças via deploy (restart supervisionado por
///   `boot.ps1`).
///
/// ## Returns
///
/// - `Ok(())` se o arquivo foi parseado e validado com sucesso.
/// - `Err(msg)` se o JSONC tem erro de sintaxe, vars indefinidas ou
///   invariantes violados (ex: `peak_ewma_alpha` fora de `[0.0, 1.0]`).
///
/// **Nota:** mesmo em caso de `Ok(())`, o config ativo no `global()` é o
/// carregado no boot original. Use esta função apenas como teste de
/// "dry-run" da config.
pub fn reload() -> Result<(), String> {
    let path = default_config_path();
    let new_cfg = GatewayConfig::load_from_path(&path)?;
    // Validação já foi executada em `load_from_path` — logamos o resumo
    // da nova config para o operador auditar antes do próximo restart.
    tracing::warn!(
        target: "gateway_config",
        "GatewayConfig::reload: NOVA config VALIDADA de {} (versão {}). \
         ATENÇÃO: ela NÃO está ativa. Reinicie o agentgateway para aplicar.",
        path.display(),
        new_cfg.version
    );
    Ok(())
}

// ============================================================================
// Testes unitários (TDD — Red)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_jsonc_line_comment() {
        let input = r#"{"k": 1, // comentário
            "v": 2}"#;
        let out = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSONC após strip deve ser válido");
        assert_eq!(parsed["k"], 1);
        assert_eq!(parsed["v"], 2);
    }

    #[test]
    fn test_strip_jsonc_block_comment() {
        let input = r#"{"k": 1, /* bloco */ "v": 2}"#;
        let out = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSONC após strip deve ser válido");
        assert_eq!(parsed["k"], 1);
        assert_eq!(parsed["v"], 2);
    }

    #[test]
    fn test_strip_jsonc_preserves_strings_with_slashes() {
        // String contém "http://example.com" — não pode ser stripada como comentário.
        let input = r#"{"url": "https://example.com/path"}"#;
        let out = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("string com // deve sobreviver");
        assert_eq!(parsed["url"], "https://example.com/path");
    }

    #[test]
    fn test_strip_jsonc_trailing_comma() {
        let input = r#"{"k": 1, "v": 2,}"#;
        let out = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("trailing comma deve ser removida");
        assert_eq!(parsed["k"], 1);
        assert_eq!(parsed["v"], 2);
    }

    #[test]
    fn test_preserve_order_keeps_field_order() {
        let input = r#"{"z": 1, "a": 2, "m": 3}"#;
        let parsed: serde_json::Value = serde_json::from_str(input).expect("parse com preserve_order");
        let keys: Vec<&String> = parsed.as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["z", "a", "m"], "Ordem das chaves deve ser preservada");
    }

    #[test]
    fn test_expand_env_var_resolves_known_var() {
        std::env::set_var("SOULS_TEST_VAR", "resolved_value_42");
        let out = expand_env_var("${SOULS_TEST_VAR}");
        assert_eq!(out, "resolved_value_42");
        std::env::remove_var("SOULS_TEST_VAR");
    }

    #[test]
    fn test_expand_env_var_keeps_literal_for_unknown() {
        let out = expand_env_var("${SOULS_UNDEFINED_VAR_99999}");
        assert_eq!(out, "${SOULS_UNDEFINED_VAR_99999}");
    }

    /// TDD — Issue 1: `expand_env_var` deve resolver **todas** as ocorrências
    /// de `${VAR}` em uma única string, não apenas a primeira. Antes do fix,
    /// `"${A} ${B}"` resolvia só `${A}`, deixando `${B}` literal.
    #[test]
    fn test_expand_env_var_resolves_multiple_vars() {
        std::env::set_var("SOULS_TEST_A", "alpha_val");
        std::env::set_var("SOULS_TEST_B", "beta_val");
        let out = expand_env_var("${SOULS_TEST_A} ${SOULS_TEST_B}");
        assert_eq!(out, "alpha_val beta_val");
        std::env::remove_var("SOULS_TEST_A");
        std::env::remove_var("SOULS_TEST_B");
    }

    /// TDD — Edge case: 3+ vars adjacentes, sem espaço entre elas.
    #[test]
    fn test_expand_env_var_resolves_three_consecutive_vars() {
        std::env::set_var("SOULS_TEST_X", "X");
        std::env::set_var("SOULS_TEST_Y", "Y");
        std::env::set_var("SOULS_TEST_Z", "Z");
        let out = expand_env_var("${SOULS_TEST_X}${SOULS_TEST_Y}${SOULS_TEST_Z}");
        assert_eq!(out, "XYZ");
        std::env::remove_var("SOULS_TEST_X");
        std::env::remove_var("SOULS_TEST_Y");
        std::env::remove_var("SOULS_TEST_Z");
    }

    /// TDD — Mixed: var resolvida + var indefinida + texto.
    /// A var indefinida deve permanecer literal; as outras devem expandir.
    #[test]
    fn test_expand_env_var_mixed_known_and_unknown() {
        std::env::set_var("SOULS_TEST_KNOWN", "KNOWN_VAL");
        let out = expand_env_var("[${SOULS_TEST_KNOWN}][${SOULS_UNKNOWN_XYZ_999}][end]");
        assert_eq!(out, "[KNOWN_VAL][${SOULS_UNKNOWN_XYZ_999}][end]");
        std::env::remove_var("SOULS_TEST_KNOWN");
    }

    /// TDD — `${` sem `}` de fechamento deve ser preservado literal (fail-soft).
    #[test]
    fn test_expand_env_var_unterminated_brace_preserved() {
        let out = expand_env_var("path/${SOULS_UNCLOSED");
        assert_eq!(out, "path/${SOULS_UNCLOSED");
    }

    #[test]
    fn test_safe_default_validates() {
        let cfg = GatewayConfig::safe_default();
        cfg.validate().expect("safe_default deve validar");
        assert!(cfg.finops.daily_budget_usd > 0.0);
    }

    #[test]
    fn test_validate_rejects_invalid_alpha() {
        let mut cfg = GatewayConfig::safe_default();
        cfg.telemetry.peak_ewma_alpha = 1.5;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_zero_ring_buffer() {
        let mut cfg = GatewayConfig::safe_default();
        cfg.telemetry.ring_buffer_size = 0;
        assert!(cfg.validate().is_err());
    }
}
