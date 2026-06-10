use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn extract_values_2d(result: &Value) -> Option<Vec<Vec<String>>> {
    let values = if let Some(values) = result.get("values").and_then(|v| v.as_array()) {
        values
    } else {
        let vr = result.get("valueRanges")?.as_array()?;
        let first = vr.first()?;
        first.get("values")?.as_array()?
    };
    let mut out = Vec::with_capacity(values.len());
    for row in values {
        let Some(cells) = row.as_array() else {
            out.push(Vec::new());
            continue;
        };
        out.push(
            cells
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect(),
        );
    }
    Some(out)
}

fn normalize_header_cell(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
}

fn read_sheet_values(spreadsheet_id: &str, sheet: &str, range: &str) -> io::Result<Value> {
    genesis_mc_lib::persist::google_workspace_mcp::read_values_blocking(
        spreadsheet_id,
        sheet,
        range,
        "f-queue-audit-dry-run",
        std::time::Duration::from_secs(180),
    )
    .map_err(io::Error::other)
}

#[derive(Debug)]
struct ColumnMap {
    status_atualizacao_idx: usize,
    project_name_idx: usize,
    repo_url_idx: usize,
}

fn resolve_column_map(header_row: &[String]) -> io::Result<ColumnMap> {
    let mut status_atualizacao_idx = None;
    let mut project_name_idx = None;
    let mut repo_url_idx = None;
    for (idx, raw) in header_row.iter().enumerate() {
        let h = normalize_header_cell(raw);
        match h.as_str() {
            "status_atualizacao" => status_atualizacao_idx = Some(idx),
            "project_name" => project_name_idx = Some(idx),
            "repo_url" => repo_url_idx = Some(idx),
            _ => {}
        }
    }
    Ok(ColumnMap {
        status_atualizacao_idx: status_atualizacao_idx
            .ok_or_else(|| io::Error::other("Cabeçalho sem status_atualizacao"))?,
        project_name_idx: project_name_idx.ok_or_else(|| io::Error::other("Cabeçalho sem project_name"))?,
        repo_url_idx: repo_url_idx.ok_or_else(|| io::Error::other("Cabeçalho sem repo_url"))?,
    })
}

fn parse_cli_args() -> Option<String> {
    let mut args = std::env::args();
    args.next();
    while let Some(arg) = args.next() {
        if arg == "--sheets-id" {
            return args.next();
        }
    }
    None
}

fn main() -> io::Result<()> {
    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let spreadsheet_id = parse_cli_args()
        .or_else(|| std::env::var("GOOGLE_SHEETS_ID").ok())
        .ok_or_else(|| io::Error::other("Missing GOOGLE_SHEETS_ID (or --sheets-id)"))?;

    let header_range = genesis_mc_lib::cognition::synthesizer::master_solutions_header_range();
    let header = read_sheet_values(&spreadsheet_id, "MASTER_SOLUTIONS", &header_range)?;
    let header_row = extract_values_2d(&header)
        .unwrap_or_default()
        .first()
        .cloned()
        .unwrap_or_default();
    let cols = resolve_column_map(&header_row)?;

    let end_col = genesis_mc_lib::persist::sheets_utils::col_idx_to_a1(
        genesis_mc_lib::cognition::synthesizer::MASTER_SOLUTIONS_CANONICAL_COLUMNS
            .len()
            .saturating_sub(1),
    );
    let data_range = format!("A2:{end_col}");
    let data = read_sheet_values(&spreadsheet_id, "MASTER_SOLUTIONS", &data_range)?;
    let values = extract_values_2d(&data).unwrap_or_default();

    for row in values {
        let status = row
            .get(cols.status_atualizacao_idx)
            .map(|s| s.trim())
            .unwrap_or("");
        if status != "PENDENTE_FASE_0" && status != "PENDENTE_IA" {
            continue;
        }
        let name = row
            .get(cols.project_name_idx)
            .map(|s| s.trim())
            .unwrap_or("");
        let url = row.get(cols.repo_url_idx).map(|s| s.trim()).unwrap_or("");
        println!("{} | {} | {}", status, name, url);
    }

    Ok(())
}
