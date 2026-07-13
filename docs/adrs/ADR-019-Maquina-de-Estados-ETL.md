---
id: "ADR-019"
title: "ADR-019-Maquina-de-Estados-ETL"
version: 2.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Formaliza a máquina de estados (Enums) do SQLite e o fluxo das 4 Fases (Harvester a Sheets) em Rust para governança das 85 colunas atômicas."
---

### ADR-019: Máquina de Estados do ETL Cognitivo e Governança de Enums

#### Status
Aceito (Ativo, Inegociável e Fundacional para a Fábrica SODA V4)

#### Contexto Técnico
A extração de dados e a análise profunda de repositórios operam sob condições hostis (falhas de rede, limites de taxa de APIs, restrições de memória). Para garantir a integridade da "Matriz Mestre" (SSOT) de 85 colunas (incluindo campos de governança como `status_atualizacao`, `status_fase` e `embargo_status`), o sistema exige checkpoints precisos. A documentação anterior estava defasada em relação aos Enums físicos gravados no banco de dados SQLite (`soda_heuristic_vault.db`). A arquitetura exige que a "Lei" (documentação) reflita estritamente o "Metal" (o código Rust implementado).

#### Decisão Arquitetural (A Máquina de Estados Físicos)
O controle transacional do pipeline e o *Self-Healing* do sistema obedecem exclusivamente à taxonomia de Enums abaixo, extraída do ambiente físico em Rust:

**Módulo 1: O Pipeline de Fases (Core Rust)**
O fluxo baseia-se em 4 fases estritas executadas pelo backend em Rust nativo:
*   **Fase 0 (Harvester / f0_harvester_cli):** Extração cega de código-fonte (AST/README).
*   **Fase 1 (Distiller / f1_distiller_cli):** Desidratação do contexto.
*   **Fase 2 (Swarm / f2_swarm_cli):** Avaliação pelas Lentes Cognitivas.
*   **Fase 3 & 4 (Synthesizer & SSOT Injector / f3_synthesizer_cli):** Síntese final com decodificação restrita e Carga Atômica no Google Sheets via memória RAM O(1).

**Módulo 2: Dicionário de Estados Estritos (Tabela `repositorios`)**
A governança da tabela principal obedece obrigatoriamente a:
*   `PENDENTE` / `APROVADO_PARA_HARVESTER`: Fila inicial de processamento.
*   `F0_OK` / `DEGRADADO_F0` / `ERRO_F0`: Checkpoints de integridade da Fase 0 (Harvester).
*   `FASE_2_RUNNING` / `F2_OK` / `ERRO_F2`: Checkpoints do Enxame Cognitivo.
*   `CONCLUIDO` / `ERRO_FASE_4`: Status da injeção SSOT (Sheets).

**Módulo 3: Status de Integração (Tabela `repo_heuristics`)**
*   **status_atualizacao:** Governa o ciclo de vida longo (`CONCLUIDO_AGUARDANDO`, `PENDENTE_FASE_0`, `REJEITADO_DESCARTE`, `REJEITADO_LINE_TIGHT`).
*   **status_fase:** Governa a sincronia fina com a nuvem (`FASE_0_HARVESTER_OK`, `FASE_3_SYNTHESIZER_OK`, `FASE_4_SHEETS_UPDATED`, `FASE_4_CLOUD_FAILED`).
*   **classificacao_terminal:** O veredito executivo arquitetural (`STACK_CORE_PLANO_A1`, `STACK_CORE_PLANO_A2`, `STACK_CORE_PLANO_B`, `INTEGRATE_AS_COMPONENT`, `ABSORB_PARTIALLY`, `ABSORB_CONCEPT`, `USE_AS_INSPIRATION_ONLY`, `REJECT`, `SHORT-CIRCUIT`, `UNKNOWN`).

#### Consequências Operacionais
*   **Positivas:** A Máquina de Estados agora reflete 100% da realidade do código em Rust. Se o agente da IDE precisar consultar como gerenciar o estado de um repositório, ele usará nomenclaturas que o compilador e o banco de dados já conhecem, evitando alucinações.
*   **Negativas:** Qualquer adição de uma nova etapa de processamento exigirá uma migração formal no SQLite e a atualização rigorosa deste documento para manter a paridade.
