---
id: "spec-039"
title: "spec-039-trailbase-como-canario-de-stress"
version: 0.3
status: Draft
owner: soda-rust-engine
adr_refs: ["ADR-031", "ADR-019", "ADR-025", "ADR-024"]
depends_on: ["spec-040"]
gates: []
created: "2026-07-16"
target_release: "Souls MC V6.1"
---

# Spec-039 v0.3: `trailbaseio/trailbase` como Canário de Stress do F0 Harvester

## Histórico de Versões
- **v0.1 (2026-07-16):** Propunha trocar trailbase por tokio-rs/tokio. **Abortado pelo Arquiteto.**
- **v0.2 (2026-07-16):** Reescrito. Trailbase é **escolha estratégica deliberada** por ser médio-grande, multi-lâminas e com edge cases. Esta spec transforma o trailbase em **baseline de regressão** do F0.
- **v0.3 (2026-07-16):** Adiciona seção **PRDs Parcialmente Implementados** referenciando melhorias já entregues nos ciclos 042-047. Baselines precisam ser recalculados pós-PRD-042/043/044/045 para refletir os novos formatos (`version_spec`, `audit_header`, deduplicação forense, placeholders estruturais eliminados).

## Contexto
A v0.1 deste spec propunha trocar o repo de teste default do F0 Harvester de `trailbaseio/trailbase` para `tokio-rs/tokio`, sob a justificativa de que o trailbase tem um conflito `libsqlite3-sys` que produz um `blob_08_health_report` dominado por "FALHA FATAL DE COMPILAÇÃO".

**Correção do Arquiteto:** o trailbase foi escolhido **deliberadamente** como default por ser:
- **Médio-grande** — exercita o pipeline de I/O com volume realista
- **Multi-lâminas** — dispara OpenGrep (Rust), Govulncheck (Go), e potencialmente Biome/Clippy em paralelo
- **Edge cases reais** — inclui dependências nativas conflitantes (`libsqlite3-sys`), sub-crates com manifests próprios, e topologia complexa

Em outras palavras, o trailbase é o **teste de stress** do F0, não o "happy path". Trocá-lo por tokio (build limpo, sem edge cases) seria **facilitar o teste**, o oposto do objetivo.

A v0.2 transforma o trailbase em **canário de regressão**: o F0 **deve** rodar sobre trailbase antes de qualquer release e satisfazer critérios mínimos de qualidade.

## Leis Aplicáveis
- **ADR-031 Lei I (Radar Global + Poda Universal):** o trailbase é exatamente o tipo de repo que exercita a Lei I — varredura global + exclusão de lixo — porque tem 5+ sub-crates, `vendor/`, builds, etc.
- **ADR-031 Lei III (Consciência de Monorepo):** o trailbase é **monorepo real** com 4+ sub-crates (`trailbase-core`, `trailbase-sqlite`, `trailbase-extension-so`, `trailbase-cli`). O F0 **deve** localizar cada `Cargo.toml` aninhado e rodar SAST no escopo correto.
- **ADR-031 Lei IV (Zero-Byte Uniforme):** o `blob_08` do trailbase documenta **explicitamente** o conflito de `libsqlite3-sys` e marca como `FALHA FATAL DE COMPILAÇÃO`. Isso é **comportamento esperado**, não regressão.
- **ADR-025 (Consciência de Monorepos):** cross-ref direto — o trailbase é o caso de teste que motivou a ADR-025.

## Objetivos
1. **Formalizar** o trailbase como **canário oficial de regressão** do F0 Harvester, não como "repo de exemplo".
2. Definir **critérios de aceite** que o F0 deve satisfazer ao rodar sobre trailbase **antes de qualquer release** da Souls MC.
3. Adicionar **2 repos complementares** ao canário, cobrindo os gaps que trailbase não cobre:
   - **Repo "happy path":** um repo pequeno, build limpo, sem edge cases (ex: `tokio-rs/mio` ou similar). Valida que o F0 não regrediu em cenários simples.
   - **Repo "zero hotspots":** um repo com código de altíssima qualidade e zero findings SAST (ex: `rust-lang/rust` em snapshot estável, ou similar). Valida que o F0 não está fabricando findings.
