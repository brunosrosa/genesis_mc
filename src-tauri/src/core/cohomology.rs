//! `cohomology.rs` — Marco 4.10.0 ETAPA 2: Cohomologia de Feixes Socráticos.
//!
//! **DIRETRIZ 1 do Arquiteto-Chefe (inegociável):**
//! - A função `first_betti_number()` NÃO é contagem de ciclos espacial cega
//!   (m - n + c). Modela cohomologia via **rank do operador d0** sobre a
//!   matriz de restrições de consistência factual extraídas do SQLite.
//! - Gauss-Elimination parcial para rank, **puro CPU em stack** (zero heap).
//! - H¹ ≠ 0 apenas se houver **contradição de restrições de verdade**, ignorando
//!   ciclos harmoniosos.
//!
//! **Stack-safety:** O ceiling é 1024 entradas, mas v1 usa 256 para garantir
//! stack-safety no Windows (default main thread stack = 1MB; matriz 1024×1025
//! floats = 4MB causaria overflow). v2 pode aumentar para 1024 com
//! `std::thread::Builder::stack_size(8 * 1024 * 1024)`.

use std::ops::Range;

use rusqlite::Connection;

use crate::core::epistemic_prober::EpistemicScores;

/// Limite físico de fatos (vértices do grafo de conhecimento).
/// Stack-safe em main thread Windows (256 × 257 × 4 = 256KB).
const MAX_FACTS: usize = 256;
/// Limite físico de relações (restrições). Igual a MAX_FACTS.
const MAX_RELATIONS: usize = 256;

/// Tipos de relação canônicos. Cada um codifica uma restrição linear:
/// - `DependsOn`: x_u = x_v (x_v - x_u = 0)
/// - `Implies`: x_u → x_v ≡ x_v ≥ x_u (modelado como x_v - x_u ≥ 0,
///   entra como linha homogênea: -x_u + x_v = 0)
/// - `ConflictsWith`: x_u ⊕ x_v = 1 (x_u + x_v = 1)
/// - `EquivalentTo`: x_u = x_v (x_v - x_u = 0)
/// - `Negates`: x_u = ¬x_v (x_u + x_v = 1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    DependsOn,
    Implies,
    ConflictsWith,
    EquivalentTo,
    Negates,
}

impl RelationKind {
    /// Decodifica o `relation_type` textual do SQLite.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "depends_on" | "depends" => Self::DependsOn,
            "implies" | "implica" => Self::Implies,
            "conflicts_with" | "conflicts" => Self::ConflictsWith,
            "equivalent_to" | "equivalent" | "equals" => Self::EquivalentTo,
            "negates" | "contradicts" | "not" => Self::Negates,
            _ => Self::DependsOn, // fallback conservador
        }
    }
}

impl std::str::FromStr for RelationKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // `parse` é infallible (fallback conservador); FromStr exige Result,
        // mas a semântica de "sempre sucesso" é intencional.
        Ok(Self::parse(s))
    }
}

/// Aresta do grafo de conhecimento: par (u, v) + tipo de restrição.
#[derive(Debug, Clone, Copy)]
pub struct FactEdge {
    pub from_idx: u16,
    pub to_idx: u16,
    pub kind: RelationKind,
}

/// Grafo de fatos carregado do `souls_state.db`. Stack-only, zero heap
/// no estado "ready for analysis". O SQLite é consultado read-only.
#[derive(Debug, Clone)]
pub struct SqliteFactGraph {
    /// Lista de vértices (nomes dos fatos). Até MAX_FACTS.
    pub vertex_names: Vec<String>,
    /// Lista de arestas (restrições). Até MAX_RELATIONS.
    pub edges: Vec<FactEdge>,
}

