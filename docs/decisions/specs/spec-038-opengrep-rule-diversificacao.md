---
id: "spec-038"
title: "spec-038-opengrep-rule-diversificacao"
version: 0.3
status: Draft_Aguardando_Audit
owner: souls-rust-engine
adr_refs: ["ADR-031", "ADR-024", "ADR-025"]
depends_on: ["spec-040"]
gates: []
created: "2026-07-16"
target_release: "Souls MC V6.1"
---

# Spec-038: Diversificação de Regras OpenGrep para o Harvester

## PRDs Parcialmente Implementados (v0.3)

Esta spec continua `Draft_Aguardando_Audit`, mas a tese de "monocultura severa" foi **amenizada** pelo header canônico introduzido no PRD-042, que separa o sinal agregado em `[SEVERITY_BREAKDOWN]: ERROR=2, WARNING=5` (informação estruturada que LLMs 3-7B interpretam sem ambiguidade):

- **PRD-042 (`render_semgrep_header`)** — em [src-tauri/src/harvester/sast/opengrep.rs](file:///Z:/souls_mc/src-tauri/src/harvester/sast/opengrep.rs). O `severity_breakdown` é uma BTreeMap `String → usize` que **fornece diversidade estatística** sobre o payload (quantos ERROR vs WARNING vs INFO), mesmo quando as findings vêm de uma única regra. **4 testes TDD verdes**.

**O que esta spec ainda pede (e não foi tocado):**
1. Rotação efetiva de **múltiplas regras** do catálogo `src-tauri/semgrep/rules` (~700 regras disponíveis, apenas 4-8 usadas).
2. Política de exclusão inteligente para não re-rotacionar para regras que produzem 0 findings.

**Implicação para o gating decision:** a monocultura **deixou de ser assintomática** — o header já entrega o sinal mínimo. A urgência diminuiu. Pode ser adiada para v7 ou executada em janela separada.

## Gating Decision (adicionado em v0.2)

Esta spec **NÃO deve ser implementada antes de `spec-040` (Auditoria Qualitativa) rodar** e confirmar que a dimensão `diversidade_fonte` dos blobs 06/08 tem score médio < 50 em **pelo menos 50% dos runs** (indicando monocultura sistêmica). Se o audit mostrar diversidade saudável, esta spec perde motivação e deve ser pausada.

**Status atual:** `Draft_Aguardando_Audit`. Será promovido a `Draft` (executável) após o gating, ou a `Pausado` se a monocultura for isolada (apenas trailbase, não sistêmica).

## Contexto
A auditoria de validação do Spec-036 (rebrand) sobre o `blob_08_health_report` do trailbase (429.781 bytes, 1.184 findings) revelou **monocultura severa de regras SAST**:

- **1.183 de 1.184 findings** (99,92%) são da mesma regra: `souls.rust.panic.unwrap.expect`.
- **1 finding** (0,08%) é `govulncheck` info, sem vulnerabilidades.
- Distribuição por arquivo: `src/records/read_record.rs` (111), `src/tests.rs` (85), `src/records/list_records.rs` (76).

Consequência: a Lente C das Fases 2-3 recebe um relatório massivamente homogêneo. Não consegue distinguir "este repo tem 1.183 unwraps perigosos em código de produção" de "este repo tem 1.183 unwraps em testes inofensivos". O sinal agregado é pobre.

Cross-ref com o [ADR-031 §4 Blob 6 e Blob 8](file:///Z:/souls_mc/docs/decisions/adrs/ADR-031-Harvester-Anatomia-11-Blobs-e-Leis-Inegociaveis.md): a lâmina é `opengrep` com `--skip-formatter` no Blob 8. A mudança proposta é **diversificar as regras** que essa lâmina aplica, mantendo o mesmo motor.

## Leis Aplicáveis
- **ADR-031 Lei I (Radar Global + Poda Universal):** novas regras devem respeitar a exclusão de `tests/`, `mocks/`, minificados, lockfiles (cross-ref ADR-024 §B).
- **ADR-031 Lei II (Timeouts Elásticos):** novas regras **não podem** reintroduzir timeout fixo. Devem usar `--allow-rule-timeout-control` ou heurística equivalente.
- **ADR-024 §C (Fobia de Minificados):** nenhuma regra nova pode ter pattern que produza combinatorial blowup em arquivos densos.
- **ADR-025 (Consciência de Monorepo):** regras devem aplicar em alvos recursivos (`./...`, `**/*.rs`), não no root cego.

## Objetivos
1. Curar e adicionar **≥3 classes novas de regras** OpenGrep que cubram dimensões ortogonais ao `panic/unwrap/expect`.
2. Demonstrar que o relatório final **quebra a monocultura** (nenhuma classe deve dominar >40% dos findings).
3. Manter ou reduzir o **False Positive Rate (FPR)** estimado, não aumentar.
4. Não regressar performance (timeout total do scan não deve dobrar).

## Não-Objetivos
- Reescrever OpenGrep do zero.
- Adicionar regras de formatação (lint) — essas são responsabilidade do Biome/Clippy, não do OpenGrep.
- Adicionar regras de segurança que dependam de build (taint analysis, type checking) — OpenGrep é análise estática pura.
- Cobrir linguagens além de Rust/TS/JS/Python/Go (escopo atual do Harvester).

## Definição de Pronto (DoD)

### Pré-condições (TDD Red)
- [ ] `tests/fixtures/sast_rules/` com 5+ regras candidatas em YAML (uma por classe nova).
- [ ] `tests/fixtures/sast_corpus/` com 10+ snippets de código de teste (positivos e negativos para cada regra).
- [ ] `tests/test_sast_diversificacao.py` com **5 testes vermelhos**:
  - `test_classe_complexity_ciclomatica_detecta()` — função com `if/for/match/&&` aninhados > 10 é flagada.
  - `test_classe_function_length_detecta()` — função > 80 linhas é flagada.
  - `test_classe_dead_code_detecta()` — função ou `pub` item nunca referenciado é flagado.
  - `test_classe_hardcoded_secrets_detecta()` — string com padrão de API key/secret é flagada.
  - `test_classe_sql_injection_detecta()` — `format!()` com `SELECT` é flagado.

### Implementação (TDD Green)
- [ ] Módulo `souls_semgrep_rules/` adicionado ao `harvester/sidecar.rs` com ≥3 classes:
  1. **Complexidade Ciclomática** (`souls.rust.complexity.cyclomatic`): conta pontos de decisão (if/else/match/for/while/&&/||/?) por função; flag se > 10.
  2. **Comprimento de Função** (`souls.rust.function.length`): conta linhas de uma função; flag se > 80.
  3. **Hardcoded Secrets** (`souls.security.hardcoded.secret`): regex sobre strings com alta entropia + prefixos comuns (`sk-`, `ghp_`, `AKIA`, `-----BEGIN`).
  4. **SQL Injection** (`souls.security.sql.injection`): `format!()` ou `concat!()` contendo `SELECT|INSERT|UPDATE|DELETE` em argumento de `execute()`.
  5. **Dead Code (heurístico)** (`souls.rust.dead.unused`): itens `pub` sem `pub use` nem chamadas detectadas no AST outline (Blob 04).
- [ ] Integração no `harvester/sidecar.rs`: as 5 regras são concatenadas ao ruleset base do OpenGrep antes da invocação.
- [ ] Adaptação do ADR-024: atualizar referência para mencionar "5 classes de regras" em vez de "1 classe dominante".

### Validação (Refactor + Verify)
- [ ] Rodar F0 sobre `trailbaseio/trailbase`. Capturar relatório antes/depois.
- [ ] Métrica principal: **nenhuma classe > 40% dos findings**.
- [ ] Métrica secundária: **total de findings cresceu ≥ 2x** (mais sinal) e **FPR estimado caiu** (menos falsos positivos em `tests/`).
- [ ] `cargo check` + `cargo clippy -- -D warnings` exit 0.
- [ ] `pytest tests/test_sast_diversificacao.py` exit 0 (5 testes verdes).
- [ ] Tempo total do OpenGrep scan sobre trailbase **não dobrou** (cross-ref ADR-024 Lei II).

## Critérios de Aceite Mensuráveis
1. **Diversidade:** após o spec, as top-5 classes de regras juntas devem cobrir ≥ 80% dos findings (hoje 1 classe cobre 99,92%).
2. **FPR estimado:** inspeção manual de 50 findings aleatórios mostra ≤ 15% em `tests/`/`mocks/` (hoje é difícil calcular, mas o ADR-031 Lei I já filtra `tests/`, então o baseline é 0%).
3. **Performance:** OpenGrep scan sobre trailbase completa em ≤ 1.5x o tempo atual (baseline = 429 KB de relatório gerado).
4. **Cobertura nova:** as 5 classes cobrem **pelo menos** complexidade, length, secrets, injection, dead code — 5 dimensões ortogonais.
5. **Falsos positivos:** zero falsos positivos nos 5 testes TDD por classe.

## Fora de Escopo
- Adicionar análise de taint/type-check (depende de compilação, fora do escopo do OpenGrep).
- Adicionar regras para linguagens além de Rust/TS/JS/Python/Go.
- Reescrever as regras SOULS existentes (`souls.rust.panic.unwrap.expect` permanece, mas perde dominância).
- Auto-tuning de thresholds (ex: 80 linhas é limite fixo por enquanto).

## Riscos & Mitigações
| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| Regra de secrets gera muitos falsos positivos (ex: hash strings) | Alta | Médio | Heurística de entropia (Shannon > 4.5) + dicionário de prefixos; testes TDD com 3 positivos e 3 negativos |
| Regra de complexidade explode em macros Rust | Média | Médio | Whitelist de macros comuns (`println!`, `vec!`, `format!`); pattern matching estrito |
| Regra de dead code tem FPR alto (itens públicos intencionalmente expostos) | Alta | Médio | Heurística adicional: só flagar itens sem `pub use` E sem match em nenhum `crate::` import dentro do repo |
| OpenGrep scan dobrar de tempo | Média | Médio | Manter `--allow-rule-timeout-control`; benchmark antes/depois; se > 1.5x, desabilitar classe menos valiosa |
| Quebra de retro-compat em CI/CD externo | Baixa | Alto | Regras são **aditivas**, não substituem; consumers continuam vendo `panic/unwrap` + 5 novas classes |

## Rollback
Se as novas regras gerarem poluição ou FPR alto em produção:
1. Flag de feature `SOULS_SAST_DIVERSIFY=off` (env var) desabilita as 5 classes novas.
2. Manter o código das regras no repositório (não deletar) para retry.
3. Atualizar `project_memory.md` (L2) com métricas de rollback.
4. Abrir spec de correção com base nos logs de FPR coletados.

## Sequência de Execução
1. **Fase 1 — TDD Red:** fixtures + 5 testes vermelhos.
2. **Fase 2 — TDD Green:** implementar 5 classes de regras + integração no sidecar.
3. **Fase 3 — Refactor:** deduplicação de patterns comuns, comentários canônicos.
4. **Fase 4 — Validação:** rodar F0 sobre trailbase, medir diversidade e FPR.
5. **Fase 5 — HITL:** apresentar blast radius + métricas ao Arquiteto, aguardar aprovação, rebase semântico.
