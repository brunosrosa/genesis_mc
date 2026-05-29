---
prj: SODA
canon: SODA_ETL_V3
phase: "5"
name: Deep Formatter (Canibalizacao)
owner: SODA
status: DRAFT
---

# PRD — Fase 5 (Deep Formatter / Canibalização)

## Objetivo
Quando uma análise for considerada pronta para integração, decompor o software em subcomponentes (COMP_0001, COMP_0002, …) com granularidade executável, preenchendo a aba `DEEP_COMPONENTS_v3` com estrutura estável e acionável.

## Gatilho (Controle HITL)
A Fase 5 não roda automaticamente.
- O operador define `status_atualizacao` para um estado de ação (ex.: `"HITL_APROVADO_PARA_DEEP"`)
- A execução é disparada por CLI dedicada ou botão/ação controlada (fora do escopo deste PRD)

## Condição de Elegibilidade (SSOT)
Ler a linha do repositório analisado na aba `MASTER_SOLUTIONS` e avaliar:
- `tipo_integracao`
- `acao_de_canibalizacao`
- `classificacao_terminal`

### Regra
Somente prosseguir quando a classificação indicar integração/absorção.
Exemplos:
- `tipo_integracao = INTEGRATE_AS_COMPONENT` ⇒ elegível
- `acao_de_canibalizacao = ABSORVER_LOGICA` ⇒ elegível
- Caso contrário ⇒ escrever uma linha de auditoria “skipped” e encerrar sem efeitos colaterais.

## Entrada (Contexto)
O prompt deve ser composto exclusivamente de:
- `repo_url`, `repo_version`, `lote_id`
- Campos-chave do SGR (Blocos 1–4) necessários para decomposição
- Resumos/essências do distilador (somente o mínimo para evitar estouro)

## Saída (Estrutura em `DEEP_COMPONENTS_v3`)
Cada subcomponente gera 1 linha:
- `repo_url`
- `lote_id`
- `comp_id` (ex.: COMP_0001)
- `comp_name`
- `comp_purpose`
- `comp_inputs`
- `comp_outputs`
- `comp_public_api`
- `comp_internal_modules`
- `comp_dependencies`
- `comp_risks`
- `comp_tests_hint`
- `comp_integration_steps`
- `model_used`
- `cost_usd`
- `created_at_epoch`

## Decodificação Restrita (Obrigatória)
A Fase 5 deve usar `response_format: json_schema` com `strict: true` para impor:
- Array não-vazio de componentes
- Campos obrigatórios por componente
- Tipos e limites de tamanho por campo
- Proibição de propriedades extras

### Envelope JSON (exemplo de shape)
`{ "components": [ { ... } ], "justifications": { ... } }`

## FinOps / Roteamento
- Modelo default: DeepSeek V4-Pro
- Budget por execução:
  - Max tokens total configurável por env
  - Abort se custo estimado/executado exceder teto
- Persistir `usage` (tokens/custo) junto às linhas geradas

## Restrições
- Proibido gerar componentes “fantasma” sem apontar módulos/arquivos/artefatos correlatos
- Proibido reescrever dados da aba `MASTER_SOLUTIONS`
- A escrita em `DEEP_COMPONENTS_v3` deve ser append-only e idempotente por `(repo_url, lote_id, comp_id)`

## Definition of Done
- Ao rodar para um repo elegível, popula `DEEP_COMPONENTS_v3` com pelo menos 2 componentes válidos
- Saída é sempre JSON estrito (sem markdown) e passa na validação de schema
- Custo e modelo usados são persistidos por linha
