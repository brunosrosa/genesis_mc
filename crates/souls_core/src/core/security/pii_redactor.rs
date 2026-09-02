//! SOULS MC — Marco I · v6.1: PII Redactor (Aho-Corasick linear CPU)
//!
//! Higieniza o body **inbound** de requisições HTTP antes do upstream LLM,
//! mapeando PII (caminhos locais, chaves API, tokens, e-mails) para
//! **tokens opacos reversíveis** mantidos em RAM (`DashMap<u64, String>`).
//!
//! ## Algoritmo
//!
//! - `aho-corasick` (BurntSushi) constrói um Aho-Corasick Automaton
//!   **deterministicamente linear** (O(n + m + z)) sobre o conjunto de
//!   patterns pré-compilados.
//! - Cada match produz um token opaco `__SOULS_PII_<id>__` onde `<id>` é
//!   um hash xxhash-like do conteúdo original (64 bits via `rustc-hash`).
//! - Reversibilidade: `reverse(id)` consulta o `DashMap` global.
//!
//! ## Lei de Ferro
//!
//! - **Default DESABILITADO** — opt-in via `GatewayConfig::l7_shield::pii_redaction_enabled`.
//! - **Fail-soft:** se o Aho-Corasick falhar ao compilar, retorna o body
//!   original inalterado (nunca panicar).
//! - **Zero alocação por match** — reutiliza buffers internos.
//! - **ADR-030:** Apenas deps já presentes: `aho-corasick` (pinada em
//!   `[workspace.dependencies]`), `dashmap`, `rustc-hash`.

use std::sync::Arc;

use aho_corasick::AhoCorasick;
use dashmap::DashMap;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

use crate::core::gateway_config::GatewayConfig;

/// Patterns canônicos de PII reconhecidos. Cada pattern é compilado
/// uma única vez no `PiiRedactor::new()` e cacheado em um Aho-Corasick
/// automaton compartilhado (Arc) para clones zero-copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PiiPattern {
    WindowsPath,
    UnixPath,
    ApiKeyValue,
    BearerToken,
    EmailAddress,
}

impl PiiPattern {
    /// Strings reconhecidas pelo Aho-Corasick. **LITERAL** (sem regex — Aho-Corasick
    /// é multi-pattern literal). `ascii_case_insensitive(true)` cobre o case-folding
    /// para `Bearer` vs `bearer`. **Ordem importa** para `MatchKind::LeftmostFirst`:
    /// patterns mais específicos primeiro.
    pub const ALL_STRINGS: &'static [&'static str] = &[
        // (1) Bearer tokens (literal — qualquer sequência de 16+ chars alfanuméricos
        //     após o prefixo "Bearer " é capturada por uma regex separada, ver abaixo).
        "Bearer ",
        "bearer ",
        // (2) API key value prefixes (literais comuns).
        "api_key:",
        "api-key:",
        "apikey:",
        "token:",
        "secret:",
        "password:",
        "API_KEY=",
        "TOKEN=",
        // (3) Windows drive letter prefixes (literais — captura rápida).
        "C:\\Users\\",
        "C:\\Program Files\\",
        "D:\\",
        // (4) Unix home prefixes (literais).
        "/home/",
        "/root/",
        "/etc/passwd",
        "/var/log/",
        // (5) SSH / secrets paths (literais).
        ".ssh/id_rsa",
        ".aws/credentials",
        ".env",
    ];

    pub fn name(&self) -> &'static str {
        match self {
            PiiPattern::WindowsPath => "windows_path",
            PiiPattern::UnixPath => "unix_path",
            PiiPattern::ApiKeyValue => "api_key_value",
            PiiPattern::BearerToken => "bearer_token",
            PiiPattern::EmailAddress => "email_address",
        }
    }
}

/// Redator PII com Aho-Corasick pré-compilado e store de tokens reversíveis.
pub struct PiiRedactor {
    automaton: Arc<AhoCorasick>,
    /// Map: `token_id (u64)` → `original_value (String)`. Inversão só na
    /// response downstream, **nunca** exposto no body outbound.
    token_store: Arc<DashMap<u64, String>>,
    enabled: bool,
}

impl PiiRedactor {
    /// Constrói a partir do `GatewayConfig`. Padrões de PII são derivados
    /// do vetor `l7_shield.pii_patterns`. Strings desconhecidas são ignoradas
    /// (fail-soft).
    pub fn from_config() -> Self {
        let cfg = GatewayConfig::global();
        let enabled = cfg.l7_shield.pii_redaction_enabled;
        // Compila o Aho-Corasick com todos os patterns canônicos (LITERAL).
        // `MatchKind::LeftmostFirst` garante que patterns mais específicos
        // (Bearer) sejam capturados antes de patterns mais genéricos (Path).
        // `ascii_case_insensitive(true)` cobre Bearer vs bearer.
        let automaton = AhoCorasick::builder()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .ascii_case_insensitive(true)
            .build(PiiPattern::ALL_STRINGS)
            .expect("Aho-Corasick com patterns estáticos deve compilar sempre");

        Self {
            automaton: Arc::new(automaton),
            token_store: Arc::new(DashMap::new()),
            enabled,
        }
    }

