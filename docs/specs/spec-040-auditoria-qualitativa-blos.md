---
id: "spec-040"
title: "spec-040-auditoria-qualitativa-blos"
version: 0.2
status: Draft
owner: souls-rust-engine
adr_refs: ["ADR-031", "ADR-019", "ADR-024", "ADR-025"]
gates: ["spec-037", "spec-038", "spec-039"]
created: "2026-07-16"
target_release: "Souls MC V6.1"
---

# Spec-040 v0.2: Auditoria Qualitativa 0-100 dos 11 Blobs do Harvester (Fase 0)

## Histórico de Versões
- **v0.1 (2026-07-16):** Definiu 8 dimensões de scoring ortogonais (tamanho, estrutura, Lei IV, diversidade, refs, slop, rebrand, schema). Gating decision para 037/038/039.
- **v0.2 (2026-07-16):** Adiciona **7 novas dimensões de scoring** derivadas dos PRDs entregues nos ciclos 042-047. As dimensões originais continuam válidas; as novas permitem **discriminar** sinais que a v0.1 colapsava (ex: `tamanho_sadio` não distinguia "manifest com version_spec" de "manifest cru").

## Contexto
O F0 Harvester extrai 11 artefatos por repositório (ver [ADR-031 §4](file:///Z:/souls_mc/docs/adrs/ADR-031-Harvester-Anatomia-11-Blobs-e-Leis-Inegociaveis.md)) e os persiste em `artefatos_brutos.payload_blob`. O SQLite contém **150+ runs históricas** (uma por repo) para os blobs 06 e 08, e algumas dezenas para outros. A maioria dos runs vem de execuções **pré-rebrand** (placeholder mínimo, ~100-500 bytes) e apenas `trailbaseio/trailbase` tem run pós-rebrand com payload rico.

**Não temos visibilidade sistêmica** sobre quais blobs são consistentemente fracos. Cada análise feita até hoje foi **ad-hoc, repo a repo**. Esta spec define o instrumento de medida que fecha essa lacuna e serve de **gating decision** para spec-037, spec-038 e spec-039.

## Leis Aplicáveis
- **ADR-031 Lei IV (Zero-Byte Uniforme):** o scoring tem dimensão binária dedicada (`lei_iv_compliance`) que zera o score de qualquer payload com `"Warning: Timeout"` ou `"Erro: 0 matches"`. Hard-cap em 50 se violada.
- **ADR-031 Lei I (Radar Global + Poda Universal):** o script **só lê** o SQLite. Não extrai nada, não roda nenhum sidecar, não toca o FS dos repos. Zero risco de I/O pesado.
- **ADR-031 §4 (Anatomia dos 11 Blobs):** o scoring é parametrizado por blob com 11 heurísticas independentes (uma por tipo).
- **ADR-001 (Core Stack Restrita):** o script usa **apenas stdlib** (sqlite3, re, json, pathlib, collections, datetime, statistics). Zero dependência externa.

## PRDs Parcialmente Implementados (v0.2)

A v0.1 definia 8 dimensões. Entre v0.1 e v0.2, **7 PRDs foram entregues** (ciclos 042-047) que mudam o formato dos blobs em aspectos que a v0.1 **não discriminava**. Sem novas dimensões, a auditoria continuaria dando scores médios parecidos para runs pré-PRD e pós-PRD — perdendo o ganho real.

**7 novas dimensões (v0.2):**

| # | Dimensão | Peso | PRD Origem | Sinal Discriminado |
|---|---|---:|---|---|
| 9 | `audit_header` | 5% | PRD-042 | Presença e qualidade do header canônico (tool, version, timestamp, target_repo, file_count). Detecta se `render_semgrep_header` está ativo no payload. |
| 10 | `version_spec` | 5% | PRD-045 | Manifest com `version_spec` ao lado do nome da dep (ex: `serde 1.0` vs `- serde`). Detecta ordenação alfabética e anotação semântica. |
| 11 | `behavior_annotation` | 5% | PRD-046 | Testes Rust com `behavior` ou `// intent:` declarado acima da assinatura. Distingue "achou assinatura de teste" de "entendeu o que o teste verifica". |
| 12 | `layer_classification` | 5% | PRD-047 | Outline arquitetural com camadas (core/adapter/infra) marcadas. Distingue árvore de arquivos de mapa semântico. |
| 13 | `infra_completeness` | 5% | PRD-043/044 | Manifest cobre `[workspace.dependencies]`, `[build-dependencies]`, `peerDependencies`, `optionalDependencies`. Distingue "capturou o que viu" de "capturou tudo que existe". |
| 14 | `test_assert_context` | 5% | PRD-046 | Testes com macro assert visível (`assert_eq!`, `assert!`, `expect()`) dentro do trecho extraído. Distingue "listou testes" de "capturou a lógica do teste". |
| 15 | `forensic_dedup_ratio` | 5% | PRD-033 | Razão entre findings únicos e total no `blob_08`. 1.0 = sem dedup necessária (clean), < 0.1 = bug de monocultura idêntica (caso trailbase pré-PRD-033). |

**Total acumulado:** 8 dimensões v0.1 (soma 100%) + 7 dimensões v0.2 (soma 35%) = **135%**. Para preservar a escala 0-100 do score final, os pesos da v0.1 são **renormalizados** com fator `100/135 ≈ 0.74` e os 7 novos pesos usam o mesmo fator. A ordem relativa entre dimensões é preservada; a escala absoluta continua 0-100.

**Fórmula de score v0.2:**

```
score_final = clamp(0, 100, Σ(peso_dim × score_dim × 0.74))
```

**Justificativa da renormalização:** manter a intuição da v0.1 ("score ≥ 60 = aceitável, ≥ 80 = excelente") é mais importante que preservar os pesos originais. A v0.1 usou pesos "políticos" (Lei IV 20% hard-fail); a v0.2 distribui o peso adicional proporcionalmente. **Mudança de comportamento:** um run que era `score=72` na v0.1 pode virar `score=58` na v0.2 (se não tiver as 7 novas dimensões) e `score=85` (se tiver todas). Isso é **desejável** — estamos premiando runs pós-PRD.

**Comportamento de `lei_iv_compliance`:** mantém o hard-fail em 50 (não renormaliza), porque é invariante de segurança.

## Objetivos
1. Computar um score **0-100 por par (repo, blob)**, baseado em 8 dimensões ortogonais.
2. Agregar o score por `artifact_type` para identificar **quais blobs são sistematicamente fracos**.
3. Identificar os **top-N piores casos** (score < 50) para inspeção manual.
4. Detectar violações da Lei IV (ADR-031), de rebrand (Spec-036) e de slop em escala.
5. Gerar **2 artefatos** consumíveis:
   - `docs/audits/quality/_QUALITY_SCORES.md` (human-readable)
   - `docs/audits/quality/_QUALITY_SCORES.json` (machine-readable)
6. **Gating decision:** os resultados deste spec determinam se spec-037, spec-038 e spec-039 devem ser executados, pausados ou reescritos.

## Não-Objetivos
- Substituir inspeção manual humana. O audit é um **filtro**, não veredito final.
- Modificar o Harvester ou qualquer código de produção.
- Adicionar dependências externas ao script.
- Re-rodar F0 em qualquer repo. O audit é **read-only** sobre o SQLite.
- Internacionalização. Mensagens e termos permanecem em inglês/português mixados (como o resto do projeto).

## Definição de Pronto (DoD)

### Pré-condições (TDD Red — validação humana)
- [ ] Script `docs/scripts/audit_blob_quality.py` criado e testado em modo `--help` (sai sem erro).
- [ ] Smoke test manual: rodar script com `--repo-allowlist trailbaseio/trailbase` e confirmar que produz 11 scores (1 por blob).
- [ ] Os 2 artefatos (`_QUALITY_SCORES.md` e `_QUALITY_SCORES.json`) são gerados em `docs/audits/quality/`.

### Implementação (TDD Green)
- [ ] 8 dimensões de scoring implementadas com pesos:
  1. `tamanho_sadio` (20%) — bytes dentro de `[min, max]` esperado por blob
  2. `estrutura_canonica` (20%) — presença de marcadores regex por blob
  3. `lei_iv_compliance` (20%, hard-fail) — zero "Warning: Timeout" no payload
  4. `diversidade_fonte` (10%) — múltiplas ferramentas (opengrep + govulncheck + ...)
  5. `refs_file_line` (10%) — findings com `file.ext :: L<n>`
  6. `sem_slop` (10%) — zero `TODO/FIXME/PLACEHOLDER/XXX/HACK/WIP`
  7. `rebrand_clean` (5%) — zero menções a `genesis_mc`
  8. `retrocompat_schema` (5%) — parseabilidade compatível com `detect_payload_column`
- [ ] Sampling eficiente: payloads > 6KB são amostrados como `head(2KB) + middle(2KB) + tail(2KB)`.
- [ ] 3 visões no output Markdown:
  1. Tabela horizontal: 11 colunas × N linhas (1 por repo)
  2. Agregado por artifact_type: média/std/min/max/n
  3. Top-10 piores e Top-10 melhores casos
- [ ] Sumário no stdout: ranking de blobs por média + top piores/melhores.

### Validação (Refactor + Verify)
- [ ] Script roda em < 60s para os 150+ runs históricos (single-thread).
- [ ] `python audit_blob_quality.py` exit 0, sem warnings, sem exceptions.
- [ ] Os scores produzidos são **estáveis** (mesmo input → mesmo output, dado timestamp).
- [ ] O `_QUALITY_SCORES.md` é legível sem renderização especial (markdown puro).

## Critérios de Aceite Mensuráveis
1. **Performance:** script completa em ≤ 60s para 500 pares (repo, blob).
2. **Determinismo:** rodar 2x seguidas produz scores idênticos (modulo timestamp do relatório).
3. **Cobertura:** 11/11 blobs são scorados para cada repo que tem o blob (artefatos faltantes são marcados como `—` na tabela, não como 0).
4. **Discriminação:** o agregado por artifact_type mostra std > 0 (diferencia blobs fortes de fracos).
5. **Detecção de violações:** a dimensão `lei_iv_compliance` zera o score de qualquer payload com `"Warning: Timeout"` e a dimensão `rebrand_clean` zera para `"genesis_mc"`.

## Gating Decision (este é o ponto-chave)

Após executar o audit, os outros 3 specs são revisados:

| Audit signal | Ação em spec-037 | Ação em spec-038 | Ação em spec-039 |
|---|---|---|---|
| `blob_08` média < 60 (sistêmico) | **Executar** | manter Draft | pausar |
| `diversidade_fonte` blob_06/08 média < 50 (monocultura sistêmica) | manter Draft | **Executar** | pausar |
| Trailbase score geral ≥ 70 (validado) | n/a | n/a | manter Pausado |
| Trailbase score geral < 50 (revelou problema estrutural) | reavaliar | reavaliar | reescrever |
| `lei_iv_compliance` violações > 10% | **bug P0 no F0** — abrir spec-041 | | |
| `rebrand_clean` violações > 0 | spec-036 não está realmente validado | | |

**Regra de ouro:** nenhum dos 3 specs (037/038/039) deve ser implementado antes do audit rodar. Atraso permitido, decisão cega proibida.

## Fora de Escopo
- Modificar o Harvester, sidecar, ou qualquer código Rust.
- Adicionar dependências externas ao Python.
- Implementar dashboard web (o `.md` é suficiente para v1).
- Re-rodar F0 em qualquer repo. O audit é puramente read-only.
- Internacionalizar mensagens.
- Substituir `extract_audit_blobs.py` (esse continua útil para extração TXT; o `audit_blob_quality.py` é para scoring).

## Riscos & Mitigações
| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| Heurística de tamanho é muito rígida para casos legítimos | Média | Médio | Constantes `SIZE_HEURISTICS` no topo do script — ajustar com 1 linha; documentar cada mudança |
| Sampling perde signal em payloads > 6KB | Baixa | Médio | 6KB é suficiente para capturar header + findings iniciais + footer; para blobs grandes isso preserva a estrutura |
| Scores não discriminam blobs fortes de fracos (todos ~70) | Baixa | Alto | Validar pós-1ª execução; se std < 5 em todos os blobs, recalibrar pesos |
| Script lento para 500+ runs (> 60s) | Baixa | Baixo | Sampling já mitiga; se necessário, paralelizar com `multiprocessing` em v2 |
| Usuário roda o script esperando JSON mas recebe MD (ou vice-versa) | Baixa | Baixo | Ambos são gerados sempre; sumário no stdout deixa claro |

## Rollback
Se o audit revelar problemas graves (ex: violações de Lei IV > 50%), **não há rollback** — o audit é read-only. Os artefatos gerados são `docs/audits/quality/_QUALITY_SCORES.{md,json}` e podem ser apagados sem efeito colateral. As decisões tomadas com base nele são registradas no `project_memory.md` (L2) para auditoria futura.

## Sequência de Execução

1. **Fase 1 — Geração de Instrumento:** este spec + `docs/scripts/audit_blob_quality.py` (escritos; sujeito a revisão do Arquiteto).
2. **Fase 2 — Smoke Test:** Arquiteto roda `python docs/scripts/audit_blob_quality.py --repo-allowlist trailbaseio/trailbase` para validar output.
3. **Fase 3 — Full Run:** Arquiteto roda `python docs/scripts/audit_blob_quality.py` (todos os 150+ repos).
4. **Fase 4 — Análise Conjunta:** Arquiteto e Agente revisam `_QUALITY_SCORES.md` juntos, decidem gating decision.
5. **Fase 5 — Replanejamento:** specs 037/038/039 são editados conforme a matriz de gating acima. Promoção de `Draft` para `Aprovado` é feita após o audit, não antes.

## Comandos de Replay

```powershell
# Smoke test (1 repo, valida output)
python "Z:\souls_mc\docs\scripts\audit_blob_quality.py" --repo-allowlist trailbaseio/trailbase

# Full audit (todos os repos)
python "Z:\souls_mc\docs\scripts\audit_blob_quality.py"

# Customizado (top-20, score mínimo 40)
python "Z:\souls_mc\docs\scripts\audit_blob_quality.py" --top 20 --min-score 40
```
