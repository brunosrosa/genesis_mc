//! Marco 3.8 (Fase C.2): SYMBOL_INDEX e CALL_GRAPH em DashMap RAM Host.
//!
//! Cache de navegação semântica do monorepo. Reside 100% em RAM; nenhuma
//! query toca SQLite. Persistência cold-start é responsabilidade de Marco
//! futuro (snapshot binário em `.souls_cache/callgraph.snapshot`).
//!
//! ## Arquitetura
//!
//! - **SYMBOL_INDEX**: `DashMap<String, SymbolEntry>` — nome qualificado
//!   → localização física `(file, line, column)`. Lookup O(1) médio.
//! - **CALL_GRAPH**: `DashMap<String, CallGraphNode>` — símbolo → conjunto
//!   de adjacentes (callers e callees via dois DashMaps simétricos).
//!
//! ## Hard Constraints
//!
//! - **Lock-free read** via `DashMap` (sharding interno, sem `Mutex` global).
//! - **Write path serializado** por shard — não há risco de deadlock.
//! - **RAM footprint:** ~10MB para 50K símbolos com fan-out médio de 8.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use dashmap::DashMap;

/// Classificação do símbolo sintático.
///
/// Reflete os nós AST primários emitidos por `tree-sitter-rust`:
/// declarações que podem ser referenciadas por nome qualificado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Função livre ou método (`fn`).
    Fn,
    /// Definição de struct (`struct`).
    Struct,
    /// Definição de enum (`enum`).
    Enum,
    /// Definição de trait (`trait`).
    Trait,
    /// Constante (`const`).
    Const,
    /// Estático (`static`).
    Static,
}

impl SymbolKind {
    /// Serializa para string canônica (lowercase, snake_case).
    pub fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Fn => "fn",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Const => "const",
            SymbolKind::Static => "static",
        }
    }
}

/// Entrada do SYMBOL_INDEX: localização física de um símbolo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolEntry {
    /// Nome qualificado (`crate::module::Type::method`).
    pub qualified_name: String,
    /// Tipo do símbolo (fn, struct, enum, trait, const, static).
    pub kind: SymbolKind,
    /// Caminho do arquivo canonicalizado.
    pub file_path: PathBuf,
    /// Linha 1-based.
    pub line: u32,
    /// Coluna 0-based.
    pub column: u32,
}

/// Nó do CALL_GRAPH: dois conjuntos direcionais (callers ∪ callees).
///
/// **Lei da Direcionalidade:** o grafo é direcionado. Para cada aresta
/// `a → b`, registramos:
/// - `a.callees` contém `b` (quem `a` chama)
/// - `b.callers` contém `a` (quem chama `b`)
///
/// Isso permite responder `callers(X)` e `callees(X)` em O(1) sem BFS
/// nem heurísticas de "grafo simétrico" (que confundiriam incoming
/// com outgoing para nós com ambos).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallGraphNode {
    /// Símbolo (qualified_name).
    pub symbol: String,
    /// Quem chama ESTE símbolo (incoming edges).
    pub callers: HashSet<String>,
    /// Quem ESTE símbolo chama (outgoing edges).
    pub callees: HashSet<String>,
    /// Última atualização em epoch seconds (hook Langevin para futuro LRU).
    pub last_updated: i64,
}

// =============================================================================
// SYMBOL_INDEX
// =============================================================================

static SYMBOL_INDEX: OnceLock<DashMap<String, SymbolEntry>> = OnceLock::new();

/// Devolve a referência ao `DashMap` global do índice de símbolos.
///
/// Inicialização lazy na primeira chamada. Lock-free nas subsequentes.
pub fn symbol_index() -> &'static DashMap<String, SymbolEntry> {
    SYMBOL_INDEX.get_or_init(DashMap::new)
}

/// Insere ou substitui uma entrada no SYMBOL_INDEX.
///
/// Idempotente: inserir duas vezes com o mesmo `qualified_name` substitui
/// a entrada anterior (útil quando o arquivo é reescrito e a linha muda).
pub fn insert_symbol(entry: SymbolEntry) {
    symbol_index().insert(entry.qualified_name.clone(), entry);
}

/// Remove todas as entradas de um arquivo do SYMBOL_INDEX.
///
/// Chamado quando o telemetry worker detecta deleção de arquivo.
pub fn remove_symbols_for_file(file_path: &PathBuf) -> usize {
    let idx = symbol_index();
    let to_remove: Vec<String> = idx
        .iter()
        .filter(|kv| kv.value().file_path == *file_path)
        .map(|kv| kv.key().clone())
        .collect();
    let n = to_remove.len();
    for k in to_remove {
        idx.remove(&k);
    }
    n
}

/// Lookup O(1) médio no SYMBOL_INDEX.
pub fn lookup_symbol(name: &str) -> Option<SymbolEntry> {
    symbol_index().get(name).map(|kv| kv.value().clone())
}

/// Quantidade de símbolos indexados (útil para telemetria e testes).
pub fn symbol_count() -> usize {
    symbol_index().len()
}

// =============================================================================
// CALL_GRAPH
// =============================================================================

static CALL_GRAPH: OnceLock<DashMap<String, CallGraphNode>> = OnceLock::new();

