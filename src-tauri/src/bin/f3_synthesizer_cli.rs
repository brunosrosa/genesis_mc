use std::io;
use std::io::IsTerminal;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{FixedOffset, Utc};
use genesis_mc_lib::cognition::synthesizer::{
    run_phase3_sgr, Block0Context, Phase3Config, Phase3Error, OFFICIAL_FORMATTER_MODEL,
};
use genesis_mc_lib::finops::finops_router::{FinOpsRouter, RoutingDestination};
use genesis_mc_lib::harvester::community::{CommunityMetaFetcher, RateLimiter};
use genesis_mc_lib::persist::ssot_injector::SsotInjector;
use genesis_mc_lib::persist::sheets_utils::{col_idx_to_a1, extract_values_2d_strict, find_col_idx};
use reqwest::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{error, info, warn};
use url::Url;

#[cfg(not(test))]
const MCP_TIMEOUT: Duration = Duration::from_secs(180);
#[cfg(test)]
const MCP_TIMEOUT: Duration = Duration::from_millis(250);

#[cfg(not(test))]
const FORMATTER_HTTP_TIMEOUT: Duration = Duration::from_secs(300);
#[cfg(test)]
const FORMATTER_HTTP_TIMEOUT: Duration = Duration::from_millis(400);

#[cfg(not(test))]
const SGR_TOTAL_TIMEOUT: Duration = Duration::from_secs(1800);
#[cfg(test)]
const SGR_TOTAL_TIMEOUT: Duration = Duration::from_millis(400);

#[cfg(not(test))]
const GITHUB_HTTP_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
const GITHUB_HTTP_TIMEOUT: Duration = Duration::from_millis(250);

async fn retry_backoff_sleep(attempt: u32, max_attempts: u32, base_ms: u64) -> bool {
    if attempt >= max_attempts {
        return false;
    }
    let jitter_ms = (Utc::now().timestamp_subsec_millis() % 250) as u64;
    let exp = attempt.saturating_sub(1).min(10);
    let delay_ms = base_ms.saturating_mul(1_u64 << exp).saturating_add(jitter_ms);
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    true
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn spawn_ghost_telemetry(repo_id: String, message: String) -> AbortOnDrop {
    let started = Instant::now();
    let handle = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            info!(
                repo_id = %repo_id,
                elapsed_s = started.elapsed().as_secs(),
                message = %message,
                "Ghost Telemetry"
            );
        }
    });
    AbortOnDrop(handle)
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn now_epoch_secs() -> io::Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("Falha ao calcular timestamp atual: {}", e)))?
        .as_secs() as i64)
}

fn count_raw_blobs_distinct(conn: &Connection, repo_id: &str) -> io::Result<usize> {
    let mut stmt = conn
        .prepare("SELECT COUNT(DISTINCT artifact_type) FROM artefatos_brutos WHERE repo_id = ?1")
        .map_err(|e| io::Error::other(format!("Falha ao preparar query blobs: {}", e)))?;
    let count: i64 = stmt
        .query_row([repo_id], |row| row.get(0))
        .map_err(|e| io::Error::other(format!("Falha ao consultar blobs: {}", e)))?;
    Ok(count.max(0) as usize)
}

async fn read_master_header(spreadsheet_id: &str) -> io::Result<Vec<String>> {
    let header_range = genesis_mc_lib::cognition::synthesizer::master_solutions_header_range();
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": header_range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d_strict(&result).map_err(io::Error::other)?;
    Ok(values.first().cloned().unwrap_or_default())
}

async fn read_cell_at(
    spreadsheet_id: &str,
    row_number_1based: u32,
    col_idx0: usize,
) -> io::Result<String> {
    let col = col_idx_to_a1(col_idx0);
    let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d_strict(&result).map_err(io::Error::other)?;
    Ok(values
        .first()
        .and_then(|r| r.first())
        .map(|s| s.trim().to_string())
        .unwrap_or_default())
}

fn try_extract_repo_id_from_repo_url(repo_url: &str) -> Option<String> {
    let url = Url::parse(repo_url).ok()?;
    let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
    if url.host_str() != Some("github.com") && !allow_host_override {
        return None;
    }
    let mut segments = url
        .path_segments()?
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches(".git"))
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return None;
    }
    let repo = segments.pop()?;
    let owner = segments.pop()?;
    Some(format!("{owner}/{repo}"))
}

#[derive(Debug, Clone)]
struct BatchCandidate {
    repo_id: String,
    row_number_1based: u32,
}

