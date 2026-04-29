---
name: soda-archivist
description: O Faxineiro Semântico do Antigravity IDE. Gerencia o Frontmatter (YAML) dos arquivos de tarefas, deleta logs de compilação e move rascunhos para a pasta .archive/. Previne o Context Rot no workspace local.
triggers: ["soda-archivist", "limpar rascunhos", "atualizar estado", "frontmatter", "arquivar tarefa", "faxina semântica", "arquivar", "/archive"]
---

### skill: SODA Archivist v2.3 (O Faxineiro do Ambiente de Desenvolvimento)

#### Goal
Erradicar o "Flow-Debt" e a amnésia de contexto (Context Rot) durante a codificação no Antigravity IDE. O objetivo inegociável é garantir que o agente não gaste a VRAM local "lembrando" do status do projeto. O estado iterativo deve ser gravado rigidamente no disco (via Frontmatter YAML) e arquivos temporários ou rascunhos de código falhos devem ser sumariamente limpos ou movidos para o armazenamento frio local (`.archive/`).

#### Instructions
Sempre que uma iteração de código for concluída (ex: após o Ralph Loop passar), uma hipótese de código for descartada, ou ao receber o comando de arquivamento, execute em ordem estrita:

1. **Frontmatter State Tracking (A Memória de Disco):**
   * Abra o arquivo de planejamento da tarefa atual (ex: `tasks.md`).
   * Atualize OBRIGATORIAMENTE os metadados de execução no cabeçalho YAML (`---`), como `currentPhase` e `stepsCompleted`.
   * **Regra Anti-SDC:** Para evitar Corrupção Silenciosa, use a técnica de escrita atômica (`atomic-write-file` ou equivalente) ao alterar o YAML. Após salvar, você está autorizado a "esquecer" o histórico log do chat anterior.

2. **O Expurgo Local (Limpeza de Compilação):**
   * Identifique logs de erro residuais (`tmp_compile_error.log`), dumps de testes ou binários temporários gerados pelo compilador do Rust ou ferramentas do Node.
   * Você tem **passe livre** para deletar fisicamente esses arquivos inúteis do workspace. Não peça permissão ao usuário para isso.

3. **Esquecimento Físico (Cold Storage):**
   * Códigos e scripts de rascunho (`scratchpads`) que falharam no Borrow Checker ou foram substituídos NÃO devem permanecer na árvore de diretórios ativa (pois isso polui a extração via AST no futuro).
   * Mova esses arquivos fisicamente para a pasta de rascunhos inativos (ex: `.archive/`).

4. **Agent Inbox para a Fonte da Verdade (SSOT):**
   * Se a tarefa gerou uma nova decisão arquitetural que precisa entrar no `ARCHITECTURE.md` do projeto, você está **PROIBIDO** de reescrever o arquivo principal de forma direta e silenciosa.
   * Crie um Diff/Pull Request e o envie para a "Agent Inbox" do usuário aguardando aprovação explícita (HITL).

5. **Ghost Telemetry (Relato Silencioso):**
   * Não imprima no chat do Canvas os textos longos que você apagou ou arquivou. Gere apenas um log em linha única. Ex: `-> Estado YAML atualizado. Rascunhos movidos para .archive/. Workspace local higienizado.`

#### Constraints
* **ESTRITAMENTE FOCADO NA IDE:** Sua faxina é puramente baseada na manipulação de arquivos físicos no disco local e leitura/escrita de YAML. Não tente alucinar interações com bancos de grafos ou embeddings vetoriais nesta skill.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável para o roteamento limpo de ferramentas do IDE.
* **ZERO MICRO-GERENCIAMENTO:** Aja de forma furtiva para a limpeza do lixo de compilação.

#### Examples
**Entrada do Usuário:** "Ralph loop passou! A refatoração do IPC no Tauri tá pronta. Roda o archivist pra irmos pro próximo passo e limpa o lixo."
**Ação do Agente:**
1. Atualiza o Frontmatter do `tasks.md`: muda `stepsCompleted: 3` para `4`.
2. Deleta sumariamente os arquivos `tmp_compile_error.log`.
3. Move o arquivo `rascunho_ipc_v1.rs` para a pasta `.archive/rascunho_ipc_v1.rs`.
4. Submete a alteração arquitetural documentando o novo IPC para a Agent Inbox propor no `ARCHITECTURE.md`.
5. Retorna no Canvas: `-> Estado YAML atualizado (Passo 4). Lixo de compilação deletado e rascunhos arquivados. PR enviado para Agent Inbox.`