//! `repo_impact.rs` — Motor Sensorial de Blast Radius (Marco 4.1.0).
//!
//! Canibalização do stub legado `cognition::observability::impact`
//! (apenas Rust regex) para o cânone `lean_vacuum` (multilíngue,
//! 22 extensões canônicas, 22 exclusões).
//!
//! ## Algoritmo
//!
//! 1. **WalkDir filtrado** — `is_excluded_dir` (22) + `is_source_ext` (22).
//! 2. **ImportExtractor multilíngue** — regex canônica captura:
//!    - Rust: `use crate::...`, `use super::...`, `mod foo;`
//!    - TS/JS: `import ... from "..."`, `require("...")`
//!    - Python: `from .x import y`, `import x`, `from x import y`
//!    - Go: `import "..."` (bloco)
//!    - C/C++: `#include "..."`
//! 3. **`ImportGraph` forward** — `BTreeMap<PathBuf, Vec<PathBuf>>`.
//! 4. **Transpor** — inverte arestas (B → [A, C] se A importa B e C importa B).
//! 5. **BFS reverso** — `VecDeque<(PathBuf, u8)>` partindo do `target`,
//!    `HashSet<PathBuf>` visited corta ciclos em O(1).
//! 6. **ImpactReport** — payload canônico MCP-serializável.
//!
//! ## Performance
//!
//! - Heap: O(V + E) onde V = arquivos, E = arestas.
//! - WalkDir é o gargalo; filtrado por `filter_entry` (poda subárvore).
//! - Regex compilada 1× via `OnceLock` (canônica ADR-009).
//!
//! ## Agnosticismo Hardware
//!
//! 100% CPU + std. Zero CUDA/Python/Node. RTX 2060m intocada.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use thiserror::Error;
use walkdir::WalkDir;

use crate::cognition::context::extensions::{is_excluded_dir, is_source_ext};

/// Profundidade padrão do BFS reverso (Cliff de Segurança).
pub const DEFAULT_MAX_DEPTH: u8 = 3;

/// Teto rígido de profundidade (anti-DoS em grafos cíclicos).
pub const MAX_DEPTH_CEILING: u8 = 10;

/// Teto rígido de arquivos varridos (anti-OOM em monorepos).
pub const MAX_FILES_SCAN: usize = 50_000;

/// Regex canônica multilíngue para detecção de imports locais.
///
/// Captura (em ordem de preferência) **o identificador de path**
/// do módulo importado. Casos cobertos (5 grupos):
/// - **Grupo 1**: Rust `use crate::a::b` ou `mod foo;`
/// - **Grupo 2**: TS/JS `import x from "./y"` ou `require("./y")` (com aspas)
/// - **Grupo 3**: Python `from .x import y` ou `from x.y import z` (sem aspas)
/// - **Grupo 4**: C/C++ `#include "header.h"`
fn import_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r#"(?x)
            (?:
                \b(?:use|mod)\s+(?:crate|super|self)::([a-zA-Z0-9_]+
                    (?:(?:::)[a-zA-Z0-9_]+)*)
                |
                \b(?:import|require)\s*\(?\s*["']([^"']+)["']
                |
                \bfrom\s+([\.a-zA-Z0-9_][\.a-zA-Z0-9_]*)\s+import\b
                |
                \#\s*include\s*["<]([^">]+)[">]
            )
            "#,
        )
        .expect("regex multilíngue canônica é estática e válida")
    })
}

/// Mapa canônico forward: `importer -> [importee, ...]`.
pub type ImportGraph = BTreeMap<PathBuf, BTreeSet<PathBuf>>;

/// Aresta direcionada: `from` importa `to`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactEdge {
    pub from: String,
    pub to: String,
}

/// Grafo de impacto: nós únicos + arestas deduplicadas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactGraphPayload {
    pub nodes: Vec<String>,
    pub edges: Vec<ImpactEdge>,
}

