---
prj: SODA
canon: SODA_INGESTAO_V5
phase: "-0.5"
name: Batedor (README truncado + JSON Mode barato)
owner: SODA
status: DRAFT
---

# PRD — REFAC 03 — Batedor (Fase -0.5)

## Objetivo Atômico
Implementar a mecânica do N2 (Batedor) para triagem barata e estritamente estruturada:

- Extrair README e truncar em **3.000 caracteres**
- Chamar DeepSeek V4 em **JSON Mode**
- Preencher apenas:
  - `proposta_original_resumo`
  - `categoria_arquitetural` (10 ENUMs)
- Atualizar estados:
  - `status_atualizacao = TRIAGEM_CONCLUIDA`
  - `status_fase = FASE_-0.5_BATEDOR_OK`

## Contrato de I/O (Entrada e Saída estritas)

### Entradas
- Gatilho: `status_atualizacao = INICIAR_TRIAGEM`
- Dados:
  - `repo_url` (para leitura do README)
  - README bruto (quando existir)
  - `blob_10_soda_canon_context` (contexto canônico anexado como âncora)
  - Catálogo dos 10 ENUMs de `categoria_arquitetural` (fonte única)

### Saídas
- Sheets:
  - `proposta_original_resumo`: texto técnico neutro, curto e verificável (sem prosa promocional)
  - `categoria_arquitetural`: um valor exclusivo dentre os 10 ENUMs
  - `status_atualizacao = TRIAGEM_CONCLUIDA`
  - `status_fase = FASE_-0.5_BATEDOR_OK`

## Cenário de Falha (1 caso explícito)

### Falha: modelo retorna JSON inválido (ou campo ausente)
- Sintoma: saída não parseável em modo JSON ou falta de `categoria_arquitetural`.
- Resposta obrigatória:
  - Não escrever `proposta_original_resumo` nem `categoria_arquitetural` no Sheets.
  - Registrar falha por repositório (erro transacional), sem abortar a rodada inteira.
  - Preservar `status_atualizacao = INICIAR_TRIAGEM` para permitir retry automático controlado (via contador de tentativas).

## Restrições Bare-Metal (Linhas vermelhas)
- Proibido preencher qualquer outra coluna além das duas do Batedor (redução FinOps obrigatória).
- Proibido exceder 3.000 caracteres de README no payload do N2.
- Proibido aceitar valores fora do catálogo de 10 ENUMs.
- Proibido “inventar categoria” quando houver ambiguidade: em dúvida, falhar de forma explícita e auditar (sem prosa livre).

## Definition of Done (exigindo TDD Scaffold)
- Existe Scaffold de testes cobrindo:
  - Truncamento determinístico (README maior → exatamente 3.000 chars; menor → preservado).
  - Validação do JSON estrito (campos obrigatórios presentes e tipos corretos).
  - Validação de ENUM (qualquer valor fora do catálogo falha).
  - Atualização de status: somente em sucesso (`TRIAGEM_CONCLUIDA` + `FASE_-0.5_BATEDOR_OK`).

