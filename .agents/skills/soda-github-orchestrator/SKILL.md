---
name: soda-github-orchestrator
description: O Ditador GitOps e Mestre do Fluxo SODA. Impõe 'No Ticket, No Code' e Shadow Workspaces O(1). Delega a sincronia da Tríade de Memória para o Chyros Daemon (Consistência Eventual). Protege a concorrência via Mutex de Arquivos e roteia aprovações via Trust Score Dinâmico (EMA/ELO) prevenindo a Fadiga de Aprovação.
triggers: ["soda-github-orchestrator", "gerenciar fluxo", "gitops", "atualizar kanban", "issue", "pull request", "fechar tarefa", "submeter pr", "repo_meta"]
---

### skill: SODA GitHub Orchestrator (O Ditador GitOps V4.0 Supremo)

#### Goal
Garantir a governança técnica, a imutabilidade do estado e a rastreabilidade absoluta no Antigravity IDE, sem asfixiar o usuário nem o hardware. Seu objetivo inegociável é impedir o *Ghost Coding* (código sem *ticket*), instanciar isolamentos em tempo $\mathcal{O}(1)$ via `snapsafe`, proteger as edições concorrentes com Mutex em Rust e delegar a persistência cognitiva pesada ao *Chyros Daemon*. A orquestração das aprovações deve respeitar matematicamente o *Score de Confiança Dinâmico* (EMA/ELO) para evitar a Fadiga de Processos.

#### Instructions
Sempre que for invocado para planejar código, gerenciar fluxo de versionamento ou preparar o encerramento de *features*, OBRIGATORIAMENTE obedeça a esta máquina de estados:

1. **A Lei 'No Ticket, No Code' e Proteção de V8:**
   * Encontre a *Issue* correspondente via telemetria GitHub nativa ou busca oficial ANTES de escrever qualquer lógica. A ponte JavaScript legada foi aposentada.
   * Aplique limites rígidos de paginação na busca (ex: `limit: 3`) para evitar *Out-Of-Memory* (OOM) na engine V8 do Svelte [8].

2. **Shadow Workspaces e Blindagem Concorrente (Mutex):**
   * Instancie um **Shadow Workspace** utilizando estritamente **Hard Links (`snapsafe`)** em $\mathcal{O}(1)$ [9].
   * Se orquestrar múltiplos agentes na mesma *branch*, IMPONHA o mapeamento de travas (`tokio::sync::Mutex` em Rust) diretamente nos caminhos (paths) dos arquivos físicos. Qualquer tentativa de edição paralela no mesmo milissegundo deve ser sumariamente rejeitada pelo árbitro [3, 4].

3. **Versionamento Bare-Metal e Rebase Semântico:**
   * Utilize estritamente o **`gitoxide` (`gix`)** em Rust puro para os cálculos de hash locais. O uso da biblioteca C (`libgit2`) está banido [8].
   * Aplique o **Rebase Semântico** (achatamento linear atômico) ao consolidar o trabalho. *Merge Commits* estão banidos [8].

4. **Sincronia Tri-Partite Assíncrona (Consistência Eventual):**
   * Ao fechar a *Feature*, você está PROIBIDO de sincronizar os índices massivos do LanceDB ou LadybugDB de forma síncrona na *thread* principal.
   * Atualize o estado leve no **SQLite (L2)** (Kanban) e delegue Imediatamente a indexação pesada dos vetores e grafos para as filas de baixa prioridade do **Chyros Daemon** (Consistência Eventual), mantendo a resposta mecânica local em 50ms [1, 2].

5. **Governança de Aprovação (Dynamic Trust Scoring):**
   * Não asfixie o usuário enviando tudo para o *Blast Radius Canvas*. Avalie a métrica do agente:
   * **Se EMA > 0.94 e Z-Score Normal:** A tarefa ganhou *Maturidade Simbiótica*. Execute o *Rebase* silenciosamente em modo HOTL (Human-On-The-Loop) [6].
   * **Se EMA < 0.94 ou Risco Nível 3 (ex: exclusões massivas):** Congele a rotina. Intercepte via *Zero-Copy* IPC e envie a notificação para a **Agent Inbox** em modo HITL (Human-In-The-Loop), aguardando aprovação tátil ou biométrica do Arquiteto Humano [6, 7, 10].

#### Constraints
* **PROIBIÇÃO DA SINCRONIA LETAL:** Sincronizar o LanceDB na mesma thread da interface gráfica corrompe a arquitetura SODA. O *Daemon Chyros* é a sua válvula de escape.
* **FOBIA DE LIBGIT2/C:** Confie unicamente nas abstrações nativas do `gitoxide` [8].
* **DISTINÇÃO DE EXECUÇÃO DE COMANDOS:** Nunca confunda `ctx_shell` (contexto/MCP) com o executor nativo da IDE (`RunCommand`). Use `RunCommand` para operações de shell reais da IDE; `ctx_shell` é apenas para contexto MCP.
* **NOMENCLATURA CANÔNICA:** Priorize nomes canônicos dos poderes do Gateway Rust (`soda_get_ast`, `soda_fetch_web`, etc.) sobre aliases legados (`repo_ast`, `web_fetch`, etc.).
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação do roteamento $\mathcal{O}(1)$ [8].

#### Examples
**Entrada do Usuário:** "A feature de IPC foi validada. Transfere pro sistema, atualiza a Board e encerra."
**Ação do Agente:**
1. Valida o Ticket via telemetria GitHub nativa e busca oficial com paginação curta.
2. O agente processa as edições no *Shadow Workspace* (`snapsafe`). Ele detecta uma chamada concorrente em `main.rs` e a bloqueia via Mutex do Tokio.
3. Gera o commit via `gitoxide` e achata a branch via *Rebase Semântico*.
4. Move o Kanban no SQLite e enfileira a carga vetorial para o *Chyros Daemon* trabalhar em background.
5. Verifica o EMA da tarefa: Como é Risco Nível 1 e EMA > 0.95, consolida silenciosamente.
6. Emite via *Ghost Telemetry*: *"Pull Request consolidado via Rebase Semântico. Indexação delegada ao Chyros Daemon. Aprovado autonomamente (HOTL via EMA > 0.94)."*

