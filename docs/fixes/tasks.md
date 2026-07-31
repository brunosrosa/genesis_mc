---
spec: rebrand-souls-mc
phase: 3-tasks
design: docs/design.md
branch: feat/rebrand-souls-mc
---

# Tasks — Rebrand Souls MC

Cada task tem um DoD (Definition of Done) executável. Tarefas marcadas `[SCAFFOLD]` exigem teste vazio de falha antes da lógica real (Lei do Scaffold). Para um rebrand textual, o "teste" é o `git grep` que deve retornar zero após a mutação.

## TASK-01 — Worker A: Rust Code (3 files)

**Escopo:** Substituir `genesis_mc` por `souls_mc` em código Rust ativo.

- [ ] `src-tauri/src/bin/f1_distiller_cli.rs:380` — `SOULS (Genesis MC)` → `SOULS (Souls MC)`
- [ ] `src-tauri/src/harvester/canon.rs:17` — `SOULS / Genesis MC:` → `SOULS / Souls MC:`
- [ ] `src-tauri/src/persist/ssot_injector.rs:1337` — `genesis-mc-sheets/1.0` → `souls-mc-sheets/1.0`

**DoD:**
- `git grep -E 'genesis[ _-]?mc' -- 'src-tauri/src/'` retorna 0
- `cargo check --manifest-path src-tauri/Cargo.toml` retorna Exit Code 0
- Nenhuma warning de compilação introduzida

## TASK-02 — Worker B: Config Files (2 files)

**Escopo:** Substituir paths em configs ativas.

- [ ] `src-tauri/semgrep/rules/_manifest.json:3` — `Z:\\genesis_mc` → `Z:\\souls_mc`
- [ ] `gateway-config.yaml:41` — `Z:/genesis_mc/.souls_data/souls_state.db` → `Z:/souls_mc/.souls_data/souls_state.db`

**DoD:**
- `git grep -E 'genesis[ _-]?mc' -- 'src-tauri/semgrep/rules/_manifest.json' 'gateway-config.yaml'` retorna 0
- JSON e YAML continuam parseáveis (sem corromper indentação/escape)

## TASK-03 — Worker C: Workspace Meta (4 files)

**Escopo:** Substituir referências ativas em metadados do workspace.

- [ ] `.trae/rules/project_rules.md:5` — `Genesis MC Core Context` → `Souls MC Core Context`
- [ ] `README.md:18` — `O Genesis MC repudia` → `O Souls MC repudia`
- [ ] `README.md:62` — `rodando no Genesis MC` → `rodando no Souls MC`
- [ ] `.agents/skills/souls-frontend-expert/SKILL.md` — `SOULS (Genesis MC)` → `SOULS (Souls MC)`

**DoD:**
- `git grep -E 'genesis[ _-]?mc' -- '.trae/' 'README.md' '.agents/skills/'` retorna 0
- Frontmatter YAML dos skills permanece íntegro
- Markdown headings (H1/H2/H3) preservados

## TASK-04 — Worker D: Docs Ativos — Specs (5 files, 30 hits)

**Escopo:** Atualizar link paths em specs ativas que apontam para o código atual.

- [ ] `docs/specs/spec-037-blob08-json-schema-v2.md` — 3 hits de `file:///Z:/genesis_mc/...`
- [ ] `docs/specs/spec-038-opengrep-rule-diversificacao.md` — 2 hits
- [ ] `docs/specs/spec-039-trailbase-como-canario-de-stress.md` — 10 hits
- [ ] `docs/specs/spec-040-auditoria-qualitativa-blos.md` — 6 hits
- [ ] `docs/specs/spec-041-diagnostico-de-placeholders-sistemicos.md` — 6 hits

**DoD:**
- `git grep -E 'genesis[ _-]?mc' -- 'docs/specs/'` retorna 0
- Apenas o segmento `genesis_mc` é substituído; âncoras `#L24-L55` etc. preservadas
- Código inline (rust code blocks) e listas markdown permanecem íntegros

## TASK-05 — Worker D': Docs Ativos — PRD (1 file, 3 hits)

**Escopo:** Atualizar link paths no PRD ativo.

- [ ] `docs/prds/PRD_REFAC_01_StateMachine.md:39,40,41` — `file:///c:/Users/rosas/Dev_Projects/genesis_mc/...` → `file:///c:/Users/rosas/Dev_Projects/souls_mc/...`
  - NOTA: o drive letter `c:` e o path `Users/rosas/Dev_Projects/` NÃO são tocados — fora do escopo do rebrand. Apenas o segmento do nome do projeto.

**DoD:**
- `git grep -E 'genesis[ _-]?mc' -- 'docs/prds/'` retorna 0
- Estrutura do PRD (YAML frontmatter, headings, listas) preservada

## TASK-06 — Worker E: ETL Python — Paths (3 files, 19 hits)

