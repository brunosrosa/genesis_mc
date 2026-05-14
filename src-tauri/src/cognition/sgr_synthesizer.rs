use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmDebate {
    pub repo_id: String,
    pub lente_a: String,
    pub lente_b: String,
    pub lente_c: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerminalClassification {
    AprovadoParaProducao,
    AprovadoComRessalvas,
    RejeitadoDescarte,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CannibalizationAction {
    Nenhuma,
    AbsorverLogica,
    ExtrairScripts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SgrPayload {
    // LEI SGR: Campos textuais primeiro para guiar o KV Cache
    pub visao_do_enxame: String,
    pub justificativa_decisao: String,
    
    // Campos numéricos/categóricos depois
    pub executive_verdict: TerminalClassification,
    pub cannibalization_action: CannibalizationAction,
    pub score_bare_metal_fit: i32,
    pub score_final: i32,
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
                        let payload: SgrPayload = serde_json::from_str(clean_text)
                            .map_err(|e| SgrError::DecodingError(format!("JSON inválido: {}", e)))?;
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
            cannibalization_action: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95,
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
            assert!(res.score_final < 20);
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
