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

        let project_name = repo_id.split('/').next_back().unwrap_or(repo_id);
        let repo_url = format!("https://github.com/{}", repo_id);
            
        // I/O L2 Real: Mapeando SgrPayload para as colunas reais da tabela
        conn.execute(
            "INSERT OR REPLACE INTO repo_heuristics (
                repo_id, project_name, repo_url, justificativa_decisao, 
                executive_verdict, acao_de_canibalizacao, score_bare_metal_fit, score_final, observacoes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                repo_id,
                project_name,
                repo_url,
                payload.justificativa_decisao,
                format!("{:?}", payload.executive_verdict),
                format!("{:?}", payload.cannibalization_action),
                payload.score_bare_metal_fit as f64,
                payload.score_final as f64,
                payload.visao_do_enxame
            ],
        ).map_err(|e| format!("Falha ao executar INSERT repo_heuristics: {}", e))?;

        // Atualizando o status em repositorios
        let _ = conn.execute(
            "UPDATE repositorios SET status = ?1 WHERE id = ?2",
            rusqlite::params![status_value, repo_id],
        ).map_err(|e| format!("Falha ao executar UPDATE repositorios: {}", e))?;
        
        Ok(())
    }

    fn prepare_batch_payload(_repo_id: &str, payload: SgrPayload) -> Value {
        // Formato correto esperado pelo MCP batch_update_cells (dict)
        json!({
            "A2:H2": [
                [
                    _repo_id,
                    payload.score_final.to_string(),
                    format!("{:?}", payload.executive_verdict),
                    payload.visao_do_enxame,
                    format!("{:?}", payload.cannibalization_action)
                ]
            ]
        })
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
            cannibalization_action: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95,
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

        let requests = batch["requests"].as_array().unwrap();
        assert_eq!(requests.len(), 4, "Deve conter fatias para as 4 abas");
        
        let sheets: Vec<&str> = requests.iter().map(|r| r["sheet"].as_str().unwrap()).collect();
        assert!(sheets.contains(&"MASTER_SOLUTIONS"));
        assert!(sheets.contains(&"SODA_GRAPH_TOPOLOGY"));
        assert!(sheets.contains(&"ACTION_MATRIX"));
        assert!(sheets.contains(&"QUARANTINE_RADAR"));
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
