import json
import csv

rust_types = {'INTEGER': 'i64', 'REAL': 'f64', 'TEXT': 'String'}
cols = []
types = []
reqs = []
with open('docs/architecture/DATABASE_SCHEMA_DIC.csv', 'r', encoding='utf-8') as f:
    reader = csv.DictReader(f)
    for row in reader:
        if row['Tabela de Destino (SQLite)'] == 'repo_heuristics':
            cols.append(row['Nome da Coluna'].strip())
            types.append(rust_types[row['Tipo SQL (SQLite)'].strip()])
            reqs.append(row['Obrigatoriedade'].strip())

struct_lines = []
for c, t, req in zip(cols, types, reqs):
    if c == 'executive_verdict': t = 'TerminalClassification'
    elif c == 'acao_de_canibalizacao': t = 'CannibalizationAction'
    elif c == 'classificacao_terminal': t = 'String'
    if req != 'NOT NULL': t = f'Option<{t}>'
    struct_lines.append(f'    #[serde(default)]\n    pub {c}: {t},')
sgr_struct = '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]\npub struct SgrPayload {\n' + '\n'.join(struct_lines) + '\n}'

sql_insert = 'INSERT OR REPLACE INTO repo_heuristics (\n                ' + ', '.join(cols) + '\n            ) VALUES (\n                ' + ', '.join(f'?{i+1}' for i in range(len(cols))) + '\n            )'

params_lines = []
for c in cols:
    if c in ('executive_verdict', 'acao_de_canibalizacao'):
        params_lines.append(f'                format!("{{:?}}", payload.{c}),')
    else:
        if c in ('score_final', 'score_fit_geral_soda', 'score_architectural_priority', 'score_human_product_priority', 'score_absorption_readiness', 'score_operational_priority', 'score_sustainability_adjusted_fit'):
            params_lines.append(f'                payload.{c} as f64,')
        else:
            params_lines.append(f'                payload.{c},')
params_block = 'rusqlite::params![\n' + '\n'.join(params_lines) + '\n            ]'

batch_lines = []
for c in cols:
    if c in ('executive_verdict', 'acao_de_canibalizacao'):
        batch_lines.append(f'                    json!(format!("{{:?}}", payload.{c})),')
    else:
        batch_lines.append(f'                    json!(payload.{c}),')
batch_block = 'vec![\n                json!(vec![\n' + '\n'.join(batch_lines) + '\n                ])\n            ]'

# Patch sgr_synthesizer.rs
with open('src-tauri/src/cognition/sgr_synthesizer.rs', 'r', encoding='utf-8') as f:
    sgr = f.read()

sgr = sgr.replace(
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub enum TerminalClassification {',
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub enum TerminalClassification {\n    #[default]'
)
sgr = sgr.replace(
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub enum CannibalizationAction {',
    '#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub enum CannibalizationAction {\n    #[default]'
)

import re
sgr = re.sub(r'#\[derive\(Debug, Clone, Serialize, Deserialize, PartialEq, Eq\)\]\npub struct SgrPayload \{.*?\}', sgr_struct, sgr, flags=re.DOTALL)

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
sgr = sgr.replace(old_decode, new_decode)

old_test = '''        let payload = SgrPayload {
            visao_do_enxame: "V".to_string(),
            justificativa_decisao: "J".to_string(),
            executive_verdict: TerminalClassification::AprovadoParaProducao,
            cannibalization_action: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95,
        };'''
new_test = '''        let payload = SgrPayload {
            visao_do_enxame: "V".to_string(),
            justificativa_decisao: "J".to_string(),
            executive_verdict: TerminalClassification::AprovadoParaProducao,
            acao_de_canibalizacao: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95.0,
            ..Default::default()
        };'''
sgr = sgr.replace(old_test, new_test)

with open('src-tauri/src/cognition/sgr_synthesizer.rs', 'w', encoding='utf-8') as f:
    f.write(sgr)


# Patch ssot_injector.rs
with open('src-tauri/src/persist/ssot_injector.rs', 'r', encoding='utf-8') as f:
    ssot = f.read()

old_exec = re.search(r'conn\.execute\(\s*"INSERT OR REPLACE INTO repo_heuristics.*?rusqlite::params!\[.*?\],\s*\)', ssot, flags=re.DOTALL).group(0)
new_exec = f'conn.execute(\n            "{sql_insert}",\n            {params_block},\n        )'
ssot = ssot.replace(old_exec, new_exec)

old_prep = re.search(r'    fn prepare_batch_payload\(_repo_id: &str, payload: SgrPayload\) -> Value \{.*?    \}', ssot, flags=re.DOTALL).group(0)
new_prep = f'''    fn prepare_batch_payload(_repo_id: &str, payload: SgrPayload) -> Value {{
        let batch_payload = {batch_block};
        let mut map = serde_json::Map::new();
        map.insert("A2:CD2".to_string(), json!(batch_payload));
        Value::Object(map)
    }}'''
ssot = ssot.replace(old_prep, new_prep)

old_test_503 = re.search(r'    #\[test\]\n    fn test_anti_503_batch_slicing\(\) \{.*?    \}', ssot, flags=re.DOTALL).group(0)
new_test_503 = '''    #[test]
    fn test_anti_503_batch_slicing() {
        let payload = mock_payload();
        let batch = SsotInjector::prepare_batch_payload("repo_1", payload);
        let arr = batch["A2:CD2"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_array().unwrap().len(), 82);
    }'''
ssot = ssot.replace(old_test_503, new_test_503)

old_mock = re.search(r'    fn mock_payload\(\) -> SgrPayload \{.*?    \}', ssot, flags=re.DOTALL).group(0)
new_mock = '''    fn mock_payload() -> SgrPayload {
        SgrPayload {
            visao_do_enxame: "V".to_string(),
            justificativa_decisao: "J".to_string(),
            executive_verdict: TerminalClassification::AprovadoParaProducao,
            acao_de_canibalizacao: CannibalizationAction::Nenhuma,
            score_bare_metal_fit: 90,
            score_final: 95.0,
            ..Default::default()
        }
    }'''
ssot = ssot.replace(old_mock, new_mock)

with open('src-tauri/src/persist/ssot_injector.rs', 'w', encoding='utf-8') as f:
    f.write(ssot)

print("Both files fully patched")
