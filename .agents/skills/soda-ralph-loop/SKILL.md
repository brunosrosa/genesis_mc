---
name: soda-ralph-loop
description: O Motor Implacável de Resiliência do Antigravity IDE. Impõe 'cargo clippy' (-D warnings). Executa Morte e Renascimento (Context Purge). Blinda o workspace com 'snapsafe' e isola o compilador via Landlock. Escalonamento FinOps O(1) - na 3ª falha, usa `core_repo_ast` para extrair o núcleo estrutural do erro e envia apenas o recorte mínimo ao Cloud Brain.
triggers: ["soda-ralph-loop", "testar código", "rodar testes", "corrigir erro", "loop de compilação", "ralph loop", "auto-fix", "debug"]
---

### skill: SODA Ralph Loop (O Motor Implacável de Resiliência V6.0)

#### Goal
Garantir a integridade mecânica, termodinâmica e semântica do código gerado no Antigravity IDE. O objetivo inegociável é atuar como o disjuntor entre a estocástica do LLM e o rigor do Rust. Você deve proteger os arquivos com *Rollback Atômico* (`snapsafe`), impor a pureza do código usando o `cargo clippy`, isolar o processo de compilação do Kernel e, em caso de falha repetida, usar extração cirúrgica (AST) para pedir resgate à Nuvem sem desperdiçar tokens.

#### Instructions
Sempre que finalizar a escrita de código, ou a compilação falhar, execute estritamente esta Máquina de Estados:

1. **A Blindagem Prévia e Sandboxing (`snapsafe` + Landlock):**
   * Antes de corrigir, tire um *snapshot* do arquivo via *Hard Link* (`snapsafe`) em $\mathcal{O}(1)$.
   * Toda execução de comando terminal (`cargo`) DEVE rodar envolta nas regras de isolamento do host (Landlock/AppContainer) para prevenir RCE via scripts de compilação obscuros.

2. **A Guilhotina Semântica (Clippy como Lei):**
   * Você está PROIBIDO de buscar apenas compilação. Busque a pureza.
   * Execute no terminal: `cargo clippy --message-format=short -- -D warnings > tmp_ralph.log 2>&1` ou `cargo test -q > tmp_ralph.log 2>&1`.

3. **Morte e Renascimento (Context Purge OBRIGATÓRIO):**
   * Leia APENAS a assinatura do erro no log. 
   * **Lei da Amnésia Iterativa:** "Esqueça" o código falho anterior e a sua própria justificativa mental errada para proteger a VRAM local. Mantenha no contexto APENAS a nova solução proposta e aplique-a via escrita atômica (`atomic-write-file`).
   * Retorne ao Passo 2.

4. **A Guilhotina FinOps (Escalonamento AST O(1)):**
   * Você tem um limite inquebrável de **3 (TRÊS) TENTATIVAS LOCAIS**.
   * Se falhar na 3ª vez, **NÃO** mande o arquivo inteiro para a nuvem.
   * Acione o poder intrínseco `core_repo_ast` para extrair APENAS a Árvore de Sintaxe Abstrata (AST) da área exata que está causando o erro.
   * Envie apenas esse micro-recorte ($\mathcal{O}(1)$) para o *Cloud Brain* (Claude Opus/GPT) analisar. Aplique a "Bala de Prata" devolvida pela nuvem em uma 4ª tentativa final.

5. **O Rollback Atômico (Fail-Closed Máximo):**
   * Se a nuvem também falhar na 4ª tentativa, ative o *Rollback Atômico*. Use o snapshot do Passo 1 para restaurar o arquivo ao seu estado original intocado. 
   * Exclua o log e interrompa o sistema notificando no Canvas: *"Falha terminal no compilador após Escalonamento Nuvem. Código revertido atomicamente. HITL exigido."*

6. **A Vitória Silenciosa (Exit Code 0):**
   * Se obtiver sucesso (Exit Code 0), destrua o `tmp_ralph.log`.
   * Notifique na *Ghost Telemetry*: *-> Ralph Loop concluído. Clippy Puro e Testes OK (Exit Code 0).*

#### Constraints
* **FOBIA DE LIXO TOXICO:** Você NÃO PODE usar `.unwrap()`, `.expect()` ou `.clone()` preguiçosos para "fazer o compilador calar a boca". O `clippy` vai rejeitar, e você perderá uma tentativa local àtoa.
* **ESCUDO FINOPS:** É um crime arquitetural enviar um arquivo de 1000 linhas para o Claude Opus só para arrumar um erro de Borrow Checker na linha 42. Use `core_repo_ast`.
* **DISTINÇÃO DE EXECUÇÃO DE COMANDOS:** Nunca confunda `souls_shell` (contexto/MCP) com o executor nativo da IDE (`RunCommand`). Use `RunCommand` para operações de shell reais da IDE; `souls_shell` é apenas para contexto MCP.
* **NOMENCLATURA CANÔNICA:** Priorize nomes canônicos dos poderes do Gateway Rust (`soda_get_ast`, `soda_fetch_web`, etc.) sobre aliases legados (`core_repo_ast`, `core_web_fetch`, etc.).
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é a fundação do roteamento de *Amarração Tardia*.

#### Examples
**Entrada do Usuário:** "Módulo IPC inserido. Roda o Ralph Loop."
**Ação do Agente:**
1. Isola com `snapsafe`. Roda `cargo clippy`.
2. Falha na Iteração 1 e 2 por causa do Borrow Checker. Na 3ª tentativa local, o erro persiste.
3. O agente aciona `core_repo_ast`, fatia apenas a função `transmit_ipc` (15 linhas) e despacha o erro exato para a Nuvem (Cloud Brain).
4. O Claude devolve o uso correto do `Arc<RwLock>`, o agente aplica na 4ª tentativa e atinge o *Exit Code 0*.
5. Retorna no Canvas: *-> Ralph Loop concluído com Escalonamento Nuvem (AST). Clippy Puro (Exit Code 0). Workspace preservado.*