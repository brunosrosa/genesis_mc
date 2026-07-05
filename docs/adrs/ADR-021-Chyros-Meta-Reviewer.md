---
id: "ADR-021"
title: "ADR-021-Chyros-Meta-Reviewer"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Define o daemon em Rust encarregado de higienizar memória e auditar o código antes do merge."
---

# ADR-021-Chyros-Meta-Reviewer

## Status
Aceito (Ativo e Inegociável)

## Contexto
Sessões intensas de ideação criativa e desenvolvimento causadas por picos de hiperfoco (comuns em usuários neurodivergentes 2e/TDAH) despejam massas de dados caóticas no SODA: logs de erros transitórios do Ralph Loop, links duplicados, arquivos temporários de rascunho de análises estruturais (IterResearch) e históricos extensos de processos MDP. Manter esses dados voláteis indefinidamente em sua forma crua inunda as bases transacionais e vetoriais, asfixiando a capacidade de busca semântica em LanceDB e consumindo preciosa memória RAM no host.

## Decisão
Estabelecer o **Chyros Daemon** como o **Meta-Reviewer e Consolidador de Memória** oficial do ecossistema SODA, operando em background assíncrono na CPU i9:
1. **O Despertar na Inatividade:** O Chyros Daemon atua de forma totalmente autônoma em períodos de inatividade do Windows ou durante a madrugada, disparado por watchdogs nativos de energia do sistema operacional.
2. **Consolidação Ontológica e Poda Sináptica:** O daemon varre a Tríade de Memória (SQLite FTS5, LanceDB e LadybugDB) executando as seguintes ações de higiene:
   - Deduplica entidades semânticas redundantes e consolida conexões de grafos.
   - Poda links mortos e logs obsoletos de depurações frustradas do compilador.
   - Transpila massas de texto de rascunhos de IterResearch/MDP em resumos densos estruturados, arquivando os originais em compressão delta (*gitoxide*) e expurgando as duplicatas da L3 ativa.
3. **Preservação de Outliers e SSOT:** A rotina do Chyros Daemon é estritamente impedida de tocar ou resumir arquivos catalogados com a tag `STABLE` (regras e decisões canônicas imutáveis), resguardando a integridade das fundações do SODA.

## Consequências
- **Aceleração da Busca Semântica:** A L3 vetorial e o grafo LadybugDB permanecem enxutos, precisos e com alta velocidade de resposta sub-milissegundo.
- **Hiperfoco Matinal:** O usuário inicia a jornada matinal de trabalho sob um ambiente purificado, limpo e com as memórias de rascunhos da véspera consolidadas, reduzindo a paralisia de análise por desorganização visual.
- **Economia de Recursos:** Desfragmentação do SQLite (`VACUUM INTO` assíncrono) para prevenir inflação no disco.

## Restrições Bare-Metal
- **Teto Térmico Noturno:** O Chyros Daemon deve operar limitado ao consumo máximo de **40% da CPU i9** sem ativar dGPU de forma contínua, preservando o resfriamento passivo do host.
- **Segurança Transacional:** A consolidação de dados utiliza travas de exclusão mútua (*Mutex* assíncrono do Tokio) para evitar colisões com sessões de usuários ativas noturnas.
- **Bypass de Escrita:** Qualquer intervenção direta do usuário cessa a rotina do Chyros imediatamente em $< 100ms$ (evicção), salvando o snapshot atual de forma segura.
- **Desfragmentação LanceDB em Baixa Prioridade:** Rotinas de **compactação contínua de blocos Lance** e **ordenação vetorial de índices** devem executar em threads de baixíssima prioridade durante a madrugada, garantindo desfragmentação e saúde de I/O do SSD do usuário.
