---
id: "ADR-006"
title: "ADR-006-SSOT-Sheets"
version: 1.0
status: Ativo_Inegociavel
epic: "FinOps"
description: "Adota o Google Sheets como única fonte de verdade bidirecional para logs, canibalização e governança do projeto."
---

# ADR-006-SSOT-Sheets

## Status
Aceito (Ativo e Inegociável)

## Contexto
O acompanhamento do pipeline de canibalização, os logs analíticos de refatorações de código e a auditoria de viabilidade de engenharia geram massas de dados altamente estruturadas. Armazenar esses dados unicamente em arquivos JSON locais fragmentados ou de forma proprietária em bancos de dados em disco impede que o Arquiteto Humano possua uma visão holística e imediata do throughput sistêmico. Também dificulta auditorias de conformidade de código e acompanhamento ágil para mentes neurodivergentes que necessitam de forte ancoragem visual externa.

## Decisão
Fica formalmente decidido estabelecer uma planilha do **Google Sheets** estruturada como a **Única Fonte de Verdade (SSOT) analítica** para a esteira de canibalização e governança do SODA:
1. **O Barramento Universal:** A planilha mestre opera de forma bidirecional integrada ao core Rust através de um servidor MCP customizado (`webcrawl-mcp` / `sheets-mcp`).
2. **Esquema de Alta Densidade (82 Colunas):** A tabela mestre de ingestão e análise adota rigorosamente uma especificação fixa e imutável de **82 colunas de metadados e telemetria** cruzadas, mapeando:
   - Identificadores atômicos (hashes, URIs e IDs de arquivos).
   - Métricas de complexidade estática de código (AST, linhas totais e nível de herança).
   - Métricas de viabilidade cognitiva e FinOps (tokens consumidos local vs. nuvem, custos operacionais e score de integridade do compilador).
   - Tríade de tags de governança (BLAST_RADIUS, do_action e red_lines).
3. **Escrita Idempotente em Lote (Batching):** IAs agênticas estão estritamente proibidas de efetuar chamadas unitárias à API de planilhas. A escrita deve ocorrer na CPU, gerando um payload JSON que é consolidado localmente e despachado em lotes compactados (`batchUpdate`) a cada ciclo de finalização de fase.
4. **Schema-Guided Reasoning (SGR):** O preenchimento e leitura lógica do Sheets operam governados por esquemas estruturados locais (`DATABASE_SCHEMA_DIC.csv`) validados estaticamente, prevenindo a injeção de colunas flutuantes causadoras de layout-shifts ou quebras de parsing.

## Consequências
- **Consistência de Estado:** Todo o andamento do projeto, os dados de faturamento (FinOps) e o status das fases são atualizados automaticamente em tempo real em um painel compartilhado na nuvem.
- **Rigor Agêntico:** Agentes autônomos operando na esteira de canibalização possuem regras claras sobre o que atualizar no Sheets antes de considerar uma tarefa concluída.
- **Mitigação de Taxas da API:** A restrição de batching local reduz o tráfego e previne bloqueios por taxa limite (*Rate Limiting*) das cotas da nuvem do Google One.

## Restrições Bare-Metal
- **Teto Rígido de Colunas:** A planilha mestre é imutável em seu layout horizontal de **82 colunas**, exigindo auditoria de schema local antes de disparar atualizações.
- **Latência de Flush Local:** O buffer de telemetria retido localmente na RAM transiente deve efetuar o dispatch de gravação para a API do Google Sheets a cada **15 iterações** de ações agênticas ou ao final de cada fase operacional.
