---
name: souls-subrepo-manager
description: O Infiltrador GitOps do SOULS. Orquestra a "Canibalização Cirúrgica" em Shadow Workspaces O(1) via 'snapsafe'. Aplica Sandboxing Zero-Trust (Landlock/AppContainer) sobre a compilação de código de terceiros para evitar RCE via build.rs. Finaliza com Rebase Semântico nativo via 'gitoxide' (sem CLI git legado) e delega a indexação ao Chyros Daemon.
triggers: ["souls-subrepo-manager", "git subrepo", "atualizar dependência", "clonar repo externo", "canibalizar repositório", "injetar submódulo", "gitops"]
---

### skill: SOULS Subrepo Manager (Infiltrador GitOps e Anti-RCE V6.0)

#### Goal
Governar a injeção e o controle de versão de lógicas de terceiros para o interior do projeto local no Antigravity IDE. O objetivo inegociável é orquestrar a "Canibalização Cirúrgica" em **Shadow Workspaces em $\mathcal{O}(1)$** (Hard Links via `snapsafe`). Você deve pulverizar fisicamente lixos tóxicos (Node.js, Python), blindar o host contra execuções remotas (RCE) enjaulando a compilação de código alheio, e garantir que a integração final na `main` ocorra via *Rebase Semântico* puramente em Rust (`gitoxide`), empurrando a re-indexação pesada para o background.

#### Instructions
Sempre que for invocado para puxar dependências externas ou canibalizar repositórios, execute OBRIGATORIAMENTE esta máquina de estados:

1. **Fase 1: O Isolamento em $\mathcal{O}(1)$ (Shadow Workspace):**
   * Você está PROIBIDO de usar `git checkout -b` ou sujar a branch `main`.
   * Acione `snapsafe` para instanciar o **Shadow Workspace** instantâneo no diretório efêmero via Hard Links (Custo: 0 bytes).

2. **Fase 2: Poda Térmica Física (A Morte do Lixo Tóxico):**
   * Utilize a CLI `git subrepo clone` para puxar a lógica externa para dentro da área efêmera.
   * Identifique a "alma matemática" (códigos `.rs`, algoritmos, arquivos Wasm).
   * **Extermínio:** Apague fisicamente do disco todo o ecossistema parasita original: `package.json`, pastas `node_modules/`, `requirements.txt`, Dockerfiles ou bibliotecas C/C++ inúteis.

3. **Fase 3: A Guilhotina de Compilação (Sandboxing Anti-RCE):**
   * Agora você deve provar que a lógica extraída funciona, MAS códigos de terceiros podem ter scripts `build.rs` maliciosos.
   * **Lei do Sandboxing:** A execução do `cargo clippy -- -D warnings` na área isolada DEVE ocorrer obrigatoriamente envelopada sob restrições do SO host (**Landlock** no Linux ou **AppContainer/LPAC** no Windows).
   * Se o kernel barrar tentativas de acesso à rede do código de terceiros, ou se a compilação falhar fatalmente: Aborte, destrua o Shadow Workspace e emita um alerta de Segurança de Workspace.

4. **Fase 4: Pull Request Semântico e Blast Radius (HITL):**
   * Com o código limpo, testado e isolado, gere o relatório tático do *Blast Radius* no Canvas. 
   * Liste o que foi clonado, o que foi expurgado, e remeta à **Agent Inbox**.
   * **Aguarde o Arquiteto Humano digitar "Aprovado".**

5. **Fase 5: Consistência Eventual e Rebase via `gitoxide`:**
   * Recebida a aprovação, você está **PROIBIDO** de rodar comandos legados como `git merge` ou `git commit` via terminal C do SO.
   * Acione a rotina nativa em Rust do **`gitoxide` (`gix`)** para realizar o *Rebase Semântico* atômico do código para a árvore principal.
   * Para não travar o Event Loop do Tokio, repasse as tarefas de indexação dos novos arquivos (AST, Vetores no LanceDB) ESTRITAMENTE para a fila do **Chyros Daemon** operar em background.
   * Apague o Shadow Workspace e retorne silêncio operacional.

#### Constraints
* **FOBIA DE CLI GIT:** Versões legadas do `git` em C corrompem a alocação de memória e criam *merge commits* horríveis. Toda consolidação passa pelo pacote Rust `gitoxide`.
* **SOBREVIVÊNCIA CONTRA RCE:** Nunca compile código `cargo` não confiável sem estar sob `prctl(PR_SET_NO_NEW_PRIVS)` via Landlock.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação do roteamento de Amarração Tardia.

#### Examples
**Entrada do Usuário:** "SOULS, canibaliza aquele parser CSV `fast_csv_parser`. Quero só a lógica em Rust."
**Ação do Agente:**
1. Cria Shadow Workspace em O(1) via `snapsafe`. Faz o `git subrepo clone`.
2. Extirpa arquivos Python e lixos `.js` que vieram com o repositório hospedeiro.
3. Roda o `cargo clippy` da dependência em um ambiente restrito por Landlock. A compilação passa com Exit Code 0 sem tentar acessar a rede.
4. Pede aprovação na Agent Inbox. O Humano aprova.
5. Usa `gitoxide` para injetar as mudanças de forma plana (Rebase Semântico).
6. Notifica o Chyros Daemon para indexar o AST do CSV Parser no LanceDB durante a madrugada, devolvendo a UI imediatamente para o usuário.