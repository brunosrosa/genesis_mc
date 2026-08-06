// SOULS-CANIBALIZED: Busca Textual Compacta no Padrão LEAN (souls_search)
// Executa varredura por regex com notação compacta agrupada por arquivo e aglomeração de linhas.

use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use walkdir::WalkDir;

const TOXIC_DIR_NAMES: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".souls_cache",
    ".souls_data",
    ".cargo",
    ".vscode",
    ".idea",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_text: String,
}

/// Executa a busca textual recursiva e retorna o relatório na Notação LEAN.
pub fn search_lean(
    root_path: &Path,
    pattern: &str,
    max_depth: usize,
) -> Result<String, String> {
    let re = Regex::new(pattern).map_err(|e| format!("Regex inválida '{pattern}': {e}"))?;

    let mut matches_by_file: BTreeMap<String, Vec<SearchMatch>> = BTreeMap::new();

    for entry in WalkDir::new(root_path)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !TOXIC_DIR_NAMES.contains(&name);
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut file_matches = Vec::new();
            for (line_idx, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    file_matches.push(SearchMatch {
                        line_number: line_idx + 1,
                        line_text: line.trim().to_string(),
                    });
                }
            }

            if !file_matches.is_empty() {
                let rel_path = path
                    .strip_prefix(root_path)
                    .unwrap_or(path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                matches_by_file.insert(rel_path, file_matches);
            }
        }
    }

    if matches_by_file.is_empty() {
        return Ok(format!("Nenhum resultado encontrado para a busca '{pattern}'."));
    }

    Ok(format_lean_notation(&matches_by_file))
}

/// Formata o mapa de correspondências na Notação Compacta LEAN.
pub fn format_lean_notation(matches_by_file: &BTreeMap<String, Vec<SearchMatch>>) -> String {
    let mut out = Vec::new();

    for (file_path, file_matches) in matches_by_file {
        out.push(format!("@{file_path}"));

        // Agrupa por conteúdo textual idêntico para achatar linhas consecutivas/repetidas
        let mut grouped: Vec<(Vec<usize>, String)> = Vec::new();
        for m in file_matches {
            if let Some(last) = grouped.last_mut() {
                if last.1 == m.line_text {
                    last.0.push(m.line_number);
                    continue;
                }
            }
            grouped.push((vec![m.line_number], m.line_text.clone()));
        }

        for (lines, text) in grouped {
            let line_refs = lines
                .iter()
                .map(|l| format!("L{l}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(format!("{line_refs}: {text}"));
        }
    }

    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_lean_notation_formatting() {
        let mut matches = BTreeMap::new();
        matches.insert(
            "src-tauri/src/bin/souls_mcp_server.rs".to_string(),
            vec![
                SearchMatch {
                    line_number: 42,
                    line_text: "fn run_inference()".to_string(),
                },
                SearchMatch {
                    line_number: 112,
                    line_text: "let gguf_meta = parse_gguf_metadata_zero_copy(model_path);".to_string(),
                },
                SearchMatch {
                    line_number: 132,
                    line_text: "let gguf_meta = parse_gguf_metadata_zero_copy(model_path);".to_string(),
                },
            ],
        );

        let output = format_lean_notation(&matches);

        assert!(output.contains("@src-tauri/src/bin/souls_mcp_server.rs"));
        assert!(output.contains("L42: fn run_inference()"));
        assert!(output.contains("L112, L132: let gguf_meta = parse_gguf_metadata_zero_copy(model_path);"));
    }
}
