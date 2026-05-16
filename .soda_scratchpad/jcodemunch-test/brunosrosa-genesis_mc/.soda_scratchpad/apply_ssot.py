# coding=utf-8
import re

with open('src-tauri/src/persist/ssot_injector.rs', 'r', encoding='utf-8') as f:
    code = f.read()

with open('rep1.txt', 'r', encoding='utf-8') as f:
    rep1 = f.read()

with open('rep2.txt', 'r', encoding='utf-8') as f:
    rep2 = f.read()

# Replace conn.execute inside update_local_status
old_conn_execute = re.search(r'        conn\.execute\(\s*"INSERT OR REPLACE INTO repo_heuristics.*?rusqlite::params!\[.*?\],\s*\)\.map_err\(\|e\| format!\("Falha ao executar INSERT repo_heuristics: \{\}", e\)\)\?;', code, flags=re.DOTALL).group(0)

code = code.replace(old_conn_execute, rep1)

# Replace prepare_batch_payload
old_prepare = re.search(r'    fn prepare_batch_payload\(_repo_id: &str, payload: SgrPayload\) -> Value \{.*?    \}', code, flags=re.DOTALL).group(0)

code = code.replace(old_prepare, rep2)

# Replace test test_anti_503_batch_slicing
old_test_503 = re.search(r'    #\[test\]\n    fn test_anti_503_batch_slicing\(\) \{.*?    \}', code, flags=re.DOTALL).group(0)
new_test_503 = '''    #[test]
    fn test_anti_503_batch_slicing() {
        let payload = mock_payload();
        let batch = SsotInjector::prepare_batch_payload("repo_1", payload);
        let arr = batch["A2:CD2"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_array().unwrap().len(), 82);
    }'''
code = code.replace(old_test_503, new_test_503)

# Replace mock_payload
old_mock = re.search(r'    fn mock_payload\(\) -> SgrPayload \{.*?    \}', code, flags=re.DOTALL).group(0)
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
code = code.replace(old_mock, new_mock)

with open('src-tauri/src/persist/ssot_injector.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Applied")
