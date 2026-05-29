---
prj: SODA
canon: SODA_INGESTAO_V5
phase: "1-4 + SC"
name: FinOps Skip + Cleanup (SHORT-CIRCUIT)
owner: SODA
status: DRAFT
---

# PRD — REFAC 04 — FinOps (Pular Colunas) + Cleanup (SHORT-CIRCUIT)

## Objetivo Atômico
Reduzir custo de tokens e impedir re-trabalho ao refatorar o fluxo do Motor Cloud (Fases 1 a 4) para:

1. Tratar como **read-only** as colunas já preenchidas pelo Batedor (N2):  
   - `proposta_original_resumo`  
   - `categoria_arquitetural`
2. Implementar a rotina de exclusão de blobs residuais vinculada ao **`SHORT-CIRCUIT`** (rejeição humana), limpando disco e congelando a linha.

## Contrato de I/O (Entrada e Saída estritas)

### Entradas
- Gatilho FinOps:
  - `status_atualizacao = APROVADO_PARA_ENXAME`
  - Linha no Sheets pode já conter valores em `proposta_original_resumo` e `categoria_arquitetural`
- Gatilho Cleanup:
  - `status_atualizacao` começa com `REJEITADO_`
- Estado local:
  - SQLite com blobs/artefatos associados ao repositório (ex.: tabela de artefatos brutos)

### Saídas
- FinOps Skip:
  - O Motor Cloud não reescreve `proposta_original_resumo` nem `categoria_arquitetural` quando já preenchidas.
  - O payload de geração estruturada da Fase 3 é reduzido para não incluir esses campos como “a gerar”.
  - A carga final no Sheets preserva os valores do Batedor e atualiza o restante.
  - `status_fase = FASE_4_SHEETS_UPDATED` ao final da Fase 4.
- Cleanup:
  - `status_fase = SHORT-CIRCUIT`
  - Deleção dos blobs residuais no SQLite associados ao repo.
  - A linha passa a ser considerada congelada (não roteável para N1..N5).

## Cenário de Falha (1 caso explícito)

### Falha: SQLite retorna SQLITE_BUSY durante deleção (cleanup)
- Sintoma: concorrência de escrita impede exclusão imediata dos blobs.
- Resposta obrigatória:
  - Retries com backoff + jitter até um limite finito.
  - Se persistir, registrar falha por repositório e manter `status_fase = SHORT-CIRCUIT` (congelamento permanece), evitando retentativas infinitas.

## Restrições Bare-Metal (Linhas vermelhas)
- Proibido re-gerar campos que já foram preenchidos no N2 (anti-custo; anti-deriva).
- Proibido sobrescrever rejeição humana (`REJEITADO_*`) por qualquer etapa mecânica.
- Proibido manter blobs após rejeição: o cleanup é mandatário para poupar disco e reduzir blast radius.
- Proibido apagar outras linhas ou artefatos não associados ao repo (deleção deve ser cirúrgica e auditável).

## Definition of Done (exigindo TDD Scaffold)
- Existe Scaffold de testes cobrindo:
  - Preservação: quando `proposta_original_resumo` e `categoria_arquitetural` já existem, o fluxo 1..4 não altera esses valores.
  - Cleanup: `REJEITADO_*` implica `status_fase = SHORT-CIRCUIT` e dispara exclusão somente dos blobs do repo alvo.
  - Idempotência: rodar cleanup duas vezes não causa falha (segunda execução é no-op).
  - Limite de retry: não existe loop infinito em caso de SQLITE_BUSY.