impl SqliteFactGraph {
    /// Constrói grafo vazio (para testes determinísticos).
    pub fn empty() -> Self {
        Self {
            vertex_names: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Carrega grafo de fatos STABLE do `souls_state.db`.
    /// Filtra relações com `entity_type` ou heurística que indique STABLE.
    /// Aqui usamos todas as relações (a flag STABLE é externa à tabela `relations`
    /// — vem de `temporal_stability` em observações).
    pub fn from_stable(conn: &Connection) -> Result<Self, rusqlite::Error> {
        // Coleta vértices distintos
        let mut vertex_names: Vec<String> = Vec::new();
        let mut stmt = conn.prepare("SELECT name FROM entities")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            vertex_names.push(row?);
        }
        // Trunca a MAX_FACTS (stack-safety v1)
        vertex_names.truncate(MAX_FACTS);

        // Coleta arestas
        let mut edges: Vec<FactEdge> = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT from_entity, to_entity, relation_type FROM relations LIMIT ?1",
        )?;
        let rows = stmt.query_map([MAX_RELATIONS as i64], |r| {
            let from_name: String = r.get(0)?;
            let to_name: String = r.get(1)?;
            let rel_type: String = r.get(2)?;
            Ok((from_name, to_name, rel_type))
        })?;
        for row in rows {
            let (from_name, to_name, rel_type) = row?;
            // Resolve índices
            let from_idx = vertex_names.iter().position(|n| n == &from_name);
            let to_idx = vertex_names.iter().position(|n| n == &to_name);
            if let (Some(u), Some(v)) = (from_idx, to_idx) {
                if u != v {
                    edges.push(FactEdge {
                        from_idx: u as u16,
                        to_idx: v as u16,
                        kind: RelationKind::parse(&rel_type),
                    });
                }
            }
        }
        edges.truncate(MAX_RELATIONS);
        Ok(Self { vertex_names, edges })
    }

    /// Construtor de teste: grafo a partir de strings e tipos de relação.
    /// Usado em testes TDD para evitar dependência de SQLite.
    pub fn from_parts(vertices: Vec<String>, edges: Vec<(usize, usize, RelationKind)>) -> Self {
        let edges = edges
            .into_iter()
            .filter(|(u, v, _)| u != v)
            .map(|(u, v, kind)| FactEdge {
                from_idx: u as u16,
                to_idx: v as u16,
                kind,
            })
            .collect();
        Self { vertex_names: vertices, edges }
    }

    /// Número de vértices.
    pub fn n_vertices(&self) -> usize {
        self.vertex_names.len()
    }

    /// Número de arestas.
    pub fn n_edges(&self) -> usize {
        self.edges.len()
    }
}

/// Matriz de restrições aumentada [A | b], armazenada em stack.
/// Tipo: f32 com pivoteamento parcial. Zero alocação na heap.
pub struct RestrictionMatrix {
    /// Cada linha é [a_1, a_2, ..., a_n, b]. n = n_vars, b = constante.
    data: [[f32; MAX_FACTS + 1]; MAX_RELATIONS],
    rows: usize,
    cols: usize,
}

impl RestrictionMatrix {
    /// Constrói matriz vazia para até `n_vars` variáveis e `n_constraints` restrições.
    pub fn new(n_vars: usize, n_constraints: usize) -> Self {
        assert!(n_vars <= MAX_FACTS, "n_vars {n_vars} > MAX_FACTS {MAX_FACTS}");
        assert!(n_constraints <= MAX_RELATIONS, "n_constraints {n_constraints} > MAX_RELATIONS {MAX_RELATIONS}");
        Self {
            data: [[0.0_f32; MAX_FACTS + 1]; MAX_RELATIONS],
            rows: 0,
            cols: n_vars,
        }
    }

    /// Adiciona uma linha `a_1·x_1 + ... + a_n·x_n = b` à matriz.
    /// `coeffs` deve ter tamanho `n_vars` (resto é zero).
    pub fn push_row(&mut self, coeffs: &[f32], b: f32) {
        assert!(self.rows < MAX_RELATIONS, "matriz cheia");
        assert!(coeffs.len() <= self.cols, "coeffs.len() {} > cols {}", coeffs.len(), self.cols);
        for (j, &c) in coeffs.iter().enumerate() {
            self.data[self.rows][j] = c;
        }
        self.data[self.rows][self.cols] = b;
        self.rows += 1;
    }

    /// Acessa a linha `i` como slice (sem a coluna aumentada).
    pub fn row(&self, i: usize) -> &[f32] {
        &self.data[i][..self.cols]
    }

    /// Acessa a coluna aumentada `b` da linha `i`.
    pub fn b(&self, i: usize) -> f32 {
        self.data[i][self.cols]
    }

    /// Número de linhas ativas.
    pub fn n_rows(&self) -> usize {
        self.rows
    }

    /// Número de variáveis (colunas, excluindo a coluna aumentada).
    pub fn n_vars(&self) -> usize {
        self.cols
    }
}

/// Resultado da análise de cohomologia.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CohomologyResult {
    /// Dimensão do primeiro grupo de cohomologia H¹.
    /// H¹ = 0 ⟹ sistema consistente (ciclos harmoniosos são OK).
    /// H¹ > 0 ⟹ existe contradição de verdade.
    pub h1_dimension: usize,
    /// True se H¹ > 0 (contradição detectada).
    pub has_contradiction: bool,
    /// Rank efetivo da matriz de restrições.
    pub rank: usize,
}

