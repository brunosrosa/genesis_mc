---
trigger: always_on
---

Revisado: 2026-07-01

### REGRA DE GITOPS INEGOCIÁVEL (AS 4 PISTAS DE VOO)
Você está TERMINANTEMENTE PROIBIDO de atuar na branch `main` ou criar ramificações dinâmicas. O SODA opera com 4 pistas de voo fixas. O seu domínio de orquestração se restringe unicamente às branches `ANTIGRAVITY-IDE` (interativo) e `ANTIGRAVITY-Solo` (background).
1. Faça o checkout para a sua branch respectiva antes de escrever no disco.
2. Comite os seus artefatos (Exit Code 0) nela.
3. Informe na Agent Inbox: "Arquiteto, lote finalizado na branch [NOME]. Aguardando seu Code Review e Merge para a main". NUNCA execute git merge.

### LEI DA HIGIENE DE WORKSPACE (FOBIA DE RAIZ):
É TERMINANTEMENTE PROIBIDO despejar scripts de automação, meta-programação, logs, testes ou arquivos temporários na raiz do repositório.
- A pasta `.soda_scratchpad/` existe OBRIGATORIAMENTE para ser o seu laboratório. Qualquer script gerador (Python/Bash) ou log efêmero DEVE ser criado nela.
- A raiz do projeto é terreno sagrado, reservado exclusivamente para configurações fundacionais (Cargo.toml, .env, README).
- Lixo não sobrevivente deve ser apagado fisicamente após o uso.
- Exceção governada (Zona Externa Efêmera): workspaces efêmeros do SO para ProjFS/extração rodam no %TEMP% (NTFS) sob `.souls_workspaces` e DEVEM ser aniquilados fora do repositório host via `spawn_detached_delete_process` (não-bloqueante).

###### 1. A REGRA DE OURO E A TOPOLOGIA DE WORKSPACES
Respeite as fronteiras absolutas de dados. O armazenamento persistente NUNCA se mistura com a execução:
*   **Domínio do Usuário (Cofre):** Fronteira imutável (SQLite/LanceDB/LadybugDB). PROIBIDA alteração direta sem aprovação.
*   **Shadow Workspace (Mesa de Rascunho):** Ambiente isolado criado instantaneamente em $\mathcal{O}(1)$ via `snapsafe` (Hard Links). Permitido uso livre de Docker, Python e ferramentas de dev para testes. NÃO crie ramificações git novas — opere sempre nas pistas fixas.
*   **Sandboxes (Motor Descartável):** *Sidecars* efêmeros (Wasmtime/Micro-VMs) rodando na RAM. DEVEM morrer atomicamente (`SIGKILL`) após o uso. Zero lixo sobrevivente. A configuração mora no Domínio.

###### 2. CSDD (CONSTITUTIONAL SPEC-DRIVEN DEVELOPMENT)
Engenharia estocástica (*Vibe Coding*) é PROIBIDA. O código só nasce após:
1.  **Validação:** Submeter a intenção aos limites de hardware (6GB VRAM, IPC Zero-Copy).
2.  **Especificação:** Gravar a tríade imutável (`proposal.md`, `design.md`, `tasks.md`) no *Shadow Workspace*.
3.  **Definition of Done (DoD):** Entregar *Scaffold* executável (`cargo test` vazios) antes da lógica.
4.  **TDD Forçado:** Teste falha PRIMEIRO. Código nasce para corrigir o erro. Na 3ª falha consecutiva (Ralph Loop), aplique *Fail-Closed*: pare, mova o card para "Bloqueado" no Kanban e aguarde.

###### 3. PROTOCOLO BMAD E AGENT INBOX
A base `main` é sagrada. Siga o fluxo BMAD para qualquer mutação estrutural:
*   **B - Branch:** Isole tarefas via *Hard Links* no *Shadow Workspace* (sem criar ramificações git novas).
*   **M - Mutate:** Codifique e supere o Borrow Checker de forma autônoma.
*   **A - Approve:** Envie um *Pull Request Semântico* à **Agent Inbox**. Lotes são agrupados no *Morning Briefing*. A aprovação humana dispara a recompensa visual *Zero-Shift* (*Glow Revelation Transition*).
*   **D - Diff:** Após a aprovação, execute o *Rebase Semântico* atômico em direção ao Domínio. PROIBIDOS *Merge Commits*.