async fn fetch_enxame_batch_candidates(spreadsheet_id: &str) -> io::Result<Vec<BatchCandidate>> {
    let header_row = read_master_header(spreadsheet_id).await?;
    let status_idx = find_col_idx(&header_row, "status_atualizacao")
        .ok_or_else(|| io::Error::other("Header missing status_atualizacao"))?;
    let repo_url_idx = find_col_idx(&header_row, "repo_url")
        .ok_or_else(|| io::Error::other("Header missing repo_url"))?;
    let lote_idx = find_col_idx(&header_row, "lote_id");

    let required = [status_idx, repo_url_idx];
    let min_idx = *required.iter().min().unwrap_or(&0);
    let max_idx = *required.iter().max().unwrap_or(&0);
    let start_col = col_idx_to_a1(min_idx);
    let end_col = col_idx_to_a1(max_idx.max(lote_idx.unwrap_or(0)));

    let range = format!("{start_col}2:{end_col}");
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d_strict(&result).map_err(io::Error::other)?;

    let lote_override = std::env::var("SODA_LOTE_ID_OVERRIDE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let mut out = Vec::new();
    for (idx, row) in values.into_iter().enumerate() {
        let row_number_1based = (idx as u32) + 2;
        let get = |abs_idx: usize| -> String {
            let rel = abs_idx.saturating_sub(min_idx);
            row.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        let status = get(status_idx);
        if status.trim() != "APROVADO_PARA_ENXAME" {
            continue;
        }
        if let Some(ref lote_expected) = lote_override {
            let Some(lote_idx) = lote_idx else { continue };
            let lote = get(lote_idx);
            if lote.trim() != lote_expected {
                continue;
            }
        }
        let repo_url = get(repo_url_idx);
        let Some(repo_id) = try_extract_repo_id_from_repo_url(&repo_url) else {
            continue;
        };
        out.push(BatchCandidate {
            repo_id,
            row_number_1based,
        });
    }
    out.sort_by(|a, b| a.repo_id.cmp(&b.repo_id).then_with(|| a.row_number_1based.cmp(&b.row_number_1based)));
    out.dedup_by(|a, b| a.repo_id == b.repo_id && a.row_number_1based == b.row_number_1based);
    Ok(out)
}

async fn try_fetch_github_latest_release_tag(repo_url: &str) -> Option<String> {
    let url = Url::parse(repo_url).ok()?;
    let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
    if url.host_str() != Some("github.com") && !allow_host_override {
        return None;
    }
    let mut segments = url
        .path_segments()
        .map(|parts| parts.collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.trim_end_matches(".git").to_string())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return None;
    }
    let repo = segments.pop()?;
    let owner = segments.pop()?;

    let base = std::env::var("SODA_GITHUB_API_BASE_URL").unwrap_or_else(|_| "https://api.github.com".to_string());
    let endpoint = format!("{}/repos/{owner}/{repo}/releases/latest", base.trim_end_matches('/'));

    #[derive(Deserialize)]
    struct GithubRelease {
        tag_name: Option<String>,
    }

    let client = Client::builder()
        .user_agent("f3-synthesizer-cli/1.0")
        .build()
        .ok()?;
    let resp = tokio::time::timeout(GITHUB_HTTP_TIMEOUT, client.get(&endpoint).send())
        .await
        .ok()?
        .ok()?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return None;
    }
    if !resp.status().is_success() {
        return None;
    }
    let release = tokio::time::timeout(GITHUB_HTTP_TIMEOUT, resp.json::<GithubRelease>())
        .await
        .ok()?
        .ok()?;
    release
        .tag_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn sanitize_repo_id(repo_id: &str) -> String {
    repo_id
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn now_brt_rfc3339() -> String {
    Utc::now()
        .with_timezone(&FixedOffset::west_opt(3 * 3600).unwrap())
        .to_rfc3339()
}

fn etl_report_path(root_dir: &Path, repo_id: &str) -> io::Result<PathBuf> {
    let dir = root_dir.join(".soda_scratchpad").join("reports");
    std::fs::create_dir_all(&dir)
        .map_err(|e| io::Error::other(format!("Falha ao criar reports_dir: {}", e)))?;

    let trimmed = repo_id.trim();
    let mut parts = trimmed.split('/').map(|s| s.trim()).filter(|s| !s.is_empty());
    let owner = parts.next().unwrap_or(trimmed);
    let repo = parts.next().unwrap_or(trimmed);
    Ok(dir.join(format!(
        "_ETL_REPORT_{}_{}.txt",
        sanitize_repo_id(owner),
        sanitize_repo_id(repo)
    )))
}

fn extract_total_cost_usd_from_lens_json(lens_json: &str) -> f64 {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(lens_json) else {
        return 0.0;
    };
    value.get("total_cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0)
}

#[derive(Debug, Clone)]
struct CliArgs {
    repo_id: String,
    e2e_full: bool,
    skip_harvester: bool,
    batch: bool,
    resume_f3: bool,
    row_override: Option<u32>,
    dry_run: bool,
    feedback_inject: bool,
    phase4_only: bool,
}

fn parse_cli_args() -> CliArgs {
    let mut args = std::env::args();
    args.next();
    let mut repo_id = String::from("aaif-goose/goose");
    let mut e2e_full = false;
    let mut skip_harvester = false;
    let mut batch = false;
    let mut resume_f3 = false;
    let mut row_override: Option<u32> = None;
    let mut dry_run = false;
    let mut feedback_inject = false;
    let mut phase4_only = false;
    while let Some(arg) = args.next() {
        if arg == "--repo" {
            if let Some(value) = args.next() {
                repo_id = value;
            }
            continue;
        }
        if arg == "--e2e-full" {
            e2e_full = true;
            continue;
        }
        if arg == "--skip-harvester" {
            skip_harvester = true;
            continue;
        }
        if arg == "--phase4-only" {
            phase4_only = true;
            continue;
        }
        if arg == "--batch" {
            batch = true;
            continue;
        }
        if arg == "--resume-f3" {
            resume_f3 = true;
            continue;
        }
        if arg == "--dry-run" {
            dry_run = true;
            continue;
        }
        if arg == "--feedback-inject" {
            feedback_inject = true;
            continue;
        }
        if arg == "--row" {
            if let Some(value) = args.next() {
                row_override = value.trim().parse::<u32>().ok().filter(|v| *v >= 2);
            }
            continue;
        }
    }
    CliArgs {
        repo_id,
        e2e_full,
        skip_harvester,
        batch,
        resume_f3,
        row_override,
        dry_run,
        feedback_inject,
        phase4_only,
    }
}

async fn fetch_resume_f3_candidates(spreadsheet_id: &str) -> io::Result<Vec<BatchCandidate>> {
    let header_row = read_master_header(spreadsheet_id).await?;
    let status_idx = find_col_idx(&header_row, "status_atualizacao")
        .ok_or_else(|| io::Error::other("Header missing status_atualizacao"))?;
    let fase_idx =
        find_col_idx(&header_row, "status_fase").ok_or_else(|| io::Error::other("Header missing status_fase"))?;
    let repo_url_idx = find_col_idx(&header_row, "repo_url")
        .ok_or_else(|| io::Error::other("Header missing repo_url"))?;
    let lote_idx = find_col_idx(&header_row, "lote_id");

    let required = [status_idx, fase_idx, repo_url_idx];
    let min_idx = *required.iter().min().unwrap_or(&0);
    let max_idx = *required.iter().max().unwrap_or(&0);
    let start_col = col_idx_to_a1(min_idx);
    let end_col = col_idx_to_a1(max_idx.max(lote_idx.unwrap_or(0)));

    let range = format!("{start_col}2:{end_col}");
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d_strict(&result).map_err(io::Error::other)?;

    let lote_override = std::env::var("SODA_LOTE_ID_OVERRIDE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let mut out = Vec::new();
    for (idx, row) in values.into_iter().enumerate() {
        let row_number_1based = (idx as u32) + 2;
        let get = |abs_idx: usize| -> String {
            let rel = abs_idx.saturating_sub(min_idx);
            row.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        let status = get(status_idx);
        if status.trim() != "APROVADO_PARA_ENXAME" {
            continue;
        }
        let status_fase = get(fase_idx);
        let status_fase_ok = matches!(
            status_fase.trim(),
            "FASE_2_ENXAME_OK" | "FASE_3_SINTETIZADOR_OK" | "ERRO_FASE_4"
        );
        if !status_fase_ok {
            continue;
        }
        if let Some(ref lote_expected) = lote_override {
            let Some(lote_idx) = lote_idx else { continue };
            let lote = get(lote_idx);
            if lote.trim() != lote_expected {
                continue;
            }
        }
        let repo_url = get(repo_url_idx);
        let Some(repo_id) = try_extract_repo_id_from_repo_url(&repo_url) else {
            continue;
        };
        out.push(BatchCandidate {
            repo_id,
            row_number_1based,
        });
    }
    out.sort_by(|a, b| a.repo_id.cmp(&b.repo_id).then_with(|| a.row_number_1based.cmp(&b.row_number_1based)));
    out.dedup_by(|a, b| a.repo_id == b.repo_id && a.row_number_1based == b.row_number_1based);
    Ok(out)
}

fn feedback_bmad_report_path(root_dir: &Path) -> io::Result<PathBuf> {
    let reports_dir = root_dir.join(".soda_scratchpad").join("reports");
    std::fs::create_dir_all(&reports_dir)?;
    Ok(reports_dir.join("_FEEDBACK_BMAD_E2E.md"))
}

fn append_feedback_bmad_report(
    root_dir: &Path,
    repo_id: &str,
    payload: &serde_json::Value,
) -> io::Result<PathBuf> {
    let report_path = feedback_bmad_report_path(root_dir)?;
    let now = now_brt_rfc3339();
    let json = serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string());
    let mut out = String::new();
    out.push_str(&format!("\n\n## BMAD E2E ({})\n\n", now));
    out.push_str(&format!("repo_id={}\n\n", repo_id));
    out.push_str("```json\n");
    out.push_str(&json);
    out.push_str("\n```\n");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&report_path)?;
    file.write_all(out.as_bytes())?;
    Ok(report_path)
}

fn extract_json_blocks_from_feedback(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    let mut current = String::new();
    for line in text.lines() {
        let trimmed = line.trim_end_matches('\r');
        if !in_block {
            if trimmed.trim() == "```json" {
                in_block = true;
                current.clear();
            }
            continue;
        }

        if trimmed.trim() == "```" {
            let candidate = current.trim().to_string();
            if !candidate.is_empty() {
                out.push(candidate);
            }
            in_block = false;
            current.clear();
            continue;
        }
        current.push_str(trimmed);
        current.push('\n');
    }
    out
}

fn select_approved_feedback_payload(
    repo_id: &str,
    blocks: &[serde_json::Value],
) -> Option<serde_json::Value> {
    let mut best: Option<(i32, usize)> = None;
    for (idx, block) in blocks.iter().enumerate() {
        let Some(obj_repo) = block.get("repo_id").and_then(|v| v.as_str()) else {
            continue;
        };
        if obj_repo.trim() != repo_id.trim() {
            continue;
        }
        let mut score = 0_i32;
        if let Some(row) = block.get("row").and_then(|v| v.as_object()) {
            let ct = row
                .get("classificacao_terminal")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase();
            let acao = row
                .get("acao_de_canibalizacao")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase();
            if ct.starts_with("APROVADO") {
                score += 10;
            }
            if acao != "NENHUMA" && !acao.is_empty() {
                score += 5;
            }
        }
        match best {
            None => best = Some((score, idx)),
            Some((best_score, _)) if score > best_score => best = Some((score, idx)),
            _ => {}
        }
    }
    best.map(|(_, idx)| blocks[idx].clone())
}

fn update_local_status_after_manual_f4(conn: &Connection, repo_id: &str) -> io::Result<()> {
    let now = now_epoch_secs()?;
    let _ = conn.execute(
        "UPDATE repositorios
         SET status_processamento = ?1,
             timestamp_fase_1 = COALESCE(NULLIF(timestamp_fase_1, 0), ?2)
         WHERE project_name = ?3",
        rusqlite::params!["CONCLUIDO", now, repo_id],
    );
    let _ = conn.execute(
        "UPDATE repo_heuristics
         SET status_atualizacao = ?2,
             status_fase = ?3
         WHERE project_name = ?1",
        rusqlite::params![repo_id, "CONCLUIDO_AGUARDANDO", "FASE_4_SHEETS_UPDATED"],
    );
    Ok(())
}

fn build_dynamic_sheet_ranges_for_row(
    row_number_1based: u32,
    header_row: &[String],
    row: &genesis_mc_lib::cognition::synthesizer::MasterSolutionsRow,
) -> serde_json::Map<String, serde_json::Value> {
    let mut ranges = serde_json::Map::new();
    let canonical_cols = genesis_mc_lib::cognition::synthesizer::MASTER_SOLUTIONS_CANONICAL_COLUMNS;
    let canonical_values = row.to_sheet_row();
    let mut by_name: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (idx, name) in canonical_cols.iter().enumerate() {
        let v = canonical_values
            .get(idx)
            .cloned()
            .unwrap_or_else(|| serde_json::Value::String(String::new()));
        let cell = match v {
            serde_json::Value::Null => String::new(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s,
            other => other.to_string(),
        };
        by_name.insert(
            genesis_mc_lib::persist::sheets_utils::normalize_header_cell(name),
            cell,
        );
    }

    for (idx, header) in header_row.iter().enumerate() {
        let key_norm = genesis_mc_lib::persist::sheets_utils::normalize_header_cell(header);
        if key_norm.is_empty() {
            continue;
        }
        if let Some(value) = by_name.get(&key_norm).cloned() {
            let col = col_idx_to_a1(idx);
            let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
            ranges.insert(range, serde_json::json!(vec![vec![value]]));
        }
    }

    ranges
}

fn get_first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
    })
}

#[derive(Debug, Default, Clone)]
struct UsageTotals {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
}

struct OpenRouterFormatterClient {
    client: Client,
    base_url: String,
    api_key: String,
    usage: std::sync::Arc<std::sync::Mutex<UsageTotals>>,
}

impl OpenRouterFormatterClient {
    fn from_env() -> Result<Self, String> {
        let api_key = get_first_env(&[
            "OPENROUTER_API_HEAVY_KEY",
            "OPENROUTER_API_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
        ])
        .ok_or_else(|| "OPENROUTER_API_HEAVY_KEY/OPENROUTER_API_KEY/OPENROUTER_API_FAST_KEY/OPENROUTER_API_FREE_KEY ausente".to_string())?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .map(|base| format!("{}/chat/completions", base.trim_end_matches('/')))
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1/chat/completions".to_string());

        Ok(Self {
            client: Client::new(),
            base_url,
            api_key,
            usage: std::sync::Arc::new(std::sync::Mutex::new(UsageTotals::default())),
        })
    }

    fn usage_totals(&self) -> UsageTotals {
        match self.usage.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => UsageTotals::default(),
        }
    }

        fn extract_openrouter_content(json: &Value) -> Option<String> {
        fn flatten(value: &Value) -> Option<String> {
            match value {
                Value::String(text) => {
                    let t = text.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                }
                Value::Array(parts) => {
                    let mut out = Vec::new();
                    for part in parts {
                        if let Some(text) = flatten(part) {
                            out.push(text);
                            continue;
                        }
                        if let Some(obj) = part.as_object() {
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                            if let Some(text) = obj.get("content").and_then(|v| v.as_str()) {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                            if let Some(text) = obj
                                .get("text")
                                .and_then(|v| v.get("value"))
                                .and_then(|v| v.as_str())
                            {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                            if let Some(text) = obj
                                .get("content")
                                .and_then(|v| v.get("value"))
                                .and_then(|v| v.as_str())
                            {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                        }
                    }
                    let joined = out.join("\n");
                    if joined.trim().is_empty() {
                        None
                    } else {
                        Some(joined)
                    }
                }
                Value::Object(obj) => {
                    if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                        let t = text.trim();
                        if !t.is_empty() {
                            return Some(t.to_string());
                        }
                    }
                    if let Some(text) = obj.get("content").and_then(|v| v.as_str()) {
                        let t = text.trim();
                        if !t.is_empty() {
                            return Some(t.to_string());
                        }
                    }
                    if obj.is_empty() {
                        return None;
                    }
                    serde_json::to_string(obj).ok().and_then(|s| {
                        let t = s.trim();
                        if t.is_empty() { None } else { Some(t.to_string()) }
                    })
                }
                _ => None,
            }
        }

        let choices = json.get("choices")?.as_array()?;
        let first = choices.first()?;
        if let Some(message) = first.get("message") {
            if let Some(content) = message.get("content") {
                if let Some(text) = flatten(content) {
                    return Some(text);
                }
            }
            if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
                if let Some(first_call) = tool_calls.first() {
                    if let Some(args) = first_call
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|a| a.as_str())
                    {
                        let t = args.trim();
                        if !t.is_empty() {
                            return Some(t.to_string());
                        }
                    }
                }
            }
            if let Some(args) = message
                .get("function_call")
                .and_then(|f| f.get("arguments"))
                .and_then(|a| a.as_str())
            {
                let t = args.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
        }

        first.get("text").and_then(flatten)
    }

    fn harvest_usage(&self, json: &Value) {
        let usage = &json["usage"];
        let prompt = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let completion = usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cost = usage
            .get("total_cost")
            .or_else(|| usage.get("cost"))
            .or_else(|| usage.get("estimated_cost"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if let Ok(mut guard) = self.usage.lock() {
            guard.prompt_tokens = guard.prompt_tokens.saturating_add(prompt);
            guard.completion_tokens = guard.completion_tokens.saturating_add(completion);
            guard.total_tokens = guard.total_tokens.saturating_add(total);
            guard.total_cost_usd += cost;
        }
    }
}

fn parse_block_from_prompt(prompt: &str) -> Option<u8> {
    let first = prompt.lines().next()?.trim();
    let value = first.strip_prefix("BLOCK=")?.trim();
    match value {
        "2A" => Some(21),
        "2B" => Some(22),
        _ => value.parse::<u8>().ok(),
    }
}

fn response_format_for_block(block: u8) -> Value {
    fn strict_object(properties: serde_json::Map<String, Value>, required: Vec<&'static str>) -> Value {
        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": required,
            "additionalProperties": false
        })
    }

    fn string_schema(max_len: u32) -> Value {
        json!({ "type": "string", "maxLength": max_len })
    }

    fn string_array_schema(min_items: u32, max_items: u32, item_max_len: u32) -> Value {
        json!({
            "type": "array",
            "minItems": min_items,
            "maxItems": max_items,
            "items": { "type": "string", "maxLength": item_max_len }
        })
    }

    fn enum_schema(options: &[&str]) -> Value {
        json!({ "type": "string", "enum": options })
    }

    fn int_0_10_schema() -> Value {
        json!({ "type": "integer", "minimum": 0, "maximum": 10 })
    }

    fn envelope_fields_only(fields_schema: Value) -> Value {
        let mut props = serde_json::Map::new();
        props.insert("fields".to_string(), fields_schema);
        strict_object(props, vec!["fields"])
    }

    fn envelope_with_justifications(fields_schema: Value) -> Value {
        let mut props = serde_json::Map::new();
        props.insert("fields".to_string(), fields_schema);
        props.insert(
            "justifications".to_string(),
            json!({
                "type": "object",
                "additionalProperties": { "type": "string", "maxLength": 5000 }
            }),
        );
        strict_object(props, vec!["fields", "justifications"])
    }

    let fields_schema = match block {
        1 => {
            let mut props = serde_json::Map::new();
            props.insert("proposta_original_resumo".to_string(), string_schema(5000));
            props.insert("declared_description_ptbr".to_string(), string_schema(5000));
            props.insert("visao_do_enxame".to_string(), string_schema(5000));
            props.insert("justificativa_decisao".to_string(), string_schema(5000));
            props.insert("executive_verdict".to_string(), string_schema(5000));
            props.insert("risco_principal".to_string(), string_schema(5000));
            props.insert("risco_linha_vermelha".to_string(), string_schema(5000));
            props.insert("observacoes".to_string(), string_schema(5000));
            strict_object(
                props,
                vec![
                    "declared_description_ptbr",
                    "visao_do_enxame",
                    "justificativa_decisao",
                    "executive_verdict",
                    "risco_principal",
                    "risco_linha_vermelha",
                    "observacoes",
                ],
            )
        }
        21 => {
            let mut props = serde_json::Map::new();
            props.insert("indicacao_otimista_canibalizacao".to_string(), string_schema(5000));
            props.insert("ouro_a_extrair".to_string(), string_schema(5000));
            props.insert("deep_pattern".to_string(), string_schema(5000));
            props.insert("transplantable_core".to_string(), string_schema(5000));
            props.insert("logic_math_heuristic".to_string(), string_schema(5000));
            props.insert("real_structural_problem".to_string(), string_schema(5000));
            props.insert("categoria_nuance_tecnica".to_string(), string_schema(2000));
            props.insert("integracao_papel_exato".to_string(), string_schema(2000));
            strict_object(
                props,
                vec![
                    "indicacao_otimista_canibalizacao",
                    "ouro_a_extrair",
                    "deep_pattern",
                    "transplantable_core",
                    "logic_math_heuristic",
                    "real_structural_problem",
                    "categoria_nuance_tecnica",
                    "integracao_papel_exato",
                ],
            )
        }
        22 => {
            let mut props = serde_json::Map::new();
            props.insert(
                "must_components_prod_ux".to_string(),
                string_array_schema(3, 8, 800),
            );
            props.insert(
                "must_components_arq".to_string(),
                string_array_schema(3, 8, 800),
            );
            props.insert(
                "must_components_ops".to_string(),
                string_array_schema(3, 8, 800),
            );
            props.insert(
                "detected_toxic_deps".to_string(),
                string_array_schema(1, 8, 800),
            );
            props.insert(
                "do_not_absorb".to_string(),
                string_array_schema(1, 8, 800),
            );
            props.insert(
                "where_ai_should_not_enter".to_string(),
                string_array_schema(1, 8, 800),
            );
            strict_object(
                props,
                vec![
                    "must_components_prod_ux",
                    "must_components_arq",
                    "must_components_ops",
                    "detected_toxic_deps",
                    "do_not_absorb",
                    "where_ai_should_not_enter",
                ],
            )
        }
        3 => {
            let mut props = serde_json::Map::new();
            props.insert(
                "classificacao_terminal".to_string(),
                enum_schema(&[
                    "APROVADO_PARA_PRODUCAO",
                    "APROVADO_COM_RESSALVAS",
                    "REJEITADO_DESCARTE",
                ]),
            );
            props.insert(
                "acao_de_canibalizacao".to_string(),
                enum_schema(&["NENHUMA", "ABSORVER_LOGICA", "EXTRAIR_SCRIPTS"]),
            );
            props.insert(
                "categoria_arquitetural".to_string(),
                enum_schema(&[
                    "CanvasUI",
                    "UILibrary",
                    "Memoria_RAG",
                    "Roteamento_FinOps",
                    "Orquestracao_Agentes",
                    "Model_Serving",
                    "Knowledge_Extraction",
                    "Seguranca_Sandbox",
                    "Infraestrutura_Core",
                    "Tooling_Dev",
                ]),
            );
            props.insert(
                "horizonte_extracao".to_string(),
                enum_schema(&["IMMEDIATE", "SHORT", "MEDIUM", "LONG", "VERY_LONG"]),
            );
            props.insert(
                "tipo_integracao".to_string(),
                enum_schema(&["INTEGRATE_AS_COMPONENT", "REIMPLEMENT_INTERNALLY", "REJECT"]),
            );
            props.insert(
                "capability_nature_primary".to_string(),
                enum_schema(&[
                    "LIBRARY",
                    "TOOLING",
                    "SERVICE",
                    "APPLICATION",
                    "SYSTEM",
                    "ALGORITHM",
                    "DATA_STRUCTURE",
                ]),
            );
            props.insert(
                "architectural_topology".to_string(),
                enum_schema(&[
                    "MODULAR",
                    "MONOLITH",
                    "LAYERED",
                    "MICROSERVICES",
                    "EVENT_DRIVEN",
                    "PIPELINE",
                    "PLUGIN",
                ]),
            );
            props.insert("temporal_stability".to_string(), enum_schema(&["STABLE", "EVOLVING"]));
            props.insert("bare_metal_fit".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("extractability_level".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("runtime_sovereignty_fit".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("local_first_fit".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert(
                "adoptability_level".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "longitudinal_sustainability".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "maintenance_burden".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert(
                "onboarding_friction".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert(
                "observability_operational".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "recoverability_level".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "degradation_behavior".to_string(),
                enum_schema(&["GRACEFUL", "ACCEPTABLE", "FRAGILE", "CATASTROPHIC"]),
            );
            props.insert(
                "curation_burden".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert(
                "evolution_cost".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert("operability_level".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("abandonment_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert(
                "time_to_first_clear_value".to_string(),
                enum_schema(&["IMMEDIATE", "SHORT", "MEDIUM", "LONG", "VERY_LONG"]),
            );
            props.insert(
                "imperfection_tolerance".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert("entropy_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert("design_misuse_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert("intrinsic_ethics_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert(
                "discipline_dependency".to_string(),
                enum_schema(&["NENHUMA", "BAIXA", "MEDIA", "ALTA", "CRITICA"]),
            );
            props.insert("regulatory_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            strict_object(
                props,
                vec![
                    "classificacao_terminal",
                    "acao_de_canibalizacao",
                    "categoria_arquitetural",
                    "horizonte_extracao",
                    "tipo_integracao",
                    "capability_nature_primary",
                    "architectural_topology",
                    "temporal_stability",
                    "bare_metal_fit",
                    "extractability_level",
                    "runtime_sovereignty_fit",
                    "local_first_fit",
                    "adoptability_level",
                    "longitudinal_sustainability",
                    "maintenance_burden",
                    "onboarding_friction",
                    "observability_operational",
                    "recoverability_level",
                    "degradation_behavior",
                    "curation_burden",
                    "evolution_cost",
                    "operability_level",
                    "abandonment_risk",
                    "time_to_first_clear_value",
                    "imperfection_tolerance",
                    "entropy_risk",
                    "design_misuse_risk",
                    "intrinsic_ethics_risk",
                    "discipline_dependency",
                    "regulatory_risk",
                ],
            )
        }
        4 => {
            let mut props = serde_json::Map::new();
            props.insert("score_philosophical_fit".to_string(), int_0_10_schema());
            props.insert("score_bare_metal_fit".to_string(), int_0_10_schema());
            props.insert("score_architectural_extractability".to_string(), int_0_10_schema());
            props.insert("score_operability".to_string(), int_0_10_schema());
            props.insert("score_creep_risk".to_string(), int_0_10_schema());
            props.insert("score_runtime_sovereignty".to_string(), int_0_10_schema());
            props.insert("score_model_logic_value".to_string(), int_0_10_schema());
            props.insert("score_ethics_safety".to_string(), int_0_10_schema());
            props.insert("score_intrinsic_risk".to_string(), int_0_10_schema());
            strict_object(
                props,
                vec![
                    "score_philosophical_fit",
                    "score_bare_metal_fit",
                    "score_architectural_extractability",
                    "score_operability",
                    "score_creep_risk",
                    "score_runtime_sovereignty",
                    "score_model_logic_value",
                    "score_ethics_safety",
                    "score_intrinsic_risk",
                ],
            )
        }
        _ => {
            let mut props = serde_json::Map::new();
            props.insert("note".to_string(), string_schema(200));
            strict_object(props, vec!["note"])
        }
    };

    let schema = if block == 3 {
        envelope_fields_only(fields_schema)
    } else {
        envelope_with_justifications(fields_schema)
    };
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": format!("soda_f3_block_{block}"),
            "strict": true,
            "schema": schema
        }
    })
}

fn example_output_for_block(block: u8) -> Value {
    let fields = match block {
        1 => json!({
            "proposta_original_resumo": "",
            "declared_description_ptbr": "",
            "visao_do_enxame": "",
            "justificativa_decisao": "",
            "executive_verdict": "",
            "risco_principal": "",
            "risco_linha_vermelha": "",
            "observacoes": ""
        }),
        21 => json!({
            "indicacao_otimista_canibalizacao": "",
            "ouro_a_extrair": "",
            "deep_pattern": "",
            "transplantable_core": "",
            "logic_math_heuristic": "",
            "real_structural_problem": "",
            "categoria_nuance_tecnica": "",
            "integracao_papel_exato": ""
        }),
        22 => json!({
            "must_components_prod_ux": ["item 1", "item 2", "item 3"],
            "must_components_arq": ["item 1", "item 2", "item 3"],
            "must_components_ops": ["item 1", "item 2", "item 3"],
            "detected_toxic_deps": ["item 1"],
            "do_not_absorb": ["item 1"],
            "where_ai_should_not_enter": ["item 1"]
        }),
        3 => json!({
            "classificacao_terminal": "APROVADO_COM_RESSALVAS",
            "acao_de_canibalizacao": "NENHUMA",
            "categoria_arquitetural": "Tooling_Dev",
            "horizonte_extracao": "SHORT",
            "tipo_integracao": "INTEGRATE_AS_COMPONENT",
            "capability_nature_primary": "TOOLING",
            "architectural_topology": "MODULAR",
            "temporal_stability": "EVOLVING",
            "bare_metal_fit": "MEDIUM",
            "extractability_level": "MEDIUM",
            "runtime_sovereignty_fit": "MEDIUM",
            "local_first_fit": "MEDIUM",
            "adoptability_level": "MEDIUM",
            "longitudinal_sustainability": "MEDIUM",
            "maintenance_burden": "MEDIUM",
            "onboarding_friction": "MEDIUM",
            "observability_operational": "MEDIUM",
            "recoverability_level": "MEDIUM",
            "degradation_behavior": "ACCEPTABLE",
            "curation_burden": "MEDIUM",
            "evolution_cost": "MEDIUM",
            "operability_level": "MEDIUM",
            "abandonment_risk": "MEDIUM",
            "time_to_first_clear_value": "SHORT",
            "imperfection_tolerance": "MEDIUM",
            "entropy_risk": "MEDIUM",
            "design_misuse_risk": "MEDIUM",
            "intrinsic_ethics_risk": "MEDIUM",
            "discipline_dependency": "MEDIA",
            "regulatory_risk": "MEDIUM"
        }),
        4 => json!({
            "score_philosophical_fit": 0,
            "score_bare_metal_fit": 0,
            "score_architectural_extractability": 0,
            "score_operability": 0,
            "score_creep_risk": 0,
            "score_runtime_sovereignty": 0,
            "score_model_logic_value": 0,
            "score_ethics_safety": 0,
            "score_intrinsic_risk": 0
        }),
        _ => json!({ "note": "" }),
    };

    if block == 3 {
        json!({ "fields": fields })
    } else {
        json!({
            "fields": fields,
            "justifications": {}
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_for_block3_has_enums_and_is_strict() {
        let rf = response_format_for_block(3);
        assert_eq!(rf.get("type").and_then(|v| v.as_str()), Some("json_schema"));
        let schema = rf
            .get("json_schema")
            .and_then(|v| v.get("schema"))
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(
            schema
                .get("additionalProperties")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
        let required = schema.get("required").and_then(|v| v.as_array()).unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].as_str(), Some("fields"));
        assert!(schema
            .get("properties")
            .and_then(|v| v.get("justifications"))
            .is_none());
        let fields = schema
            .get("properties")
            .and_then(|v| v.get("fields"))
            .and_then(|v| v.get("properties"))
            .and_then(|v| v.as_object())
            .unwrap();
        let ct = fields.get("classificacao_terminal").unwrap();
        let opts = ct.get("enum").and_then(|v| v.as_array()).unwrap();
        assert!(opts.iter().any(|v| v.as_str() == Some("APROVADO_PARA_PRODUCAO")));
    }
}

impl genesis_mc_lib::cognition::synthesizer::FormatterClient for OpenRouterFormatterClient {
    fn format<'a>(
        &'a self,
        model: &'a str,
        prompt: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let block = parse_block_from_prompt(prompt).unwrap_or(0);
            let example = example_output_for_block(block);
            let mut user_prompt = prompt.to_string();
            user_prompt.push_str("\n\nExample Output (JSON)\n");
            let fallback_example = if block == 3 {
                r#"{"fields":{}}"#.to_string()
            } else {
                r#"{"fields":{},"justifications":{}}"#.to_string()
            };
            user_prompt.push_str(
                &serde_json::to_string_pretty(&example)
                    .unwrap_or_else(|_| fallback_example),
            );
            let max_tokens = if block == 3 { 1800 } else { 4096 };
            let mut body_obj = serde_json::Map::new();
            body_obj.insert("model".to_string(), json!(model));
            body_obj.insert(
                "messages".to_string(),
                json!([
                    {
                        "role": "system",
                        "content": "Responda SOMENTE com JSON válido (sem markdown, sem texto extra). Campos descritivos devem estar em Português (PT-BR). Campos ENUM devem manter tokens canônicos do catálogo."
                    },
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ]),
            );
            body_obj.insert("temperature".to_string(), json!(0.0));
            body_obj.insert("max_tokens".to_string(), json!(max_tokens));
            if block == 3 {
                body_obj.insert("reasoning".to_string(), json!({ "exclude": true }));
                body_obj.insert("include_reasoning".to_string(), json!(false));
            }
            body_obj.insert(
                "response_format".to_string(),
                response_format_for_block(block),
            );
            let body = Value::Object(body_obj);

            let max_attempts: u32 = 3;
            for attempt in 1..=max_attempts {
                let decision = FinOpsRouter::classify_text(&user_prompt).map_err(|e| e.to_string())?;
                match decision.destination {
                    RoutingDestination::PassThrough | RoutingDestination::CloudCascade => {}
                    RoutingDestination::LocalModel { .. } => {
                        return Err(
                            "FinOpsRouter exigiu LocalModel para SGR, mas este caminho é cloud-only. Ajuste SODA_FACTORY_CLOUD_ONLY=true ou implemente SGR local.".to_string()
                        );
                    }
                }
                let response = self
                    .client
                    .post(&self.base_url)
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send();
                let response = match tokio::time::timeout(FORMATTER_HTTP_TIMEOUT, response).await {
                    Ok(Ok(resp)) => resp,
                    Ok(Err(e)) => {
                        if retry_backoff_sleep(attempt, max_attempts, 800).await {
                            continue;
                        }
                        return Err(format!("Erro de rede: {}", e));
                    }
                    Err(_) => {
                        if retry_backoff_sleep(attempt, max_attempts, 800).await {
                            continue;
                        }
                        return Err(format!(
                            "Timeout chamando OpenRouter (timeout_s={})",
                            FORMATTER_HTTP_TIMEOUT.as_secs()
                        ));
                    }
                };

                let status = response.status();
                let raw = match tokio::time::timeout(FORMATTER_HTTP_TIMEOUT, response.text()).await {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => {
                        if retry_backoff_sleep(attempt, max_attempts, 800).await {
                            continue;
                        }
                        return Err(e.to_string());
                    }
                    Err(_) => {
                        if retry_backoff_sleep(attempt, max_attempts, 800).await {
                            continue;
                        }
                        return Err(format!(
                            "Timeout lendo body OpenRouter (timeout_s={})",
                            FORMATTER_HTTP_TIMEOUT.as_secs()
                        ));
                    }
                };
                if !status.is_success() {
                    let should_retry = (status.as_u16() == 429 || status.is_server_error())
                        && attempt < max_attempts;
                    if should_retry {
                        let _ = retry_backoff_sleep(attempt, max_attempts, 800).await;
                        continue;
                    }
                    return Err(format!("HTTP {}: {}", status.as_u16(), raw));
                }

                let envelope: Value = match serde_json::from_str(&raw) {
                    Ok(v) => v,
                    Err(e) => {
                        if retry_backoff_sleep(attempt, max_attempts, 800).await {
                            continue;
                        }
                        return Err(format!("Envelope JSON inválido do OpenRouter: {}", e));
                    }
                };
                self.harvest_usage(&envelope);
                let content_opt = Self::extract_openrouter_content(&envelope)
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty());
                if let Some(content) = content_opt {
                    return Ok(content);
                }

                if attempt < max_attempts {
                    let _ = retry_backoff_sleep(attempt, max_attempts, 800).await;
                    continue;
                }

                return Err("Resposta vazia do OpenRouter".to_string());
            }

            Err("Resposta vazia do OpenRouter".to_string())
        })
    }
}

fn fetch_debates(conn: &Connection, repo_id: &str) -> io::Result<(String, String, String)> {
    conn.query_row(
        "SELECT lens_a_json, lens_b_json, lens_c_json
         FROM debates_enxame
         WHERE repo_id = ?1
         LIMIT 1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(|e| io::Error::other(format!("Debates da F2 (Enxame Cognitivo) ausentes em debates_enxame para {}: {}", repo_id, e)))
}

fn fetch_repo_core(conn: &Connection, repo_id: &str) -> io::Result<(String, String)> {
    conn.query_row(
        "SELECT lote_id, repo_url
         FROM repositorios
         WHERE project_name = ?1
         LIMIT 1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|e| io::Error::other(format!("Metadados base ausentes em repositorios para {}: {}", repo_id, e)))
}

fn try_fetch_repositorios_release_info(
    conn: &Connection,
    repo_id: &str,
) -> (Option<String>, Option<String>) {
    let mut stmt = match conn.prepare(
        "SELECT COALESCE(NULLIF(repo_analised_version, ''), NULLIF(repo_version, '')) AS repo_analised_version, ultima_versao_online
         FROM repositorios
         WHERE project_name = ?1
         LIMIT 1",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return (None, None),
    };
    let row: (Option<String>, Option<String>) = match stmt.query_row(params![repo_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }) {
        Ok(value) => value,
        Err(_) => return (None, None),
    };
    let repo_analised_version = row
        .0
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let ultima_versao_online = row
        .1
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    (repo_analised_version, ultima_versao_online)
}

fn try_fetch_repo_heuristics_seed(
    conn: &Connection,
    repo_id: &str,
) -> Option<(String, String, String, String, String, String, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(NULLIF(repo_analised_version, ''), NULLIF(repo_version, '')) AS repo_analised_version, ultima_versao_online, licenca, stack_base, declared_description, proposta_original_resumo, categoria_arquitetural
             FROM repo_heuristics
             WHERE project_name = ?1
             LIMIT 1",
        )
        .ok()?;
    stmt.query_row(params![repo_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })
    .ok()
}

fn try_fetch_repo_heuristics_row(
    conn: &Connection,
    repo_id: &str,
) -> Option<genesis_mc_lib::cognition::synthesizer::MasterSolutionsRow> {
    let mut stmt = conn
        .prepare(
            "SELECT status_atualizacao, status_fase, project_name, repo_url,
                    COALESCE(NULLIF(repo_analised_version, ''), NULLIF(repo_version, '')) AS repo_analised_version,
                    ultima_versao_online, indicacao_otimista_canibalizacao, lote_id, data_ultima_analise, analise_origem,
                    licenca, stack_base, declared_description, proposta_original_resumo,
                    lente_a_sentido_prod_ux, lente_b_estrutura_arq, lente_c_realidade_ops,
                    visao_do_enxame, justificativa_decisao, executive_verdict,
                    risco_principal, risco_linha_vermelha, observacoes,
                    ouro_a_extrair, deep_pattern, transplantable_core, logic_math_heuristic, real_structural_problem,
                    categoria_nuance_tecnica, integracao_papel_exato,
                    must_components_prod_ux, must_components_arq, must_components_ops,
                    detected_toxic_deps, do_not_absorb, where_ai_should_not_enter,
                    classificacao_terminal, acao_de_canibalizacao, categoria_arquitetural, horizonte_extracao, tipo_integracao,
                    capability_nature_primary, architectural_topology, temporal_stability,
                    bare_metal_fit, extractability_level, runtime_sovereignty_fit, local_first_fit,
                    adoptability_level, longitudinal_sustainability,
                    maintenance_burden, onboarding_friction, observability_operational, recoverability_level,
                    degradation_behavior, curation_burden, evolution_cost, operability_level,
                    abandonment_risk, time_to_first_clear_value, imperfection_tolerance,
                    entropy_risk, design_misuse_risk, intrinsic_ethics_risk, discipline_dependency, regulatory_risk,
                    score_philosophical_fit, score_bare_metal_fit, score_architectural_extractability,
                    score_operability, score_creep_risk, score_runtime_sovereignty, score_model_logic_value,
                    score_ethics_safety, score_intrinsic_risk,
                    score_final, score_fit_geral_soda,
                    score_architectural_priority, score_human_product_priority, score_absorption_readiness,
                    score_operational_priority, score_sustainability_adjusted_fit,
                    valid_from, valid_to, embargo_status
             FROM repo_heuristics
             WHERE project_name = ?1
             LIMIT 1",
        )
        .ok()?;
    let json_val: serde_json::Value = stmt
        .query_row(params![repo_id], |row| {
            let mut obj = serde_json::Map::new();
            obj.insert("status_atualizacao".to_string(), serde_json::json!(row.get::<_, String>(0)?));
            obj.insert("status_fase".to_string(), serde_json::json!(row.get::<_, String>(1)?));
            obj.insert("project_name".to_string(), serde_json::json!(row.get::<_, String>(2)?));
            obj.insert("repo_url".to_string(), serde_json::json!(row.get::<_, String>(3)?));
            obj.insert("repo_analised_version".to_string(), serde_json::json!(row.get::<_, String>(4)?));
            obj.insert("ultima_versao_online".to_string(), serde_json::json!(row.get::<_, String>(5)?));
            obj.insert("indicacao_otimista_canibalizacao".to_string(), serde_json::json!(row.get::<_, String>(6)?));
            obj.insert("lote_id".to_string(), serde_json::json!(row.get::<_, String>(7)?));
            obj.insert("data_ultima_analise".to_string(), serde_json::json!(row.get::<_, i64>(8)?));
            obj.insert("analise_origem".to_string(), serde_json::json!(row.get::<_, String>(9)?));
            obj.insert("licenca".to_string(), serde_json::json!(row.get::<_, String>(10)?));
            obj.insert("stack_base".to_string(), serde_json::json!(row.get::<_, String>(11)?));
            obj.insert("declared_description".to_string(), serde_json::json!(row.get::<_, String>(12)?));
            obj.insert("declared_description_ptbr".to_string(), serde_json::json!(""));
            obj.insert("proposta_original_resumo".to_string(), serde_json::json!(row.get::<_, String>(13)?));
            obj.insert("lente_a_sentido_prod_ux".to_string(), serde_json::json!(row.get::<_, String>(14)?));
            obj.insert("lente_b_estrutura_arq".to_string(), serde_json::json!(row.get::<_, String>(15)?));
            obj.insert("lente_c_realidade_ops".to_string(), serde_json::json!(row.get::<_, String>(16)?));
            obj.insert("visao_do_enxame".to_string(), serde_json::json!(row.get::<_, String>(17)?));
            obj.insert("justificativa_decisao".to_string(), serde_json::json!(row.get::<_, String>(18)?));
            obj.insert("executive_verdict".to_string(), serde_json::json!(row.get::<_, String>(19)?));
            obj.insert("risco_principal".to_string(), serde_json::json!(row.get::<_, String>(20)?));
            obj.insert("risco_linha_vermelha".to_string(), serde_json::json!(row.get::<_, String>(21)?));
            obj.insert("observacoes".to_string(), serde_json::json!(row.get::<_, String>(22)?));
            obj.insert("ouro_a_extrair".to_string(), serde_json::json!(row.get::<_, String>(23)?));
            obj.insert("deep_pattern".to_string(), serde_json::json!(row.get::<_, String>(24)?));
            obj.insert("transplantable_core".to_string(), serde_json::json!(row.get::<_, String>(25)?));
            obj.insert("logic_math_heuristic".to_string(), serde_json::json!(row.get::<_, String>(26)?));
            obj.insert("real_structural_problem".to_string(), serde_json::json!(row.get::<_, String>(27)?));
            obj.insert("categoria_nuance_tecnica".to_string(), serde_json::json!(row.get::<_, String>(28)?));
            obj.insert("integracao_papel_exato".to_string(), serde_json::json!(row.get::<_, String>(29)?));
            obj.insert("must_components_prod_ux".to_string(), serde_json::json!(row.get::<_, String>(30)?));
            obj.insert("must_components_arq".to_string(), serde_json::json!(row.get::<_, String>(31)?));
            obj.insert("must_components_ops".to_string(), serde_json::json!(row.get::<_, String>(32)?));
            obj.insert("detected_toxic_deps".to_string(), serde_json::json!(row.get::<_, String>(33)?));
            obj.insert("do_not_absorb".to_string(), serde_json::json!(row.get::<_, String>(34)?));
            obj.insert("where_ai_should_not_enter".to_string(), serde_json::json!(row.get::<_, String>(35)?));
            obj.insert("classificacao_terminal".to_string(), serde_json::json!(row.get::<_, String>(36)?));
            obj.insert("acao_de_canibalizacao".to_string(), serde_json::json!(row.get::<_, String>(37)?));
            obj.insert("categoria_arquitetural".to_string(), serde_json::json!(row.get::<_, String>(38)?));
            obj.insert("horizonte_extracao".to_string(), serde_json::json!(row.get::<_, String>(39)?));
            obj.insert("tipo_integracao".to_string(), serde_json::json!(row.get::<_, String>(40)?));
            obj.insert("capability_nature_primary".to_string(), serde_json::json!(row.get::<_, String>(41)?));
            obj.insert("architectural_topology".to_string(), serde_json::json!(row.get::<_, String>(42)?));
            obj.insert("temporal_stability".to_string(), serde_json::json!(row.get::<_, String>(43)?));
            obj.insert("bare_metal_fit".to_string(), serde_json::json!(row.get::<_, String>(44)?));
            obj.insert("extractability_level".to_string(), serde_json::json!(row.get::<_, String>(45)?));
            obj.insert("runtime_sovereignty_fit".to_string(), serde_json::json!(row.get::<_, String>(46)?));
            obj.insert("local_first_fit".to_string(), serde_json::json!(row.get::<_, String>(47)?));
            obj.insert("adoptability_level".to_string(), serde_json::json!(row.get::<_, String>(48)?));
            obj.insert("longitudinal_sustainability".to_string(), serde_json::json!(row.get::<_, String>(49)?));
            obj.insert("maintenance_burden".to_string(), serde_json::json!(row.get::<_, String>(50)?));
            obj.insert("onboarding_friction".to_string(), serde_json::json!(row.get::<_, String>(51)?));
            obj.insert("observability_operational".to_string(), serde_json::json!(row.get::<_, String>(52)?));
            obj.insert("recoverability_level".to_string(), serde_json::json!(row.get::<_, String>(53)?));
            obj.insert("degradation_behavior".to_string(), serde_json::json!(row.get::<_, String>(54)?));
            obj.insert("curation_burden".to_string(), serde_json::json!(row.get::<_, String>(55)?));
            obj.insert("evolution_cost".to_string(), serde_json::json!(row.get::<_, String>(56)?));
            obj.insert("operability_level".to_string(), serde_json::json!(row.get::<_, String>(57)?));
            obj.insert("abandonment_risk".to_string(), serde_json::json!(row.get::<_, String>(58)?));
            obj.insert("time_to_first_clear_value".to_string(), serde_json::json!(row.get::<_, String>(59)?));
            obj.insert("imperfection_tolerance".to_string(), serde_json::json!(row.get::<_, String>(60)?));
            obj.insert("entropy_risk".to_string(), serde_json::json!(row.get::<_, String>(61)?));
            obj.insert("design_misuse_risk".to_string(), serde_json::json!(row.get::<_, String>(62)?));
            obj.insert("intrinsic_ethics_risk".to_string(), serde_json::json!(row.get::<_, String>(63)?));
            obj.insert("discipline_dependency".to_string(), serde_json::json!(row.get::<_, String>(64)?));
            obj.insert("regulatory_risk".to_string(), serde_json::json!(row.get::<_, String>(65)?));
            obj.insert("score_philosophical_fit".to_string(), serde_json::json!(row.get::<_, i64>(66)?));
            obj.insert("score_bare_metal_fit".to_string(), serde_json::json!(row.get::<_, i64>(67)?));
            obj.insert("score_architectural_extractability".to_string(), serde_json::json!(row.get::<_, i64>(68)?));
            obj.insert("score_operability".to_string(), serde_json::json!(row.get::<_, i64>(69)?));
            obj.insert("score_creep_risk".to_string(), serde_json::json!(row.get::<_, i64>(70)?));
            obj.insert("score_runtime_sovereignty".to_string(), serde_json::json!(row.get::<_, i64>(71)?));
            obj.insert("score_model_logic_value".to_string(), serde_json::json!(row.get::<_, i64>(72)?));
            obj.insert("score_ethics_safety".to_string(), serde_json::json!(row.get::<_, i64>(73)?));
            obj.insert("score_intrinsic_risk".to_string(), serde_json::json!(row.get::<_, i64>(74)?));
            obj.insert("score_final".to_string(), serde_json::json!(row.get::<_, f64>(75)?));
            obj.insert("score_fit_geral_soda".to_string(), serde_json::json!(row.get::<_, f64>(76)?));
            obj.insert("score_architectural_priority".to_string(), serde_json::json!(row.get::<_, f64>(77)?));
            obj.insert("score_human_product_priority".to_string(), serde_json::json!(row.get::<_, f64>(78)?));
            obj.insert("score_absorption_readiness".to_string(), serde_json::json!(row.get::<_, f64>(79)?));
            obj.insert("score_operational_priority".to_string(), serde_json::json!(row.get::<_, f64>(80)?));
            obj.insert("score_sustainability_adjusted_fit".to_string(), serde_json::json!(row.get::<_, f64>(81)?));
            obj.insert("valid_from".to_string(), serde_json::json!(row.get::<_, i64>(82)?));
            obj.insert("valid_to".to_string(), serde_json::json!(row.get::<_, Option<i64>>(83)?));
            obj.insert("embargo_status".to_string(), serde_json::json!(row.get::<_, i64>(84)?));
            Ok(serde_json::Value::Object(obj))
        })
        .ok()?;
    serde_json::from_value::<genesis_mc_lib::cognition::synthesizer::MasterSolutionsRow>(json_val).ok()
}

fn fetch_block3_justifications(conn: &Connection, repo_id: &str) -> HashMap<String, String> {
    let json_text: Option<String> = conn
        .query_row(
            "SELECT justifications_json
             FROM repo_heuristics_justifications
             WHERE project_name = ?1 AND block = 3
             LIMIT 1",
            params![repo_id],
            |row| row.get::<_, String>(0),
        )
        .ok();
    json_text
        .and_then(|t| serde_json::from_str::<HashMap<String, String>>(&t).ok())
        .unwrap_or_default()
}

fn is_unknown_like(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower == "unknown" || lower == "desconhecido" || lower == "n/a"
}

fn normalize_categoria_arquitetural_seed(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_unknown_like(trimmed) {
        return None;
    }
    let mapped = match trimmed {
        "Tooling_Dev"
        | "Infraestrutura_Core"
        | "Seguranca_Sandbox"
        | "Knowledge_Extraction"
        | "Model_Serving"
        | "Orquestracao_Agentes"
        | "Roteamento_FinOps"
        | "Memoria_RAG"
        | "UILibrary"
        | "CanvasUI" => trimmed.to_string(),
        "TOOLING" => "Tooling_Dev".to_string(),
        "INFRASTRUCTURE" => "Infraestrutura_Core".to_string(),
        "RUNTIME" => "Model_Serving".to_string(),
        "LIBRARY" => "UILibrary".to_string(),
        _ => return None,
    };
    Some(mapped)
}

fn detect_repo_kind_from_raw_blobs(conn: &Connection, repo_id: &str) -> &'static str {
    let mut hay = String::new();
    if let Some(t) = fetch_raw_artifact_text(conn, repo_id, "blob_04_repo_outline") {
        hay.push_str(&t);
        hay.push('\n');
    }
    if let Some(t) = fetch_raw_artifact_text(conn, repo_id, "blob_05_architecture_map") {
        hay.push_str(&t);
        hay.push('\n');
    }
    let lower = hay.to_ascii_lowercase();
    if lower.contains("kind: skilllibrary") {
        return "SkillLibrary";
    }
    if lower.contains("kind: contentrepo") {
        return "ContentRepo";
    }
    "CodeRepo"
}

fn fetch_raw_artifact_text(conn: &Connection, repo_id: &str, artifact_type: &str) -> Option<String> {
    conn.query_row(
        "SELECT CAST(payload_blob AS TEXT)
         FROM artefatos_brutos
         WHERE repo_id = ?1 AND artifact_type = ?2
         ORDER BY artifact_id DESC
         LIMIT 1",
        params![repo_id, artifact_type],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
}

fn derive_stack_base_from_manifest_blob(text: &str) -> Option<String> {
    for line in text.lines().take(20) {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("stack_base:") else {
            continue;
        };
        let val = rest.trim();
        if val.is_empty() {
            return None;
        }
        if is_unknown_like(val) {
            return None;
        }
        return Some(val.to_string());
    }
    None
}

fn derive_license_from_community_meta_json(text: &str) -> Option<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return None;
    };
    value
        .get("licenca")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !is_unknown_like(s))
}

fn derive_license_from_readme(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("mit license") || (lower.contains("license") && lower.contains("mit")) {
        return Some("MIT".to_string());
    }
    if lower.contains("apache license") || lower.contains("apache-2.0") {
        return Some("Apache-2.0".to_string());
    }
    if lower.contains("gnu general public license") || lower.contains("gpl") {
        return Some("GPL".to_string());
    }
    None
}

fn derive_declared_description_from_readme(text: &str) -> Option<String> {
    let mut fallback_heading: Option<String> = None;
    for line in text.lines().take(120) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("<") {
            continue;
        }
        if trimmed.starts_with('#') {
            if fallback_heading.is_none() {
                let heading = trimmed.trim_start_matches('#').trim();
                if !heading.is_empty() {
                    fallback_heading = Some(heading.to_string());
                }
            }
            continue;
        }
        let cleaned = trimmed.replace("**", "").replace('`', "");
        let candidate = cleaned.trim();
        if candidate.is_empty() {
            continue;
        }
        let max_chars = 2000_usize;
        let mut prefix = candidate.chars().take(max_chars).collect::<String>();
        if candidate.chars().count() > max_chars {
            while !prefix.is_empty() {
                let last = prefix.chars().last().unwrap_or(' ');
                if last.is_whitespace()
                    || last == '.'
                    || last == ','
                    || last == ';'
                    || last == ':'
                    || last == ')'
                    || last == ']'
                {
                    break;
                }
                prefix.pop();
            }
            prefix = prefix.trim_end().to_string();
        }
        if !prefix.is_empty() {
            return Some(prefix);
        }
    }
    fallback_heading
}

async fn call_mcp(tool_name: &str, arguments: Value) -> io::Result<Value> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;

    let creds = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .map_err(|_| io::Error::other("Missing GOOGLE_APPLICATION_CREDENTIALS"))?;

    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "phase3-4-cli", "version": "1.0.0" }
        }
    });
    let initialized_notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    });

    let max_attempts: u32 = match tool_name {
        "get_sheet_data" | "batch_update_cells" => 4,
        _ => 1,
    };

    for attempt in 1..=max_attempts {
        if attempt > 1 {
            let jitter_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_millis() as u64
                % 250;
            let exp = attempt.saturating_sub(2).min(10);
            let base_ms = 500_u64.saturating_mul(1_u64 << exp);
            tokio::time::sleep(Duration::from_millis(base_ms.saturating_add(jitter_ms))).await;
        }

        let mut child = Command::new("mcp-google-sheets")
            .env("GOOGLE_APPLICATION_CREDENTIALS", &creds)
            .env("UV_NO_PROGRESS", "1")
            .env("UV_QUIET", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| io::Error::other(format!("Falha ao spawnar mcp-google-sheets: {}", e)))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("stdin indisponível"))?;
        stdin
            .write_all(format!("{}\n", init_req).as_bytes())
            .await
            .map_err(|e| io::Error::other(format!("Falha ao escrever init no MCP: {}", e)))?;
        stdin
            .write_all(format!("{}\n", initialized_notif).as_bytes())
            .await
            .map_err(|e| io::Error::other(format!("Falha ao escrever initialized no MCP: {}", e)))?;
        stdin
            .write_all(format!("{}\n", mcp_request).as_bytes())
            .await
            .map_err(|e| io::Error::other(format!("Falha ao escrever tools/call no MCP: {}", e)))?;
        drop(stdin);

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout indisponível"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("stderr indisponível"))?;
        let (status, stdout_buf, stderr_buf) = match tokio::time::timeout(MCP_TIMEOUT, async {
            use tokio::io::AsyncReadExt;
            let mut out_buf = Vec::new();
            let mut err_buf = Vec::new();
            let (out_res, err_res, status_res) = tokio::join!(
                stdout.read_to_end(&mut out_buf),
                stderr.read_to_end(&mut err_buf),
                child.wait()
            );
            out_res.map_err(|e| io::Error::other(format!("Falha ao ler stdout MCP: {}", e)))?;
            err_res.map_err(|e| io::Error::other(format!("Falha ao ler stderr MCP: {}", e)))?;
            let status =
                status_res.map_err(|e| io::Error::other(format!("Falha ao aguardar mcp-google-sheets: {}", e)))?;
            Ok::<_, io::Error>((status, out_buf, err_buf))
        })
        .await
        {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                if attempt < max_attempts {
                    continue;
                }
                return Err(e);
            }
            Err(_) => {
                let _ = child.kill().await;
                let err = io::Error::other(format!(
                    "Timeout aguardando mcp-google-sheets tool={} timeout_s={}",
                    tool_name,
                    MCP_TIMEOUT.as_secs()
                ));
                if attempt < max_attempts {
                    continue;
                }
                return Err(err);
            }
        };
        if !status.success() {
            let err = io::Error::other(format!(
                "mcp-google-sheets falhou. Exit {}. STDERR: {}",
                status,
                String::from_utf8_lossy(&stderr_buf)
            ));
            if attempt < max_attempts {
                continue;
            }
            return Err(err);
        }

        let stdout_str = String::from_utf8_lossy(&stdout_buf);
        for line in stdout_str.lines().rev() {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                    if value.get("error").is_some() {
                        let err = io::Error::other(format!("MCP retornou erro: {}", value));
                        if attempt < max_attempts {
                            continue;
                        }
                        return Err(err);
                    }
                    if let Some(result) = value.get("result") {
                        let normalized = normalize_mcp_tool_result(result.clone());
                        if tool_name == "get_sheet_data"
                            && normalized.get("error").is_none()
                            && normalized.get("values").is_none()
                            && normalized.get("valueRanges").is_none()
                            && normalized
                                .get("data")
                                .and_then(|d| d.get("values"))
                                .is_none()
                        {
                            let err = io::Error::other(
                                "Sheets payload inválido: sem 'values', 'valueRanges' ou 'data.values'",
                            );
                            if attempt < max_attempts {
                                continue;
                            }
                            return Err(err);
                        }
                        return Ok(normalized);
                    }
                }
            }
        }

        if attempt < max_attempts {
            continue;
        }
        return Err(io::Error::other("Resposta MCP não encontrada no stdout"));
    }

    Err(io::Error::other("Falha inesperada: loop de tentativas vazio"))
}

