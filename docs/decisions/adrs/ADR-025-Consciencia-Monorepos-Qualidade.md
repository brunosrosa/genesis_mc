---
id: "ADR-025"
title: "ADR-025-Consciencia-Monorepos-Qualidade"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Define a governança e estratégias de análise estática direcionadas a monorepos e estruturas complexas."
---

# ADR-025-Consciencia-Monorepos-Qualidade

## Status
Aceito (Ativo e Inegociável)

## Contexto
Durante a execução do Harvester Nativo (Fase 0 - V6.0), a auditoria revelou que o orquestrador tolerava falhas silenciosas ("Fail-Soft") em linters SAST pesados, como `govulncheck` (Go) e `sobelow` (Elixir). A execução ingênua dos comandos na raiz bruta do repositório falhava catastroficamente em topologias de **Monorepo**, onde os manifestos de dependência (`go.mod`, `mix.exs`) residem em subdiretórios aninhados. Em vez de investigar a topologia real, o extrator falhava, devolvia 0 bytes e mascarava a ineficiência com um falso "sucesso" na pipeline, enviando payloads corrompidos ou incompletos para as Lentes B e C.

## Decisão
Fica decretada a lei de **Qualidade 100/100** e a **Consciência Obrigatória de Monorepos** para o Motor de Extração SOULS:

1. **A Morte do Fail-Soft Mascarado:** Tolerância ZERO a truncamento cego ou falhas silenciosas de extração por "ignorância topológica". Se o linter não conseguir executar por não achar a raiz lógica do ecossistema, o erro DEVE ser tratado e o alvo correto deve ser localizado. O payload gravado deve ser a extração tática 10/10.
2. **Consciência de Monorepo (Topology-Aware Routing):** Os *sidecars* de linters (SAST) são estritamente proibidos de disparar comandos às cegas na raiz do projeto clonado. O extrator em Rust deve realizar um "Pre-flight Check", buscando a localização física exata dos manifestos raiz (ex: localização do `go.mod` principal ou `mix.exs`) e atuar com base nesse diretório (`cwd`).
3. **Alvos Recursivos Explícitos:** Comandos SAST, como `govulncheck`, devem ser instruídos a usar escopos de varredura recursivos explícitos (ex: executar `./...` em vez de invocar cegamente a pasta raiz), garantindo que a árvore completa de pacotes do monorepo seja auditada em profundidade O(N).

## Consequências
* **Zero Ponto Cego:** A auditoria SAST (Blob 06 e Blob 08) enxergará vulnerabilidades em todos os microserviços e sub-pacotes contidos no repositório.
* **Integridade do FinOps:** A Fase 1.5 não desperdiçará tokens lendo logs de erro do tipo *"go.mod not found"* gerados por execução em pasta errada. 
* **Complexidade Cíclica Mitigada:** O motor Rust absorve a carga lógica de descobrir onde estão os arquivos de configuração, blindando a LLM de ter que adivinhar a estrutura do monorepo.