/// Relatório canônico de Blast Radius (payload MCP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactReport {
    pub target_file: String,
    pub total_impacted_files: usize,
    pub max_depth_reached: u8,
    pub impact_graph: ImpactGraphPayload,
}

/// Erros canônicos do motor sensorial.
#[derive(Debug, Error)]
pub enum RepoImpactError {
    #[error("caminho inválido: {0}")]
    InvalidPath(String),
    #[error("arquivo-alvo não encontrado: {0}")]
    TargetNotFound(String),
    #[error("falha de I/O ao varrer repositório: {0}")]
    Io(String),
}

/// Calcula o Blast Radius de `target` no monorepo `root`.
///
/// # Argumentos
/// - `root`: diretório raiz do monorepo (será varrido).
/// - `target`: arquivo-alvo (relativo a `root` ou absoluto).
/// - `max_depth`: profundidade máxima do BFS reverso (clamp 1..=10).
///
/// # Retorno
/// `ImpactReport` com nós/árestas deduplicados e `max_depth_reached`
/// igual à maior profundidade efetivamente alcançada.
pub fn repo_impact(root: &Path, target: &Path, max_depth: u8) -> Result<ImpactReport, RepoImpactError> {
    if !root.is_dir() {
        return Err(RepoImpactError::InvalidPath(format!(
            "root não é diretório: {}",
            root.display()
        )));
    }
    let depth = max_depth.clamp(1, MAX_DEPTH_CEILING);

    // Resolve `target` relativamente a `root` se for relativo.
    let target_abs = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    let target_canon = target_abs
        .strip_prefix(root)
        .map_err(|_| RepoImpactError::InvalidPath(format!(
            "target fora do root: {}",
            target_abs.display()
        )))?
        .to_path_buf();

    if !target_abs.exists() {
        return Err(RepoImpactError::TargetNotFound(
            target_canon.to_string_lossy().into_owned(),
        ));
    }

    // ── Fase 1: WalkDir filtrado ────────────────────────────────────
    let graph = build_import_graph(root)?;

    // ── Fase 2: Transpor arestas ────────────────────────────────────
    let transposed = transpose_graph(&graph);

    // ── Fase 3: BFS reverso com visited set ─────────────────────────
    let (nodes, edges, max_depth_reached) =
        bfs_reverse(&transposed, &target_canon, depth);

    Ok(ImpactReport {
        target_file: target_canon.to_string_lossy().replace('\\', "/"),
        total_impacted_files: nodes.len(),
        max_depth_reached,
        impact_graph: ImpactGraphPayload { nodes, edges },
    })
}

/// Executa a travessia BFS de impacto exclusivamente em RAM Host (< 3ms) via DashMap do CALL_GRAPH / SYMBOL_INDEX.
pub fn repo_impact_from_ram(target: &str, max_depth: u8) -> ImpactReport {
    let depth = max_depth.clamp(1, MAX_DEPTH_CEILING);
    let graph = crate::cognition::ast::observability::call_graph_global();

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, u8)> = VecDeque::new();
    let mut edges: Vec<ImpactEdge> = Vec::new();
    let mut max_depth_reached: u8 = 0;

    visited.insert(target.to_string());
    queue.push_back((target.to_string(), 0));

    while let Some((current, d)) = queue.pop_front() {
        if d >= depth {
            continue;
        }
        if let Some(node) = graph.get(&current) {
            for caller in &node.callers {
                edges.push(ImpactEdge {
                    from: caller.clone(),
                    to: current.clone(),
                });
                if visited.insert(caller.clone()) {
                    let next_depth = d + 1;
                    if next_depth > max_depth_reached {
                        max_depth_reached = next_depth;
                    }
                    queue.push_back((caller.clone(), next_depth));
                }
            }
        }
    }

    let mut nodes: Vec<String> = visited.into_iter().filter(|n| n != target).collect();
    nodes.sort();
    edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    edges.dedup();

    ImpactReport {
        target_file: target.replace('\\', "/"),
        total_impacted_files: nodes.len(),
        max_depth_reached,
        impact_graph: ImpactGraphPayload { nodes, edges },
    }
}

