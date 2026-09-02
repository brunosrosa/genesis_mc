// SOULS-CANIBALIZED Fase 3: Compressão textual estrutural.
//
// Transcrição nativa de:
//   - `aggressive_compress`  ← lean-ctx/src/core/compressor.rs::aggressive_compress (linhas 33-101)
//   - `lightweight_cleanup`  ← lean-ctx/src/core/compressor.rs::lightweight_cleanup (linhas 105-150)
//
// Sem dependência de `regex` ou qualquer crate do cadáver. Apenas stdlib.

/// SOULS Marco 4.9.2 — Cerca perimétrica de prosa.
///
/// Extensões NESTA lista são tratadas como **prosa literária / Markdown**:
/// stripping de comentários (`#`, `//`, `/* */`, `--`) é TERMINANTEMENTE
/// PROIBIDO porque corromperia cabeçalhos Markdown (`# Título`), bullet
/// points iniciados com `-`/`*` e blocos de fenced code.
///
/// `None` (extensão desconhecida) é tratado como prosa por padrão conservador
/// (fail-closed: é mais seguro preservar do que strippar).
const PROSE_EXTENSIONS: &[&str] = &["md", "markdown", "mdx"];

/// Helper de detecção de prosa. Usado pelo `aggressive_compress` como
/// curto-circuito para bypass total do stripping de comentários.
fn is_prose(ext: Option<&str>) -> bool {
    ext.is_none_or(|e| PROSE_EXTENSIONS.contains(&e))
}

/// Remove linhas de comentário por extensão de arquivo e concatena runs de chaves.
///
/// **Marco 4.9.2 — Cerca perimétrica invertida:**
/// - `ext = Some("md" | "markdown" | "mdx")` ou `ext = None` → **BYPASS**:
///   nenhum stripping é aplicado. Cabeçalhos `#`, `//` em prosa, fenced
///   code blocks ` ``` `, e bullet points permanecem intactos.
/// - Demais extensões reconhecidas: stripping estrutural por linguagem.
///
/// Extensões reconhecidas (whitelist de código-fonte):
///   - `.rs`, `.c`, `.cpp`, `.h`, `.js`, `.ts`, `.tsx`, `.jsx`, `.go`, `.java`
///     → strip de `//` e `/* */`
///   - `.py`, `.sh`, `.bash`, `.zsh`, `.rb`, `.yaml`, `.toml`
///     → strip de `#` (mas preserva `#!` shebang)
///   - `.sql`     → strip de `--`
///   - `.html`, `.htm`, `.xml`, `.svg` → strip de `<!-- -->`
///
/// Linhas em branco são descartadas. Runs de `}` / `);` / `});` consecutivos são
/// **concatenadas** na última linha do resultado.
///
/// **Correção sobre o cadáver:** a versão original do `lean-ctx` tinha um bug —
/// ela checava `last.trim() == "}"` mas após a primeira concatenação a linha
/// vira `"}}"` e a próxima `}` não casa mais. Aqui usamos `is_brace_only(last)`
/// que aceita `"}}"`, `"}};"`, etc.
pub fn aggressive_compress(content: &str, ext: Option<&str>) -> String {
    // SOULS Marco 4.9.2 — Curto-circuito antecipado para prosa/Markdown.
    // Se a extensão indica prosa, bypassa TODO o stripping de comentário e
    // aplica apenas a concatenação de brace runs (que não depende de comentários).
    if is_prose(ext) {
        return collapse_brace_runs_only(content);
    }

    let is_python = matches!(ext, Some("py" | "sh" | "bash" | "zsh" | "rb" | "yaml" | "toml"));
    let is_html = matches!(ext, Some("html" | "htm" | "xml" | "svg"));
    let is_sql = matches!(ext, Some("sql"));

    let mut result: Vec<String> = Vec::new();
    let mut in_block_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if in_block_comment {
            if trimmed.contains("*/") || (is_html && trimmed.contains("-->")) {
                in_block_comment = false;
            }
            continue;
        }

        if trimmed.starts_with("/*") || (is_html && trimmed.starts_with("<!--")) {
            if !(trimmed.contains("*/") || trimmed.contains("-->")) {
                in_block_comment = true;
            }
            continue;
        }

        // Strip line comments (sem shebang Python/Shell).
        if trimmed.starts_with("//") && !trimmed.starts_with("///") {
            continue;
        }
        if is_python && trimmed.starts_with('#') && !trimmed.starts_with("#!") {
            continue;
        }
        if is_sql && trimmed.starts_with("--") {
            continue;
        }

        // Concatena runs de `}` / `);` / `});` / `)` na última linha.
        // Usa is_brace_only() para tolerar concatenações anteriores (}}} etc.).
        if matches!(trimmed, "}" | "};" | ");" | "});" | ")") {
            if let Some(last) = result.last() {
                if is_brace_only(last.trim()) {
                    if let Some(last_mut) = result.last_mut() {
                        last_mut.push_str(trimmed);
                    }
                    continue;
                }
            }
            result.push(trimmed.to_string());
            continue;
        }

        result.push(line.to_string());
    }

    result.join("\n")
}

