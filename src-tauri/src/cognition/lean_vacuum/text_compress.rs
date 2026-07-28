// SODA-CANIBALIZED Fase 3: Compressão textual estrutural.
//
// Transcrição nativa de:
//   - `aggressive_compress`  ← lean-ctx/src/core/compressor.rs::aggressive_compress (linhas 33-101)
//   - `lightweight_cleanup`  ← lean-ctx/src/core/compressor.rs::lightweight_cleanup (linhas 105-150)
//
// Sem dependência de `regex` ou qualquer crate do cadáver. Apenas stdlib.

/// Remove linhas de comentário por extensão de arquivo e concatena runs de chaves.
///
/// Extensões reconhecidas:
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
}