/// Varre `root` filtrando diretórios tóxicos e extensões não-canônicas,
/// extraindo declarações de import multilíngue.
fn build_import_graph(root: &Path) -> Result<ImportGraph, RepoImpactError> {
    let mut graph: ImportGraph = BTreeMap::new();
    let re = import_regex();
    let mut file_count = 0usize;

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Poda subárvore tóxica: se é diretório e está em EXCLUDE_DIRS,
            // rejeita entry (WalkDir pula toda a subárvore).
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !is_excluded_dir(name);
                }
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // I/O resiliente: pula entry quebrado.
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();

        // Filtro de extensão (22 canônicas).
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !is_source_ext(ext) {
            continue;
        }

        // Hard cap anti-OOM.
        file_count += 1;
        if file_count > MAX_FILES_SCAN {
            return Err(RepoImpactError::Io(format!(
                "monorepo excede {MAX_FILES_SCAN} arquivos (anti-OOM)"
            )));
        }

        // Lê conteúdo (best-effort: pula arquivo ilegível).
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Path canônico (relativo ao root, com `/`).
        let canonical = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_path_buf();

        let mut importees: BTreeSet<PathBuf> = BTreeSet::new();
        for cap in re.captures_iter(&content) {
            // Grupo 1: Rust `crate::path`     → normalizar para path
            // Grupo 2: aspas literais        → "./x" ou "x/y" (TS/JS/Python com aspas)
            // Grupo 3: Python `from x import` → path sem aspas
            // Grupo 4: include "x.h"         → "x.h"
            let raw = cap
                .get(1)
                .or_else(|| cap.get(2))
                .or_else(|| cap.get(3))
                .or_else(|| cap.get(4))
                .map(|m| m.as_str().trim().to_string());

            if let Some(r) = raw {
                if let Some(resolved) = resolve_import(&r, &canonical, root, ext) {
                    importees.insert(resolved);
                }
            }
        }

        if !importees.is_empty() {
            graph.insert(canonical, importees);
        }
    }

    Ok(graph)
}

/// Resolve um literal de import para um `PathBuf` relativo ao `root`.
///
/// Estratégia baseada em extensão do **importer**:
/// - `.rs` → Rust (`crate::a::b` normalizado para `a/b.rs`).
/// - `.ts/.tsx/.js/.jsx` → relativo (`./x` ou `../x`).
/// - `.py` → `from .x import y` resolve para `./x.py`.
/// - `.h/.c/.cpp` → `#include "x.h"`.
fn resolve_import(
    raw: &str,
    importer: &Path,
    root: &Path,
    ext: &str,
) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // ── Rust: normalizar `crate::a::b` ou `super::a` para path ──
    if ext == "rs" {
        let normalized = trimmed
            .trim_start_matches("crate::")
            .trim_start_matches("super::")
            .trim_start_matches("self::")
            .replace("::", "/");
        for candidate in [
            format!("{normalized}.rs"),
            format!("{normalized}/mod.rs"),
        ] {
            let p = root.join(&candidate);
            if p.exists() {
                return Some(strip_root(&p, root));
            }
        }
        // Nenhum candidato existe no disco: pulamos para evitar
        // poluir o grafo com paths fantasmas. Anti-FALSO-VERDE.
        return None;
    }

    // ── TS/JS: relativo `./x` ou `../x` ─────────────────────────
    if matches!(ext, "ts" | "tsx" | "js" | "jsx") {
        if trimmed.starts_with("./") || trimmed.starts_with("../") {
            let parent = importer.parent()?;
            let joined = parent.join(trimmed);
            return Some(strip_root(&joined, root));
        }
        return None; // bare module (npm) — ignorar
    }

    // ── Python: `from .x import y` ou `import x` ────────────────
    if ext == "py" {
        if trimmed.starts_with('.') {
            let parent = importer.parent()?;
            let joined = parent.join(trimmed.trim_start_matches('.'));
            let with_ext = joined.with_extension("py");
            if with_ext.exists() {
                return Some(strip_root(&with_ext, root));
            }
            return Some(strip_root(&joined, root));
        }
        let direct = root.join(format!("{trimmed}.py"));
        if direct.exists() {
            return Some(strip_root(&direct, root));
        }
        return None;
    }

    // ── C/C++: `#include "header.h"` ───────────────────────────
    if matches!(ext, "h" | "c" | "cpp") {
        let direct = root.join(trimmed);
        if direct.exists() {
            return Some(strip_root(&direct, root));
        }
        return Some(PathBuf::from(trimmed));
    }

    None
}

