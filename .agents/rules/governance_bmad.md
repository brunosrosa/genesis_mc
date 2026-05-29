---
trigger: always_on
---

###### 1. A REGRA DE OURO E A TOPOLOGIA DE WORKSPACES
Respeite as fronteiras absolutas de dados. O armazenamento persistente NUNCA se mistura com a execução:
*   **Domínio do Usuário (Cofre):** Fronteira imutável (SQLite/LanceDB/LadybugDB). PROIBIDA alteração direta sem aprovação.
*   **Shadow Workspace (Mesa de Rascunho):** Ambiente isolado criado instantaneamente em $\mathcal{O}(1)$ via `snapsafe` (Hard Links). Permitido uso livre de Docker, Python e ferramentas de dev para testes.
*   **Sandboxes (Motor Descartável):** *Sidecars* efêmeros (Wasmtime/Micro-VMs) rodando na RAM. DEVEM morrer atomicamente (`SIGKILL`) após o uso. Zero lixo sobrevivente. A configuração mora no Domínio.

###### 2. CSDD (CONSTITUTIONAL SPEC-DRIVEN DEVELOPMENT)
Engenharia estocástica (*Vibe Coding*) é PROIBIDA. O código só nasce após:
1.  **Validação:** Submeter a intenção aos limites de hardware (6GB VRAM, IPC Zero-Copy).
2.  **Especificação:** Gravar a tríade imutável (`proposal.md`, `design.md`, `tasks.md`) no *Shadow Workspace*.
3.  **Definition of Done (DoD):** Entregar *Scaffold* executável (`cargo test` vazios) antes da lógica.
4.  **TDD Forçado:** Teste falha PRIMEIRO. Código nasce para corrigir o erro. Na 3ª falha consecutiva (Ralph Loop), aplique *Fail-Closed*: pare, mova o card para "Bloqueado" no Kanban e aguarde.

###### 3. PROTOCOLO BMAD E AGENT INBOX
A base `main` é sagrada. Siga o fluxo BMAD para qualquer mutação estrutural:
*   **B - Branch:** Isole tarefas em ramificações via *Hard Links* no *Shadow Workspace*.
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

###### 7. A DOUTRINA DE CANIBALIZAÇÃO E GIT SUBREPO
*   **Expurgo Absoluto:** USO OBRIGATÓRIO do `git-subrepo` para internalizar bibliotecas (`git submodule` banido).
*   **Extração AST $\mathcal{O}(1)$:** OBRIGATÓRIO usar `jcodemunch` (Byte-Offset) para extrair a "alma matemática". NUNCA leia repositórios inteiros por força bruta.
*   **Descarte do Monólito:** Após sugar a lógica, DESTRUA arquivos Node.js, Python e Docker pesados da biblioteca original. O *Rebase* absorve estritamente o código Rust purificado.
 acompanharem o repositório original. O SODA consome a lógica estrutural em Rust/Wasm e descarta o lixo.

### LEI DA HIGIENE DE WORKSPACE (FOBIA DE RAIZ):
É TERMINANTEMENTE PROIBIDO despejar scripts de automação, meta-programação, logs, testes ou arquivos temporários na raiz do repositório.
- A pasta `.soda_scratchpad/` existe OBRIGATORIAMENTE para ser o seu laboratório. Qualquer script gerador (Python/Bash) ou log efêmero DEVE ser criado nela.
- A raiz do projeto é terreno sagrado, reservado exclusivamente para configurações fundacionais (Cargo.toml, .env, README).
- Lixo não sobrevivente deve ser apagado fisicamente após o uso.