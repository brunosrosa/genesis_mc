//! Impact: Blast Radius (BFS no grafo transposto de imports).
//!
//! O SODA trata o monorepo como um **DAG de imports** em RAM. Para cada
//! arquivo-alvo, a funcao [`blast_radius`] descobre recursivamente todos
//! os arquivos que o importam (direta ou transitivamente).
//!
//! O grafo e' armazenado como `BTreeMap<String, Vec<String>>`:
//! - chave = path do arquivo
//! - valor = lista de paths que ele importa
//!
//! O BFS opera no **grafo transposto** (invertendo o sentido das arestas),
//! partindo do target e seguindo os importadores.
//!
//! Complexidade: O(V + E) onde V = arquivos, E = arestas de import.
//! Memoria: O(V + E) em RAM.
//!
//! Zero crates externos: apenas `std::collections::{BTreeMap, VecDeque}`.

use crate::cognition::memory_graph::errors::CognitiveError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Regex canonica para detectar imports Rust locais.
///
/// Cobre: `use crate::...`, `use super::...`, `use self::...`,
/// `mod foo;`, `mod foo { ... }`. Nao cobre `use std::...` ou
/// `use serde::...` (crates externos — nao sao arquivos do monorepo).
fn rust_import_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Captura o path do modulo a partir de `use ...::path::to::item;`
        // ou `mod path::to::mod;`.
        Regex::new(
            r"(?m)^(?:\s*)(?:use\s+(?:crate|super|self)::([a-zA-Z0-9_:{}+]*)|mod\s+([a-zA-Z0-9_]+))",
        )
        .expect("regex de imports Rust valida")
    })
}

/// Mapa canonico de imports: `path -> Vec<path_importado>`.
pub type ImportGraph = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpactReport {
    /// Arquivo-alvo da analise.
    pub target: String,
    /// Lista ordenada de arquivos afetados (ordem de descoberta BFS).
    pub affected: Vec<String>,
    /// Profundidade maxima alcancada (`0` se nenhum importador direto).
    pub depth: usize,
}

/// Constroi o grafo de imports do monorepo a partir de um diretorio raiz.
///
/// Varre recursivamente todos os arquivos `.rs` (com excecao de `target/`,
/// `.souls_cache/`, `vendor/`, `third_party/`) e extrai as declaracoes
/// `use`/`mod` que referenciam paths locais do monorepo.
///
/// Retorna `Err(CognitiveError::GraphError)` em caso de I/O falha.
pub fn build_import_graph(root: &Path) -> Result<ImportGraph, CognitiveError> {
    let mut graph: ImportGraph = BTreeMap::new();
    let import_re = rust_import_regex();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Poda cirurgica: banir target, vendor, cache, .git, etc.
            !(name == "target"
                || name == "vendor"
                || name == "third_party"
                || name == ".git"
                || name == ".souls_cache"
                || name == ".souls_sandbox"
                || name == ".souls_data"
                || name == "node_modules"
                || name == "dist")
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let canonical = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let mut imported: Vec<String> = Vec::new();
        for cap in import_re.captures_iter(&content) {
            let raw = cap
                .get(1)
                .or_else(|| cap.get(2))
                .map(|m| m.as_str().to_string());
            if let Some(r) = raw {
                // Normaliza: `crate::cognition::lean_vacuum` -> path
                let normalized = r
                    .trim_start_matches("crate::")
                    .replace("::", "/")
                    .trim_end_matches('/')
                    .to_string();
                if !normalized.is_empty() && !normalized.starts_with("super::")
                    && !normalized.starts_with("self::")
                {
                    // Sufixo `.rs` para casar com chaves do grafo.
                    let candidate = if normalized.ends_with(".rs") {
                        normalized
                    } else {
                        format!("{normalized}.rs")
                    };
                    imported.push(candidate);
                }
            }
        }
        graph.insert(canonical, imported);
    }

    Ok(graph)
}

/// Calcula o Blast Radius via BFS no **grafo transposto**.
///
/// Retorna a lista ordenada de arquivos que dependem (direta ou
/// transitivamente) de `target`. A ordem reflete a profundidade de
/// descoberta BFS (importadores diretos primeiro).
///
/// Algoritmo:
/// 1. Constroi o grafo transposto (inverte arestas).
/// 2. BFS a partir de `target`.
/// 3. Deduplica via `BTreeSet` e converte para `Vec` ordenada.
pub fn blast_radius(graph: &ImportGraph, target: &str) -> Vec<String> {
    // Constroi grafo transposto: para cada (A imports B), cria (B -> A).
    let mut transposed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (importer, importees) in graph {
        for importee in importees {
            transposed
                .entry(importee.clone())
                .or_default()
                .push(importer.clone());
        }
    }
    // BFS a partir do target.
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((target.to_string(), 0));
    let mut result: Vec<(String, usize)> = Vec::new();
    let mut max_depth = 0usize;
    while let Some((node, depth)) = queue.pop_front() {
        if !visited.insert(node.clone()) {
            continue;
        }
        if node != target {
            result.push((node.clone(), depth));
            if depth > max_depth {
                max_depth = depth;
            }
        }
        if let Some(neighbors) = transposed.get(&node) {
            for n in neighbors {
                if !visited.contains(n) {
                    queue.push_back((n.clone(), depth + 1));
                }
            }
        }
    }
    // Ordena por (profundidade, path) para saida deterministica.
    result.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    result.into_iter().map(|(p, _)| p).collect()
}

/// Constrói o `ImpactReport` completo (helper de alto nivel).
pub fn impact_report(graph: &ImportGraph, target: &str) -> ImpactReport {
    let affected = blast_radius(graph, target);
    // Profundidade maxima via BFS.
    let mut transposed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (importer, importees) in graph {
        for importee in importees {
            transposed
                .entry(importee.clone())
                .or_default()
                .push(importer.clone());
        }
    }
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((target.to_string(), 0));
    let mut max_depth = 0usize;
    while let Some((node, depth)) = queue.pop_front() {
        if !visited.insert(node.clone()) {
            continue;
        }
        if depth > max_depth {
            max_depth = depth;
        }
        if let Some(neighbors) = transposed.get(&node) {
            for n in neighbors {
                if !visited.contains(n) {
                    queue.push_back((n.clone(), depth + 1));
                }
            }
        }
    }
    ImpactReport {
        target: target.to_string(),
        affected,
        depth: max_depth,
    }
}

/// Helper: resolve um path de arquivo para o formato canonico do grafo
/// (relativo ao root, com `/` e sufixo `.rs`).
pub fn canonicalize_path(root: &Path, file: &Path) -> String {
    let stripped = file.strip_prefix(root).unwrap_or(file);
    stripped.to_string_lossy().replace('\\', "/")
}

/// Helper de teste interno (exposto para tests via `#[cfg(test)]`).
#[doc(hidden)]
pub fn _build_path_buf(s: &str) -> PathBuf {
    PathBuf::from(s)
}
