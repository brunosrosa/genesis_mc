// SODA-CANIBALIZED: Leitor Inteligente (Token-Aware Auto-Shrink)
// Medição de tokens via tiktoken (cl100k_base) com desidratação em camadas e Fail-Closed.

use super::ansi_filter::strip_ansi;
use super::text_compress::lightweight_cleanup;
use tiktoken::get_encoding;

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
    let initial_tokens = count_tokens(text);
    if initial_tokens <= max_tokens_budget {
        return Ok(text.to_string());
    }

    // Passo A: Poda de prosa e logs (lightweight_cleanup + ANSI filter)
    let cleaned = lightweight_cleanup(&strip_ansi(text));
    if count_tokens(&cleaned) <= max_tokens_budget {
        return Ok(cleaned);
    }

    // Passo B: Poda Sintática de Funções (extração de assinaturas/outline estrutural)
    let outlined = extract_outline_signatures(&cleaned);
    if count_tokens(&outlined) <= max_tokens_budget {
        return Ok(outlined);
    }

    // Passo C: Fail-Closed
    Err((
        -32010,
        format!(
            "Context Budget Exceeded: arquivo desidratado com {} tokens excede o limite estrito de {} tokens.",
            count_tokens(&outlined),
            max_tokens_budget
        ),
    ))
}

/// Extrai assinaturas sintáticas e preserva a casca estrutural omitindo corpos de funções.
pub fn extract_outline_signatures(code: &str) -> String {
    let mut out = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub struct")
            || trimmed.starts_with("struct")
            || trimmed.starts_with("pub enum")
            || trimmed.starts_with("enum")
            || trimmed.starts_with("pub trait")
            || trimmed.starts_with("trait")
            || trimmed.starts_with("impl")
            || trimmed.starts_with("pub fn")
            || trimmed.starts_with("fn")
            || trimmed.starts_with("pub const")
            || trimmed.starts_with("pub type")
            || trimmed.starts_with("use ")
            || trimmed.starts_with("mod ")
        {
            if let Some(brace_idx) = line.find('{') {
                out.push(format!("{} {{ /* body omitted */ }}", &line[..brace_idx].trim_end()));
            } else {
                out.push(line.to_string());
            }
        }
    }
    if out.is_empty() {
        // Fallback para texto não-código: pega os primeiros parágrafos se não for código Rust/estruturado
        code.lines().take(50).collect::<Vec<_>>().join("\n")
    } else {
        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_read_budget_enforcement() {
        // Gera um código com corpo de função longo que excede o orçamento
        let mut code = String::from("pub fn large_function() {\n");
        for i in 0..3000 {
            code.push_str(&format!("    let var_{i} = {i};\n"));
        }
        code.push_str("}\n");

        let initial_count = count_tokens(&code);
        assert!(initial_count > 500);

        // Aplica o smart_read com orçamento restrito
        let result = smart_read_text(&code, 200).expect("Deve desidratar com sucesso");
        let shrink_tokens = count_tokens(&result);
        assert!(
            shrink_tokens <= 200,
            "Contagem de tokens {shrink_tokens} excedeu o limite de 200"
        );
        assert!(result.contains("/* body omitted */"));

        // Orçamento impossível -> Fail-Closed (-32010)
        let err = smart_read_text(&code, 5);
        assert!(err.is_err());
        let (code_err, msg) = err.unwrap_err();
        assert_eq!(code_err, -32010);
        assert!(msg.contains("Context Budget Exceeded"));
    }
}
