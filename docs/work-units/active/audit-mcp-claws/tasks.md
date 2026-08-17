# Tasks — Operação Auto-Raio-X: Auditoria Comportamental e Estresse das 50 Garras MCP

## 1. Fase de Preparação e Governança Territorial
- [x] Criar diretório e registrar `design.md` e `tasks.md` em `docs/work-units/active/audit-mcp-claws/`. <!-- id: 0 -->
- [x] Configurar entrada `[[bin]]` no `src-tauri/Cargo.toml` para `soda_mcp_tester_cli`. <!-- id: 1 -->

## 2. Construção do Harness `soda_mcp_tester_cli`
- [x] Implementar motor assíncrono em `src-tauri/src/bin/soda_mcp_tester_cli.rs`: <!-- id: 2 -->
  - Handshake inicial JSON-RPC 2.0 (`initialize` e `notifications/initialized`).
  - Descoberta e validação de `tools/list` (50 ferramentas).
  - Bateria de testes de estresse para as 50 ferramentas com cronometragem em microssegundos (`us`).
  - Verificação de isolamento de canal stdio (ADR-003: sem escapes ANSI ou logs).
  - Classificação clínica (`LIVE_PRODUCTION`, `STUB_MOCK`, `BROKEN_ERROR`).
  - Exportação formatada para `.souls_scratchpad/reports/mcp_claws_clinical_audit.md`.

## 3. Testes de Suporte e Resiliência
- [x] Adicionar testes de estresse de concorrência e integridade de router em `src-tauri/src/bin/souls_mcp_server/tests.rs`. <!-- id: 3 -->

## 4. Execução Forense & FinOps
- [x] Executar o harness no hardware real e gerar o laudo `.souls_scratchpad/reports/mcp_claws_clinical_audit.md`. <!-- id: 4 -->
- [x] Executar `cargo clippy` com saída direcionada a `.souls_scratchpad/logs/misc/clippy_mcp_audit.log` garantindo 0 warnings e Exit Code 0. <!-- id: 5 -->
