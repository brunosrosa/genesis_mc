use crate::cognition::synthesizer::{
    apply_phase4_block5, master_solutions_header_range, sheet_range_for_row, ArchitecturalCategory,
    MasterSolutionsRow, MASTER_SOLUTIONS_CANONICAL_COLUMNS,
};
use thiserror::Error;
use serde_json::{json, Value};
use rusqlite::{Connection, ErrorCode};
use std::collections::{HashMap, HashSet};
use std::env;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use url::Url;
use tracing::{info, warn};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SsotError {
    #[error("Falha na validacao do payload SSOT: {0}")]
    ValidationFailure(String),
    #[error("Falha na persistência L2 (SQLite): {0}")]
    L2Failure(String),
    #[error("Falha no despacho para a nuvem (Sheets): {0}")]
    CloudFailure(String),
    #[error("Config ausente: {0}")]
    ConfigMissing(&'static str),
    #[error("Falha de rede/MCP: {0}")]
    NetworkFailure(String),
}

pub struct SsotInjector;

const SSOT_EXPECTED_COLUMNS: usize = 85;
const MASTER_SOLUTIONS_SHEET: &str = "MASTER_SOLUTIONS";

#[cfg(not(test))]
const MCP_TIMEOUT: Duration = Duration::from_secs(180);
#[cfg(test)]
const MCP_TIMEOUT: Duration = Duration::from_millis(250);

#[cfg(not(test))]
const MCP_CHUNK_DELAY: Duration = Duration::from_secs(1);
#[cfg(test)]
const MCP_CHUNK_DELAY: Duration = Duration::from_millis(1);

#[cfg(not(test))]
const MCP_RELOAD_DELAY: Duration = Duration::from_secs(4);
#[cfg(test)]
const MCP_RELOAD_DELAY: Duration = Duration::from_millis(10);

pub type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
pub type SheetsFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SqliteRetryPolicy {
    max_attempts: usize,
    base_delay_ms: u64,
    max_delay_ms: u64,
    jitter_ms: u64,
}

pub trait SheetsClient: Send + Sync {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a>;

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a>;
}

pub struct McpGoogleSheetsClient;

impl SheetsClient for McpGoogleSheetsClient {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a> {
        Box::pin(async move {
            let result = SsotInjector::call_mcp_google_sheets_tool(
                "get_sheet_data",
                json!({
                    "spreadsheet_id": spreadsheet_id,
                    "sheet": sheet,
                    "range": range,
                    "include_grid_data": false
                }),
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(SsotInjector::extract_values_2d(&result).unwrap_or_default())
        })
    }

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a> {
        Box::pin(async move {
            let call_once = |chunk: Value| async move {
                SsotInjector::call_mcp_google_sheets_tool(
                    "batch_update_cells",
                    json!({
                        "spreadsheet_id": spreadsheet_id,
                        "sheet": sheet,
                        "ranges": chunk
                    }),
                )
                .await
                .map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            };

            let call_with_reload = |chunk: Value| async move {
                match call_once(chunk.clone()).await {
                    Ok(()) => Ok(()),
                    Err(err1) => {
                        warn!(
                            spreadsheet_id = spreadsheet_id,
                            sheet = sheet,
                            error = %err1,
                            "Falha no MCP Sheets; reiniciando sessão (cold-start) antes do retry"
                        );
                        tokio::time::sleep(MCP_RELOAD_DELAY).await;
                        call_once(chunk).await
                    }
                }
            };

            match ranges {
                Value::Object(map) if map.len() > 30 => {
                    let entries: Vec<(String, Value)> = map.into_iter().collect();
                    let total = entries.len();
                    let chunk_count = 3.min(total.max(1));
                    let chunk_size = (total + chunk_count - 1) / chunk_count;

                    for chunk_idx in 0..chunk_count {
                        let start = chunk_idx * chunk_size;
                        if start >= total {
                            break;
                        }
                        let end = ((chunk_idx + 1) * chunk_size).min(total);
                        let mut chunk_map = serde_json::Map::new();
                        for (k, v) in entries[start..end].iter() {
                            chunk_map.insert(k.clone(), v.clone());
                        }
                        info!(
                            spreadsheet_id = spreadsheet_id,
                            sheet = sheet,
                            chunk_idx = chunk_idx,
                            chunk_count = chunk_count,
                            ranges = chunk_map.len(),
                            "Despachando micro-lote para Google Sheets (chunking)"
                        );
                        call_with_reload(Value::Object(chunk_map)).await?;

                        if chunk_idx + 1 < chunk_count {
                            tokio::time::sleep(MCP_CHUNK_DELAY).await;
                        }
                    }
                }
                other => {
                    call_with_reload(other).await?;
                }
            }
            Ok(())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedSsotFields {
    project_name: String,
    repo_url: String,
    repo_analised_version: String,
    ultima_versao_online: String,
    lote_id: String,
    data_ultima_analise: i64,
    analise_origem: String,
    declared_description: String,
    proposta_original_resumo: String,
    stack_base: String,
    licenca: String,
}

impl SsotInjector {
    pub(crate) fn open_vault_connection() -> Result<Connection, SsotError> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let root_dir = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
        Connection::open(&db_path).map_err(|e| SsotError::L2Failure(format!("Falha ao conectar no SQLite: {}", e)))
    }

    pub(crate) fn try_load_repo_heuristics_row(
        repo_id: &str,
    ) -> Result<Option<MasterSolutionsRow>, SsotError> {
        let conn = Self::open_vault_connection()?;
        Self::ensure_repo_heuristics_schema(&conn).map_err(SsotError::L2Failure)?;

        let mut stmt = match conn.prepare(
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
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                return Err(SsotError::L2Failure(format!(
                    "Falha ao preparar SELECT repo_heuristics: {e}"
                )))
            }
        };

        let json_val: Result<Value, _> = stmt.query_row(rusqlite::params![repo_id], |row| {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "status_atualizacao".to_string(),
                serde_json::json!(row.get::<_, String>(0)?),
            );
            obj.insert("status_fase".to_string(), serde_json::json!(row.get::<_, String>(1)?));
            obj.insert("project_name".to_string(), serde_json::json!(row.get::<_, String>(2)?));
            obj.insert("repo_url".to_string(), serde_json::json!(row.get::<_, String>(3)?));
            obj.insert(
                "repo_analised_version".to_string(),
                serde_json::json!(row.get::<_, String>(4)?),
            );
            obj.insert(
                "ultima_versao_online".to_string(),
                serde_json::json!(row.get::<_, String>(5)?),
            );
            obj.insert(
                "indicacao_otimista_canibalizacao".to_string(),
                serde_json::json!(row.get::<_, String>(6)?),
            );
            obj.insert("lote_id".to_string(), serde_json::json!(row.get::<_, String>(7)?));
            obj.insert(
                "data_ultima_analise".to_string(),
                serde_json::json!(row.get::<_, i64>(8)?),
            );
            obj.insert(
                "analise_origem".to_string(),
                serde_json::json!(row.get::<_, String>(9)?),
            );
            obj.insert("licenca".to_string(), serde_json::json!(row.get::<_, String>(10)?));
            obj.insert(
                "stack_base".to_string(),
                serde_json::json!(row.get::<_, String>(11)?),
            );
            obj.insert(
                "declared_description".to_string(),
                serde_json::json!(row.get::<_, String>(12)?),
            );
            obj.insert("declared_description_ptbr".to_string(), serde_json::json!(""));
            obj.insert(
                "proposta_original_resumo".to_string(),
                serde_json::json!(row.get::<_, String>(13)?),
            );
            obj.insert(
                "lente_a_sentido_prod_ux".to_string(),
                serde_json::json!(row.get::<_, String>(14)?),
            );
            obj.insert(
                "lente_b_estrutura_arq".to_string(),
                serde_json::json!(row.get::<_, String>(15)?),
            );
            obj.insert(
                "lente_c_realidade_ops".to_string(),
                serde_json::json!(row.get::<_, String>(16)?),
            );
            obj.insert(
                "visao_do_enxame".to_string(),
                serde_json::json!(row.get::<_, String>(17)?),
            );
            obj.insert(
                "justificativa_decisao".to_string(),
                serde_json::json!(row.get::<_, String>(18)?),
            );
            obj.insert(
                "executive_verdict".to_string(),
                serde_json::json!(row.get::<_, String>(19)?),
            );
            obj.insert(
                "risco_principal".to_string(),
                serde_json::json!(row.get::<_, String>(20)?),
            );
            obj.insert(
                "risco_linha_vermelha".to_string(),
                serde_json::json!(row.get::<_, String>(21)?),
            );
            obj.insert(
                "observacoes".to_string(),
                serde_json::json!(row.get::<_, String>(22)?),
            );
            obj.insert(
                "ouro_a_extrair".to_string(),
                serde_json::json!(row.get::<_, String>(23)?),
            );
            obj.insert(
                "deep_pattern".to_string(),
                serde_json::json!(row.get::<_, String>(24)?),
            );
            obj.insert(
                "transplantable_core".to_string(),
                serde_json::json!(row.get::<_, String>(25)?),
            );
            obj.insert(
                "logic_math_heuristic".to_string(),
                serde_json::json!(row.get::<_, String>(26)?),
            );
            obj.insert(
                "real_structural_problem".to_string(),
                serde_json::json!(row.get::<_, String>(27)?),
            );
            obj.insert(
                "categoria_nuance_tecnica".to_string(),
                serde_json::json!(row.get::<_, String>(28)?),
            );
            obj.insert(
                "integracao_papel_exato".to_string(),
                serde_json::json!(row.get::<_, String>(29)?),
            );
            obj.insert(
                "must_components_prod_ux".to_string(),
                serde_json::json!(row.get::<_, String>(30)?),
            );
            obj.insert(
                "must_components_arq".to_string(),
                serde_json::json!(row.get::<_, String>(31)?),
            );
            obj.insert(
                "must_components_ops".to_string(),
                serde_json::json!(row.get::<_, String>(32)?),
            );
            obj.insert(
                "detected_toxic_deps".to_string(),
                serde_json::json!(row.get::<_, String>(33)?),
            );
            obj.insert(
                "do_not_absorb".to_string(),
                serde_json::json!(row.get::<_, String>(34)?),
            );
            obj.insert(
                "where_ai_should_not_enter".to_string(),
                serde_json::json!(row.get::<_, String>(35)?),
            );
            obj.insert(
                "classificacao_terminal".to_string(),
                serde_json::json!(row.get::<_, String>(36)?),
            );
            obj.insert(
                "acao_de_canibalizacao".to_string(),
                serde_json::json!(row.get::<_, String>(37)?),
            );
            obj.insert(
                "categoria_arquitetural".to_string(),
                serde_json::json!(row.get::<_, String>(38)?),
            );
            obj.insert(
                "horizonte_extracao".to_string(),
                serde_json::json!(row.get::<_, String>(39)?),
            );
            obj.insert(
                "tipo_integracao".to_string(),
                serde_json::json!(row.get::<_, String>(40)?),
            );
            obj.insert(
                "capability_nature_primary".to_string(),
                serde_json::json!(row.get::<_, String>(41)?),
            );
            obj.insert(
                "architectural_topology".to_string(),
                serde_json::json!(row.get::<_, String>(42)?),
            );
            obj.insert(
                "temporal_stability".to_string(),
                serde_json::json!(row.get::<_, String>(43)?),
            );
            obj.insert(
                "bare_metal_fit".to_string(),
                serde_json::json!(row.get::<_, String>(44)?),
            );
            obj.insert(
                "extractability_level".to_string(),
                serde_json::json!(row.get::<_, String>(45)?),
            );
            obj.insert(
                "runtime_sovereignty_fit".to_string(),
                serde_json::json!(row.get::<_, String>(46)?),
            );
            obj.insert(
                "local_first_fit".to_string(),
                serde_json::json!(row.get::<_, String>(47)?),
            );
            obj.insert(
                "adoptability_level".to_string(),
                serde_json::json!(row.get::<_, String>(48)?),
            );
            obj.insert(
                "longitudinal_sustainability".to_string(),
                serde_json::json!(row.get::<_, String>(49)?),
            );
            obj.insert(
                "maintenance_burden".to_string(),
                serde_json::json!(row.get::<_, String>(50)?),
            );
            obj.insert(
                "onboarding_friction".to_string(),
                serde_json::json!(row.get::<_, String>(51)?),
            );
            obj.insert(
                "observability_operational".to_string(),
                serde_json::json!(row.get::<_, String>(52)?),
            );
            obj.insert(
                "recoverability_level".to_string(),
                serde_json::json!(row.get::<_, String>(53)?),
            );
            obj.insert(
                "degradation_behavior".to_string(),
                serde_json::json!(row.get::<_, String>(54)?),
            );
            obj.insert(
                "curation_burden".to_string(),
                serde_json::json!(row.get::<_, String>(55)?),
            );
            obj.insert(
                "evolution_cost".to_string(),
                serde_json::json!(row.get::<_, String>(56)?),
            );
            obj.insert(
                "operability_level".to_string(),
                serde_json::json!(row.get::<_, String>(57)?),
            );
            obj.insert(
                "abandonment_risk".to_string(),
                serde_json::json!(row.get::<_, String>(58)?),
            );
            obj.insert(
                "time_to_first_clear_value".to_string(),
                serde_json::json!(row.get::<_, String>(59)?),
            );
            obj.insert(
                "imperfection_tolerance".to_string(),
                serde_json::json!(row.get::<_, String>(60)?),
            );
            obj.insert(
                "entropy_risk".to_string(),
                serde_json::json!(row.get::<_, String>(61)?),
            );
            obj.insert(
                "design_misuse_risk".to_string(),
                serde_json::json!(row.get::<_, String>(62)?),
            );
            obj.insert(
                "intrinsic_ethics_risk".to_string(),
                serde_json::json!(row.get::<_, String>(63)?),
            );
            obj.insert(
                "discipline_dependency".to_string(),
                serde_json::json!(row.get::<_, String>(64)?),
            );
            obj.insert(
                "regulatory_risk".to_string(),
                serde_json::json!(row.get::<_, String>(65)?),
            );
            obj.insert(
                "score_philosophical_fit".to_string(),
                serde_json::json!(row.get::<_, i64>(66)?),
            );
            obj.insert(
                "score_bare_metal_fit".to_string(),
                serde_json::json!(row.get::<_, i64>(67)?),
            );
            obj.insert(
                "score_architectural_extractability".to_string(),
                serde_json::json!(row.get::<_, i64>(68)?),
            );
            obj.insert(
                "score_operability".to_string(),
                serde_json::json!(row.get::<_, i64>(69)?),
            );
            obj.insert(
                "score_creep_risk".to_string(),
                serde_json::json!(row.get::<_, i64>(70)?),
            );
            obj.insert(
                "score_runtime_sovereignty".to_string(),
                serde_json::json!(row.get::<_, i64>(71)?),
            );
            obj.insert(
                "score_model_logic_value".to_string(),
                serde_json::json!(row.get::<_, i64>(72)?),
            );
            obj.insert(
                "score_ethics_safety".to_string(),
                serde_json::json!(row.get::<_, i64>(73)?),
            );
            obj.insert(
                "score_intrinsic_risk".to_string(),
                serde_json::json!(row.get::<_, i64>(74)?),
            );
            obj.insert(
                "score_final".to_string(),
                serde_json::json!(row.get::<_, f64>(75)?),
            );
            obj.insert(
                "score_fit_geral_soda".to_string(),
                serde_json::json!(row.get::<_, f64>(76)?),
            );
            obj.insert(
                "score_architectural_priority".to_string(),
                serde_json::json!(row.get::<_, f64>(77)?),
            );
            obj.insert(
                "score_human_product_priority".to_string(),
                serde_json::json!(row.get::<_, f64>(78)?),
            );
            obj.insert(
                "score_absorption_readiness".to_string(),
                serde_json::json!(row.get::<_, f64>(79)?),
            );
            obj.insert(
                "score_operational_priority".to_string(),
                serde_json::json!(row.get::<_, f64>(80)?),
            );
            obj.insert(
                "score_sustainability_adjusted_fit".to_string(),
                serde_json::json!(row.get::<_, f64>(81)?),
            );
            obj.insert(
                "valid_from".to_string(),
                serde_json::json!(row.get::<_, i64>(82)?),
            );
            obj.insert(
                "valid_to".to_string(),
                serde_json::json!(row.get::<_, Option<i64>>(83)?),
            );
            obj.insert(
                "embargo_status".to_string(),
                serde_json::json!(row.get::<_, i64>(84)?),
            );
            Ok(serde_json::Value::Object(obj))
        });

        match json_val {
            Ok(value) => serde_json::from_value::<MasterSolutionsRow>(value)
                .map(Some)
                .map_err(|e| SsotError::L2Failure(format!("Falha ao decodificar MasterSolutionsRow do SQLite: {e}"))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SsotError::L2Failure(format!(
                "Falha ao ler repo_heuristics do SQLite: {e}"
            ))),
        }
    }

    pub(crate) fn load_block3_justifications(
        repo_id: &str,
    ) -> Result<HashMap<String, String>, SsotError> {
        let conn = Self::open_vault_connection()?;
        Self::ensure_repo_heuristics_justifications_schema(&conn).map_err(SsotError::L2Failure)?;
        let json_text: Result<String, _> = conn.query_row(
            "SELECT justifications_json
             FROM repo_heuristics_justifications
             WHERE project_name = ?1 AND block = 3
             LIMIT 1",
            rusqlite::params![repo_id],
            |row| row.get::<_, String>(0),
        );
        match json_text {
            Ok(text) => Ok(serde_json::from_str::<HashMap<String, String>>(&text).unwrap_or_default()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(HashMap::new()),
            Err(e) => Err(SsotError::L2Failure(format!(
                "Falha ao ler justifications do Bloco 3 no SQLite: {e}"
            ))),
        }
    }

    fn load_l2_curated_overrides(
        repo_id: &str,
    ) -> Result<(Option<String>, Option<String>), SsotError> {
        let conn = Self::open_vault_connection()?;
        let out: Result<(String, String), _> = conn.query_row(
            "SELECT proposta_original_resumo, categoria_arquitetural
             FROM repo_heuristics
             WHERE project_name = ?1
             LIMIT 1",
            rusqlite::params![repo_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        let (proposta, categoria) = match out {
            Ok(v) => v,
            Err(_) => return Ok((None, None)),
        };
        let proposta = proposta.trim().to_string();
        let proposta = (!proposta.is_empty()).then_some(proposta);
        let categoria = categoria.trim().to_string();
        let categoria = (!categoria.is_empty() && !categoria.eq_ignore_ascii_case("unknown")).then_some(categoria);
        Ok((proposta, categoria))
    }

    fn header_idx(header_row: &[String], canonical: &str) -> Option<usize> {
        header_row.iter().enumerate().find_map(|(idx, raw)| {
            (Self::normalize_header_cell(raw) == canonical).then_some(idx)
        })
    }

    async fn read_sheet_cell(
        client: &dyn SheetsClient,
        spreadsheet_id: &str,
        sheet: &str,
        row_number_1based: u32,
        header_row: &[String],
        canonical_col: &str,
    ) -> Result<String, SsotError> {
        let Some(idx) = Self::header_idx(header_row, canonical_col) else {
            return Ok(String::new());
        };
        let col = Self::col_idx_to_a1(idx);
        let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
        let values = client
            .get_sheet_data(spreadsheet_id, sheet, range)
            .await
            .map_err(SsotError::CloudFailure)?;
        Ok(values
            .first()
            .and_then(|r| r.first())
            .map(|s| s.trim().to_string())
            .unwrap_or_default())
    }
    fn should_short_circuit(status_atualizacao: &str) -> bool {
        status_atualizacao.trim().starts_with("REJEITADO_")
    }

    fn status_fase_to_persist<'a>(status_atualizacao: &str, status_fase: &'a str) -> &'a str {
        if Self::should_short_circuit(status_atualizacao) {
            "SHORT-CIRCUIT"
        } else {
            status_fase
        }
    }

    fn retry_sqlite_busy<T, F>(
        policy: SqliteRetryPolicy,
        mut op: F,
    ) -> Result<T, rusqlite::Error>
    where
        F: FnMut() -> Result<T, rusqlite::Error>,
    {
        let attempts = policy.max_attempts.max(1);
        let mut delay_ms = policy.base_delay_ms.max(1);
        let max_delay_ms = policy.max_delay_ms.max(delay_ms);

        for attempt in 1..=attempts {
            match op() {
                Ok(value) => return Ok(value),
                Err(err) => {
                    let is_busy = match &err {
                        rusqlite::Error::SqliteFailure(ffi_err, _) => matches!(
                            ffi_err.code,
                            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                        ),
                        _ => false,
                    };
                    if !is_busy || attempt == attempts {
                        return Err(err);
                    }

                    let jitter = if policy.jitter_ms > 0 {
                        rand::random::<u64>() % (policy.jitter_ms + 1)
                    } else {
                        0
                    };
                    let sleep_ms = (delay_ms.saturating_add(jitter)).min(max_delay_ms);
                    std::thread::sleep(Duration::from_millis(sleep_ms));
                    delay_ms = (delay_ms.saturating_mul(2)).min(max_delay_ms);
                }
            }
        }

        Err(rusqlite::Error::InvalidQuery)
    }

    fn cleanup_artefatos_brutos_for_repo_id(
        conn: &Connection,
        repo_id: &str,
        policy: SqliteRetryPolicy,
    ) -> Result<usize, String> {
        let out = Self::retry_sqlite_busy(policy, || {
            conn.execute("DELETE FROM artefatos_brutos WHERE repo_id = ?1", [repo_id])
        });
        match out {
            Ok(deleted) => Ok(deleted),
            Err(e) => Err(e.to_string()),
        }
    }
    /// Injeta os dados no SSOT (SQLite + Google Sheets Batch)
    pub async fn inject_ssot(
        repo_id: &str,
        mut row: MasterSolutionsRow,
        block3_justifications: HashMap<String, String>,
        now_epoch: i64,
    ) -> Result<u32, SsotError> {
        let validated = Self::validate_payload(repo_id, &row)?;

        apply_phase4_block5(now_epoch, &mut row);

        let spreadsheet_id =
            env::var("GOOGLE_SHEETS_ID").map_err(|_| SsotError::ConfigMissing("GOOGLE_SHEETS_ID"))?;
        let sheet = "MASTER_SOLUTIONS";
        let row_number_1based =
            Self::resolve_row_number_by_repo_url(
                &spreadsheet_id,
                sheet,
                &validated.repo_url,
            )
            .await?;

        let client = McpGoogleSheetsClient;
        let header_row = Self::load_master_solutions_header(&client, &spreadsheet_id).await?;
        let (l2_proposta, l2_categoria) = Self::load_l2_curated_overrides(repo_id)?;
        let sheet_proposta = Self::read_sheet_cell(
            &client,
            &spreadsheet_id,
            sheet,
            row_number_1based,
            &header_row,
            "proposta_original_resumo",
        )
        .await?;
        let sheet_categoria = Self::read_sheet_cell(
            &client,
            &spreadsheet_id,
            sheet,
            row_number_1based,
            &header_row,
            "categoria_arquitetural",
        )
        .await?;

        if !sheet_proposta.is_empty() {
            row.proposta_original_resumo = sheet_proposta.clone();
        } else if let Some(v) = l2_proposta.as_deref() {
            if !v.trim().is_empty() {
                row.proposta_original_resumo = v.trim().to_string();
            }
        }
        if !sheet_categoria.is_empty() {
            if let Ok(cat) = ArchitecturalCategory::parse_strict(&sheet_categoria) {
                if !matches!(cat, ArchitecturalCategory::Unknown | ArchitecturalCategory::Unspecified) {
                    row.categoria_arquitetural = cat;
                }
            }
        } else if let Some(v) = l2_categoria.as_deref() {
            if let Ok(cat) = ArchitecturalCategory::parse_strict(v) {
                if !matches!(cat, ArchitecturalCategory::Unknown | ArchitecturalCategory::Unspecified) {
                    row.categoria_arquitetural = cat;
                }
            }
        }
        let lote_idx = header_row
            .iter()
            .enumerate()
            .find_map(|(idx, raw)| (Self::normalize_header_cell(raw) == "lote_id").then_some(idx))
            .ok_or_else(|| SsotError::CloudFailure("Header missing lote_id".to_string()))?;
        let lote_col = Self::col_idx_to_a1(lote_idx);
        let lote_range = format!("{lote_col}{row_number_1based}:{lote_col}{row_number_1based}");
        let lote_values = client
            .get_sheet_data(&spreadsheet_id, sheet, lote_range)
            .await
            .map_err(SsotError::CloudFailure)?;
        let lote_cell = lote_values
            .first()
            .and_then(|r| r.first())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let mut dynamic_skip: Vec<&'static str> = Vec::new();
        if !lote_cell.is_empty() {
            dynamic_skip.push("lote_id");
        }
        if !sheet_proposta.is_empty() {
            dynamic_skip.push("proposta_original_resumo");
        }
        if !sheet_categoria.is_empty() {
            dynamic_skip.push("categoria_arquitetural");
        }
        let batch_payload =
            Self::prepare_batch_payload_dynamic_with_skip(row_number_1based, &header_row, &row, &dynamic_skip)?;

        match Self::dispatch_to_cloud(batch_payload).await {
            Ok(()) => {
                row.status_fase = "FASE_4_SHEETS_UPDATED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Ok(row_number_1based)
            }
            Err(err) => {
                row.status_fase = "FASE_4_CLOUD_FAILED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Err(err)
            }
        }
    }

    pub async fn inject_ssot_with_skip_columns(
        repo_id: &str,
        mut row: MasterSolutionsRow,
        block3_justifications: HashMap<String, String>,
        now_epoch: i64,
        skip_columns: &[&'static str],
    ) -> Result<u32, SsotError> {
        let validated = Self::validate_payload(repo_id, &row)?;

        apply_phase4_block5(now_epoch, &mut row);

        let spreadsheet_id =
            env::var("GOOGLE_SHEETS_ID").map_err(|_| SsotError::ConfigMissing("GOOGLE_SHEETS_ID"))?;
        let sheet = "MASTER_SOLUTIONS";
        let row_number_1based =
            Self::resolve_row_number_by_repo_url(&spreadsheet_id, sheet, &validated.repo_url).await?;

        let client = McpGoogleSheetsClient;
        let header_row = Self::load_master_solutions_header(&client, &spreadsheet_id).await?;
        let (l2_proposta, l2_categoria) = Self::load_l2_curated_overrides(repo_id)?;
        let sheet_proposta = Self::read_sheet_cell(
            &client,
            &spreadsheet_id,
            sheet,
            row_number_1based,
            &header_row,
            "proposta_original_resumo",
        )
        .await?;
        let sheet_categoria = Self::read_sheet_cell(
            &client,
            &spreadsheet_id,
            sheet,
            row_number_1based,
            &header_row,
            "categoria_arquitetural",
        )
        .await?;

        if !sheet_proposta.is_empty() {
            row.proposta_original_resumo = sheet_proposta.clone();
        } else if let Some(v) = l2_proposta.as_deref() {
            if !v.trim().is_empty() {
                row.proposta_original_resumo = v.trim().to_string();
            }
        }
        if !sheet_categoria.is_empty() {
            if let Ok(cat) = ArchitecturalCategory::parse_strict(&sheet_categoria) {
                if !matches!(cat, ArchitecturalCategory::Unknown | ArchitecturalCategory::Unspecified) {
                    row.categoria_arquitetural = cat;
                }
            }
        } else if let Some(v) = l2_categoria.as_deref() {
            if let Ok(cat) = ArchitecturalCategory::parse_strict(v) {
                if !matches!(cat, ArchitecturalCategory::Unknown | ArchitecturalCategory::Unspecified) {
                    row.categoria_arquitetural = cat;
                }
            }
        }
        let lote_idx = header_row
            .iter()
            .enumerate()
            .find_map(|(idx, raw)| (Self::normalize_header_cell(raw) == "lote_id").then_some(idx))
            .ok_or_else(|| SsotError::CloudFailure("Header missing lote_id".to_string()))?;
        let lote_col = Self::col_idx_to_a1(lote_idx);
        let lote_range = format!("{lote_col}{row_number_1based}:{lote_col}{row_number_1based}");
        let lote_values = client
            .get_sheet_data(&spreadsheet_id, sheet, lote_range)
            .await
            .map_err(SsotError::CloudFailure)?;
        let lote_cell = lote_values
            .first()
            .and_then(|r| r.first())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let mut merged_skip: Vec<&'static str> = skip_columns.iter().copied().collect();
        if !lote_cell.is_empty() && !merged_skip.contains(&"lote_id") {
            merged_skip.push("lote_id");
        }
        if !sheet_proposta.is_empty() && !merged_skip.contains(&"proposta_original_resumo") {
            merged_skip.push("proposta_original_resumo");
        }
        if !sheet_categoria.is_empty() && !merged_skip.contains(&"categoria_arquitetural") {
            merged_skip.push("categoria_arquitetural");
        }
        let batch_payload = Self::prepare_batch_payload_dynamic_with_skip(
            row_number_1based,
            &header_row,
            &row,
            &merged_skip,
        )?;

        match Self::dispatch_to_cloud(batch_payload).await {
            Ok(()) => {
                row.status_fase = "FASE_4_SHEETS_UPDATED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Ok(row_number_1based)
            }
            Err(err) => {
                row.status_fase = "FASE_4_CLOUD_FAILED".to_string();
                Self::update_local_status(
                    repo_id,
                    row.status_atualizacao.as_str(),
                    &row,
                    &validated,
                    &block3_justifications,
                    now_epoch,
                )
                .map_err(SsotError::L2Failure)?;
                Err(err)
            }
        }
    }

    pub async fn inject_ssot_from_db(repo_id: &str, now_epoch: i64) -> Result<u32, SsotError> {
        let Some(mut row) = Self::try_load_repo_heuristics_row(repo_id)? else {
            return Err(SsotError::L2Failure(format!(
                "Outbox ausente: repo_heuristics não encontrado para repo_id={}",
                repo_id
            )));
        };
        let block3_justifications = Self::load_block3_justifications(repo_id)?;

        let validated = Self::validate_payload(repo_id, &row)?;
        apply_phase4_block5(now_epoch, &mut row);

        let spreadsheet_id =
            env::var("GOOGLE_SHEETS_ID").map_err(|_| SsotError::ConfigMissing("GOOGLE_SHEETS_ID"))?;
        let sheet = "MASTER_SOLUTIONS";
        let row_number_1based =
            Self::resolve_row_number_by_repo_url(&spreadsheet_id, sheet, &validated.repo_url).await?;

        let client = McpGoogleSheetsClient;
        let header_row = Self::load_master_solutions_header(&client, &spreadsheet_id).await?;
        let (l2_proposta, l2_categoria) = Self::load_l2_curated_overrides(repo_id)?;
        let sheet_proposta = Self::read_sheet_cell(
            &client,
            &spreadsheet_id,
            sheet,
            row_number_1based,
            &header_row,
            "proposta_original_resumo",
        )
        .await?;
        let sheet_categoria = Self::read_sheet_cell(
            &client,
            &spreadsheet_id,
            sheet,
            row_number_1based,
            &header_row,
            "categoria_arquitetural",
        )
        .await?;

        if !sheet_proposta.is_empty() {
            row.proposta_original_resumo = sheet_proposta.clone();
        } else if let Some(v) = l2_proposta.as_deref() {
            if !v.trim().is_empty() {
                row.proposta_original_resumo = v.trim().to_string();
            }
        }
        if !sheet_categoria.is_empty() {
            if let Ok(cat) = ArchitecturalCategory::parse_strict(&sheet_categoria) {
                if !matches!(cat, ArchitecturalCategory::Unknown | ArchitecturalCategory::Unspecified) {
                    row.categoria_arquitetural = cat;
                }
            }
        } else if let Some(v) = l2_categoria.as_deref() {
            if let Ok(cat) = ArchitecturalCategory::parse_strict(v) {
                if !matches!(cat, ArchitecturalCategory::Unknown | ArchitecturalCategory::Unspecified) {
                    row.categoria_arquitetural = cat;
                }
            }
        }

        let lote_idx = header_row
            .iter()
            .enumerate()
            .find_map(|(idx, raw)| (Self::normalize_header_cell(raw) == "lote_id").then_some(idx))
            .ok_or_else(|| SsotError::CloudFailure("Header missing lote_id".to_string()))?;
        let lote_col = Self::col_idx_to_a1(lote_idx);
        let lote_range = format!("{lote_col}{row_number_1based}:{lote_col}{row_number_1based}");
        let lote_values = client
            .get_sheet_data(&spreadsheet_id, sheet, lote_range)
            .await
            .map_err(SsotError::CloudFailure)?;
        let lote_cell = lote_values
            .first()
            .and_then(|r| r.first())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let mut dynamic_skip: Vec<&'static str> = Vec::new();
        if !lote_cell.is_empty() {
            dynamic_skip.push("lote_id");
        }
        if !sheet_proposta.is_empty() {
            dynamic_skip.push("proposta_original_resumo");
        }
        if !sheet_categoria.is_empty() {
            dynamic_skip.push("categoria_arquitetural");
        }
        let batch_payload =
            Self::prepare_batch_payload_dynamic_with_skip(row_number_1based, &header_row, &row, &dynamic_skip)?;

        {
            let conn = Self::open_vault_connection()?;
            Self::ensure_repo_heuristics_schema(&conn).map_err(SsotError::L2Failure)?;
            let _ = conn.execute(
                "UPDATE repo_heuristics
                 SET score_final = ?2,
                     score_fit_geral_soda = ?3,
                     score_architectural_priority = ?4,
                     score_human_product_priority = ?5,
                     score_absorption_readiness = ?6,
                     score_operational_priority = ?7,
                     score_sustainability_adjusted_fit = ?8,
                     valid_from = ?9,
                     valid_to = ?10,
                     embargo_status = ?11
                 WHERE project_name = ?1",
                rusqlite::params![
                    repo_id,
                    row.score_final,
                    row.score_fit_geral_soda,
                    row.score_architectural_priority,
                    row.score_human_product_priority,
                    row.score_absorption_readiness,
                    row.score_operational_priority,
                    row.score_sustainability_adjusted_fit,
                    row.valid_from,
                    row.valid_to,
                    row.embargo_status
                ],
            );
        }

        match Self::dispatch_to_cloud(batch_payload).await {
            Ok(()) => {
                let conn = Self::open_vault_connection()?;
                let _ = conn.execute(
                    "UPDATE repo_heuristics
                     SET status_atualizacao = ?2,
                         status_fase = ?3
                     WHERE project_name = ?1",
                    rusqlite::params![repo_id, "CONCLUIDO_AGUARDANDO", "FASE_4_SHEETS_UPDATED"],
                );
                let _ = conn.execute(
                    "UPDATE repositorios
                     SET status_processamento = ?1
                     WHERE project_name = ?2",
                    rusqlite::params!["CONCLUIDO", repo_id],
                );
                let _ = Self::checkpoint_upsert_repo_heuristics_full(
                    repo_id,
                    &row,
                    "CONCLUIDO_AGUARDANDO",
                    "FASE_4_SHEETS_UPDATED",
                    &block3_justifications,
                    now_epoch,
                );
                Ok(row_number_1based)
            }
            Err(err) => {
                let conn = Self::open_vault_connection()?;
                let _ = conn.execute(
                    "UPDATE repo_heuristics
                     SET status_fase = ?2
                     WHERE project_name = ?1",
                    rusqlite::params![repo_id, "ERRO_FASE_4"],
                );
                let _ = conn.execute(
                    "UPDATE repositorios
                     SET status_processamento = ?1
                     WHERE project_name = ?2",
                    rusqlite::params!["ERRO_FASE_4", repo_id],
                );
                Err(err)
            }
        }
    }

    async fn resolve_row_number_by_repo_url(
        spreadsheet_id: &str,
        sheet: &str,
        repo_url: &str,
    ) -> Result<u32, SsotError> {
        let client = McpGoogleSheetsClient;
        let header_row = Self::load_master_solutions_header(&client, spreadsheet_id).await?;
        let repo_url_idx = header_row
            .iter()
            .enumerate()
            .find_map(|(idx, raw)| (Self::normalize_header_cell(raw) == "repo_url").then_some(idx))
            .ok_or_else(|| {
                SsotError::CloudFailure(format!(
                    "Header missing repo_url (headers_len={})",
                    header_row.len()
                ))
            })?;
        let col = Self::col_idx_to_a1(repo_url_idx);
        let range = format!("{col}2:{col}");
        let values = client
            .get_sheet_data(spreadsheet_id, sheet, range)
            .await
            .map_err(SsotError::CloudFailure)?;

        let needle = repo_url.trim_end_matches('/').to_ascii_lowercase();
        if let Some(found) = Self::resolve_row_number_from_repo_url_column(&values, &needle) {
            return Ok(found);
        }

        let mut non_empty_examples: Vec<String> = Vec::new();
        for row in values.iter().take(500) {
            let v = row.first().map(|s| s.trim()).unwrap_or("");
            if v.is_empty() {
                continue;
            }
            non_empty_examples.push(v.to_string());
            if non_empty_examples.len() >= 5 {
                break;
            }
        }
        Err(SsotError::CloudFailure(format!(
            "Linha SSOT não encontrada por match perfeito repo_url='{}' (repo_url_col={} idx0={} headers_len={} examples={:?}). Append é proibido; abortando.",
            repo_url,
            col,
            repo_url_idx,
            header_row.len(),
            non_empty_examples
        )))
    }

    fn resolve_row_number_from_repo_url_column(
        values: &[Vec<String>],
        repo_url_needle: &str,
    ) -> Option<u32> {
        for (idx, row) in values.iter().enumerate() {
            let repo_cell = row.first().map(|s| s.trim()).unwrap_or("");
            let repo_hay = repo_cell.trim_end_matches('/').to_ascii_lowercase();
            if !repo_hay.is_empty() && repo_hay == repo_url_needle {
                return Some((idx as u32) + 2);
            }
        }
        None
    }

    async fn call_mcp_google_sheets_tool(
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, SsotError> {
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        let creds = env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| SsotError::ConfigMissing("GOOGLE_APPLICATION_CREDENTIALS"))?;

        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": { "name": "genesis-mc", "version": "1.0" },
                "capabilities": {}
            }
        });
        let initialized_notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let mcp_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": tool_name, "arguments": arguments }
        });

        let mut child = Command::new("mcp-google-sheets")
            .env("GOOGLE_APPLICATION_CREDENTIALS", &creds)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SsotError::NetworkFailure(format!("Falha ao spawnar mcp-google-sheets: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(format!("{}\n", init_req).as_bytes())
                .await
                .map_err(|e| SsotError::NetworkFailure(format!("Falha ao escrever init no MCP: {}", e)))?;
            stdin
                .write_all(format!("{}\n", initialized_notif).as_bytes())
                .await
                .map_err(|e| SsotError::NetworkFailure(format!("Falha ao escrever initialized no MCP: {}", e)))?;
            stdin
                .write_all(format!("{}\n", mcp_request).as_bytes())
                .await
                .map_err(|e| SsotError::NetworkFailure(format!("Falha ao escrever tools/call no MCP: {}", e)))?;
        }

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| SsotError::NetworkFailure("stdout indisponível".to_string()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| SsotError::NetworkFailure("stderr indisponível".to_string()))?;
        let (status, stdout_buf, stderr_buf) =
            match tokio::time::timeout(MCP_TIMEOUT, async {
                use tokio::io::AsyncReadExt;
                let mut out_buf = Vec::new();
                let mut err_buf = Vec::new();
                let (out_res, err_res, status_res) = tokio::join!(
                    stdout.read_to_end(&mut out_buf),
                    stderr.read_to_end(&mut err_buf),
                    child.wait()
                );
                out_res.map_err(|e| SsotError::NetworkFailure(format!("Falha ao ler stdout MCP: {}", e)))?;
                err_res.map_err(|e| SsotError::NetworkFailure(format!("Falha ao ler stderr MCP: {}", e)))?;
                let status =
                    status_res.map_err(|e| SsotError::NetworkFailure(format!("Falha ao aguardar processo MCP: {}", e)))?;
                Ok::<_, SsotError>((status, out_buf, err_buf))
            })
            .await
            {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    let _ = child.kill().await;
                    return Err(SsotError::NetworkFailure(format!(
                        "Timeout aguardando mcp-google-sheets tool={} timeout_s={}",
                        tool_name,
                        MCP_TIMEOUT.as_secs()
                    )));
                }
            };
        if !status.success() {
            return Err(SsotError::NetworkFailure(format!(
                "Falha no processo MCP. Exit {}. STDERR: {}",
                status,
                String::from_utf8_lossy(&stderr_buf)
            )));
        }

        let stdout_str = String::from_utf8_lossy(&stdout_buf);
        Self::parse_mcp_tool_stdout(&stdout_str, 2).map_err(SsotError::NetworkFailure)
    }

    fn parse_mcp_tool_stdout(stdout: &str, expected_id: i64) -> Result<Value, String> {
        for line in stdout.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let id = value.get("id").and_then(|v| v.as_i64());
            if id != Some(expected_id) {
                continue;
            }
            if let Some(err) = value.get("error") {
                return Err(format!("MCP error: {}", err));
            }
            let Some(result) = value.get("result") else {
                return Err("MCP missing result".to_string());
            };
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                let mut combined = String::new();
                for part in content {
                    if part.get("type").and_then(|t| t.as_str()) != Some("text") {
                        continue;
                    }
                    let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    if !text.trim().is_empty() {
                        if !combined.is_empty() {
                            combined.push('\n');
                        }
                        combined.push_str(text);
                    }
                }
                if !combined.trim().is_empty() {
                    if let Ok(json_val) = serde_json::from_str::<Value>(&combined) {
                        return Ok(json_val);
                    }
                    return Ok(json!({ "text": combined }));
                }
            }
            return Ok(result.clone());
        }
        Err("MCP tool response not found in stdout".to_string())
    }

    fn extract_values_2d(value: &Value) -> Option<Vec<Vec<String>>> {
        if let Some(values) = value.get("values").and_then(|v| v.as_array()) {
            return Some(Self::parse_values_array(values));
        }
        if let Some(vrs) = value.get("valueRanges").and_then(|v| v.as_array()) {
            if let Some(first) = vrs.first() {
                if let Some(values) = first.get("values").and_then(|v| v.as_array()) {
                    return Some(Self::parse_values_array(values));
                }
            }
        }
        if let Some(result) = value.get("result") {
            return Self::extract_values_2d(result);
        }
        if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                return Self::extract_values_2d(&parsed);
            }
        }
        None
    }

    fn parse_values_array(values: &[Value]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|row| {
                row.as_array()
                    .map(|r| {
                        r.iter()
                            .map(|cell| cell.as_str().unwrap_or("").to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
    }

    fn validate_payload(
        repo_id: &str,
        payload: &MasterSolutionsRow,
    ) -> Result<ValidatedSsotFields, SsotError> {
        if payload.categoria_arquitetural == crate::cognition::synthesizer::ArchitecturalCategory::Unknown {
            return Err(SsotError::ValidationFailure(
                "categoria_arquitetural invalida (fora do catálogo de 10 ENUMs)".to_string(),
            ));
        }
        let _ = crate::cognition::synthesizer::ArchitecturalCategory::parse_strict(
            payload.categoria_arquitetural.as_str(),
        )
        .map_err(SsotError::ValidationFailure)?;

        let project_name = Self::require_non_empty("project_name", &payload.project_name)?;
        if project_name != repo_id {
            return Err(SsotError::ValidationFailure(format!(
                "project_name divergente do repo_id: esperado '{}', recebido '{}'",
                repo_id, project_name
            )));
        }

        let repo_url = Self::require_non_empty("repo_url", &payload.repo_url)?;
        let parsed_repo_url = Url::parse(&repo_url)
            .map_err(|e| SsotError::ValidationFailure(format!("repo_url invalido: {}", e)))?;
        let expected_repo_url = format!("https://github.com/{}", repo_id);
        if parsed_repo_url.as_str().trim_end_matches('/') != expected_repo_url {
            return Err(SsotError::ValidationFailure(format!(
                "repo_url divergente do repo_id: esperado '{}', recebido '{}'",
                expected_repo_url, parsed_repo_url
            )));
        }

        let repo_analised_version = Self::require_non_empty(
            "repo_analised_version",
            &payload.repo_analised_version,
        )?;
        let repo_analised_version_lower = repo_analised_version.to_ascii_lowercase();
        if repo_analised_version_lower == "main" || repo_analised_version_lower == "master" {
            return Err(SsotError::ValidationFailure(format!(
                "repo_analised_version invalido (branch): '{}'",
                repo_analised_version
            )));
        }
        let ultima_versao_online =
            Self::require_non_empty("ultima_versao_online", &payload.ultima_versao_online)?;
        let lote_id = Self::require_non_empty("lote_id", &payload.lote_id)?;
        let data_ultima_analise = if payload.data_ultima_analise > 0 {
            payload.data_ultima_analise
        } else {
            return Err(SsotError::ValidationFailure(
                "data_ultima_analise ausente ou invalida".to_string(),
            ));
        };
        let analise_origem = Self::require_non_empty("analise_origem", &payload.analise_origem)?;
        let declared_description =
            Self::require_non_empty("declared_description", &payload.declared_description)?;
        let proposta_original_resumo = Self::require_non_empty(
            "proposta_original_resumo",
            &payload.proposta_original_resumo,
        )?;
        let stack_base = Self::require_non_empty("stack_base", &payload.stack_base)?;
        let licenca = Self::require_non_empty("licenca", &payload.licenca)?;

        Ok(ValidatedSsotFields {
            project_name,
            repo_url,
            repo_analised_version,
            ultima_versao_online,
            lote_id,
            data_ultima_analise,
            analise_origem,
            declared_description,
            proposta_original_resumo,
            stack_base,
            licenca,
        })
    }

    fn require_non_empty(field: &str, value: &str) -> Result<String, SsotError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(SsotError::ValidationFailure(format!(
                "campo obrigatorio '{}' ausente ou vazio",
                field
            )));
        }

        Ok(trimmed.to_string())
    }

    fn update_local_status(
        repo_id: &str,
        status_value: &str,
        payload: &MasterSolutionsRow,
        validated: &ValidatedSsotFields,
        block3_justifications: &HashMap<String, String>,
        now_epoch: i64,
    ) -> Result<(), String> {
        let conn = Self::open_vault_connection()
            .map_err(|e| format!("Falha ao conectar no SQLite: {e:?}"))?;

        Self::ensure_repo_heuristics_schema(&conn)?;
        Self::ensure_repo_heuristics_justifications_schema(&conn)?;

        let status_fase_to_persist =
            Self::status_fase_to_persist(&payload.status_atualizacao, &payload.status_fase)
                .to_string();

        Self::upsert_repo_heuristics_row_internal(
            &conn,
            repo_id,
            payload,
            validated,
            payload.status_atualizacao.as_str(),
            &status_fase_to_persist,
            block3_justifications,
            now_epoch,
        )?;

        // Atualizando o status em repositorios
        let _ = conn.execute(
            "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
            rusqlite::params![status_value, repo_id],
        )
        .map_err(|e| format!("Falha ao executar UPDATE repositorios: {}", e))?;

        Ok(())
    }

    pub(crate) fn checkpoint_upsert_repo_heuristics_full(
        repo_id: &str,
        payload: &MasterSolutionsRow,
        status_atualizacao: &str,
        status_fase: &str,
        block3_justifications: &HashMap<String, String>,
        now_epoch: i64,
    ) -> Result<(), SsotError> {
        let validated = Self::validate_payload(repo_id, payload)?;
        let conn = Self::open_vault_connection()?;
        Self::ensure_repo_heuristics_schema(&conn).map_err(SsotError::L2Failure)?;
        Self::ensure_repo_heuristics_justifications_schema(&conn).map_err(SsotError::L2Failure)?;
        let status_fase_to_persist =
            Self::status_fase_to_persist(status_atualizacao, status_fase).to_string();
        Self::upsert_repo_heuristics_row_internal(
            &conn,
            repo_id,
            payload,
            &validated,
            status_atualizacao,
            &status_fase_to_persist,
            block3_justifications,
            now_epoch,
        )
        .map_err(SsotError::L2Failure)?;
        Ok(())
    }

    fn upsert_repo_heuristics_row_internal(
        conn: &Connection,
        repo_id: &str,
        payload: &MasterSolutionsRow,
        validated: &ValidatedSsotFields,
        status_atualizacao_to_persist: &str,
        status_fase_to_persist: &str,
        block3_justifications: &HashMap<String, String>,
        now_epoch: i64,
    ) -> Result<(), String> {
        let repo_version_to_persist = {
            let primary = payload.repo_analised_version.trim();
            if !primary.is_empty() {
                primary.to_string()
            } else {
                let fallback = payload.ultima_versao_online.trim();
                if !fallback.is_empty() {
                    fallback.to_string()
                } else {
                    "unknown".to_string()
                }
            }
        };

        let existing_curated: Result<(String, String), _> = conn.query_row(
            "SELECT proposta_original_resumo, categoria_arquitetural
             FROM repo_heuristics
             WHERE project_name = ?1
             LIMIT 1",
            rusqlite::params![repo_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        let mut proposta_original_resumo_to_persist = payload.proposta_original_resumo.trim().to_string();
        let mut categoria_arquitetural_to_persist = payload.categoria_arquitetural.as_str().to_string();
        if let Ok((proposta, categoria)) = existing_curated {
            let proposta = proposta.trim();
            if !proposta.is_empty() {
                proposta_original_resumo_to_persist = proposta.to_string();
            }
            let categoria = categoria.trim();
            if !categoria.is_empty() {
                categoria_arquitetural_to_persist = categoria.to_string();
            }
        }

        // I/O L2 Real: Mapeando SgrPayload para as colunas reais da tabela
        conn.execute(
            "INSERT OR REPLACE INTO repo_heuristics (
                project_name, status_atualizacao, status_fase, repo_url, repo_analised_version, repo_version, ultima_versao_online, lote_id, data_ultima_analise, analise_origem, declared_description, proposta_original_resumo, stack_base, licenca, lente_a_sentido_prod_ux, lente_b_estrutura_arq, lente_c_realidade_ops, visao_do_enxame, justificativa_decisao, executive_verdict, classificacao_terminal, acao_de_canibalizacao, categoria_arquitetural, horizonte_extracao, tipo_integracao, categoria_nuance_tecnica, integracao_papel_exato, ouro_a_extrair, deep_pattern, transplantable_core, logic_math_heuristic, real_structural_problem, must_components_prod_ux, must_components_arq, must_components_ops, detected_toxic_deps, do_not_absorb, where_ai_should_not_enter, bare_metal_fit, extractability_level, operability_level, entropy_risk, design_misuse_risk, intrinsic_ethics_risk, discipline_dependency, risco_principal, risco_linha_vermelha, observacoes, score_final, score_fit_geral_soda, score_philosophical_fit, score_bare_metal_fit, score_architectural_extractability, score_operability, score_creep_risk, score_runtime_sovereignty, score_model_logic_value, score_ethics_safety, score_intrinsic_risk, capability_nature_primary, architectural_topology, runtime_sovereignty_fit, local_first_fit, temporal_stability, adoptability_level, longitudinal_sustainability, abandonment_risk, maintenance_burden, onboarding_friction, observability_operational, recoverability_level, degradation_behavior, curation_burden, time_to_first_clear_value, imperfection_tolerance, evolution_cost, regulatory_risk, score_architectural_priority, score_human_product_priority, score_absorption_readiness, score_operational_priority, score_sustainability_adjusted_fit, valid_from, valid_to, embargo_status, indicacao_otimista_canibalizacao
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, ?81, ?82, ?83, ?84, ?85, ?86
            )",
            rusqlite::params![
                &validated.project_name,
                status_atualizacao_to_persist,
                &status_fase_to_persist,
                &validated.repo_url,
                &validated.repo_analised_version,
                &repo_version_to_persist,
                &payload.ultima_versao_online,
                &payload.lote_id,
                payload.data_ultima_analise,
                &payload.analise_origem,
                &payload.declared_description,
                &proposta_original_resumo_to_persist,
                &payload.stack_base,
                &payload.licenca,
                &payload.lente_a_sentido_prod_ux,
                &payload.lente_b_estrutura_arq,
                &payload.lente_c_realidade_ops,
                &payload.visao_do_enxame,
                &payload.justificativa_decisao,
                &payload.executive_verdict,
                payload.classificacao_terminal.as_str(),
                payload.acao_de_canibalizacao.as_str(),
                &categoria_arquitetural_to_persist,
                payload.horizonte_extracao.as_str(),
                payload.tipo_integracao.as_str(),
                &payload.categoria_nuance_tecnica,
                &payload.integracao_papel_exato,
                &payload.ouro_a_extrair,
                &payload.deep_pattern,
                &payload.transplantable_core,
                &payload.logic_math_heuristic,
                &payload.real_structural_problem,
                &payload.must_components_prod_ux,
                &payload.must_components_arq,
                &payload.must_components_ops,
                &payload.detected_toxic_deps,
                &payload.do_not_absorb,
                &payload.where_ai_should_not_enter,
                payload.bare_metal_fit.as_str(),
                payload.extractability_level.as_str(),
                payload.operability_level.as_str(),
                payload.entropy_risk.as_str(),
                payload.design_misuse_risk.as_str(),
                payload.intrinsic_ethics_risk.as_str(),
                payload.discipline_dependency.as_str(),
                &payload.risco_principal,
                &payload.risco_linha_vermelha,
                &payload.observacoes,
                payload.score_final,
                payload.score_fit_geral_soda,
                payload.score_philosophical_fit,
                payload.score_bare_metal_fit,
                payload.score_architectural_extractability,
                payload.score_operability,
                payload.score_creep_risk,
                payload.score_runtime_sovereignty,
                payload.score_model_logic_value,
                payload.score_ethics_safety,
                payload.score_intrinsic_risk,
                payload.capability_nature_primary.as_str(),
                payload.architectural_topology.as_str(),
                payload.runtime_sovereignty_fit.as_str(),
                payload.local_first_fit.as_str(),
                payload.temporal_stability.as_str(),
                payload.adoptability_level.as_str(),
                payload.longitudinal_sustainability.as_str(),
                payload.abandonment_risk.as_str(),
                payload.maintenance_burden.as_str(),
                payload.onboarding_friction.as_str(),
                payload.observability_operational.as_str(),
                payload.recoverability_level.as_str(),
                payload.degradation_behavior.as_str(),
                payload.curation_burden.as_str(),
                payload.time_to_first_clear_value.as_str(),
                payload.imperfection_tolerance.as_str(),
                payload.evolution_cost.as_str(),
                payload.regulatory_risk.as_str(),
                payload.score_architectural_priority,
                payload.score_human_product_priority,
                payload.score_absorption_readiness,
                payload.score_operational_priority,
                payload.score_sustainability_adjusted_fit,
                payload.valid_from,
                payload.valid_to,
                payload.embargo_status,
                &payload.indicacao_otimista_canibalizacao,
            ],
        ).map_err(|e| format!("Falha ao executar INSERT repo_heuristics: {}", e))?;

        if status_fase_to_persist == "SHORT-CIRCUIT" {
            let policy = SqliteRetryPolicy {
                max_attempts: 5,
                base_delay_ms: 25,
                max_delay_ms: 400,
                jitter_ms: 50,
            };
            if let Err(err) = Self::cleanup_artefatos_brutos_for_repo_id(&conn, repo_id, policy) {
                info!(repo_id = %repo_id, error = %err, "SHORT-CIRCUIT: cleanup de blobs falhou (fail-soft)");
            }
        }

        if !block3_justifications.is_empty() {
            let json_text =
                serde_json::to_string(block3_justifications).unwrap_or_else(|_| "{}".to_string());
            conn.execute(
                "INSERT INTO repo_heuristics_justifications (project_name, block, justifications_json, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(project_name, block) DO UPDATE SET
                    justifications_json = excluded.justifications_json,
                    created_at = excluded.created_at",
                rusqlite::params![repo_id, 3_i64, json_text, now_epoch],
            )
            .map_err(|e| format!("Falha ao persistir justifications do Bloco 3 em SQLite: {}", e))?;
        }
        Ok(())
    }

    pub fn persist_phase3_snapshot(
        repo_id: &str,
        row: &MasterSolutionsRow,
        block3_justifications: &HashMap<String, String>,
        now_epoch: i64,
    ) -> Result<(), SsotError> {
        let mut snapshot = row.clone();
        snapshot.status_atualizacao = "CONCLUIDO_AGUARDANDO".to_string();
        snapshot.status_fase = "FASE_3_SYNTHESIZER_OK".to_string();
        let validated = Self::validate_payload(repo_id, &snapshot)?;
        Self::update_local_status(
            repo_id,
            snapshot.status_atualizacao.as_str(),
            &snapshot,
            &validated,
            block3_justifications,
            now_epoch,
        )
        .map_err(SsotError::L2Failure)
    }

    pub(crate) fn ensure_repo_heuristics_schema(conn: &Connection) -> Result<(), String> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repo_heuristics (
                project_name TEXT PRIMARY KEY,
                status_atualizacao TEXT NOT NULL,
                status_fase TEXT NOT NULL,
                repo_url TEXT NOT NULL,
                repo_analised_version TEXT NOT NULL,
                repo_version TEXT,
                ultima_versao_online TEXT,
                indicacao_otimista_canibalizacao TEXT NOT NULL DEFAULT '',
                lote_id TEXT NOT NULL,
                data_ultima_analise INTEGER NOT NULL,
                analise_origem TEXT NOT NULL,
                declared_description TEXT NOT NULL,
                proposta_original_resumo TEXT NOT NULL,
                stack_base TEXT NOT NULL,
                licenca TEXT,
                lente_a_sentido_prod_ux TEXT,
                lente_b_estrutura_arq TEXT,
                lente_c_realidade_ops TEXT,
                visao_do_enxame TEXT NOT NULL,
                justificativa_decisao TEXT NOT NULL,
                executive_verdict TEXT NOT NULL,
                classificacao_terminal TEXT NOT NULL,
                acao_de_canibalizacao TEXT NOT NULL,
                categoria_arquitetural TEXT NOT NULL,
                horizonte_extracao TEXT NOT NULL,
                tipo_integracao TEXT NOT NULL,
                categoria_nuance_tecnica TEXT NOT NULL,
                integracao_papel_exato TEXT NOT NULL,
                ouro_a_extrair TEXT NOT NULL,
                deep_pattern TEXT NOT NULL,
                transplantable_core TEXT NOT NULL,
                logic_math_heuristic TEXT NOT NULL,
                real_structural_problem TEXT NOT NULL,
                must_components_prod_ux TEXT NOT NULL,
                must_components_arq TEXT NOT NULL,
                must_components_ops TEXT NOT NULL,
                detected_toxic_deps TEXT NOT NULL,
                do_not_absorb TEXT NOT NULL,
                where_ai_should_not_enter TEXT NOT NULL,
                bare_metal_fit TEXT NOT NULL,
                extractability_level TEXT NOT NULL,
                operability_level TEXT NOT NULL,
                entropy_risk TEXT NOT NULL,
                design_misuse_risk TEXT NOT NULL,
                intrinsic_ethics_risk TEXT NOT NULL,
                discipline_dependency TEXT NOT NULL,
                risco_principal TEXT NOT NULL,
                risco_linha_vermelha TEXT NOT NULL,
                observacoes TEXT NOT NULL,
                score_final REAL NOT NULL,
                score_fit_geral_soda REAL NOT NULL,
                score_philosophical_fit INTEGER NOT NULL,
                score_bare_metal_fit INTEGER NOT NULL,
                score_architectural_extractability INTEGER NOT NULL,
                score_operability INTEGER NOT NULL,
                score_creep_risk INTEGER NOT NULL,
                score_runtime_sovereignty INTEGER NOT NULL,
                score_model_logic_value INTEGER NOT NULL,
                score_ethics_safety INTEGER NOT NULL,
                score_intrinsic_risk INTEGER NOT NULL,
                capability_nature_primary TEXT NOT NULL,
                architectural_topology TEXT NOT NULL,
                runtime_sovereignty_fit TEXT NOT NULL,
                local_first_fit TEXT NOT NULL,
                temporal_stability TEXT NOT NULL,
                adoptability_level TEXT NOT NULL,
                longitudinal_sustainability TEXT NOT NULL,
                abandonment_risk TEXT NOT NULL,
                maintenance_burden TEXT NOT NULL,
                onboarding_friction TEXT NOT NULL,
                observability_operational TEXT NOT NULL,
                recoverability_level TEXT NOT NULL,
                degradation_behavior TEXT NOT NULL,
                curation_burden TEXT NOT NULL,
                time_to_first_clear_value TEXT NOT NULL,
                imperfection_tolerance TEXT NOT NULL,
                evolution_cost TEXT NOT NULL,
                regulatory_risk TEXT NOT NULL,
                score_architectural_priority REAL NOT NULL,
                score_human_product_priority REAL NOT NULL,
                score_absorption_readiness REAL NOT NULL,
                score_operational_priority REAL NOT NULL,
                score_sustainability_adjusted_fit REAL NOT NULL,
                valid_from INTEGER NOT NULL,
                valid_to INTEGER,
                embargo_status INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Falha ao criar tabela repo_heuristics: {}", e))?;

        let mut stmt = conn
            .prepare("PRAGMA table_info('repo_heuristics')")
            .map_err(|e| format!("Falha ao preparar PRAGMA table_info(repo_heuristics): {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("Falha ao executar PRAGMA table_info(repo_heuristics): {e}"))?;

        let mut has_status_atualizacao = false;
        let mut has_status_fase = false;
        let mut has_repo_analised_version = false;
        let mut has_repo_version = false;
        let mut has_indicacao_otimista = false;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Falha ao iterar PRAGMA table_info(repo_heuristics): {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("Falha ao ler coluna name do PRAGMA table_info: {e}"))?;
            match name.as_str() {
                "status_atualizacao" => has_status_atualizacao = true,
                "status_fase" => has_status_fase = true,
                "repo_analised_version" => has_repo_analised_version = true,
                "repo_version" => has_repo_version = true,
                "indicacao_otimista_canibalizacao" => has_indicacao_otimista = true,
                _ => {}
            }
        }

        if !has_status_atualizacao {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN status_atualizacao TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna status_atualizacao: {e}"))?;
        }
        if !has_status_fase {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN status_fase TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna status_fase: {e}"))?;
        }
        if !has_repo_analised_version {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN repo_analised_version TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna repo_analised_version: {e}"))?;
        }
        if !has_repo_version {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN repo_version TEXT",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna repo_version (legado): {e}"))?;
        }
        if !has_indicacao_otimista {
            conn.execute(
                "ALTER TABLE repo_heuristics ADD COLUMN indicacao_otimista_canibalizacao TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(|e| format!("Falha ao adicionar coluna indicacao_otimista_canibalizacao: {e}"))?;
        }

        let _ = conn.execute(
            "UPDATE repo_heuristics
             SET repo_analised_version = repo_version
             WHERE (repo_analised_version IS NULL OR repo_analised_version = '')
               AND repo_version IS NOT NULL
               AND repo_version != ''",
            [],
        );

        Ok(())
    }

    pub(crate) fn ensure_repo_heuristics_justifications_schema(conn: &Connection) -> Result<(), String> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repo_heuristics_justifications (
                project_name TEXT NOT NULL,
                block INTEGER NOT NULL,
                justifications_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_name, block)
            )",
            [],
        )
        .map_err(|e| format!("Falha ao criar tabela repo_heuristics_justifications: {}", e))?;
        Ok(())
    }

    fn normalize_header_cell(raw: &str) -> String {
        raw.trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "_")
    }

    fn col_idx_to_a1(idx_0based: usize) -> String {
        let mut n = (idx_0based + 1) as u32;
        let mut s = String::new();
        while n > 0 {
            let rem = ((n - 1) % 26) as u8;
            s.insert(0, (b'A' + rem) as char);
            n = (n - 1) / 26;
        }
        s
    }

    async fn load_master_solutions_header(
        sheets: &dyn SheetsClient,
        spreadsheet_id: &str,
    ) -> Result<Vec<String>, SsotError> {
        let raw = sheets
            .get_sheet_data(
                spreadsheet_id,
                MASTER_SOLUTIONS_SHEET,
                master_solutions_header_range(),
            )
            .await
            .map_err(SsotError::CloudFailure)?;
        Ok(raw.first().cloned().unwrap_or_default())
    }

    fn validate_sheet_row_values(row_values_by_name: &HashMap<&'static str, Value>) -> Result<(), SsotError> {
        let valid_from_ok = row_values_by_name
            .get("valid_from")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !valid_from_ok {
            return Err(SsotError::ValidationFailure(
                "valid_from invalido no payload do Sheets".to_string(),
            ));
        }
        let valid_to_ok = row_values_by_name
            .get("valid_to")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().is_empty() || s.contains('-'))
            .unwrap_or(false);
        if !valid_to_ok {
            return Err(SsotError::ValidationFailure(
                "valid_to invalido no payload do Sheets".to_string(),
            ));
        }
        let embargo_ok = row_values_by_name
            .get("embargo_status")
            .and_then(|v| v.as_str())
            .map(|s| s == "LIVRE" || s == "EMBARGADO")
            .unwrap_or(false);
        if !embargo_ok {
            return Err(SsotError::ValidationFailure(
                "embargo_status invalido no payload do Sheets".to_string(),
            ));
        }
        Ok(())
    }

    fn prepare_batch_payload_dynamic(
        row_number_1based: u32,
        header_row: &[String],
        payload: &MasterSolutionsRow,
    ) -> Result<Value, SsotError> {
        Self::prepare_batch_payload_dynamic_with_skip(row_number_1based, header_row, payload, &[])
    }

    fn prepare_batch_payload_dynamic_with_skip(
        row_number_1based: u32,
        header_row: &[String],
        payload: &MasterSolutionsRow,
        skip_columns: &[&'static str],
    ) -> Result<Value, SsotError> {
        let row_values = payload.to_sheet_row();
        if row_values.len() != SSOT_EXPECTED_COLUMNS {
            return Err(SsotError::ValidationFailure(format!(
                "Payload interno desalinhado: esperado {} colunas, recebeu {}",
                SSOT_EXPECTED_COLUMNS,
                row_values.len()
            )));
        }

        let mut by_name: HashMap<&'static str, Value> = HashMap::with_capacity(SSOT_EXPECTED_COLUMNS);
        for (idx, name) in MASTER_SOLUTIONS_CANONICAL_COLUMNS.iter().enumerate() {
            by_name.insert(*name, row_values[idx].clone());
        }
        if let Some(v) = by_name.get("repo_analised_version").cloned() {
            by_name.insert("repo_version", v);
        }
        Self::validate_sheet_row_values(&by_name)?;

        let mut canonical_set: HashSet<&'static str> =
            MASTER_SOLUTIONS_CANONICAL_COLUMNS.iter().copied().collect();
        canonical_set.insert("repo_version");
        let mut skip_set: HashSet<&'static str> = HashSet::new();
        for s in skip_columns {
            skip_set.insert(*s);
        }
        let mut map = serde_json::Map::new();
        for (idx, raw) in header_row.iter().enumerate() {
            let key = Self::normalize_header_cell(raw);
            if key.is_empty() {
                continue;
            }
            if !canonical_set.contains(key.as_str()) {
                continue;
            }
            if skip_set.contains(key.as_str()) {
                continue;
            }
            let Some(value) = by_name.get(key.as_str()) else { continue };
            let col = Self::col_idx_to_a1(idx);
            let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
            map.insert(range, json!(vec![vec![value]]));
        }

        if map.is_empty() {
            return Err(SsotError::ValidationFailure(
                "HeaderResolver não encontrou colunas conhecidas para escrita".to_string(),
            ));
        }

        info!(
            ranges = map.len(),
            sheet_range = %sheet_range_for_row(row_number_1based),
            "Payload dinâmico do Google Sheets montado (late-binding por header)"
        );
        Ok(Value::Object(map))
    }

    pub async fn dispatch_master_solutions_row(
        sheets: &dyn SheetsClient,
        spreadsheet_id: &str,
        row_number_1based: u32,
        row: &MasterSolutionsRow,
    ) -> Result<(), SsotError> {
        let header_row = Self::load_master_solutions_header(sheets, spreadsheet_id).await?;
        let payload = Self::prepare_batch_payload_dynamic(row_number_1based, &header_row, row)?;
        sheets
            .batch_update_cells(spreadsheet_id, MASTER_SOLUTIONS_SHEET, payload)
            .await
            .map_err(SsotError::CloudFailure)
    }

    pub async fn update_single_status_fase(
        row_number_1based: u32,
        new_status: &str,
    ) -> Result<(), SsotError> {
        let spreadsheet_id =
            env::var("GOOGLE_SHEETS_ID").map_err(|_| SsotError::ConfigMissing("GOOGLE_SHEETS_ID"))?;
        let client = McpGoogleSheetsClient;
        let header_row = Self::load_master_solutions_header(&client, &spreadsheet_id).await?;
        let status_idx = Self::header_idx(&header_row, "status_fase").ok_or_else(|| {
            SsotError::CloudFailure("Header missing status_fase".to_string())
        })?;
        let col = Self::col_idx_to_a1(status_idx);
        let range = format!("{col}{row_number_1based}:{col}{row_number_1based}");
        client
            .batch_update_cells(
                &spreadsheet_id,
                MASTER_SOLUTIONS_SHEET,
                json!({
                    range: [[new_status]]
                }),
            )
            .await
            .map_err(SsotError::CloudFailure)?;
        info!(
            row_number = row_number_1based,
            status_fase = new_status,
            "SSOT: micro-sync de status_fase concluído"
        );
        Ok(())
    }

    async fn dispatch_to_cloud(payload: Value) -> Result<(), SsotError> {
        let sheets_id = env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| SsotError::CloudFailure("Missing GOOGLE_SHEETS_ID".to_string()))?;
        let client = McpGoogleSheetsClient;
        client
            .batch_update_cells(&sheets_id, MASTER_SOLUTIONS_SHEET, payload)
            .await
            .map_err(SsotError::CloudFailure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::ffi::Error as FfiError;
    use rusqlite::{params, Error as RusqliteError, ErrorCode};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn resolve_row_number_never_returns_row_1() {
        let values = vec![vec!["https://github.com/acme/widget".to_string()]];
        let needle = "https://github.com/acme/widget".to_string();
        let row =
            SsotInjector::resolve_row_number_from_repo_url_column(&values, &needle).unwrap();
        assert!(row >= 2);
        assert_eq!(row, 2);
    }

    #[test]
    fn resolve_row_number_returns_none_when_repo_url_not_found() {
        let values = vec![
            vec!["https://github.com/acme/a".to_string()],
            vec!["".to_string()],
        ];
        let needle = "https://github.com/acme/unknown".to_string();
        assert!(SsotInjector::resolve_row_number_from_repo_url_column(&values, &needle).is_none());
    }

    #[test]
    fn test_prepare_batch_payload_is_85_columns_a_to_cg() {
        let row = MasterSolutionsRow {
            status_atualizacao: "CONCLUIDO".to_string(),
            status_fase: "F4".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SGR".to_string(),
            declared_description: "Descricao".to_string(),
            proposta_original_resumo: "Resumo".to_string(),
            stack_base: "Rust".to_string(),
            licenca: "MIT".to_string(),
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            ..Default::default()
        };

        let validated = SsotInjector::validate_payload("owner/repo", &row).unwrap();
        let header_row: Vec<String> = MASTER_SOLUTIONS_CANONICAL_COLUMNS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let batch = SsotInjector::prepare_batch_payload_dynamic(2, &header_row, &row).unwrap();
        let obj = batch.as_object().unwrap();

        assert_eq!(
            obj.get("C2:C2").unwrap(),
            &json!(vec![vec![json!("owner / repo")]])
        );
        assert_eq!(
            obj.get("D2:D2").unwrap(),
            &json!(vec![vec![json!("https://github.com/owner/repo")]])
        );
        assert!(obj
            .get("CE2:CE2")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty() && s.contains('-'))
            .unwrap_or(false));
        assert_eq!(obj.get("CF2:CF2").unwrap(), &json!(vec![vec![json!("")]]));
        assert_eq!(
            obj.get("CG2:CG2").unwrap(),
            &json!(vec![vec![json!("LIVRE")]])
        );
        assert_eq!(validated.project_name, "owner/repo");
    }

    #[test]
    fn prepare_batch_payload_respects_header_reordering_by_index() {
        let row = MasterSolutionsRow {
            status_atualizacao: "CONCLUIDO".to_string(),
            status_fase: "F4".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SGR".to_string(),
            declared_description: "Descricao".to_string(),
            proposta_original_resumo: "Resumo".to_string(),
            stack_base: "Rust".to_string(),
            licenca: "MIT".to_string(),
            score_final: 9.4,
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            ..Default::default()
        };
        let mut header_row: Vec<String> = MASTER_SOLUTIONS_CANONICAL_COLUMNS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let score_idx = header_row
            .iter()
            .position(|s| s == "score_final")
            .expect("score_final missing in canonical header");
        let score_col = header_row.remove(score_idx);
        header_row.insert(10, score_col);

        let batch = SsotInjector::prepare_batch_payload_dynamic(2, &header_row, &row).unwrap();
        let obj = batch.as_object().unwrap();
        assert_eq!(obj.get("K2:K2").unwrap(), &json!(vec![vec![json!("9.4")]]));
    }

    #[test]
    fn test_payload_validation_rejects_missing_required_fields() {
        let row = MasterSolutionsRow::default();
        let result = SsotInjector::validate_payload("owner/repo", &row);
        assert!(matches!(result, Err(SsotError::ValidationFailure(_))));
    }

    #[test]
    fn categoria_arquitetural_rejects_value_outside_10_enums_fail_closed() {
        let row = MasterSolutionsRow {
            status_atualizacao: "INICIAR_TRIAGEM".to_string(),
            status_fase: "FASE_-0.5_BATEDOR_OK".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SGR".to_string(),
            declared_description: "Descricao".to_string(),
            proposta_original_resumo: "Resumo".to_string(),
            stack_base: "Rust".to_string(),
            licenca: "MIT".to_string(),
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            categoria_arquitetural: crate::cognition::synthesizer::ArchitecturalCategory::Unknown,
            ..Default::default()
        };

        let out = SsotInjector::validate_payload("owner/repo", &row);
        assert!(matches!(out, Err(SsotError::ValidationFailure(_))));
    }

    #[test]
    fn rejected_lixo_toxico_triggers_systemic_short_circuit_status_fase() {
        let out = SsotInjector::status_fase_to_persist(
            "REJEITADO_LIXO_TOXICO",
            "FASE_0_HARVESTER_OK",
        );
        assert_eq!(out, "SHORT-CIRCUIT");
    }

    fn busy_err() -> RusqliteError {
        RusqliteError::SqliteFailure(
            FfiError {
                code: ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            Some("SQLITE_BUSY".to_string()),
        )
    }

    fn setup_artefatos_brutos_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE artefatos_brutos (
                artifact_id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                payload_blob BLOB NOT NULL,
                timestamp_extracao INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn short_circuit_cleanup_deletes_only_target_repo_blobs() {
        let conn = setup_artefatos_brutos_db();
        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
            params!["a/b", "blob_01", vec![1_u8], 1_i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
            params!["a/b", "blob_02", vec![2_u8], 2_i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
            params!["c/d", "blob_01", vec![3_u8], 3_i64],
        )
        .unwrap();

        let policy = SqliteRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 10,
            max_delay_ms: 50,
            jitter_ms: 10,
        };
        let out = SsotInjector::cleanup_artefatos_brutos_for_repo_id(&conn, "a/b", policy);
        assert!(out.is_ok());

        let remaining_a: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM artefatos_brutos WHERE repo_id = ?1",
                params!["a/b"],
                |row| row.get(0),
            )
            .unwrap();
        let remaining_c: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM artefatos_brutos WHERE repo_id = ?1",
                params!["c/d"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_a, 0);
        assert_eq!(remaining_c, 1);
    }

    #[test]
    fn sqlite_busy_retry_applies_backoff_and_finishes_successfully() {
        let policy = SqliteRetryPolicy {
            max_attempts: 5,
            base_delay_ms: 1,
            max_delay_ms: 5,
            jitter_ms: 1,
        };
        let mut calls = 0usize;
        let res = SsotInjector::retry_sqlite_busy(policy, || {
            calls += 1;
            if calls < 3 {
                Err(busy_err())
            } else {
                Ok(42usize)
            }
        });
        assert!(res.is_ok());
        assert_eq!(calls, 3);
    }

    #[test]
    fn sqlite_busy_retry_aborts_without_infinite_loop() {
        let policy = SqliteRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 1,
            max_delay_ms: 5,
            jitter_ms: 1,
        };
        let mut calls = 0usize;
        let res: Result<usize, _> = SsotInjector::retry_sqlite_busy(policy, || {
            calls += 1;
            Err(busy_err())
        });
        assert!(res.is_err());
        assert_eq!(calls, 3);
    }

    struct MockSheetsClient {
        calls: Arc<Mutex<Vec<(String, String, Value)>>>,
        header: Vec<String>,
    }

    impl MockSheetsClient {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                header: MASTER_SOLUTIONS_CANONICAL_COLUMNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            }
        }
    }

    impl SheetsClient for MockSheetsClient {
        fn get_sheet_data<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            range: String,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>> {
            Box::pin(async move {
                if range.contains("1:") {
                    Ok(vec![self.header.clone()])
                } else {
                    Ok(Vec::new())
                }
            })
        }

        fn batch_update_cells<'a>(
            &'a self,
            spreadsheet_id: &'a str,
            sheet: &'a str,
            ranges: Value,
        ) -> SheetsFuture<'a> {
            Box::pin(async move {
                self.calls.lock().await.push((
                    spreadsheet_id.to_string(),
                    sheet.to_string(),
                    ranges,
                ));
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn dispatch_uses_mock_sheets_client() {
        let row = MasterSolutionsRow {
            status_atualizacao: "CONCLUIDO".to_string(),
            status_fase: "F4".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SGR".to_string(),
            declared_description: "Descricao".to_string(),
            proposta_original_resumo: "Resumo".to_string(),
            stack_base: "Rust".to_string(),
            licenca: "MIT".to_string(),
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            ..Default::default()
        };

        let sheets = MockSheetsClient::new();
        SsotInjector::dispatch_master_solutions_row(&sheets, "SHEET_ID", 2, &row)
            .await
            .unwrap();

        let calls = sheets.calls.lock().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "SHEET_ID");
        assert_eq!(calls[0].1, "MASTER_SOLUTIONS");
        assert!(calls[0].2.get("D2:D2").is_some());
    }
}
