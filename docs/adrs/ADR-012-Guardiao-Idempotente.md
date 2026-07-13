---
id: "ADR-012"
title: "ADR-012-Guardiao-Idempotente"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Estabelece o guardião de transição para o Google Sheets, garantindo idempotência e re-runs seguros de lotes."
---

# ADR-012-Guardiao-Idempotente

## Status
Aceito (Ativo e Inegociável)

## Contexto
Durante longas sessões de desenvolvimento com múltiplos agentes e subagentes operando concorrentemente em Shadow Workspaces, modificações manuais do usuário ou falhas de sincronia podem introduzir desvios físicos ("drifts") ocultos em arquivos fundamentais de arquitetura. Utilizar inteligência artificial estocástica na GPU para monitorar desvios e auditar se os arquivos do disco condizem com os hashes oficiais seria absurdamente ineficiente, caro e passível de falsos negativos provocados por alucinação.

## Decisão
Implementar a arquitetura do **Guardião Idempotente** como um módulo nativo de verificação determinística **Zero-AI** embutido na esteira Rust:
1. **Auditoria por Hash Criptográfico (SHA-256):** O monitoramento de desvios de arquivos no workspace descarta o uso de processamento generativo. O core Rust calcula continuamente hashes SHA-256 locais das assinaturas da AST e das estruturas críticas documentadas.
2. **Sincronia Determinística via Gitoxide e API GitHub:** O Guardião compara os hashes locais em tempo real com a árvore de commits e logs de transações oficiais mantidos em disco e no repositório remoto via chamadas assíncronas de baixo nível do `gitoxide` local e endpoints da API do GitHub.
3. **Disjuntor de Drift Concorrente:** Se o Guardião detectar qualquer modificação física arbitrária realizada à margem do protocolo de governança BMAD (um drift estrutural), a esteira de montagem de IA é imediatamente paralisada (*Fail-Closed*). O sistema congela a Agent Inbox e barra qualquer consolidação de Rebase Semântico até que a integridade física do repositório seja reestabelecida ou aprovada manualmente pelo usuário.

## Consequências
- **Segurança de Código Absoluta:** Garantia termodinâmica e mecânica de que nenhuma alteração acidental ou scripts externos maliciosos corrompam silenciosamente as fundações Bare-Metal do SODA.
- **Custo Computacional Irrisório:** Auditorias de integridade rodam em tempo sub-milissegundo consumindo frações de ciclo da CPU i9, com custo financeiro US$ 0,00 e VRAM intocada.
- **Previsibilidade Operacional:** Conflitos e desalinhamentos entre instâncias do repositório são detectados e tratados antes que ocorram erros de compilação confusos.

## Restrições Bare-Metal
- **Latência de Auditoria do Guardião:** O cálculo incremental de integridade SHA-256 de um arquivo de desenvolvimento pelo core Rust deve rodar em menos de **1ms**.
- **Carga de CPU em Background:** O watchdog de monitoramento estático na CPU principal consome no máximo **1% de utilização** de background, priorizando operações determinísticas de baixo nível.
- **Tratamento de Drift:** Mutações sem hash correspondente registrado no FrankenSQLite do cérebro são travadas instantaneamente por tipagem (*Zero-Trust*).
- **I/O e Criptografia fora do Event Loop:** Operações do **gitoxide** (snapshots, verificação de drift) e cálculos criptográficos pesados (**SHA-256**) são proibidos na thread principal do Tokio; devem ser descarregados para `tokio::task::spawn_blocking` ou *background workers* dedicados comunicando-se via **MPSC**, prevenindo starvation do Event Loop.
