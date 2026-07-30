// SODA-CANIBALIZED: Leitor Inteligente Poliglota (Token-Aware Auto-Shrink)
// Medição de tokens via tiktoken (cl100k_base) com desidratação poliglota (Rust, Python, Elixir, etc.) e Fail-Closed.

use super::ansi_filter::strip_ansi;
use super::text_compress::lightweight_cleanup;
use tiktoken::get_encoding;

/// Paradigmas sintáticos de linguagens para poda cirúrgica de corpos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageParadigm {
    Brace,  // Rust, JS, TS, C, C++, C#, Go, Java (delimitadores {})
    Indent, // Python (indentação de colunas + pass)
    Block,  // Elixir, Ruby (blocos do/end)
}

/// Identifica dinamicamente o paradigma sintático da linguagem com base na extensão ou conteúdo.
pub fn detect_paradigm(code: &str, ext_or_path: Option<&str>) -> LanguageParadigm {
    if let Some(ext) = ext_or_path {
        let lower = ext.to_lowercase();
        if lower.ends_with(".py") || lower == "py" || lower == "python" {
            return LanguageParadigm::Indent;
        }
        if lower.ends_with(".ex") || lower.ends_with(".exs") || lower.ends_with(".rb") || lower == "ex" || lower == "elixir" || lower == "ruby" {
            return LanguageParadigm::Block;
        }
        if lower.ends_with(".rs") || lower.ends_with(".js") || lower.ends_with(".ts") || lower.ends_with(".tsx")
           || lower.ends_with(".cpp") || lower.ends_with(".c") || lower.ends_with(".h") || lower.ends_with(".hpp")
           || lower.ends_with(".cs") || lower.ends_with(".go") || lower.ends_with(".java") {
            return LanguageParadigm::Brace;
        }
    }
    // Fallback de auto-detecção por marcadores de conteúdo
    if code.contains("defmodule ") || code.contains("defp ") {
        LanguageParadigm::Block
    } else if code.contains("def ") && code.contains(":\n") && !code.contains('{') {
        LanguageParadigm::Indent
    } else {
        LanguageParadigm::Brace
    }
}

/// Conta o número de tokens do texto usando a encodagem `cl100k_base` na CPU.
pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    match get_encoding("cl100k_base") {
        Some(enc) => enc.count(text),
        None => text.len() / 4,
    }
}

/// Aplica a desidratação consciente do orçamento de tokens (smart_read).
pub fn smart_read_text(text: &str, max_tokens_budget: usize) -> Result<String, (i64, String)> {
    smart_read_text_for_lang(text, max_tokens_budget, None)
}

/// Aplica a desidratação poliglota baseada na extensão/linguagem do arquivo.
pub fn smart_read_text_for_lang(
    text: &str,
    max_tokens_budget: usize,
    ext_or_path: Option<&str>,
) -> Result<String, (i64, String)> {
    let initial_tokens = count_tokens(text);
    if initial_tokens <= max_tokens_budget {
        return Ok(text.to_string());
    }

    // Passo A: Poda de prosa e logs (lightweight_cleanup + ANSI filter)
    let cleaned = lightweight_cleanup(&strip_ansi(text));
    if count_tokens(&cleaned) <= max_tokens_budget {
        return Ok(cleaned);
    }

    // Passo B: Poda Sintática Poliglota de Funções (extração de assinaturas/outline estrutural)
    let outlined = extract_outline_signatures_polyglot(&cleaned, ext_or_path);
    if count_tokens(&outlined) <= max_tokens_budget {
        return Ok(outlined);
    }

    // Passo C: Fail-Closed (-32010)
    Err((
        -32010,
        format!(
            "Context Budget Exceeded: arquivo desidratado com {} tokens excede o limite estrito de {} tokens.",
            count_tokens(&outlined),
            max_tokens_budget
        ),
    ))
}

/// Extrai assinaturas sintáticas (compatibilidade retroativa com padrão Brace).
pub fn extract_outline_signatures(code: &str) -> String {
    extract_outline_signatures_polyglot(code, None)
}

/// Extrai assinaturas sintáticas aplicando a máquina de estados poliglota apropriada.
pub fn extract_outline_signatures_polyglot(code: &str, ext_or_path: Option<&str>) -> String {
    let paradigm = detect_paradigm(code, ext_or_path);
    match paradigm {
        LanguageParadigm::Brace => extract_brace_outline(code),
        LanguageParadigm::Indent => extract_python_outline(code),
        LanguageParadigm::Block => extract_elixir_outline(code),
    }
}

