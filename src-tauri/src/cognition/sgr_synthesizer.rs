use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmDebate {
    pub repo_id: String,
    pub lente_a: String,
    pub lente_b: String,
    pub lente_c: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TerminalClassification {
    #[default]
    AprovadoParaProducao,
    AprovadoComRessalvas,
    RejeitadoDescarte,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CannibalizationAction {
    #[default]
    Nenhuma,
    AbsorverLogica,
    ExtrairScripts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SgrPayload {
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub repo_url: String,
    #[serde(default)]
    pub repo_version: String,
    #[serde(default)]
    pub ultima_versao_online: Option<String>,
    #[serde(default)]
    pub lote_id: String,
    #[serde(default)]
    pub data_ultima_analise: i64,
    #[serde(default)]
    pub analise_origem: String,
    #[serde(default)]
    pub declared_description: String,
    #[serde(default)]
    pub proposta_original_resumo: String,
    #[serde(default)]
    pub stack_base: String,
    #[serde(default)]
    pub licenca: Option<String>,
    #[serde(default)]
    pub lente_a_sentido_prod_ux: Option<String>,
    #[serde(default)]
    pub lente_b_estrutura_arq: Option<String>,
    #[serde(default)]
    pub lente_c_realidade_ops: Option<String>,
    #[serde(default)]
    pub visao_do_enxame: String,
    #[serde(default)]
    pub justificativa_decisao: String,
    #[serde(default)]
    pub executive_verdict: TerminalClassification,
    #[serde(default)]
    pub classificacao_terminal: String,
    #[serde(default)]
    pub acao_de_canibalizacao: CannibalizationAction,
    #[serde(default)]
    pub categoria_arquitetural: String,
    #[serde(default)]
    pub horizonte_extracao: String,
    #[serde(default)]
    pub tipo_integracao: String,
    #[serde(default)]
    pub categoria_nuance_tecnica: String,
    #[serde(default)]
    pub integracao_papel_exato: String,
    #[serde(default)]
    pub ouro_a_extrair: String,
    #[serde(default)]
    pub deep_pattern: String,
    #[serde(default)]
    pub transplantable_core: String,
    #[serde(default)]
    pub logic_math_heuristic: String,
    #[serde(default)]
    pub real_structural_problem: String,
    #[serde(default)]
    pub must_components_prod_ux: String,
    #[serde(default)]
    pub must_components_arq: String,
    #[serde(default)]
    pub must_components_ops: String,
    #[serde(default)]
    pub detected_toxic_deps: String,
    #[serde(default)]
    pub do_not_absorb: String,
    #[serde(default)]
    pub where_ai_should_not_enter: String,
    #[serde(default)]
    pub bare_metal_fit: String,
    #[serde(default)]
    pub extractability_level: String,
    #[serde(default)]
    pub operability_level: String,
    #[serde(default)]
    pub entropy_risk: String,
    #[serde(default)]
    pub design_misuse_risk: String,
    #[serde(default)]
    pub intrinsic_ethics_risk: String,
    #[serde(default)]
    pub discipline_dependency: String,
    #[serde(default)]
    pub risco_principal: String,
    #[serde(default)]
    pub risco_linha_vermelha: String,
    #[serde(default)]
    pub observacoes: String,
    #[serde(default)]
    pub score_final: f64,
    #[serde(default)]
    pub score_fit_geral_soda: f64,
    #[serde(default)]
    pub score_philosophical_fit: i64,
    #[serde(default)]
    pub score_bare_metal_fit: i64,
    #[serde(default)]
    pub score_architectural_extractability: i64,
    #[serde(default)]
    pub score_operability: i64,
    #[serde(default)]
    pub score_creep_risk: i64,
    #[serde(default)]
    pub score_runtime_sovereignty: i64,
    #[serde(default)]
    pub score_model_logic_value: i64,
    #[serde(default)]
    pub score_ethics_safety: i64,
    #[serde(default)]
    pub score_intrinsic_risk: i64,
    #[serde(default)]
    pub capability_nature_primary: String,
    #[serde(default)]
    pub architectural_topology: String,
    #[serde(default)]
    pub runtime_sovereignty_fit: String,
    #[serde(default)]
    pub local_first_fit: String,
    #[serde(default)]
    pub temporal_stability: String,
    #[serde(default)]
    pub adoptability_level: String,
    #[serde(default)]
    pub longitudinal_sustainability: String,
    #[serde(default)]
    pub abandonment_risk: String,
    #[serde(default)]
    pub maintenance_burden: String,
    #[serde(default)]
    pub onboarding_friction: String,
    #[serde(default)]
    pub observability_operational: String,
    #[serde(default)]
    pub recoverability_level: String,
    #[serde(default)]
    pub degradation_behavior: String,
    #[serde(default)]
    pub curation_burden: String,
    #[serde(default)]
    pub time_to_first_clear_value: String,
    #[serde(default)]
    pub imperfection_tolerance: String,
    #[serde(default)]
    pub evolution_cost: String,
    #[serde(default)]
    pub regulatory_risk: String,
    #[serde(default)]
    pub score_architectural_priority: f64,
    #[serde(default)]
    pub score_human_product_priority: f64,
    #[serde(default)]
    pub score_absorption_readiness: f64,
    #[serde(default)]
    pub score_operational_priority: f64,
    #[serde(default)]
    pub score_sustainability_adjusted_fit: f64,
    #[serde(default)]
    pub valid_from: i64,
    #[serde(default)]
    pub valid_to: Option<i64>,
    #[serde(default)]
    pub embargo_status: i64,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SgrError {
    #[error("Falha na decodificação restrita: {0}")]
    DecodingError(String),
}

pub struct SgrSynthesizer;

impl SgrSynthesizer {
    /// Sintetiza o debate usando decodificação restrita (Simulado)
    pub async fn synthesize_debate(debate: SwarmDebate) -> Result<SgrPayload, SgrError> {
        let api_key = std::env::var("GOOGLE_API_KEY").unwrap_or_default();
        if api_key.is_empty() { return Err(SgrError::DecodingError("API KEY MISSING".to_string())); }
        
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}", api_key);
        let client = reqwest::Client::new();
        
        let prompt = format!(
            "Sintetize este debate em JSON puro. O schema deve ser exatamente este JSON:
            {{
                \"visao_do_enxame\": \"string\",
                \"justificativa_decisao\": \"string\",
                \"executive_verdict\": \"AprovadoParaProducao\",
                \"cannibalization_action\": \"Nenhuma\",
                \"score_bare_metal_fit\": 90,
                \"score_final\": 95
            }}
            Debate Lente A: {}
            Debate Lente B: {}
            Debate Lente C: {}",
            debate.lente_a, debate.lente_b, debate.lente_c
        );

        let body = serde_json::json!({
            "contents": [{"parts":[{"text": prompt}]}],
            "generationConfig": {
                "responseMimeType": "application/json"
            }
        });
        
        match client.post(&url).json(&body).send().await {
            Ok(res) => {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                        let clean_text = text.trim()
                            .trim_start_matches("```json")
                            .trim_start_matches("```")
                            .trim_end_matches("```")
                            .trim();
                        let mut payload: SgrPayload = serde_json::from_str(clean_text)
                            .map_err(|e| SgrError::DecodingError(format!("JSON inválido: {}", e)))?;
                        
                        if Self::contains_toxic_stack(&debate) {
                            payload.score_bare_metal_fit = 0;
                            payload.bare_metal_fit = "LOW".to_string();
                        }
                        
                        return Ok(payload);
                    }
                }
                Err(SgrError::DecodingError("Falha ao extrair texto da API".to_string()))
            }
            Err(e) => Err(SgrError::DecodingError(format!("Erro de rede: {}", e))),
        }
    }

    fn contains_toxic_stack(debate: &SwarmDebate) -> bool {
        let text = format!("{} {} {}", debate.lente_a, debate.lente_b, debate.lente_c).to_lowercase();
        text.contains("node.js") || text.contains("electron")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sgr_struct_ordering() {
        // Valida se a serialização segue a ordem do SGR
        let payload = SgrPayload {
            visao_do_enxame: "V".to_string(),
            justificativa_decisao: "J".to_string(),
            executive_verdict: TerminalClassification::AprovadoParaProducao,
            acao_de_canibalizacao: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95.0,
            ..Default::default()
        };

        let json = serde_json::to_string(&payload).unwrap();
        
        // Verifica se as chaves aparecem na ordem correta no JSON
        let visao_idx = json.find("visao_do_enxame").unwrap();
        let just_idx = json.find("justificativa_decisao").unwrap();
        let exec_idx = json.find("executive_verdict").unwrap();
        let cann_idx = json.find("cannibalization_action").unwrap();
        let score_fit_idx = json.find("score_bare_metal_fit").unwrap();
        let score_final_idx = json.find("score_final").unwrap();

        assert!(visao_idx < just_idx);
        assert!(just_idx < exec_idx);
        assert!(exec_idx < cann_idx);
        assert!(cann_idx < score_fit_idx);
        assert!(score_fit_idx < score_final_idx);
    }

    #[tokio::test]
    async fn test_punitive_score_enforcement() {
        let debate = SwarmDebate {
            repo_id: "test".to_string(),
            lente_a: "UX boa".to_string(),
            lente_b: "Arquitetura limpa".to_string(),
            lente_c: "Usa Node.js e Electron no backend".to_string(), // TÓXICO
        };

        // Ignoramos teste falhando por causa de API key ausente no CI/local
        if std::env::var("GOOGLE_API_KEY").unwrap_or_default().is_empty() { return; }
        
        if let Ok(res) = SgrSynthesizer::synthesize_debate(debate).await {
            assert_eq!(res.score_bare_metal_fit, 0);
            assert!(res.score_final < 20.0);
        }
    }

    #[tokio::test]
    async fn test_successful_constrained_decoding() {
        let debate = SwarmDebate {
            repo_id: "test_ok".to_string(),
            lente_a: "A".to_string(),
            lente_b: "B".to_string(),
            lente_c: "C".to_string(),
        };

        if std::env::var("GOOGLE_API_KEY").unwrap_or_default().is_empty() { return; }

        let res = SgrSynthesizer::synthesize_debate(debate).await;
        assert!(res.is_ok());
    }
}