/// SOULS Marco 4.9.2 — Bypass para prosa: aplica APENAS a concatenação
/// de brace runs (idêntica à lógica do `aggressive_compress`), preservando
/// linhas de comentário, cabeçalhos, bullet points e fenced code intactos.
///
/// É a versão "cirúrgica" do compressor para `ext ∈ {md, markdown, mdx, None}`.
fn collapse_brace_runs_only(content: &str) -> String {
    let mut result: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Concatena runs de `}` / `);` / `});` / `)` na última linha.
        if matches!(trimmed, "}" | "};" | ");" | "});" | ")") {
            if let Some(last) = result.last() {
                if is_brace_only(last.trim()) {
                    if let Some(last_mut) = result.last_mut() {
                        last_mut.push_str(trimmed);
                    }
                    continue;
                }
            }
            result.push(trimmed.to_string());
            continue;
        }

        result.push(line.to_string());
    }

    result.join("\n")
}

/// Retorna true se a linha é composta apenas por chaves/parênteses de fechamento
/// concatenados (ex: `"}"`, `"}}"`, `"};}"`, `"););"`). Usado para colapsar runs.
fn is_brace_only(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| matches!(c, '}' | ';' | ')'))
}

/// Colapsa linhas em branco consecutivas em no máximo 1 e runs de chaves.
///
/// Diferente de `aggressive_compress`, **não remove comentários** —
/// é seguro aplicar sobre qualquer conteúdo textual arbitrário.
pub fn lightweight_cleanup(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    let mut result: Vec<String> = Vec::new();
    let mut blank_count: u32 = 0;
    let mut brace_run: Vec<&str> = Vec::new();

    let flush_brace_run = |run: &mut Vec<&str>, out: &mut Vec<String>| {
        if run.is_empty() {
            return;
        }
        if total <= 200 || run.len() <= 5 {
            for l in run.iter() {
                out.push(l.to_string());
            }
        } else {
            out.push(run[0].to_string());
            out.push(run[1].to_string());
            out.push(format!("[{} brace-only lines collapsed]", run.len() - 2));
        }
        run.clear();
    };

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            flush_brace_run(&mut brace_run, &mut result);
            blank_count += 1;
            if blank_count <= 1 {
                result.push(String::new());
            }
            continue;
        }
        blank_count = 0;

        if matches!(trimmed, "}" | "};" | ");" | "});" | ")") {
            brace_run.push(trimmed);
            continue;
        }

        flush_brace_run(&mut brace_run, &mut result);
        result.push(line.to_string());
    }
    flush_brace_run(&mut brace_run, &mut result);

    result.join("\n")
}

use std::path::Path;
use super::ansi_filter::strip_ansi;

/// Limite rígido de leitura de arquivo: 5 MB.
pub const MAX_READ_BYTES: u64 = 5 * 1024 * 1024;

/// Orquestrador nativo do Saco a Vácuo:
/// 1. `strip_ansi` (remove sequências de escape ANSI)
/// 2. `aggressive_compress` (remove comentários por extensão)
/// 3. `lightweight_cleanup` (colapsa blank lines + brace runs)
pub fn compress_to_lean(text: &str, ext: Option<&str>) -> String {
    let stripped = strip_ansi(text);
    let compressed = aggressive_compress(&stripped, ext);
    lightweight_cleanup(&compressed)
}