4. Implementar o **Stress Test Suite** como rotina automatizada: roda F0 sobre os 3 repos, captura os 33 blobs (3 × 11), roda `spec-040` auditoria sobre eles, e falha o release se algum critério for violado.

## Não-Objetivos
- **Substituir** o trailbase por outro repo. O trailbase **permanece** como default.
- Validar o F0 em todos os 150+ runs históricos (cobertura excessiva, escopo de spec separado).
- Implementar paralelismo multi-repo no F0 (spec separado).
- Curar o conflito `libsqlite3-sys` upstream (não somos maintainers; **é feature**).
- Modificar a lâmina SAST para "driblar" o conflito (o conflito é o teste).

## Definição de Pronto (DoD)

### Pré-condições (TDD Red — validação humana)
- [ ] **spec-040 auditada primeiro** (gating). Esta spec depende do audit data.
- [ ] Selecionados os 2 repos complementares:
  - **happy path:** candidato `tokio-rs/mio` (pode mudar com base no audit)
  - **zero hotspots:** candidato `rust-lang/rust` ou `bitflags/bitflags` (pequeno, altíssima qualidade)
- [ ] Critérios de aceite **validados empiricamente** com 1 rodada do audit sobre os 3 repos.

### Implementação (TDD Green)
- [ ] Documento `docs/state/CANARY_REPOS.md` (novo) com:
  - Lista dos 3 repos canários (trailbase + happy + zero-hotspots)
  - Justificativa de cada escolha
  - Critérios de aceite específicos por repo
- [ ] Script `docs/scripts/run_canary.sh` (POSIX) ou `docs/scripts/run_canary.ps1` (Windows) que:
  1. Roda F0 sobre os 3 repos em sequência (sequencial, não paralelo, para isolar falhas)
  2. Aguarda cada um completar (sem timeout cego — Lei II)
  3. Roda `audit_blob_quality.py` sobre os 33 blobs resultantes
  4. Falha (exit 1) se qualquer critério for violado
  5. Imprime sumário no stdout com cores ANSI (verde/amarelo/vermelho)
- [ ] Integração no pipeline de release (`.github/workflows/release.yml` ou equivalente): o canário **deve** passar antes de taggear uma release.
- [ ] Métricas baseline por repo registradas em `docs/state/CANARY_BASELINES.json`:
  - `trailbaseio/trailbase`: scores esperados por blob (do primeiro audit)
  - `tokio-rs/mio`: scores esperados
  - `rust-lang/rust`: scores esperados

### Validação (Refactor + Verify)
- [ ] Rodar o canário 1x para gerar baseline.
- [ ] Rodar novamente e confirmar que os scores estão **dentro da margem** (±5 pontos) do baseline.
- [ ] Modificar intencionalmente 1 linha do Harvester que sabidamente piora o F0. Confirmar que o canário **detecta a regressão** e exit 1.
- [ ] Reverter a modificação. Confirmar que o canário volta a passar.
- [ ] `cargo check` + `pytest` exit 0.
- [ ] Documento `docs/state/CANARY_RUNS.md` (log) com os 3 últimos runs do canário.

## Critérios de Aceite Mensuráveis (por repo)

### `trailbaseio/trailbase` (canário principal)
- **blob_04_repo_outline:** score ≥ 60 (AST outline completo, com assinaturas Rust)
- **blob_06_unsafe_hotspots:** score ≥ 50 (findings reais, não só header)
- **blob_08_health_report:** score ≥ 50 (documenta falha de compilação, mas tem estrutura)
- **blob_02_dependency_manifest:** score ≥ 80 (multi-ecosystem: cargo + go.sum)
- **Demais blobs:** score ≥ 60
- **Lei IV compliance:** 100% (zero "Warning: Timeout")
- **Rebrand clean:** 100% (zero "genesis_mc")

