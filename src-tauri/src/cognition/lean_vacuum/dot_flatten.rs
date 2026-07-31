// SOULS-CANIBALIZED Fase 3: LEAN Dot-Flattening encoder.
//
// Formato canônico LEAN para serialização compacta de `serde_json::Value`:
//
//   Object aninhado:  {"a": {"b": 1}}       ->  "a.b=1"
//   Array simples:    {"xs": [1, 2, 3]}      ->  "xs[0]=1\nxs[1]=2\nxs[2]=3"
//   Mixto:            {"u": [{"n": "a"}]}    ->  "u[0].n=a"
//   Booleanos:        {"on": true}           ->  "on=true"   (LEAN canônico: literais)
//   Null:             {"x": null}            ->  "x=null"
//   Strings:          {"m": "hello world"}   ->  "m=\"hello world\""
//
// Característica LEAN chave: booleanos são literais `true`/`false` (sem aspas),
// ao contrário de JSON. Strings SEMPRE recebem aspas duplas para preservar
// espaços, dois-pontos e caracteres que poderiam ser confundidos com sintaxe
// LEAN. A máquina de estados do `response_healing.rs` (Fase 1) usa esta
// garantia para evitar reparos cegos.

use serde_json::Value;

/// Codifica um `serde_json::Value` no formato LEAN (Dot-Flattening).
///
/// `prefix` é usado internamente pela recursão; callers externos devem passar `""`.
/// O separador de registros é `\n` (LF).
pub fn dot_flatten(value: &Value) -> String {
    let mut out = String::new();
    flatten_into(value, "", &mut out);
    // Remove o \n final se houver, para evitar string vazia trailing.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn flatten_into(value: &Value, prefix: &str, out: &mut String) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = prefix_key(prefix, k);
                if is_scalar(v) {
                    out.push_str(&key);
                    out.push('=');
                    push_scalar(v, out);
                    out.push('\n');
                } else {
                    flatten_into(v, &key, out);
                }
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = format!("{prefix}[{i}]");
                if is_scalar(v) {
                    out.push_str(&key);
                    out.push('=');
                    push_scalar(v, out);
                    out.push('\n');
                } else {
                    flatten_into(v, &key, out);
                }
            }
        }
        // Raiz escalar: emite "{prefix}={value}" ou apenas "{value}" se prefix vazio.
        scalar => {
            if !prefix.is_empty() {
                out.push_str(prefix);
                out.push('=');
            }
            push_scalar(scalar, out);
            out.push('\n');
        }
    }
}

fn is_scalar(v: &Value) -> bool {
    matches!(
        v,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn push_scalar(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
            out.push('"');
            // Escape mínimo: aspas duplas e contrabarras dentro da string.
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    other => out.push(other),
                }
            }
            out.push('"');
        }
        // Não esperado (is_scalar == false) — fallback defensivo.
        _ => out.push_str("<non-scalar>"),
    }
}

fn prefix_key(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dot_flatten_simple_object() {
        let v = json!({"a": {"b": {"c": 42}}});
        assert_eq!(dot_flatten(&v), "a.b.c=42");
    }

    #[test]
    fn dot_flatten_with_booleans_literal() {
        let v = json!({"enabled": true, "verbose": false});
        assert_eq!(dot_flatten(&v), "enabled=true\nverbose=false");
    }

    #[test]
    fn dot_flatten_array() {
        let v = json!({"items": [1, 2, 3]});
        assert_eq!(dot_flatten(&v), "items[0]=1\nitems[1]=2\nitems[2]=3");
    }

    #[test]
    fn dot_flatten_nested_array_of_objects() {
        let v = json!({"users": [{"name": "alice"}, {"name": "bob"}]});
        // Strings SEMPRE aspeadas (característica LEAN canônica — protege Response Healing
        // contra reparos cegos em literais que contenham `=` ou `:`).
        assert_eq!(
            dot_flatten(&v),
            "users[0].name=\"alice\"\nusers[1].name=\"bob\""
        );
    }

    #[test]
    fn dot_flatten_string_with_special_chars_preserves_value() {
        let v = json!({"msg": "true = false : end"});
        // Strings sempre aspeadas para preservar literais e espaços.
        assert_eq!(dot_flatten(&v), "msg=\"true = false : end\"");
    }

    #[test]
    fn dot_flatten_string_with_quotes_is_escaped() {
        let v = json!({"msg": "she said \"hi\""});
        assert_eq!(dot_flatten(&v), "msg=\"she said \\\"hi\\\"\"");
    }

    #[test]
    fn dot_flatten_null_value() {
        let v = json!({"x": null});
        assert_eq!(dot_flatten(&v), "x=null");
    }

    #[test]
    fn dot_flatten_mixed_types_in_object() {
        let v = json!({"name": "alice", "age": 30, "admin": true, "data": null});
        // Map ordering é BTreeMap (alfabético/determinístico). Verificamos o conjunto de
        // linhas e o formato de cada uma — sem depender da ordem de inserção.
        let out = dot_flatten(&v);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 4, "expected 4 lines, got: {out}");
        assert!(out.contains("name=\"alice\""), "missing name: {out}");
        assert!(out.contains("age=30"), "missing age: {out}");
        assert!(out.contains("admin=true"), "missing admin: {out}");
        assert!(out.contains("data=null"), "missing data: {out}");
    }

    #[test]
    fn dot_flatten_empty_object_returns_empty_string() {
        let v = json!({});
        assert_eq!(dot_flatten(&v), "");
    }

    #[test]
    fn dot_flatten_root_scalar() {
        assert_eq!(dot_flatten(&json!(true)), "true");
        assert_eq!(dot_flatten(&json!(42)), "42");
        assert_eq!(dot_flatten(&json!("hello")), "\"hello\"");
    }
}
