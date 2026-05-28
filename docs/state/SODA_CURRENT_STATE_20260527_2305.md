# SODA Current State — 20260527_2305

Este documento é uma foto técnica do estado operacional do Pipeline ETL Cognitivo SODA (V3) no workspace `genesis_mc` neste timestamp.

## 1) Fronteira de I/O (Cérebro vs Janela de Vidro)

### Cérebro / Input (L2: SQLite)
- Banco local (estado, blobs, essências e heurísticas): `.soda_data/soda_heuristic_vault.db`
- Papel: SSOT local e durável da execução (Execução Durável). O pipeline sempre consegue retomar, reprocessar e auditar o que foi extraído/decidido sem depender do Sheets.
- Tabelas relevantes:
  - `repositorios`: fila de ingestão e estado alto nível por `project_name` (repo_id).
  - `artefatos_brutos`: blobs mecânicos da Fase 0 (F1/Harvester) e outros artefatos.
  - `artefatos_destilados`, `pacotes_destilados`: Fase 1.5 (FinOps distillation + packages A/B/C).
  - `debates_enxame`: saídas da Fase 2 (Lentes A/B/C).
  - `repo_heuristics`: estado consolidado pós-Fase 4 (resultado do SGR + metadados base).

### Janela de Vidro / Output (L0: Google Sheets via MCP)
- Planilha SSOT humana: `MASTER_SOLUTIONS` (Google Sheets).
- Acesso: via MCP `mcp-google-sheets` roteado pelo AgentGateway (configurado em `gateway-config.yaml`).
- Escrita: sempre por `batch_update_cells` para minimizar round-trips e manter escrita atômica por ranges.
- Proteção operacional: `mcp_stdio_guard` impõe guilhotina (timeout) por target. Para `mcp-google-sheets`, o timeout foi elevado para permitir batches maiores.

## 2) Orquestrador de Lotes (HITL)

### Script
- Arquivo: `src-tauri/soda_batch_runner.ps1`
- Função: “Janela de Vidro” operacional com HITL.
  - Pré-flight: imprime lote e fila (`$BATCH_ID` + `$REPOS`).
  - Sincroniza a fila no SQLite (inserindo/upsertando em `repositorios` com `status_processamento='PENDENTE'` e `lote_id` do lote).
  - HITL: pede aprovação explícita antes de ignição do motor Rust.
  - Ignição: dispara o bin `f3_synthesizer_cli` em modo `--e2e-full` (F0→F4).

### Observação de CWD (importante)
- O script usa paths relativos para o SQLite (`.soda_data/soda_heuristic_vault.db`) e invoca `cargo run`.
- Na prática, o operador deve garantir que o terminal esteja num CWD onde:
  - `.soda_data/` seja resolvido corretamente (raiz do projeto), e
  - `cargo run` encontre o `Cargo.toml` (pasta `src-tauri/`).

## 3) Guardião (Fase -1)

### Binário
- Arquivo: `src-tauri/src/bin/f_minus_1_guardian.rs`
- Execução: varre o Sheets (MCP) e preenche/atualiza lacunas de versão/estado, além de calcular drift.

### Função operacional
- Leitura do Sheets: obtém header e dados; mapeia colunas dinamicamente por cabeçalho (sem índices hardcoded).
- Resolução de versão (GitHub):
  - Preferência por Release mais recente.
  - Fallback para tags.
  - Fallback final para SHA curto (7 chars) do último commit da branch default.
- Escrita:
  - Micro-lotes (chunking) para `batch_update_cells` (reduz payload e evita asfixia de stdio/Batch API).
  - Timeout do lado Rust ajustado para operações em lote.
- Persistência local:
  - Atualiza SQLite local de forma durável conforme o estado evolui.

## 4) Trator ETL (Fase 3 → Fase 4)

### Binário principal
- Arquivo: `src-tauri/src/bin/f3_synthesizer_cli.rs`
- Modo de lote: `--e2e-full` executa F0 (Harvester) → F1.5 (Distiller) → F2 (Swarm) → F3 (SGR/Formatter) → F4 (SSOT Injector).

### Fôlego narrativo (maxLength)
- O schema de Structured Outputs (JSON Schema strict) foi ajustado para evitar truncamento no meio de palavras:
  - Campos narrativos críticos: `maxLength` ~3000.
  - `declared_description` (derivado do README): ~2000 (preservando palavra inteira).
- Objetivo: impedir “amputação” causada por validação estrita (o modelo corta cedo para obedecer `maxLength`).

### Persistência e bifurcação temporal
- Sheets + relatórios: data/hora em ISO-8601 com offset de Brasília (`-03:00`).
- SQLite: timestamps permanecem como `INTEGER` epoch (i64). Schema local não é alterado para strings temporais.

## 5) Comandos de Ignição (fluxo atual)

### A) Atualizar fila do lote (HITL)
1) Abrir terminal no projeto (com ambiente Python + Cargo disponíveis).
2) Executar:
   - `pwsh -File .\\src-tauri\\soda_batch_runner.ps1`
3) Confirmar (S) quando solicitado para iniciar o motor Rust.

### B) Rodar o Guardião (Fase -1)
- Exemplo:
  - `cargo run -q --features tauri-app --bin f_minus_1_guardian -- --sheets-id $env:GOOGLE_SHEETS_ID`
- Flags:
  - `--dry-run` faz varredura sem mutação.

### C) Rodar o Trator ETL diretamente (sem script)
- Dentro de `src-tauri/`:
  - `cargo run -q --features tauri-app --bin f3_synthesizer_cli -- --repo owner/repo --e2e-full`

## Apêndice: artefatos operacionais
- Reports por repo (append): `.soda_scratchpad/reports/_ETL_REPORT_{owner}_{repo}.txt`
- Config AgentGateway (MCP): `gateway-config.yaml`
