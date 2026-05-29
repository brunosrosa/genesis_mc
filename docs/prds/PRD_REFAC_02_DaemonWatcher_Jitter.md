---
prj: SODA
canon: SODA_INGESTAO_V5
phase: "N0"
name: Daemon Watcher (Jitter + Rate Limit Guard)
owner: SODA
status: DRAFT
---

# PRD — REFAC 02 — Daemon Watcher + Jitter (Amortecedor de Rede)

## Objetivo Atômico
Criar a engrenagem contínua do Olheiro (N0) que varre a planilha e dispara N1..N5 de forma assíncrona e idempotente, aplicando amortecimento de rede (jitter + backoff) para:

- Evitar rate limit do GitHub
- Evitar bursts no Google Sheets
- Preservar FinOps (custo e tráfego previsíveis)

## Contrato de I/O (Entrada e Saída estritas)

### Entradas
- SSOT: Google Sheets (somente as colunas necessárias para roteamento):
  - `repo_url`
  - `project_name` (quando já conhecido)
  - `status_atualizacao`
  - `status_fase`
- Configuração local:
  - intervalo base de varredura (ms)
  - faixa de jitter (ms)
  - limites de concorrência (máximo de linhas processadas em paralelo)
  - limites de retry por repositório (kill-switch)

### Saídas
- Despacho determinístico por linha (roteamento):
  - `status_atualizacao` vazio → N1
  - `INICIAR_TRIAGEM` → N2
  - `APROVADO_PARA_HARVESTER` → N3
  - `APROVADO_PARA_ENXAME` → N4
  - `APROVADO_DEEP_COMPONENTS_ANALYSIS` → N5
  - `REJEITADO_*` → rotina de SHORT-CIRCUIT (cleanup + congelamento)
- Telemetria mínima por rodada (contadores):
  - linhas_inspecionadas
  - linhas_roteadas_por_no (N1..N5 + SHORT-CIRCUIT)
  - erros_sheets
  - erros_github (quando aplicável)

## Cenário de Falha (1 caso explícito)

### Falha: GitHub responde 429 (Rate Limit)
- Sintoma: múltiplas linhas acionam N1 em sequência e a API retorna 429.
- Resposta obrigatória:
  - Backoff exponencial com jitter (por host) antes de novas tentativas.
  - Fail-soft por linha: o restante do lote continua, sem abortar o loop global.
  - Sem sobrescrever `status_atualizacao` para “erro”: preservar HITL e permitir retentativa automática.

## Restrições Bare-Metal (Linhas vermelhas)
- Proibido rodar como “tempestade”: nenhuma rodada pode disparar chamadas em burst sem jitter.
- Proibido bloquear a thread principal: o loop deve operar assíncrono (não travar UI/Tauri).
- Proibido “varrer tudo”: leitura do Sheets deve ser seletiva (colunas mínimas para roteamento).
- Proibido iniciar fases que não sejam compatíveis com o comando humano presente em `status_atualizacao`.

## Definition of Done (exigindo TDD Scaffold)
- Existe Scaffold de testes cobrindo:
  - Roteamento correto N0 → N1..N5 para todos os valores do catálogo de `status_atualizacao`.
  - Garantia de jitter: duas execuções consecutivas não geram padrões determinísticos de burst.
  - Fail-soft: erro em uma linha não interrompe o processamento das demais.
- O Olheiro consegue rodar em modo contínuo (daemon) e em modo “rodada única” (para auditoria HITL).

