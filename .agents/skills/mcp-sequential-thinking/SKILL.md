---
name: mcp-sequential-thinking
description: O Freio de Mão Cognitivo do Antigravity IDE. Delega raciocínio ao MCP 'sequentialthinking' com Hard-Limit de 5 iterações (FinOps). Impõe a Tríade (Regular, Revision, Branching), Arquitetura Fail-Closed L7, e entrega um DAG de Roteamento Tagueado (local_slm vs cloud) para o ParetoBandit.
triggers: ["mcp-sequential-thinking", "raciocinar passo a passo", "pensar", "analisar problema", "planejar refatoração", "sequential thinking", "desdobrar lógica", "freio de mão"]
---

### skill: MCP Sequential Thinking (O Freio de Mão e Orquestrador DAG V5.0)

#### Goal
Atuar como o regulador de cadência cognitiva, filtro de viabilidade e Orquestrador "Cloud Brain" do Antigravity IDE. O objetivo inegociável é erradicar o *Vibe Coding* e a Sicofania. Para proteger a VRAM e o FinOps, você DEVE delegar o raciocínio profundo ao servidor MCP usando os três construtos nativos de pensamento (Regular, Revision, Branching) sob um Hard-Limit de 5 iterações. A saída final NUNCA é código bruto, mas sim um Grafo Acíclico Dirigido (DAG) tagueado para o roteamento do ParetoBandit, blindado por uma política estrita de Fail-Closed em caso de falhas no servidor.

#### Instructions
Sempre que se deparar com uma arquitetura nova, um bug complexo, ou for instruído a "planejar/raciocinar", execute esta máquina de estados:

1. **A Trava de Geração (Zero-Code):**
   * Você está expressamente PROIBIDO de gerar código-fonte textual de implementação na sua primeira resposta.

2. **Invocação MCP e Proteção Fail-Closed:**
   * Acione OBRIGATORIAMENTE a ferramenta de nome exato `sequentialthinking`.
   * **Lei do Fail-Closed L7:** Se o servidor MCP retornar erro, timeout ou indisponibilidade, você está SUMARIAMENTE PROIBIDO de continuar o raciocínio por conta própria. Paralise a propagação, aborte a tarefa e reporte ao usuário: *"Falha no Sequential Thinking. Fail-Closed acionado para evitar alucinações."*

3. **A Tríade de Construtos (Max 5 Iterações):**
   * Emita os pensamentos de forma iterativa, aplicando obrigatoriamente a mecânica de Falsificação Coercitiva (Free-MAD) através destes construtos:
     * **Regular Thoughts:** Decomposição do problema e proposição da tese.
     * **Revision Thoughts:** Avaliação retroativa das suas premissas. Aplique as Leis Duras do SODA (Zero-Copy? Menos de 6GB VRAM? Node.js?) para tentar destruir a tese original e corrigir o curso (`isRevision: true`).
     * **Branching Thoughts:** Se o impasse técnico persistir, gere pensamentos de ramificação para explorar abordagens arquiteturais antagônicas sem corromper a árvore central.
   * **HARD-LIMIT FINOPS:** Orçamento inegociável de 5 pensamentos (`thoughtNumber: 5`). No 5º pensamento, force o encerramento do laço: `nextThoughtNeeded: false`.

4. **Trabalho Invisível:**
   * A "bagunça" do vai-e-vem do JSON-RPC não deve ser renderizada no chat. Fica restrita aos logs em background.

5. **O Handoff Operacional (Geração do DAG):**
   * Ao finalizar as iterações, devolva no Canvas EXCLUSIVAMENTE a Síntese Arquitetural sob o formato de um **Grafo Acíclico Dirigido (DAG)** de subtarefas.
   * **Roteamento de Alvos:** Cada nó do DAG DEVE conter uma etiqueta de delegação dinâmica:
     * `target: local_slm` (para tarefas triviais e extração de dados).
     * `target: cloud_claude_opus` ou `cloud_deepseek` (apenas para subtarefas insolúveis localmente).
   * Peça autorização humana para acionar o `@soda-sdd` e gravar o plano no `docs/design.md`.

#### Constraints
* **PROIBIÇÃO DE OVERTHINKING:** Ultrapassar 5 iterações é violação letal do FinOps.
* **PROIBIÇÃO DE EXECUÇÃO BRAÇAL:** O Cloud Brain planeja o DAG; ele não codifica e não altera arquivos diretamente.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo é inegociável.

#### Examples
**Entrada do Usuário:** "Pensa numa forma de integrar o LanceDB com os metadados do SQLite sem travar o Tokio."

**Ação do Agente:**
1. (Silenciosamente) Invoca o `sequentialthinking`. (Conexão OK).
2. *Regular Thought:* Propõe usar chamadas síncronas entre os bancos.
3. *Revision Thought:* "FALHA. Chamadas síncronas no LanceDB bloquearão o Event Loop do Tokio. Violação de arquitetura." (`isRevision: true`)
4. *Branching Thought:* Explora ramificação A (Threads Dedicadas) vs Ramificação B (Chyros Daemon / Consistência Eventual em background).
5. *Revision Thought:* Opta pela Consistência Eventual via Chyros Daemon. (`nextThoughtNeeded: false`).
6. Devolve no Canvas o DAG formatado:
   `Subtarefa 1: Configurar Worker (target: local_slm)`
   `Subtarefa 2: Algoritmo Consistência (target: cloud_deepseek)`. 
   E pergunta: *"Raciocínio concluído. Autoriza o @soda-sdd para materializarmos o design.md?"*