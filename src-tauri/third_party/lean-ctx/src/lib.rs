// ============================================================================
// SOULS-CANIBALIZED: Comentários físicos via "Pessimismo da Razão".
// Tudo abaixo foi CONGELADO e NÃO compila. O código permanece na pasta
// para canibalização futura caso a SSOT do SOULS autorize.
//
// Desplugados nesta fase (F2/Alvo3, branch feat/lean-mcp-integration):
//   - proxy*, http_server, server     -> Conflitam com nosso gateway/TCP
//   - tui, terminal_ui, dashboard     -> ratatui/crossterm (bloat TUI)
//   - cloud_client, cloud_sync        -> Lettre/JSONWebToken (cloud lock-in)
//   - engine, setup, doctor, status   -> Orquestração externa ao MCP
//   - shell, shell_hook, proxy_autostart, proxy_setup
//                                     -> Acoplamento ao proxy HTTP morto
//   - token_report, report, uninstall -> CLI helpers não-MCP
//
// Desplugar é IRREVERSÍVEL sem reabrir a SSOT.
// ============================================================================

pub mod cli; // Mantido: utilitários CLI puros
pub mod compound_lexer;
pub mod config_io;
pub mod core;
pub mod heatmap;
// pub mod hook_handlers; // SOULS-CANIBALIZED Fase 2.5: depende de setup (congelado)
// pub mod hooks; // SOULS-CANIBALIZED Fase 2.5: depende de setup (congelado)
pub mod instructions;
pub mod marked_block;
pub mod mcp_stdio;
pub mod rewrite_registry;
pub mod rules_inject;
// pub mod tool_defs; // SOULS-CANIBALIZED Fase 2.5: congelado (depende de rmcp::model::Tool)
pub mod tools;
