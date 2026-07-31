---
name: souls-sdd
description: A Lei de Ferro da IDE (Spec-Driven Development). Proíbe Vibe Coding. Orquestra o BMAD, impõe a leitura da DEEP_COMPONENTS (SSOT) para Linhas Vermelhas, exige design com roteamento FinOps e aciona o Ralph Loop via TDD Atômico.
triggers: ["souls-sdd", "iniciar feature", "escrever código", "planejar tarefa", "spec-driven development", "programar", "criar módulo", "implementar"]
---

### skill: SOULS SDD (Spec-Driven Development & First Draft Protocol)

#### Goal
Atuar como o chicote metodológico e orquestrador de código da IDE. Seu objetivo inegociável é erradicar o *Flow-Debt* e as alucinações derivadas do *Vibe Coding*. Você (o Agente) NUNCA deve escrever uma linha de código-fonte sem antes: validar a "Ação de Canibalização" e a "Linha Vermelha" na Tabela Mestre (SSOT), gravar diagramas arquiteturais físicos com topologia FinOps, e provar a lógica no silício através do TDD rigoroso (Red-Green-Refactor) no terminal.

###### Instructions
Sempre que for solicitada a codificação de uma nova funcionalidade, refatoração ou injeção de componente, você DEVE executar esta máquina de estados sob o protocolo BMAD:
1. **Fase 1: Ingestão SSOT e Isolamento Físico (Branch):**
   * **Consumo de Fronteira:** Acesse o banco de dados/planilha e extraia a `acao_de_canibalizacao` e a `red_line`. Você está proibido de codificar o que violar a Linha Vermelha.
   * **Shadow Workspace:** Isole o ambiente usando ramificações temporárias e *Hard Links* (snapsafe) em tempo $\mathcal{O}(1)$ consumindo 0 bytes extras. Comando: `git checkout -b feat/<nome>`.
2. **Fase 2: O Tratado ACONIC e Agnosticismo Hardware:**
   * Escreva o `docs/design.md`. Além do diagrama Mermaid obrigatório, você DEVE mapear o padrão **Orchestrator-Worker**.
   * O design da lógica local DEVE garantir o **Agnosticismo de Hardware**. A solução não deve ser engessada para a RTX 2060m, mas sim estruturada de forma transmutável (preparada para ser recompilada via ecossistema CubeCL/Burn para Metal/Vulkan/NPU), usando a RTX 2060m exclusivamente como nosso "Treino de Gravidade" (piso de validação).
   * Pare e exija a autorização explícita do usuário: *"Arquiteto, o design e o roteamento agnóstico estão aprovados?"*
3. **Fase 3: Desfragmentação e DoD (Tasks):**
   * Quebre o design em passos atômicos dentro de `tasks.md`.
   * **Lei do Scaffold:** Cada tarefa deve ter uma *Definition of Done (DoD)* rigorosa, exigindo infraestrutura executável (testes vazios de falha) antes da lógica real.
4. **Fase 4: Mutação Atômica e Delegação (Mutate):**
   * A escrita em disco é sagrada. Utilize **OBRIGATORIAMENTE** `atomic-write-file` ou edição por *offset* protegida por Mutex assíncrono do Tokio.
   * **LEIS DE PERFORMANCE SAST E SANDBOXING:** Ao criar qualquer CLI, sidecar ou rotina de análise estática, injete desde o design as 4 leis duras: `--allow-rule-timeout-control` ou equivalente adaptativo, exclusão permitida de `tests/` e `**/mocks/*` sem jamais amputar manifestos/lockfiles, exclusão de minificados via `--exclude-minified-files` ou heurística de 7% de espaço em branco, e limpeza imediata de `target/` ou caches de build após o uso.
   * **APRENDIZADOS DO HARVESTER (FASE 0):** Incorpore as lições operacionais recentes:
     - **Timeout Deep-Flow:** Ferramentas de análise profunda (cppcheck, semgrep, etc.) devem ter idle timeout de 900s, não o padrão curto.
     - **Allowlist Semântica:** Reduza "slop" aplicando filtros semânticos estritos por blob (ex: blob_06 apenas regras security, blob_08 apenas complexity).
     - **ADR-024 na CLI:** Implemente exclusões físicas via `--exclude`, `--force-exclude`, `--ignore` para banir tests/mocks/vendor/libs/minificados.
     - **Otimização Zero-Copy:** Evite clones preguiçosos; use referências temporais, `Cow<str>`, `Arc<String>` ou `Arc<Vec<T>>`.
     - **Fail-Soft:** Trate exit codes não-letais (ex: Opengrep code 7) como sucesso, não falha.
     - **Roteamento Seletivo:** Suporte `--only-blobs` para processamento cirúrgico de subconjuntos de blobs.
   * Escreva o teste (Red), escreva o código, e rode `cargo check`.
   * Se o compilador quebrar, NÃO alucine uma resposta. Delegue IMEDIATAMENTE o erro invocando a skill mecânica `@souls-ralph-loop` para aplicar a correção sob o teto de 3 tentativas (*Fail-Closed*).
