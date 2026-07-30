// SODA-CANIBALIZED Fase 3: Saco a Vácuo Nativo.
//
// Transmutação pura da "Alma Matemática" do `third_party/lean-ctx/` (cadáver READ-ONLY).
// Este módulo é 100% nativo: zero dependência de `rmcp`, `axum`, `ratatui`, `crossterm`,
// `lettre`, `jsonwebtoken`, `rten`, `tokio-postgres` ou qualquer toxidade do cadáver.

use std::path::Path;

pub mod ansi_filter;
pub mod dedup;
pub mod dot_flatten;
pub mod myers_diff;
pub mod search;
pub mod smart_read;
pub mod text_compress;

pub use ansi_filter::{ansi_density, strip_ansi};
pub use dedup::{
    clear_session_cache, clear_session_dedup_cache, deduplicate_blocks, deduplicate_blocks_session,
    SESSION_DEDUP_CACHE,
};
pub use dot_flatten::dot_flatten;
pub use myers_diff::myers_diff;
pub use search::{format_lean_notation, search_lean, SearchMatch};
pub use smart_read::{count_tokens, smart_read_text};
pub use text_compress::{aggressive_compress, lightweight_cleanup};

/// Limite rígido de leitura de arquivo: 5 MB.
/// Acima disso, retorna `io::ErrorKind::InvalidData` para proteger a RAM/VRAM.
pub const MAX_READ_BYTES: u64 = 5 * 1024 * 1024;

/// Orquestrador nativo do Saco a Vácuo:
/// 1. `strip_ansi` (remove sequências de escape ANSI)
/// 2. `aggressive_compress` (remove comentários por extensão)
/// 3. `lightweight_cleanup` (colapsa blank lines + brace runs)
///
/// Retorna o texto limpo e compactado.
pub fn compress_to_lean(text: &str, ext: Option<&str>) -> String {
    let stripped = strip_ansi(text);
    let compressed = aggressive_compress(&stripped, ext);
    lightweight_cleanup(&compressed)
}

/// Lê um arquivo do disco aplicando o pipeline `compress_to_lean`.
/// Hard cap de 5 MB para proteger RAM/VRAM.
pub fn read_to_lean(path: &Path) -> std::io::Result<String> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_READ_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Arquivo excede o limite rígido de {} bytes ({} bytes reais). \
                 O Saco a Vácuo nativo do SODA recusa payloads >5MB para proteger a VRAM.",
                MAX_READ_BYTES,
                metadata.len()
            ),
        ));
    }
    let raw = std::fs::read_to_string(path)?;
    let ext = path.extension().and_then(|e| e.to_str());
    Ok(compress_to_lean(&raw, ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_to_lean_removes_ansi_and_comments() {
        let raw = "\x1b[31mERROR\x1b[0m\n// debug print\nreal line\n";
        let out = compress_to_lean(raw, Some("rs"));
        assert!(!out.contains("\x1b[31m"), "ANSI should be stripped: {out}");
        assert!(!out.contains("// debug print"), "comment not stripped: {out}");
        assert!(out.contains("real line"), "code lost: {out}");
    }

    #[test]
    fn read_to_lean_rejects_oversized_file() {
        let bogus = Path::new("__no_such_file_lean_vacuum__.rs");
        assert!(read_to_lean(bogus).is_err());
    }
}