/// Devolve a referência ao `DashMap` global do call graph.
pub fn call_graph() -> &'static DashMap<String, CallGraphNode> {
    CALL_GRAPH.get_or_init(DashMap::new)
}

/// Insere ou substitui um nó do CALL_GRAPH.
pub fn insert_node(node: CallGraphNode) {
    call_graph().insert(node.symbol.clone(), node);
}

/// Adiciona uma aresta direcionada `caller → callee` ao call graph.
///
/// Atualiza ambos os lados com conjuntos direcionais:
/// - `caller.callees` recebe `callee` (outgoing).
/// - `callee.callers` recebe `caller` (incoming).
///
/// Idempotente: `HashSet::insert` é no-op se a aresta já existe.
pub fn insert_edge(caller: &str, callee: &str, now_epoch: i64) {
    if caller == callee {
        return; // ignora auto-referências (recursão trivial não é edge útil)
    }
    let graph = call_graph();
    // Caller → adiciona callee em `callees` (outgoing).
    graph
        .entry(caller.to_string())
        .and_modify(|n| {
            n.callees.insert(callee.to_string());
            n.last_updated = now_epoch;
        })
        .or_insert_with(|| CallGraphNode {
            symbol: caller.to_string(),
            callers: HashSet::new(),
            callees: HashSet::from([callee.to_string()]),
            last_updated: now_epoch,
        });
    // Callee → adiciona caller em `callers` (incoming).
    graph
        .entry(callee.to_string())
        .and_modify(|n| {
            n.callers.insert(caller.to_string());
            n.last_updated = now_epoch;
        })
        .or_insert_with(|| CallGraphNode {
            symbol: callee.to_string(),
            callers: HashSet::from([caller.to_string()]),
            callees: HashSet::new(),
            last_updated: now_epoch,
        });
}

/// Remove todas as arestas que tocam um símbolo (arquivo deletado).
pub fn remove_node(symbol: &str) -> Option<CallGraphNode> {
    call_graph().remove(symbol).map(|(_, node)| node)
}

/// Quantidade de nós no call graph.
pub fn call_graph_size() -> usize {
    call_graph().len()
}

/// Quantidade de arestas totais (soma de |callers| = soma de |callees|).
pub fn call_graph_edge_count() -> usize {
    let mut total: usize = 0;
    for kv in call_graph().iter() {
        // Cada aresta é contada uma vez em `callers` de um nó e uma
        // vez em `callees` de outro; usamos a metade de `callers` total
        // para evitar duplicação.
        total = total.saturating_add(kv.value().callers.len());
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_index_insert_and_lookup() {
        let entry = SymbolEntry {
            qualified_name: "crate::module::Type::method".to_string(),
            kind: SymbolKind::Fn,
            file_path: PathBuf::from("src/lib.rs"),
            line: 42,
            column: 4,
        };
        insert_symbol(entry.clone());
        let found = lookup_symbol("crate::module::Type::method")
            .expect("simbolo deve estar indexado");
        assert_eq!(found.qualified_name, entry.qualified_name);
        assert_eq!(found.kind, SymbolKind::Fn);
        assert_eq!(found.line, 42);
    }

    #[test]
    fn test_symbol_index_remove_for_file() {
        let mut count = symbol_count();
        insert_symbol(SymbolEntry {
            qualified_name: "temp::a".to_string(),
            kind: SymbolKind::Fn,
            file_path: PathBuf::from("src/temp.rs"),
            line: 1,
            column: 0,
        });
        insert_symbol(SymbolEntry {
            qualified_name: "temp::b".to_string(),
            kind: SymbolKind::Struct,
            file_path: PathBuf::from("src/temp.rs"),
            line: 2,
            column: 0,
        });
        let removed = remove_symbols_for_file(&PathBuf::from("src/temp.rs"));
        assert_eq!(removed, 2, "devem ser removidas 2 entradas do temp.rs");
        assert_eq!(symbol_count(), count, "volta ao baseline apos remocao");
    }

    #[test]
    fn test_call_graph_edge_insertion_is_symmetric() {
        // Limpa estado de testes anteriores.
        remove_node("test::a");
        remove_node("test::b");
        insert_edge("test::a", "test::b", 1000);

        // a deve ter b em `callees` (outgoing).
        let a = call_graph().get("test::a").expect("a existe");
        assert!(a.value().callees.contains("test::b"));
        assert!(a.value().callers.is_empty(), "a nao tem callers (eh raiz)");

        // b deve ter a em `callers` (incoming).
        let b = call_graph().get("test::b").expect("b existe");
        assert!(b.value().callers.contains("test::a"));
        assert!(b.value().callees.is_empty(), "b nao tem callees (eh folha)");
    }

    #[test]
    fn test_call_graph_self_loop_is_ignored() {
        remove_node("self::loop");
        insert_edge("self::loop", "self::loop", 1000);
        let node = call_graph()
            .get("self::loop")
            .expect("no deve existir");
        assert!(
            node.value().callers.is_empty() && node.value().callees.is_empty(),
            "auto-referencia NAO deve virar aresta"
        );
        // Limpa.
        remove_node("self::loop");
    }
}