**Escopo:** Atualizar paths em scripts Python de ETL (dev-only) para que o próximo run produza dumps alinhados com o novo nome.

- [ ] `docs/scripts/extract_audit_blobs.py:33` — `parents[2]=genesis_mc` → `parents[2]=souls_mc`
- [ ] `docs/scripts/souls_adr_compiler.py:6,7` — `Z:\genesis_mc\...` → `Z:\souls_mc\...` (2 hits)
- [ ] `docs/scripts/souls_context_dumps_compiler.py` — 15 hits `Z:\genesis_mc\...` → `Z:\souls_mc\...`

**DoD:**
- `git grep -E 'genesis[ _-]?mc' -- 'docs/scripts/extract_audit_blobs.py' 'docs/scripts/souls_adr_compiler.py' 'docs/scripts/souls_context_dumps_compiler.py'` retorna 0
- A lista `REBRAND_FORBIDDEN` em `docs/scripts/audit_blob_quality.py:106-110` permanece INTACTA (assinatura do auditor)

## TASK-07 — PRESERVE: Históricos (16+ files)

**Escopo:** NÃO MEXER. Confirmar byte-identidade.

- [ ] `docs/audits/blobs/_AUDIT_blob_06_hotspots.txt`
- [ ] `docs/audits/blobs/_AUDIT_blob_08_health.txt`
- [ ] `docs/audits/crates/_CARGO_TOML_STATE.txt`
- [ ] `docs/audits/crates/_DUPLICATE_DEPS.txt`
- [ ] `docs/audits/mcp_inventory/audit_transition_zero_brand.md`
- [ ] `docs/audits/quality/_QUALITY_SCORES.md`
- [ ] `docs/context_dumps/_ADRs_ALL.txt`
- [ ] `docs/context_dumps/_ENV_CLEAN.txt`
- [ ] `docs/context_dumps/_IGNITION_SCRIPTS.txt`
- [ ] `docs/context_dumps/_MCP_INVENTORY.txt`
- [ ] `docs/context_dumps/_RULES_IN_IDEs.txt`
- [ ] `docs/context_dumps/_SKILLS_IN_IDEs.txt`
- [ ] `docs/context_dumps/_WOKSPACE_MAP.txt`
- [ ] `docs/context_dumps/_YAML_AgentGateway_e_souls_mcp_server.rs.txt`
- [ ] `docs/state/DB_STATE_REPORT.md`
- [ ] `docs/state/_CURRENT_REALITY_AUDIT_2026-07-05.md`
- [ ] `docs/state/debugs/debug-rust-incremental-noise.md`

**DoD:**
- `git status` mostra esses arquivos como `unmodified` (ou seja, não foram tocados)
- `git diff --stat` sobre esses paths é vazio

## TASK-08 — PRESERVE: Auditor (1 file, lista REBRAND_FORBIDDEN)

**Escopo:** NÃO MEXER. A lista `REBRAND_FORBIDDEN` é a *assinatura* do auditor de rebrand; alterá-la cegaria o auditor.

- [ ] `docs/scripts/audit_blob_quality.py:106-110` (lista `REBRAND_FORBIDDEN`)

**DoD:**
- `git diff -- 'docs/scripts/audit_blob_quality.py'` é vazio (ou só mostra mudanças em outros lugares do arquivo, se houver)

## TASK-09 — Validation: cargo check + cargo test

**Escopo:** Provar que o silício assimilou o rebrand.

- [ ] `cd src-tauri && cargo check --all-targets` → Exit Code 0, 0 warnings
- [ ] `cd src-tauri && cargo test --no-run` → Exit Code 0
- [ ] Se `cargo check` falhar com "crate `souls_mc_lib` not found": rastrear import residual e corrigir (não deve acontecer, mas contingência)
- [ ] Se falhar por qualquer outro motivo: invocar `souls-ralph-loop` (3-tentativas ceiling, Fail-Closed)

**DoD:**
- `cargo check` retorna `Exit Code 0` com zero warnings
- `cargo test` retorna `Exit Code 0` (testes existentes permanecem verdes)
- Nenhum teste novo introduzido (rebrand não muda comportamento)

## TASK-10 — Blast Radius Report + HITL

**Escopo:** Compilar diff stats e enviar para aprovação humana antes do rebase semântico.

- [ ] `git diff --stat` capturado
- [ ] `git diff --stat` filtrado por categoria (code / config / meta / docs ativos / preservados)
- [ ] Mensagem de HITL gerada com: branch, número de arquivos tocados, contagem de linhas, lista de paths preservados
- [ ] NÃO fazer merge. NÃO criar merge commit.
- [ ] Aguardar aprovação do Arquiteto para `git rebase` semântico em direção a `main`

**DoD:**
- Mensagem HITL enviada (via `NotifyUser` ou equivalente)
- Nenhuma operação destrutiva no git sem aprovação
