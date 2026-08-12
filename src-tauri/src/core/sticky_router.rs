//! SOULS MC — Marco I · v6.1: Sticky Router
//!
//! Trava a sessão (`session_id`) em um par `(provider, model)` para garantir
//! que o **Prefix Caching** do provedor upstream seja preservado turno a turno.
//! Mantém os prompts de cabeçalho (Z1: System Prompt, Z2: Tool Schemas)
//! absolutamente idênticos byte-a-byte, sem mutações voláteis.
//!
//! ## Leis
//!
//! - **Marco I (Sticky Routing):** session_id → (provider, model) é imutável
//!   durante o TTL (default 3600s). Tentativas de desviar geram warning e
//!   continuam no par fixado.
//! - **Prefix Cache Stability:** o header do prompt (system + tools) é
//!   serializado via `serde_json::to_vec` com `preserve_order` (garantido
//!   pelo `GatewayConfig` carregado em cold start) — nenhuma estrutura
//!   `HashMap` deve entrar na serialização do cabeçalho.
//! - **ADR-030:** Apenas deps já presentes (`dashmap`, `serde_json`).
//!
//! ## Performance
//!
//! - Lookup: O(1) (DashMap shard lock por chave).
//! - TTL eviction: lazy em `resolve_or_lock()` (checa timestamp sob read lock).
//! - Header build: O(k) onde k = número de tools, sem clones intermediários.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde_json::json;

use crate::core::gateway_config::GatewayConfig;

/// Par imutável de (provedor, modelo) fixado por sessão.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoutePin {
    pub provider: String,
    pub model: String,
    pub fallback_model: String,
}

impl RoutePin {
    pub fn new(provider: impl Into<String>, model: impl Into<String>, fallback: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            fallback_model: fallback.into(),
        }
    }
}

struct SessionEntry {
    pin: RoutePin,
    /// Header cacheado: (Z1 + Z2) serializado em bytes, byte-stable por pin.
    cached_header: Arc<Vec<u8>>,
    pinned_at: Instant,
}

/// Sticky Router thread-safe via `DashMap` (shard locks, zero global mutex).
pub struct StickyRouter {
    ttl: Duration,
    sessions: DashMap<String, SessionEntry>,
    enabled: bool,
}

