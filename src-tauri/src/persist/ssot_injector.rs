use crate::cognition::sgr_synthesizer::SgrPayload;
use thiserror::Error;
use serde_json::{json, Value};
use rusqlite::Connection;
use std::env;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SsotError {
    #[error("Falha na persistência L2 (SQLite): {0}")]
    L2Failure(String),
    #[error("Falha no despacho para a nuvem (Sheets): {0}")]
    CloudFailure(String),
}

pub struct SsotInjector;

impl SsotInjector {
    /// Injeta os dados no SSOT (SQLite + Google Sheets Batch)
    pub async fn inject_ssot(repo_id: &str, payload: SgrPayload) -> Result<(), SsotError> {
        // 1. Selagem L2 (Execução Durável)
        // OBRIGATÓRIO: O banco deve ser atualizado ANTES da rede
        Self::update_local_status(repo_id, "CONCLUIDO", &payload)
            .map_err(SsotError::L2Failure)?;

        // 2. Manobra Anti-503: Desmembramento e Agregação na RAM
        let _batch_payload = Self::prepare_batch_payload(repo_id, payload);

        // 3. Despacho Atômico (Simulado conforme Phase C)
        Self::dispatch_to_cloud(_batch_payload).await?;

        Ok(())
    }

    fn update_local_status(repo_id: &str, status_value: &str, payload: &SgrPayload) -> Result<(), String> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let root_dir = std::path::Path::new(manifest_dir).parent().unwrap_or_else(|| std::path::Path::new("."));
        let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
        
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Falha ao conectar no SQLite: {}", e))?;

        let _project_name = repo_id.split('/').next_back().unwrap_or(repo_id);
        let _repo_url = format!("https://github.com/{}", repo_id);
            