fn normalize_mcp_tool_result(result: Value) -> Value {
    if result.get("values").is_some()
        || result.get("valueRanges").is_some()
        || result.get("data").and_then(|d| d.get("values")).is_some()
        || result.get("error").is_some()
    {
        return result;
    }

    let content = match result.get("content").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return result,
    };

    for item in content {
        if let Some(json_val) = item.get("json") {
            if json_val.is_string() {
                if let Some(s) = json_val.as_str() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                        return parsed;
                    }
                }
            } else {
                return json_val.clone();
            }
        }
        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                return parsed;
            }
            let mut msg = text.trim().to_string();
            if msg.len() > 800 {
                msg.truncate(800);
            }
            return json!({ "error": { "message": msg } });
        }
    }

    result
}

fn extract_values_2d(result: &Value) -> Option<Vec<Vec<String>>> {
    if let Some(values) = result.get("values").and_then(|v| v.as_array()) {
        let mut out = Vec::new();
        for row in values {
            let arr = row.as_array()?;
            out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
        }
        return Some(out);
    }

    if let Some(ranges) = result.get("valueRanges").and_then(|v| v.as_array()) {
        let first = ranges.first()?;
        let values = first.get("values").and_then(|v| v.as_array())?;
        let mut out = Vec::new();
        for row in values {
            let arr = row.as_array()?;
            out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
        }
        return Some(out);
    }

    if let Some(grid) = result.get("data") {
        if let Some(values) = grid.get("values").and_then(|v| v.as_array()) {
            let mut out = Vec::new();
            for row in values {
                let arr = row.as_array()?;
                out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
            }
            return Some(out);
        }
    }

    None
}