fn strip_root(p: &Path, root: &Path) -> PathBuf {
    p.strip_prefix(root).unwrap_or(p).to_path_buf()
}

/// Inverte arestas: `A importa B` vira `B → [A]`.
fn transpose_graph(graph: &ImportGraph) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    let mut transposed: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    for (importer, importees) in graph {
        for importee in importees {
            transposed
                .entry(importee.clone())
                .or_default()
                .insert(importer.clone());
        }
    }
    transposed
}

/// BFS reverso partindo de `target` no grafo transposto.
///
/// Retorna `(nodes, edges, max_depth_reached)`:
/// - `nodes`: paths únicos (excluindo o próprio `target`) atingidos.
/// - `edges`: arestas `(from=importador, to=importado)` que
///   participaram do BFS (deduplicadas, determinísticas).
/// - `max_depth_reached`: maior `depth` efetivamente visitada.
fn bfs_reverse(
    transposed: &BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    target: &Path,
    max_depth: u8,
) -> (Vec<String>, Vec<ImpactEdge>, u8) {
    let mut visited: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut queue: VecDeque<(PathBuf, u8)> = VecDeque::new();
    queue.push_back((target.to_path_buf(), 0));
    visited.insert(target.to_path_buf());

    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    let mut max_depth_reached: u8 = 0;

    while let Some((node, depth)) = queue.pop_front() {
        if depth > max_depth_reached {
            max_depth_reached = depth;
        }
        if let Some(neighbors) = transposed.get(&node) {
            for n in neighbors {
                if visited.insert(n.clone()) {
                    // Aresta: n (importador) → node (importado)
                    let from = path_to_string(n);
                    let to = path_to_string(&node);
                    edges.insert((from.clone(), to.clone()));
                    if depth < max_depth {
                        queue.push_back((n.clone(), depth + 1));
                        nodes.insert(from);
                    }
                }
            }
        }
    }

    let nodes_vec: Vec<String> = nodes.into_iter().collect();
    let edges_vec: Vec<ImpactEdge> = edges
        .into_iter()
        .map(|(from, to)| ImpactEdge { from, to })
        .collect();

    (nodes_vec, edges_vec, max_depth_reached)
}