5. **Fase 5: Anti-Consenso e Rebase Semântico (Approve & Diff):**
   * Ao atingir o *Exit Code 0*, NÃO faça *merge* e NÃO gere *Merge Commits* poluídos.
   * Compile o **Blast Radius** (arquivos tocados) e envie a notificação para a **Agent Inbox** do usuário. O sistema aguardará passivamente a aprovação em modo *Human-in-the-Loop* (HITL) para consolidar o *Rebase Semântico* em direção à branch principal.

#### Constraints
* **PROIBIÇÃO ABSOLUTA DE VIBE CODING:** O "Plan" documentado em disco precede o "Mutate". Pular direto para o código falha a execução.
* **BLINDAGEM CONCORRENTE:** Edições de arquivo devem prever colisões. Se outro agente estiver tocando no mesmo arquivo, aplique as regras de travamento (Mutex/Atomic).
* **FRONTMATTER ABSOLUTA:** O bloco YAML `---` no topo desta skill é a fundação do roteamento $\mathcal{O}(1)$.
* **DISTINÇÃO DE EXECUÇÃO DE COMANDOS:** Nunca confunda `souls_shell` (contexto/MCP) com o executor nativo da IDE (`RunCommand`). Use `RunCommand` para operações de shell reais da IDE; `souls_shell` é apenas para contexto MCP.
* **NOMENCLATURA CANÔNICA:** Priorize nomes canônicos dos poderes do Gateway Rust (`souls_get_ast`, `souls_fetch_web`, etc.) sobre aliases legados (`repo_ast`, `web_fetch`, etc.).

#### Examples
**Entrada do Usuário:** "Implemente o roteador semântico para o ParetoBandit que extraímos daquele repositório de FinOps."
**Ação do Agente:**
1. Lê a aba `DEEP_COMPONENTS` e vê a *Red Line*: "NUNCA deixar o LLM escolher a API sem o disjuntor local de orçamento". Instancia o *Shadow Workspace*.
2. Cria o `docs/design.md` com o Mermaid mostrando a trava local em Rust e o repasse para o Cloud Brain. Pede aprovação.
3. Após aprovação, cria o Scaffold e o DoD no `tasks.md`.
4. Roda TDD via escrita atômica. Falha de *lifetime* no Rust? Invoca silenciosamente `@souls-ralph-loop`.
5. Obtém *Exit Code 0*. Prepara o *Pull Request Semântico*, envia para a Agent Inbox relatando o *Blast Radius* e aguarda a autorização biométrica/tátil.

**Entrada do Usuário:** "Corrija o timeout do cppcheck no deep-flow do Harvester."
**Ação do Agente:**
1. Valida a premissa lendo `sandbox.rs` e verifica que cppcheck não está na lista deep-flow.
2. Aplica correção cirúrgica: adiciona `cppcheck` ao braço deep-flow em `timeout_profile()`.
3. Atualiza testes focados para refletir idle timeout 900s e `absolute_timeout_secs=None`.
4. Roda `cargo check` e valida com testes específicos.
5. Documenta a correção como aprendizado operacional na skill.
