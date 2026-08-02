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
//! Toda a migracao de schema e feita em [`ops::migrate_v2_to_v3`]
//! (idempotente; bumpa `PRAGMA user_version` para 3).

pub mod feedback;
pub mod heatmap;
pub mod impact;
pub mod ops;
pub mod routes;
pub mod types;

pub use feedback::{TelemetryReport, aggregate_telemetry, e3_efficiency, e3_efficiency_v2};
pub use heatmap::{HeatmapEntry, compute_heatmap, langevin_score};
pub use impact::{ImpactReport, blast_radius, build_import_graph, impact_report};
pub use ops::{migrate_v2_to_v3, migrate_v3_to_v4};
pub use routes::{RouteReport, scan_routes};
pub use types::{FileAccessLog, TelemetryLog};
