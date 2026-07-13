/// Módulo de Purificação de Caminhos UNC — `path_sanitizer.rs`
///
/// Expõe a função `soda_clean_path` que:
/// 1. Remove matematicamente o prefixo `\\?\` (e variantes) de caminhos Windows
/// 2. Aplica `dunce::canonicalize()` se o path existir no filesystem
/// 3. Garante que `Command::current_dir` e chamadas Win32 do AppContainer
///    JAMAIS recebam caminhos com prefixo UNC poluído.
///
/// Esta função é pura, sem side-effects de mutação, e totalmente testável
/// sem acesso ao filesystem real (testes unitários offline).
use std::path::{Path, PathBuf};

/// Remove deterministicamente o prefixo UNC `\\?\` (e variantes) de um caminho Windows.
///
/// Variantes tratadas:
/// - `\\?\C:\foo`            -> `C:\foo`        (UNC namespace clássico)
/// - `\\?\UNC\server\share`  -> `\\server\share` (UNC de rede via namespace)
/// - `//?/C:/foo`            -> `C:/foo`         (variante forward-slash)
/// - Caminhos normais são retornados sem modificação.
///
/// Após a remoção do prefixo, se o path existir no filesystem, aplica
/// `dunce::canonicalize()` para resolver symlinks e normalizar separadores.
/// Se o path não existir (diretório efêmero ainda não criado), retorna o
/// path limpo sem canonicalize.
pub fn soda_clean_path(path: &Path) -> PathBuf {
    let clean_path = soda_strip_unc_prefix(path);

    // Aplica dunce::canonicalize somente se o path existir.
    // dunce é preferível a std::fs::canonicalize porque não adiciona \\?\ no Windows.
    if clean_path.exists() {
        dunce::canonicalize(&clean_path).unwrap_or(clean_path)
    } else {
        clean_path
    }
}