/// Padrão Brace (Rust, TS, JS, C++, C#, Go, Java): profundidade por chaves `{}`.
fn extract_brace_outline(code: &str) -> String {
    let mut result = String::with_capacity(code.len() / 2);
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_string = false;
    let mut string_char = '"';
    let mut in_char_literal = false;
    let mut escape = false;

    let mut pending_fn = false;
    let mut body_depth = 0usize;
    let mut suppressing_body = false;

    while i < len {
        let ch = chars[i];

        if escape {
            escape = false;
            if !suppressing_body {
                result.push(ch);
            }
            i += 1;
            continue;
        }

        if (in_string || in_char_literal) && ch == '\\' {
            escape = true;
            if !suppressing_body {
                result.push(ch);
            }
            i += 1;
            continue;
        }

        if !in_block_comment && !in_string && !in_char_literal && !in_line_comment && ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            in_line_comment = true;
        }

        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
            }
            if !suppressing_body {
                result.push(ch);
            }
            i += 1;
            continue;
        }

        if !in_line_comment && !in_string && !in_char_literal {
            if !in_block_comment && ch == '/' && i + 1 < len && chars[i + 1] == '*' {
                in_block_comment = true;
            } else if in_block_comment && ch == '*' && i + 1 < len && chars[i + 1] == '/' {
                if !suppressing_body {
                    result.push_str("*/");
                }
                in_block_comment = false;
                i += 2;
                continue;
            }
        }

        if in_block_comment {
            if !suppressing_body {
                result.push(ch);
            }
            i += 1;
            continue;
        }

        if !in_string && !in_char_literal {
            if ch == '"' || ch == '`' {
                in_string = true;
                string_char = ch;
            } else if ch == '\'' && i + 2 < len && chars[i + 2] == '\'' {
                in_char_literal = true;
            }
        } else if in_string && ch == string_char {
            in_string = false;
        } else if in_char_literal && ch == '\'' {
            in_char_literal = false;
        }

        if !in_string && !in_char_literal && !suppressing_body
            && (ch == 'f' || ch == 'd') && (i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_')
        {
            let rest: String = chars[i..len.min(i + 12)].iter().collect();
            if rest.starts_with("fn ") || rest.starts_with("fn(") || rest.starts_with("def ") || rest.starts_with("function ") {
                pending_fn = true;
            }
        }

        if !in_string && !in_char_literal {
            if ch == '{' {
                if pending_fn && !suppressing_body {
                    suppressing_body = true;
                    body_depth = 1;
                    pending_fn = false;
                    result.push_str("{ /* body omitted */ }");
                    i += 1;
                    continue;
                } else if suppressing_body {
                    body_depth += 1;
                    i += 1;
                    continue;
                }
            } else if ch == '}' {
                if suppressing_body {
                    body_depth -= 1;
                    if body_depth == 0 {
                        suppressing_body = false;
                    }
                    i += 1;
                    continue;
                }
            } else if ch == ';' && pending_fn && !suppressing_body {
                pending_fn = false;
            }
        }

        if !suppressing_body {
            result.push(ch);
        }

        i += 1;
    }

    if result.trim().is_empty() {
        code.lines().take(50).collect::<Vec<_>>().join("\n")
    } else {
        result
    }
}

/// Padrão Indent (Python): alinhamento por coluna + `pass  # body omitted`.
fn extract_python_outline(code: &str) -> String {
    let mut out = Vec::new();
    let mut suppressing_body = false;
    let mut suppress_indent = 0usize;

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !suppressing_body {
                out.push(line.to_string());
            }
            continue;
        }

        let current_indent = line.chars().take_while(|c| c.is_whitespace()).count();

        if suppressing_body {
            if current_indent > suppress_indent && !trimmed.starts_with("def ") && !trimmed.starts_with("class ") && !trimmed.starts_with("async def ") {
                continue;
            } else {
                suppressing_body = false;
            }
        }

        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            suppressing_body = true;
            suppress_indent = current_indent;
            if let Some(colon_idx) = line.rfind(':') {
                let sig = line[..colon_idx + 1].trim_end();
                out.push(format!("{sig} pass  # body omitted"));
            } else {
                out.push(format!("{line}: pass  # body omitted"));
            }
        } else if trimmed.starts_with("class ") {
            out.push(line.to_string());
        } else if trimmed.starts_with("import ") || trimmed.starts_with("from ") || trimmed.starts_with('#') || current_indent == 0 {
            out.push(line.to_string());
        } else if !suppressing_body {
            out.push(line.to_string());
        }
    }

    if out.is_empty() {
        code.lines().take(50).collect::<Vec<_>>().join("\n")
    } else {
        out.join("\n")
    }
}

