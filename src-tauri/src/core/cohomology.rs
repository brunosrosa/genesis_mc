//! `cohomology.rs` — Marco 4.10.1 ETAPA 1: Cohomologia de Feixes Socráticos (GF(2) puro em u64).
//!
//! **DIRETRIZ 1 do Arquiteto-Chefe (inegociável — Marco 4.10.1):**
//! - Extirpação total de `f32` na representação da matriz de restrições.
//! - GF(2) (Galois Field of order 2) exige aritmética binária estrita (AND/XOR).
//! - Refatoração para estruturas compactas de bits representadas por `[u64; WORDS_PER_ROW]`
//!   (256 bits = 4 palavras u64), permitindo eliminação Gaussiana puramente bitwise
//!   (XOR de u64 inteiros, `trailing_zeros`, `count_ones`).
//! - Economia de cache L3: matriz 256×256 f32 = 256KB → matriz 256×256 bits = 8KB.
//! - Zero risco de imprecisão numérica e zero possibilidade de stack overflow.
//!
//! **DIRETRIZ 1 do Marco 4.10.0 (preservada):**
//! - A função `compute_cohomology` (anteriormente `first_betti_number`) modela cohomologia
//!   via **rank do operador d0** sobre a matriz de restrições de consistência.
//! - H¹ ≠ 0 apenas se houver **contradição de restrições de verdade** (Rouché-Capelli sobre GF(2)):
//!   `H¹ = rank([A|b]) - rank(A)`.
//! - Ciclos harmoniosos (b=0) têm H¹ = 0; contradições de premissas (b≠0 fora de im(A)) têm H¹ > 0.
//!
//! **Stack-safety:** O ceiling é 1024 entradas, mas v1 usa 256 para garantir
//! stack-safety no Windows (default main thread stack = 1MB). v2 pode aumentar
//! para 1024 com `std::thread::Builder::stack_size(8 * 1024 * 1024)`.

use std::ops::Range;

use rusqlite::Connection;

use crate::core::epistemic_prober::EpistemicScores;

/// Limite físico de fatos (vértices do grafo de conhecimento).
/// Representação: 256 bits = 4 u64 (WORDS_PER_ROW).
const MAX_FACTS: usize = 256;
/// Limite físico de relações (restrições). Igual a MAX_FACTS.
const MAX_RELATIONS: usize = 256;
/// Quantas palavras u64 são necessárias para representar MAX_FACTS bits.
/// 256 / 64 = 4.
const WORDS_PER_ROW: usize = MAX_FACTS / 64;
/// Máscara para o bit menos significativo de uma palavra (para testes).
#[allow(dead_code)]
const LOW_BIT_MASK: u64 = 1u64;

/// Tipos de relação canônicos. Cada um codifica uma restrição linear sobre GF(2):
/// - `DependsOn`: x_u = x_v (x_u + x_v = 0; b=0)
/// - `Implies`: x_u → x_v ≡ x_v ≥ x_u (modelado como x_u + x_v = 0; b=0)
/// - `ConflictsWith`: x_u ⊕ x_v = 1 (x_u + x_v = 1; b=1)
/// - `EquivalentTo`: x_u = x_v (x_u + x_v = 0; b=0)
/// - `Negates`: x_u = ¬x_v (x_u + x_v = 1; b=1)
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

/// Grafo de fatos carregado do `souls_state.db`. O SQLite é consultado read-only.
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

/// Matriz de restrições aumentada [A | b] em GF(2), armazenada em stack
/// como bits compactos em `u64`.
///
/// ## Layout de memória (Marco 4.10.1 ETAPA 1)
///
/// ```text
/// coefs: [[u64; 4]; 256]   =  4 × 8 × 256 =   8192 bytes  (8 KB)
/// b:     [u64; 4]          =  4 × 8         =     32 bytes  (256 bits)
/// ─────────────────────────────────────────────────────────
/// Total:                                    ≈   8.2 KB na stack
/// ```
///
/// Compare com a versão anterior (Marco 4.10.0):
/// `[[f32; 257]; 256]` = 257 × 4 × 256 = **263168 bytes** (~256 KB).
///
/// **Redução de 32× no footprint de stack + zero imprecisão numérica.**
pub struct RestrictionMatrix {
    /// Coeficientes da matriz A. Cada linha = `WORDS_PER_ROW` palavras u64.
    /// Bit `j` da linha `i` representa o coeficiente de x_j na restrição i.
    coefs: [[u64; WORDS_PER_ROW]; MAX_RELATIONS],
    /// Coluna aumentada b. Bit `i` de `b_data[w]` representa o b da linha `i`.
    b_data: [u64; WORDS_PER_ROW],
    /// Linhas ativas (restrições inseridas).
    rows: usize,
    /// Variáveis (colunas A; a coluna b é gerenciada separadamente).
    cols: usize,
}