/// Gauss-Elimination parcial in-place para calcular o rank da matriz
/// **homogênea** `A` sobre GF(2) (aritmética booleana).
///
/// **Justificativa matemática:** Para detectar contradições lógicas em
/// grafos de conhecimento (fatos booleanos), o sistema `Ax = b` deve
/// ser resolvido sobre GF(2) (XOR), não sobre ℝ. Em ℝ, sistemas com
/// variáveis contínuas geralmente têm solução; em GF(2), a negação
/// `x_u ⊕ x_v = 1` cria contradições detectáveis.
///
/// O rank sobre GF(2) é o número de pivôs após eliminação via XOR.
/// Ciclos harmoniosos (b=0) têm rank consistente com o sistema
/// `Ax = 0`; contradições de verdade (b≠0 fora de im(A) sobre GF(2))
/// elevam o rank da augmentada.
pub fn rank_homogeneous(mat: &mut RestrictionMatrix) -> usize {
    let mut rank = 0;
    let mut row_idx = 0;
    let n = mat.n_vars();
    for col in 0..n {
        // Pivoteamento: encontra linha com 1 na coluna `col` a partir de `row_idx`.
        let mut pivot_row: Option<usize> = None;
        for r in row_idx..mat.n_rows() {
            if mat.data[r][col] > 0.5 {
                pivot_row = Some(r);
                break;
            }
        }
        let Some(pivot_r) = pivot_row else { continue };
        if pivot_r != row_idx {
            mat.data.swap(pivot_r, row_idx);
        }
        // Elimina via XOR com a linha-pivô (apenas cols >= col, não toca b).
        // GF(2): r[c] = r[c] XOR pivot[c]  ⟺  r[c] = 1 se r[c] ≠ pivot[c], senão 0.
        for r in (row_idx + 1)..mat.n_rows() {
            if mat.data[r][col] > 0.5 {
                for c in col..n {
                    let p = mat.data[row_idx][c] > 0.5;
                    let x = mat.data[r][c] > 0.5;
                    mat.data[r][c] = if x == p { 0.0 } else { 1.0 };
                }
            }
        }
        row_idx += 1;
        rank += 1;
    }
    rank
}

/// Gauss-Elimination parcial in-place para calcular o rank da matriz
/// **aumentada** `[A | b]` sobre GF(2). Retorna o rank (número de pivôs).
pub fn rank_augmented(mat: &mut RestrictionMatrix) -> usize {
    let mut rank = 0;
    let mut row_idx = 0;
    let n = mat.n_vars();
    for col in 0..=n {
        // Pivoteamento
        let mut pivot_row: Option<usize> = None;
        for r in row_idx..mat.n_rows() {
            if mat.data[r][col] > 0.5 {
                pivot_row = Some(r);
                break;
            }
        }
        let Some(pivot_r) = pivot_row else { continue };
        if pivot_r != row_idx {
            mat.data.swap(pivot_r, row_idx);
        }
        // Elimina via XOR com a linha-pivô (incluindo a coluna b).
        for r in (row_idx + 1)..mat.n_rows() {
            if mat.data[r][col] > 0.5 {
                for c in col..=n {
                    let p = mat.data[row_idx][c] > 0.5;
                    let x = mat.data[r][c] > 0.5;
                    mat.data[r][c] = if x == p { 0.0 } else { 1.0 };
                }
            }
        }
        row_idx += 1;
        rank += 1;
    }
    rank
}

