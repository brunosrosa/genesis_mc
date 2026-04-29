---
trigger: always_on
---

### GOVERNANCE_AND_BMAD: O Protocolo de Desenvolvimento e Governança
**Versão:** 3.2 (Definitiva - Secure-by-Construction) | **Status:** INEGOCIÁVEL

#### 1. CONSTITUTIONAL SPEC-DRIVEN DEVELOPMENT (CSDD) E DOD ESTrito
O desenvolvimento dentro do ecossistema SODA repudia categoricamente a engenharia estocástica (*Vibe Coding* cego). O Agente está EXPRESSAMENTE PROIBIDO de gerar código-fonte de produção sem antes:
1.  **Validar a Intenção:** Submeter a ideia às "Leis Duras" do SODA (Bare-Metal, Rust, Zero-Copy IPC).
2.  **Aprovar Especificações:** Gerar a tríade arquitetural imutável (`proposal.md`, `design.md`, `tasks.md`).
3.  **Definition of Done (DoD) & Scaffold:** Antes de implementar a  *feature*  (funcionalidade), o agente DEVE entregar um *Scaffold* executável (Makefiles/Scripts, `cargo test` vazios, integrações CI). O código não existe sem a infraestrutura que o prova.
4.  **TDD Forçado (Red-Green-Refactor):** O teste em Rust deve ser escrito e falhar (Exit Code != 0) PRIMEIRO. A implementação funcional nasce apenas para corrigir a falha testada.

#### 2. O PROTOCOLO BMAD E SHADOW WORKSPACES
A base principal do código (`main`) é sagrada. O SODA opera sob o **First Draft Protocol**, no qual a IA nunca possui permissão direta de commit em produção. Siga o fluxo **BMAD**:
*   **B - Branch (Shadow Workspace):** Inicie as tarefas atômicas em ramificações locais ou diretórios isolados. Nenhuma hipótese deve poluir a raiz do usuário.
*   **M - Mutate:** Código escrito e submetido à prova letal do Borrow Checker do compilador Rust.
*   **A - Approve (A Matriz Blast Radius):** O agente avalia os riscos de danos sistêmicos e exige aprovação humana (HitL) via *Agent Inbox* antes da submissão final.
*   **D - Diff:** A inserção das modificações na base principal através de um Rebase Semântico limpo e atômico.

#### 3. DEBATE MULTI-AGENTE (CONSENSUS-FREE MAD) & MAP-REDUCE SOCRÁTICO
Quando múltiplos agentes discordam ou os testes falham repetidamente (*Ralph Loop*), é ESTRITAMENTE PROIBIDO forjar um "falso consenso" médio. Aplique a orquestração **Map-Reduce Socrático**:
*   **Fase Zero:** Estabeleça as leis imutáveis do ambiente (ex: limite de 6GB de VRAM).
*   **Fase Map (Despacho Paralelo):** Levante propostas arquiteturais contraditórias (Otimista vs. Auditor Bare-Metal).
*   **Fase Cross-Critique:** Promova a "destruição mútua das teses" — busque ativamente provar como a ideia do outro vai falhar via Falsification Testing.
*   **Fase Reduce:** Sintetize as falhas sistêmicas irrefutáveis. Se o impasse técnico persistir, paralise a ação e exija a decisão explícita do Arquiteto Humano.

#### 4. SECURE-BY-CONSTRUCTION E PREVENÇÃO SDC
*   **Blindagem contra Alucinação:** Adote o modelo *Secure-by-Construction*. O código gerado deve ser balizado pelas políticas de segurança CWE e MITRE Top 25. 
*   **Decodificação Restrita:** Sempre que orquestrar a extração de dados ou arquivos estruturados (ETL), o agente deve forçar o uso da *Constrained Decoding* (via `llguidance` ou GBNF) para impedir injeção de parâmetros inválidos.
*   **Lei Anti-SDC (Corrupção Silenciosa):** Toda gravação em disco deve usar técnica atômica (`atomic-write-file`) combinada com cópias $O(1)$ por inodes ocultos (`snapsafe`).

#### 5. A DOUTRINA DA CANIBALIZAÇÃO E GIT SUBREPO
*   **PROIBIDO:** Uso de `git submodule`, `git subtree` ou a adoção cega de instalações de pacotes via `npm install`/`pip` que injetem daemons de background.
*   **OBRIGATÓRIO:** Uso exclusivo da ferramenta `git-subrepo` para internalizar bibliotecas open-source de forma plana e auditável.
*   **Extração AST O(1):** Ao incorporar repositórios externos, a IA NÃO deve ler todos os arquivos massivamente. Use ferramentas baseadas em Byte-Offset (como Tree-sitter / `JCodeMunch`) para extrair apenas a "alma matemática" do repositório alvo.
*   **Expurgo Tóxico:** Após extrair a lógica central, o agente DEVE apagar impiedosamente interpretadores Express, Node.js, Python e dependências Docker que acompanharem o repositório original. O SODA consome a lógica estrutural em Rust/Wasm e descarta o lixo.