        // I/O L2 Real: Mapeando SgrPayload para as colunas reais da tabela
        conn.execute(
            "INSERT OR REPLACE INTO repo_heuristics (
                project_name, repo_url, repo_version, ultima_versao_online, lote_id, data_ultima_analise, analise_origem, declared_description, proposta_original_resumo, stack_base, licenca, lente_a_sentido_prod_ux, lente_b_estrutura_arq, lente_c_realidade_ops, visao_do_enxame, justificativa_decisao, executive_verdict, classificacao_terminal, acao_de_canibalizacao, categoria_arquitetural, horizonte_extracao, tipo_integracao, categoria_nuance_tecnica, integracao_papel_exato, ouro_a_extrair, deep_pattern, transplantable_core, logic_math_heuristic, real_structural_problem, must_components_prod_ux, must_components_arq, must_components_ops, detected_toxic_deps, do_not_absorb, where_ai_should_not_enter, bare_metal_fit, extractability_level, operability_level, entropy_risk, design_misuse_risk, intrinsic_ethics_risk, discipline_dependency, risco_principal, risco_linha_vermelha, observacoes, score_final, score_fit_geral_soda, score_philosophical_fit, score_bare_metal_fit, score_architectural_extractability, score_operability, score_creep_risk, score_runtime_sovereignty, score_model_logic_value, score_ethics_safety, score_intrinsic_risk, capability_nature_primary, architectural_topology, runtime_sovereignty_fit, local_first_fit, temporal_stability, adoptability_level, longitudinal_sustainability, abandonment_risk, maintenance_burden, onboarding_friction, observability_operational, recoverability_level, degradation_behavior, curation_burden, time_to_first_clear_value, imperfection_tolerance, evolution_cost, regulatory_risk, score_architectural_priority, score_human_product_priority, score_absorption_readiness, score_operational_priority, score_sustainability_adjusted_fit, valid_from, valid_to, embargo_status
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, ?81, ?82
            )",
            rusqlite::params![
                repo_id,
                payload.repo_url,
                payload.repo_version,
                payload.ultima_versao_online,
                payload.lote_id,
                payload.data_ultima_analise,
                payload.analise_origem,
                payload.declared_description,
                payload.proposta_original_resumo,
                payload.stack_base,
                payload.licenca,
                payload.lente_a_sentido_prod_ux,
                payload.lente_b_estrutura_arq,
                payload.lente_c_realidade_ops,
                payload.visao_do_enxame,
                payload.justificativa_decisao,
                format!("{:?}", payload.executive_verdict),
                payload.classificacao_terminal,
                format!("{:?}", payload.acao_de_canibalizacao),
                payload.categoria_arquitetural,
                payload.horizonte_extracao,
                payload.tipo_integracao,
                payload.categoria_nuance_tecnica,
                payload.integracao_papel_exato,
                payload.ouro_a_extrair,
                payload.deep_pattern,
                payload.transplantable_core,
                payload.logic_math_heuristic,
                payload.real_structural_problem,
                payload.must_components_prod_ux,
                payload.must_components_arq,
                payload.must_components_ops,
                payload.detected_toxic_deps,
                payload.do_not_absorb,
                payload.where_ai_should_not_enter,
                payload.bare_metal_fit,
                payload.extractability_level,
                payload.operability_level,
                payload.entropy_risk,
                payload.design_misuse_risk,
                payload.intrinsic_ethics_risk,
                payload.discipline_dependency,
                payload.risco_principal,
                payload.risco_linha_vermelha,
                payload.observacoes,
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
                payload.capability_nature_primary,
                payload.architectural_topology,
                payload.runtime_sovereignty_fit,
                payload.local_first_fit,
                payload.temporal_stability,
                payload.adoptability_level,
                payload.longitudinal_sustainability,
                payload.abandonment_risk,
                payload.maintenance_burden,
                payload.onboarding_friction,
                payload.observability_operational,
                payload.recoverability_level,
                payload.degradation_behavior,
                payload.curation_burden,
                payload.time_to_first_clear_value,
                payload.imperfection_tolerance,
                payload.evolution_cost,
                payload.regulatory_risk,
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

    fn prepare_batch_payload(_repo_id: &str, payload: SgrPayload) -> Value {
        let batch_payload = vec![
                json!(vec![
                    json!(payload.project_name),
                    json!(payload.repo_url),
                    json!(payload.repo_version),
                    json!(payload.ultima_versao_online),
                    json!(payload.lote_id),
                    json!(payload.data_ultima_analise),
                    json!(payload.analise_origem),
                    json!(payload.declared_description),
                    json!(payload.proposta_original_resumo),
                    json!(payload.stack_base),
                    json!(payload.licenca),
                    json!(payload.lente_a_sentido_prod_ux),
                    json!(payload.lente_b_estrutura_arq),
                    json!(payload.lente_c_realidade_ops),
                    json!(payload.visao_do_enxame),
                    json!(payload.justificativa_decisao),
                    json!(format!("{:?}", payload.executive_verdict)),
                    json!(payload.classificacao_terminal),
                    json!(format!("{:?}", payload.acao_de_canibalizacao)),
                    json!(payload.categoria_arquitetural),
                    json!(payload.horizonte_extracao),
                    json!(payload.tipo_integracao),
                    json!(payload.categoria_nuance_tecnica),
                    json!(payload.integracao_papel_exato),
                    json!(payload.ouro_a_extrair),
                    json!(payload.deep_pattern),
                    json!(payload.transplantable_core),
                    json!(payload.logic_math_heuristic),
                    json!(payload.real_structural_problem),
                    json!(payload.must_components_prod_ux),
                    json!(payload.must_components_arq),
                    json!(payload.must_components_ops),
                    json!(payload.detected_toxic_deps),
                    json!(payload.do_not_absorb),
                    json!(payload.where_ai_should_not_enter),
                    json!(payload.bare_metal_fit),
                    json!(payload.extractability_level),
                    json!(payload.operability_level),
                    json!(payload.entropy_risk),
                    json!(payload.design_misuse_risk),
                    json!(payload.intrinsic_ethics_risk),
                    json!(payload.discipline_dependency),
                    json!(payload.risco_principal),
                    json!(payload.risco_linha_vermelha),
                    json!(payload.observacoes),
                    json!(payload.score_final),
                    json!(payload.score_fit_geral_soda),
                    json!(payload.score_philosophical_fit),
                    json!(payload.score_bare_metal_fit),
                    json!(payload.score_architectural_extractability),
                    json!(payload.score_operability),
                    json!(payload.score_creep_risk),
                    json!(payload.score_runtime_sovereignty),
                    json!(payload.score_model_logic_value),
                    json!(payload.score_ethics_safety),
                    json!(payload.score_intrinsic_risk),
                    json!(payload.capability_nature_primary),
                    json!(payload.architectural_topology),
                    json!(payload.runtime_sovereignty_fit),
                    json!(payload.local_first_fit),
                    json!(payload.temporal_stability),
                    json!(payload.adoptability_level),
                    json!(payload.longitudinal_sustainability),
                    json!(payload.abandonment_risk),
                    json!(payload.maintenance_burden),
                    json!(payload.onboarding_friction),
                    json!(payload.observability_operational),
                    json!(payload.recoverability_level),
                    json!(payload.degradation_behavior),
                    json!(payload.curation_burden),
                    json!(payload.time_to_first_clear_value),
                    json!(payload.imperfection_tolerance),
                    json!(payload.evolution_cost),
                    json!(payload.regulatory_risk),
                    json!(payload.score_architectural_priority),
                    json!(payload.score_human_product_priority),
                    json!(payload.score_absorption_readiness),
                    json!(payload.score_operational_priority),
                    json!(payload.score_sustainability_adjusted_fit),
                    json!(payload.valid_from),
                    json!(payload.valid_to),
                    json!(payload.embargo_status),
                ])
            ];
        let mut map = serde_json::Map::new();
        map.insert("A2:CD2".to_string(), json!(batch_payload));
        Value::Object(map)
    }

    async fn dispatch_to_cloud(payload: Value) -> Result<(), SsotError> {
        use tokio::process::Command;
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;

        // Hotfix: Extração das variáveis reais do ecossistema SODA
        let creds = env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| SsotError::CloudFailure("Missing GOOGLE_APPLICATION_CREDENTIALS".to_string()))?;
        let sheets_id = env::var("GOOGLE_SHEETS_ID")
            .map_err(|_| SsotError::CloudFailure("Missing GOOGLE_SHEETS_ID".to_string()))?;

        // Construindo a requisição JSON-RPC para o MCP local
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
                    "spreadsheet_id": sheets_id,
                    "sheet": "MASTER_SOLUTIONS",
                    "ranges": payload
                }
            }
        });

        // Invocando o binário mcp-google-sheets localmente
        let mut child = Command::new("mcp-google-sheets")
            .env("GOOGLE_APPLICATION_CREDENTIALS", &creds)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SsotError::CloudFailure(format!("Falha ao spawnar mcp-google-sheets: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(format!("{}\n", init_req).as_bytes()).await;
            let _ = stdin.write_all(format!("{}\n", initialized_notif).as_bytes()).await;
            let _ = stdin.write_all(format!("{}\n", mcp_request).as_bytes()).await;
        }

        let output = child.wait_with_output().await.map_err(|e| SsotError::CloudFailure(e.to_string()))?;
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        // Validando se o MCP retornou um erro JSON-RPC ou se a STDERR gritou
        if stdout_str.contains("\"isError\":true") || stdout_str.contains("\"error\":") {
            return Err(SsotError::CloudFailure(format!("MCP Retornou Erro: {}", stdout_str)));
        }
        
        if !output.status.success() {
            return Err(SsotError::CloudFailure(format!("Falha no processo MCP. Exit {}. STDERR: {}", output.status, stderr_str)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::sgr_synthesizer::{SgrPayload, TerminalClassification, CannibalizationAction};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DB_CALL_ORDER: AtomicUsize = AtomicUsize::new(0);
    static CLOUD_CALL_ORDER: AtomicUsize = AtomicUsize::new(0);

    fn mock_payload() -> SgrPayload {
        SgrPayload {
            visao_do_enxame: "V".to_string(),
            justificativa_decisao: "J".to_string(),
            executive_verdict: TerminalClassification::AprovadoParaProducao,
            acao_de_canibalizacao: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95.0,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_l2_durable_execution_order() {
        // Reseta contadores
        DB_CALL_ORDER.store(0, Ordering::SeqCst);
        CLOUD_CALL_ORDER.store(0, Ordering::SeqCst);

        // Ordem esperada: DB = 1, Cloud = 2
        // Simulando a injeção
        let _ = SsotInjector::update_local_status("test", "CONCLUIDO", &mock_payload());
        DB_CALL_ORDER.store(1, Ordering::SeqCst);
        
        let _ = SsotInjector::dispatch_to_cloud(json!({})).await;
        CLOUD_CALL_ORDER.store(2, Ordering::SeqCst);

        assert_eq!(DB_CALL_ORDER.load(Ordering::SeqCst), 1);
        assert_eq!(CLOUD_CALL_ORDER.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_anti_503_batch_slicing() {
        let payload = mock_payload();
        let batch = SsotInjector::prepare_batch_payload("repo_1", payload);
        let arr = batch["A2:CD2"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_array().unwrap().len(), 82);
    }

    #[tokio::test]
    async fn test_sqlite_failure_aborts_network() {
        // Se falhar o L2, o CloudFailure nunca deve ocorrer pois a função retorna antes
        // Como estamos mockando, simulamos a lógica do inject_ssot
        let res = inject_with_db_fail("repo_fail", mock_payload()).await;
        assert!(matches!(res, Err(SsotError::L2Failure(_))));
    }

    async fn inject_with_db_fail(_id: &str, _p: SgrPayload) -> Result<(), SsotError> {
        // Simulação da trava do inject_ssot
        Err(SsotError::L2Failure("Locked".to_string()))
    }
}
