//! `test_repo_impact.rs` — Marco 4.1.0: 3 contratos rígidos do motor
//! sensorial de Blast Radius.
//!
//! Estes são os testes de integração da Alfândega de Release. Os
//! mesmos contratos estão duplicados como `#[cfg(test)] mod tests`
//! em [`repo_impact.rs`] para TDD Red-Green local. A versão deste
//! arquivo é a **canônica de release** — se houver divergência,
//! esta prevalece.
//!
//! ## Contratos
//!
//! 1. **`test_repo_impact_direct_dependents`**: cadeia A→B→C. Analisar
//!    `C` retorna `B` (nível 1) e `A` (nível 2).
//! 2. **`test_repo_impact_cyclic_protection`**: loop A↔B. `visited`
//!    corta em O(1) e aborta com segurança de ciclo de vida.
//! 3. **`test_repo_impact_respects_max_depth`**: cadeia A→B→C→D.
//!    `max_depth=1` poda dependentes nível 2+.
//!
//! ## Isolamento
//!
//! Usa `tempfile::TempDir` para criar monorepos sintéticos
//! descartáveis. Zero I/O no workspace real do SOULS.

use souls_mc_lib::cognition::lean_vacuum::repo_impact_fn;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper canônico: escreve arquivo em diretório temporário.
fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(&p, content).expect("write file");
    p
}

// ──────────────────────────────────────────────────────────────────────
// CONTRATO 1 — Cadeia direta de dependentes
// ──────────────────────────────────────────────────────────────────────

/// Cadeia: A → B → C (A importa B; B importa C; C é leaf).
///
/// Esperado: analisar `C` retorna `B` no nível 1 e `A` no nível 2.
#[test]
fn test_repo_impact_direct_dependents() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(root, "A.rs", "use crate::B;\n");
    write(root, "B.rs", "use crate::C;\n");
    write(root, "C.rs", "// leaf module\n");

    let report = repo_impact_fn(root, Path::new("C.rs"), 3).expect("impact ok");

    // Alvo canônico
    assert_eq!(report.target_file, "C.rs", "target_file canônico");

    // Total de impactados
    assert_eq!(
        report.total_impacted_files, 2,
        "B + A = 2 impactados (got: {:?})",
        report.impact_graph.nodes
    );

    // Profundidade máxima atingida
    assert_eq!(
        report.max_depth_reached, 2,
        "BFS reverso deve alcançar nível 2 (C←B←A)"
    );

    // Nós: B (nível 1) e A (nível 2)
    let nodes: BTreeSet<String> = report.impact_graph.nodes.iter().cloned().collect();
    assert!(
        nodes.contains("B.rs"),
        "B deve aparecer (nível 1): {nodes:?}"
    );
    assert!(
        nodes.contains("A.rs"),
        "A deve aparecer (nível 2): {nodes:?}"
    );

    // Arestas: A→B (A importa B) e B→C (B importa C)
    let edge_set: BTreeSet<(String, String)> = report
        .impact_graph
        .edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    assert!(
        edge_set.contains(&("A.rs".to_string(), "B.rs".to_string())),
        "aresta A→B presente: {edge_set:?}"
    );
    assert!(
        edge_set.contains(&("B.rs".to_string(), "C.rs".to_string())),
        "aresta B→C presente: {edge_set:?}"
    );
}

// ──────────────────────────────────────────────────────────────────────
// CONTRATO 2 — Proteção contra ciclos (O(1) cycle cut)
// ──────────────────────────────────────────────────────────────────────

/// Loop: A ↔ B (A importa B; B importa A).
///
/// Esperado: BFS corta o ciclo em O(1) via `visited: HashSet<PathBuf>`,
/// `B` aparece exatamente 1× no grafo, sem Stack Overflow.
#[test]
fn test_repo_impact_cyclic_protection() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(root, "A.rs", "use crate::B;\n");
    write(root, "B.rs", "use crate::A;\n");

    // max_depth=10 explicitamente para expor qualquer tentativa de
    // expansão infinita — o visited set deve cortar antes.
    let report = repo_impact_fn(root, Path::new("A.rs"), 10).expect("impact ok");

    // Target
    assert_eq!(report.target_file, "A.rs");

    // B aparece EXATAMENTE 1× (dedup de ciclo)
    let b_count = report
        .impact_graph
        .nodes
        .iter()
        .filter(|n| n.as_str() == "B.rs")
        .count();
    assert_eq!(
        b_count, 1,
        "B deve aparecer exatamente 1× (cycle cut): {:?}",
        report.impact_graph.nodes
    );

    // Profundidade cortada em 1 (não expande infinitamente)
    assert!(
        report.max_depth_reached <= 1,
        "BFS não pode expandir além de 1 em ciclo A↔B: depth={}",
        report.max_depth_reached
    );

    // Sanity: aresta B→A presente (B importa A — semântica do BFS
    // reverso: aresta (from=importador, to=importado) onde o importado
    // foi alcançado via BFS).
    let edge_set: BTreeSet<(String, String)> = report
        .impact_graph
        .edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    assert!(
        edge_set.contains(&("B.rs".to_string(), "A.rs".to_string())),
        "aresta B→A presente (B importa A): {edge_set:?}"
    );
    // A aresta A→B (A importa B) NÃO é capturada porque o BFS
    // reverso parte de A (target) e só segue os importadores de A.
    // Esta é a semântica intencional do grafo transposto.
    assert!(
        !edge_set.contains(&("A.rs".to_string(), "B.rs".to_string())),
        "aresta A→B não é capturada pelo BFS reverso (semântica intencional): {edge_set:?}"
    );
}

// ──────────────────────────────────────────────────────────────────────
// CONTRATO 3 — Respeito ao `max_depth`
// ──────────────────────────────────────────────────────────────────────

/// Cadeia: A → B → C → D. `max_depth=1` deve podar B e A.
#[test]
fn test_repo_impact_respects_max_depth() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(root, "A.rs", "use crate::B;\n");
    write(root, "B.rs", "use crate::C;\n");
    write(root, "C.rs", "use crate::D;\n");
    write(root, "D.rs", "// leaf\n");

    let report = repo_impact_fn(root, Path::new("D.rs"), 1).expect("impact ok");

    // max_depth_reached deve ser 1 (C imediato)
    assert_eq!(
        report.max_depth_reached, 1,
        "max_depth_reached deve ser 1 (apenas C é vizinho direto de D)"
    );

    // C presente (nível 1)
    let nodes: BTreeSet<String> = report.impact_graph.nodes.iter().cloned().collect();
    assert!(
        nodes.contains("C.rs"),
        "C presente (nível 1): {nodes:?}"
    );

    // B e A PODADOS (nível 2 e 3)
    assert!(
        !nodes.contains("B.rs"),
        "B deve ser PODADO (nível 2): {nodes:?}"
    );
    assert!(
        !nodes.contains("A.rs"),
        "A deve ser PODADO (nível 3): {nodes:?}"
    );

    // total_impacted_files = 1 (apenas C)
    assert_eq!(
        report.total_impacted_files, 1,
        "total deve ser 1 (apenas C): {:?}",
        report.impact_graph.nodes
    );
}