async fn resolve_row_number_by_repo_url_and_lote_id(
    spreadsheet_id: &str,
    repo_url: &str,
    lote_id: &str,
) -> io::Result<u32> {
    let header_row = read_master_header(spreadsheet_id).await?;
    let repo_url_idx = find_col_idx(&header_row, "repo_url")
        .ok_or_else(|| io::Error::other("Header missing repo_url"))?;
    let lote_id_idx =
        find_col_idx(&header_row, "lote_id").ok_or_else(|| io::Error::other("Header missing lote_id"))?;
    let min_idx = repo_url_idx.min(lote_id_idx);
    let max_idx = repo_url_idx.max(lote_id_idx);
    let start_col = col_idx_to_a1(min_idx);
    let end_col = col_idx_to_a1(max_idx);
    let range = format!("{start_col}2:{end_col}");
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d(&result).unwrap_or_default();
    let needle = repo_url.trim_end_matches('/').to_ascii_lowercase();
    let lote_needle = lote_id.trim();
    let needle_repo_id = try_extract_repo_id_from_repo_url(repo_url)
        .unwrap_or_default()
        .to_ascii_lowercase();

    let extract_repo_id_loose = |raw: &str| -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some(repo_id) = try_extract_repo_id_from_repo_url(trimmed) {
            return Some(repo_id.to_ascii_lowercase());
        }
        if trimmed.contains("://") {
            return None;
        }
        let candidate = trimmed.replace(' ', "");
        let parts: Vec<&str> = candidate.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() == 2 {
            return Some(format!("{}/{}", parts[0], parts[1]).to_ascii_lowercase());
        }
        None
    };

    for (idx, row) in values.iter().enumerate() {
        let repo_cell = row
            .get(repo_url_idx.saturating_sub(min_idx))
            .map(|s| s.trim())
            .unwrap_or("");
        let lote_cell = row
            .get(lote_id_idx.saturating_sub(min_idx))
            .map(|s| s.trim())
            .unwrap_or("");
        let repo_hay = repo_cell.trim_end_matches('/').to_ascii_lowercase();
        let repo_id_hay = extract_repo_id_loose(repo_cell);
        let repo_matches = (!repo_hay.is_empty() && repo_hay == needle)
            || (!needle_repo_id.is_empty()
                && repo_id_hay
                    .as_deref()
                    .map(|v| v == needle_repo_id)
                    .unwrap_or(false));
        if repo_matches && !lote_cell.is_empty() && lote_cell == lote_needle
        {
            return Ok((idx as u32) + 2);
        }
    }

    for (idx, row) in values.iter().enumerate() {
        let repo_cell = row
            .get(repo_url_idx.saturating_sub(min_idx))
            .map(|s| s.trim())
            .unwrap_or("");
        let repo_hay = repo_cell.trim_end_matches('/').to_ascii_lowercase();
        let repo_id_hay = extract_repo_id_loose(repo_cell);
        let repo_matches = (!repo_hay.is_empty() && repo_hay == needle)
            || (!needle_repo_id.is_empty()
                && repo_id_hay
                    .as_deref()
                    .map(|v| v == needle_repo_id)
                    .unwrap_or(false));
        if repo_matches {
            return Ok((idx as u32) + 2);
        }
    }

    for (idx, row) in values.iter().enumerate() {
        let repo_cell = row
            .get(repo_url_idx.saturating_sub(min_idx))
            .map(|s| s.trim())
            .unwrap_or("");
        let lote_cell = row
            .get(lote_id_idx.saturating_sub(min_idx))
            .map(|s| s.trim())
            .unwrap_or("");
        if repo_cell.is_empty() && lote_cell.is_empty() {
            return Ok((idx as u32) + 2);
        }
    }

    Err(io::Error::other(format!(
        "Não foi possível resolver row_number: repo_url+lote_id não encontrados e não há linhas vazias disponíveis (planilha aparenta estar cheia até a última linha do range). repo_url={} lote_id={}",
        repo_url,
        lote_id
    )))
}

