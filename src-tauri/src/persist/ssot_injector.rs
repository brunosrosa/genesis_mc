use crate::cognition::phase3_4::{apply_phase4_block5, build_batch_update_payload, MasterSolutionsRow};
use thiserror::Error;
use serde_json::{json, Value};
use rusqlite::Connection;
use std::env;
use std::future::Future;
use std::pin::Pin;
use url::Url;
use tracing::info;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SsotError {
    #[error("Falha na validacao do payload SSOT: {0}")]
    ValidationFailure(String),
    #[error("Falha na persistência L2 (SQLite): {0}")]
    L2Failure(String),
    #[error("Falha no despacho para a nuvem (Sheets): {0}")]
    CloudFailure(String),
}

pub struct SsotInjector;

const SSOT_STATUS_CONCLUIDO: &str = "CONCLUIDO";
const SSOT_EXPECTED_COLUMNS: usize = 82;
const MASTER_SOLUTIONS_SHEET: &str = "MASTER_SOLUTIONS";

pub type SheetsFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

pub trait SheetsClient: Send + Sync {
    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a>;
}

pub struct McpGoogleSheetsClient;

impl SheetsClient for McpGoogleSheetsClient {
    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a> {
        Box::pin(async move {
            use std::process::Stdio;
            use tokio::io::AsyncWriteExt;
            use tokio::process::Command;

            let creds = env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .map_err(|_| "Missing GOOGLE_APPLICATION_CREDENTIALS".to_string())?;

            let init_req = json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "soda-injector", "version": "1.0.0" }
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
                    "name": "batch_update_cells",
                    "arguments": {
                        "spreadsheet_id": spreadsheet_id,
                        "sheet": sheet,
                        "ranges": ranges
                    }
                }
            });

            let mut child = Command::new("mcp-google-sheets")
                .env("GOOGLE_APPLICATION_CREDENTIALS", &creds)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Falha ao spawnar mcp-google-sheets: {}", e))?;

            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(format!("{}\n", init_req).as_bytes()).await;
                let _ = stdin
                    .write_all(format!("{}\n", initialized_notif).as_bytes())
                    .await;
                let _ = stdin.write_all(format!("{}\n", mcp_request).as_bytes()).await;
            }

            let output = child
                .wait_with_output()
                .await
                .map_err(|e| format!("Falha ao aguardar processo MCP: {}", e))?;
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let stderr_str = String::from_utf8_lossy(&output.stderr);

            if stdout_str.contains("\"isError\":true") || stdout_str.contains("\"error\":") {
                return Err(format!("MCP Retornou Erro: {}", stdout_str));
            }

            if !output.status.success() {
                return Err(format!(
                    "Falha no processo MCP. Exit {}. STDERR: {}",
                    output.status, stderr_str
                ));
            }

            Ok(())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedSsotFields {
    project_name: String,
    repo_url: String,
    repo_version: String,
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
    /// Injeta os dados no SSOT (SQLite + Google Sheets Batch)
    pub async fn inject_ssot(
        repo_id: &str,
        mut row: MasterSolutionsRow,
        row_number_1based: u32,
        now_epoch: i64,
    ) -> Result<(), SsotError> {
        let validated = Self::validate_payload(repo_id, &row)?;

        // 1. Selagem L2 (Execução Durável)
        // OBRIGATÓRIO: O banco deve ser atualizado ANTES da rede
        apply_phase4_block5(now_epoch, &mut row);
        Self::update_local_status(repo_id, SSOT_STATUS_CONCLUIDO, &row, &validated)
            .map_err(SsotError::L2Failure)?;

        // 2. Manobra Anti-503: Desmembramento e Agregação na RAM
        let batch_payload = Self::prepare_batch_payload(row_number_1based, &row)?;

        // 3. Despacho Atômico (Simulado conforme Phase C)
        Self::dispatch_to_cloud(batch_payload).await?;

        Ok(())
    }

    fn validate_payload(
        repo_id: &str,
        payload: &MasterSolutionsRow,
    ) -> Result<ValidatedSsotFields, SsotError> {
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

        let repo_version = Self::require_non_empty("repo_version", &payload.repo_version)?;
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
            repo_version,
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
    ) -> Result<(), String> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let root_dir = std::path::Path::new(manifest_dir).parent().unwrap_or_else(|| std::path::Path::new("."));
        let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
        
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Falha ao conectar no SQLite: {}", e))?;

        Self::ensure_repo_heuristics_schema(&conn)?;

        // I/O L2 Real: Mapeando SgrPayload para as colunas reais da tabela
        conn.execute(
            "INSERT OR REPLACE INTO repo_heuristics (
                project_name, repo_url, repo_version, ultima_versao_online, lote_id, data_ultima_analise, analise_origem, declared_description, proposta_original_resumo, stack_base, licenca, lente_a_sentido_prod_ux, lente_b_estrutura_arq, lente_c_realidade_ops, visao_do_enxame, justificativa_decisao, executive_verdict, classificacao_terminal, acao_de_canibalizacao, categoria_arquitetural, horizonte_extracao, tipo_integracao, categoria_nuance_tecnica, integracao_papel_exato, ouro_a_extrair, deep_pattern, transplantable_core, logic_math_heuristic, real_structural_problem, must_components_prod_ux, must_components_arq, must_components_ops, detected_toxic_deps, do_not_absorb, where_ai_should_not_enter, bare_metal_fit, extractability_level, operability_level, entropy_risk, design_misuse_risk, intrinsic_ethics_risk, discipline_dependency, risco_principal, risco_linha_vermelha, observacoes, score_final, score_fit_geral_soda, score_philosophical_fit, score_bare_metal_fit, score_architectural_extractability, score_operability, score_creep_risk, score_runtime_sovereignty, score_model_logic_value, score_ethics_safety, score_intrinsic_risk, capability_nature_primary, architectural_topology, runtime_sovereignty_fit, local_first_fit, temporal_stability, adoptability_level, longitudinal_sustainability, abandonment_risk, maintenance_burden, onboarding_friction, observability_operational, recoverability_level, degradation_behavior, curation_burden, time_to_first_clear_value, imperfection_tolerance, evolution_cost, regulatory_risk, score_architectural_priority, score_human_product_priority, score_absorption_readiness, score_operational_priority, score_sustainability_adjusted_fit, valid_from, valid_to, embargo_status
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, ?81, ?82
            )",
            rusqlite::params![
                &validated.project_name,
                &validated.repo_url,
                &validated.repo_version,
                &payload.ultima_versao_online,
                &payload.lote_id,
                payload.data_ultima_analise,
                &payload.analise_origem,
                &payload.declared_description,
                &payload.proposta_original_resumo,
                &payload.stack_base,
                &payload.licenca,
                &payload.lente_a_sentido_prod_ux,
                &payload.lente_b_estrutura_arq,
                &payload.lente_c_realidade_ops,
                &payload.visao_do_enxame,
                &payload.justificativa_decisao,
                &payload.executive_verdict,
                &payload.classificacao_terminal,
                &payload.acao_de_canibalizacao,
                &payload.categoria_arquitetural,
                &payload.horizonte_extracao,
                &payload.tipo_integracao,
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
                &payload.bare_metal_fit,
                &payload.extractability_level,
                &payload.operability_level,
                &payload.entropy_risk,
                &payload.design_misuse_risk,
                &payload.intrinsic_ethics_risk,
                &payload.discipline_dependency,
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
                &payload.capability_nature_primary,
                &payload.architectural_topology,
                &payload.runtime_sovereignty_fit,
                &payload.local_first_fit,
                &payload.temporal_stability,
                &payload.adoptability_level,
                &payload.longitudinal_sustainability,
                &payload.abandonment_risk,
                &payload.maintenance_burden,
                &payload.onboarding_friction,
                &payload.observability_operational,
                &payload.recoverability_level,
                &payload.degradation_behavior,
                &payload.curation_burden,
                &payload.time_to_first_clear_value,
                &payload.imperfection_tolerance,
                &payload.evolution_cost,
                &payload.regulatory_risk,
                payload.score_architectural_priority,
                payload.score_human_product_priority,
                payload.score_absorption_readiness,
                payload.score_operational_priority,
                payload.score_sustainability_adjusted_fit,
                payload.valid_from,
                payload.valid_to,
                payload.embargo_status,
            ],
        ).map_err(|e| format!("Falha ao executar INSERT repo_heuristics: {}", e))?;

        // Atualizando o status em repositorios
        let _ = conn.execute(
            "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
            rusqlite::params![status_value, repo_id],
        ).map_err(|e| format!("Falha ao executar UPDATE repositorios: {}", e))?;
        
        Ok(())
    }

    fn ensure_repo_heuristics_schema(conn: &Connection) -> Result<(), String> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repo_heuristics (
                project_name TEXT PRIMARY KEY,
                repo_url TEXT NOT NULL,
                repo_version TEXT NOT NULL,
                ultima_versao_online TEXT,
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
        Ok(())
    }

    fn prepare_batch_payload(
        row_number_1based: u32,
        payload: &MasterSolutionsRow,
    ) -> Result<Value, SsotError> {
        let ranges = build_batch_update_payload(row_number_1based, payload);
        let mut map = serde_json::Map::new();

        for (range, rows) in ranges {
            if rows.len() != 1 || rows[0].len() != SSOT_EXPECTED_COLUMNS {
                return Err(SsotError::ValidationFailure(format!(
                    "Payload do Google Sheets desalinhado: esperado 1x{}, recebeu {}x{}",
                    SSOT_EXPECTED_COLUMNS,
                    rows.len(),
                    rows.get(0).map(|r| r.len()).unwrap_or(0)
                )));
            }
            let last = rows[0].len() - 1;
            let idx_valid_from = last - 2;
            let idx_valid_to = last - 1;
            let idx_embargo = last;
            if rows[0][idx_valid_from].as_i64().is_none() {
                return Err(SsotError::ValidationFailure(
                    "valid_from invalido no payload do Sheets".to_string(),
                ));
            }
            if rows[0][idx_embargo].as_i64().is_none() {
                return Err(SsotError::ValidationFailure(
                    "embargo_status invalido no payload do Sheets".to_string(),
                ));
            }
            let valid_to_ok = rows[0][idx_valid_to].as_i64().is_some()
                || rows[0][idx_valid_to]
                    .as_str()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(false);
            if !valid_to_ok {
                return Err(SsotError::ValidationFailure(
                    "valid_to invalido no payload do Sheets".to_string(),
                ));
            }
            map.insert(range, json!(rows));
        }

        info!(columns = SSOT_EXPECTED_COLUMNS, "Payload do Google Sheets montado para batch_update_cells");
        Ok(Value::Object(map))
    }

    pub async fn dispatch_master_solutions_row(
        sheets: &dyn SheetsClient,
        spreadsheet_id: &str,
        row_number_1based: u32,
        row: &MasterSolutionsRow,
    ) -> Result<(), SsotError> {
        let payload = Self::prepare_batch_payload(row_number_1based, row)?;
        sheets
            .batch_update_cells(spreadsheet_id, MASTER_SOLUTIONS_SHEET, payload)
            .await
            .map_err(SsotError::CloudFailure)
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
    use std::sync::Arc;
    use tokio::sync::Mutex;
    #[test]
    fn test_prepare_batch_payload_is_82_columns_a_to_cd() {
        let mut row = MasterSolutionsRow::default();
        row.project_name = "owner/repo".to_string();
        row.repo_url = "https://github.com/owner/repo".to_string();
        row.repo_version = "v1.0.0".to_string();
        row.ultima_versao_online = "v1.0.1".to_string();
        row.lote_id = "LOTE_01".to_string();
        row.data_ultima_analise = 1_715_000_000;
        row.analise_origem = "SGR".to_string();
        row.declared_description = "Descricao".to_string();
        row.proposta_original_resumo = "Resumo".to_string();
        row.stack_base = "Rust".to_string();
        row.licenca = "MIT".to_string();
        row.valid_from = 1_700_000_000;
        row.valid_to = None;
        row.embargo_status = 0;

        let validated = SsotInjector::validate_payload("owner/repo", &row).unwrap();
        let batch = SsotInjector::prepare_batch_payload(2, &row).unwrap();
        let range = "A2:CD2";
        let arr = batch[range].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        let row_arr = arr[0].as_array().unwrap();
        assert_eq!(row_arr.len(), SSOT_EXPECTED_COLUMNS);
        assert_eq!(arr[0][0], json!("owner/repo"));
        assert_eq!(arr[0][1], json!("https://github.com/owner/repo"));
        assert_eq!(row_arr[SSOT_EXPECTED_COLUMNS - 3], json!(1_700_000_000));
        assert_eq!(row_arr[SSOT_EXPECTED_COLUMNS - 2], json!(""));
        assert_eq!(row_arr[SSOT_EXPECTED_COLUMNS - 1], json!(0));
        assert_eq!(validated.project_name, "owner/repo");
    }

    #[test]
    fn test_payload_validation_rejects_missing_required_fields() {
        let row = MasterSolutionsRow::default();
        let result = SsotInjector::validate_payload("owner/repo", &row);
        assert!(matches!(result, Err(SsotError::ValidationFailure(_))));
    }

    struct MockSheetsClient {
        calls: Arc<Mutex<Vec<(String, String, Value)>>>,
    }

    impl MockSheetsClient {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl SheetsClient for MockSheetsClient {
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
        let mut row = MasterSolutionsRow::default();
        row.project_name = "owner/repo".to_string();
        row.repo_url = "https://github.com/owner/repo".to_string();
        row.repo_version = "v1.0.0".to_string();
        row.ultima_versao_online = "v1.0.1".to_string();
        row.lote_id = "LOTE_01".to_string();
        row.data_ultima_analise = 1_715_000_000;
        row.analise_origem = "SGR".to_string();
        row.declared_description = "Descricao".to_string();
        row.proposta_original_resumo = "Resumo".to_string();
        row.stack_base = "Rust".to_string();
        row.licenca = "MIT".to_string();

        let sheets = MockSheetsClient::new();
        SsotInjector::dispatch_master_solutions_row(&sheets, "SHEET_ID", 2, &row)
            .await
            .unwrap();

        let calls = sheets.calls.lock().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "SHEET_ID");
        assert_eq!(calls[0].1, "MASTER_SOLUTIONS");
        assert!(calls[0].2.get("A2:CD2").is_some());
    }
}
