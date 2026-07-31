// SOULS-CANIBALIZED: Deduplicação de blocos de 5 linhas consecutivas (Session Cross-File Cache).
// Identifica sequências repetidas de 5 linhas consecutivas entre arquivos da mesma sessão
// e substitui ocorrências por marcadores de deduplicação com apontamento de localização.

use dashmap::DashMap;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Cache de deduplicação global da sessão mantido na RAM do Host.
/// Mapeia hashes de 64 bits para (Caminho, LinhaInicial, LinhaFinal).
pub static SESSION_DEDUP_CACHE: LazyLock<DashMap<u64, (PathBuf, usize, usize)>> =
    LazyLock::new(DashMap::new);

/// Limpa completamente o cache de deduplicação da sessão em RAM.
/// Invoca o descarte físico dos nós para liberar a memória principal do Host.
pub fn clear_session_cache() {
    SESSION_DEDUP_CACHE.clear();
}

pub fn clear_session_dedup_cache() {
    clear_session_cache();
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hash_5_line_block(block: &[&str]) -> u64 {
    let mut hasher = FxHasher::default();
    for line in block {
        let norm = normalize_line(line);
        norm.hash(&mut hasher);
    }
    hasher.finish()
}

/// Executa a deduplicação cross-file com base no cache de sessão.
pub fn deduplicate_blocks_session(text: &str, file_path: Option<&Path>) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 5 {
        return text.to_string();
    }

    let current_path = file_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("buffer"));

    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if i + 5 <= lines.len() {
            let block = &lines[i..i + 5];
            let hash = hash_5_line_block(block);
            let start_line = i + 1;
            let end_line = i + 5;

            if let Some(entry) = SESSION_DEDUP_CACHE.get(&hash) {
                let (other_path, orig_start, orig_end) = entry.value();
                if other_path != &current_path {
                    let path_str = other_path.display().to_string().replace('\\', "/");
                    result.push(format!(
                        "// [dedup: 5 lines hidden. Duplicate of {path_str} lines L{orig_start}-L{orig_end}]"
                    ));
                    i += 5;
                    continue;
                }
            } else {
                SESSION_DEDUP_CACHE.insert(hash, (current_path.clone(), start_line, end_line));
            }
        }
        result.push(lines[i].to_string());
        i += 1;
    }

    let mut out = result.join("\n");
    if text.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Alias para deduplicação sem contexto de arquivo (intra-buffer/sessão).
pub fn deduplicate_blocks(text: &str) -> String {
    deduplicate_blocks_session(text, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_deduplicate_blocks_5_lines() {
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_session_dedup_cache();
        let block = "line1\nline2\nline3\nline4\nline5\n";
        let file1 = Path::new("file1.rs");
        let file2 = Path::new("file2.rs");

        let out1 = deduplicate_blocks_session(block, Some(file1));
        assert_eq!(out1, block);

        let out2 = deduplicate_blocks_session(block, Some(file2));
        assert!(out2.contains("// [dedup: 5 lines hidden. Duplicate of file1.rs lines L1-L5]"));
    }

    #[test]
    fn test_cross_file_deduplication_successful() {
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_session_dedup_cache();
        let block = "fn calculate_hash() {\n    let val = 42;\n    let result = val * 2;\n    println!(\"{}\", result);\n}\n";
        let file1_path = Path::new("src/module_a.rs");
        let file2_path = Path::new("src/module_b.rs");

        // Primeira leitura: registra hashes no cache de sessão
        let out1 = deduplicate_blocks_session(block, Some(file1_path));
        assert_eq!(out1, block);

        // Segunda leitura em arquivo diferente: colisão aciona o marcador com caminho e intervalo de linhas
        let out2 = deduplicate_blocks_session(block, Some(file2_path));
        assert!(
            out2.contains("// [dedup: 5 lines hidden. Duplicate of src/module_a.rs lines L1-L5]"),
            "out2 foi: {out2}"
        );
    }

    #[test]
    fn test_session_cache_clear_successful() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let path = PathBuf::from("src/main.rs");
        SESSION_DEDUP_CACHE.insert(12345, (path.clone(), 1, 5));
        assert!(!SESSION_DEDUP_CACHE.is_empty(), "O cache deveria conter dados simulados.");

        clear_session_cache();

        assert!(SESSION_DEDUP_CACHE.is_empty(), "O cache deveria estar completamente vazio pós-limpeza.");
    }
}
