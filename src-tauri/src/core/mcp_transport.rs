//! SOULS-CANIBALIZED: Trait `McpTransport` + `LeanVacuum`.
//!
//! **Princípio arquitetural:** O envelope da comunicação MCP é JSON-RPC 2.0
//! (para compatibilidade com a IDE Trae). O formato LEAN (Dot-Flattening,
//! booleanos literais) atua como "Saco a Vácuo" — comprime APENAS o payload
//! denso (conteúdo de texto) que viaja dentro de `result.content.text`.
//!
//! A trait `McpTransport` isola a manipulação de `serde_json::Value` na borda
//! (Alfândega), permitindo trocar a implementação concreta (NDJSON puro,
//! rmcp, WebSocket) sem alterar o router.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Erros do transporte MCP.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("JSON inválido: {0}")]
    Json(String),
    #[error("Envelope MCP ausente: campo obrigatório '{0}' não encontrado")]
    MissingField(&'static str),
    #[error("ID inválido: {0}")]
    InvalidId(String),
    #[error("LeanVacuum falhou: {0}")]
    Vacuum(String),
}

/// Request JSON-RPC 2.0 recebido pelo transporte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// Resultado de uma tool (ainda em formato bruto, antes de entrar no envelope).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub kind: String, // "text" | "image" | ...
    pub text: String,
}

impl ToolResult {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                kind: "text".into(),
                text: s.into(),
            }],
            is_error: false,
        }
    }

    pub fn error(s: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                kind: "text".into(),
                text: s.into(),
            }],
            is_error: true,
        }
    }
}

/// Trait central de transporte MCP.
///
/// **Regra de ouro:** Toda manipulação de `serde_json::Value` DEVE ocorrer
/// dentro de um `impl McpTransport`. O router NUNCA toca `serde_json::Value`
/// diretamente — ele opera sobre `JsonRpcRequest` / `ToolResult` / `Value`.
pub trait McpTransport: Send + Sync {
    /// Codifica uma resposta de tool em envelope JSON-RPC 2.0.
    /// Opcionalmente comprime o payload via `LeanVacuum` antes de empacotar.
    fn encode_envelope(
        &self,
        id: Value,
        result: ToolResult,
    ) -> Result<String, McpError>;

    /// Decodifica um payload NDJSON de entrada em `JsonRpcRequest`.
    fn decode_envelope(&self, raw: &str) -> Result<JsonRpcRequest, McpError>;
}

// ============================================================================
// LeanVacuum: Saco a Vácuo do payload LEAN.
// ============================================================================

/// Comprime o conteúdo de texto via formato LEAN antes de empacotar.
/// Na Fase 2, é um stub que comprime espaços redundantes. A compressão
/// real (Dot-Flattening, booleanos literais) virá quando o `lean_ctx::core::compressor`
/// for exposto com default=[].
pub struct LeanVacuum {
    pub enabled: bool,
}

impl Default for LeanVacuum {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl LeanVacuum {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Aplica a compressão LEAN ao texto.
    ///
    /// **Fase 2 (stub):** remove linhas em branco redundantes e colapsa
    /// múltiplos espaços. Sem dependência de `lean_ctx::core::compressor`
    /// para manter esta trait isolada do lean-ctx (inversão de dependência).
    pub fn compress(&self, input: &str) -> String {
        if !self.enabled {
            return input.to_string();
        }
        // Heurística simples: colapsa 3+ espaços em 1, remove linhas 100% vazias duplicadas.
        let mut out = String::with_capacity(input.len());
        let mut prev_blank = false;
        let mut space_run = 0usize;
        for ch in input.chars() {
            if ch == '\n' {
                if prev_blank {
                    continue;
                }
                prev_blank = true;
                out.push('\n');
            } else if ch == ' ' {
                space_run += 1;
                if space_run <= 1 {
                    out.push(' ');
                }
            } else {
                if ch != ' ' {
                    space_run = 0;
                }
                prev_blank = false;
                out.push(ch);
            }
        }
        out
    }
}

// ============================================================================
// NdjsonMcpTransport: implementação concreta padrão (Fase 2).
// ============================================================================

/// Implementação NDJSON pura do transporte MCP.
/// Mantida como a implementação canônica. Trocar de transporte = criar outra `impl McpTransport`.
pub struct NdjsonMcpTransport {
    pub vacuum: LeanVacuum,
}

impl Default for NdjsonMcpTransport {
    fn default() -> Self {
        Self {
            vacuum: LeanVacuum::default(),
        }
    }
}

impl McpTransport for NdjsonMcpTransport {
    fn encode_envelope(
        &self,
        id: Value,
        result: ToolResult,
    ) -> Result<String, McpError> {
        // Aplica o Saco a Vácuo em cada bloco de texto do resultado.
        let compressed: ToolResult = ToolResult {
            is_error: result.is_error,
            content: result
                .content
                .into_iter()
                .map(|c| ToolContent {
                    kind: c.kind,
                    text: self.vacuum.compress(&c.text),
                })
                .collect(),
        };

        let envelope = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": compressed,
        });

