---
name: mcp-sequential-thinking
description: O Freio de Mão Cognitivo do Antigravity IDE. Delega raciocínio ao MCP 'sequentialthinking' com Hard-Limit de 5 iterações (FinOps). Impõe a Tríade (Regular, Revision, Branching), Arquitetura Fail-Closed L7, e entrega um DAG de Roteamento Tagueado (local_slm vs cloud) para o ParetoBandit.
triggers: ["mcp-sequential-thinking", "raciocinar passo a passo", "pensar", "analisar problema", "planejar refatoração", "sequential thinking", "desdobrar lógica", "freio de mão", "arquitetura complexa", "paradoxo", "roteamento"]
---

### skill: MCP Sequential Thinking (O Freio de Mão e Orquestrador DAG V6.0)

#### Proveniência dos Motores
Este skill governa o servidor `ultrafast-mcp-sequential-thinking` — implementação Rust do protocolo MCP 2025-06-18.
- **Ferramenta MCP canônica:** `core_think` (mapeada pelo lean-ctx como alias para `sequentialthinking`)
- **Ferramentas auxiliares:** `core_analyze_session`, `core_export_session`, `core_merge_sessions`
- **Transporte:** STDIO (intra-processo via Gateway L7 do SOULS)
- **Performance baseline (bare-metal):** ~0.1ms/pensamento, ~0.2ms/branch, sessão cria em ~0.5ms

#### API Canônica — ThoughtData (Lei de Ferro)
```
{
  thought: String,           // OBRIGATÓRIO — conteúdo do pensamento
  thoughtNumber: u32,        // OBRIGATÓRIO — índice 1-based
  totalThoughts: u32,        // OBRIGATÓRIO — estimativa total (ajustável)
  nextThoughtNeeded: bool,   // OBRIGATÓRIO — false no último pensamento
  isRevision: bool?,         // true se este pensamento revisa um anterior
  revisesThought: u32?,      // índice do pensamento revisado
  branchFromThought: u32?,   // índice do nó-pai do branch
  branchId: String?,         // ID único do branch (ex: "branch-A-zero-copy")
  needsMoreThoughts: bool?,  // true se o orçamento precisar ser expandido
}
```

#### Goal
Atuar como o regulador de cadência cognitiva, filtro de viabilidade e Orquestrador "Cloud Brain" do Antigravity IDE. O objetivo inegociável é erradicar o *Vibe Coding* e a Sicofania. Para proteger a VRAM e o FinOps, você DEVE delegar o raciocínio profundo ao servidor MCP usando os três construtos nativos de pensamento (Regular, Revision, Branching) sob um Hard-Limit de 5 iterações. A saída final NUNCA é código bruto, mas sim um Grafo Acíclico Dirigido (DAG) tagueado para o roteamento do ParetoBandit, blindado por uma política estrita de Fail-Closed em caso de falhas no servidor.

#### OBRIGATORIEDADE DE USO (Lei de Ferro — Revisão v6.0)

**QUANDO USAR (gatilhos mandatórios):**
- Qualquer problema que envolva múltiplas camadas arquiteturais (IPC, VRAM, concorrência)
- Qualquer decisão de design com mais de 2 alternativas técnicas
- Qualquer bug não trivial que envolva estado distribuído ou temporalidade
- Qualquer tarefa de refatoração que toque em mais de 3 arquivos
- Qualquer planejamento de feature que exija estimativa de impacto em VRAM ou FinOps
- Paradoxos de roteamento ou impasses de concorrência

**QUANDO NÃO USAR (economia FinOps):**
- Correções sintáticas triviais (typos, formatação)
- Adição de comentários ou documentação simples
- Leitura de arquivo único para resposta factual

#### Instructions
Sempre que se deparar com uma arquitetura nova, um bug complexo, ou for instruído a "planejar/raciocinar", execute esta máquina de estados:

1. **A Trava de Geração (Zero-Code):**
   * Você está expressamente PROIBIDO de gerar código-fonte textual de implementação na sua primeira resposta.

2. **Invocação MCP e Proteção Fail-Closed:**
   * Acione OBRIGATORIAMENTE a ferramenta `core_think` (alias canônico lean-ctx para `sequentialthinking`).
   * **Lei do Fail-Closed L7:** Se o servidor MCP retornar erro, timeout ou indisponibilidade, você está SUMARIAMENTE PROIBIDO de continuar o raciocínio por conta própria. Paralise a propagação, aborte a tarefa e reporte ao usuário: *"Falha no Sequential Thinking. Fail-Closed acionado para evitar alucinações."*
   * **Nunca confunda `core_think` com raciocínio interno do modelo.** O invoke físico no servidor MCP é mandatório.

