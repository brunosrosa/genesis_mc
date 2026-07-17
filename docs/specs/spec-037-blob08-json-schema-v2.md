---
id: "spec-037"
title: "spec-037-blob08-json-schema-v2"
version: 0.3
status: Draft_Aguardando_Audit
owner: soda-rust-engine
adr_refs: ["ADR-031", "ADR-019"]
depends_on: ["spec-040"]
gates: []
created: "2026-07-16"
target_release: "Souls MC V6.1"
---

# Spec-037: `blob_08_health_report` — Schema v2 JSON Estruturado

## PRDs Parcialmente Implementados (v0.3)

Esta spec continua `Draft_Aguardando_Audit`, mas **2 PRDs de sua tese foram JÁ implementados e testados em TDD** no ciclo atual (Dumb-LLM Test aprovado):

- **PRD-033 (`deduplicate_forensic_diagnostics`)** — em [src-tauri/src/harvester/sast/mod.rs](file:///Z:/souls_mc/src-tauri/src/harvester/sast/mod.rs). Colapsa 32+ diagnósticos idênticos do libsqlite3-sys (mesmo erro em N sub-crates de um monorepo) em 1 entrada canônica. Reduz o `blob_08` de trailbase de ~245KB para < 60KB sem perda de sinal. **4 testes TDD verdes** (collapses_identical_failures, handles_empty_input, handles_malformed_entries, keeps_distinct_failures).
- **PRD-042 (`render_semgrep_header`)** — em [src-tauri/src/harvester/sast/opengrep.rs](file:///Z:/souls_mc/src-tauri/src/harvester/sast/opengrep.rs). Adiciona header de auditoria canônico ao topo do `blob_06` E `blob_08`:
  ```
  [AUDITED_AT]: 2026-07-16T...
  [OPENGREP_VERSION]: 1.45.0 | unknown
  [FILES_SCANNED]: N
  [FINDINGS_COUNT]: N
  [SEVERITY_BREAKDOWN]: ERROR=2, WARNING=5
  ```
  Isso é o **mínimo denominador comum** entre os formatos "texto-relatório" (atual) e "JSON estruturado" (proposto nesta spec). **4 testes TDD verdes** + 2 long_tail preservados.

**Implicação para o gating decision:** a premissa "blob_08 é texto-relatório massivamente inflado" foi **parcialmente refutada**. O bloat por duplicação foi resolvido (PRD-033). O header canônico (PRD-042) já entrega metade do valor prometido pelo JSON v2 sem mudar o formato. A urgência do JSON schema v2 diminuiu — pode ser adiada para v7 sem perder a auditoria.

## Gating Decision (adicionado em v0.2)

Esta spec **NÃO deve ser implementada antes de `spec-040` (Auditoria Qualitativa) rodar** e confirmar que `blob_08_health_report` tem score médio < 60 nos 150+ runs históricos. Se o audit mostrar score ≥ 60, esta spec perde motivação e deve ser pausada ou reescrita.

**Status atual:** `Draft_Aguardando_Audit`. Será promovido a `Draft` (executável) após a decisão do gating, ou a `Pausado` se o audit revelar que o problema não é sistêmico.

## Contexto
O `blob_08_health_report` é definido pelo [ADR-031 §4](file:///Z:/souls_mc/docs/adrs/ADR-031-Harvester-Anatomia-11-Blobs-e-Leis-Inegociaveis.md) como "A Podridão Estrutural": o mesmo motor SAST do `blob_06`, mas com a flag `--skip-formatter`, focado em complexidade ciclomática, código morto e code smells.

A auditoria de validação do Spec-036 (rebrand) revelou que o `blob_08` do trailbase (429.781 bytes) é um **texto-relatório com markers `[DOMAIN:...]`** e não JSON estruturado:

- Header: `[DIAGNÓSTICO ESTRUTURAL RUST: FALHA FATAL DE COMPILAÇÃO OU RCE BLOQUEADO]`
- 1.184 findings no formato: `- [warning] [opengrep] src/records/read_record.rs :: L191: <rule_id> (WARNING / general-debt) -> <message>`
- 1 govulncheck info: `- [info] [govulncheck] [INFO] Nenhuma vulnerabilidade...`

Consequência prática: o script `docs/scripts/extract_audit_blobs.py --pretty-json` é **no-op** neste blob. A Lente C (Realidade/Operação) das Fases 2-3 não consegue fazer queries estruturadas (ex: "todos os findings com severidade error e source=opengrep"). Cada consumo é parsing ad-hoc regex.

## Leis Aplicáveis
- **ADR-031 Lei IV (Zero-Byte Uniforme):** o schema v2 não pode mascarar falhas. Se o parsing v2 falhar, o payload deve ser gravado como 0 bytes e a flag `ERRO_F0` deve ser acionada (cross-ref ADR-019).
- **ADR-031 §4 Blob 8 (Podridão Estrutural):** a lâmina tática é o motor SAST com `--skip-formatter`. A mudança é apenas no **formato de saída**, não na lâmina.

## Objetivos
1. Definir um schema v2 JSON estruturado para o `blob_08_health_report` que:
   - Preserve o conteúdo semântico (todos os findings continuam presentes).
   - Permita queries estruturadas pelas Lentes B e C.
   - Permita retro-compatibilidade com o schema v1 (texto-relatório) durante a migração.
2. Implementar um parser Python de validação que rejeite payloads v2 malformados (fail-closed).
3. Atualizar `docs/scripts/extract_audit_blobs.py` para reconhecer e exibir o schema v2 corretamente.

## Não-Objetivos
- Mudar a lâmina SAST (continua sendo OpenGrep + Clippy + Biome com `--skip-formatter`).
- Mudar o schema do `blob_06_unsafe_hotspots` (tem taxonomia similar, mas é spec separado).
- Adicionar campos derivados de LLM. O schema v2 é puramente determinístico.
- Internacionalização. Campos e mensagens permanecem em inglês (canonical para tooling).

## Definição de Pronto (DoD)

### Pré-condições (TDD Red)
- [ ] `tests/fixtures/blob_08_v1_sample.txt` (atual texto-relatório do trailbase) preservado como fixture de retro-compat.
- [ ] `tests/fixtures/blob_08_v2_sample.json` criado a partir do sample v1 via conversor de teste.
- [ ] `tests/test_blob_08_schema_v2.py` com **3 testes vermelhos**:
  - `test_v2_valid_payload_parses()` — schema v2 válido é aceito.
  - `test_v2_malformed_payload_rejected()` — schema v2 malformado é rejeitado com erro descritivo.
  - `test_v1_text_payload_preserved()` — payload v1 (texto) continua legível como string, com flag `schema_version: "v1"`.

### Implementação (TDD Green)
- [ ] Módulo Rust `src-tauri/src/harvester/blob_08_schema.rs` com:
  - `pub enum Blob08Schema { V1, V2 }` detectado por heurística (primeiro byte `{` → V2; `[` ou letra → V1).
  - `pub struct Blob08V2 { schema_version: String, domains: Vec<Domain>, findings: Vec<Finding>, summary: Summary }`.
  - Serialize/deserialize via `serde` + `serde_json`.
- [ ] Conversor `v1_text_to_v2_json()` que parseia o texto-relatório atual e emite JSON estruturado:
  - Detecta seções `[DOMAIN: <name>]` → array `domains[].name`.
  - Detecta linhas `- [sev] [source] file :: L<line>: rule_id (CATEGORY) -> msg` → array `findings[]`.
  - Detecta marcadores `FALHA FATAL DE COMPILAÇÃO OU RCE BLOQUEADO` → campo `summary.fatal_compilation_blocked: bool`.
  - Detecta govulncheck info → array `findings[]` com `source: "govulncheck"`.
- [ ] Adaptação do `extract_audit_blobs.py`:
  - Flag `--schema-version {auto,v1,v2}` (default `auto`).
  - Quando `v2` detectado, `--pretty-json` aplica `json.dumps(indent=2)` real.
  - Quando `v1` detectado, comportamento atual (texto puro) preservado.
- [ ] Campo `schema_version` no payload v2 com valores `"v1"` ou `"v2"` para que o consumidor saiba o que esperar.

### Validação (Refactor + Verify)
- [ ] Rodar F0 Harvester sobre `trailbaseio/trailbase` com a nova lâmina. Confirmar que:
  - `blob_08_health_report` é gravado como v2.
  - `extract_audit_blobs.py --summary --pretty-json` produz JSON estruturado parseável.
  - Número de findings antes e depois é **exatamente igual** (sem perda).
- [ ] `cargo check` + `cargo clippy -- -D warnings` exit 0.
- [ ] `pytest tests/test_blob_08_schema_v2.py` exit 0 (3 testes verdes).

## Critérios de Aceite Mensuráveis
1. **Cobertura 100%:** 1.184 findings no trailbase são preservados integralmente no v2 (assert via `len(findings) == 1184`).
2. **Zero perda de metadados:** cada finding no v2 tem `severity`, `source`, `file`, `line`, `rule_id`, `category`, `message` (todos os campos do texto v1).
3. **Retro-compat:** fixture v1 (trailbase original) carrega via parser com `schema_version: "v1"` e exibe conteúdo textual idêntico.
4. **Performance:** conversão v1→v2 sobre o trailbase completa em < 2s (single-thread, release build).
5. **Fail-closed:** payload v2 com campo obrigatório faltando é rejeitado pelo parser com mensagem citando o campo ausente (não silenciosamente truncado).

## Fora de Escopo
- Mudar o schema de **qualquer outro blob** (1-7, 9, 10, 11). O spec-037 cobre **apenas** o blob_08.
- Adicionar suporte a novas fontes SAST (Govulncheck já é o único além de OpenGrep atualmente).
- Internacionalizar mensagens (i18n).
- Compressão do payload (deixar para spec futuro se vier a ser necessário).

## Riscos & Mitigações
| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| Conversor v1→v2 perde findings por regex frágil | Média | Alto | TDD: `len(findings)` no v2 == `len(findings)` no v1, validação via contagem dupla |
| Schema v2 malformado gravado no SQLite | Baixa | Alto | Validador `serde_json::from_str` no caminho de escrita; falhar = 0 bytes + `ERRO_F0` |
| Retro-compat quebrada (consumidor v1 não lê v2) | Média | Médio | Campo `schema_version` explícito; consumidores verificam antes de parsear |
| Overhead de conversão em F0 | Baixa | Baixo | Conversão é single-pass O(N) sobre texto já extraído; benchmark < 2s |

## Rollback
Se o schema v2 falhar em produção:
1. Reverter a detecção `Blob08Schema::detect()` para sempre retornar `V1`.
2. Manter o conversor `v1_text_to_v2_json()` no código (não deletar) para próximo retry.
3. Limpar o SQLite dos payloads v2 malformados: `DELETE FROM artefatos_brutos WHERE artifact_type = 'blob_08_health_report' AND json_valid(payload_blob) = 0`.
4. Documentar o incidente no `project_memory.md` (L2) e abrir spec para o retry.

## Sequência de Execução
1. **Fase 1 — TDD Red:** fixtures + 3 testes vermelhos.
2. **Fase 2 — TDD Green:** implementar módulo Rust + conversor + adaptação do script Python.
3. **Fase 3 — Refactor:** cleanup de código duplicado, adicionar tracing/logs estruturados.
4. **Fase 4 — Validação:** rodar F0 sobre trailbase, confirmar 1.184 findings preservados, `cargo check` + `pytest` exit 0.
5. **Fase 5 — HITL:** apresentar blast radius (arquivos tocados) ao Arquiteto Humano, aguardar aprovação, fazer rebase semântico.
