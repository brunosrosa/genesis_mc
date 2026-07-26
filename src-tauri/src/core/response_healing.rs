/// Intercepta buffers de streaming/resposta de LLMs e realiza cura sintática zero-token em < 1ms.
/// Remove cercas Markdown, limpa trailing commas, fecha delimitadores truncados e normaliza literais.
pub fn repair_json_buffer(input: &str) -> String {
    let mut cleaned = input.trim();

    // 1. Stripping de cercas Markdown (```json ... ``` ou ``` ... ```)
    if cleaned.starts_with("```") {
        if let Some(first_newline) = cleaned.find('\n') {
            cleaned = &cleaned[first_newline + 1..];
        }
        if cleaned.ends_with("```") {
            cleaned = &cleaned[..cleaned.len() - 3];
        }
        cleaned = cleaned.trim();
    }

    let mut out = String::with_capacity(cleaned.len() + 16);
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = cleaned.chars().peekable();

    while let Some(ch) = chars.next() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && in_string {
            escaped = true;
            out.push(ch);
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            out.push(ch);
            continue;
        }

        if in_string {
            out.push(ch);
            continue;
        }

        // Fora de strings: analisar delimitadores e vírgulas pendentes
        match ch {
            '{' => {
                stack.push('}');
                out.push(ch);
            }
            '[' => {
                stack.push(']');
                out.push(ch);
            }
            '}' | ']' => {
                // Remover vírgula pendente logo antes de fechar objeto/array
                while out.ends_with(|c: char| c.is_whitespace() || c == ',') {
                    out.pop();
                }
                if let Some(top) = stack.last() {
                    if *top == ch {
                        stack.pop();
                    }
                }
                out.push(ch);
            }
            ',' => {
                // Espiar próximo caractere não-espaço para ignorar trailing comma
                let mut peek_chars = chars.clone();
                let mut is_trailing = false;
                while let Some(&next_c) = peek_chars.peek() {
                    if next_c.is_whitespace() {
                        peek_chars.next();
                    } else {
                        if next_c == '}' || next_c == ']' {
                            is_trailing = true;
                        }
                        break;
                    }
                }
                if !is_trailing {
                    out.push(ch);
                }
            }
            _ => {
                out.push(ch);
            }
        }
    }

    // Se a string foi truncada no meio de uma string JSON, fecha as aspas
    if in_string {
        out.push('"');
    }

    // Auto-fechamento por pilha de delimitadores para estruturas truncadas por limite de contexto
    while let Some(closing) = stack.pop() {
        // Remover vírgulas pendentes ao final do buffer antes de fechar
        while out.ends_with(|c: char| c.is_whitespace() || c == ',') {
            out.pop();
        }
        out.push(closing);
    }

    // Normalização de literais Python/JS
    let result = out
        .replace(": True", ": true")
        .replace(": False", ": false")
        .replace(": None", ": null")
        .replace(": undefined", ": null");

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_response_healing_sub_millisecond_json_repair() {
        let malformed_json = "```json\n{\"status\": \"success\", \"data\": [1, 2, 3, ]";
        let start = Instant::now();
        let repaired = repair_json_buffer(malformed_json);
        let elapsed = start.elapsed();

        println!("Tempo de cura sintática: {:?}", elapsed);
        assert!(
            elapsed.as_micros() < 1000,
            "Reparo sintático deve ser concluído em < 1ms (micro-segundos)"
        );

        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&repaired);
        assert!(
            parsed.is_ok(),
            "JSON curado deve ser estritamente válido RFC 8259. Resultado: {}",
            repaired
        );

        let val = parsed.unwrap();
        assert_eq!(val["status"], "success");
        assert_eq!(val["data"], serde_json::json!([1, 2, 3]));
    }
}