async fn read_status_atualizacao_e_fase(
    spreadsheet_id: &str,
    row_number_1based: u32,
    cols: MasterCols,
) -> io::Result<(String, String)> {
    let required = [cols.status_atualizacao_idx, cols.status_fase_idx];
    let min_idx = *required.iter().min().unwrap_or(&0);
    let max_idx = *required.iter().max().unwrap_or(&0);
    let start_col = col_idx_to_a1(min_idx);
    let end_col = col_idx_to_a1(max_idx);
    let range = format!("{start_col}{row_number_1based}:{end_col}{row_number_1based}");
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d(&result).unwrap_or_default();
    let row = values.first().cloned().unwrap_or_default();
    let get = |abs_idx: usize| -> String {
        let rel = abs_idx.saturating_sub(min_idx);
        row.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
    };
    let status_atualizacao = get(cols.status_atualizacao_idx);
    let status_fase = get(cols.status_fase_idx);
    Ok((status_atualizacao, status_fase))
}

#[derive(Clone, Copy)]
struct MasterCols {
    status_atualizacao_idx: usize,
    status_fase_idx: usize,
}

fn resolve_master_cols(header_row: &[String]) -> io::Result<MasterCols> {
    let status_atualizacao_idx = find_col_idx(header_row, "status_atualizacao")
        .ok_or_else(|| io::Error::other("Header missing status_atualizacao"))?;
    let status_fase_idx =
        find_col_idx(header_row, "status_fase").ok_or_else(|| io::Error::other("Header missing status_fase"))?;
    Ok(MasterCols {
        status_atualizacao_idx,
        status_fase_idx,
    })
}