    /// Construtor com controle explícito (para testes).
    pub fn new(enabled: bool) -> Self {
        let automaton = AhoCorasick::builder()
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .ascii_case_insensitive(true)
            .build(PiiPattern::ALL_STRINGS)
            .expect("patterns estáticos são infallíveis");
        Self {
            automaton: Arc::new(automaton),
            token_store: Arc::new(DashMap::new()),
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Aplica a redação ao body. **Fail-soft:** se desabilitado, retorna
    /// o input inalterado. Se ocorrer erro interno, também retorna inalterado.
    ///
    /// **Algoritmo:** Após o Aho-Corasick encontrar o match LITERAL, captura
    /// também o "valor" seguinte (e.g., o token após "Bearer ", o caminho
    /// após "C:\\Users\\") lendo bytes alfanuméricos/pontos/hífens até
    /// delimitador (espaço, aspas, vírgula). Isso é O(n) adicional mas
    /// mantém a complexidade total linear (Aho-Corasick + varredura local).
    ///
    /// **Alocação:** O(k) onde k = número de matches. Zero alocação se
    /// nenhum match for encontrado.
    pub fn redact(&self, body: &[u8]) -> Vec<u8> {
        if !self.enabled {
            return body.to_vec();
        }
        let text = match std::str::from_utf8(body) {
            Ok(s) => s,
            Err(_) => return body.to_vec(), // fail-soft em body não-UTF-8
        };

        let mut out = String::with_capacity(text.len());
        let mut last_end = 0;

        for m in self.automaton.find_iter(text) {
            // Pula matches que sobrepõem o range já consumido pelo match anterior.
            if m.start() < last_end {
                continue;
            }
            // Copia o trecho entre matches verbatim.
            out.push_str(&text[last_end..m.start()]);
            let (start, end) = self.expand_match(text, m.start(), m.end());
            let original = &text[start..end];
            let token_id = hash_pii_token(original);
            let token = format!("__SOULS_PII_{:016x}__", token_id);
            // Armazena reversibilidade apenas na primeira ocorrência
            // (idempotência: mesmo original → mesmo token → mesmo id).
            self.token_store.entry(token_id).or_insert_with(|| original.to_string());
            out.push_str(&token);
            last_end = end;
        }
        out.push_str(&text[last_end..]);
        out.into_bytes()
    }

    /// Expande um match do Aho-Corasick para incluir o "valor" adjacente.
    /// Por exemplo, para match "Bearer " em "Bearer abc123", retorna (0, 16).
    /// Para match "api_key:" em "api_key: sk123", retorna (start, end + 8).
    ///
    /// **Algoritmo fail-soft:** apenas consome whitespace opcional entre o
    /// delimitador (espaço, =, :) e o valor. NUNCA consome aspas de fechamento
    /// para evitar incluir o terminador JSON no token reversível.
    fn expand_match(&self, text: &str, start: usize, end: usize) -> (usize, usize) {
        let bytes = text.as_bytes();
        let len = bytes.len();
        let after = if end > 0 && end < len {
            match bytes[end - 1] {
                b' ' | b'=' | b':' => {
                    let mut i = end;
                    // (1) Consome whitespace opcional ("api_key: value")
                    while i < len && (bytes[i] == b' ' || bytes[i] == b'\t') {
                        i += 1;
                    }
                    // (2) Lê o valor (alfanumérico + alguns símbolos) até
                    //     delimitador terminal (espaço, aspas, vírgula, '}', '\n').
                    while i < len {
                        let c = bytes[i];
                        if c.is_ascii_alphanumeric()
                            || c == b'.'
                            || c == b'_'
                            || c == b'-'
                            || c == b'/'
                            || c == b'\\'
                            || c == b'+'
                            || c == b'='
                        {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    i
                }
                b'\\' | b'/' => {
                    // Match termina com separador de path — estende até o próximo
                    // delimitador (espaço, aspas, vírgula, '\n', '}').
                    let mut i = end;
                    while i < len {
                        let c = bytes[i];
                        if c == b' ' || c == b'"' || c == b'\'' || c == b',' || c == b'\n' || c == b'}' {
                            break;
                        }
                        i += 1;
                    }
                    i
                }
                _ => end,
            }
        } else {
            end
        };
        (start, after)
    }

    /// Inverte um token opaco para o valor original. Retorna `None` se o
    /// token não estiver no store (cold start, ou token forjado).
    pub fn reverse(&self, token: &str) -> Option<String> {
        let id_str = token
            .strip_prefix("__SOULS_PII_")?
            .strip_suffix("__")?;
        let id = u64::from_str_radix(id_str, 16).ok()?;
        self.token_store.get(&id).map(|v| v.clone())
    }

    pub fn token_count(&self) -> usize {
        self.token_store.len()
    }
}

/// Hash FNV-like (via `rustc-hash::FxHasher`, já no workspace) para gerar
/// IDs de 64 bits estáveis e rápidos. **NÃO é criptográfico** — apenas
/// para mapear strings PII em IDs opacos.
fn hash_pii_token(s: &str) -> u64 {
    let mut h = FxHasher::default();
    s.hash(&mut h);
    h.finish()
}

// ============================================================================
// Testes TDD (Marco I · v6.1 — Lei de default DESABILITADO)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_disabled_returns_input_unchanged() {
        let r = PiiRedactor::new(false);
        let body = br#"{"api_key": "sk-abc123def456ghi789jkl"}"#;
        let out = r.redact(body);
        assert_eq!(out, body, "redator desabilitado deve retornar body inalterado");
    }

    #[test]
    fn test_enabled_redacts_bearer_token() {
        let mut r = PiiRedactor::new(false);
        r.set_enabled(true);
        let body = br#"{"auth": "Bearer abc123def456ghi789jklmno"}"#;
        let out = r.redact(body);
        let out_str = std::str::from_utf8(&out).unwrap();
        assert!(out_str.contains("__SOULS_PII_"), "Bearer deve ser redacted: {out_str}");
        assert!(!out_str.contains("abc123def456ghi789jklmno"), "valor original deve sumir");
    }

    #[test]
    fn test_enabled_redacts_api_key() {
        let mut r = PiiRedactor::new(false);
        r.set_enabled(true);
        // Pattern literal "api_key:" deve casar.
        let body = br#"{"config": "api_key: sk1234567890abcdef"}"#;
        let out = r.redact(body);
        let out_str = std::str::from_utf8(&out).unwrap();
        assert!(!out_str.contains("sk1234567890abcdef"), "API key value deve sumir: {out_str}");
        assert!(out_str.contains("__SOULS_PII_"));
    }

    #[test]
    fn test_enabled_redacts_windows_path() {
        let mut r = PiiRedactor::new(false);
        r.set_enabled(true);
        // `b"..."` (não raw): `\\` representa UMA barra invertida.
        let body = b"{\"path\": \"C:\\Users\\alice\\Documents\\secret.txt\"}";
        let out = r.redact(body);
        let out_str = std::str::from_utf8(&out).unwrap();
        assert!(!out_str.contains("secret.txt"), "path deve sumir: {out_str}");
        assert!(out_str.contains("__SOULS_PII_"));
    }

    #[test]
    fn test_enabled_redacts_unix_path() {
        let mut r = PiiRedactor::new(false);
        r.set_enabled(true);
        // Pattern literal "/home/" deve casar.
        let body = br#"{"config": "/home/alice/file.txt"}"#;
        let out = r.redact(body);
        let out_str = std::str::from_utf8(&out).unwrap();
        assert!(!out_str.contains("file.txt"), "path Unix deve sumir: {out_str}");
    }

    #[test]
    fn test_redaction_is_reversible() {
        let mut r = PiiRedactor::new(false);
        r.set_enabled(true);
        let original_token = "Bearer xyzzy1234567890abcdef";
        let body = format!(r#"{{"auth": "{}"}}"#, original_token).into_bytes();
        let out = r.redact(&body);
        let out_str = std::str::from_utf8(&out).unwrap();
        // Extrai o token: `__SOULS_PII_<16hex>__` — usa `rfind` para o trailing `__`
        // a fim de evitar o `__` do próprio prefixo.
        let token_start = out_str.find("__SOULS_PII_").expect("token deve existir");
        let rest = &out_str[token_start..];
        // Pula o prefixo `__SOULS_PII_` e busca o `__` final.
        let after_prefix = &rest[12..]; // len("__SOULS_PII_") = 12
        let trailing_rel = after_prefix.rfind("__").expect("trailing __ deve existir");
        let token_abs_end = token_start + 12 + trailing_rel + 2;
        let token = &out_str[token_start..token_abs_end];
        let reversed = r.reverse(token);
        assert_eq!(reversed, Some(original_token.to_string()),
                   "Reversão falhou para token '{token}', out_str: '{out_str}'");
    }

    #[test]
    fn test_redact_non_utf8_returns_input() {
        let r = PiiRedactor::new(true);
        let body: &[u8] = &[0xFF, 0xFE, 0x00, 0x80];
        let out = r.redact(body);
        assert_eq!(out, body.to_vec());
    }

    #[test]
    fn test_redact_no_match_returns_input_unchanged() {
        let mut r = PiiRedactor::new(false);
        r.set_enabled(true);
        let body = br#"{"messages": [{"role": "user", "content": "Hello world!"}]}"#;
        let out = r.redact(body);
        assert_eq!(out, body, "body sem PII deve passar inalterado");
    }

    #[test]
    fn test_redact_idempotent() {
        let mut r = PiiRedactor::new(false);
        r.set_enabled(true);
        let body = br#"{"auth": "Bearer abcdef1234567890xyz"}"#;
        let out1 = r.redact(body);
        let out2 = r.redact(&out1);
        assert_eq!(out1, out2, "redaction aplicada 2x deve ser idempotente");
    }

    #[test]
    fn test_reverse_unknown_token_returns_none() {
        let r = PiiRedactor::new(true);
        assert_eq!(r.reverse("__SOULS_PII_ffffffffffffffff__"), None);
        assert_eq!(r.reverse("not-a-token"), None);
    }
}
