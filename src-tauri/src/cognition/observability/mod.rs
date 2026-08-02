//! Marco 3.7 (Fase B): Observabilidade Cognitiva Sensorial.
//!
//! Quatro sentidos nativos do SODA, operando 100% em RAM Host + SQLite
//! (`souls_state.db` v3) sem dependencia de CUDA ou Tauri:
//!
//! - [`heatmap`]: Mapeia arquivos quentes via decaimento exponencial Langevin
//!   sobre `file_access_logs` (lambda=0.05, calibrado para meia-vida ≈ 14s).
//! - [`impact`]: Calcula o Blast Radius (importadores afetados) de qualquer
//!   arquivo do monorepo via BFS no grafo transposto de imports.
//! - [`routes`]: Mapeia o contrato de IPC Tauri↔Svelte via varredura regex
//!   de comandos backend e invokes frontend.
//! - [`feedback`]: Dumps FinOps de telemetria local com calculo da eficiencia
//!   E3 (tokens salvos / tokens brutos).
//!
//! Marco 3.8 (Fase C.2): Enjaulamento Wasmtime do Tree-Sitter.
//!
//! - [`wasm_engine`]: Cerca Wasmtime WASI 0.2 com memory limiter 16MiB e
//!   fuel metering 10M. Classifica traps em [`wasm_engine::WasmTrap`].
//! - [`call_graph`]: SYMBOL_INDEX + CALL_GRAPH em DashMap RAM Host. Lookup
//!   O(1) médio para ferramentas `symbol`/`callers`/`callees`.
//! - [`mpsc_telemetry`]: Bridge MPSC fire-and-forget (HIPER-FORWARD) que
//!   alimenta o call graph a partir de mutações de arquivo.
//!
//! Toda a migracao de schema e feita em [`ops::migrate_v2_to_v3`]
//! (idempotente; bumpa `PRAGMA user_version` para 3).

pub mod call_graph;
pub mod feedback;
pub mod heatmap;
pub mod impact;
pub mod mpsc_telemetry;
pub mod ops;
pub mod routes;
pub mod types;
pub mod wasm_engine;

pub use call_graph::{
    call_graph as call_graph_global, call_graph_edge_count, call_graph_size, insert_edge,
    insert_node, insert_symbol, lookup_symbol, remove_node, remove_symbols_for_file,
    symbol_count, symbol_index as symbol_index_global, CallGraphNode, SymbolEntry, SymbolKind,
};
pub use feedback::{TelemetryReport, aggregate_telemetry, e3_efficiency};
pub use heatmap::{HeatmapEntry, compute_heatmap, langevin_score};
pub use impact::{ImpactReport, blast_radius, build_import_graph, impact_report};
pub use mpsc_telemetry::{events_dropped, init_telemetry_worker, try_emit_event, TelemetryEvent};
pub use ops::migrate_v2_to_v3;
pub use routes::{RouteReport, scan_routes};
pub use types::{FileAccessLog, TelemetryLog};
pub use wasm_engine::{
    WasmEngine, WasmMemoryLimiter, WasmTrap, FUEL_LIMIT, MEMORY_LIMIT_BYTES,
};