async fn update_status_fase_only(
    spreadsheet_id: &str,
    row_number_1based: u32,
    cols: MasterCols,
    status_fase: &str,
) -> io::Result<()> {
    let col = col_idx_to_a1(cols.status_fase_idx);
    let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
    let _ = call_mcp(
        "batch_update_cells",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "ranges": {
                range: [[status_fase]]
            }
        }),
    )
    .await?;
    Ok(())
}

async fn update_status_atualizacao_e_fase(
    spreadsheet_id: &str,
    row_number_1based: u32,
    cols: MasterCols,
    status_atualizacao: &str,
    status_fase: &str,
) -> io::Result<()> {
    let status_col = col_idx_to_a1(cols.status_atualizacao_idx);
    let fase_col = col_idx_to_a1(cols.status_fase_idx);
    let range_a = format!("{status_col}{row_number_1based}:{status_col}{row_number_1based}");
    let range_b = format!("{fase_col}{row_number_1based}:{fase_col}{row_number_1based}");
    let _ = call_mcp(
        "batch_update_cells",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "ranges": {
                range_a: [[status_atualizacao]],
                range_b: [[status_fase]]
            }
        }),
    )
    .await?;
    Ok(())
}

async fn confirm_sheet_write(row_number_1based: u32, expected_repo_id: &str) -> io::Result<bool> {
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let expected_pretty = expected_repo_id.replace("/", " / ");
    let expected_url = format!("https://github.com/{}", expected_repo_id.trim());
    let header_row = read_master_header(&spreadsheet_id).await?;
    let project_idx = find_col_idx(&header_row, "project_name")
        .ok_or_else(|| io::Error::other("Header missing project_name"))?;
    let repo_url_idx = find_col_idx(&header_row, "repo_url")
        .ok_or_else(|| io::Error::other("Header missing repo_url"))?;
    let min_idx = project_idx.min(repo_url_idx);
    let max_idx = project_idx.max(repo_url_idx);
    let start_col = col_idx_to_a1(min_idx);
    let end_col = col_idx_to_a1(max_idx);
    let range = format!(
        "{start_col}{row_number_1based}:{end_col}{row_number_1based}"
    );
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d(&result).unwrap_or_default();
    let row = values
        .first()
        .cloned()
        .unwrap_or_default();
    let get = |abs_idx: usize| -> String {
        let rel = abs_idx.saturating_sub(min_idx);
        row.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
    };
    let project_cell = get(project_idx);
    let repo_url_cell = get(repo_url_idx);
    Ok(project_cell == expected_repo_id
        || project_cell == expected_pretty
        || repo_url_cell == expected_url)
}

async fn inspect_row_width_a_to_cf(row_number_1based: u32) -> io::Result<usize> {
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let range = genesis_mc_lib::cognition::synthesizer::sheet_range_for_row(row_number_1based);
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )
    .await?;
    let values = extract_values_2d(&result).unwrap_or_default();
    Ok(values.first().map(|r| r.len()).unwrap_or(0))
}

