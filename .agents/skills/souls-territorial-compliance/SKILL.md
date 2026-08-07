---
name: souls-territorial-compliance
description: Garante que o agente respeite a topologia territorial do workspace SOULS conforme o _WORKSPACE_MAP.md v6.0 (Marco 3.10). Use no início de cada sessão e antes de criar qualquer arquivo.
---

# souls-territorial-compliance (Marco 3.10 — F7)

A workspace SOULS é organizada em **6 zonas** com paths canônicos fixos (ver
`_WORKSPACE_MAP.md` v6.0). Esta skill garante que você:

1. **Conheça as zonas antes de criar qualquer arquivo.**
2. **Consulte o auditor automatizado** antes de marcar tarefas como concluídas.
3. **Reportar violações** quando detectar (sem auto-cura silenciosa).

---

## 1. PROTOCOLO DE ABERTURA DE SESSÃO (OBRIGATÓRIO)

Antes de começar qualquer trabalho, **carregue o mapa territorial** lendo:

```bash
# 1. Ler o mapa canônico
cat _WORKSPACE_MAP.md

# 2. Rodar o auditor para confirmar baseline limpo
python docs/scripts/audit_workspace_compliance.py --quiet
```

Se a saída do auditor mostrar **0 findings** → workspace OK, prossiga.
Se mostrar findings → **NÃO PROSSIGA** até reportar ao Arquiteto Humano.

---

## 2. TABELA DE ZONAS (RESUMO OPERACIONAL)

| Zona | Onde Mora | Exemplos | Regra de Criação |
|------|-----------|----------|------------------|
| **ZONA 1** - Fábrica & Agente | `.agents/`, `.trae/`, `.antigravitycli/`, `.vscode/` | skills, rules, sidecars, tasks | Apenas via skills oficiais |
| **ZONA 2** - Estado & Cache | `.souls_data/`, `.souls_cache/`, `.souls_sandbox/`, `.souls_scratchpad/` | SQLite, LanceDB, logs, scripts efêmeros | Logs vão para `.souls_scratchpad/logs/<origem>/` |
| **ZONA 3** - Cânone | `docs/`, `.archive/` | ADRs, PRDs, DAGs, scripts de compilação, audits | Apenas paths canônicos da tabela §3 |
| **ZONA 4** - Backend Rust | `src-tauri/` | src/, third_party/, vendor/, semgrep/, resources/ | Tudo dentro de src-tauri/ |
| **ZONA 5** - Frontend Svelte | `src/` | components/, lib/, routes/, App.svelte, index.css | Sem lógica de negócios |
| **ZONA 6** - Raiz | arquivos pontuais | README, package.json, gateway-config.yaml, _WORKSPACE_MAP.md | NUNCA criar pasta nova aqui |

---

## 3. PATHS CANÔNICOS DA ZONA 3 (`docs/`)

| Subzona | Path | Quando Criar |
|---------|------|--------------|
| Work units ativas | `docs/work-units/active/<tipo>-<slug>/{design.md, tasks.md}` | Nova feature ou fix |
| Work units históricas | `docs/work-units/history/<marco>-<fase>/{design.md, tasks.md}` | Marco concluído |
| Templates | `docs/work-units/_templates/` | (NUNCA duplicar, usar o existente) |
| PRDs | `docs/planning/prds/<nome>.md` | Nova especificação |
| PRDs históricos | `docs/planning/prds/.archive/<nome>.md` | PRD finalizado |
| Roadmap | `docs/planning/roadmap/PRD-N.M-<nome>.md` | Roadmap de longo prazo |
| ADRs | `docs/decisions/adrs/ADR-NNN-<slug>.md` | Nova decisão arquitetural |
| Architecture | `docs/decisions/architecture/<doc>.md` | Manual macro de subsistema |
| Canibalization essence | `docs/decisions/architecture/canibalization_essence/<doc>.md` | Post-mortem de canibalização |
| Specs | `docs/decisions/specs/<doc>.md` | Especificação tabular/dicionário |
| Audits | `docs/observability/audits/{blobs,crates,mcp_inventory,quality}/` | Resultado de auditoria |
| State | `docs/observability/state/<doc>.md` | Monitoramento de estado |
| Reports | `docs/observability/reports/<doc>.md` | Relatório canônico |
| DAGs | `docs/runtime/dags/<DAG>.md` | Grafo de design/ingestão |
| Context dumps | `docs/runtime/context_dumps/_<NAME>.txt` | Snapshot compilado (gerado por script) |
| Scripts | `docs/runtime/scripts/<script>.{py,ps1}` | Compilador ou utilitário |
| Debugs | `docs/debugs/debug-<slug>.md` | Documento operacional ad-hoc |
| Manifesto | `docs/SOULS_CANON_MANIFEST.md` | (NÃO criar, referenciar o existente) |

