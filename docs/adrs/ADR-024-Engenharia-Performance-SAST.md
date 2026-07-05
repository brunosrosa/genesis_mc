---
id: "ADR-024"
title: "ADR-024-Engenharia-Performance-SAST"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Implementa travas de performance no pipeline de análise estática de código (SAST) para evitar saturação de CPU/Ramdisk."
---

# ADR-024-Engenharia-Performance-SAST

## Status
Aceito (Ativo e Inegociável)

## Contexto
O Harvester da Fase 0 passou a operar sobre monorepos, sidecars heterogêneos e regras SAST locais em modo air-gapped. Essa expansão elevou o risco físico de asfixia térmica, timeouts cegos, parsing infinito em arquivos densos e perda de rastreabilidade de Supply Chain quando filtros agressivos escondem manifestos e lockfiles. O ecossistema SODA precisa congelar essas decisões no Cânone antes de qualquer mutação futura no Rust ou criação de novas lâminas de análise.

## Decisão
Ficam decretadas, para toda extração SAST e para qualquer futura ferramenta CLI de análise do SODA, as seguintes 4 Leis Duras de Extração:

1. **A. O Fim do Timeout Cego (O Bisturi Adaptativo):**
   Ferramentas orientadas por regras, como OpenGrep, devem escalar o tempo dinamicamente por arquivo e por regra. O uso da flag `--allow-rule-timeout-control` torna-se obrigatório sempre que disponível, delegando o limite ao tamanho matemático do arquivo, da regra e do custo real do match. Timeout fixo e cego é proibido como estratégia primária.

2. **B. Escudo de Cadeia de Suprimentos (A Regra do Lockfile):**
   Filtros de exclusão podem ignorar sumariamente lógica descartável de teste e simulação, como `tests/` e `**/mocks/*`, para reduzir ruído operacional. Porém, é estritamente proibido ignorar manifestos e lockfiles de cadeia de suprimentos, incluindo `Cargo.lock`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `go.sum`, `poetry.lock`, `Pipfile.lock`, `mix.lock` e equivalentes. O rastreio de Supply Chain é superior à conveniência do scan.

3. **C. Fobia de Código Minificado:**
   A barreira do infinito em parsing AST e scanning textual de blobs densos deve ser eliminada preventivamente. Sempre que a ferramenta suportar, a flag `--exclude-minified-files` torna-se obrigatória. Na ausência da flag, o agente deve reproduzir a heurística: arquivos com menos de 7% de espaço em branco não entram na memória de análise.

4. **D. Higiene de I/O em Tempo Real:**
   Qualquer sidecar que acione compilações agressivas, geração transitória de artefatos ou expansão volumétrica de cache local, como `cargo clippy`, deve conter rotina explícita de limpeza do cache de build (`target/` ou equivalente) imediatamente após o uso. O objetivo é proteger o Ramdisk, o SSD e a sandbox contra colapso de espaço e entropia residual.

## Consequências
- **Mais Determinismo Termodinâmico:** scans deixam de depender de timeouts arbitrários e passam a respeitar o custo real do arquivo e da regra.
- **Preservação da Supply Chain:** lockfiles continuam auditáveis mesmo quando o restante do ruído de testes e mocks é amputado.
- **Redução de Loops Patológicos:** arquivos minificados deixam de contaminar AST e regex engines com curvas exponenciais de custo.
- **Higiene de Disco e Sandbox:** sidecars compilatórios passam a devolver espaço imediatamente após o uso, reduzindo falhas por `os error 112` e saturação do sandbox.
- **Padronização do Cânone:** toda nova CLI ou lâmina SAST futura deve nascer já obedecendo as mesmas 4 leis.

## Restrições Bare-Metal
- **Fail-Closed de Lockfiles:** nenhuma regra de exclusão pode amputar lockfiles ou manifestos de pacote por conveniência de performance.
- **Sem Timeout Cego Global:** timeouts fixos só sobrevivem como piso de segurança; a decisão primária deve ser adaptativa e local à regra/arquivo.
- **Sem Parsing de Minificados:** arquivos abaixo do limiar de 7% de espaço em branco devem ser descartados antes de entrar no pipeline de parsing/scan.
- **Limpeza Pós-Uso Obrigatória:** sidecars compilatórios que materializam `target/`, caches locais ou build dirs temporários devem limpá-los ao final, ainda dentro da rotina de teardown.
