use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tracing::{info, warn};

use souls_mc_lib::persist::ssot_injector::ReqwestGoogleWorkspaceSheetsClient;
use souls_mc_lib::telemetry::{enable_virtual_terminal, init_cli_tracing, parse_log_level_from_env};

const MASTER_SOLUTIONS_SHEET: &str = "MASTER_SOLUTIONS";

#[cfg(not(test))]
const POST_BATCH_WRITE_DELAY: Duration = Duration::from_millis(1_500);
#[cfg(test)]
const POST_BATCH_WRITE_DELAY: Duration = Duration::from_millis(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsyncedRepoRow {
    pub project_name: String,
    pub repo_url: String,
    pub lote_id: String,
    pub status_processamento: String,
    pub repo_version: String,
    pub ultima_versao_online: String,
    pub proposta_original_resumo: String,
    pub categoria_arquitetural: String,
}

pub type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
pub type SheetsFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

pub trait OutboxSheetsClient: Send + Sync {
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

impl OutboxSheetsClient for ReqwestGoogleWorkspaceSheetsClient {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a> {
        souls_mc_lib::persist::ssot_injector::SheetsClient::get_sheet_data(
            self,
            spreadsheet_id,
            sheet,
            range,
        )
    }

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a> {
        souls_mc_lib::persist::ssot_injector::SheetsClient::batch_update_cells(
            self,
            spreadsheet_id,
            sheet,
            ranges,
        )
    }
}

pub struct OutboxSynchronizer<S: OutboxSheetsClient> {
    sheets: Arc<S>,
    db_path: PathBuf,
    spreadsheet_id: String,
}

pub const CANONICAL_HEADERS: &[&str] = &[
    "status_atualizacao",
    "status_fase",
    "categoria_arquitetural",
    "project_name",
    "repo_url",
    "licenca",
    "proposta_original_resumo",
    "ultima_versao_online",
    "declared_description",
    "stack_base",
    "indicacao_otimista_canibalizacao",
    "repo_analised_version",
    "lente_a_sentido_prod_ux",
    "lente_b_estrutura_arq",
    "lente_c_realidade_ops",
    "visao_do_enxame",
    "justificativa_decisao",
    "executive_verdict",
    "ouro_a_extrair",
    "deep_pattern",
    "transplantable_core",
    "logic_math_heuristic",
    "real_structural_problem",
    "classificacao_terminal",
    "integracao_papel_exato",
    "categoria_nuance_tecnica",
    "capability_nature_primary",
    "must_components_prod_ux",
    "must_components_arq",
    "must_components_ops",
    "detected_toxic_deps",
    "do_not_absorb",
    "where_ai_should_not_enter",
    "acao_de_canibalizacao",
    "tipo_integracao",
    "horizonte_extracao",
    "time_to_first_clear_value",
    "extractability_level",
    "temporal_stability",
    "architectural_topology",
    "adoptability_level",
    "bare_metal_fit",
    "maintenance_burden",
    "runtime_sovereignty_fit",
    "longitudinal_sustainability",
    "local_first_fit",
    "onboarding_friction",
    "observability_operational",
    "recoverability_level",
    "degradation_behavior",
    "curation_burden",
    "evolution_cost",
    "operability_level",
    "imperfection_tolerance",
    "discipline_dependency",
    "risco_principal",
    "risco_linha_vermelha",
    "observacoes",
    "abandonment_risk",
    "design_misuse_risk",
    "intrinsic_ethics_risk",
    "entropy_risk",
    "regulatory_risk",
    "score_final",
    "score_philosophical_fit",
    "score_fit_geral_soda",
    "score_architectural_priority",
    "score_architectural_extractability",
    "score_human_product_priority",
    "score_absorption_readiness",
    "score_operational_priority",
    "score_bare_metal_fit",
    "score_operability",
    "score_runtime_sovereignty",
    "score_model_logic_value",
    "score_ethics_safety",
    "score_creep_risk",
    "score_intrinsic_risk",
    "score_sustainability_adjusted_fit",
    "valid_from",
    "valid_to",
    "analise_origem",
    "data_ultima_analise",
    "lote_id",
    "embargo_status",
];

pub const ALLOWED_CATEGORIA_ARQUITETURAL: &[&str] = &[
    "AI_Research - Foundation_Model",
    "CanvasUI - Core_Pattern",
    "CanvasUI - Domain_App",
    "CanvasUI - Ops_Dashboard",
    "CanvasUI - Terminal_Workspace",
    "Comms_Social - Platform_Client",
    "Domain_App - Self_Hosted",
    "Infraestrutura_Core - Concurrency_OS",
    "Infraestrutura_Core - Data_Pipeline",
    "Infraestrutura_Core - Data_Serialization",
    "Infraestrutura_Core - Hardware_Ops",
    "Knowledge_Extraction - Doc_Parsing",
    "Knowledge_Extraction - Generic",
    "Knowledge_Extraction - Multimedia_Parsing",
    "Knowledge_Extraction - Semantic_Mining",
    "Knowledge_Extraction - Web_Scraping",
    "Memoria_RAG - Graph_Store",
    "Memoria_RAG - Relational_Episodic",
    "Memoria_RAG - Vector_Store",
    "Model_Serving - Edge_Deployment",
    "Model_Serving - Inference_Engine",
    "Model_Serving - Resource_Scheduler",
    "Model_Serving - Training_FineTuning",
    "Orquestracao_Agentes - Dev_Framework",
    "Orquestracao_Agentes - OS_Runtime",
    "Orquestracao_Agentes - Simulation_Environment",
    "Orquestracao_Agentes - Skill_Library",
    "Orquestracao_Agentes - Specialized_Worker",
    "Orquestracao_Agentes - Workflow_DAG",
    "Roteamento_FinOps - API_Gateway",
    "Roteamento_FinOps - Cost_Analytics",
    "Roteamento_FinOps - Network_Tunnel",
    "Roteamento_FinOps - Prompt_Caching",
    "Seguranca_Sandbox - Auth_Crypto",
    "Seguranca_Sandbox - MicroVM_Container",
    "Seguranca_Sandbox - Privacy_Governance",
    "Seguranca_Sandbox - Runtime_Isolation",
    "Tooling_Dev - CLI_Utilities",
    "Tooling_Dev - Knowledge_Curation",
    "Tooling_Dev - MCP_Bridging",
    "Tooling_Dev - Observability_Eval",
    "Tooling_Dev - Prompt_Knowledge",
    "UILibrary - Animation_Graphics",
    "UILibrary - Component_System",
    "UILibrary - Generative_UI",
    "UILibrary - Terminal_TUI",
    "Outros - Uncategorized",
];

fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_lowercase()
}

#[allow(dead_code)]
fn is_column_a_status(status: &str) -> bool {
    status == "PENDENTE"
        || status.contains("NOVO_LINK")
        || status.contains("TRIAGEM")
        || status.contains("APROVADO_")
        || status.contains("CONCLUIDO_")
        || status.contains("REJEITADO_")
        || status == "DESATUALIZADA"
}

fn is_column_b_status(status: &str) -> bool {
    status.starts_with("FASE_")
        || status == "SHORT-CIRCUIT"
        || status.contains("HARVESTER")
        || status.contains("DESTILADOR")
        || status.contains("ENXAME")
        || status.contains("SINTETIZADOR")
        || status.contains("GUARDIAO")
        || status.contains("BATEDOR")
        || status.contains("DEEP")
}

impl<S: OutboxSheetsClient> OutboxSynchronizer<S> {
    pub fn new(sheets: Arc<S>, db_path: PathBuf, spreadsheet_id: String) -> Self {
        Self {
            sheets,
            db_path,
            spreadsheet_id,
        }
    }

    fn col_idx_to_a1(col_idx0: usize) -> String {
        let mut n = col_idx0 + 1;
        let mut out = String::new();
        while n > 0 {
            let rem = (n - 1) % 26;
            out.insert(0, (b'A' + rem as u8) as char);
            n = (n - 1) / 26;
        }
        out
    }

    pub async fn sync_once(&self) -> Result<usize, String> {
        let db_path = self.db_path.clone();

        // 1. LEITURA DE LINHAS DESSINCRONIZADAS DO SQLITE (ORDER BY ROWID DESC)
        let unsynced_rows: Vec<UnsyncedRepoRow> = tokio::task::spawn_blocking(move || -> Result<Vec<UnsyncedRepoRow>, String> {
            let conn = Connection::open(&db_path)
                .map_err(|e| format!("Outbox: falha ao abrir SQLite: {}", e))?;

            // Garantia de Migração Idempotente de Colunas
            let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN sheets_synced INTEGER DEFAULT 0", []);
            let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN proposta_original_resumo TEXT", []);
            let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN categoria_arquitetural TEXT", []);

            let mut stmt = conn
                .prepare(
                    "SELECT project_name, repo_url, lote_id, status_processamento,
                            COALESCE(repo_version, ''), COALESCE(ultima_versao_online, ''),
                            COALESCE(proposta_original_resumo, ''), COALESCE(categoria_arquitetural, '')
                     FROM repositorios
                     WHERE sheets_synced = 0 OR sheets_synced IS NULL
                     ORDER BY ROWID DESC",
                )
                .map_err(|e| format!("Outbox: erro ao consultar SQLite: {e}"))?;

            let rows = stmt
                .query_map([], |r| {
                    Ok(UnsyncedRepoRow {
                        project_name: r.get(0)?,
                        repo_url: r.get(1)?,
                        lote_id: r.get(2)?,
                        status_processamento: r.get(3)?,
                        repo_version: r.get(4)?,
                        ultima_versao_online: r.get(5)?,
                        proposta_original_resumo: r.get(6)?,
                        categoria_arquitetural: r.get(7)?,
                    })
                })
                .map_err(|e| format!("Outbox: erro na leitura do cursor: {e}"))?;

            let mut out = Vec::new();
            for item in rows {
                out.push(item.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| format!("Join error: {e}"))??;

        if unsynced_rows.is_empty() {
            info!("Outbox: Nenhuma linha pendente de sincronização no SQLite.");
            return Ok(0);
        }

        // 2. BULK READ O(1) DAS COLUNAS DE IDENTIFICAÇÃO NO GOOGLE SHEETS
        let sheet_data = self
            .sheets
            .get_sheet_data(&self.spreadsheet_id, MASTER_SOLUTIONS_SHEET, "A1:CG2000".to_string())
            .await?;

        if sheet_data.is_empty() {
            return Err("Outbox: Planilha Google Sheets vazia".to_string());
        }

        let header_row = &sheet_data[0];

        let repo_url_idx = header_row
            .iter()
            .position(|h| {
                let name = h.trim().to_lowercase();
                name == "repo_url" || name == "url" || name == "link"
            })
            .unwrap_or(4); // Default idx 4 (Col E: repo_url)

        let status_atualizacao_opt = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("status_atualizacao"));

        let status_fase_opt = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("status_fase") || h.trim().eq_ignore_ascii_case("status_processamento"));

        let categoria_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("categoria_arquitetural"))
            .unwrap_or(2); // Default idx 2 (Col C: categoria_arquitetural)

        let resumo_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("proposta_original_resumo"))
            .unwrap_or(6); // Default idx 6 (Col G: proposta_original_resumo)

        let version_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("ultima_versao_online") || h.trim().eq_ignore_ascii_case("repo_version"))
            .unwrap_or(7); // Default idx 7 (Col H: ultima_versao_online)

        // Indexação O(1) por repo_url normalizada
        let mut index_map: HashMap<String, u32> = HashMap::new();
        for (i, row) in sheet_data.iter().enumerate().skip(1) {
            let row_number = (i + 1) as u32;
            if let Some(url_raw) = row.get(repo_url_idx) {
                let norm = normalize_url(url_raw);
                if !norm.is_empty() {
                    index_map.insert(norm, row_number);
                }
            }
        }

        let total_cols = header_row.len();
        if total_cols == 0 {
            return Err("Outbox: Header do Sheets possui 0 colunas".to_string());
        }
        let max_col_idx = total_cols.saturating_sub(1);
        let end_col = Self::col_idx_to_a1(max_col_idx);

        let mut synced_urls: Vec<String> = Vec::new();

        // 3. ATUALIZAÇÃO CIRÚRGICA LINHA A LINHA APENAS DAS LINHAS ALVO (MUTAÇÃO ATÔMICA PRECISÃO CIRÚRGICA)
        for row in unsynced_rows {
            let norm_url = normalize_url(&row.repo_url);
            let Some(&row_number) = index_map.get(&norm_url) else {
                warn!(
                    repo_url = %row.repo_url,
                    "Outbox: repositório não encontrado na planilha do Sheets para sincronização"
                );
                continue;
            };

            // Copia a linha existente da planilha preservando células não alteradas
            let existing_row = sheet_data.get((row_number - 1) as usize);
            let mut row_values: Vec<String> = Vec::with_capacity(total_cols);

            if let Some(r) = existing_row {
                for c in r.iter().take(total_cols) {
                    row_values.push(c.clone());
                }
            }
            while row_values.len() < total_cols {
                row_values.push(String::new());
            }

            // Atualização Protegida do Status (Separação Canônica Coluna A vs Coluna B; NUNCA escreve vazio)
            if !row.status_processamento.trim().is_empty() {
                let status_val = row.status_processamento.trim();
                if is_column_b_status(status_val) {
                    let target = status_fase_opt.or(status_atualizacao_opt).unwrap_or(1);
                    if target < total_cols {
                        row_values[target] = status_val.to_string();
                    }
                } else {
                    let target = status_atualizacao_opt.or(status_fase_opt).unwrap_or(0);
                    if target < total_cols {
                        row_values[target] = status_val.to_string();
                    }
                }
            }

            // Atualização de Versão (apenas se não for vazio)
            if version_idx < total_cols {
                let ver = if !row.ultima_versao_online.is_empty() {
                    &row.ultima_versao_online
                } else {
                    &row.repo_version
                };
                if !ver.trim().is_empty() {
                    row_values[version_idx] = ver.trim().to_string();
                }
            }

            // Atualização de Resumo (apenas se não for vazio)
            if resumo_idx < total_cols && !row.proposta_original_resumo.trim().is_empty() {
                row_values[resumo_idx] = row.proposta_original_resumo.trim().to_string();
            }

            // Atualização de Categoria (apenas se não for vazio)
            if categoria_idx < total_cols && !row.categoria_arquitetural.trim().is_empty() {
                row_values[categoria_idx] = row.categoria_arquitetural.trim().to_string();
            }

            // ADR-006: Payload de 1 único range atômico por requisição (A{row}:{end_col}{row})
            let atomic_range = format!("A{row_number}:{end_col}{row_number}");
            let mut batch_map = serde_json::Map::new();
            batch_map.insert(atomic_range.clone(), json!([row_values]));

            info!(
                repo_url = %row.repo_url,
                row_number = row_number,
                range = %atomic_range,
                "Outbox: enviando atualização atômica da linha para o Sheets"
            );

            // 4. DISPARO HTTP COM RETENTATIVA ANTI-429
            let mut attempts = 0;
            loop {
                let res = self
                    .sheets
                    .batch_update_cells(
                        &self.spreadsheet_id,
                        MASTER_SOLUTIONS_SHEET,
                        Value::Object(batch_map.clone()),
                    )
                    .await;

                match res {
                    Ok(()) => break,
                    Err(err)
                        if err.contains("429")
                            || err.contains("RESOURCE_EXHAUSTED")
                            || err.contains("Too Many Requests")
                            || err.contains("Quota exceeded") =>
                    {
                        attempts += 1;
                        if attempts > 5 {
                            return Err(format!(
                                "Outbox: excedeu limite de retentativas HTTP 429 no Sheets: {err}"
                            ));
                        }
                        let backoff_secs = 2u64.pow(attempts) + (attempts as u64);
                        warn!(
                            attempt = attempts,
                            backoff_secs = backoff_secs,
                            "Outbox: HTTP 429 Quota Exceeded. Aplicando recuo exponencial..."
                        );
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    }
                    Err(err) => return Err(err),
                }
            }

            // 5. ATUALIZAÇÃO NO SQLITE PARA sheets_synced = 1
            let db_path = self.db_path.clone();
            let target_url = row.repo_url.clone();
            tokio::task::spawn_blocking(move || -> Result<(), String> {
                let conn = Connection::open(&db_path)
                    .map_err(|e| format!("Outbox: falha ao abrir SQLite para confirmação: {e}"))?;
                conn.execute(
                    "UPDATE repositorios SET sheets_synced = 1 WHERE repo_url = ?1",
                    params![target_url],
                )
                .map_err(|e| format!("Outbox: falha ao atualizar sheets_synced: {e}"))?;
                Ok(())
            })
            .await
            .map_err(|e| format!("Join error: {e}"))??;

            synced_urls.push(row.repo_url);

            // CADÊNCIA ANTI-429 SLEEP ENTRE LINHAS (150ms)
            tokio::time::sleep(POST_BATCH_WRITE_DELAY).await;
        }

        if synced_urls.is_empty() {
            info!("Outbox: Nenhuma alteração pendente mapeada para despacho no Sheets.");
            return Ok(0);
        }

        let synced_count = synced_urls.len();

        info!(
            synced_count,
            "Outbox: sincronização concluída com sucesso no Sheets."
        );

        Ok(synced_count)
    }
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    #[cfg(windows)]
    let _ = enable_ansi_support::enable_ansi_support();
    enable_virtual_terminal();
    let level = parse_log_level_from_env();
    init_cli_tracing(level);

    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let db_path = root_dir.join(".souls_data").join("souls_heuristic_vault.db");
    let sheets_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Falta a variável de ambiente GOOGLE_SHEETS_ID"))?;

    let sheets_client = Arc::new(ReqwestGoogleWorkspaceSheetsClient);
    let synchronizer = OutboxSynchronizer::new(sheets_client, db_path, sheets_id);

    let synced = synchronizer.sync_once().await.map_err(io::Error::other)?;
    info!(synced, "Outbox Synchronizer: execução concluída");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::sync::Mutex;

    struct MockSheetsClient {
        get_data_calls: Mutex<usize>,
        batch_update_calls: Mutex<usize>,
        last_batch_payload: Mutex<Option<Value>>,
    }

    impl OutboxSheetsClient for MockSheetsClient {
        fn get_sheet_data<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            _range: String,
        ) -> SheetsDataFuture<'a> {
            *self.get_data_calls.lock().unwrap() += 1;
            Box::pin(async move {
                // Header + row 2
                Ok(vec![
                    vec![
                        "project_name".to_string(),
                        "repo_url".to_string(),
                        "status_fase".to_string(),
                        "repo_version".to_string(),
                        "lote_id".to_string(),
                        "proposta_original_resumo".to_string(),
                        "categoria_arquitetural".to_string(),
                    ],
                    vec![
                        "acme/widget".to_string(),
                        "https://github.com/acme/widget".to_string(),
                        "PENDENTE".to_string(),
                        "v1.0.0".to_string(),
                        "L1".to_string(),
                        "".to_string(),
                        "".to_string(),
                    ],
                ])
            })
        }

        fn batch_update_cells<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            ranges: Value,
        ) -> SheetsFuture<'a> {
            *self.batch_update_calls.lock().unwrap() += 1;
            *self.last_batch_payload.lock().unwrap() = Some(ranges);
            Box::pin(async move { Ok(()) })
        }
    }

    fn setup_test_db(db_path: &Path) -> Connection {
        let conn = Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE repositorios (
                project_name TEXT PRIMARY KEY,
                lote_id TEXT NOT NULL,
                repo_url TEXT NOT NULL UNIQUE,
                repo_analised_version TEXT,
                repo_version TEXT,
                ultima_versao_online TEXT,
                soda_universal_uuid TEXT NOT NULL UNIQUE,
                status_processamento TEXT NOT NULL,
                timestamp_fase_1 INTEGER,
                timestamp_fase_3 INTEGER,
                retry_count INTEGER NOT NULL,
                proposta_original_resumo TEXT,
                categoria_arquitetural TEXT,
                sheets_synced INTEGER DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[tokio::test]
    async fn test_outbox_bulk_read_and_batch_update_sync() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count, proposta_original_resumo, categoria_arquitetural, sheets_synced
            ) VALUES (
                'acme/widget', 'L1', 'https://github.com/acme/widget', 'v1.0.0', 'v1.0.0', 'v1.0.0',
                'UUID-1', 'PENDENTE_HARVESTER', 0, 'Resumo técnico da ferramenta.', 'Tooling_Dev - CLI_Utilities', 0
            )",
            [],
        )
        .unwrap();
        drop(conn);

        let mock_sheets = Arc::new(MockSheetsClient {
            get_data_calls: Mutex::new(0),
            batch_update_calls: Mutex::new(0),
            last_batch_payload: Mutex::new(None),
        });

        let sync = OutboxSynchronizer::new(
            mock_sheets.clone(),
            tmp.path().to_path_buf(),
            "SHEET_ID_TEST".to_string(),
        );

        let count = sync.sync_once().await.unwrap();

        assert_eq!(count, 1);

        let conn = Connection::open(tmp.path()).unwrap();
        let synced: i32 = conn
            .query_row(
                "SELECT sheets_synced FROM repositorios WHERE project_name = 'acme/widget'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(synced, 1);
        assert_eq!(*mock_sheets.get_data_calls.lock().unwrap(), 1); // 1 Bulk Read O(1)
        assert_eq!(*mock_sheets.batch_update_calls.lock().unwrap(), 1); // 1 Batch Update

        // Validação ADR-006: Payload de escrita DEVE ter exatamente 1 range atômico único
        let payload = mock_sheets
            .last_batch_payload
            .lock()
            .unwrap()
            .clone()
            .expect("Payload deve ter sido gravado");
        let map = payload.as_object().expect("Payload deve ser um objeto JSON Map");
        assert_eq!(map.len(), 1, "Payload DEVE conter ESTRITAMENTE 1 range atômico único (ADR-006)");
        let (range, values) = map.iter().next().unwrap();
        assert_eq!(range, "A2:G2", "Range atômico deve cobrir da coluna A até G na linha 2");
        let rows = values.as_array().expect("Values deve ser uma matriz 2D (array de arrays)");
        assert_eq!(rows.len(), 1, "Matriz 2D deve ter exatamente 1 linha");
        let row_vals = rows[0].as_array().expect("Linha deve ser um array de colunas");
        assert_eq!(row_vals.len(), 7, "Linha deve conter todas as 7 colunas do header");
        assert_eq!(row_vals[2], "PENDENTE_HARVESTER", "Coluna status_fase atualizada");
        assert_eq!(row_vals[5], "Resumo técnico da ferramenta.", "Coluna proposta_original_resumo atualizada");
        assert_eq!(row_vals[6], "Tooling_Dev - CLI_Utilities", "Coluna categoria_arquitetural atualizada");
    }

    #[tokio::test]
    async fn test_outbox_sync_skips_when_no_unsynced_rows() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count, proposta_original_resumo, categoria_arquitetural, sheets_synced
            ) VALUES (
                'acme/widget', 'L1', 'https://github.com/acme/widget', 'v1.0.0', 'v1.0.0', 'v1.0.0',
                'UUID-1', 'FASE_-1_OK', 0, '', '', 1
            )",
            [],
        )
        .unwrap();
        drop(conn);

        let mock_sheets = Arc::new(MockSheetsClient {
            get_data_calls: Mutex::new(0),
            batch_update_calls: Mutex::new(0),
            last_batch_payload: Mutex::new(None),
        });

        let sync = OutboxSynchronizer::new(
            mock_sheets.clone(),
            tmp.path().to_path_buf(),
            "SHEET_ID_TEST".to_string(),
        );

        let count = sync.sync_once().await.unwrap();

        assert_eq!(count, 0);
        assert_eq!(*mock_sheets.get_data_calls.lock().unwrap(), 0); // No read if 0 unsynced
        assert_eq!(*mock_sheets.batch_update_calls.lock().unwrap(), 0); // No batch update
    }
}
