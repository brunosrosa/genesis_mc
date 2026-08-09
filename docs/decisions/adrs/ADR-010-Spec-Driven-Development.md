---
id: "ADR-010"
title: "ADR-010-Spec-Driven-Development"
version: 2.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Impõe o SDD e TDD como metodologia constitucional, exigindo a cascata documental atômica de 4 vias (REQUIREMENTS.md, DESIGN.md, TASKS.md, TEST_SPECS.md) e a trava Fail-Closed do Ralph Loop no Shadow Workspace (snapsafe)."
---

# ADR-010: Pipeline Spec-Driven Development (SDD), Cascata 4-Vias e Ralph Loop

## Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SOULS V6)

## Contexto Técnico e a Erradicação do Vibe Coding
A prática estocástica de engenharia de software baseada em intuição rápida ("Vibe Coding") introduz débito de código ("slop"), regressões silenciosas e desalinhamento de intenção em sistemas complexos. Agentes de IA sem amarras tendem a corrigir falhas gerando mutações cegas e redundantes, provocando exaustão de contexto e loops infinitos de compilação.
No ecossistema SOULS Bare-Metal (Rust), a especificação declarativa estrita e a validação por testes automatizados devem obrigatoriamente preceder a escrita de qualquer linha de lógica funcional.

## Decisão Arquitetural (Macro-Pipeline Declarativo e Micro-TDD)
Fica decretado a obrigatoriedade do **Spec-Driven Development (SDD)** unificado ao **Test-Driven Development (TDD)** através dos seguintes pilares constitucionais:

### 1. Cascata Documental Atômica de 4 Vias (SPEC-014)
É terminantemente proibida qualquer mutação de código-fonte sem que a cascata documental de 4 vias esteja gerada, validada e versionada fisicamente em disco na pasta de trabalho ativa:
1.  **`REQUIREMENTS.md` (HITL Gate #1):** Documento de requisitos, escopo, regras de negócio e não-objetivos. Atua como o Portão nº 1 de validação e assinatura do Operador Humano antes de avançar para a arquitetura técnica.
2.  **`DESIGN.md`:** Especificação técnica detalhada, diagramas Mermaid, structs/enums Rust, schemas de banco e contratos de API. Auditado por Peer-Review em sessão isolada.
3.  **`TASKS.md`:** Matriz de tarefas atômicas e independentes organizadas como Grafo Acíclico Dirigido (DAG). Cada tarefa possui Definition of Done (DoD) com scaffold executável.
4.  **`TEST_SPECS.md`:** Especificação exata dos cenários de teste, entradas, saídas esperadas e critérios de aceite funcional.

### 2. Isolamento Físico em Shadow Workspace (`snapsafe`)
*   Toda execução de mutação e compilação de código ocorre em um **Shadow Workspace** isolado.
*   A instanciação é realizada em tempo constante $\mathcal{O}(1)$ via links físicos rígidos (*snapsafe*), consumindo 0 bytes adicionais do disco host e garantindo a preservação da branch principal até a aprovação final.

### 3. Ciclo Micro-TDD Atômico (Red-Green-Refactor)
*   **Fase RED:** O Worker escreve primeiramente o teste especificado em `TEST_SPECS.md`, comprovando a falha inicial no compilador/runner.
*   **Fase GREEN:** Escreve a lógica funcional mínima necessária para obter aprovação com Exit Code 0 no terminal.
*   **Fase REFACTOR:** Limpa avisos do compilador e conformidade com `cargo clippy --all-targets --all-features -- -D warnings`.

### 4. O Corretor Autônomo Ralph Loop (Teto de 3 Tentativas e Fail-Closed)
*   Se o compilador Rust ou a suíte de testes falhar durante a fase GREEN/REFACTOR, a engine aciona autonomamente o **Ralph Loop**.
*   O erro do terminal é reinjetado no prompt do Worker para auto-correção sob o limite rígido e imutável de **no máximo 3 tentativas síncronas consecutivas de compilação**.
*   **Status FAIL-CLOSED:** Se a falha persistir na 3ª tentativa, o ciclo é paralisado imediatamente. As alterações na branch do Shadow Workspace são travadas no git, a anomalia é gravada no log de telemetria e o controle é devolvido compulsoriamente ao operador humano sem aplicar rebase no disco físico principal.

## Consequências Operacionais e Defesa contra o Slop
- **Higiene e Rigor:** Erradicação total de lixo tecnológico no backend e garantia de que todo código em produção atende a uma especificação formal pré-aprovada.
- **Eficiência FinOps:** A trava Fail-Closed na 3ª tentativa impede desperdício de tokens de API e evita picos de aquecimento da GPU causados por loops de correção redundantes.
- **Rastrabilidade Total:** O ciclo documental de 4 vias em disco fornece auditoria completa da evolução arquitetural do sistema.

## Restrições Bare-Metal
- **Cascata Obrigatória:** `REQUIREMENTS.md` (HITL Gate #1) -> `DESIGN.md` -> `TASKS.md` -> `TEST_SPECS.md`.
- **Trava do Ralph Loop:** Máximo de 3 retentativas locais síncronas antes do status FAIL-CLOSED.
- **Conformidade Clippy:** Requer Exit Code 0 estrito em `cargo clippy --all-targets --all-features -- -D warnings`.