### `tokio-rs/mio` (happy path)
- **Todos os 11 blobs:** score ≥ 80
- **blob_08_health_report:** NÃO contém "FALHA FATAL DE COMPILAÇÃO"
- **Build limpo:** `cargo check` exit 0 antes de F0 rodar

### `rust-lang/rust` (zero hotspots) — ou similar
- **blob_06_unsafe_hotspots:** score ≥ 70 (poucos findings, alta precisão)
- **blob_08_health_report:** score ≥ 70 (poucos warnings, alta precisão)
- **blob_01_promessa_readme:** score ≥ 90 (README oficial, alta qualidade)

## Fora de Escopo
- Re-rodar F0 em todos os 150+ runs históricos (spec separado se vier a ser necessário).
- Adicionar mais de 3 canários (3 é o mínimo viável; mais aumenta custo sem retorno marginal).
- Implementar paralelismo (sequencial é mais diagnóstico; paralelismo é spec separado).
- Modificar o trailbase upstream (não somos maintainers; o conflito é feature).
- Curar a falha de compilação do trailbase (o ponto é documentar a falha, não resolvê-la).

## Riscos & Mitigações
| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| Tokio/mio é **também** um repo com dependências que mudam | Média | Médio | Pin commit SHA no canário; atualizar manualmente quando necessário |
| rust-lang/rust é muito grande (> 1GB clone) | Alta | Alto | Trocar por repo menor de mesma qualidade (ex: `bitflags/bitflags`, `dtolnay/thiserror`) |
| Canário demora > 30min para rodar | Média | Médio | Rodar em background, exit code 0/1, relatório ao final; CI notifica |
| Baseline do trailbase muda quando trailbase atualiza | Alta | Médio | Pin commit SHA; atualizar baseline em PR separado, revisado por humano |
| Critérios de aceite são muito rígidos (0 findings no rust-lang) | Baixa | Médio | Critérios são **por score agregado**, não por contagem absoluta |

## Rollback
Se o canário revelar que o F0 não pode satisfazer os critérios para nenhum dos 3 repos:
1. Reverter a integração no pipeline de release.
2. Manter o script `run_canary.ps1` no repositório (não deletar) para diagnóstico futuro.
3. Abrir spec de **degradação graceful**: o F0 vira **advisory** (relata, não bloqueia) até o problema ser resolvido.
4. Documentar o bloqueio no `project_memory.md` (L2) e abrir spec-041 (a ser definido) com o root cause.

## Sequência de Execução
1. **Fase 0 — Gating:** spec-040 roda, audit data é revisado, gating decisions são aplicadas.
2. **Fase 1 — Seleção de Canários:** Arquiteto valida `tokio-rs/mio` e o repo "zero hotspots" candidato.
3. **Fase 2 — Baseline:** rodar F0 sobre os 3 repos, capturar scores, registrar em `CANARY_BASELINES.json`.
4. **Fase 3 — Script:** implementar `run_canary.ps1` + integração no release pipeline.
5. **Fase 4 — Validação:** rodar canário 3x, verificar estabilidade. Modificar Harvester intencionalmente, verificar detecção.
6. **Fase 5 — HITL:** apresentar blast radius + baselines ao Arquiteto, aguardar aprovação, promover a `Aprovado`.

## Comandos de Replay (após implementação)

```powershell
# Rodar canário completo (3 repos sequenciais)
powershell "Z:\genesis_mc\docs\scripts\run_canary.ps1"

# Rodar canário para 1 repo apenas
powershell "Z:\genesis_mc\docs\scripts\run_canary.ps1" -Repo trailbaseio/trailbase

# Auditar 33 blobs do canário
python "Z:\genesis_mc\docs\scripts\audit_blob_quality.py"

# Ver baselines históricos
cat "Z:\genesis_mc\docs\state\CANARY_BASELINES.json"
```

