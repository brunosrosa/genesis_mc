use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;
use rusqlite::{params, Connection};

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
    souls_mc_lib::persist::google_workspace_mcp::read_values_blocking(
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

#[derive(Default)]
struct CliArgs {
    sheets_id: Option<String>,
    repo_url: Option<String>,
    scan_mojibake: bool,
}

fn parse_cli_args() -> CliArgs {
    let mut args = std::env::args();
    args.next();
    let mut out = CliArgs::default();
    while let Some(arg) = args.next() {
        if arg == "--sheets-id" {
            out.sheets_id = args.next();
            continue;
        }
        if arg == "--repo-url" {
            out.repo_url = args.next();
            continue;
        }
        if arg == "--scan-mojibake" {
            out.scan_mojibake = true;
            continue;
        }
    }
    out
}

fn normalize_repo_url(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn parse_repo_id_from_url(repo_url: &str) -> Option<String> {
    let u = normalize_repo_url(repo_url);
    let marker = "github.com/";
    let idx = u.find(marker)?;
    let tail = &u[(idx + marker.len())..];
    let tail = tail.split('?').next().unwrap_or(tail);
    let tail = tail.split('#').next().unwrap_or(tail);
    let tail = tail.trim().trim_matches('/');
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_string())
    }
}

fn normalize_project_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.contains('/') {
        let parts: Vec<&str> = trimmed.split('/').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 {
            return format!("{}/{}", parts[0], parts[1]);
        }
    }
    trimmed.to_string()
}

fn is_safe_sql_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn open_vault_db(root_dir: &Path) -> io::Result<Connection> {
    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    Connection::open(&db_path).map_err(|e| {
        io::Error::other(format!(
            "Falha ao abrir vault em {}: {}",
            db_path.display(),
            e
        ))
    })
}

fn summarize_scalar(raw: &str) -> String {
    const MAX: usize = 120;
    let trimmed = raw.trim();
    if trimmed.len() > MAX {
        format!("{}...(truncated)", &trimmed[..MAX])
    } else {
        trimmed.to_string()
    }
}

fn looks_like_mojibake(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && (s.contains('Ã') || s.contains('Â'))
}

fn mojibake_probe(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    let mut window: Vec<char> = Vec::new();
    let mut started = false;
    for ch in s.chars() {
        if !started && (ch == 'Ã' || ch == 'Â') {
            started = true;
        }
        if started {
            window.push(ch);
            if window.len() >= 8 {
                break;
            }
        }
    }
    let mut parts: Vec<String> = Vec::new();
    for ch in window {
        parts.push(format!("{:04X}", ch as u32));
    }
    parts.join(" ")
}

fn fix_mojibake_ptbr(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    if !(s.contains('Ã') || s.contains('Â')) {
        return s.to_string();
    }
    let mut out = s.to_string();
    out = out.replace("Â", "");
    for (from, to) in [
        ("Ã\u{00A1}", "á"),
        ("Ã\u{00A0}", "à"),
        ("Ã\u{00A2}", "â"),
        ("Ã\u{00A3}", "ã"),
        ("Ã\u{00A4}", "ä"),
        ("Ã\u{00A9}", "é"),
        ("Ã\u{00A8}", "è"),
        ("Ã\u{00AA}", "ê"),
        ("Ã\u{00AB}", "ë"),
        ("Ã\u{00AD}", "í"),
        ("Ã\u{00AC}", "ì"),
        ("Ã\u{00AE}", "î"),
        ("Ã\u{00AF}", "ï"),
        ("Ã\u{00B3}", "ó"),
        ("Ã\u{00B2}", "ò"),
        ("Ã\u{00B4}", "ô"),
        ("Ã\u{00B5}", "õ"),
        ("Ã\u{00B6}", "ö"),
        ("Ã\u{00BA}", "ú"),
        ("Ã\u{00B9}", "ù"),
        ("Ã\u{00BB}", "û"),
        ("Ã\u{00BC}", "ü"),
        ("Ã\u{00A7}", "ç"),
        ("Ã\u{00B1}", "ñ"),
    ] {
        out = out.replace(from, to);
    }
    out
}

