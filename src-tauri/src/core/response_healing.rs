use std::borrow::Cow;

use jsonrepair::{repair_json, Options};

/// Opções canônicas do parser estrutural.
///
/// Defaults da crate `jsonrepair 0.1.0`:
/// - `tolerate_hash_comments = true`     → aceita `#` como comentário fora de strings
/// - `allow_python_keywords = true`     → normaliza `True`/`False`/`None` como primitivos
/// - `fenced_code_blocks = true`        → strippa cercas ` ```json ... ``` `
/// - `repair_undefined = true`          → `undefined` → `null`
/// - `normalize_js_nonfinite = true`    → `NaN`/`Infinity` → `null`
///
/// Todas essas transformações operam **estruturalmente** no tokenizer
/// recursivo — strings literais válidas do payload do usuário são
/// imutáveis (R1 da Linha Vermelha do Marco 4.9.2).
fn canonical_repair_options() -> Options {
    Options::default()
}

/// Interface zero-copy/alloc para reparo sintático de JSON malformado.
///
/// Retorna `Cow::Borrowed` se nenhuma alteração for necessária, evitando
/// alocações desnecessárias. Em caso de erro de parse, devolve o input
/// original (`fail-soft`): LLM upstream é best-effort, nunca panicar.
///
/// **Contrato (Marco 4.9.2 — Defesa SDC):** strings literais válidas
/// do payload do usuário são IMUTÁVEIS. Apenas primitivos soltos
/// (True/False/None/NaN) e delimitadores truncados são curados.
pub fn heal_malformed_json<'a>(input: &'a str) -> Cow<'a, str> {
    match repair_json(input, &canonical_repair_options()) {
        Ok(repaired) if repaired == input => Cow::Borrowed(input),
        Ok(repaired) => Cow::Owned(repaired),
        // Fail-soft: payload upstream é best-effort; preserva input original.
        Err(_) => Cow::Borrowed(input),
    }
}

/// Wrapper thin preservado para callers legados. Delega ao
/// `heal_malformed_json` que opera via parser recursivo estrutural
/// do `jsonrepair` — latência alvo: < 1ms para payloads típicos
/// de streaming SSE de LLM (< 4 KB).
///
/// **Substituiu** (Marco 4.9.2) a implementação manual anterior que
/// usava `.replace(": True", ": true")` cego, o que corrompia strings
/// literais válidas do payload do usuário (SDC adversarial).
pub fn repair_json_buffer(input: &str) -> String {
    heal_malformed_json(input).into_owned()
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

        eprintln!("Tempo de cura sintática: {:?}", elapsed);
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed.as_micros() < 1000,
            "Reparo sintático deve ser concluído em < 1ms (micro-segundos)"
        );
        #[cfg(debug_assertions)]
        assert!(
            elapsed.as_millis() < 50,
            "Reparo sintático em modo debug deve ser concluído em < 50ms"
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

    #[test]
    fn test_heal_malformed_json_cow() {
        // Input já compactado e válido → Cow::Borrowed (fast-path, zero alocação).
        let clean = "{\"status\":\"ok\"}";
        let healed_clean = heal_malformed_json(clean);
        assert!(
            matches!(healed_clean, Cow::Borrowed(_)),
            "Input já válido e compactado deve ser devolvido sem alocação: {healed_clean:?}"
        );

        // Input malformado (trailing comma) → Cow::Owned (alocação necessária).
        let malformed = "{\"status\":\"ok\",}";
        let healed_malformed = heal_malformed_json(malformed);
        assert!(
            matches!(healed_malformed, Cow::Owned(_)),
            "Input malformado deve alocar reparo: {healed_malformed:?}"
        );
        let owned = healed_malformed.into_owned();
        // Marco 4.9.2: o `jsonrepair` compacta whitespace por design.
        // Validação semântica (Value equality), não byte-a-byte.
        let parsed: serde_json::Value = serde_json::from_str(&owned)
            .expect("Output deve ser RFC 8259 válido");
        let expected: serde_json::Value = serde_json::from_str("{\"status\":\"ok\"}").unwrap();
        assert_eq!(parsed, expected, "Conteúdo semântico deve ser preservado");
    }

    /// SOULS Marco 4.9.2 — Defesa contra Corrupção Silenciosa de Dados (SDC).
    /// Garante que strings literais válidas do payload do usuário NUNCA são
    /// tocadas, mesmo que contenham substrings que o parser manual cego
    /// (`.replace(": True", ": true")`) interpretaria como primitivos Python.
    #[test]
    fn test_response_healing_with_user_strings() {
        // String legítima do usuário contém ": True" — o parser manual corromperia.
        let input = r#"{"query": "Answer: True", "data": [1, 2, 3,"#;
        let healed = heal_malformed_json(input);
        let healed_owned = healed.into_owned();

        // A string do usuário deve permanecer byte-a-byte intacta.
        assert!(
            healed_owned.contains(r#""query":"Answer: True""#),
            "String literal do usuário corrompida. Output: {healed_owned}"
        );

        // A estrutura deve estar RFC 8259 válida.
        let parsed: serde_json::Value = serde_json::from_str(&healed_owned)
            .unwrap_or_else(|e| panic!("JSON curado inválido: {e}. Output: {healed_owned}"));

        assert_eq!(parsed["query"], "Answer: True");
        assert_eq!(parsed["data"], serde_json::json!([1, 2, 3]));
    }

    /// SOULS Marco 4.9.2 — Normalização estrutural de primitivos Python/JS.
    /// True/False/None soltos (sem aspas) são normalizados pelo parser
    /// recursivo, mas strings contendo essas substrings são preservadas.
    #[test]
    fn test_response_healing_normalizes_python_primitives_structurally() {
        let input = r#"{status: True, count: None, ok: False}"#;
        let healed = heal_malformed_json(input);
        let parsed: serde_json::Value = serde_json::from_str(&healed)
            .expect("primitivos Python devem ser normalizados estruturalmente");

        assert_eq!(parsed["status"], true);
        assert_eq!(parsed["count"], serde_json::Value::Null);
        assert_eq!(parsed["ok"], false);
    }

    /// SOULS Marco 4.9.2 — Latência obrigatória do gate SSE (< 1ms em release).
    /// O parser do `jsonrepair` é zero-copy e opera em &str puro.
    #[test]
    fn test_response_healing_sub_millisecond_after_repair() {
        let malformed = r#"```json
        {"choices": [{"delta": {"content": "Hello", "role": "assistant"},}], "usage": {"prompt_tokens": 10}}"#;
        let start = Instant::now();
        let _ = repair_json_buffer(malformed);
        let elapsed = start.elapsed();
        eprintln!("Latência de cura após reescrita: {:?}", elapsed);
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed.as_micros() < 1000,
            "Reparo deve ser < 1ms (release); medido: {elapsed:?}"
        );
    }
}
