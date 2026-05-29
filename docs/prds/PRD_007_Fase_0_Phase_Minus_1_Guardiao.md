---
prj: SODA
canon: SODA_ETL_V3
phase: "-1"
name: Guardiao (Daemon Chyros)
owner: SODA
status: DRAFT
---

# PRD — Fase -1 (Guardião / Daemon Chyros)

## Objetivo
Manter a SSOT (Google Sheets) alinhada com a realidade do mundo (versão online mais recente), detectando drift de versão por repositório e sinalizando necessidade de ação humana via `status_atualizacao`.

## Regra HITL (Inquebrável)
O Guardião NUNCA aciona automaticamente as Fases 0 a 4. Ele apenas:
- Detecta drift
- Atualiza campos de status
- Publica evidências (versão online + timestamps)

## Entradas / Fontes de Verdade
- Google Sheets (aba `MASTER_SOLUTIONS`)
  - `repo_url`
  - `repo_version`
  - `ultima_versao_online`
  - `status_atualizacao`
  - `status_fase`
- GitHub API
  - `GET /repos/{owner}/{repo}/releases/latest` (preferencial)
  - Fallback: `GET /repos/{owner}/{repo}/tags` ou `GET /repos/{owner}/{repo}/releases` (opcional; somente se `releases/latest` não existir)

## Saídas
- Atualização atômica da linha do repositório no Sheets:
  - `ultima_versao_online`: preenchida/atualizada com o `tag_name` do GitHub quando detectável
  - `status_atualizacao`: setado para `"DESATUALIZADA"` quando `repo_version != ultima_versao_online` (após normalização)
  - `status_atualizacao`: mantido como `"CONCLUIDO"` (ou `"OK"`, conforme canon) quando não houver drift
  - `status_fase`: não deve ser alterado (apenas leitura) — a fase refletida é de responsabilidade do pipeline de análise

## Critérios de Drift (Versões)
### Normalização
- Remover prefixo `v` (ex.: `v1.2.3` → `1.2.3`)
- Trim de espaços
- Aceitar semver e tags de release comuns (ex.: `1.2.3`, `1.2.3-beta.1`)

### Regras
- Se não existir `repo_version` ou `repo_url`, o Guardião ignora a linha (não falha o job)
- Se GitHub não retornar release/tag, não altera `ultima_versao_online` (mas pode registrar evidência em log)
- Drift verdadeiro quando:
  - `repo_version_normalizada` é diferente de `ultima_versao_online_normalizada`

## Orquestração (Cronjob)
### Agendamento
- Cron local (Tokio + interval) ou execução pontual via CLI (modo recomendado para HITL)

### Fluxo por rodada
1. Ler a planilha (somente as colunas necessárias)
2. Para cada linha:
   - Resolver owner/repo por parsing do `repo_url`
   - Consultar GitHub `releases/latest`
   - Calcular drift
3. Para cada drift detectado:
   - Montar payload de update para a linha correspondente
   - Executar `batch_update_cells` com range atômico da linha

## Restrições e Segurança
- Rate limiting agressivo (GitHub e Sheets)
- Retries com backoff e jitter (falhas transitórias)
- Proibido logar tokens/segredos
- Falhas em um repo NÃO podem abortar a rodada inteira (fail-soft por linha)

## Observabilidade
- Métricas por rodada:
  - repos_inspecionados
  - repos_com_drift
  - repos_sem_release
  - erros_github
  - erros_sheets

## Definition of Done
- Rodada completa atualiza `status_atualizacao="DESATUALIZADA"` para repos com drift real
- Rodada não dispara nenhuma fase do pipeline (verificado por ausência de chamadas aos bins F0..F4)
- Execução idempotente (rodar 2x não altera linhas sem drift)
