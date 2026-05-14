# coding=utf-8
import re

with open('src-tauri/src/cognition/sgr_synthesizer.rs', 'r', encoding='utf-8') as f:
    code = f.read()

# Add #[derive(Default)] to enums
code = code.replace(
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub enum TerminalClassification {',
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub enum TerminalClassification {\n    #[default]'
)
code = code.replace(
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub enum CannibalizationAction {',
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub enum CannibalizationAction {\n    #[default]'
)

with open('gen_code.rs', 'r', encoding='utf-8') as f:
    gen_code = f.read()

sgr_struct = gen_code.split('INSERT')[0].strip()

# Add Default to struct
sgr_struct = sgr_struct.replace('#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]', '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]')

# Replace struct definition
struct_pattern = r'#\[derive\(Debug, Clone, Serialize, Deserialize, PartialEq, Eq\)\]\npub struct SgrPayload \{.*?\}'
code = re.sub(struct_pattern, sgr_struct, code, flags=re.DOTALL)

# Replace synthesize_debate JSON decode logic
old_decode = '''                        let payload: SgrPayload = serde_json::from_str(clean_text)
                            .map_err(|e| SgrError::DecodingError(format!("JSON inválido: {}", e)))?;
                        return Ok(payload);'''

new_decode = '''                        let mut payload: SgrPayload = serde_json::from_str(clean_text)
                            .map_err(|e| SgrError::DecodingError(format!("JSON inválido: {}", e)))?;
                        
                        if Self::contains_toxic_stack(&debate) {
                            payload.score_bare_metal_fit = 0;
                            payload.bare_metal_fit = "LOW".to_string();
                        }
                        
                        return Ok(payload);'''
code = code.replace(old_decode, new_decode)

# Replace test test_sgr_struct_ordering
old_test = '''    #[test]
    fn test_sgr_struct_ordering() {
        // Valida se a serialização segue a ordem do SGR
        let payload = SgrPayload {
            visao_do_enxame: "V".to_string(),
            justificativa_decisao: "J".to_string(),
            executive_verdict: TerminalClassification::AprovadoParaProducao,
            cannibalization_action: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95,
        };'''

new_test = '''    #[test]
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
        };'''
code = code.replace(old_test, new_test)

with open('src-tauri/src/cognition/sgr_synthesizer.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Patch SGR applied")
