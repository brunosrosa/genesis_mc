//! Helpers canônicos para testes TDD do domínio socrático.
//!
//! **Marco 3.9.1 (Higiene):** estes helpers foram extraídos do
//! `bin/souls_mcp_server.rs` para garantir uma **Única Fonte de Verdade**
//! (SSOT) na reconstrução de árvores socráticas. Antes desta faxina,
//! o bin tinha 3 cópias paralelas dos helpers (`open_socratic_state_db`,
//! `build_socratic_tree`, `render_socratic_markdown`) que corriam o
//! risco de drift se o schema V5 evoluísse.
//!
//! ## Visibilidade
//!
//! O módulo é `pub` (não `#[cfg(test)]`) para que tanto o bin MCP
//! quanto outros consumidores da lib possam importar os helpers em
//! seus próprios `#[test]`. As funções são leves (pouca lógica,
//! sem I/O async) e o overhead de incluir este módulo na lib é
//! negligível (zero impacto em runtime, só em tempo de compilação).
//!
//! ## Agnosticismo
//!
//! `build_socratic_tree` e `render_socratic_markdown` operam apenas em
//! fatias (`&[SocraticThought]`) — não tocam I/O. Podem ser exercitados
//! em qualquer arquitetura (RTX, M-series, NUMA, WebAssembly) sem custo
//! de portabilidade.

use crate::cognition::thinking::persistence::SocraticThought;
use crate::cognition::thinking::ops;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::path::Path;

/// Helper: abre `souls_state.db` em modo leitura+escrita, garantindo
/// que `.souls_data/` exista (idempotente) e que FKs estejam ON.
///
/// **Versão de teste:** retorna `Connection` (não `Result<Connection,
/// RpcError>` como a versão canônica em [`handlers::handle_export_session`])
/// porque os testes TDD usam o `?` puro sem precisar do `RpcError`.
/// Mapeia erros para `String` para ergonomia de asserções.
pub fn open_socratic_state_db(workspace_root: &Path) -> Result<Connection, String> {
    let souls_data_dir = workspace_root.join(".souls_data");
    std::fs::create_dir_all(&souls_data_dir).map_err(|e| {
        format!(
            "Falha ao criar diretório .souls_data/ ({}): {e}",
            souls_data_dir.display()
        )
    })?;
    let db_path = souls_data_dir.join("souls_state.db");
    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(|e| format!("Falha ao abrir souls_state.db: {e}"))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| format!("Falha ao habilitar FK: {e}"))?;
    let mut conn_mut = conn;
    ops::migrate_v3_to_v5(&mut conn_mut)
        .map_err(|e| format!("Falha na migração V3→V5: {e}"))?;
    Ok(conn_mut)
}

/// Reconstrução iterativa da árvore socrática.
///
/// Devolve `(roots, children_map)` onde:
/// - `roots` = pensamentos sem pai (Tese inicial de cada branch).
/// - `children_map` = `parent_thought_id → Vec<&SocraticThought>`.
///
/// Complexidade: **O(N)** sobre o slice de entrada (uma única passagem).
/// SEM recursão: a renderização é responsabilidade do chamador, que
/// decide se faz DFS manual ou BFS por profundidade.
pub fn build_socratic_tree(
    thoughts: &[SocraticThought],
) -> (Vec<&SocraticThought>, HashMap<&str, Vec<&SocraticThought>>) {
    let mut roots: Vec<&SocraticThought> = Vec::new();
    let mut children: HashMap<&str, Vec<&SocraticThought>> = HashMap::new();
    for t in thoughts {
        match &t.parent_thought_id {
            None => roots.push(t),
            Some(parent_id) => {
                children
                    .entry(parent_id.as_str())
                    .or_default()
                    .push(t);
            }
        }
    }
    // Ordena filhos por (step_number, branch_id) para reconstrução determinística.
    for v in children.values_mut() {
        v.sort_by(|a, b| {
            a.step_number
                .cmp(&b.step_number)
                .then_with(|| a.branch_id.cmp(&b.branch_id))
        });
    }
    roots.sort_by(|a, b| {
        a.branch_id
            .cmp(&b.branch_id)
            .then_with(|| a.step_number.cmp(&b.step_number))
    });
    (roots, children)
}

/// Renderiza a árvore em Markdown com indentação por profundidade.
///
/// **Output canônico para export_session(format="markdown"):**
/// ```text
/// # Socratic Session Tree
///
/// - **Tese** [th_1] step=1 dur=10ms
///   > Conteúdo da tese
///   - **Antitese** [th_2] step=2 dur=15ms
///     > Conteúdo da antítese
/// ```
///
/// DFS pre-order via pilha explícita (não-recursivo, zero-stack-overflow
/// em cadeias profundas).
pub fn render_socratic_markdown(
    roots: &[&SocraticThought],
    children: &HashMap<&str, Vec<&SocraticThought>>,
) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str("# Socratic Session Tree\n\n");
    let mut stack: Vec<(&SocraticThought, usize)> = roots
        .iter()
        .rev() // DFS pre-order: empilha em ordem reversa para que o 1º saia primeiro.
        .map(|t| (*t, 0_usize))
        .collect();
    while let Some((node, depth)) = stack.pop() {
        let indent = "  ".repeat(depth);
        out.push_str(&format!(
            "{indent}- **{}** [{}] step={} dur={}ms\n",
            node.thought_type.as_str(),
            node.thought_id,
            node.step_number,
            node.duration_ms
        ));
        if !node.content.trim().is_empty() {
            // Conteúdo multilinha: indenta cada linha com 2 espaços a mais.
            for line in node.content.lines() {
                out.push_str(&format!("{indent}  > {line}\n"));
            }
        }
        if let Some(kids) = children.get(node.thought_id.as_str()) {
            for k in kids.iter().rev() {
                stack.push((*k, depth + 1));
            }
        }
    }
    out
}
