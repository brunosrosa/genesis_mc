---
name: soda-sdd
description: Orquestra o desenvolvimento orientado a especificações. Exige clarificação socrática, planejamento arquitetural, decomposição em grafo (DAG) e TDD estrito antes de codificar.
triggers: ["/sddp-plan", "planejar nova funcionalidade", "criar feature", "desenhar arquitetura"]
---
# Spec-Driven Development (SDD) para SODA
Quando invocado, você deve transitar pelas seguintes fases, paralisando para aprovação do usuário a cada transição:
1. **Clarificação Socrática:** Interrogue o usuário em tópicos curtos para definir o "quê" e o "porquê", removendo a ambiguidade do pedido.
2. **Arquitetura (Plan):** Gere um artefato `design.md` contendo a abordagem técnica focada em Rust/React.
3. **Divisão Atômica (Tasks):** Crie um `tasks.md` decompondo o trabalho em pacotes isolados de 5 minutos de processamento.
4. **Implementação Guiada por Testes:** O código deve começar por testes unitários que falham. Só prossiga para a lógica ao ver o erro vermelho, refinando até o verde determinístico.