/// Padrão Block (Elixir/Ruby): blocos `do ... end` + `do\n  # body omitted\nend`.
fn extract_elixir_outline(code: &str) -> String {
    let mut out = Vec::new();
    let mut suppressing_body = false;
    let mut do_depth = 0i32;

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !suppressing_body {
                out.push(line.to_string());
            }
            continue;
        }

        if suppressing_body {
            if trimmed.contains(" do") || trimmed.ends_with("do") {
                do_depth += 1;
            }
            if trimmed == "end" || trimmed.starts_with("end ") || trimmed.ends_with("end") {
                do_depth -= 1;
                if do_depth <= 0 {
                    suppressing_body = false;
                    let indent_size = line.chars().take_while(|c| c.is_whitespace()).count();
                    let indent = " ".repeat(indent_size);
                    out.push(format!("{indent}  # body omitted\n{indent}end"));
                }
            }
            continue;
        }

        if trimmed.starts_with("defmodule ") {
            out.push(line.to_string());
        } else if trimmed.starts_with("def ") || trimmed.starts_with("defp ") || trimmed.starts_with("defmacro ") {
            if trimmed.contains(", do:") {
                if let Some(idx) = line.find(", do:") {
                    let prefix = &line[..idx];
                    out.push(format!("{prefix}, do: :ok  # body omitted"));
                } else {
                    out.push(line.to_string());
                }
            } else if trimmed.contains(" do") || trimmed.ends_with("do") {
                suppressing_body = true;
                do_depth = 1;
                out.push(line.to_string());
            } else {
                out.push(line.to_string());
            }
        } else if trimmed.starts_with("use ") || trimmed.starts_with("import ") || trimmed.starts_with("alias ") || trimmed.starts_with('#') {
            out.push(line.to_string());
        } else if !suppressing_body {
            out.push(line.to_string());
        }
    }

    if out.is_empty() {
        code.lines().take(50).collect::<Vec<_>>().join("\n")
    } else {
        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_read_budget_enforcement_rust() {
        let mut code = String::from("pub fn large_function() {\n");
        for i in 0..3000 {
            code.push_str(&format!("    let var_{i} = {i};\n"));
        }
        code.push_str("}\n");

        let initial_count = count_tokens(&code);
        assert!(initial_count > 500);

        let result = smart_read_text_for_lang(&code, 200, Some("rs")).expect("Deve desidratar com sucesso");
        let shrink_tokens = count_tokens(&result);
        assert!(shrink_tokens <= 200, "Contagem de tokens {shrink_tokens} excedeu o limite de 200");
        assert!(result.contains("/* body omitted */"));

        let err = smart_read_text_for_lang(&code, 5, Some("rs"));
        assert!(err.is_err());
        let (code_err, msg) = err.unwrap_err();
        assert_eq!(code_err, -32010);
        assert!(msg.contains("Context Budget Exceeded"));
    }

    #[test]
    fn test_smart_read_polyglot_python_indent_pass() {
        let mut code = String::from("class DataProcessor:\n    def process_items(self):\n");
        for i in 0..1000 {
            code.push_str(&format!("        item_{i} = {i} * 2\n"));
        }
        code.push_str("        return True\n");

        let result = extract_outline_signatures_polyglot(&code, Some("py"));
        assert!(result.contains("class DataProcessor:"), "Deveria conter a classe");
        assert!(result.contains("def process_items(self): pass  # body omitted"), "Deveria amputar corpo Python com pass");
        assert!(!result.contains("item_500 = 1000"), "Corpo interno Python deveria ser desidratado");
    }

    #[test]
    fn test_smart_read_polyglot_elixir_block_end() {
        let mut code = String::from("defmodule PipelineEngine do\n  def run_heavy_pipeline(data) do\n");
        for i in 0..1000 {
            code.push_str(&format!("    step_{i} = transform(data, {i})\n"));
        }
        code.push_str("    {:ok, step_999}\n  end\nend\n");

        let result = extract_outline_signatures_polyglot(&code, Some("ex"));
        assert!(result.contains("defmodule PipelineEngine do"), "Deveria conter defmodule");
        assert!(result.contains("def run_heavy_pipeline(data) do"), "Deveria conter assinatura da funcao");
        assert!(result.contains("# body omitted"), "Deveria amputar bloco Elixir com comentario");
        assert!(result.contains("end"), "Deveria manter o end de encerramento do bloco");
        assert!(!result.contains("step_500 = transform"), "Linhas internas Elixir deveriam ser desidratadas");
    }
}
