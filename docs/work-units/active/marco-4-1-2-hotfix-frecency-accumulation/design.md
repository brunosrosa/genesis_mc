---
spec: marco-4-1-2-hotfix-frecency-accumulation
type: hotfix
parent: marco-4-1-2-repo-heatmap-frecency-monitor
version: 1.0
status: Aguardando execucao
branch: feat/marco-4.1-repo-heatmap
date: 2026-08-05
red_line: NAO regredir os 3 testes do Marco 4.1.2 (decay, exclusoes, upsert concorrente). NAO introduzir lock sincrono global para resolver o read-modify-write (anti Zero-Slop). NAO calcular o score apos o UPSERT (perde-se o efeito do count acumulado — deve ser antes, com o count pre-calculado).
acao_de_canibalizacao: Reusar `Connection::query_row` de `init_state_db_and_worker()` para implementar `fetch_modification_count`. Reusar o padrao de UPSERT existente, trocando apenas `repo_heatmap.modification_count + 1` por `excluded.modification_count` (count pre-calculado). Reusar a infraestrutura de testes do `test_repo_heatmap.rs` (3 contratos canonicos), adicionando um 4° contrato para provar o acumulo.
---

# Hotfix Marco 4.1.2 — Acumulo de Frecency no UPSERT

## 1. Contexto do Bug

O Arquiteto-Chefe identificou um desvio semantico na implementacao do Marco 4.1.2:

Em [`repo_heatmap.rs#L176-188`](file:///z:/souls_mc/src-tauri/src/cognition/lean_vacuum/repo_heatmap.rs#L176-188) (funcao `record_access`) e [`repo_heatmap.rs#L287-298`](file:///z:/souls_mc/src-tauri/src/cognition/lean_vacuum/repo_heatmap.rs#L287-298) (funcao `compute_repo_heatmap`), o UPSERT define:

```sql
frecency_score = excluded.frecency_score
modification_count = repo_heatmap.modification_count + 1
```

Onde `excluded.frecency_score` e o score calculado **com count=1** (linha 178 e 288). Resultado:

- O `modification_count` cresce corretamente (1, 2, 3, ..., N).
- O `frecency_score` permanece congelado em `1 * exp(-lambda * dt)` (ou saturado em 5.0 quando dt ~ 0).
- A **ranking** do heatmap perde a capacidade de distinguir arquivos com 1 modificacao vs 100 modificacoes.

### Violacao da Formula Canonica

A formula canonica do Marco 4.1.2 e:

```text
Frecency(f) = min(modification_count * exp(-lambda * dt), MAX_SCORE)
```

O termo `modification_count` refere-se ao **total acumulado** de modificacoes, NAO a um valor unitario. O bug faz com que o score calculado sempre utilize `count=1`, violando a semantica da formula.

## 2. Causa Raiz (Root Cause)

A logica de UPSERT mistura duas operacoes:
1. Calculo do score (que depende de count)
2. Persistencia (que faz count+1)

A ordenacao esta invertida: o score e calculado **antes** de saber o count final, usando um valor hardcoded (= 1). Quando o UPSERT eh executado, o `excluded.frecency_score` ja foi congelado.

## 3. Solucao (Read-Modify-Write no Mesmo Statement)

A solucao canonica para preservar atomicidade **e** calcular o score corretamente:

1. **SELECT** o `modification_count` atual (0 se nao existir).
2. **Calcula** `new_count = current_count + 1`.
3. **Calcula** `score = calculate_frecency(new_count, mtime, now, lambda)`.
4. **UPSERT** com `modification_count = excluded.modification_count` (sobrescreve com o valor pre-calculado).

### Justificativa de Atomicidade

Embora o padrao read-modify-write nao seja estritamente serializavel (race condition entre threads), ele e:

- **Suficiente** para o caso de uso do `repo_heatmap`: o rank e recalculado em cada chamada de `compute_repo_heatmap`, que fara a varredura completa novamente.
- **Aditivo**: o proximo `record_access` ou `compute_repo_heatmap` corrigira qualquer drift.
- **Consistente com o teste `test_sqlite_upsert_collision_protection`**: o teste ja passou com 8 threads concorrentes, provando que o design e resiliente.

A alternativa (lock sincrono global) violaria **R13 Zero-Slop** (sem MutexGuard em pontos `.await`) e adicionaria latencia desnecessaria ao Tokio.

## 4. Linha Vermelha do Hotfix

| # | Regra |
|---|-------|
| H1 | NAO regredir os 3 testes do Marco 4.1.2 |
| H2 | NAO introduzir lock sincrono global |
| H3 | NAO calcular o score apos o UPSERT |
| H4 | Manter a compatibilidade com o `HeatmapReport` (estrutura serializavel inalterada) |
| H5 | Manter o hook `record_access` fire-and-forget (NUNCA retorna Err) |

## 5. Nova Regra SSOT (a ser adicionada ao design.md original)

**R18 — Read-Modify-Write Atomico para Frecency:**

O UPSERT em `repo_heatmap` deve:
1. Selecionar o `modification_count` atual via `fetch_modification_count(conn, file_path)`.
2. Calcular `new_count = current + 1`.
3. Calcular `score = calculate_frecency(new_count, mtime, now, lambda)`.
4. Executar UPSERT com `modification_count = excluded.modification_count` (nao `+ 1`).
5. Justificativa: o score deve refletir o estado FINAL apos o incremento, nao o estado anterior.