/// Calcula a cohomologia do feixe socrático sobre o grafo de fatos.
///
/// **DIRETRIZ 1 do Arquiteto-Chefe (inegociável):**
/// H¹ = `rank([A|b]) - rank(A)` (Rouché-Capelli). Esta métrica distingue
/// corretamente:
/// - **Ciclos harmoniosos** (b=0): H¹ = 0 (sistema consistente, mesmo com ciclos).
/// - **Conflito de premissas** (b≠0 fora de im(A)): H¹ > 0 (contradição).
///
/// A função NÃO usa a fórmula simplista `m - rank` que confunde ciclos
/// topológicos com contradições lógicas.
pub fn compute_cohomology(graph: &SqliteFactGraph) -> CohomologyResult {
    let n = graph.n_vertices();
    let m = graph.n_edges();
    if n == 0 {
        return CohomologyResult { h1_dimension: 0, has_contradiction: false, rank: 0 };
    }
    // 1. Constrói matriz para calcular rank da homogênea (A).
    let mut mat_h = RestrictionMatrix::new(n, m);
    // 2. Constrói matriz separada para calcular rank da augmentada ([A|b]).
    //    Como RestrictionMatrix é in-place, precisamos de duas cópias.
    //    Usa a API `row` para clonar a linha antes de adicionar a `b`.
    let mut mat_aug = RestrictionMatrix::new(n, m);
    for edge in &graph.edges {
        let u = edge.from_idx as usize;
        let v = edge.to_idx as usize;
        if u >= n || v >= n {
            continue;
        }
        let mut coeffs = vec![0.0_f32; n];
        let b = match edge.kind {
            // Em GF(2): x_v = x_u ≡ x_u + x_v = 0. Coefs: +1, +1, b=0.
            RelationKind::DependsOn | RelationKind::EquivalentTo | RelationKind::Implies => {
                coeffs[u] = 1.0;
                coeffs[v] = 1.0;
                0.0
            }
            // Em GF(2): x_u ⊕ x_v = 1 ≡ x_u + x_v = 1. Coefs: +1, +1, b=1.
            RelationKind::ConflictsWith | RelationKind::Negates => {
                coeffs[u] = 1.0;
                coeffs[v] = 1.0;
                1.0
            }
        };
        mat_h.push_row(&coeffs, 0.0);
        mat_aug.push_row(&coeffs, b);
    }
    // rank_homogeneous é mutativo; clonar a matriz via swap_rows é caro.
    // Como mat_h e mat_aug são cópias independentes, calculamos os ranks separadamente.
    let rank_h = rank_homogeneous(&mut mat_h);
    let rank_aug = rank_augmented(&mut mat_aug);
    // H¹ = rank([A|b]) - rank(A)  (Rouché-Capelli sobre GF(2))
    // rank_aug > rank_h → sistema inconsistente → H¹ > 0 (contradição de verdade)
    // rank_aug == rank_h → sistema consistente → H¹ = 0 (ciclos harmoniosos OK)
    let h1 = rank_aug.saturating_sub(rank_h);
    CohomologyResult {
        h1_dimension: h1,
        has_contradiction: h1 > 0,
        rank: rank_h,
    }
}

/// Boost determinístico de `conflito_memoria` quando H¹ > 0.
/// Threshold: retorna max(base, 0.86) quando há contradição, senão `base`.
pub fn boost_conflito_memoria(base: f32, h1: &CohomologyResult) -> f32 {
    if h1.has_contradiction {
        base.max(0.86)
    } else {
        base
    }
}

/// Orquestrador principal: carrega grafo do SQLite, calcula cohomologia,
/// aplica boost ao `EpistemicScores` se houver contradição.
pub fn apply_cohomology_boost(
    conn: &Connection,
    base_scores: &mut EpistemicScores,
) -> Result<CohomologyResult, rusqlite::Error> {
    let graph = SqliteFactGraph::from_stable(conn)?;
    let h1 = compute_cohomology(&graph);
    if h1.has_contradiction {
        base_scores.conflito_memoria = boost_conflito_memoria(base_scores.conflito_memoria, &h1);
    }
    Ok(h1)
}

/// Helper de teste: cria um grafo de fatos com dependências circulares.
/// Usado em testes TDD para simular contradições sem SQLite.
pub fn test_graph_with_cycle(n: usize, with_contradiction: bool) -> SqliteFactGraph {
    let vertices: Vec<String> = (0..n).map(|i| format!("fact_{i}")).collect();
    let mut edges = Vec::new();
    // Cria ciclo harmonioso: 0→1→2→...→(n-1)→0 com `DependsOn` (consistente)
    for i in 0..n {
        edges.push(((i + 1) % n, i, RelationKind::DependsOn));
    }
    if with_contradiction {
        // Adiciona uma aresta `ConflictsWith` que quebra a consistência.
        // Ex: fact_0 ConflictsWith fact_2 (se fact_0 = fact_2 via ciclo, contradição)
        if n >= 3 {
            edges.push((0, 2, RelationKind::ConflictsWith));
        }
    }
    SqliteFactGraph::from_parts(vertices, edges)
}

