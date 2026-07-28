// SODA-CANIBALIZED Fase 3: Smart Filtering ANSI.
//
// Transcrição nativa de `lean-ctx/src/core/compressor.rs::strip_ansi` (linhas 3-23)
// e `ansi_density` (linhas 25-31). Sem dependência de `regex` ou qualquer crate
// do cadáver. Varredura de chars em O(N) sobre o slice.

/// Remove sequências de escape ANSI (`\x1b[...m`, `\x1b[1m`, etc.).
///
/// Otimização early-return: se a string não contém nenhum `\x1b`, retorna uma cópia
/// direta sem iterar chars.
///
/// Algoritmo: varre chars. Ao encontrar `\x1b`, entra em modo `in_escape = true` e
/// suprime chars subsequentes até encontrar um ASCII alpha (m, K, H, J, etc.).
/// Strings sem ANSI são copiadas 1:1.
pub fn strip_ansi(s: &str) -> String {
    if !s.contains('\x1b') {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }
        result.push(c);
    }
    result
}

/// Densidade de códigos ANSI em uma string (0.0 a 1.0).
/// Retorna 0.0 para string vazia.
pub fn ansi_density(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let escape_bytes = s.chars().filter(|&c| c == '\x1b').count();
    escape_bytes as f64 / s.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_color_codes() {
        let raw = "\x1b[31mERROR\x1b[0m: failed at \x1b[1mline 42\x1b[0m";
        let cleaned = strip_ansi(raw);
        assert_eq!(cleaned, "ERROR: failed at line 42");
    }

    #[test]
    fn strip_ansi_passthrough_when_no_escape() {
        let plain = "no ansi here, just text";
        assert_eq!(strip_ansi(plain), plain);
    }

    #[test]
    fn strip_ansi_handles_cursor_and_clear_codes() {
        let raw = "\x1b[2K\x1b[Hclear screen and home";
        let cleaned = strip_ansi(raw);
        assert_eq!(cleaned, "clear screen and home");
    }

    #[test]
    fn ansi_density_is_zero_for_plain_text() {
        assert_eq!(ansi_density("plain text"), 0.0);
    }

    #[test]
    fn ansi_density_counts_only_esc_chars() {
        let raw = "\x1b[31mred\x1b[0m";
        let d = ansi_density(raw);
        // 2 escapes em 11 chars = 0.1818...
        assert!(d > 0.15 && d < 0.20);
    }

    #[test]
    fn ansi_density_handles_empty_string() {
        assert_eq!(ansi_density(""), 0.0);
    }
}