/// Remove deterministicamente o prefixo UNC de um Path, sem tocar o filesystem.
/// Usada internamente por `soda_clean_path` e diretamente em contextos de testes puros.
pub fn soda_strip_unc_prefix(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    let mut cleaned = raw.as_ref();

    // 1. Tratativa de caminhos UNC de Rede estendidos (com host/share)
    if cleaned.starts_with(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", &cleaned[r"\\?\UNC\".len()..]));
    }
    if cleaned.starts_with(r"\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", &cleaned[r"\?\UNC\".len()..]));
    }
    if cleaned.starts_with("//?/UNC/") {
        return PathBuf::from(format!("//{}", &cleaned["//?/UNC/".len()..]));
    }
    if cleaned.starts_with("/?/UNC/") {
        return PathBuf::from(format!("//{}", &cleaned["/?/UNC/".len()..]));
    }

    // 2. Tratativa de caminhos locais estendidos (como drives \\?\C:\ ou \?\C:\)
    if cleaned.starts_with(r"\\?\") {
        cleaned = &cleaned[r"\\?\".len()..];
    } else if cleaned.starts_with(r"\?\") {
        cleaned = &cleaned[r"\?\".len()..];
    } else if cleaned.starts_with("//?/") {
        cleaned = &cleaned["//?/".len()..];
    } else if cleaned.starts_with("/?/") {
        cleaned = &cleaned["/?/".len()..];
    }

    PathBuf::from(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // RED TESTS — soda_strip_unc_prefix (puro, sem filesystem)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_unc_extended_prefix_is_stripped() {
        // PRD-031 §B: O prefixo \\?\ DEVE ser removido deterministicamente.
        let input = Path::new(r"\\?\C:\Windows");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"C:\Windows"),
            r"\\?\C:\Windows deve ser limpo para C:\Windows");
    }

    #[test]
    fn test_unc_single_backslash_aberration_is_stripped() {
        // PRD-032 §3: A aberração \?\C:\Windows deve ser purificada de forma idêntica.
        let input = Path::new(r"\?\C:\Windows");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"C:\Windows"),
            r"\?\C:\Windows deve ser limpo para C:\Windows");
    }

    #[test]
    fn test_unc_single_backslash_network_is_stripped() {
        let input = Path::new(r"\?\UNC\server\share");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"\\server\share"));
    }

    #[test]
    fn test_unc_extended_prefix_deep_path_is_stripped() {
        let input = Path::new(r"\\?\C:\foo\bar\baz");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"C:\foo\bar\baz"));
    }

    #[test]
    fn test_unc_network_prefix_is_preserved_correctly() {
        // \\?\UNC\server\share -> \\server\share (UNC de rede)
        let input = Path::new(r"\\?\UNC\server\share");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"\\server\share"),
            r"UNC de rede deve perder apenas o prefixo \\?\UNC");
    }

    #[test]
    fn test_normal_windows_path_is_unchanged() {
        // PRD-031 §B: Caminhos normais DEVEM ser mantidos intactos.
        let input = Path::new(r"C:\Windows\System32");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"C:\Windows\System32"),
            "Caminho normal nao deve ser modificado");
    }

    #[test]
    fn test_normal_path_without_drive_is_unchanged() {
        let input = Path::new("foo/bar/baz");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from("foo/bar/baz"),
            "Caminho relativo nao deve ser modificado");
    }

    #[test]
    fn test_forward_slash_unc_variant_is_stripped() {
        let input = Path::new("//?/C:/foo/bar");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from("C:/foo/bar"));
    }

    #[test]
    fn test_already_clean_absolute_path_is_unchanged() {
        let input = Path::new(r"C:\Users\rosas\projects");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"C:\Users\rosas\projects"));
    }

    #[test]
    fn test_root_drive_path_is_unchanged() {
        let input = Path::new(r"C:\");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"C:\"));
    }

    #[test]
    fn test_empty_string_is_unchanged() {
        let input = Path::new("");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(""));
    }

    #[test]
    fn test_unc_prefix_with_spaces_in_path_is_stripped() {
        let input = Path::new(r"\\?\C:\Program Files\tool.exe");
        let result = soda_strip_unc_prefix(input);
        assert_eq!(result, PathBuf::from(r"C:\Program Files\tool.exe"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RED TESTS — soda_clean_path (com canonicalize para paths existentes)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_soda_clean_path_nonexistent_path_strips_unc() {
        // Para paths não-existentes, apenas remove prefixo — sem canonicalize
        let input = Path::new(r"\\?\Z:\soda-ephemeral-test-nonexistent\subdir");
        let result = soda_clean_path(input);
        assert_eq!(result, PathBuf::from(r"Z:\soda-ephemeral-test-nonexistent\subdir"),
            "Path nao-existente deve ter prefixo removido sem canonicalize");
    }

    #[test]
    fn test_soda_clean_path_normal_path_is_returned_as_is() {
        let input = Path::new(r"Z:\non-existent-soda-dir");
        let result = soda_clean_path(input);
        assert_eq!(result, PathBuf::from(r"Z:\non-existent-soda-dir"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_soda_clean_path_existing_temp_dir_has_no_unc() {
        // O diretório temporário real do sistema NUNCA deve retornar com \\?\
        let temp_dir = std::env::temp_dir();
        let result = soda_clean_path(&temp_dir);
        let result_str = result.to_string_lossy();
        assert!(!result_str.starts_with(r"\\?\"),
            "soda_clean_path em diretório existente nao deve conter \\\\?\\: {result_str}");
        assert!(!result_str.starts_with("//?/"),
            "soda_clean_path em diretório existente nao deve conter //?/: {result_str}");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_soda_clean_path_on_std_temp_dir_is_rooted() {
        // O resultado deve ser um path absoluto com letra de drive
        let temp_dir = std::env::temp_dir();
        let result = soda_clean_path(&temp_dir);
        assert!(result.is_absolute(),
            "soda_clean_path deve retornar path absoluto: {}", result.display());
        let s = result.to_string_lossy();
        assert!(s.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false),
            "Path Windows deve comecar com letra de drive: {s}");
    }
}
