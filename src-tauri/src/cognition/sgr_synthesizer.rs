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
    pub fn synthesize_debate(debate: SwarmDebate) -> Result<SgrPayload, SgrError> {
        // PT-SGR-1: Aplicação do Score Punitivo antes da síntese final
        let (punitive_fit, punitive_final) = if Self::contains_toxic_stack(&debate) {
            (0, 10) // Score Punitivo: Nota 0 no Fit, derruba a nota final drasticamente
        } else {
            (90, 95) // Notas de exemplo para sucesso
        };

        // Simulação de Decodificação Restrita via llguidance
        // Em produção, isso seria uma chamada para o LLM com o template SGR
        Ok(SgrPayload {
            visao_do_enxame: "Síntese consolidada das Lentes A, B e C.".to_string(),
            justificativa_decisao: "A decisão foi tomada baseada no alinhamento bare-metal.".to_string(),
            executive_verdict: TerminalClassification::AprovadoParaProducao,
            cannibalization_action: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: punitive_fit,
            score_final: punitive_final,
        })
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

    #[test]
    fn test_punitive_score_enforcement() {
        let debate = SwarmDebate {
            repo_id: "test".to_string(),
            lente_a: "UX boa".to_string(),
            lente_b: "Arquitetura limpa".to_string(),
            lente_c: "Usa Node.js e Electron no backend".to_string(), // TÓXICO
        };

        let res = SgrSynthesizer::synthesize_debate(debate).unwrap();
        assert_eq!(res.score_bare_metal_fit, 0);
        assert!(res.score_final < 20);
    }

    #[test]
    fn test_successful_constrained_decoding() {
        let debate = SwarmDebate {
            repo_id: "test_ok".to_string(),
            lente_a: "A".to_string(),
            lente_b: "B".to_string(),
            lente_c: "C".to_string(),
        };

        let res = SgrSynthesizer::synthesize_debate(debate);
        assert!(res.is_ok());
        let payload = res.unwrap();
        assert_eq!(payload.score_bare_metal_fit, 90);
    }
}
