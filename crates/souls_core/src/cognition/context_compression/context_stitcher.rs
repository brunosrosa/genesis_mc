// SOULS V6 — MARCO 5.5.0: Tecelão de Contexto (ContextStitcher)
//
// Governa a composição em 4 Zonas rígidas e seriais do prompt de inferência:
// - Z1: System Prompt (SODA Canon RAW)
// - Z2: Tool Schemas (Esquemas JSON-RPC MCP ordenados alfabeticamente)
// - Z3: Visão Materializada de Estado (Snapshot de memória local)
// - Z4: Sufixo Dinâmico (Entrada do chat e histórico ativo efêmero)
//
// Aplica alinhamento de bordas de bloco (Block Boundary Padding) a cada uma
// das três zonas estáveis (Z1, Z2, Z3) forçando encerramento em múltiplos de 64 tokens.

use serde_json::Value;
use crate::core::gigatoken_encoder::GigaTokenEncoder;

/// Conta o número de tokens de um texto usando o tokenizador CPU do Marco 5.4.0.
pub fn count_tokens_gigatoken(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    match GigaTokenEncoder::global().tokenize_to_bin(text) {
        Ok(tokens) => tokens.len(),
        Err(_) => text.len() / 4,
    }
}

/// Aplica o 'Block Boundary Padding' a uma zona textual:
/// Insere preenchimentos silenciosos (" /* pad */") ao final do texto para forçar
/// que a contagem de tokens seja exatamente um múltiplo de 64 (N % 64 == 0).
pub fn pad_zone_to_64_tokens(text: &str) -> String {
    if text.trim().is_empty() {
        return text.to_string();
    }
    let mut padded = text.to_string();
    let count = count_tokens_gigatoken(&padded);
    if count == 0 {
        return padded;
    }
    let rem = count % 64;
    if rem == 0 {
        return padded;
    }

    // Preenche com comentário silencioso " /* pad */"
    while !count_tokens_gigatoken(&padded).is_multiple_of(64) {
        padded.push_str(" /* pad */");
    }
    padded
}

/// O Tecelão de Contexto responsável por compor o prompt final.
#[derive(Debug, Clone)]
pub struct ContextStitcher {
    pub z1_system_prompt: String,
    pub z2_tool_schemas: Vec<Value>,
    pub z3_materialized_state: String,
    pub z4_dynamic_suffix: String,
}

impl ContextStitcher {
    pub fn new(
        z1_system_prompt: String,
        z2_tool_schemas: Vec<Value>,
        z3_materialized_state: String,
        z4_dynamic_suffix: String,
    ) -> Self {
        Self {
            z1_system_prompt,
            z2_tool_schemas,
            z3_materialized_state,
            z4_dynamic_suffix,
        }
    }

    /// Retorna Z1 (System Prompt) com padding de 64 tokens.
    pub fn z1_padded(&self) -> String {
        pad_zone_to_64_tokens(&self.z1_system_prompt)
    }

    /// Retorna Z2 (Tool Schemas ordenados alfabeticamente por nome) com padding de 64 tokens.
    pub fn z2_padded(&self) -> String {
        let mut schemas = self.z2_tool_schemas.clone();
        schemas.sort_by(|a, b| {
            let name_a = a.get("name").and_then(Value::as_str).unwrap_or_default();
            let name_b = b.get("name").and_then(Value::as_str).unwrap_or_default();
            name_a.cmp(name_b)
        });
        let z2_raw = serde_json::to_string(&schemas).unwrap_or_default();
        pad_zone_to_64_tokens(&z2_raw)
    }

    /// Retorna Z3 (Materialized State View) com padding de 64 tokens.
    pub fn z3_padded(&self) -> String {
        pad_zone_to_64_tokens(&self.z3_materialized_state)
    }

    /// Retorna Z4 (Dynamic Suffix).
    pub fn z4_dynamic(&self) -> &str {
        &self.z4_dynamic_suffix
    }

    /// Junta as 4 Zonas no prompt final de inferência.
    pub fn stitch(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}",
            self.z1_padded(),
            self.z2_padded(),
            self.z3_padded(),
            self.z4_dynamic()
        )
    }

    /// BARREIRA DE DOMÍNIO: Verifica se compressores estatísticos de prosa (ex: LLMLingua-2)
    /// têm permissão para processar o caminho ou bloco fornecido.
    /// É TERMINANTEMENTE PROIBIDO aplicar compressores de prosa em código-fonte.
    pub fn is_prose_compressor_allowed_for_path(path_or_content: &str) -> bool {
        let path = path_or_content.trim();
        // Extensões de código-fonte banidas de compressores de prosa
        let source_extensions = [
            ".rs", ".ts", ".js", ".py", ".c", ".cpp", ".h", ".hpp",
            ".cs", ".go", ".java", ".svelte", ".html", ".css", ".json",
            ".toml", ".yaml", ".yml", ".sql", ".sh", ".ps1",
        ];

        for ext in source_extensions {
            if path.ends_with(ext) {
                return false; // Proibido compressor de prosa em código-fonte
            }
        }

        // Se o conteúdo contém marcadores de código ou muitas chaves/sintaxe
        if path.contains("fn ") || path.contains("struct ") || path.contains("impl ")
            || path.contains("pub mod") || path.contains("class ") || path.contains("def ")
        {
            return false;
        }

        true
    }
}