---

## 4. SCRATCHPAD (ZONA 2) - 5 ZONAS EFÊMERAS

| Pasta | Quando Usar | Retenção |
|-------|-------------|----------|
| `.souls_scratchpad/logs/cargo/` | Logs de `cargo check`, `cargo test`, `cargo build` | 30 dias |
| `.souls_scratchpad/logs/git/` | Logs de `git ...`, `gh ...` | 30 dias |
| `.souls_scratchpad/logs/adr/` | Logs de geração de ADRs | 30 dias |
| `.souls_scratchpad/logs/marco/` | Logs de execução de marcos | 30 dias |
| `.souls_scratchpad/logs/misc/` | Logs sem origem clara | 30 dias |
| `.souls_scratchpad/commits/` | Mensagens de commit em rascunho (`commit_msg_*.md`) | até usar |
| `.souls_scratchpad/scripts/` | Scripts Python de inspeção efêmera | até usar |
| `.souls_scratchpad/reports/` | Outputs textuais da IA (`_PHASE_REPORT_*.txt`) | até usar |
| `.souls_scratchpad/.archive/` | Logs com > 30 dias | permanente (append-only) |

---

## 5. REGRAS FAIL-CLOSED

1. **PROIBIDO** criar pasta nova fora das zonas acima. Se precisar, **pergunte ao Arquiteto Humano** antes.
2. **PROIBIDO** criar arquivo na raiz do projeto (ZONA 6 só tem paths pré-aprovados).
3. **PROIBIDO** criar `docs/<subpasta>/` que não esteja na tabela §3.
4. **PROIBIDO** criar logs de cargo/git/adr/marco fora de `.souls_scratchpad/logs/<origem>/`.
5. **PROIBIDO** renomear zonas sem Marco formal (cada rename = ADR ou Marco 3.X).
6. **PROIBIDO** mover arquivos para `.archive/` "temporariamente" — esse diretório é append-only.

---

## 6. AUDITORIA AUTOMATIZADA

Sempre que terminar uma sequência de mudanças territoriais, rode:

```bash
python docs/scripts/audit_workspace_compliance.py
```

Saídas:
- **0 findings** → compliance OK.
- **WARN deprecated_ref** → ref a path antigo. Conserte com o fix_hint.
- **WARN non_canonical_path** → arquivo em zona não-canônica. Mova para zona correta.
- **ERROR missing_required_zone** → zona obrigatória ausente. CRÍTICO, reporte ao Arquiteto.
- **Exit 0** → OK ou WARN-only.
- **Exit 1** → ERROR presente (reporte obrigatório).
- **Exit 2** → erro de execução (workspace inválido).

Modos avançados:
- `--json` → saída JSON para parsing programático.
- `--quiet` → exit 0 mesmo com findings (para CI/relatórios).
- `--no-refs-scan` / `--no-paths-scan` / `--no-zones-check` → desabilitar varreduras específicas.

---

## 7. QUANDO ESTA SKILL DEVE SER INVOCADA

1. **Início de cada sessão de trabalho.** Carregar o mapa e rodar o auditor.
2. **Antes de criar QUALQUER arquivo** fora de zonas conhecidas. Consultar §3 e §4.
3. **Após mover/renomear arquivos** de zona. Rodar auditor para confirmar.
4. **Antes de fazer commit** de mudanças territoriais. Rodar auditor.
5. **Em auditoria periódica** (semanal ou após PR de reorg). Modo `--json` + diff.

---

## 8. REFERÊNCIAS

- **Mapa canônico:** `_WORKSPACE_MAP.md` (v6.0)
- **Auditor:** `docs/scripts/audit_workspace_compliance.py`
- **Manifesto:** `docs/SOULS_CANON_MANIFEST.md`
- **Marco:** Marco 3.10 — Reorganização Territorial do Workspace

> **Última revisão:** Marco 3.10 — `feat/branches-sync-script`