## PRDs Parcialmente Implementados (v0.3)

A v0.2 foi escrita assumindo que o F0 entregaria blobs em formato "raw" e que o audit (`spec-040`) avaliaria puramente a qualidade do sinal. Entre v0.2 e v0.3, **5 PRDs foram entregues** que mudam o formato dos blobs e, portanto, **invalidam os baselines propostos na v0.2**. O canário precisa ser recalibrado:

- **PRD-042 (`render_semgrep_header`)** — em [src-tauri/src/harvester/sast/opengrep.rs](file:///Z:/genesis_mc/src-tauri/src/harvester/sast/opengrep.rs). Adiciona `audit_header` canônico ao topo do `blob_06` e `blob_08` (tool, version, timestamp, duration, target_repo, file_count). **Implicação para o canário:** o `score ≥ 60` do `blob_04` e `blob_06` agora se decompõe em "header (10 pts) + findings (50 pts)" — recalcular o peso.
- **PRD-043 (`cargo_workspace_deps_capture`)** — em [src-tauri/src/harvester/extract.rs](file:///Z:/genesis_mc/src-tauri/src/harvester/extract.rs). `parse_cargo_toml` agora cobre `[workspace.dependencies]` e `[build-dependencies]`. **Implicação:** o critério `score ≥ 80` do `blob_02` para trailbase sobe automaticamente (trailbase tem ~15 deps de workspace que antes eram ignoradas). Baseline vira ≥ 90.
- **PRD-044 (`package_json_peer_optional_deps`)** — em [src-tauri/src/harvester/extract.rs](file:///Z:/genesis_mc/src-tauri/src/harvester/extract.rs). `parse_package_json` agora cobre `peerDependencies` e `optionalDependencies`. **Implicação:** o critério de `tokio-rs/mio` (happy path) precisa de repo JS no canário para validar — **mio não exercita este PRD**. Adicionar `vercel/next.js` ou similar como canário secundário (futuro).
- **PRD-045 (`manifest_version_spec_annotation`)** — em [src-tauri/src/harvester/extract.rs](file:///Z:/genesis_mc/src-tauri/src/harvester/extract.rs). `extract_manifest_block` agora ordena alfabeticamente e anexa `version_spec` (ex: `serde 1.0`) em vez de apenas `- serde`. **Implicação:** o `Dumb-LLM Test` agora passa para LLM 3B no `blob_02` — o canário precisa incluir um teste de "LLM 3B consegue listar as 5 maiores deps" como gate de aceite.
- **PRD-033 (`deduplicate_forensic_diagnostics`)** — em [src-tauri/src/harvester/sast/mod.rs](file:///Z:/genesis_mc/src-tauri/src/harvester/sast/mod.rs). Colapsa 32+ erros idênticos de `libsqlite3-sys` em 1 entrada canônica. **Implicação direta:** o `blob_08` do trailbase deixa de ter 245KB de ruído e cai para < 60KB. O critério `score ≥ 50` (linha 92) está **subdimensionado** — a qualidade real agora permite ≥ 80.

**Ação obrigatória antes de promover v0.3 → v0.4:** rodar F0 sobre trailbase com os PRDs 042/043/045/033 ativos e gerar **novos baselines** em `docs/state/CANARY_BASELINES.json`. Os limites da v0.2 ficam como **piso mínimo**; os novos valores serão empíricos.

## Nota para o Futuro
Quando o F0 amadurecer o suficiente para que os critérios de aceite pareçam "fáceis demais", isso é um sinal de que o canário está saturado. Subir os limites (ex: `score ≥ 90` em vez de `≥ 60`) ou adicionar um quarto canário. O canário deve ser **desafiador**, nunca confortável.
