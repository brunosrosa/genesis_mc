pub mod observability;
pub mod repo_heatmap;
pub mod repo_impact;
pub mod souls_symbol;

pub use repo_heatmap::{
    calculate_frecency, compute_langevin_decay, compute_repo_heatmap, compute_repo_heatmap_from_db,
    compute_repo_heatmap_langevin, ensure_heatmap_table, fetch_modification_count,
    open_heatmap_db, record_access, record_access_log, upsert_heatmap_row, HeatmapEntry,
    HeatmapError, HeatmapReport, DEFAULT_LAMBDA, MAX_FILES_SCAN, MAX_SCORE,
};
pub use repo_impact::{
    repo_impact as repo_impact_fn, repo_impact_from_ram, ImpactEdge, ImpactGraphPayload, ImpactReport, RepoImpactError,
    DEFAULT_MAX_DEPTH, MAX_DEPTH_CEILING,
};
pub use souls_symbol::{resolve_symbol, SymbolError, SymbolKind, SymbolLocation};