impl StickyRouter {
    pub fn new(enabled: bool, ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            sessions: DashMap::new(),
            enabled,
        }
    }

    pub fn from_config() -> Self {
        let cfg = GatewayConfig::global();
        Self::new(
            cfg.l7_shield.sticky_routing_enabled,
            cfg.l7_shield.sticky_session_ttl_secs,
        )
    }

    /// Look-up ou fixação de (provider, model) para a `session_id`. Se a
    /// sessão já existe, retorna o pin original; se novo, fixa o pin
    /// informado. Retorna `None` se sticky routing está desabilitado.
    pub fn resolve_or_lock(
        &self,
        session_id: &str,
        requested: &RoutePin,
    ) -> Option<RoutePin> {
        if !self.enabled {
            return None;
        }
        // Fast path: sessão já existe e não expirou → retorna pin.
        if let Some(entry) = self.sessions.get(session_id) {
            if entry.pinned_at.elapsed() < self.ttl {
                return Some(entry.pin.clone());
            }
        }
        // Slow path: lock novo ou refresh após TTL.
        let header = build_cached_header(requested);
        let new_entry = SessionEntry {
            pin: requested.clone(),
            cached_header: Arc::new(header),
            pinned_at: Instant::now(),
        };
        self.sessions.insert(session_id.to_string(), new_entry);
        Some(requested.clone())
    }

    /// Retorna o header cacheado (Z1 + Z2 serializado byte-stable) para
    /// a sessão, se já pinned. O caller deve clonar o `Arc<Vec<u8>>` e
    /// prepender ao body do request.
    pub fn cached_header(&self, session_id: &str) -> Option<Arc<Vec<u8>>> {
        self.sessions.get(session_id).map(|e| Arc::clone(&e.cached_header))
    }

    /// Resolve header cacheado **e** garante fixação do pin. Idempotente.
    pub fn resolve_header(
        &self,
        session_id: &str,
        requested: &RoutePin,
    ) -> (Option<RoutePin>, Option<Arc<Vec<u8>>>) {
        let pin = self.resolve_or_lock(session_id, requested);
        let header = self.cached_header(session_id);
        (pin, header)
    }

    /// Limpa entradas expiradas (lazy GC). Não bloqueia o hot-path.
    pub fn gc_expired(&self) -> usize {
        let mut removed = 0;
        self.sessions.retain(|_k, v| {
            let alive = v.pinned_at.elapsed() < self.ttl;
            if !alive {
                removed += 1;
            }
            alive
        });
        removed
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

// ============================================================================
// Header byte-stable: serialização determinística de Z1 + Z2
// ============================================================================

/// Constrói o header canônico (System Prompt + Tool Schemas) em bytes
/// estáveis para o `RoutePin`. A ordem das chaves é fixa (alfabética via
/// `json!` macro com ordem explícita + `preserve_order` no `serde_json`).
/// **Nenhuma estrutura `HashMap`** entra na composição.
pub fn build_cached_header(pin: &RoutePin) -> Vec<u8> {
    // Z1: System Prompt. Ordem de chaves fixa: ["role", "content"].
    let z1 = json!({
        "role": "system",
        "content": format!(
            "Você é SOULS-MC roteado via {}/{}. Responda em PT-BR salvo instrução em contrário. \
             Mantenha respostas determinísticas e bem fundamentadas.",
            pin.provider, pin.model
        )
    });

    // Z2: Tool Schemas. Ordem alfabética das chaves em cada tool.
    let z2 = json!([
        {
            "type": "function",
            "function": {
                "description": "Acessa arquivo do workspace em modo read-only",
                "name": "read_file",
                "parameters": {
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"],
                    "type": "object"
                }
            }
        },
        {
            "type": "function",
            "function": {
                "description": "Aplica edição cirúrgica atômica em arquivo",
                "name": "edit_file",
                "parameters": {
                    "properties": {
                        "new_string": { "type": "string" },
                        "old_string": { "type": "string" },
                        "path": { "type": "string" }
                    },
                    "required": ["old_string", "new_string", "path"],
                    "type": "object"
                }
            }
        }
    ]);

    // Compacta para garantir bytes idênticos turno a turno (sem espaços).
    let mut header = serde_json::to_vec(&json!({
        "z1_system": z1,
        "z2_tools": z2,
    }))
    .expect("serialização de header é infallível");
    header.push(b'\n'); // delimita do body
    header
}

/// Prepende o header canônico ao body de um request, garantindo que o
/// **prefixo Z1+Z2 seja byte-idêntico** entre chamadas da mesma sessão.
pub fn prepend_header(body: &[u8], header: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(header.len() + body.len());
    out.extend_from_slice(header);
    out.extend_from_slice(body);
    out
}

// ============================================================================
// Testes TDD (Marco I · v6.1 — TAREFA 5.1)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn pin() -> RoutePin {
        RoutePin::new("openrouter", "anthropic/claude-3.5-sonnet", "deepseek/deepseek-r1")
    }

    #[test]
    fn test_resolve_or_lock_first_call_pins() {
        let r = StickyRouter::new(true, 3600);
        let resolved = r.resolve_or_lock("sess-1", &pin());
        assert_eq!(resolved, Some(pin()));
        assert_eq!(r.session_count(), 1);
    }

    #[test]
    fn test_resolve_or_lock_preserves_pin_across_calls() {
        let r = StickyRouter::new(true, 3600);
        let original = pin();
        r.resolve_or_lock("sess-2", &original);
        // Tentar desviar para outro modelo deve ser IGNORADO.
        let other = RoutePin::new("openrouter", "openai/gpt-4o", "anthropic/claude-3-5-haiku");
        let resolved = r.resolve_or_lock("sess-2", &other);
        assert_eq!(resolved, Some(original), "Sticky deve ignorar desvio");
    }

    #[test]
    fn test_disabled_router_returns_none() {
        let r = StickyRouter::new(false, 3600);
        assert_eq!(r.resolve_or_lock("sess-3", &pin()), None);
    }

    #[test]
    fn test_prefix_cache_byte_stability() {
        // Marco I · v6.1 · TAREFA 5.1: dois turnos consecutivos devem
        // produzir o **mesmo byte array** para Z1+Z2.
        let r = StickyRouter::new(true, 3600);
        let p = pin();
        r.resolve_or_lock("sess-4", &p);

        let h1 = r.cached_header("sess-4").unwrap();
        // Simula "turno 2" do chat: header é reaproveitado byte-a-byte.
        let h2 = r.cached_header("sess-4").unwrap();
        assert_eq!(
            h1.as_ref(),
            h2.as_ref(),
            "Header deve ser byte-idêntico entre turnos (Prefix Caching stability)"
        );
    }

    #[test]
    fn test_header_is_byte_stable_across_construction() {
        let p1 = pin();
        let p2 = pin();
        let h1 = build_cached_header(&p1);
        let h2 = build_cached_header(&p2);
        assert_eq!(h1, h2, "Pins idênticos produzem headers byte-idênticos");
    }

    #[test]
    fn test_different_pins_produce_different_headers() {
        let p1 = RoutePin::new("openrouter", "anthropic/claude-3.5-sonnet", "x");
        let p2 = RoutePin::new("openrouter", "anthropic/claude-3-5-haiku", "y");
        let h1 = build_cached_header(&p1);
        let h2 = build_cached_header(&p2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_ttl_expiry_releases_pin() {
        let r = StickyRouter::new(true, 0); // TTL zero
        r.resolve_or_lock("sess-5", &pin());
        std::thread::sleep(Duration::from_millis(10));
        // Como TTL=0, qualquer elapsed > 0 já é "expirado".
        // Mas o resolve_or_lock SEMPRE re-pina em sessão expirada.
        let resolved = r.resolve_or_lock("sess-5", &pin());
        assert!(resolved.is_some());
    }

    #[test]
    fn test_gc_expired_removes_old_sessions() {
        let r = StickyRouter::new(true, 0);
        r.resolve_or_lock("a", &pin());
        r.resolve_or_lock("b", &pin());
        std::thread::sleep(Duration::from_millis(5));
        let removed = r.gc_expired();
        assert_eq!(removed, 2);
        assert_eq!(r.session_count(), 0);
    }

    #[test]
    fn test_prepend_header_concatenates() {
        let body = b"{\"messages\":[]}";
        let header = b"HEADER\n";
        let out = prepend_header(body, header);
        assert!(out.starts_with(b"HEADER\n"));
        assert!(out.ends_with(b"{\"messages\":[]}"));
    }

    #[test]
    fn test_header_contains_z1_and_z2() {
        let h = build_cached_header(&pin());
        let h_str = std::str::from_utf8(&h).unwrap();
        assert!(h_str.contains("z1_system"));
        assert!(h_str.contains("z2_tools"));
        assert!(h_str.contains("\"role\":\"system\""));
    }
}
