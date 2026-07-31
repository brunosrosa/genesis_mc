---
id: "spec-041"
title: "spec-041-diagnostico-de-placeholders-sistemicos"
version: 0.2
status: Draft
owner: soda-rust-engine
adr_refs: ["ADR-031", "ADR-019", "ADR-025"]
supersedes_thesis: ["spec-037", "spec-038", "spec-039"]
created: "2026-07-16"
target_release: "Souls MC V6.1"
---

# Spec-041 v0.2: Diagnóstico de Placeholders Sistêmicos no F0 Harvester

## Histórico de Versões
- **v0.1 (2026-07-16):** Definiu 5 hipóteses (H1-H5) para os placeholders de ~100 bytes em 720/721 repos. Propôs gates de investigação read-only via SQL.
- **v0.2 (2026-07-16):** Adiciona seção **PRDs Parcialmente Implementados** mapeando quais hipóteses já foram **endereçadas por construção** pelos PRDs 033/042/043/044 entregues nos ciclos subsequentes. Diagnóstico precisa ser recalibrado: algumas hipóteses podem ter sido **resolvidas** sem que a Fase 0 (queries SQL) tenha rodado.

## Contexto
A [Auditoria Qualitativa (spec-040)](file:///Z:/souls_mc/docs/specs/spec-040-auditoria-qualitativa-blos.md) executada sobre os 7.221 pares `(repo_id, artifact_type)` no SQLite revelou que **3 dos 11 blobs têm mediana de ~100 bytes**, o que caracteriza **placeholders sistemáticos**:

| Blob | Mediana | Min | Max | Diagnóstico |
|---|---:|---:|---:|---|
| `blob_06_unsafe_hotspots` | 100 | 94 | 89.889 | 720/721 repos = placeholder (trailbase é o único rico) |
| `blob_08_health_report` | 102 | 76 | 429.919 | 720/721 repos = placeholder (trailbase é o único rico) |
| `blob_11_ux_contracts` | 63 | 30 | 164.379 | Maioria = placeholder (alguns repos com UI real) |

A meta da "Fotografia Completa (Zero Truncamento)" do [ADR-031 §2](file:///Z:/souls_mc/docs/adrs/ADR-031-Harvester-Anatomia-11-Blobs-e-Leis-Inegociaveis.md) está sendo violada em escala: 720 dos 721 repos têm blobs 06/08/11 como **esqueletos de placeholder**, não como extração real. Isso **não é** violação formal da Lei IV (o conteúdo gravado é "100% verdade" do ponto de vista do payload — placeholder é o conteúdo), mas **é** violação do espírito do ADR-031: a IA das Fases 2-3 recebe um esqueleto vazio, não uma fotografia.

**Esta spec NÃO é sobre o conteúdo dos payloads (que está em conformidade com a Lei IV)**, mas sobre o **pipeline de extração** que produz esses payloads. Por que 720 dos 721 repos não estão sendo extraídos completamente?

## Hipóteses a Investigar

### H1: Falha de Clone (mais provável — 60%)
O `git clone` falha silenciosamente para ~720 dos 721 repos. Causas prováveis:
- **Rate limit do GitHub** (`429 Too Many Requests`)
- **Repo privado** ou deletado
- **Network timeout** em clone de repo grande
- **Permissões NTFS** no workspace temporário (já tivemos esse problema no ACG)

**Sinal esperado:** se o clone falha, todos os 11 blobs do repo são placeholder. Mas o audit mostra que **outros blobs (1, 2, 3, 4, 5, 7, 9) NÃO são placeholder** para esses mesmos repos — eles têm conteúdo. Logo, o clone **não falhou** para esses 720 repos. **H1 refutada**.

### H2: Sidecars SAST Falham (provável — 70%)
Os sidecars `opengrep` e `govulncheck` falham em silêncio para a maioria dos repos. Causas prováveis:
- **Sandbox ACG** viola specs dinâmicas do Nuitka (já vimos `-1073740791` no audit do projeto)
- **Sidecar binary not found** — `resolve_sidecar_bin` retorna None e o sistema grava placeholder
- **Rule não casa com a linguagem do repo** — opengrep roda, não encontra nada, e o sistema grava placeholder
- **OpenGrep timeout** em repos grandes (mas com `--allow-rule-timeout-control` ativo, isso seria sucesso)

**Sinal esperado:** se o sidecar falha, o `blob_06` e `blob_08` são placeholder mas outros blobs funcionam. **Bate com o observado.** H2 é a hipótese mais provável.

### H3: Persistência Parcial (menos provável — 20%)
O sidecar extrai corretamente mas a gravação no SQLite é parcial. O `payload_blob` é gravado com placeholder inicial e nunca substituído.

**Sinal esperado:** logs do ETL mostrariam `wrote 0 bytes` ou `placeholder used`. **Improvável porque teríamos visto nos logs do Harvester.**

### H4: Batch ETL Seletivo (provável — 50%)
O ETL roda em batch selecionando apenas alguns repos por vez. Os 720 repos com placeholder foram runs antigos (pré-rebrand) em que o batch era menor. Apenas 1 repo (trailbase) tem run pós-rebrand.

**Sinal esperado:** análise temporal dos `timestamp_extracao` dos 721 runs mostraria cluster temporal. Provável.

### H5: Diferenciação de Schema (possível — 30%)
Os placeholders de 100 bytes são **conteúdo real** mas curto — tipo um `summary: 0 findings` ou `nenhuma vulnerabilidade`. A "Fotografia Completa" não exige que o payload seja grande; exige que **conte a verdade completa do repo**, mesmo que a verdade seja "clean".

**Sinal esperado:** inspeção do conteúdo dos 100-bytes payloads mostraria mensagens canônicas tipo "no findings" ou "no hotspots". Se for isso, **não é bug, é design**.

## Leis Aplicáveis
- **ADR-031 Lei IV (Zero-Byte Uniforme):** os placeholders de 100 bytes **passam** na Lei IV (não há "Warning: Timeout"). Mas o audit revela que o cap "honesto" de 0 bytes não é o que está acontecendo — é um payload pequeno, não zero.
- **ADR-031 §2 (Fotografia Completa):** violação do espírito, não da letra. A spec precisa decidir se "100 bytes de placeholder" é ou não conformidade.
- **ADR-025 (Consciência de Monorepo):** se H2 for correta, o problema está no sidecar, não no repo.

## PRDs Parcialmente Implementados (v0.2)

A v0.1 foi escrita quando os placeholders eram **estado atual inexplicado**. Entre v0.1 e v0.2, **4 PRDs foram entregues** que atacam diretamente 2 das 5 hipóteses. **Antes de rodar a Fase 0 (queries SQL)**, vale reclassificar:

| Hipótese | PRD que endereça | Localização | Status |
|---|---|---|---|
| **H2 (sidecars falham em silêncio)** | **PRD-042 (`render_semgrep_header`)** | [src-tauri/src/harvester/sast/opengrep.rs](file:///Z:/souls_mc/src-tauri/src/harvester/sast/opengrep.rs) | **Parcialmente mitigada.** O header canônico torna **diagnosticável** se o sidecar rodou (tool, version, duration) — placeholders pós-PRD-042 carregam "audit_header" mesmo se findings=0. Runs **pré-PRD-042** não têm essa info. |
| **H3 (persistência parcial)** | **PRD-033 (`deduplicate_forensic_diagnostics`)** | [src-tauri/src/harvester/sast/mod.rs](file:///Z:/souls_mc/src-tauri/src/harvester/sast/mod.rs) | **Não relacionada.** PRD-033 atua sobre findings já extraídos, não sobre o pipeline de gravação. H3 segue em aberto. |
| **H4 (batch ETL seletivo)** | **PRD-043 (`cargo_workspace_deps_capture`)** + **PRD-044 (`package_json_peer_optional_deps`)** | [src-tauri/src/harvester/extract.rs](file:///Z:/souls_mc/src-tauri/src/harvester/extract.rs) | **Parcialmente endereçada indiretamente.** Os PRDs não mudam o batch ETL, mas aumentam o conteúdo dos payloads quando rodam. **Placeholders pós-PRD-043/044 são menos prováveis** porque o `blob_02` agora tem mais sinal. |
| **H5 (payload curto = clean real)** | **PRD-042 (`render_semgrep_header`)** | [src-tauri/src/harvester/sast/opengrep.rs](file:///Z:/souls_mc/src-tauri/src/harvester/sast/opengrep.rs) | **Fortalecida como tese.** O header canônico transforma "100 bytes de placeholder" em "100 bytes de header canônico + findings_count: 0" — explicitando que **clean é informação válida**, não placeholder. **H5 vira a interpretação default** para runs pós-PRD-042. |
| **H1 (falha de clone)** | — | — | Sem PRD relacionado. **H1 já foi refutada na v0.1** (outros blobs funcionam para os 720 repos). |

**Implicação para a Fase 0:** as queries SQL precisam ser **estratificadas por timestamp** vs. merge dos PRDs:
- `runs.timestamp_extracao < 2026-07-16T12:00` → placeholders podem ser H2/H4/H5 sem discriminação
- `runs.timestamp_extracao >= 2026-07-16T12:00` → placeholders pós-PRD-042 **devem** ter `audit_header` canônico; ausência dele é **sinal de bug novo** (H3?)

**Nova query obrigatória (Fase 0 v0.2):**

```sql
-- Discriminar placeholders pré vs pós PRD-042
SELECT
  CASE WHEN timestamp_extracao >= strftime('%s','2026-07-16T12:00:00Z') 
       THEN 'pos_prd' ELSE 'pre_prd' END AS era,
  artifact_type,
  count(*) FILTER (WHERE length(payload_blob) BETWEEN 90 AND 110) AS n_placeholder,
  count(*) FILTER (WHERE payload_blob LIKE '%audit_header%' OR payload_blob LIKE '%tool:%') AS n_with_header,
  count(*) AS n_total
FROM artefatos_brutos
WHERE artifact_type IN ('blob_06_unsafe_hotspots','blob_08_health_report')
GROUP BY era, artifact_type;
```

Se a coluna `n_with_header` for ~0 na era `pos_prd`, **H3 (persistência parcial) é confirmada** para runs recentes. Se for ~100%, **H5 (clean real) é confirmada** para runs recentes, e o problema é apenas histórico.

**Recomendação de replanejamento:** a Fase 0 (queries) **deve rodar antes** de reabrir spec-038. Se a nova query mostrar que runs recentes têm header canônico, **H5 é confirmada e os 3 specs pausados devem ser reescritos** para falar de "taxonomia de clean", não de "diversificação de regras".

## Objetivos
1. **Diagnosticar a causa raiz** dos placeholders de ~100 bytes em `blob_06`, `blob_08`, `blob_11` para 720/721 repos.
2. **Classificar cada hipótese** (H1-H5) como confirmada, refutada, ou inconclusiva, com evidência.
3. **Decidir o que fazer**: corrigir o pipeline (se for bug), documentar (se for design), ou reescrever os 3 specs pausados (se a tese original estava errada).
4. **Instrumentar o F0** com tracing mínimo para que runs futuros gerem telemetria útil.
5. **Estabelecer baseline pós-correção** (se houver correção) via spec-040 audit rodado novamente.

## Não-Objetivos
- Modificar regras SAST (spec-038 está pausado por bons motivos — a tese mudou).
- Mudar schema dos blobs (spec-037 está pausado — o schema funciona, o conteúdo está faltando).
- Re-rodar F0 em todos os 720 repos (caro, e o problema é de diagnóstico, não de execução).
- Curar o trailbase ou outros repos individuais.

## Definição de Pronto (DoD)

### Fase 0 — Coleta de Evidência (read-only, eu executo)
- [ ] Query SQL ao `artefatos_brutos` para distribuição temporal dos runs (confirma/refuta H4).
- [ ] Query SQL para `blob_06`/`blob_08`/`blob_11`: amostra de 10 payloads de 100 bytes para inspeção do conteúdo.
- [ ] Inspeção visual: os 100 bytes são "summary: clean" (H5) ou "skeleton vazio" (bug)?
- [ ] Grep nos logs do `.souls_scratchpad\reports\` por padrões `sidecar.*fail`, `acl.*denied`, `appcontainer`, `-1073740791`.

### Fase 1 — Análise Comparativa (read-only, eu executo)
- [ ] Se H5 for correta: documentar a taxonomia de "payload curto = clean real".
- [ ] Se H2/H4 for correta: identificar os 5 primeiros repos da lista de placeholders e rodar F0 neles **manualmente** (você roda, eu analiso output) para ver o que falha.
- [ ] Se H3 for correta: identificar inconsistência entre `length(payload_blob)` e `metadata`.

### Fase 2 — Diagnóstico Final (síntese)
- [ ] Relatório `docs/diagnostics/PLACEHOLDER_DIAGNOSIS.md` com:
  - Tabela de H1-H5 com status (confirmada/refutada/inconclusiva) e evidência
  - Decisão: o que fazer com os 3 specs pausados
  - Se houver bug: roadmap de correção (spec-042 ou reativação de spec-038)

### Fase 3 — Decisão de Specs Pausados
- [ ] **spec-037:** se H5 correta, reescrever para falar de "métricas de clean" (não schema). Se H2 correta, manter pausado e atacar sidecars.
- [ ] **spec-038:** se H2 correta (sidecars falham), esta spec perde sentido — o problema não é diversificação, é invocação. Se H5 correta, manter e atacar via catálogo de regras.
- [ ] **spec-039 (canário):** se H4 correta, o canário deveria incluir 1-2 dos 720 repos com placeholder, não só trailbase. Se H5 correta, manter foco em trailbase.

## Critérios de Aceite Mensuráveis
1. **Diagnóstico fechado:** as 5 hipóteses têm status claro (confirmada/refutada/inconclusiva) com evidência objetiva.
2. **Decisão sobre specs 037/038/039:** cada spec tem uma nova tese alinhada com o diagnóstico.
3. **Instrumentação:** o F0 tem 1+ log estruturado que permite runs futuros serem diagnosticáveis sem re-rodar.
4. **Zero regressão:** a investigação não modifica nenhum código de produção.
5. **Tempo de execução:** o diagnóstico completo roda em ≤ 2 horas, majoritariamente read-only.

## Fora de Escopo
- Modificar o F0 (Harvester) ou sidecars antes do diagnóstico fechar.
- Re-rodar F0 em batch (caro, foco é entender o passado).
- Reescrever schemas ou regras.
- Promover qualquer spec pausada para `Aprovado`.

## Riscos & Mitigações
| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| H5 é a resposta (placeholders são "clean real") e os 3 specs são todos pausáveis | Alta | Médio | Documentar formalmente, atualizar ADR-031 §2 com taxonomia de "clean placeholder" |
| H2 é a resposta (sidecars falham) e a correção exige refactor grande do sidecar | Média | Alto | Roadmap incremental: spec-042 corrige opengrep, spec-043 corrige govulncheck |
| A análise revela que o problema é arquitetural (ETL não escala para 720 repos) | Média | Alto | spec-044 (a definir) sobre paralelismo e rate limit |
| Logs históricos não têm telemetria suficiente para diagnosticar | Alta | Médio | Aceitar conclusão parcial ("inconclusiva por falta de evidência") + instrumentar F0 para runs futuros |

## Rollback
Esta spec é read-only por natureza (diagnóstico). Nenhum rollback necessário. Se as decisões resultantes levarem a specs de correção, cada uma terá seu próprio rollback.

## Sequência de Execução
1. **Fase 0 (eu):** queries SQL + amostra de 10 payloads + grep em logs. Output: 1 sumário de evidência.
2. **Fase 1 (eu + você):** se H2/H4, identificar 5 repos e você roda F0 neles. Eu analiso saída.
3. **Fase 2 (eu):** relatório `PLACEHOLDER_DIAGNOSIS.md`.
4. **Fase 3 (nós):** decisão conjunta sobre specs 037/038/039.
5. **Fase 4 (HITL):** apresentar diagnóstico + decisões, aguardar aprovação.

## Comandos de Replay

```sql
-- Distribuição temporal dos runs
SELECT
    date(timestamp_extracao, 'unixepoch') AS day,
    artifact_type,
    count(*) AS n
FROM artefatos_brutos
WHERE artifact_type IN ('blob_06_unsafe_hotspots','blob_08_health_report','blob_11_ux_contracts')
GROUP BY day, artifact_type
ORDER BY day DESC, n DESC;

-- Amostra de 10 payloads de 100 bytes do blob_06
SELECT repo_id, length(payload_blob) AS sz, substr(payload_blob, 1, 200) AS head
FROM artefatos_brutos
WHERE artifact_type = 'blob_06_unsafe_hotspots' AND length(payload_blob) BETWEEN 90 AND 110
ORDER BY timestamp_extracao DESC
LIMIT 10;
```