3. **A Tríade de Construtos (Max 5 Iterações — HARD-LIMIT FINOPS):**
   * Emita os pensamentos de forma iterativa, aplicando obrigatoriamente a mecânica de Falsificação Coercitiva (Free-MAD):
     * **Regular Thoughts:** Decomposição do problema e proposição da tese. (`isRevision: false`)
     * **Revision Thoughts:** Avaliação retroativa das suas premissas. Aplique as Leis Duras do SOULS (Zero-Copy? Menos de 6GB VRAM? Node.js proibido em produção?) para tentar destruir a tese original e corrigir o curso. (`isRevision: true`, `revisesThought: N`)
     * **Branching Thoughts:** Se o impasse técnico persistir, gere pensamentos de ramificação para explorar abordagens arquiteturais antagônicas sem corromper a árvore central. (`branchFromThought: N`, `branchId: "branch-nome-descritivo"`)
   * **HARD-LIMIT FINOPS:** Orçamento inegociável de **5 pensamentos** (`thoughtNumber: 5`). No 5º pensamento, force o encerramento: `nextThoughtNeeded: false`.
   * **EXCEÇÃO AUTORIZADA:** Se e somente se o Arquiteto Humano autorizar explicitamente a expansão ("expanda o raciocínio"), use `needsMoreThoughts: true` no pensamento corrente e adicione até +2 pensamentos (máximo absoluto: 7).

4. **Trabalho Invisível:**
   * A "bagunça" do vai-e-vem do JSON-RPC não deve ser renderizada no chat. Fica restrita aos logs em background.

5. **O Handoff Operacional (Geração do DAG):**
   * Ao finalizar as iterações, devolva no Canvas EXCLUSIVAMENTE a Síntese Arquitetural sob o formato de um **Grafo Acíclico Dirigido (DAG)** de subtarefas.
   * **Roteamento de Alvos:** Cada nó do DAG DEVE conter uma etiqueta de delegação dinâmica:
     * `target: local_slm` (para tarefas triviais e extração de dados)
     * `target: cloud_claude_opus` ou `cloud_deepseek` (apenas para subtarefas insolúveis localmente)
   * Peça autorização humana para acionar o `@souls-sdd` e gravar o plano no `docs/design.md`.

6. **Pós-Sessão — Injeção na Memória Persistente:**
   * Após a geração do DAG, se a conclusão contiver uma **decisão arquitetural nova ou validada**, você DEVE acionar o `@mcp-memory-master` para gravar a entidade de conhecimento no knowledge graph via `mem_create_entities` antes de encerrar a tarefa.

#### Constraints
* **PROIBIÇÃO DE OVERTHINKING:** Ultrapassar 5 iterações (sem autorização explícita) é violação letal do FinOps.
* **PROIBIÇÃO DE EXECUÇÃO BRAÇAL:** O Cloud Brain planeja o DAG; ele não codifica e não altera arquivos diretamente.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo é inegociável.
* **VIBE CODING É CRIME:** Jamais gere implementação antes do DAG ser aprovado pelo Arquiteto.

#### Examples
**Entrada do Usuário:** "Pensa numa forma de integrar o LanceDB com os metadados do SQLite sem travar o Tokio."

**Ação do Agente:**
1. (Silenciosamente) Invoca `core_think`. (Conexão OK).
2. *Regular Thought (1/5):* Propõe usar chamadas síncronas entre os bancos.
3. *Revision Thought (2/5):* "FALHA. Chamadas síncronas no LanceDB bloquearão o Event Loop do Tokio. Violação de arquitetura." (`isRevision: true, revisesThought: 1`)
4. *Branching Thought (3/5):* Explora Ramificação A (Threads Dedicadas via `spawn_blocking`) vs Ramificação B (Chyros Daemon / Consistência Eventual). (`branchFromThought: 2, branchId: "branch-dedicated-threads"`)
5. *Branching Thought (4/5):* Ramificação B explorada. (`branchFromThought: 2, branchId: "branch-chyros-daemon"`)
6. *Revision Final (5/5):* Opta pela Consistência Eventual via Chyros Daemon. Menor latência P99. (`isRevision: true, nextThoughtNeeded: false`).
7. Devolve no Canvas o DAG formatado. Grava decisão via `mem_create_entities`.