fn main() -> io::Result<()> {
    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let cli = parse_cli_args();
    let spreadsheet_id = cli
        .sheets_id
        .or_else(|| std::env::var("GOOGLE_SHEETS_ID").ok())
        .ok_or_else(|| io::Error::other("Missing GOOGLE_SHEETS_ID (or --sheets-id)"))?;

    let header_range = souls_mc_lib::cognition::synthesizer::master_solutions_header_range();
    let header = read_sheet_values(&spreadsheet_id, "MASTER_SOLUTIONS", &header_range)?;
    let header_row = extract_values_2d(&header)
        .unwrap_or_default()
        .first()
        .cloned()
        .unwrap_or_default();
    let cols = resolve_column_map(&header_row)?;

    let end_col = souls_mc_lib::persist::sheets_utils::col_idx_to_a1(
        souls_mc_lib::cognition::synthesizer::MASTER_SOLUTIONS_CANONICAL_COLUMNS
            .len()
            .saturating_sub(1),
    );
    let data_range = format!("A2:{end_col}");
    let data = read_sheet_values(&spreadsheet_id, "MASTER_SOLUTIONS", &data_range)?;
    let values = extract_values_2d(&data).unwrap_or_default();

    if let Some(repo_url) = cli.repo_url {
        let needle = normalize_repo_url(&repo_url);
        let mut found: Option<(usize, Vec<String>)> = None;
        for (idx0, row) in values.into_iter().enumerate() {
            let url = row.get(cols.repo_url_idx).map(|s| normalize_repo_url(s)).unwrap_or_default();
            if url == needle {
                found = Some((idx0, row));
                break;
            }
        }
        let Some((idx0, row)) = found else {
            return Err(io::Error::other(format!(
                "Repo URL não encontrado em MASTER_SOLUTIONS: {}",
                needle
            )));
        };

        let row_number_1based = (idx0 as u32) + 2;
        let status_atualizacao = row
            .get(cols.status_atualizacao_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let project_name_cell = row
            .get(cols.project_name_idx)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let project_name = if !project_name_cell.trim().is_empty() {
            normalize_project_name(&project_name_cell)
        } else {
            parse_repo_id_from_url(&needle).unwrap_or_default()
        };

        let mut header_map = std::collections::HashMap::<String, usize>::new();
        for (idx, raw) in header_row.iter().enumerate() {
            header_map.insert(normalize_header_cell(raw), idx);
        }
        let status_fase = header_map
            .get("status_fase")
            .and_then(|&idx| row.get(idx))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let canonical_cols = souls_mc_lib::cognition::synthesizer::MASTER_SOLUTIONS_CANONICAL_COLUMNS;
        let mut missing_sheet_cols: Vec<String> = Vec::new();
        let mut missing_header_cols: Vec<String> = Vec::new();
        for col_name in canonical_cols {
            let key = normalize_header_cell(col_name);
            let Some(&idx) = header_map.get(&key) else {
                missing_header_cols.push(key);
                continue;
            };
            let cell = row.get(idx).map(|s| s.trim()).unwrap_or("");
            if cell.is_empty() {
                missing_sheet_cols.push(key);
            }
        }

        println!("row_number={}", row_number_1based);
        println!("status_atualizacao={}", status_atualizacao);
        println!("status_fase={}", status_fase);
        println!("project_name={}", project_name);
        println!("repo_url={}", needle);

        if !missing_header_cols.is_empty() {
            println!("missing_in_header={}", missing_header_cols.join(","));
        }
        if !missing_sheet_cols.is_empty() {
            println!("missing_in_sheet={}", missing_sheet_cols.join(","));
        }

        if cli.scan_mojibake {
            let mut hits: Vec<String> = Vec::new();
            for (idx, raw) in header_row.iter().enumerate() {
                let cell = row.get(idx).map(|s| s.as_str()).unwrap_or("");
                if looks_like_mojibake(cell) {
                    hits.push(format!(
                        "{}={} cp={}",
                        normalize_header_cell(raw),
                        summarize_scalar(cell),
                        mojibake_probe(cell)
                    ));
                }
            }
            if !hits.is_empty() {
                println!("mojibake_in_sheet={}", hits.join(","));
            }
        }

        let conn = open_vault_db(&root_dir)?;
        let repo_status_processamento: Option<String> = conn
            .query_row(
                "SELECT status_processamento FROM repositorios WHERE project_name = ?1 LIMIT 1",
                params![project_name.as_str()],
                |r| r.get(0),
            )
            .ok();
        println!(
            "sqlite.repositorios.status_processamento={}",
            repo_status_processamento.unwrap_or_default()
        );

        let debate_row: Option<(String, i64, i64, i64, String)> = conn
            .query_row(
                "SELECT phase_status,
                        length(lens_a_json) AS a_len,
                        length(lens_b_json) AS b_len,
                        length(lens_c_json) AS c_len,
                        model_used
                 FROM debates_enxame
                 WHERE repo_id = ?1
                 LIMIT 1",
                params![project_name.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .ok();
        if let Some((phase_status, a_len, b_len, c_len, model_used)) = debate_row {
            println!("sqlite.debates_enxame.phase_status={}", phase_status);
            println!("sqlite.debates_enxame.lens_a_len={}", a_len);
            println!("sqlite.debates_enxame.lens_b_len={}", b_len);
            println!("sqlite.debates_enxame.lens_c_len={}", c_len);
            println!("sqlite.debates_enxame.model_used={}", model_used);
        } else {
            println!("sqlite.debates_enxame.missing=1");
        }

        if cli.scan_mojibake {
            let sqlite_row_opt: Option<(String, String)> = conn
                .query_row(
                    "SELECT categoria_nuance_tecnica, must_components_prod_ux FROM repo_heuristics WHERE project_name = ?1 LIMIT 1",
                    params![project_name.as_str()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .ok();
            if let Some((cat, must)) = sqlite_row_opt {
                if looks_like_mojibake(&cat) {
                    let fixed = fix_mojibake_ptbr(&cat);
                    println!(
                        "sqlite.categoria_nuance_tecnica.raw={} fixed={}",
                        summarize_scalar(&cat),
                        summarize_scalar(&fixed)
                    );
                }
                if looks_like_mojibake(&must) {
                    let fixed = fix_mojibake_ptbr(&must);
                    println!(
                        "sqlite.must_components_prod_ux.raw={} fixed={}",
                        summarize_scalar(&must),
                        summarize_scalar(&fixed)
                    );
                }
            }
        }

        let mut cols_to_check: Vec<String> = missing_sheet_cols
            .into_iter()
            .filter(|c| is_safe_sql_ident(c))
            .collect();
        cols_to_check.sort();
        cols_to_check.dedup();
        if !cols_to_check.is_empty() {
            let select_list = cols_to_check
                .iter()
                .map(|c| format!("CAST({} AS TEXT) AS {}", c, c))
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT {} FROM repo_heuristics WHERE project_name = ?1 LIMIT 1",
                select_list
            );
            let row_opt = conn
                .query_row(&query, params![project_name.as_str()], |r| {
                    let mut out = Vec::<(String, String)>::new();
                    for (idx, col) in cols_to_check.iter().enumerate() {
                        let v: String = r.get(idx)?;
                        out.push((col.clone(), v));
                    }
                    Ok(out)
                })
                .ok();
            if let Some(items) = row_opt {
                let mut present = Vec::new();
                let mut absent = Vec::new();
                for (k, v) in items {
                    if v.trim().is_empty() {
                        absent.push(k);
                    } else {
                        present.push(format!("{}={}", k, summarize_scalar(&v)));
                    }
                }
                if !present.is_empty() {
                    println!("sqlite.repo_heuristics.present_for_missing_sheet={}", present.join(","));
                }
                if !absent.is_empty() {
                    println!("sqlite.repo_heuristics.also_missing={}", absent.join(","));
                }
            } else {
                println!("sqlite.repo_heuristics.missing=1");
            }
        }

        return Ok(());
    }

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