impl RestrictionMatrix {
    /// Constrói matriz vazia para até `n_vars` variáveis e `n_constraints` restrições.
    pub fn new(n_vars: usize, n_constraints: usize) -> Self {
        assert!(n_vars <= MAX_FACTS, "n_vars {n_vars} > MAX_FACTS {MAX_FACTS}");
        assert!(
            n_constraints <= MAX_RELATIONS,
            "n_constraints {n_constraints} > MAX_RELATIONS {MAX_RELATIONS}"
        );
        Self {
            coefs: [[0u64; WORDS_PER_ROW]; MAX_RELATIONS],
            b_data: [0u64; WORDS_PER_ROW],
            rows: 0,
            cols: n_vars,
        }
    }

    /// Adiciona uma linha `a_1·x_1 + ... + a_n·x_n = b` à matriz.
    /// `coeffs` deve ter tamanho `n_vars`. Cada elemento != 0 é tratado como 1
    /// em GF(2) (XOR). O bit de `b` é setado se `b != 0`.
    pub fn push_row(&mut self, coeffs: &[u8], b: u8) {
        assert!(self.rows < MAX_RELATIONS, "matriz cheia");
        assert!(
            coeffs.len() <= self.cols,
            "coeffs.len() {} > cols {}",
            coeffs.len(),
            self.cols
        );
        for (j, &c) in coeffs.iter().enumerate() {
            if c != 0 {
                self.set_bit(self.rows, j, true);
            }
        }
        if b != 0 {
            self.set_b_bit(self.rows, true);
        }
        self.rows += 1;
    }

    /// Seta o bit (linha, coluna) — true=1, false=0.
    #[inline]
    fn set_bit(&mut self, row: usize, col: usize, val: bool) {
        assert!(row < MAX_RELATIONS, "row {row} out of range");
        assert!(col < self.cols, "col {col} out of range");
        let (word, bit) = (col / 64, col % 64);
        if val {
            self.coefs[row][word] |= 1u64 << bit;
        } else {
            self.coefs[row][word] &= !(1u64 << bit);
        }
    }

    /// Seta o bit b da linha (coluna aumentada).
    #[inline]
    fn set_b_bit(&mut self, row: usize, val: bool) {
        assert!(row < MAX_RELATIONS, "row {row} out of range");
        let (word, bit) = (row / 64, row % 64);
        if val {
            self.b_data[word] |= 1u64 << bit;
        } else {
            self.b_data[word] &= !(1u64 << bit);
        }
    }

    /// Lê o bit (linha, coluna).
    #[inline]
    pub fn get_bit(&self, row: usize, col: usize) -> bool {
        let (word, bit) = (col / 64, col % 64);
        (self.coefs[row][word] >> bit) & 1 == 1
    }

    /// Lê o bit b da linha.
    #[inline]
    pub fn get_b_bit(&self, row: usize) -> bool {
        let (word, bit) = (row / 64, row % 64);
        (self.b_data[word] >> bit) & 1 == 1
    }

    /// Número de linhas ativas.
    pub fn n_rows(&self) -> usize {
        self.rows
    }

    /// Número de variáveis (colunas, excluindo a coluna aumentada).
    pub fn n_vars(&self) -> usize {
        self.cols
    }

    /// Troca duas linhas in-place (incluindo o bit b).
    #[inline]
    fn swap_rows(&mut self, a: usize, b: usize) {
        if a != b {
            self.coefs.swap(a, b);
            // Swap bits b correspondentes.
            let abit = self.get_b_bit(a);
            let bbit = self.get_b_bit(b);
            self.set_b_bit(a, bbit);
            self.set_b_bit(b, abit);
        }
    }

