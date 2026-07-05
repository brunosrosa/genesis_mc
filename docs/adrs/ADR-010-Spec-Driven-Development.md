---
id: "ADR-010"
title: "ADR-010-Spec-Driven-Development"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Impõe o SDD e TDD como metodologia de desenvolvimento, bloqueando novas lógicas sem plano arquitetural aprovado."
---

# ADR-010-Spec-Driven-Development

## Status
Aceito (Ativo e Inegociável)

## Contexto
A prática irresponsável de engenharia estocástica de IA baseada em intuição rápida ("Vibe Coding") introduz códigos poluídos, dependências incoerentes e regressões catastróficas em repositórios complexos. IAs tendem a tentar corrigir erros gerando mais códigos falhos ("slop"), provocando loops de compilação infinitos e exaustão de contexto do modelo. No desenvolvimento de software Bare-Metal (Rust), a validação estrita do compilador e de testes de estresse deve preceder a escrita de qualquer lógica de negócios.

## Decisão
Implementar rigidamente a metodologia **Spec-Driven Development (SDD)** combinada a **TDD Forçado** em todo o ciclo de vida de mutação de código do SODA:
1. **O Plano Precede o Código:** Fica expressamente proibida a mutação direta de código-fonte sem a prévia especificação do design arquitetural (`docs/design.md`) em disco contendo diagramas Mermaid e definição do padrão Orchestrator-Worker. O Arquiteto Humano deve validar o design antes de prosseguir.
2. **Definição de Done e Scaffold (Tasks):** As tarefas são desfragmentadas de forma atômica no checklist `tasks.md`. Cada tarefa exige uma Definition of Done (DoD) com scaffold executável (testes unitários que falham primeiro - Red) antes de escrever a lógica funcional em Rust/TypeScript.
3. **Ciclo de TDD Atômico:** O desenvolvimento segue o fluxo clássico:
   - *Red:* Teste falho.
   - *Green:* Implementação mínima para passar no teste com Exit Code 0.
   - *Refactor:* Limpeza de avisos do compilador (`cargo clippy`) e melhorias estéticas.
4. **O Corretor Ralph Loop (Teto de 3 Tentativas):** Se o compilador ou os testes falharem, o core dispara automaticamente o Ralph Loop para auto-correção sob o teto rígido de no máximo **3 tentativas de compilação consecutivas**.
5. **Bloqueio Fail-Closed:** Se a falha persistir na 3ª tentativa, o agente de IA é compulsoriamente forçado a parar. O card correspondente é movido para a coluna "Bloqueado" no Kanban, a anomalia é gravada na telemetria e o controle é devolvido ao usuário.

## Consequências
- **Higiene e Qualidade de Código:** Erradicação total de lixo tecnológico e redução drástica de regressões lógicas no backend.
- **Raciocínio Estruturado:** O design do código é robusto e documentado de forma clara no próprio repositório antes mesmo de sua implementação real.
- **Resiliência contra Loops Infinitos:** Prevenção confiável de gastos inflacionados de API decorrentes de IAs que tentam corrigir repetidamente erros de compilação de forma redundante.

## Restrições Bare-Metal
- **Teto do Ralph Loop:** Limite rígido e imutável de **3 tentativas autônomas** consecutivas de correção.
- **Conformidade Clippy:** Todo código integrado deve passar compulsoriamente no linter nativo do Rust:
  `cargo clippy --all-targets --all-features -- -D warnings`
- **Validação de Testes:** Sucesso exige "exit code zero" em testes unitários locais.
