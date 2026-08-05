---
spec: marco-4-1-3-tools-audit-e-cura-zero-brand
phase: 3-tasks
design: docs/work-units/active/marco-4-1-3-tools-audit/design.md
branch: TRAE-IDE
---

# Tasks — Marco 4.1.3: Audit & Cura Zero-Brand

## TASK-01 — CURA 1: Re-descrever 4 stubs (FALSO VERDE)

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs`

- [ ] `semantic_search` (linha 402): reescrever desc
- [ ] `execute` (linha 433): re-descrever com aviso de sandbox pendente
- [ ] `metrics` (linha 474): re-descrever como stub
- [ ] `intent` (linha 475): re-descrever como stub

**DoD:** `audit-tools-list.ps1` retorna 0 stubs.

## TASK-02 — CURA 2: Exterminação das duplicatas de impact

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs`

- [ ] Remover entrada `souls_impact` (linhas 690-702) do `tools/list`
- [ ] Remover entrada `ctx_impact` (linhas 703-715) do `tools/list`
- [ ] Manter `repo_impact` (linhas 677-689) como **única entrada canônica**
- [ ] Aliases no dispatcher (linha 848) permanecem: `"repo_impact" | "souls_impact" | "ctx_impact" => run_repo_impact(params).await,`

**DoD:** `audit-tools-list.ps1` retorna 0 tools com prefixo proibido.

## TASK-03 — CURA 3: Higienizar 6 descrições com "Cânone SOULS"

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs`

- [ ] `get_ast` (linha 148): remover "(Cânone SOULS, ex-repo_ast)"
- [ ] `fetch_web` (linha 163): remover "(Cânone SOULS, ex-web_fetch)"
- [ ] `sys_time` (linha 178): remover menção SOULS
- [ ] `web_search` (linha 187): remover menção SOULS
- [ ] `repo_meta` (linha 208): remover menção SOULS
- [ ] `sqlite_query` (linha 223): remover menção SOULS

**DoD:** `audit-tools-list.ps1` retorna 0 brand violations.

## TASK-04 — Expandir `tools_list_returns_unprefixed_names`

**Arquivo:** `src-tauri/src/bin/souls_mcp_server.rs` (test module, linha 4598)

- [ ] Adicionar asserções para `souls_impact` e `ctx_impact` ao teste existente
- [ ] Garantir que o teste continua passando

**DoD:** Teste verde + cerca perimetrica atualizada.

## TASK-05 — Validacao Master

- [ ] `cargo test --bin souls_mcp_server` → 41 verdes
- [ ] `cargo test --workspace` → 601 verdes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- [ ] `pwsh audit-tools-list.ps1` → 0 issues

**DoD:** 0 issues em todas as dimensoes (ADR-026, ADR-037, ADR-041).

## TASK-06 — Blast Radius Report

- [ ] `git status --short` capturado
- [ ] Apenas 1 arquivo editado: `souls_mcp_server.rs`
- [ ] Apenas arquivos novos em `docs/work-units/active/marco-4-1-3-tools-audit/`
