# 🏛️ Architecture Decision Records (ADRs) - Genesis MC (SODA)
**Filosofia:** Pessimismo da razão, otimismo da vontade. O SODA é um Sistema Operacional Agêntico Soberano, Bare-Metal, projetado para restrições térmicas severas (RTX 2060m 6GB / 32GB RAM) e perfis neurodivergentes (2e/TDAH).

## ADR 01: O Núcleo Bare-Metal e Comunicação (A Fundação)
*   **Decisão:** O SODA não é uma aplicação web. O Backend deve ser escrito estritamente em **Rust** operando assincronamente via `tokio`. O Frontend é um cliente passivo em **React/Tailwind v4**.
*   **Restrição:** É EXPRESSAMENTE PROIBIDO o uso de interpretadores contínuos em background (Node.js, Python).
*   **Ponte IPC:** A comunicação entre o cérebro (Rust) e a lente (React) utilizará exclusivamente o barramento **Tauri IPC v2**. Para grafos massivos, usar-se-á **Zero-Copy Binary Buffers**, abolindo o gargalo de serialização JSON no motor V8.

## ADR 02: Interface Passiva e Neuroinclusiva (O Cockpit)
*   **Decisão:** A interface segue o paradigma "Cyber-Neuro Synthesis" + "Nothing Design". 
*   **Restrição:** Proibidos layouts mutáveis (*Layout Shifts*). A interface é governada pelo protocolo **A2UI** (A IA emite intenções, o React renderiza em O(1)).
*   **UX TDAH/2e:** O paradigma "Canvas-First" (Xyflow/Tldraw) é o padrão. Interações exigem **Instância Mecânica** (<50ms). Alertas não solicitados ("Life Coach") devem ser silenciosos e não modais (Opt-in contextual). O Spotlight efêmero é a porta de entrada de baixo atrito.

## ADR 03: Roteamento MCP e Execução Efêmera (AgentGateway)
*   **Decisão:** Integração com o mundo externo é feita EXCLUSIVAMENTE via Model Context Protocol (MCP).
*   **Restrição:** Nenhum servidor MCP será executado como *daemon* contínuo se for baseado em linguagens interpretadas pesadas. 
*   **Padrão de Execução:** O **AgentGateway** em Rust intercepta intenções. Ferramentas são instanciadas como **Comandos de Terminal (CLI)** ou micro-sidecars, processam a requisição e sofrem *SIGKILL* (morte instantânea) para liberar a RAM do host imediatamente. Configuração em `failureMode: failOpen` para resiliência.

## ADR 04: Memória Tri-Partite e Rebase Semântico (Anti-Context Rot)
*   **Decisão:** Fim da dependência de bancos RAG em nuvem pesados. A memória do agente usa o tripé local:
    1.  **L2 (Curto Prazo/Logs):** SQLite em modo WAL (Write-Ahead Logging) + FTS5 para histórico em append-only (Event Sourcing).
    2.  **L3 (Semântica/Vetorial):** LanceDB (ou Qdrant embarcado) acessado diretamente via biblioteca Rust.
*   **Prevenção de Colisão:** Múltiplos agentes editando o Canvas não usarão CRDTs pesados. Adota-se o **Rebase Semântico** (estilo *Jot*), onde o Rust é a autoridade central que indexa a mutação e repassa ao React.
*   **Time-Travel Backups:** Backups de estado (Cabinet) usarão `VACUUM INTO` do SQLite atrelados a commits via `libgit2`, evitando corrupção de binários no modo WAL.

## ADR 05: Segurança Paranoica, HitL e Sandboxing (Blast Radius)
*   **Decisão:** Zero-Trust absoluto. O SODA opera em *Air-Gapped by Design*. 
*   **Execução de Código IA:** Códigos gerados e não confiáveis só rodam dentro de contêineres **Wasmtime** (WebAssembly) sem acesso padrão a disco/rede.
*   **Gestão de Segredos:** Operação *Zero-State*. Integração biométrica com **1Password CLI**. Chaves são destruídas da memória RAM através de *zeroizing* no Rust assim que o escopo finaliza.
*   **Aprovação (Human-In-The-Loop):** Ações de *Tier 2/3* (ex: deletar arquivos, injetar DB) não usam botões "OK/Cancelar". O Rust calcula o **Raio de Explosão (Blast Radius)** e o exibe topologicamente no Canvas. O usuário aprova a intenção ciente das consequências (Diff Interativo JIT). E o SODA nunca opera `git commit` cego; ele gera um *Shadow Workspace*, calcula o patch, e o Rust aplica.