        serde_json::to_string(&envelope).map_err(|e| McpError::Json(e.to_string()))
    }

    fn decode_envelope(&self, raw: &str) -> Result<JsonRpcRequest, McpError> {
        // NDJSON: cada linha é um envelope JSON independente.
        // Esta implementação aceita uma única linha por vez (compatibilidade
        // com a leitura síncrona do `souls_mcp_server` original).
        let line = raw.trim();
        if line.is_empty() {
            return Err(McpError::MissingField("jsonrpc"));
        }
        let parsed: Value =
            serde_json::from_str(line).map_err(|e| McpError::Json(e.to_string()))?;

        let jsonrpc = parsed
            .get("jsonrpc")
            .and_then(|v| v.as_str())
            .ok_or(McpError::MissingField("jsonrpc"))?
            .to_string();

        if jsonrpc != "2.0" {
            return Err(McpError::InvalidId(format!(
                "Esperado jsonrpc='2.0', recebido '{jsonrpc}'"
            )));
        }

        let method = parsed
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or(McpError::MissingField("method"))?
            .to_string();

        let id = parsed.get("id").cloned().unwrap_or(Value::Null);
        let params = parsed.get("params").cloned().unwrap_or(Value::Null);

        Ok(JsonRpcRequest {
            jsonrpc,
            id,
            method,
            params,
        })
    }
}

// ============================================================================
// Testes TDD
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_transport_trait_encode_decode_roundtrip() {
        let transport = NdjsonMcpTransport::default();

        // Encode: cria envelope JSON-RPC 2.0 com id=42, result=ToolResult::text.
        let result = ToolResult::text("hello world");
        let encoded = transport
            .encode_envelope(json!(42), result)
            .expect("encode should succeed");
        assert!(encoded.contains("\"jsonrpc\":\"2.0\""));
        assert!(encoded.contains("\"id\":42"));
        assert!(encoded.contains("\"text\":\"hello world\""));

        // Decode: parseia o envelope de requisição de volta.
        let raw_req = r#"{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"test"}}"#;
        let request = transport
            .decode_envelope(raw_req)
            .expect("decode should succeed");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, json!(42));
        assert_eq!(request.method, "tools/call");
    }

    #[test]
    fn test_lean_vacuum_disabled_returns_input_intact() {
        let v = LeanVacuum::new(false);
        let input = "linha 1\n\n\nlinha 2";
        assert_eq!(v.compress(input), input);
    }

    #[test]
    fn test_lean_vacuum_collapses_blank_lines() {
        let v = LeanVacuum::new(true);
        let input = "a\n\n\n\nb";
        let out = v.compress(input);
        // Apenas uma quebra entre 'a' e 'b', não quatro.
        assert_eq!(out, "a\nb");
    }

    #[test]
    fn test_lean_vacuum_collapses_multiple_spaces() {
        let v = LeanVacuum::new(true);
        let out = v.compress("a    b");
        // Múltiplos espaços colapsam em um.
        assert_eq!(out, "a b");
    }

    #[test]
    fn test_decode_rejects_invalid_jsonrpc_version() {
        let transport = NdjsonMcpTransport::default();
        let bad = r#"{"jsonrpc":"1.0","method":"ping","id":1}"#;
        let err = transport.decode_envelope(bad).unwrap_err();
        match err {
            McpError::InvalidId(_) => {}
            other => panic!("Esperado InvalidId, recebido: {other:?}"),
        }
    }

    #[test]
    fn test_decode_rejects_missing_method() {
        let transport = NdjsonMcpTransport::default();
        let bad = r#"{"jsonrpc":"2.0","id":1}"#;
        let err = transport.decode_envelope(bad).unwrap_err();
        match err {
            McpError::MissingField("method") => {}
            other => panic!("Esperado MissingField(method), recebido: {other:?}"),
        }
    }

    #[test]
    fn test_tool_result_error_marker() {
        let r = ToolResult::error("boom");
        assert!(r.is_error);
        assert_eq!(r.content[0].text, "boom");
    }
}
