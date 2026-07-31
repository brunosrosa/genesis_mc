use std::sync::Arc;

use rmcp::model::*;
use serde_json::{json, Map, Value};

mod granular;
pub use granular::{granular_tool_defs, list_all_tool_defs, unified_tool_defs};

pub fn extract_action(name: &str) -> &str {
    let mut s = name;
    loop {
        if let Some(rest) = s.strip_prefix("souls_ctx_") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("lean_ctx_") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("souls_") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("ctx_") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("lean_") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("lean-") {
            s = rest;
        } else {
            break;
        }
    }
    s
}

pub fn canonical_tool_name(name: &str) -> String {
    extract_action(name).to_string()
}

pub fn tool_def(name: &'static str, description: &'static str, schema_value: Value) -> Tool {
    let schema: Map<String, Value> = match schema_value {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    let souls_name = canonical_tool_name(name);
    Tool::new(souls_name, description, Arc::new(schema))
}

pub fn lazy_tool_defs() -> Vec<Tool> {
    let all = granular_tool_defs();
    let mut core: Vec<Tool> = all
        .into_iter()
        .filter(|t| {
            let action = extract_action(&t.name);
            matches!(
                action,
                "read" | "multi_read" | "shell" | "search" | "tree" | "edit" | "session" | "knowledge"
            )
        })
        .collect();

    core.push(tool_def(
        "discover_tools",
        "Search available SOULS context tools by keyword. Returns matching tool names + descriptions for on-demand loading.",
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search keyword (e.g. 'graph', 'cost', 'workflow', 'dedup')"
                }
            },
            "required": ["query"]
        }),
    ));

    core
}

pub fn discover_tools(query: &str) -> String {
    let all = list_all_tool_defs();
    let query_lower = query.to_lowercase();
    let matches: Vec<(&str, &str)> = all
        .iter()
        .filter(|(name, desc, _)| {
            name.to_lowercase().contains(&query_lower) || desc.to_lowercase().contains(&query_lower)
        })
        .map(|(name, desc, _)| (*name, *desc))
        .collect();

    if matches.is_empty() {
        return format!("No tools found matching '{query}'. Try broader terms like: graph, cost, session, search, compress, agent, workflow, gain.");
    }

    let mut out = format!("{} tools matching '{query}':\n", matches.len());
    for (name, desc) in &matches {
        let short = if desc.len() > 80 { &desc[..80] } else { desc };
        out.push_str(&format!("  {name} — {short}\n"));
    }
    out.push_str("\nCall the tool directly by name to use it.");
    out
}

pub fn is_lazy_mode() -> bool {
    std::env::var("LEAN_CTX_LAZY_TOOLS").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_action() {
        assert_eq!(extract_action("read"), "read");
        assert_eq!(extract_action("ctx_read"), "read");
        assert_eq!(extract_action("souls_ctx_read"), "read");
        assert_eq!(extract_action("souls_ctx_souls_ctx_read"), "read");
        assert_eq!(extract_action("lean_ctx_search"), "search");
        assert_eq!(extract_action("lean_ctx_lean_ctx_search"), "search");
        assert_eq!(extract_action("souls_ctx_session"), "session");
        assert_eq!(extract_action("ctx_multi_read"), "multi_read");
    }

    #[test]
    fn test_canonical_tool_name_idempotency() {
        assert_eq!(canonical_tool_name("read"), "read");
        assert_eq!(canonical_tool_name("ctx_read"), "read");
        assert_eq!(canonical_tool_name("souls_ctx_read"), "read");
        assert_eq!(canonical_tool_name("souls_ctx_souls_ctx_read"), "read");
        assert_eq!(canonical_tool_name("lean_ctx_search"), "search");
        assert_eq!(canonical_tool_name("souls_ctx_souls_ctx_souls_ctx_tree"), "tree");
    }

    #[test]
    fn test_all_granular_tools_have_clean_canonical_names() {
        let tools = granular_tool_defs();
        println!("\n=== EXPORTED MCP TOOL NAMES (CLEAN LISTING) ===");
        for tool in tools {
            println!("  • {}", tool.name);
            assert!(
                !tool.name.starts_with("souls_ctx_"),
                "Tool {} should not have souls_ctx_ prefix internally in lean-ctx server because agentgateway prepends target name!",
                tool.name
            );
            assert!(
                !tool.name.contains("_ctx_ctx_"),
                "Tool {} has double ctx prefix!",
                tool.name
            );
        }
        println!("================================================\n");
    }
}