fn path_to_string(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn unit_import_regex_captures_rust_crate() {
        let re = import_regex();
        let caps = re.captures("use crate::cognition::lean_vacuum::repo_impact;").unwrap();
        assert_eq!(
            caps.get(1).unwrap().as_str(),
            "cognition::lean_vacuum::repo_impact"
        );
    }

    #[test]
    fn unit_import_regex_captures_js_require() {
        let re = import_regex();
        let caps = re.captures(r#"const x = require("./foo");"#).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str(), "./foo");
    }

    #[test]
    fn unit_import_regex_captures_python_from() {
        let re = import_regex();
        let caps = re.captures("from .mod import util").unwrap();
        // Python `from X import` cai no grupo 3 (sem aspas).
        assert_eq!(caps.get(3).unwrap().as_str(), ".mod");
    }

    #[test]
    fn unit_transpose_graph_inverts_edges() {
        let mut g: ImportGraph = BTreeMap::new();
        let a: PathBuf = "a.rs".into();
        let b: PathBuf = "b.rs".into();
        let c: PathBuf = "c.rs".into();
        g.insert(a.clone(), {
            let mut s = BTreeSet::new();
            s.insert(b.clone());
            s
        });
        g.insert(c.clone(), {
            let mut s = BTreeSet::new();
            s.insert(b.clone());
            s
        });
        let t = transpose_graph(&g);
        assert!(t.get(&b).unwrap().contains(&a));
        assert!(t.get(&b).unwrap().contains(&c));
    }

    #[test]
    fn unit_bfs_reverse_cuts_cycle_in_o1() {
        // Grafo transposto: a → b, b → a (ciclo).
        let mut t: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
        let a: PathBuf = "a.rs".into();
        let b: PathBuf = "b.rs".into();
        t.insert(a.clone(), {
            let mut s = BTreeSet::new();
            s.insert(b.clone());
            s
        });
        t.insert(b.clone(), {
            let mut s = BTreeSet::new();
            s.insert(a.clone());
            s
        });
        let (nodes, _edges, depth) = bfs_reverse(&t, &a, 10);
        assert_eq!(nodes.len(), 1, "B aparece exatamente 1× (dedup ciclo)");
        assert_eq!(depth, 1, "ciclo cortado em profundidade 1");
    }

    // ── Testes de Integração (TASK-01/02/03 do Marco 4.1.0) ──────

    #[test]
    fn integration_direct_dependents_chain() {
        // A → B → C (A importa B, B importa C)
        // Esperado: analisar C retorna B (nível 1) e A (nível 2).
        let dir = TempDir::new().unwrap();
        let r = dir.path();
        write(r, "A.rs", "use crate::B;");
        write(r, "B.rs", "use crate::C;");
        write(r, "C.rs", "// leaf\n");

        let report = repo_impact(r, Path::new("C.rs"), 3).unwrap();

        assert_eq!(report.target_file, "C.rs");
        assert_eq!(report.total_impacted_files, 2);
        assert_eq!(report.max_depth_reached, 2);
        let nodes: BTreeSet<String> = report.impact_graph.nodes.iter().cloned().collect();
        assert!(nodes.contains("B.rs"), "B deve aparecer (nível 1): {nodes:?}");
        assert!(nodes.contains("A.rs"), "A deve aparecer (nível 2): {nodes:?}");
    }

    #[test]
    fn integration_cyclic_protection_aborts_safely() {
        // A ↔ B (A importa B, B importa A)
        let dir = TempDir::new().unwrap();
        let r = dir.path();
        write(r, "A.rs", "use crate::B;");
        write(r, "B.rs", "use crate::A;");

        let report = repo_impact(r, Path::new("A.rs"), 10).unwrap();

        assert_eq!(report.target_file, "A.rs");
        assert_eq!(
            report.impact_graph.nodes.len(),
            1,
            "B aparece exatamente 1× (dedup ciclo): {:?}",
            report.impact_graph.nodes
        );
        assert!(report.impact_graph.nodes.contains(&"B.rs".to_string()));
        assert!(
            report.max_depth_reached <= 1,
            "BFS corta ciclo em profundidade 1: depth={}",
            report.max_depth_reached
        );
    }

    #[test]
    fn integration_respects_max_depth_one() {
        // A → B → C → D. max_depth=1 → só C aparece.
        let dir = TempDir::new().unwrap();
        let r = dir.path();
        write(r, "A.rs", "use crate::B;");
        write(r, "B.rs", "use crate::C;");
        write(r, "C.rs", "use crate::D;");
        write(r, "D.rs", "// leaf\n");

        let report = repo_impact(r, Path::new("D.rs"), 1).unwrap();

        assert_eq!(report.max_depth_reached, 1);
        let nodes: BTreeSet<String> = report.impact_graph.nodes.iter().cloned().collect();
        assert!(nodes.contains("C.rs"), "C presente (nível 1): {nodes:?}");
        assert!(!nodes.contains("B.rs"), "B deve ser PODADO: {nodes:?}");
        assert!(!nodes.contains("A.rs"), "A deve ser PODADO: {nodes:?}");
    }
}
