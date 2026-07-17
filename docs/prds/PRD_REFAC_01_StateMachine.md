---
prj: SODA
canon: SODA_INGESTAO_V5
phase: "REFAC"
name: StateMachine (status_atualizacao + status_fase)
owner: SODA
status: DRAFT
---

# PRD — REFAC 01 — StateMachine (SQLite + Sheets + Schemas)

## Objetivo Atômico
Introduzir e estabilizar a nova máquina de estados do funil de ingestão, separando rigorosamente:

- **`status_atualizacao`** (controle humano/HITL)
- **`status_fase`** (rastro mecânico da máquina)

E consolidando os catálogos:

- Taxonomia de rejeição (`REJEITADO_*`) com efeito sistêmico de **`SHORT-CIRCUIT`**
- `categoria_arquitetural` com 10 ENUMs canônicos

## Contrato de I/O (Entrada e Saída estritas)

### Entradas (SSOT e Estado Local)
- Google Sheets (linha de repositório), no mínimo:
  - `repo_url`
  - `project_name` (quando aplicável)
  - `status_atualizacao`
  - `status_fase`
  - `proposta_original_resumo` (pode estar vazio)
  - `categoria_arquitetural` (pode estar vazio)
- SQLite (estado por repositório), no mínimo:
  - Identificador estável do repo (chave primária canônica do projeto)
  - Tabelas que armazenam heurísticas e artefatos (blobs) já existentes no funil atual

### Saídas (Contratos Persistidos)
- Sheets:
  - `status_atualizacao` aceita exclusivamente os valores definidos em [DAG_funil_ingestao_v5](file:///c:/Users/rosas/Dev_Projects/souls_mc/docs/dags/DAG_funil_ingestao_v5.md#L24-L55)
  - `status_fase` aceita exclusivamente os valores definidos em [DAG_funil_ingestao_v5](file:///c:/Users/rosas/Dev_Projects/souls_mc/docs/dags/DAG_funil_ingestao_v5.md#L57-L69)
  - `categoria_arquitetural` aceita exclusivamente os 10 ENUMs definidos em [DAG_funil_ingestao_v5](file:///c:/Users/rosas/Dev_Projects/souls_mc/docs/dags/DAG_funil_ingestao_v5.md#L71-L84)
- SQLite:
  - Persistência das duas colunas de status por repositório (mesmos valores do Sheets)
  - Capacidade de registrar `SHORT-CIRCUIT` e impedir reprocessamento

## Cenário de Falha (1 caso explícito)

### Falha: valor fora do ENUM em `categoria_arquitetural`
- Sintoma: `categoria_arquitetural` recebe um valor fora dos 10 permitidos (ex.: “MemoryRAG”, “UI”).
- Resposta obrigatória:
  - Não persistir a coluna inválida no Sheets.
  - Registrar falha por repositório (log/erro transacional) sem abortar o lote inteiro.
  - Manter `status_atualizacao` inalterado (para permitir correção HITL e retry).

## Restrições Bare-Metal (Linhas vermelhas)
- Proibido fundir controle humano e rastro de máquina em um único campo.
- Proibido criar novos valores de `status_fase` fora do catálogo v5.
- Proibido “corrigir” automaticamente uma rejeição humana: `REJEITADO_*` é definitivo e implica `SHORT-CIRCUIT`.
- Proibido qualquer dependência residente de Node.js/Python no Produto; execução deve permanecer Rust/Tokio + chamadas externas estritamente controladas.

## Definition of Done (exigindo TDD Scaffold)
- Existe Scaffold de testes cobrindo:
  - Validação de ENUMs (status e categoria) com casos válidos e inválidos.
  - Invariante HITL: a máquina não sobrescreve `status_atualizacao` quando este estiver em qualquer `APROVADO_*` ou `REJEITADO_*`.
  - Invariante de congelamento: após `REJEITADO_*`, a linha não é roteada para N1..N5.
- Migrações de schema (quando aplicáveis) são idempotentes e reversíveis (fail-closed em caso de drift).