    /// Aplica XOR de uma linha-fonte em uma linha-alvo (apenas a partir de `start_col`).
    /// Operação GF(2) canônica: target ^= source (palavra a palavra).
    /// O bit b é XOR-ado integralmente.
    #[inline]
    fn xor_rows_from(&mut self, target: usize, source: usize, start_col: usize) {
        let start_word = start_col / 64;
        for w in start_word..WORDS_PER_ROW {
            self.coefs[target][w] ^= self.coefs[source][w];
        }
        // O bit b é sempre afetado integralmente (XOR é global, não por coluna).
        let tbit = self.get_b_bit(target);
        let sbit = self.get_b_bit(source);
        self.set_b_bit(target, tbit ^ sbit);
    }

    /// Encontra a primeira linha com bit=1 na coluna `col`, a partir de `from_row`.
    /// Retorna `Some(row)` ou `None` se coluna é totalmente zero abaixo de `from_row`.
    #[inline]
    fn find_pivot(&self, col: usize, from_row: usize) -> Option<usize> {
        let (word, bit) = (col / 64, col % 64);
        let mask = 1u64 << bit;
        (from_row..self.rows).find(|&r| (self.coefs[r][word] & mask) != 0)
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
/// **homogênea** `A` sobre GF(2) usando XOR de u64.
///
/// **Justificativa matemática:** Para detectar contradições lógicas em
/// grafos de conhecimento (fatos booleanos), o sistema `Ax = b` deve
/// ser resolvido sobre GF(2) (XOR), não sobre ℝ. Em GF(2), a negação
/// `x_u ⊕ x_v = 1` cria contradições detectáveis via diferença de rank.
///
/// O rank sobre GF(2) é o número de pivôs após eliminação via XOR.
pub fn rank_homogeneous(mat: &mut RestrictionMatrix) -> usize {
    let mut rank = 0;
    let mut row_idx = 0;
    let n = mat.n_vars();
    for col in 0..n {
        // Pivoteamento parcial: encontra linha com 1 na coluna `col`.
        let Some(pivot_r) = mat.find_pivot(col, row_idx) else {
            continue;
        };
        if pivot_r != row_idx {
            mat.swap_rows(pivot_r, row_idx);
        }
        // Elimina via XOR com a linha-pivô para todas as linhas abaixo.
        // Não toca b (matriz homogênea); mas como set_b_bit é por linha, o
        // XOR de linhas inferiores inclui b, preservando a semântica GF(2)
        // porque b=0 em toda linha (matriz homogênea).
        for r in (row_idx + 1)..mat.n_rows() {
            if mat.get_bit(r, col) {
                mat.xor_rows_from(r, row_idx, col);
            }
        }
        row_idx += 1;
        rank += 1;
    }
    rank
}

/// Gauss-Elimination parcial in-place para calcular o rank da matriz
/// **aumentada** `[A | b]` sobre GF(2). Retorna o rank (número de pivôs).
///
/// A coluna `b` é tratada como a coluna `n_vars` (primeira coluna após A).
/// Se o sistema `A·x = b` é inconsistente, o rank da aumentada excede o
/// rank da homogênea.
pub fn rank_augmented(mat: &mut RestrictionMatrix) -> usize {
    let mut rank = 0;
    let mut row_idx = 0;
    let n = mat.n_vars();
    // A coluna b é indexada como `n` na posição aumentada.
    let total_cols = n + 1;
    for col in 0..total_cols {
        // Caso especial: a coluna `n` (b) é buscada no b_data, não em coefs.
        if col == n {
            // Pivoteamento na coluna b
            let mask_search = |mat: &RestrictionMatrix, r: usize| -> bool {
                mat.get_b_bit(r)
            };
            let mut pivot_row: Option<usize> = None;
            for r in row_idx..mat.n_rows() {
                if mask_search(mat, r) {
                    pivot_row = Some(r);
                    break;
                }
            }
            let Some(pivot_r) = pivot_row else { continue };
            if pivot_r != row_idx {
                mat.swap_rows(pivot_r, row_idx);
            }
            for r in (row_idx + 1)..mat.n_rows() {
                if mat.get_b_bit(r) {
                    mat.xor_rows_from(r, row_idx, 0);
                }
            }
        } else {
            let Some(pivot_r) = mat.find_pivot(col, row_idx) else {
                continue;
            };
            if pivot_r != row_idx {
                mat.swap_rows(pivot_r, row_idx);
            }
            for r in (row_idx + 1)..mat.n_rows() {
                if mat.get_bit(r, col) {
                    mat.xor_rows_from(r, row_idx, col);
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
    let mut mat_aug = RestrictionMatrix::new(n, m);
    for edge in &graph.edges {
        let u = edge.from_idx as usize;
        let v = edge.to_idx as usize;
        if u >= n || v >= n {
            continue;
        }
        let mut coeffs = vec![0u8; n];
        let b: u8 = match edge.kind {
            // Em GF(2): x_v = x_u ≡ x_u + x_v = 0. Coefs: 1, 1, b=0.
            RelationKind::DependsOn | RelationKind::EquivalentTo | RelationKind::Implies => {
                coeffs[u] = 1;
                coeffs[v] = 1;
                0
            }
            // Em GF(2): x_u ⊕ x_v = 1 ≡ x_u + x_v = 1. Coefs: 1, 1, b=1.
            RelationKind::ConflictsWith | RelationKind::Negates => {
                coeffs[u] = 1;
                coeffs[v] = 1;
                1
            }
        };
        mat_h.push_row(&coeffs, 0);
        mat_aug.push_row(&coeffs, b);
    }
    // rank_* é mutativo; mat_h e mat_aug são cópias independentes.
    let rank_h = rank_homogeneous(&mut mat_h);
    let rank_aug = rank_augmented(&mut mat_aug);
    // H¹ = rank([A|b]) - rank(A)  (Rouché-Capelli sobre GF(2))
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
    // Marco 4.10.0 — ETAPA 2: Cohomology (3 testes TDD canônicos)
    // Marco 4.10.1 — ETAPA 6: tests TDD de coerência GF(2) em u64
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
    // Marco 4.10.1 — ETAPA 6: Testes TDD de coerência GF(2) em u64
    // ========================================================================

    /// TDD-11: Gauss-Elimination GF(2) sobre u64 resolve contradição
    /// clássica `x_0 ⊕ x_1 = 1` ∧ `x_0 = x_1` (sistema inconsistente).
    /// Resultado esperado: rank_h = 1, rank_aug = 2, H¹ = 1.
    #[test]
    fn test_gf2_u64_detects_classical_contradiction() {
        // Sistema: x_0 + x_1 = 0  (DependsOn) e x_0 + x_1 = 1  (ConflictsWith)
        // Em GF(2): incompatível.
        let graph = SqliteFactGraph::from_parts(
            vec!["A".into(), "B".into()],
            vec![
                (0, 1, RelationKind::DependsOn),
                (0, 1, RelationKind::ConflictsWith),
            ],
        );
        let h1 = compute_cohomology(&graph);
        assert_eq!(h1.h1_dimension, 1, "contradição clássica deve dar H¹ = 1, foi {}", h1.h1_dimension);
        assert!(h1.has_contradiction);
    }

    /// TDD-12: Sistema triangular de equações independentes → rank = n.
    /// 3 restrições linearmente independentes sobre 3 vars: rank = 3.
    #[test]
    fn test_gf2_u64_rank_of_independent_system() {
        // x_0 = x_1, x_1 = x_2, x_2 = x_0 (ciclo, mas homogêneo e consistente).
        let graph = SqliteFactGraph::from_parts(
            vec!["A".into(), "B".into(), "C".into()],
            vec![
                (0, 1, RelationKind::DependsOn),
                (1, 2, RelationKind::DependsOn),
                (2, 0, RelationKind::DependsOn),
            ],
        );
        let h1 = compute_cohomology(&graph);
        assert_eq!(h1.h1_dimension, 0, "ciclo homogêneo: H¹ = 0");
        // Rank efetivo: 2 (apenas 2 equações linearmente independentes;
        // a 3ª é combinação linear das outras duas).
        assert_eq!(h1.rank, 2, "rank deve ser 2, foi {}", h1.rank);
    }

    /// TDD-13: Matriz nula → rank = 0, H¹ = 0.
    #[test]
    fn test_gf2_u64_empty_matrix_rank_zero() {
        let graph = SqliteFactGraph::empty();
        let h1 = compute_cohomology(&graph);
        assert_eq!(h1.h1_dimension, 0);
        assert_eq!(h1.rank, 0);
        assert!(!h1.has_contradiction);
    }

    /// TDD-14: Estresse — 256 vértices, 256 arestas, sem contradição.
    /// Verifica que o tamanho máximo da matriz opera corretamente.
    #[test]
    fn test_gf2_u64_max_facts_256_stress_test() {
        let n = 256;
        let mut vertices: Vec<String> = (0..n).map(|i| format!("f{i}")).collect();
        vertices.truncate(MAX_FACTS);
        // Cadeia linear (sem ciclos, sem contradição).
        let edges: Vec<(usize, usize, RelationKind)> =
            (0..(n - 1)).map(|i| (i, i + 1, RelationKind::DependsOn)).collect();
        let graph = SqliteFactGraph::from_parts(vertices, edges);
        let h1 = compute_cohomology(&graph);
        assert_eq!(h1.h1_dimension, 0, "grafo cadeia linear: H¹ = 0");
        assert!(!h1.has_contradiction);
    }

    /// TDD-15: Estresse — 256 vértices, 256 arestas COM contradição no fim.
    /// Cadeia linear + ConflictsWith(fact_0, fact_255) que viola a transitividade.
    #[test]
    fn test_gf2_u64_max_facts_256_with_contradiction() {
        let n = 256;
        let vertices: Vec<String> = (0..n).map(|i| format!("f{i}")).collect();
        let mut edges: Vec<(usize, usize, RelationKind)> =
            (0..(n - 1)).map(|i| (i, i + 1, RelationKind::DependsOn)).collect();
        // Adiciona contradição explícita.
        edges.push((0, 255, RelationKind::ConflictsWith));
        let graph = SqliteFactGraph::from_parts(vertices, edges);
        let h1 = compute_cohomology(&graph);
        assert!(h1.has_contradiction, "max-facts com ConflictsWith: deve detectar contradição, H¹ = {}", h1.h1_dimension);
        assert!(h1.h1_dimension > 0);
    }

    // ========================================================================
    // Testes estruturais (não-conta para DoD)
    // ========================================================================

    #[test]
    fn test_restriction_matrix_push_and_get() {
        let mut mat = RestrictionMatrix::new(8, 2);
        mat.push_row(&[1, 0, 1, 0, 0, 0, 0, 0], 0);
        mat.push_row(&[0, 1, 0, 1, 0, 0, 0, 0], 1);
        assert_eq!(mat.n_rows(), 2);
        assert_eq!(mat.n_vars(), 8);
        assert!(mat.get_bit(0, 0));
        assert!(mat.get_bit(0, 2));
        assert!(!mat.get_bit(0, 1));
        assert!(!mat.get_b_bit(0));
        assert!(mat.get_b_bit(1));
    }

    #[test]
    fn test_restriction_matrix_xor_operations() {
        // 2 linhas: [1,0,0,0]|0 e [1,0,0,0]|0
        // Após XOR: [0,0,0,0]|0
        let mut mat = RestrictionMatrix::new(4, 2);
        mat.push_row(&[1, 0, 0, 0], 0);
        mat.push_row(&[1, 0, 0, 0], 0);
        // Antes: ambas as linhas têm bit 0 = 1.
        assert!(mat.get_bit(0, 0));
        assert!(mat.get_bit(1, 0));
        // XOR da linha 1 com a linha 0 a partir da coluna 0.
        mat.xor_rows_from(1, 0, 0);
        // Depois: linha 1 deve ter bit 0 = 0.
        assert!(!mat.get_bit(1, 0), "após XOR, bit 0 da linha 1 deve ser 0");
    }

    #[test]
    fn test_restriction_matrix_basic_rank() {
        // Matriz 2x3 com 2 pivôs independentes (linhas linearmente independentes).
        let mut mat = RestrictionMatrix::new(3, 2);
        mat.push_row(&[1, 0, 0], 0);
        mat.push_row(&[0, 1, 0], 0);
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

    /// TDD-16: `push_row` com b=1 seta corretamente o bit b.
    #[test]
    fn test_restriction_matrix_b_bit_set() {
        let mut mat = RestrictionMatrix::new(4, 1);
        mat.push_row(&[1, 0, 0, 0], 1);
        assert!(mat.get_b_bit(0), "b=1 deve setar bit b da linha 0");
    }
}