async fn run_phase_binary(binary_stem: &str, repo_id: &str) -> io::Result<u128> {
    use std::process::Stdio;

    let started = Instant::now();
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| io::Error::other("Falha ao resolver pasta do executável atual (parent = None)"))?;
    let profile = exe_dir
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .filter(|v| v == "debug" || v == "release")
        .unwrap_or_else(|| "debug".to_string());
    let cargo_bin = if cfg!(target_os = "windows") {
        binary_stem.trim_end_matches(".exe").to_string()
    } else {
        binary_stem.to_string()
    };
    let bin_name = if cfg!(target_os = "windows") {
        format!("{cargo_bin}.exe")
    } else {
        cargo_bin.clone()
    };
    let candidate_local = exe_dir.join(&bin_name);
    let root_dir = workspace_root()?;
    let candidate_target = root_dir
        .join("src-tauri")
        .join("target")
        .join(&profile)
        .join(&bin_name);
    let manifest_path = root_dir.join("src-tauri").join("Cargo.toml");

    let _ghost = spawn_ghost_telemetry(
        repo_id.to_string(),
        format!("Executando subfase '{binary_stem}'"),
    );
    let mut command = if candidate_local.exists() {
        tokio::process::Command::new(candidate_local)
    } else if candidate_target.exists() {
        tokio::process::Command::new(candidate_target)
    } else {
        let mut build_cmd = tokio::process::Command::new("cargo");
        build_cmd
            .arg("build")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .arg("--bin")
            .arg(&cargo_bin);
        if profile == "release" {
            build_cmd.arg("--release");
        }
        build_cmd
            .current_dir(root_dir.join("src-tauri"))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let build_status = build_cmd.status().await.map_err(|e| {
            io::Error::other(format!(
                "Falha ao compilar subfase '{binary_stem}' via cargo: {e}"
            ))
        })?;
        if !build_status.success() {
            return Err(io::Error::other(format!(
                "Compilação da subfase '{binary_stem}' falhou via cargo: {build_status}"
            )));
        }

        if candidate_local.exists() {
            tokio::process::Command::new(candidate_local)
        } else if candidate_target.exists() {
            tokio::process::Command::new(candidate_target)
        } else {
            let mut run_cmd = tokio::process::Command::new("cargo");
            run_cmd
                .arg("run")
                .arg("--quiet")
                .arg("--manifest-path")
                .arg(&manifest_path)
                .arg("--bin")
                .arg(&cargo_bin);
            if profile == "release" {
                run_cmd.arg("--release");
            }
            run_cmd
                .arg("--")
                .args(["--repo", repo_id])
                .current_dir(root_dir.join("src-tauri"))
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let status = run_cmd.status().await.map_err(|e| {
                io::Error::other(format!("Falha ao executar fase '{binary_stem}' via cargo run: {e}"))
            })?;
            if !status.success() {
                return Err(io::Error::other(format!(
                    "Fase '{binary_stem}' via cargo run retornou exit code != 0: {status}"
                )));
            }
            return Ok(started.elapsed().as_millis());
        }
    };

    let status = command
        .args(["--repo", repo_id])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| io::Error::other(format!("Falha ao executar fase '{binary_stem}': {e}")))?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "Fase '{binary_stem}' retornou exit code != 0: {status}"
        )));
    }

    Ok(started.elapsed().as_millis())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let level = match rust_log.to_ascii_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    let ansi = !cfg!(windows) && std::io::stderr().is_terminal();
    tracing_subscriber::fmt().with_max_level(level).with_ansi(ansi).init();

    let started_total = Instant::now();
    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let CliArgs {
        repo_id,
        e2e_full,
        skip_harvester,
        batch,
        resume_f3,
        row_override,
        dry_run,
        feedback_inject,
        phase4_only,
    } = parse_cli_args();

    if batch {
        if resume_f3 {
            let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
                .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
            let candidates = fetch_resume_f3_candidates(&spreadsheet_id).await?;
            info!(
                count = candidates.len(),
                "F3/F4: modo batch (resume_f3) via Sheets (APROVADO_PARA_ENXAME + F2_OK|F3_OK|ERRO_F4)"
            );
            let exe = std::env::current_exe()
                .map_err(|e| io::Error::other(format!("Falha ao resolver current_exe: {e}")))?;
            for item in candidates {
                info!(
                    repo_id = %item.repo_id,
                    row_number = item.row_number_1based,
                    "F3/F4(batch resume_f3): iniciando"
                );
                let mut cmd = tokio::process::Command::new(&exe);
                cmd.arg("--repo").arg(&item.repo_id);
                cmd.arg("--row").arg(item.row_number_1based.to_string());
                if e2e_full {
                    cmd.arg("--e2e-full");
                }
                if skip_harvester {
                    cmd.arg("--skip-harvester");
                }
                let status = cmd.status().await.map_err(|e| {
                    io::Error::other(format!("Falha ao executar f3_synthesizer_cli (batch resume_f3): {e}"))
                })?;
                if !status.success() {
                    warn!(
                        repo_id = %item.repo_id,
                        row_number = item.row_number_1based,
                        status = %status,
                        "F3/F4(batch resume_f3): falha (seguindo fail-soft)"
                    );
                }
            }
            return Ok(());
        } else {
            let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
                .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
            let candidates = fetch_enxame_batch_candidates(&spreadsheet_id).await?;
            info!(count = candidates.len(), "F3/F4: modo batch (APROVADO_PARA_ENXAME)");
            let exe = std::env::current_exe()
                .map_err(|e| io::Error::other(format!("Falha ao resolver current_exe: {e}")))?;
            for item in candidates {
                info!(
                    repo_id = %item.repo_id,
                    row_number = item.row_number_1based,
                    "F3/F4(batch): iniciando"
                );
                let mut cmd = tokio::process::Command::new(&exe);
                cmd.arg("--repo").arg(&item.repo_id);
                cmd.arg("--row").arg(item.row_number_1based.to_string());
                if e2e_full {
                    cmd.arg("--e2e-full");
                }
                if skip_harvester {
                    cmd.arg("--skip-harvester");
                }
                let status = cmd.status().await.map_err(|e| {
                    io::Error::other(format!("Falha ao executar f3_synthesizer_cli (batch): {e}"))
                })?;
                if !status.success() {
                    warn!(
                        repo_id = %item.repo_id,
                        row_number = item.row_number_1based,
                        status = %status,
                        "F3/F4(batch): falha (seguindo fail-soft)"
                    );
                }
            }
            return Ok(());
        }
    }

    if e2e_full {
        info!(repo_id = %repo_id, "E2E FULL: iniciando F0 → F4 (disparo completo)");
    } else {
        info!(repo_id = %repo_id, "E2E: iniciando F3 → F4 (munição real)");
    }

    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path).map_err(|e| {
        io::Error::other(format!("Falha ao abrir vault em {}: {}", db_path.display(), e))
    })?;

    if feedback_inject {
        let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
        let feedback_path = feedback_bmad_report_path(&root_dir)?;
        let feedback_text = std::fs::read_to_string(&feedback_path).map_err(|e| {
            io::Error::other(format!(
                "Falha ao ler feedback report em {}: {}",
                feedback_path.display(),
                e
            ))
        })?;
        let raw_blocks = extract_json_blocks_from_feedback(&feedback_text);
        let mut parsed_blocks = Vec::new();
        for raw in raw_blocks {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                parsed_blocks.push(v);
            }
        }
        let selected = select_approved_feedback_payload(&repo_id, &parsed_blocks).ok_or_else(|| {
            io::Error::other(format!(
                "Nenhum payload JSON correspondente encontrado no feedback para repo_id={}",
                repo_id
            ))
        })?;
        let row_val = selected.get("row").cloned().ok_or_else(|| {
            io::Error::other("Payload no feedback sem campo 'row'".to_string())
        })?;
        let just_val = selected
            .get("block3_justifications")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let mut row: genesis_mc_lib::cognition::synthesizer::MasterSolutionsRow =
            serde_json::from_value(row_val).map_err(|e| {
                io::Error::other(format!("Falha ao decodificar MasterSolutionsRow do feedback: {}", e))
            })?;
        row.status_atualizacao = "CONCLUIDO_AGUARDANDO".to_string();
        row.status_fase = "FASE_4_SHEETS_UPDATED".to_string();
        let _ = just_val;
        info!(
            repo_id = %repo_id,
            project_name = %row.project_name,
            repo_url = %row.repo_url,
            lote_id = %row.lote_id,
            "Feedback-inject: payload selecionado do arquivo"
        );

        let row_number = if let Some(row) = row_override {
            row
        } else {
            resolve_row_number_by_repo_url_and_lote_id(&spreadsheet_id, &row.repo_url, &row.lote_id)
                .await?
        };

        let header_row = read_master_header(&spreadsheet_id).await?;
        let project_idx = find_col_idx(&header_row, "project_name").unwrap_or(0);
        let repo_url_idx = find_col_idx(&header_row, "repo_url").unwrap_or(0);
        let lote_id_idx = find_col_idx(&header_row, "lote_id").unwrap_or(0);
        info!(
            row_number,
            header_len = header_row.len(),
            project_idx,
            repo_url_idx,
            lote_id_idx,
            project_header = %header_row.get(project_idx).cloned().unwrap_or_default(),
            repo_url_header = %header_row.get(repo_url_idx).cloned().unwrap_or_default(),
            lote_id_header = %header_row.get(lote_id_idx).cloned().unwrap_or_default(),
            "Feedback-inject: header resolvido"
        );
        let ranges = build_dynamic_sheet_ranges_for_row(row_number, &header_row, &row);
        let project_col = col_idx_to_a1(project_idx);
        let repo_url_col = col_idx_to_a1(repo_url_idx);
        let project_range = format!("{project_col}{row_number}:{project_col}{row_number}");
        let repo_url_range = format!("{repo_url_col}{row_number}:{repo_url_col}{row_number}");
        info!(
            ranges_len = ranges.len(),
            has_project = ranges.contains_key(&project_range),
            has_repo_url = ranges.contains_key(&repo_url_range),
            "Feedback-inject: payload ranges montado"
        );
        let update_result = call_mcp(
            "batch_update_cells",
            json!({
                "spreadsheet_id": spreadsheet_id,
                "sheet": "MASTER_SOLUTIONS",
                "ranges": ranges
            }),
        )
        .await?;
        let mut update_result_str = update_result.to_string();
        if update_result_str.len() > 800 {
            update_result_str.truncate(800);
        }
        info!(payload = %update_result_str, "Feedback-inject: retorno batch_update_cells");
        if let Some(err) = update_result.get("error") {
            return Err(io::Error::other(format!(
                "Feedback-inject: batch_update_cells falhou: {}",
                err
            )));
        }

        let confirmed = confirm_sheet_write(row_number, &repo_id).await?;
        if !confirmed {
            let header_row = read_master_header(&spreadsheet_id).await?;
            let project_idx = find_col_idx(&header_row, "project_name").unwrap_or(0);
            let repo_url_idx = find_col_idx(&header_row, "repo_url").unwrap_or(0);
            let min_idx = project_idx.min(repo_url_idx);
            let max_idx = project_idx.max(repo_url_idx);
            let start_col = col_idx_to_a1(min_idx);
            let end_col = col_idx_to_a1(max_idx);
            let range = format!("{start_col}{row_number}:{end_col}{row_number}");
            let result = call_mcp(
                "get_sheet_data",
                json!({
                    "spreadsheet_id": spreadsheet_id,
                    "sheet": "MASTER_SOLUTIONS",
                    "range": range,
                    "include_grid_data": false
                }),
            )
            .await?;
            let values = extract_values_2d(&result).unwrap_or_default();
            let row_read = values.first().cloned().unwrap_or_default();
            let get = |abs_idx: usize| -> String {
                let rel = abs_idx.saturating_sub(min_idx);
                row_read.get(rel).map(|s| s.trim().to_string()).unwrap_or_default()
            };
            let project_cell = get(project_idx);
            let repo_url_cell = get(repo_url_idx);
            return Err(io::Error::other(format!(
                "Feedback-inject: confirmação falhou. row_number={} project_name_cell='{}' repo_url_cell='{}'",
                row_number, project_cell, repo_url_cell
            )));
        }

        update_local_status_after_manual_f4(&conn, &repo_id)?;

        info!(repo_id = %repo_id, row_number, "Feedback-inject: concluído com confirmação");
        return Ok(());
    }

    if phase4_only {
        let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
        let row = try_fetch_repo_heuristics_row(&conn, &repo_id).ok_or_else(|| {
            io::Error::other(format!(
                "F4-only: repo_heuristics vazio/ausente para repo_id={}",
                repo_id
            ))
        })?;
        let block3_justifications = fetch_block3_justifications(&conn, &repo_id);
        let now = now_epoch_secs()?;
        let row_number = SsotInjector::inject_ssot(&repo_id, row, block3_justifications, now)
            .await
            .map_err(|e| io::Error::other(format!("F4-only: falha ao injetar no Sheets: {}", e)))?;
        let confirmed = confirm_sheet_write(row_number, &repo_id).await?;
        if !confirmed {
            return Err(io::Error::other(
                "F4-only: atualização enviada, mas confirmação via leitura não bateu",
            ));
        }
        let header_row = read_master_header(&spreadsheet_id).await?;
        let cols = resolve_master_cols(&header_row)?;
        update_status_atualizacao_e_fase(
            &spreadsheet_id,
            row_number,
            cols,
            "CONCLUIDO_AGUARDANDO",
            "FASE_4_SHEETS_UPDATED",
        )
        .await?;
        info!(repo_id = %repo_id, row_number, "F4-only: concluído com confirmação");
        return Ok(());
    }

    let (lote_id, repo_url) = fetch_repo_core(&conn, &repo_id).unwrap_or_else(|_| {
        (
            "LOTE_E2E".to_string(),
            format!("https://github.com/{}", repo_id),
        )
    });
    let lote_id = std::env::var("SODA_LOTE_ID_OVERRIDE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or(lote_id);

    let mut n4_skip_columns: Vec<&'static str> = Vec::new();
    let mut n4_sheet_proposta: Option<String> = None;
    let mut n4_sheet_categoria: Option<String> = None;
    if !dry_run && (skip_harvester || !e2e_full) {
        let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
        let row_number = if let Some(row) = row_override {
            row
        } else {
            resolve_row_number_by_repo_url_and_lote_id(&spreadsheet_id, &repo_url, &lote_id).await?
        };
        let header_row = read_master_header(&spreadsheet_id).await?;
        let cols = resolve_master_cols(&header_row)?;
        let (status_atualizacao, status_fase) =
            read_status_atualizacao_e_fase(&spreadsheet_id, row_number, cols).await?;
        let status_ok = status_atualizacao.trim() == "APROVADO_PARA_ENXAME"
            || (status_atualizacao.trim() == "CONCLUIDO_AGUARDANDO"
                && status_fase.trim() == "FASE_4_SHEETS_UPDATED");
        if !status_ok {
            info!(
                repo_id = %repo_id,
                row_number,
                status_atualizacao = %status_atualizacao,
                expected = "APROVADO_PARA_ENXAME",
                "N4/N5: skip (fora do gatilho rígido)"
            );
            return Ok(());
        }
    }

    if e2e_full && skip_harvester {
        let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
        let row_number = if let Some(row) = row_override {
            row
        } else {
            resolve_row_number_by_repo_url_and_lote_id(&spreadsheet_id, &repo_url, &lote_id).await?
        };

        let blobs = count_raw_blobs_distinct(&conn, &repo_id)?;
        if blobs < 11 {
            info!(
                repo_id = %repo_id,
                blobs,
                expected = 11,
                "N4: skip (idempotência: blobs RAW ausentes no SQLite)"
            );
            return Ok(());
        }

        let header_row = read_master_header(&spreadsheet_id).await?;
        if let Some(idx) = find_col_idx(&header_row, "proposta_original_resumo") {
            let v = read_cell_at(&spreadsheet_id, row_number, idx).await?;
            if !v.trim().is_empty() {
                n4_sheet_proposta = Some(v);
                n4_skip_columns.push("proposta_original_resumo");
            }
        }
        if let Some(idx) = find_col_idx(&header_row, "categoria_arquitetural") {
            let v = read_cell_at(&spreadsheet_id, row_number, idx).await?;
            if !v.trim().is_empty() {
                n4_sheet_categoria = Some(v);
                n4_skip_columns.push("categoria_arquitetural");
            }
        }
    }

    if e2e_full && !skip_harvester {
        if let Ok(spreadsheet_id) = std::env::var("GOOGLE_SHEETS_ID") {
            let row_number =
                resolve_row_number_by_repo_url_and_lote_id(&spreadsheet_id, &repo_url, &lote_id).await?;
            let header_row = read_master_header(&spreadsheet_id).await?;
            let cols = resolve_master_cols(&header_row)?;
            let (status_atualizacao, _status_fase) =
                read_status_atualizacao_e_fase(&spreadsheet_id, row_number, cols).await?;
            if status_atualizacao == "PENDENTE_FASE_0" {
                info!(
                    row_number,
                    "Orquestrador: gatilho HITL detectado (PENDENTE_FASE_0). Executando apenas F0"
                );
                let phase0_ms = run_phase_binary("f0_harvester_cli", &repo_id).await?;
                update_status_fase_only(&spreadsheet_id, row_number, cols, "FASE_0_OK").await?;
                info!(
                    phase0_ms,
                    row_number,
                    "Orquestrador: F0 concluída; status_fase atualizado; encerrando sem LLM"
                );
                return Ok(());
            }
        }
    }

    let phase1_ms = if e2e_full && !skip_harvester {
        run_phase_binary("f0_harvester_cli", &repo_id).await?
    } else {
        0
    };
    let phase1_5_ms = if e2e_full {
        run_phase_binary("f1_distiller_cli", &repo_id).await?
    } else {
        0
    };
    let phase2_ms = if e2e_full {
        run_phase_binary("f2_swarm_cli", &repo_id).await?
    } else {
        0
    };

    if e2e_full {
        if let Ok(spreadsheet_id) = std::env::var("GOOGLE_SHEETS_ID") {
            let row_number =
                resolve_row_number_by_repo_url_and_lote_id(&spreadsheet_id, &repo_url, &lote_id).await?;
            SsotInjector::update_single_status_fase(row_number, "FASE_2_ENXAME_OK")
                .await
                .map_err(|e| io::Error::other(format!("Falha no micro-sync F2->F3: {e}")))?;
            info!(
                repo_id = %repo_id,
                row_number,
                "E2E: micro-sync intermediário executado após Fase 2 e antes do SGR"
            );
        }
    }

    let (lens_a, lens_b, lens_c) = fetch_debates(&conn, &repo_id)?;
    let lens_a_report = lens_a.clone();
    let lens_b_report = lens_b.clone();
    let lens_c_report = lens_c.clone();
    let phase2_cost_usd = if e2e_full {
        extract_total_cost_usd_from_lens_json(&lens_a)
            + extract_total_cost_usd_from_lens_json(&lens_b)
            + extract_total_cost_usd_from_lens_json(&lens_c)
    } else {
        0.0
    };

    let seed = try_fetch_repo_heuristics_seed(&conn, &repo_id);
    let (
        seed_repo_analised_version,
        seed_ultima_versao_online,
        seed_licenca,
        seed_stack_base,
        seed_declared_description,
        mut seed_proposta_original_resumo,
        mut seed_categoria_arquitetural,
    ) = seed.unwrap_or_else(|| {
        (
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "".to_string(),
            "".to_string(),
        )
    });
    if let Some(v) = n4_sheet_proposta.as_deref() {
        seed_proposta_original_resumo = v.trim().to_string();
    }
    if let Some(v) = n4_sheet_categoria.as_deref() {
        seed_categoria_arquitetural = v.trim().to_string();
    }

    let repo_kind = detect_repo_kind_from_raw_blobs(&conn, &repo_id);
    seed_categoria_arquitetural = normalize_categoria_arquitetural_seed(&seed_categoria_arquitetural)
        .or_else(|| {
            if repo_kind != "CodeRepo" {
                Some("Knowledge_Extraction".to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let now = now_epoch_secs()?;
    let (repo_analised_version_from_repositorios, ultima_versao_online_from_repositorios) =
        try_fetch_repositorios_release_info(&conn, &repo_id);
    let repo_analised_version = repo_analised_version_from_repositorios
        .or_else(|| {
            let trimmed = seed_repo_analised_version.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .unwrap_or_default();
    let repo_analised_version_lower = repo_analised_version.to_ascii_lowercase();
    if repo_analised_version.trim().is_empty()
        || repo_analised_version_lower == "main"
        || repo_analised_version_lower == "master"
        || is_unknown_like(&repo_analised_version)
    {
        return Err(io::Error::other(
            "repo_analised_version ausente/invalidado. Rode a Fase 0 (Harvester) ou preencha a coluna no SSOT antes de rodar F3/F4.",
        ));
    }
    let mut ultima_versao_online = ultima_versao_online_from_repositorios
        .or_else(|| {
            let trimmed = seed_ultima_versao_online.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .unwrap_or_else(|| "UNKNOWN".to_string());

    if ultima_versao_online.eq_ignore_ascii_case("unknown") || is_unknown_like(&ultima_versao_online) {
        if let Some(tag) = try_fetch_github_latest_release_tag(&repo_url).await {
            ultima_versao_online = tag;
        } else if let Ok(url) = Url::parse(&repo_url) {
            let limiter = RateLimiter;
            if let Ok(meta) = CommunityMetaFetcher::fetch(&url, &limiter).await {
                if let Some(sha) = meta.last_commit_sha {
                    let short = sha.chars().take(7).collect::<String>();
                    if !short.is_empty() {
                        ultima_versao_online = short;
                    }
                }
            }
        }
    }
    info!(
        repo_analised_version = %repo_analised_version,
        ultima_versao_online = %ultima_versao_online,
        "E2E: Bloco 0 (versões) resolvido"
    );

    let mut licenca = seed_licenca.trim().to_string();
    let mut stack_base = seed_stack_base.trim().to_string();
    let mut declared_description = seed_declared_description.trim().to_string();

    if is_unknown_like(&stack_base) {
        if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_02_dependency_manifest") {
            if let Some(derived) = derive_stack_base_from_manifest_blob(&text) {
                stack_base = derived;
            }
        }
    }

    if is_unknown_like(&licenca) {
        if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_09_community_meta") {
            if let Some(derived) = derive_license_from_community_meta_json(&text) {
                licenca = derived;
            }
        }
        if is_unknown_like(&licenca) {
            if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_01_promessa_readme") {
                if let Some(derived) = derive_license_from_readme(&text) {
                    licenca = derived;
                }
            }
        }
    }

    if is_unknown_like(&declared_description) {
        if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_01_promessa_readme") {
            if let Some(derived) = derive_declared_description_from_readme(&text) {
                declared_description = derived;
            }
        }
    }

    info!(
        repo_id = %repo_id,
        licenca = %licenca,
        stack_base = %stack_base,
        declared_description = %declared_description,
        "E2E: sementes do Bloco 0 resolvidas a partir do vault/blobs"
    );
    let block0 = Block0Context {
        status_atualizacao: "EM_PROCESSAMENTO".to_string(),
        status_fase: "F3".to_string(),
        project_name: repo_id.clone(),
        repo_url,
        repo_analised_version,
        ultima_versao_online,
        lote_id: lote_id.clone(),
        data_ultima_analise: now,
        analise_origem: "SODA_E2E_F3".to_string(),
        licenca,
        stack_base,
        declared_description,
        proposta_original_resumo: Some(seed_proposta_original_resumo)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        categoria_arquitetural: Some(seed_categoria_arquitetural)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        lente_a_sentido_prod_ux: lens_a,
        lente_b_estrutura_arq: lens_b,
        lente_c_realidade_ops: lens_c,
    };

    let formatter = OpenRouterFormatterClient::from_env().map_err(io::Error::other)?;
    let formatter_model = std::env::var("OPENROUTER_FORMATTER_MODEL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| OFFICIAL_FORMATTER_MODEL.to_string());
    let cfg = Phase3Config {
        model: formatter_model.clone(),
        max_attempts_per_block: 3,
    };

    let started_phase3_4 = Instant::now();
    let phase3_out = match tokio::time::timeout(SGR_TOTAL_TIMEOUT, run_phase3_sgr(&formatter, &cfg, block0)).await {
        Ok(Ok(out)) => out,
        Ok(Err(Phase3Error::RetryExhausted { block, attempts, message })) => {
            error!(block, attempts, message = %message, "E2E: falha terminal no SGR após retries");
            return Err(io::Error::other("Falha terminal no SGR"));
        }
        Ok(Err(e)) => {
            error!(error = %e, "E2E: falha no SGR");
            return Err(io::Error::other(format!("Falha SGR: {}", e)));
        }
        Err(_) => {
            return Err(io::Error::other(format!(
                "Timeout no SGR total (timeout_s={})",
                SGR_TOTAL_TIMEOUT.as_secs()
            )));
        }
    };

    if dry_run {
        let payload = serde_json::json!({
            "repo_id": repo_id,
            "block3_justifications": phase3_out.block3_justifications,
            "row": phase3_out.row,
            "lens_a_json": lens_a_report,
            "lens_b_json": lens_b_report,
            "lens_c_json": lens_c_report
        });
        let path = append_feedback_bmad_report(&root_dir, &repo_id, &payload)?;
        info!(report = %path.display(), "BMAD E2E: relatório anexado (dry-run)");
        return Ok(());
    }

    info!("E2E: F3 concluída. Iniciando F4 (carga atômica Sheets)");
    let block3_justifications = phase3_out.block3_justifications;
    let mut row = phase3_out.row;
    let _ghost_f4 = spawn_ghost_telemetry(repo_id.clone(), "F4 (SSOT Sheets) em processamento".to_string());
    if let Some(v) = n4_sheet_proposta.as_deref() {
        row.proposta_original_resumo = v.trim().to_string();
    }
    if let Some(v) = n4_sheet_categoria.as_deref() {
        row.categoria_arquitetural =
            genesis_mc_lib::cognition::synthesizer::ArchitecturalCategory::parse_strict(v)
                .unwrap_or(row.categoria_arquitetural);
    }
    if matches!(
        row.categoria_arquitetural,
        genesis_mc_lib::cognition::synthesizer::ArchitecturalCategory::Unspecified
            | genesis_mc_lib::cognition::synthesizer::ArchitecturalCategory::Unknown
    ) {
        let repo_kind = detect_repo_kind_from_raw_blobs(&conn, &repo_id);
        row.categoria_arquitetural = if repo_kind != "CodeRepo" {
            genesis_mc_lib::cognition::synthesizer::ArchitecturalCategory::KnowledgeExtraction
        } else {
            genesis_mc_lib::cognition::synthesizer::ArchitecturalCategory::ToolingDev
        };
    }

    SsotInjector::persist_phase3_snapshot(&repo_id, &row, &block3_justifications, now)
        .map_err(|e| io::Error::other(format!("Falha ao persistir snapshot F3 em SQLite: {}", e)))?;

    let row_number = if n4_skip_columns.is_empty() {
        SsotInjector::inject_ssot(&repo_id, row, block3_justifications, now)
            .await
            .map_err(|e| io::Error::other(format!("Falha na F4 (Carga SSOT Sheets): {}", e)))?
    } else {
        SsotInjector::inject_ssot_with_skip_columns(&repo_id, row, block3_justifications, now, &n4_skip_columns)
            .await
            .map_err(|e| io::Error::other(format!("Falha na F4 (Carga SSOT Sheets): {}", e)))?
    };
    drop(_ghost_f4);

    let confirmed = confirm_sheet_write(row_number, &repo_id).await?;
    if !confirmed {
        return Err(io::Error::other(
            "E2E: atualização enviada, mas confirmação via leitura não bateu",
        ));
    }

    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let header_row = read_master_header(&spreadsheet_id).await?;
    let cols = resolve_master_cols(&header_row)?;
    update_status_atualizacao_e_fase(
        &spreadsheet_id,
        row_number,
        cols,
        "CONCLUIDO_AGUARDANDO",
        "FASE_4_SHEETS_UPDATED",
    )
    .await?;

    let width_a_to_cf = inspect_row_width_a_to_cf(row_number).await?;
    info!(width_a_to_cf, "E2E: inspeção pós-write (A:END) para largura do row");

    let usage = formatter.usage_totals();
    let elapsed_phase3_4_ms = started_phase3_4.elapsed().as_millis();
    info!(
        elapsed_ms = elapsed_phase3_4_ms,
        prompt_tokens = usage.prompt_tokens,
        completion_tokens = usage.completion_tokens,
        total_tokens = usage.total_tokens,
        total_cost_usd = usage.total_cost_usd,
        "E2E: concluído com confirmação de escrita no Sheets"
    );

    let e2e_full_total_cost_usd = phase2_cost_usd + usage.total_cost_usd;
    let e2e_full_total_ms = phase1_ms + phase1_5_ms + phase2_ms + elapsed_phase3_4_ms;
    let report_path = etl_report_path(&root_dir, &repo_id)?;
    let mut report = String::new();
    report.push_str(&format!(
        "\n\n=== FASE 3-4: SGR + SSOT @ {} ===\n\n",
        now_brt_rfc3339()
    ));
    report.push_str(&format!("repo_id={}\n", repo_id));
    report.push_str(&format!("row_number={}\n", row_number));
    report.push_str(&format!("model_used={}\n", formatter_model));
    report.push_str(&format!("lote_id={}\n", lote_id));
    report.push_str(&format!("latency_f3_f4_ms={}\n", elapsed_phase3_4_ms));
    report.push_str(&format!(
        "latency_total_ms={}\n",
        if e2e_full { e2e_full_total_ms } else { elapsed_phase3_4_ms }
    ));
    report.push_str(&format!("prompt_tokens={}\n", usage.prompt_tokens));
    report.push_str(&format!("completion_tokens={}\n", usage.completion_tokens));
    report.push_str(&format!("total_tokens={}\n", usage.total_tokens));
    report.push_str(&format!(
        "total_cost_usd={:.6}\n",
        if e2e_full { e2e_full_total_cost_usd } else { usage.total_cost_usd }
    ));
    report.push_str(&format!("cost_f2_usd={:.6}\n", phase2_cost_usd));
    report.push_str(&format!("cost_f3_f4_usd={:.6}\n", usage.total_cost_usd));
    report.push_str(&format!("sheets_write_confirmed={}\n", confirmed));
    report.push_str(&format!("row_width_a_to_cf={}\n", width_a_to_cf));
    report.push_str(&format!(
        "elapsed_total_wall_ms={}\n",
        started_total.elapsed().as_millis()
    ));

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&report_path)
        .map_err(|e| io::Error::other(format!("Falha ao abrir relatório ETL {}: {}", report_path.display(), e)))?;
    file.write_all(report.as_bytes())
        .map_err(|e| io::Error::other(format!("Falha ao anexar relatório ETL: {}", e)))?;
    info!(report = %report_path.display(), "E2E: relatório ETL anexado");
    Ok(())
}