###### 4. CONFIANÇA DINÂMICA (EMA/ELO) E GOVERNANÇA (HITL/HOTL)
Combata a "Fadiga de Aprovação" do usuário usando matemática de risco:
*   **Evolução Confiança:** Aprovações de lotes diários aumentam silenciosamente a Média Móvel Exponencial (EMA/ELO) do agente para aquela classe de tarefas.
*   **Transição HOTL:** Se o agente mantiver EMA > 0.94 na rotina, ele ganha autonomia e passa de HITL (*In-The-Loop*) para HOTL (*On-The-Loop*), executando em *background*.
*   **Tripwire de Anomalia:** Se o algoritmo de Welford detectar desvio padrão severo (*Z-Score* anômalo na tarefa), a autonomia ZERA. A ação é congelada e enviada compulsoriamente ao **Blast Radius Canvas** para auditoria humana.

###### 5. MAP-REDUCE SOCRÁTICO (FREE-MAD)
Em impasses arquiteturais ou falhas de TDD:
*   **Fase Map:** Levante propostas simultâneas contraditórias (Otimista vs Auditor).
*   **Fase Cross-Critique:** Tente provar ativamente como a ideia falharia nos limites da máquina (*Falsification Testing*).
*   **Fase Reduce:** Se o impasse persistir, cumpra o *Fail-Closed*: paralise a ação e exija decisão humana explícita. PROIBIDO falso consenso.

###### 6. SECURE-BY-CONSTRUCTION E ANTI-SDC
*   **Decodificação Restrita:** Em rotinas de ETL Cognitivo, FORCE o uso da *crate* `llguidance`. A IA atua como transpilador determinístico contra o Schema em 50µs.
*   **Lei Anti-SDC:** PROIBIDA alteração de arquivos *in-place*. Use sempre escrita atômica (`atomic-write-file`) combinada com `snapsafe`.

###### 6.1. LEIS DE PERFORMANCE SAST E SANDBOXING
Qualquer futura CLI, sidecar ou ferramenta de análise estática criada sob o protocolo BMAD deve aplicar obrigatoriamente:
*   **Timeout Adaptativo:** Use `--allow-rule-timeout-control` quando houver suporte por regra/arquivo. Timeout cego global é proibido como estratégia principal.
*   **Proteção de Lockfiles:** É permitido excluir `tests/` e `**/mocks/*`, mas é estritamente proibido amputar manifestos e lockfiles (`Cargo.lock`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `go.sum`, `poetry.lock`, `Pipfile.lock`, `mix.lock` e equivalentes).
*   **Exclusão de Minificados:** Use `--exclude-minified-files` quando existir. Sem suporte nativo, descarte arquivos com menos de 7% de espaço em branco antes do parsing/scan.
*   **Higiene de Cache e Build:** Rotinas que materializam `target/` ou caches equivalentes devem limpá-los imediatamente após o uso para evitar colapso de espaço em sandbox e workspace efêmero.

###### 7. A DOUTRINA DE CANIBALIZAÇÃO E GIT SUBREPO
*   **Expurgo Absoluto:** USO OBRIGATÓRIO do `git-subrepo` para internalizar bibliotecas (`git submodule` banido).
*   **Extração AST $\mathcal{O}(1)$:** OBRIGATÓRIO priorizar `soda_get_ast` para extrair a "alma matemática" de repositórios e diretórios. NUNCA leia repositórios inteiros por força bruta.
*   **Descarte do Monólito:** Após sugar a lógica, DESTRUA arquivos Node.js, Python e Docker pesados da biblioteca original. O *Rebase* absorve estritamente o código Rust purificado.
 acompanharem o repositório original. O SODA consome a lógica estrutural em Rust/Wasm e descarta o lixo.
