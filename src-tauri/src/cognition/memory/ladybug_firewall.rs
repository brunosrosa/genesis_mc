// SOULS V6 — Memory Module: LadybugDB Ontological Firewall (Anti-RAG Poisoning)
// Conforme ADR-001, ADR-005, ADR-030 e Marco VI (Firewall Ontológico Fail-Closed na RAM Host).

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

/// Representação de um nó ontológico no grafo LadybugDB em memória.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OntologicalNode {
    pub id: String,
    pub node_type: String, // "ADR" | "SourceCode" | "Dependency" | "Constraint"
    pub stability_status: String, // "STABLE" | "EVOLVING"
    pub banned_patterns: Vec<String>,
    pub required_patterns: Vec<String>,
}

/// Representação de uma aresta direcionada no grafo ontológico.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct OntologicalEdge {
    pub from_node: String,
    pub to_node: String,
    pub relation_type: String, // "depends_on" | "replaces" | "implements" | "violates" | "violates_red_line_of"
}

/// Veredito emitido pelo Firewall Ontológico após varredura BFS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FirewallVerdict {
    Approved,
    Vetoed {
        reason: String,
        violated_node: String,
        relation_path: Vec<String>,
    },
}

/// Firewall Ontológico gerenciando a malha de grafos em RAM Host via DashMap.
#[derive(Debug, Clone)]
pub struct OntologicalFirewall {
    nodes: Arc<DashMap<String, OntologicalNode>>,
    edges: Arc<DashMap<String, Vec<OntologicalEdge>>>,
}

impl Default for OntologicalFirewall {
    fn default() -> Self {
        let firewall = Self::new();
        firewall.seed_canonical_adrs();
        firewall
    }
}

impl OntologicalFirewall {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            edges: Arc::new(DashMap::new()),
        }
    }

    /// Carrega as ADRs canônicas invioláveis no grafo em RAM.
    pub fn seed_canonical_adrs(&self) {
        self.register_node(
            "ADR-030",
            "ADR",
            "STABLE",
            &["winapi", "core_affinity"],
            &["windows-sys = \"=0.61.2\""],
        );

        self.register_node(
            "ADR-027",
            "ADR",
            "STABLE",
            &["cudaMalloc(rag)", "vram_rag_alloc", "gpu_vector_index"],
            &["0 MB VRAM", "mmap2", "CPU Host"],
        );

        self.register_node(
            "ADR-001",
            "ADR",
            "STABLE",
            &["python_runtime_prod", "node_runtime_prod"],
            &["Rust", "Tokio"],
        );
    }

    /// Registra ou atualiza um nó no grafo ontológico.
    pub fn register_node(
        &self,
        id: &str,
        node_type: &str,
        stability: &str,
        banned_patterns: &[&str],
        required_patterns: &[&str],
    ) {
        self.nodes.insert(
            id.to_string(),
            OntologicalNode {
                id: id.to_string(),
                node_type: node_type.to_string(),
                stability_status: stability.to_string(),
                banned_patterns: banned_patterns.iter().map(|s| s.to_string()).collect(),
                required_patterns: required_patterns.iter().map(|s| s.to_string()).collect(),
            },
        );
    }

    /// Registra uma aresta causal direcionada entre entidades no grafo.
    pub fn register_edge(&self, from: &str, to: &str, relation_type: &str) {
        let edge = OntologicalEdge {
            from_node: from.to_string(),
            to_node: to.to_string(),
            relation_type: relation_type.to_string(),
        };

        self.edges
            .entry(from.to_string())
            .or_default()
            .push(edge);
    }

    /// Executa travessia BFS limitada (depth <= max_depth) a partir de um nó inicial,
    /// avaliando se o conteúdo sugerido viola alguma decisão ou regra das entidades alcançadas.
    pub fn bfs_check_compliance(
        &self,
        start_node: &str,
        candidate_content: &str,
        max_depth: usize,
    ) -> FirewallVerdict {
        let mut queue: VecDeque<(String, usize, Vec<String>)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();

        queue.push_back((start_node.to_string(), 0, vec![start_node.to_string()]));
        visited.insert(start_node.to_string());

        while let Some((curr_node_id, depth, path)) = queue.pop_front() {
            // Verifica as regras do nó atual
            if let Some(node_ref) = self.nodes.get(&curr_node_id) {
                let node = node_ref.value();
                for banned in &node.banned_patterns {
                    if candidate_content.contains(banned) {
                        let reason = format!(
                            "RAG Poisoning detectado: chunk viola o nó estável '{}' ({}) contendo padrão banido '{}'",
                            node.id, node.stability_status, banned
                        );
                        eprintln!("[LadybugDB Ontological Firewall] VETO: {}", reason);
                        return FirewallVerdict::Vetoed {
                            reason,
                            violated_node: node.id.clone(),
                            relation_path: path.clone(),
                        };
                    }
                }
            }

            if depth >= max_depth {
                continue;
            }

            // Explora arestas de saída
            if let Some(edges_ref) = self.edges.get(&curr_node_id) {
                for edge in edges_ref.value() {
                    if !visited.contains(&edge.to_node) {
                        visited.insert(edge.to_node.clone());
                        let mut next_path = path.clone();
                        next_path.push(format!("{}:{}", edge.relation_type, edge.to_node));
                        queue.push_back((edge.to_node.clone(), depth + 1, next_path));
                    }
                }
            }
        }

        FirewallVerdict::Approved
    }

    /// Intercepta e filtra uma lista de conteúdos recuperados, expurgando fragmentos que violem o firewall ontológico.
    pub fn sanitize_chunks<T, F>(&self, target_entity: &str, items: Vec<T>, get_text: F) -> (Vec<T>, Vec<String>)
    where
        F: Fn(&T) -> &str,
    {
        let mut approved = Vec::new();
        let mut vetoed_reasons = Vec::new();

        for item in items {
            let text = get_text(&item);
            match self.bfs_check_compliance(target_entity, text, 4) {
                FirewallVerdict::Approved => approved.push(item),
                FirewallVerdict::Vetoed { reason, .. } => vetoed_reasons.push(reason),
            }
        }

        (approved, vetoed_reasons)
    }
}
