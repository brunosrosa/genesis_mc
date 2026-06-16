# Debug Session: watcher-row-720

Status: FIXED

Symptom:
- O `n0_daemon_watcher` reporta `linhas=719` e não detecta a linha 720 da `MASTER_SOLUTIONS`.
- O `f_minus_1_guardian` tenta processar `webassembly/wasi`, mas falha ao persistir no SQLite porque nenhuma linha foi atualizada.

Repo alvo:
- `repo_url` informado: ` `https://github.com/webassembly/wasi` `
- Linha esperada no Sheets: `720`

Hypotheses:
1. A linha 720 não está presente no payload retornado pelo Google Sheets para o range lido pelo watcher.
2. O valor de `repo_url` contém crases/espaços e a linha não é reconhecida como válida no fluxo inicial.
3. O adaptador `google_workspace_mcp` está truncando a leitura antes da última linha não vazia.
4. O guardião lê a planilha, mas o watcher nunca criou a linha correspondente em `repositorios`.
5. A coluna canônica usada pelo watcher não é a mesma onde o valor foi preenchido na planilha.

Plan:
1. Inspecionar watcher, guardião e adaptador de leitura do Sheets.
2. Reproduzir a leitura/ranges até a linha 720.
3. Determinar se o problema é leitura do Sheets, coluna errada ou normalização do valor.
4. Instrumentar apenas o caminho de leitura, se necessário.

Findings:
- `linhas=719` no watcher corresponde exatamente às linhas `2..720`; portanto a linha `720` está dentro do payload.
- Instrumentação no watcher confirmou:
  - `row_number_1based=720`
  - `raw_repo_url=https://github.com/webassembly/wasi`
  - `raw_project_name=`
  - `raw_status_atualizacao=NOVO_LINK_OK`
  - `raw_status_fase=`
- O watcher não roteia `NOVO_LINK_OK`; o catálogo atual só trata vazio como `N1`, depois `INICIAR_TRIAGEM`, `APROVADO_*` e `REJEITADO_*`.
- O watcher do `route 0` usa `NoopDispatcher` no `main`, então ele não cria linha no SQLite nem materializa `repositorios`.
- O guardião enxerga o link e tenta persistir a versão, mas falha porque `repositorios` ainda não contém `webassembly/wasi`.

Current Conclusion:
- O problema não é limite de leitura nem corte em `719`.
- A quebra real é de fluxo:
  1. `N0` lê a linha `720`, mas faz `Skip` para `NOVO_LINK_OK`.
  2. `N0` também não tem dispatcher real para criar estado local.
  3. `N1`/guardião tenta operar sobre um repo que ainda não existe no SQLite.

Fixes Applied:
- `route_for_status_atualizacao("")` e `route_for_status_atualizacao("NOVO_LINK_OK")` agora convergem para `N1`.
- O `main` do watcher deixou de usar `NoopDispatcher` e passou a usar um dispatcher real para `N1`.
- O dispatcher real faz `upsert` em `repositorios`, derivando `project_name` do `repo_url` quando necessário e usando `lote_id` da planilha quando existir.
- Quando a linha entra vazia em `status_atualizacao`, o `N0` agora escreve `NOVO_LINK_OK` no Sheets após sucesso local.

Post-fix Evidence:
- Teste: `cargo test --features tauri-app --bin n0_daemon_watcher` -> `0`
- Bin: `cargo check --features tauri-app --bin n0_daemon_watcher` com `CARGO_INCREMENTAL=0` -> `0`
- Runtime: `cargo run --features tauri-app --bin n0_daemon_watcher -- --once` com `CARGO_INCREMENTAL=0` -> `0`
- Snapshot pós-fix do SQLite para `webassembly/wasi`:
  - `project_name = webassembly/wasi`
  - `repo_url = https://github.com/webassembly/wasi`
  - `status_processamento = PENDENTE`

Notes:
- O `rustc` incremental no Windows ainda pode gerar ICE/acesso negado esporádico; com `CARGO_INCREMENTAL=0` a validação do bin concluiu normalmente.
- A instrumentação temporária no watcher foi removida após confirmação do usuário.