/// Range de IDs (helper para testes). Não usado em produção.
#[allow(dead_code)]
pub fn valid_id_range(n: usize) -> Range<usize> {
    0..n
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Marco 4.10.0 — ETAPA 2: Cohomology (3 testes TDD)
    // ========================================================================

    /// TDD-5: Grafo acíclico → H¹ = 0 (sem contradição).
    /// Cadeia linear A → B → C com `DependsOn` (sistema consistente).
    #[test]
    fn test_cohomology_acyclic_graph_has_zero_h1() {
        let graph = SqliteFactGraph::from_parts(
            vec!["A".into(), "B".into(), "C".into()],
            vec![(0, 1, RelationKind::DependsOn), (1, 2, RelationKind::DependsOn)],
        );
        let h1 = compute_cohomology(&graph);
        assert_eq!(h1.h1_dimension, 0, "grafo acíclico deve ter H¹ = 0, foi {}", h1.h1_dimension);
        assert!(!h1.has_contradiction);
        // Boost: base 0.5 sem contradição → 0.5 (inalterado)
        let boosted = boost_conflito_memoria(0.5, &h1);
        assert!((boosted - 0.5).abs() < 1e-4, "boost deve preservar 0.5 sem contradição, foi {boosted}");
    }

    /// TDD-6: Ciclo harmonioso (b₁ = 1 mas H¹ = 0) → sem contradição.
    /// Triângulo A→B→C→A com `DependsOn` é ciclo topológico mas matematicamente
    /// consistente (não há contradição de verdade).
    #[test]
    fn test_cohomology_harmonious_cycle_has_zero_h1() {
        let graph = test_graph_with_cycle(3, false);
        assert_eq!(graph.n_vertices(), 3);
        assert_eq!(graph.n_edges(), 3);
        let h1 = compute_cohomology(&graph);
        assert_eq!(h1.h1_dimension, 0, "ciclo harmonioso deve ter H¹ = 0, foi {}", h1.h1_dimension);
        assert!(!h1.has_contradiction, "ciclo de DependsOn não é contradição");
        // Boost preserva base
        let boosted = boost_conflito_memoria(0.3, &h1);
        assert!((boosted - 0.3).abs() < 1e-4, "boost sem contradição = base, foi {boosted}");
    }

    /// TDD-7: Conflito de premissas (H¹ > 0) → boost > 0.85.
    /// Grafo com ciclo + aresta `ConflictsWith` que cria contradição lógica.
    #[test]
    fn test_cohomology_conflicting_premises_boosts_score_above_threshold() {
        let graph = test_graph_with_cycle(3, true);
        assert_eq!(graph.n_vertices(), 3);
        assert!(graph.n_edges() >= 4, "deve ter ciclo + conflito, n_edges = {}", graph.n_edges());
        let h1 = compute_cohomology(&graph);
        assert!(h1.has_contradiction, "deve detectar contradição, H¹ = {}", h1.h1_dimension);
        assert!(h1.h1_dimension > 0, "H¹ > 0 esperado, foi {}", h1.h1_dimension);
        // Boost: base 0.5 com contradição → 0.86
        let boosted = boost_conflito_memoria(0.5, &h1);
        assert!(boosted > 0.85, "boost com contradição deve ser > 0.85, foi {boosted}");
        // Boost: base 0.9 com contradição → preserva 0.9
        let boosted2 = boost_conflito_memoria(0.9, &h1);
        assert!((boosted2 - 0.9).abs() < 1e-4, "boost com base > 0.86 deve preservar base, foi {boosted2}");
    }

    // ========================================================================
    // Testes estruturais (não-conta para DoD de 3 testes)
    // ========================================================================

    #[test]
    fn test_restriction_matrix_basic() {
        // Em GF(2): coeffs=-1.0 é tratado como 0, então [1,-1,0] ≡ [1,0,0].
        // Matriz é diagonal com 2 pivôs.
        let mut mat = RestrictionMatrix::new(3, 2);
        mat.push_row(&[1.0, 0.0, 0.0], 0.0);
        mat.push_row(&[0.0, 1.0, 0.0], 0.0);
        assert_eq!(mat.n_rows(), 2);
        assert_eq!(mat.n_vars(), 3);
        let rank = rank_augmented(&mut mat);
        assert_eq!(rank, 2, "matriz 2x3 com 2 pivôs independentes");
    }

    #[test]
    fn test_rank_augmented_handles_zero_matrix() {
        let mut mat = RestrictionMatrix::new(3, 3);
        // Matriz nula → rank 0
        let rank = rank_augmented(&mut mat);
        assert_eq!(rank, 0);
    }

    #[test]
    fn test_relation_kind_parsing() {
        assert_eq!(RelationKind::parse("depends_on"), RelationKind::DependsOn);
        assert_eq!(RelationKind::parse("CONFLICTS_WITH"), RelationKind::ConflictsWith);
        assert_eq!(RelationKind::parse("implies"), RelationKind::Implies);
        assert_eq!(RelationKind::parse("unknown_kind"), RelationKind::DependsOn); // fallback
    }
}
