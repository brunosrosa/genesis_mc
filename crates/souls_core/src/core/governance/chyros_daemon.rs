// SOULS V6 MARCO 5.7.0 — Core Chyros Daemon Module (AutoDream & Metabolismo Estocástico)
// Conforme ADR-001, ADR-005, ADR-027 e Marco VI.

pub use crate::cognition::memory::chyros_daemon::{
    ActivityTracker, ChyrosDaemon, ConsolidationReport,
};
pub use crate::cognition::memory::langevin_decay::{
    apply_langevin_decay, compute_langevin_score, proj_poincare, PoincareVector,
};