/// Lê um arquivo do disco aplicando o pipeline `compress_to_lean`.
pub fn read_to_lean(path: &Path) -> std::io::Result<String> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_READ_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Arquivo excede o limite rígido de {} bytes ({} bytes reais). \
                 O Saco a Vácuo nativo do SOULS recusa payloads >5MB para proteger a VRAM.",
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
    fn aggressive_compress_strips_rust_line_comments() {
        let raw = "fn main() {\n    // debug print\n    println!(\"hi\");\n}";
        let out = aggressive_compress(raw, Some("rs"));
        assert!(!out.contains("// debug print"), "comment not stripped: {out}");
        assert!(out.contains("fn main()"), "code lost: {out}");
    }

    #[test]
    fn aggressive_compress_strips_python_comments_but_preserves_shebang() {
        let raw = "#!/usr/bin/env python3\n# debug\nimport os\n";
        let out = aggressive_compress(raw, Some("py"));
        assert!(out.contains("#!/usr/bin/env python3"), "shebang lost: {out}");
        assert!(!out.contains("# debug"), "comment not stripped: {out}");
        assert!(out.contains("import os"));
    }

    #[test]
    fn aggressive_compress_strips_sql_comments() {
        // Linhas de comentário em SQL começam com `--` (linha pura, não in-line).
        let raw = "-- debug comment\nSELECT 2;\n";
        let out = aggressive_compress(raw, Some("sql"));
        assert!(!out.contains("-- debug comment"), "comment not stripped: {out}");
        assert!(out.contains("SELECT 2"), "code lost: {out}");
    }

    #[test]
    fn aggressive_compress_collapses_long_brace_runs() {
        // Comportamento idêntico ao cadáver: 4 linhas consecutivas de `}` viram
        // UMA única linha `}}}}` (concatenação na última entrada).
        let raw = "}\n}\n}\n}\n";
        let out = aggressive_compress(raw, Some("rs"));
        let line_count = out.lines().count();
        assert_eq!(
            line_count, 1,
            "expected 1 collapsed line, got {line_count}: {out}"
        );
        assert!(out.contains("}}}}"), "expected 4-brace concatenation: {out}");
    }

    #[test]
    fn lightweight_cleanup_collapses_consecutive_blank_lines() {
        let raw = "a\n\n\n\n\nb";
        let out = lightweight_cleanup(raw);
        // Apenas 1 blank line entre a e b.
        assert_eq!(out, "a\n\nb");
    }

    #[test]
    fn lightweight_cleanup_preserves_code_with_brace_run() {
        let raw = "fn a() {\n}\nfn b() {\n}\n";
        let out = lightweight_cleanup(raw);
        // 200 linhas ou menos, brace runs <= 5 são preservadas individualmente.
        assert!(out.contains("fn a()"));
        assert!(out.contains("}"));
    }

    /// SOULS Marco 4.9.2 — Cercadinho de prosa preserva cabeçalhos Markdown.
    /// Prova que `aggressive_compress` com `ext = Some("md")` bypassa
    /// TODO o stripping de comentário (R4 da Linha Vermelha).
    #[test]
    fn aggressive_compress_preserves_markdown_headers() {
        let raw = "# Título\n## Subtítulo\n- item 1\n- item 2\n```rust\nfn main() {}\n```\n";
        let out = aggressive_compress(raw, Some("md"));

        assert!(out.contains("# Título"), "Cabeçalho H1 perdido: {out}");
        assert!(out.contains("## Subtítulo"), "Cabeçalho H2 perdido: {out}");
        assert!(out.contains("- item 1"), "Bullet perdido: {out}");
        assert!(out.contains("- item 2"), "Bullet perdido: {out}");
        assert!(out.contains("fn main()"), "Fenced code perdido: {out}");
    }

    /// SOULS Marco 4.9.2 — Cercadinho com `ext = None` trata como prosa.
    /// Fail-closed: extensão desconhecida é preservada inalterada.
    #[test]
    fn aggressive_compress_preserves_prose_with_none_ext() {
        let raw = "# Capítulo\n// nota de revisão\n- bullet point\n```\nbloco fenced\n```\n";
        let out = aggressive_compress(raw, None);

        assert!(out.contains("# Capítulo"), "H1 perdido: {out}");
        assert!(out.contains("// nota de revisão"), "// prosa strippada: {out}");
        assert!(out.contains("- bullet point"), "Bullet strippado: {out}");
        assert!(out.contains("bloco fenced"), "Fenced code perdido: {out}");
    }

    /// SOULS Marco 4.9.2 — Cercadinho com `ext = Some("markdown")` (alias) também preserva.
    #[test]
    fn aggressive_compress_preserves_markdown_alias() {
        let raw = "# H1\n- bullet\n// comment-style\n";
        let out = aggressive_compress(raw, Some("markdown"));
        assert!(out.contains("# H1"), "H1 perdido: {out}");
        assert!(out.contains("- bullet"), "Bullet perdido: {out}");
        assert!(out.contains("// comment-style"), "// perdido: {out}");
    }

    /// SOULS Marco 4.9.2 — Cercadinho NÃO quebra código-fonte legítimo.
    /// Garante que a whitelist invertida não regredir o comportamento para `.rs`.
    #[test]
    fn aggressive_compress_still_strips_rust_when_ext_is_rs() {
        let raw = "// debug print\nfn main() {}\n/* block comment */\nlet x = 1;\n";
        let out = aggressive_compress(raw, Some("rs"));
        assert!(!out.contains("// debug print"), "Rust comment não strippado: {out}");
        assert!(!out.contains("/* block comment */"), "Rust block comment não strippado: {out}");
        assert!(out.contains("fn main()"), "Código Rust perdido: {out}");
        assert!(out.contains("let x = 1;"), "Código Rust perdido: {out}");
    }
}
