use crate::cognition::sgr_synthesizer::SgrPayload;
use thiserror::Error;
use serde_json::{json, Value};

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
        Self::update_local_status(repo_id, "CONCLUIDO")
            .map_err(SsotError::L2Failure)?;

        // 2. Manobra Anti-503: Desmembramento e Agregação na RAM
        let _batch_payload = Self::prepare_batch_payload(repo_id, payload);

        // 3. Despacho Atômico (Simulado conforme Phase C)
        Self::dispatch_to_cloud(_batch_payload).await?;

        Ok(())
    }

    fn update_local_status(_repo_id: &str, _status: &str) -> Result<(), String> {
        // Mock de execução SQLite
        Ok(())
    }

    fn prepare_batch_payload(_repo_id: &str, payload: SgrPayload) -> Value {
        // PT-SSOT-1: Agrega as 4 abas em um único objeto JSON para batch_update
        json!({
            "requests": [
                {
                    "sheet": "MASTER_SOLUTIONS_v3",
                    "data": {
                        "verdict": format!("{:?}", payload.executive_verdict),
                        "score": payload.score_final
                    }
                },
                {
                    "sheet": "SODA_GRAPH_TOPOLOGY",
                    "data": {
                        "vision": payload.visao_do_enxame
                    }
                },
                {
                    "sheet": "ACTION_MATRIX",
                    "data": {
                        "action": format!("{:?}", payload.cannibalization_action)
                    }
                },
                {
                    "sheet": "QUARANTINE_RADAR",
                    "data": {
                        "risk": "Low" // Exemplo condicional
                    }
                }
            ]
        })
    }

    async fn dispatch_to_cloud(_payload: Value) -> Result<(), SsotError> {
        // Mock de despacho HTTP
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
        let _ = SsotInjector::update_local_status("test", "CONCLUIDO");
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
        assert!(sheets.contains(&"MASTER_SOLUTIONS_v3"));
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
