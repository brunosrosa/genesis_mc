---
spec: marco-4-1-2-decoupling-fabrica-produto
phase: 3-tasks
design: docs/work-units/active/marco-4-1-2-decoupling/design.md
branch: TRAE-IDE
---

# Tasks — Marco 4.1.2: Desacoplamento Fábrica/Produto

Cada task tem DoD (Definition of Done) executável. **Lei do Scaffold:** a build 1.5 deve falhar-fechado se `cargo build` retornar != 0 (R1 da Linha Vermelha). O sleep de 1s é não-negociável (R2/NTFS).

## TASK-01 — Edit `boot.ps1`: inserir etapa 1.5/5 (Transplante Físico)

**Arquivo:** `boot.ps1` (EDIT — inserção após a linha 167)

**Escopo:** Logo após a confirmação "Supervisores antigos encerrados e portas locais liberadas." (final do step 1 de expurgo de zumbis), inserir uma nova seção rotulada `[1.5/5]` que:

- [ ] Cria `.agents/bin/` na raiz do projeto (idempotente via `New-Item -ItemType Directory -Force`).
- [ ] Executa `cargo build` focado nos 3 daemons com as mesmas features do step 4 (`tauri-app,gateway_ccr,llama_backend`).
- [ ] Se exit != 0, dispara `throw` ou `exit 1` (Fail-Closed, R1).
- [ ] `Start-Sleep -Seconds 1` após o build (liberação de handles NTFS, R2).
- [ ] `Copy-Item -Force` para os 3 `.exe` de `target/debug/` → `.agents/bin/`.
- [ ] Log `[TRANSPLANTE]` indicando sucesso.

**DoD:**
- Script compila (PowerShell syntax check via `pwsh -NoProfile -Command`).
- Em codigo quebrado: `cargo build` falha → script `exit 1`.
- Em codigo OK: 3 .exe aparecem em `.agents/bin/`.

## TASK-02 — Edit `gateway-config.yaml`: apontar para `.agents/bin/`

**Arquivo:** `gateway-config.yaml` (EDIT — linha 18)

**Escopo:** Modificar a propriedade `cmd` do backend `souls_mcp`.

- [ ] Mudar de `cmd: 'Z:/souls_mc/src-tauri/target/debug/souls_mcp_server.exe'`
- [ ] Para `cmd: 'Z:/souls_mc/.agents/bin/souls_mcp_server.exe'`

**DoD:**
- Arquivo YAML ainda válido (`yaml-language-server` schema).
- Path absoluto Z:/ com forward slashes (consistente com o resto do arquivo).

## TASK-03 — Validar boot end-to-end + concorrência contra NTFS

**Escopo:** Provar o desacoplamento físico.

- [ ] Rodar `./boot.ps1` em terminal.
- [ ] Validar que os 3 .exe foram transplantados para `.agents/bin/`.
- [ ] **Prova de fogo concorrente** (com gateway rodando em background):
  - [ ] `cargo check --workspace` → Exit 0
  - [ ] `cargo test --test test_souls_symbol` → 3 verdes
  - [ ] `cargo test --test test_repo_impact` → 3 verdes

**DoD:**
- 0 sharing violations.
- 0 linker errors.
- 0 regressões nos 601 testes do Marco 4.1.1.

## TASK-04 — Blast Radius Report

- [ ] `git status --short` capturado.
- [ ] Apenas 2 arquivos modificados: `boot.ps1` e `gateway-config.yaml`.
- [ ] Nenhum `.exe` commitado (binários em `.gitignore`).
- [ ] Aguardar aprovação do Arquiteto para o Rebase Semântico.
