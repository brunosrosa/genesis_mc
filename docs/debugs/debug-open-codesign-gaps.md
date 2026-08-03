# Debug Session: open-codesign-gaps

Status: FIXED

Symptom:
- O repo `https://github.com/OpenCoworkAI/open-codesign` tem seis colunas vazias na `MASTER_SOLUTIONS`:
  - `visao_do_enxame`
  - `justificativa_decisao`
  - `executive_verdict`
  - `risco_principal`
  - `risco_linha_vermelha`
  - `observacoes`

Hypotheses:
1. Os campos não foram gerados no SQLite/artefato local da F3.
2. Os campos existem localmente, mas a F4 não os escreveu no Sheets.
3. O resolvedor de linha acertou o repo, mas escreveu em outra linha.
4. A síntese original já veio parcial, com esses campos vazios.
5. Houve falha parcial/retry na injeção e o status final mascarou o furo.

Plan:
1. Localizar a linha/estado do repo no SQLite e na planilha.
2. Verificar se os seis campos existem em `repo_heuristics` ou artefato equivalente.
3. Confirmar o caminho de escrita dessas colunas no `ssot_injector`.
4. Se necessário, regenerar especificamente o repo e reinjetar a linha.

Findings:
- `repositorios` estava íntegro:
  - `project_name = OpenCoworkAI/open-codesign`
  - `status_processamento = CONCLUIDO`
  - `repo_analised_version = v0.2.1`
- Os seis campos faltantes estavam vazios também em `repo_heuristics`; portanto o furo não era exclusivo do Sheets.
- `repo_heuristics.status_fase = FASE_4_SHEETS_UPDATED` e `status_atualizacao = CONCLUIDO_AGUARDANDO` mesmo com o bloco 1 vazio.
- Instrumentação em `run_phase3_sgr` confirmou o falso checkpoint:
  - `stage_from_status = 5`
  - `stage_from_content = 0`
  - todos os seis campos do bloco 1 estavam vazios.

Root Cause:
- O sintetizador confiava em `status_fase = FASE_4_SHEETS_UPDATED` como checkpoint terminal e pulava a F3, mesmo quando o payload persistido no SQLite ainda estava incompleto.
- Resultado: a F4 reinjetava no Sheets exatamente a mesma linha parcial.

Fixes Applied:
- Adicionada instrumentação temporária no checkpoint da F3 para evidenciar divergência entre `stage_from_status` e `stage_from_content`.
- Corrigida a reconciliação de estágio em `src-tauri/src/cognition/synthesizer.rs`:
  - quando o status indica estágio terminal (`>= 5`) mas o conteúdo persistido está abaixo de `5`, o SGR invalida o checkpoint e volta a confiar no estágio inferido do payload.
- Adicionado teste unitário `terminal_checkpoint_does_not_override_incomplete_payload_stage`.

Operational Replay:
- Reexecução repo-específica:
  - `cargo run --features tauri-app --bin f3_synthesizer_cli -- --repo OpenCoworkAI/open-codesign --e2e-full`
- A F3 voltou a executar:
  - bloco 1 -> OK
  - bloco 2A -> OK
  - bloco 2B -> OK
  - bloco 3 -> OK
  - bloco 4 -> OK
- A F4 concluiu com confirmação de escrita no Sheets para a linha `60`.

Post-fix Evidence:
- SQLite (`repo_heuristics`) agora contém texto não vazio em:
  - `visao_do_enxame`
  - `justificativa_decisao`
  - `executive_verdict`
  - `risco_principal`
  - `risco_linha_vermelha`
  - `observacoes`
- Teste:
  - `cargo test --features tauri-app --lib terminal_checkpoint_does_not_override_incomplete_payload_stage` -> `0`
- Check:
  - `cargo check --features tauri-app --lib` -> `0`

Cleanup:
- A instrumentação temporária do checkpoint da F3 foi removida após confirmação visual do usuário na planilha.
- Permaneceram apenas a correção funcional e o teste de regressão.
