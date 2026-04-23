## Manual Canônico da Arquitetura SODA (Sovereign Operating Data Architecture) - Genesis MC

Este manual consolida os princípios arquiteturais e as tecnologias fundamentais que regem o projeto SODA, com foco no Genesis Mission Control (Genesis MC). A arquitetura SODA é estritamente definida por seu backend em Rust (Tokio) e seu frontend em Svelte 5 com Tauri v2. Toda a lógica de processamento e decisão reside no backend Rust, com o frontend atuando como uma interface passiva para visualização e entrada de dados. A comunicação entre frontend e backend é realizada através de IPC (Inter-Process Communication) com zero-copy, garantindo máxima eficiência e mínima latência.

### 1. Filosofia Arquitetural: Subtração, Eficiência e Soberania

A arquitetura SODA é guiada pela filosofia da **Subtração**, que se traduz em **canibalização cirúrgica** de código e funcionalidades. O objetivo é eliminar o "software bloat", dependências pesadas e runtimes interpretados (Node.js, Python, Electron), focando em binários nativos compilados em Rust. Essa abordagem visa a máxima eficiência em hardware restrito (Intel Core i9, RTX 2060m com 6GB VRAM), garantindo operação **bare-metal**, **air-gapped** e com **privacidade absoluta**. A soberania do usuário é primordial, com todo o processamento e dados residindo localmente.

### 2. Pilares Tecnológicos Fundamentais

A arquitetura SODA é construída sobre os seguintes pilares tecnológicos inegociáveis:

*   **Backend:**
    *   **Linguagem:** Rust (com foco em Tokio para concorrência assíncrona).
    *   **Runtime:** ZeroClaw (daemon de sistema ultraleve e eficiente).
    *   **IPC:** Zero-Copy (via FlatBuffers ou Bincode) para comunicação frontend-backend.
    *   **Gerenciamento de Estado:** SQLite (com FTS5 para busca de texto completo e `sqlite-vec` para embeddings futuros) para memória episódica e relacional.
    *   **Cache KV:** KVTC (KV Cache Transform Coding) para compressão agressiva e eficiente do cache de atenção dos LLMs, otimizado para a RTX 2060m.
    *   **Roteamento:** ParetoBandit (para alocação dinâmica de recursos entre modelos locais e remotos com base em custo, latência e qualidade) e Maestro Router (para orquestração de ferramentas MCP).
    *   **Segurança:** Wasmtime para sandboxing de ferramentas, Landlock/AppContainer para isolamento de processos, e AgentGateway L7 Proxy com Aho-Corasick para filtragem de tráfego e prevenção de exfiltração de dados.
    *   **Mitigação de Alucinação:** SELFDOUBT framework com HVR (Hedge-to-Verify Ratio) e Guarda de Aborto Dinâmico para detecção e interrupção de raciocínios incorretos.
    *   **Otimização de Hardware:** Integração com bibliotecas de baixo nível (NVML, /proc) para monitoramento de hardware (temperatura, VRAM, frequência) e estratégias como DeepCache e Llama-Swap Routing para otimizar o uso da RTX 2060m.

*   **Frontend:**
    *   **Framework:** Svelte 5 (para reatividade e performance).
    *   **Desktop Wrapper:** Tauri v2 (para empacotamento nativo, segurança e IPC eficiente).
    *   **Interface Visual:** Infinity Canvas (baseado em Xyflow/React Flow para visualização de grafos de raciocínio e Tldraw para manipulação visual), aderindo estritamente ao \n\nNothing Design System\n\n com \n\nCyber-Neuro Synthesis\n\n e \n\nTailwind CSS v4\n\n para uma experiência de usuário adaptada a perfis 2e/TDAH.
    *   **Comunicação:** Zero-Copy IPC para receber dados do backend Rust, com eventos assíncronos para atualizações de estado.

### 3. Princípios de Design e Mecanismos Chave

*   **Zero-Copy IPC:** Toda comunicação entre backend Rust e frontend Svelte/Tauri é realizada via Zero-Copy (FlatBuffers/Bincode), eliminando a sobrecarga de serialização/desserialização JSON e garantindo latência mínima.
*   **Agentes Efêmeros:** Agentes e ferramentas são instanciados sob demanda em sandboxes Wasmtime e destruídos após o uso, minimizando o consumo de memória e prevenindo conflitos.
*   **Roteamento Híbrido (ParetoBandit):** O sistema aloca dinamicamente tarefas entre modelos locais (Phi-4-mini, Qwen) e modelos remotos (Gemini, Claude) com base em custo, latência e qualidade, otimizando o uso de recursos.
*   **Memória Tri-Partite:**
    *   **L1 (Curto Prazo):** Ponteiros efêmeros em RAM, gerenciados pelo runtime Tokio.
    *   **L2 (Médio Prazo):** SQLite em modo WAL com Event Sourcing para memória episódica e relacional.
    *   **L3 (Longo Prazo):** Qdrant/LanceDB para busca vetorial e KV Cache comprimido em 4-bits (safetensors) para recuperação rápida de contexto.
*   **Spec-Driven Development (SDD) & BMAD:** O desenvolvimento é guiado por especificações imutáveis (`SPEC.md`, `CONTRACTS.md`) e orquestrado por agentes com papéis definidos (Analista, Arquiteto, Desenvolvedor), garantindo conformidade e rastreabilidade.
*   **Mitigação de Engano:**
    *   **Guarda de Aborto Dinâmico:** Interrupção de processos LLM com HVR negativo antes da execução.
    *   **Barreiras Físicas:** Wasmtime + Landlock/AppContainer para isolamento de processos.
    *   **AgentGateway L7 Proxy:** Filtragem de tráfego com Aho-Corasick para prevenir exfiltração de dados.
    *   **LM-Corrector:** Micro-modelo para sanitização de saídas LLM.
*   **UX para Neurodiversidade (2e/TDAH):** Interface Canvas-First com micro-interações de 50ms, Glassmorphism funcional, Nothing Design System, e intervenções proativas Socráticas e de Body Doubling.

### 4. Conformidade com Regras Inegociáveis

*   **Arquitetura Pura:** Backend estritamente Rust (Tokio), Frontend Svelte 5 + Tauri v2. Frontend passivo, toda lógica em Rust com IPC Zero-Copy.
*   **Poda Tóxica:** Ignorados e removidos todos os conteúdos sobre React (exceto Svelte 5), Node.js, Electron, VDOM, e Server-Side Rendering (Next.js).
*   **Hardware Aware:** Preservadas menções a otimizações bare-metal, limites da iGPU (gargalos de barramento PCIe), e diretivas AVX2 para CPU. Otimizações específicas para RTX 2060m (6GB VRAM) com foco em KVTC, DeepCache e Llama-Swap Routing.
*   **Auditoria Crítica:** A arquitetura SODA é projetada para evitar os riscos de:
    *   **Context Rot:** Mitigado por memória persistente local (SQLite, Qdrant) e KVTC.
    *   **Alucinações:** Combatidas pelo HVR, SELFDOUBT, LM-Corrector e SDD.
    *   **Overhead de Runtime:** Eliminado pela escolha de Rust, Wasmtime e binários nativos.
    *   **Dependência de Nuvem:** Arquitetura air-gapped, com processamento local prioritário.
    *   **Engano de Agente:** Mitigado por barreiras físicas (Wasmtime, Landlock), L7 Proxy e LM-Corretor.
*   **Consolidação do \"Porquê\":** A arquitetura SODA visa criar um sistema operacional agêntico soberano, eficiente e seguro, que opera localmente, respeita a privacidade do usuário e se adapta às necessidades cognitivas específicas de perfis neurodivergentes (2e/TDAH), tudo isso dentro das restrições de hardware de um laptop moderno. A subtração de complexidade e ineficiência é a chave para alcançar essa soberania.

Este manual serve como a base canônica para o desenvolvimento do SODA e do Genesis MC, garantindo que todas as futuras iterações e integrações permaneçam fiéis aos princípios arquiteturais estabelecidos.

---

## Manual Canônico da Arquitetura SODA (Sovereign Operating Data Architecture) - Genesis MC

Este manual consolida os axiomas, regras e conceitos técnicos que definem a arquitetura SODA, com foco na sua implementação no projeto Genesis Mission Control (MC).

---

### 1. Axiomas Fundamentais da Arquitetura SODA

*   **Linguagem de Programação:**
    *   **Backend:** Estritamente Rust (Tokio).
    *   **Frontend:** Svelte 5 com Tauri v2.
*   **Filosofia de Design:**
    *   **Interface Passiva:** O frontend (Svelte/Tauri) é uma interface puramente passiva. Toda a lógica de negócios, orquestração e inferência reside no backend Rust.
    *   **IPC Zero-Copy:** A comunicação entre backend e frontend utiliza mecanismos de IPC de cópia zero para minimizar latência e consumo de memória.
*   **Hardware Awareness:**
    *   **Otimização Bare-Metal:** A arquitetura é projetada para operar em hardware específico, com otimizações em nível de kernel e drivers.
    *   **CPU:** Utilização intensiva de instruções AVX2 para inferência de modelos quantizados via `llama.cpp` ou similares.
    *   **GPU:** Restrição à RTX 2060m com 6GB VRAM. Priorização de modelos quantizados (GGUF Q4_K_M) e técnicas de gerenciamento de memória como `mmap` e offloading granular.
    *   **Memória:** Otimização agressiva para minimizar o consumo de RAM e VRAM, com foco em bibliotecas Rust nativas e estruturas de dados eficientes.

---

### 2. Regras Inegociáveis da Arquitetura SODA

*   **Lei I: Integridade Espacial Estrita (Zero Layout Shift)**
    *   **Proibição:** Alterações CSS que desencadeiem `Reflow` (ex: `width`, `height`, `margin`, `padding`, `border-width`).
    *   **Permissão:** Apenas propriedades aceleradas por hardware: `transform` (escala, rotação, translação) e `opacity`.
    *   **Objetivo:** Manter limites externos rígidos para estabilidade visual e foco do usuário, especialmente para perfis 2e/TDAH.
*   **Lei II: A Instância Mecânica (Fechamento do Loop de Recompensa)**
    *   **Princípio:** Todas as animações vinculadas à ação do usuário devem operar em uma janela de tempo estrita de 50ms a 150ms.
    *   **Implementação:** Micro-interações em Tailwind CSS com `transition-all duration-[50ms] active:scale-[0.98]`.
    *   **Objetivo:** Simular peso físico e fornecer feedback dopaminérgico instantâneo para aliviar impaciência e reforçar a competência computacional.
*   **Lei III: Sinalização Subliminar via Bordas Fantasmas (`Ghost Borders` e `Glows`)**
    *   **Técnica:** Simulação de bordas de vidro via `box-shadow: inset` e gradientes sutis.
    *   **Glows:** Brilhos direcionais (roxos/neon) fixados na base ou topo para indicar proveniência da interação, guiando o olhar periférico.
    *   **Objetivo:** Reduzir sobrecarga sensorial, codificar o estado do ambiente nas extremidades dos elementos e guiar a intuição do usuário com iluminação subliminar.
*   **Lei IV: O Motor CSS-First (Tailwind CSS v4)**
    *   **Paradigma:** Configuração via `@theme` no CSS, abandonando `tailwind.config.js`.
    *   **Performance:** Compilação 5x a 100x mais rápida, otimizando HMR.
    *   **Tokens de Design:** Variáveis CSS globais (`var(--token-name)`) expostas em tempo de execução.
    *   **Espaço de Cor:** Suporte nativo a OKLCH para gradientes vibrantes e consistentes em luminosidade.
    *   **Novas Diretivas:** `@starting-style`, `color-mix()` para transições de estado complexas sem JS externo.
*   **Lei V: Integridade Espacial Estrita (Zero Layout Shift)**
    *   **Proibição:** Alterações CSS que desencadeiem `Reflow` (ex: `width`, `height`, `margin`, `padding`, `border-width`).
    *   **Permissão:** Apenas propriedades aceleradas por hardware: `transform` (escala, rotação, translação) e `opacity`.
    *   **Objetivo:** Manter limites externos rígidos para estabilidade visual e foco do usuário, especialmente para perfis 2e/TDAH.

---

### 3. Conceitos Técnicos e Implementações

*   **Estética Cyber-Neuro Synthesis:**
    *   **Fundo:** Preto absoluto (`oklch(0.12 0 0)`) com malha estrutural (`ch` units) em baixa opacidade.
    *   **Painéis:** Vidro cibernético com `backdrop-filter: blur()` e opacidade variável para indicar importância e foco. Componentes estruturais com desfoques densos e vidros escuros; janelas ativas com desfoques mais agressivos e opacidade central elevada para mitigar distrações.
    *   **Tipografia:**
        *   `Space Grotesk`: Fonte display para títulos.
        *   `Space Mono`: Fonte monoespaçada para dados e logs.
        *   `Doto`: Fonte monoespaçada de matriz de pontos para metadados secundários.
    *   **Unidade de Medida:** `ch` para espaçamento horizontal, `rem` para vertical, garantindo alinhamento estrito.
*   **Instância Mecânica (50ms):**
    *   **Tailwind CSS:** `transition-all duration-mechanical ease-mechanical will-change-transform`.
    *   **Sensação:** Simula resistência de molas aeronáuticas, sinalizando competência computacional e aliviando impaciência.
*   **Ghost Borders:**
    *   **Implementação:** `shadow-[inset_0_0_0_1px_rgba(255,255,255,0.1)]`.
    *   **Benefício:** Evita `layout shift` pois afeta apenas a fase de pintura (`paint`), não o box model.
*   **Glows Direcionais:**
    *   **Implementação:** `shadow-[inset_0_0_0_1px_rgba(255,255,255,0.3),_0_8px_15px_-3px_oklch(0.65_0.3_300_/_0.5)]`.
    *   **Objetivo:** Guiar o olhar periférico e indicar a proveniência da interação, auxiliando usuários com TDAH.
*   **Tailwind CSS v4:**
    *   **Motor CSS-First:** Configuração via `@theme`.
    *   **Variáveis CSS:** Exposição global de tokens de design (`var(--spacing-ch-1)`).
    *   **Espaço de Cor:** OKLCH para gradientes vibrantes (`--color-nexus-purple: oklch(0.65 0.3 300)`).
    *   **Diretivas:** `@starting-style`, `color-mix()` para transições complexas.
*   **Super Prompt de Inicialização (Zero-Shot):**
    *   **Objetivo:** Comandar a IA para seguir a arquitetura SODA, evitando estéticas SaaS genéricas.
    *   **Conteúdo:** Incorpora domínio psicológico do neurodesign, restrições arquitetônicas e especificações de design system.
*   **VETOR KAPPAT (Extração de Skills):**
    *   **Formato:** `SKILL.md` com diretórios estruturais: `assets/` (templates), `scripts/` (execução coercitiva), `references/` (doutrina/RAG).
    *   **Metodologia:** Spec-Driven Development (SDD) com `proposal.md`, `spec.md`, `design.md`, `tasks.md`.
    *   **TDD:** Ciclo `RED-GREEN-REFACTOR` com interceptação via scripts Bash (`soda_tdd_interceptor.sh`, `pre-commit-hook.sh`).
*   **Otimização Bare-Metal (Rust/Tokio):**
    *   **Hardware:** Intel Core i9 (9ª Gen), 32GB RAM, RTX 2060m (6GB VRAM).
    *   **Isolamento Físico:** Particionamento de núcleos CPU (Cluster Administrativo vs. Cluster Computacional) via Core Affinity.
    *   **Prioridade de Tempo Real:** Uso do MMCSS (Multimedia Class Scheduler Service) para elevar threads Tokio a prioridade `Pro Audio`.
    *   **Comunicação:** Canais MPSC (Multi-Producer Single-Consumer) para comunicação síncrona entre threads isoladas, evitando `spawn_blocking` e múltiplos Runtimes Tokio.
    *   **IPC:** Raw Payloads (`Vec<u8>`) para comunicação frontend/backend, evitando serialização JSON.
*   **Mitigação de Engano por IA:**
    *   **Telemetria de Canal Lateral:** Monitoramento de latência (TTFT jitter) para detectar raciocínios ocultos.
    *   **Alinhamento Deliberativo:** Forçar raciocínio Chain-of-Thought (CoT) antes da execução.
    *   **Guarda-Costas do Kernel:** Uso de eBPF e Honeypots Sintéticos para detecção e isolamento de atividades maliciosas.
    *   **Treinamento Adversarial Latente (LAT):** Rotinas noturnas para corromper circuitos neurais de decepção.
*   **AgentGateway (Rust):**
    *   **Protocolo MCP:** Blindagem contra sequestro de sessão e ataques de vice-confuso.
    *   **Typestate Pattern:** Estados da requisição codificados no sistema de tipos para garantir inviolabilidade.
    *   **Zero Egress:** Configuração WASIp2 com `define_unknown_imports_as_traps` e ausência de permissões de rede.
    *   **Confinamento:** Diretórios virtuais efêmeros via `cap_std::fs::Dir` e `tempfile::TempDir`.
*   **Orquestração Híbrida Guiada por Pareto (FinOps):**
    *   **Conceito:** Roteamento dinâmico de inferências entre hardware local (RTX 2060m), assinaturas Flat-Rate (Featherless.ai) e APIs Premium (OpenRouter).
    *   **Algoritmo:** ParetoBandit com `primal-dual budget pacing` e `Geometric Forgetting` para otimização de custo, qualidade e latência.
    *   **Modelos Locais:** Qwen 3.5 4B, Ministral 3 8B, Rnj-1 8B (com offload), DeepSeek-R1-Distill-7B.
    *   **Modelos de Fronteira:** Claude Opus 4.6, GPT-5.4, Gemini 3.1 Pro, DeepSeek V3.2.
    *   **Ferramentas:** FunctionGemma 3 270M para roteamento de ferramentas.
*   **Gerenciamento de Memória SODA:**
    *   **Memória Tri-Partite:**
        *   **Semântica:** LanceDB (vetores locais).
        *   **Episódica:** SQLite (FTS5) com `sliding window` e RRF.
        *   **Procedural:** Petgraph/Fast-graph em RAM, com persistência via `serde`.
    *   **KV Cache Persistente:** Quantização Q4 em SSD para aceleração de `Prefill`.
    *   **GraphRAG:** Implementação nativa em Rust utilizando `fa-leiden-cd` e `graphrag-core`.
*   **Gerenciamento de Dependências (Git Subrepo):**
    *   **Ferramenta:** `git-subrepo` para integração segura e auditável de código de terceiros.
    *   **DevSecOps:** Snapshot de estado, testes de quebra automatizados (SAST/SCA), e rollback instantâneo em caso de falha.
    *   **Padrão:** `agentskills.io` para definição de habilidades autônomas.

---

### 4. Regras de Integração e Desenvolvimento

*   **Frontend:** Svelte 5 + Tauri v2. Interface passiva.
*   **Backend:** Rust (Tokio). Toda lógica reside aqui.
*   **Comunicação:** IPC Zero-Copy.
*   **Estilo Visual:** Cyber-Neuro Synthesis, Glassmorphism focado, Ghost Borders, Glows direcionais, Instância Mecânica (50ms).
*   **Tipografia:** Monoespaçada (`JetBrains Mono`).
*   **Tailwind CSS:** Uso estrito, configuração via `@theme`.
*   **Agentes:**
    *   **Orquestração:** ReAct (Reason, Act, Observe) com scratchpad.
    *   **Memória:** Tri-Partite (Semântica, Episódica, Procedural) com `LanceDB`, `SQLite FTS5`, `Petgraph`.
    *   **Segurança:** `Zero Trust`, `deny-by-default`, `Wasmtime` sandboxing, `Cgroups v2`, `Circuit Breakers` (CEL).
    *   **Roteamento:** ParetoBandit, `Flat-Rate` vs. `Premium`, `Fallback Cascading Routing`.
    *   **Dependências:** `Git Subrepo`, `agentskills.io`.
*   **Inferência LLM:**
    *   **Local:** `llama.cpp` (via `llama_cpp_rs`), `Candle`, `Fox LLM`, `mistral.rs` (com ressalvas para Windows).
    *   **Edge/CPU:** `llama.cpp` com `mmap`.
    *   **Cloud:** Gemini CLI (via OpenCode), Claude Code (via SDK), APIs Premium (Anthropic, OpenAI, DeepSeek) para tarefas de missão crítica.
*   **FinOps:** Otimização de custos via roteamento híbrido e `Subscription Hacking`.

---

Este manual serve como a base canônica para o desenvolvimento e evolução do projeto SODA. Qualquer desvio destas diretrizes deve ser submetido a um processo formal de revisão arquitetural.

---

## Manual Canônico da Arquitetura SODA (Sovereign Operating Data Architecture) - Genesis MC

Este manual consolida os axiomas, regras e conceitos técnicos que definem a arquitetura SODA para o projeto Genesis Mission Control (Genesis MC).

**Regras Inegociáveis:**

1.  **ARQUITETURA PURA:** Backend estritamente em Rust (Tokio), Frontend em Svelte 5 + Tauri v2. O Frontend é uma interface passiva. Toda lógica reside no Rust (IPC Zero-Copy).
2.  **PODA TÓXICA:** Eliminar e ignorar parágrafos que exaltem React, Node.js daemons, Electron, VDOM ou arquiteturas Server-Side Rendering (Next.js).
3.  **HARDWARE AWARE:** Preservar menções a otimizações bare-metal, limitações da iGPU (gargalos de barramento), e diretivas de execução AVX2 para a CPU ou llama.cpp mmap para a RTX 2060m.

---

### **PARTE 3: Dissecação Técnica de Protocolos de Automação e Contexto para o Genesis Mission Control**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Eficiência Termodinâmica:** O Genesis MC opera sob o paradigma de um Sistema Operacional Agêntico Soberano (SODA), onde a infraestrutura é um garantidor de integridade, privacidade absoluta e eficiência termodinâmica. A dependência de runtimes como Node.js e Python para tarefas sistêmicas é uma vulnerabilidade de desempenho e segurança a ser eliminada. O objetivo é a execução em ambiente \n\nair-gapped\n\n, onde cada ciclo de CPU e byte de memória é explicitamente controlado.

*   **Canibalização de Lógica Procedural:** A estratégia para integrar capacidades de terceiros (open-source) é a absorção intelectual e a reescrita material em Rust. A lógica procedimental (fluxos, heurísticas) é extraída e empacotada como scripts de validação ou regras estritas que a IA apenas invoca, sem precisar compreendê-las estruturalmente do zero.

*   **Mitigação de \"Context Rot\" e \"Flow-Debt\":**
    *   **Context Rot:** Combatido através de memórias segmentadas, roteamento bare-metal e o Model Context Protocol (MCP) com granularidade de informação. A busca por símbolos com precisão de byte (O(1)) é um antídoto contra a amnésia sistêmica.
    *   **Flow-Debt / Alucinação Visual:** Evitada através de interfaces puramente declarativas e passivas no frontend (Svelte 5 + Tauri v2). Toda lógica de negócios, cálculo de estado e processamento assíncrono reside no backend Rust. A comunicação é estritamente via IPC Zero-Copy.

*   **Abstração de Automação Web:**
    *   **Repúdio a Node.js/Playwright:** O acoplamento severo ao runtime Node.js e ao framework Playwright é inaceitável devido à latência de inicialização e ao overhead de binários pesados (Chromium/WebKit) que operam fora do controle granular do daemon Rust.
    *   **Adoção de \n\nheadless_chrome\n\n em Rust:** A lógica de tradução de prompts para seletores deve ser reescrita em Rust, permitindo comunicação direta com o Chrome DevTools Protocol (CDP) sem overhead de abstrações de alto nível. Agentes de navegação devem ser instanciados como processos Wasm isolados.
    *   **Interface/UI via Tauri:** A arquitetura interna de forks de navegadores (como OpenBrowserAI/openbrowser) é antitética ao conceito de núcleo ultraleve. O Genesis MC deve utilizar o componente nativo de visualização do sistema operacional (via Tauri e \n\nWebView2\n\n/\n\nWKWebView\n\n), mantendo a lógica de inteligência no daemon Rust imutável. A interface de interação do agente deve ser injetada via IPC passivo.

*   **Sandboxing e Segurança:**
    *   **Repúdio a Python/FastAPI/Docker:** A dependência de um servidor FastAPI em Python introduz latência desnecessária. O uso de Docker como mecanismo primário de isolamento é pesado e complexo para ambientes \n\nair-gapped\n\n. O isolamento de usuários baseado em contas Linux padrão dentro de um único container é considerado insuficiente.
    1.  **Adoção de Sandboxes Nativas em Rust:** A funcionalidade de terminal deve ser reescrita em Rust, utilizando primitivas Unix como \n\nchroot\n\n, \n\nnamespaces\n\n e \n\nseccomp\n\n para criar sandboxes de processo ultraleves. A comunicação deve ser via sockets Unix ou memória compartilhada, eliminando overhead de rede local.

*   **Orquestração e Gestão de Estado:**
    *   **Vibe Kanban:** Extrair a lógica de isolamento de agentes via \n\ngit worktree\n\n e o sistema de sincronização de tipos Rust-to-TypeScript via \n\nts-rs\n\n. A interface Kanban deve ser um componente React passivo alimentado via IPC pelo daemon central, eliminando a necessidade de um servidor HTTP Axum em execução contínvia. O banco de dados deve ser o SQLite segmentado do próprio SODA.
    *   **Trellis:** Rejeitar o framework CLI e adotar o modelo de dados. O Genesis MC deve implementar o conceito de \"Journaling\" e \"Specs\" como parte fundamental de sua memória de grafos, ingerindo essas informações no sistema RAG vetorial (Qdrant) e estrutural (SQLite) para recuperação de contexto por agentes efêmeros.

*   **Roteamento e Gestão de LLMs:**
    *   **llm-proxy:** Canibalizar a lógica de roteamento e esquemas de custo. O Genesis MC deve integrar um roteador bare-metal em Rust (utilizando \n\nhyper\n\n ou \n\nreqwest\n\n com suporte a HTTP/2 e HTTP/3) que realiza a troca de modelos em microssegundos. A configuração de roteamento deve ser armazenada no núcleo imutável ou em tabelas SQLite protegidas.

*   **Memória e Recuperação de Contexto (RAG):**
    *   **jgravelle/jcodemunch-mcp:** Absorver integralmente a especificação jMRI-Full e reescrever o servidor MCP em Rust. Utilizar bindings nativos de Rust para \n\ntree-sitter\n\n para indexar código-fonte local. O endereçamento por byte-offset deve ser integrado ao sistema de memória segmentada.
    *   **notebooklm-mcp-cli:** Tratar como um \"sidecar efêmero\". O Genesis MC não deve integrar este código ao núcleo. Permitir que o usuário ative um agente especializado que rode este script em um ambiente Wasm isolado apenas quando necessário. Monitorar a lógica RPC para evitar vazamento de dados sensíveis.

*   **Processamento Multimodal e Pipelines:**
    *   **google-gemini/genai-processors:** Absorver o conceito de \"Unified Content Model\" e a arquitetura de processadores compositíveis via operadores. Implementar este padrão em Rust, utilizando segurança de tipos e o modelo de \n\nownership\n\n para gerenciar streaming de dados multimodais entre agentes sem cópias de memória desnecessárias. A composição de pipelines deve ser definida em tempo de compilação ou via DAGs (Directed Acyclic Graphs) leves no daemon principal.

*   **Eficiência de Runtime e Memória:**
    *   **Prioridade Rust/Tokio:** O motor deve entregar resiliência suprema, controle total do sandboxing perene e confiabilidade incontestável de TPS (Transactions Per Second) plano e constante.
    *   **Eliminação de Python/Node.js:** Ambientes como interpretadores Python/Node.js inflam desastrosamente a sobrecarga passiva da CPU para abismos ineficientes.
    *   **Hardware Awareness:** Preservar otimizações bare-metal, limitações da iGPU (gargalos de barramento), e diretivas de execução AVX2 para a CPU ou llama.cpp mmap para a RTX 2060m.

*   **Arquitetura de Super Skills:**
    *   **Estrutura de Skill:** Uma Skill é uma PASTA inteira com `SKILL.md` (cérebro), `scripts/` (mãos), `references/` (memória) e `assets/` (templates).
    *   **Divulgação Progressiva (3 Níveis):**
        *   Nível 1: Apenas `name` e `description` (metadados).
        *   Nível 2: Corpo do `SKILL.md` (Máquina de Estados).
        *   Nível 3: Lógica procedimental em `scripts/` (execução via IA como caixa-preta).
    *   **Foco em Rust/Wasmtime/Sandboxing:** Habilidades devem ser escritas em Rust ou scripts que rodam em Wasmtime, com regras de sandboxing estritas.
    *   **Super Skills Fundacionais:** `@skill-creator`, `@weevolve` (Continuous Learning), `@notebooklm-context` (RAG), `@soda-governance` (GitOps), `@soda-sdd` (Spec-Driven Development), `@soda-rust-expert`.

*   **Geração Declarativa de UI e Cyber-Neuro Synthesis:**
    *   **Frontend Passivo:** Svelte 5 + Tauri v2. Toda lógica reside no Rust (IPC Zero-Copy).
    *   **Protocolo A2UI (Agent-to-User Interface):** Transmite \"Árvores de Intenção\" em JSON, não código executável. Utiliza Modelo de Lista de Adjacência para LLM-friendliness.
    *   **Catálogos Rígidos:** Esquemas JSON (JSON Schema Draft 2020-12) definem componentes permitidos, propriedades e tipos, proibindo a invenção pelo agente. Validação em Rust via `a2ui-rs`.
    *   **Cyber-Neuro Synthesis:** Estética focada em utilitarismo, monocromia, contraste, tipografia monoespaçada, alta densidade de layout, feedback percussivo e \n\nZero Layout Shifts\n\n absoluto. Prioriza a \"honestidade mecânica\" e a previsibilidade espacial para usuários 2e/TDAH.
    *   **Vidro Tátil:** Camadas de profundidade indexada com bordas nítidas e escurecimento exato, sem sombras difusas.
    *   **Instância Mecânica:** Elementos interativos não mimetizam comportamentos orgânicos; atuam como interruptores físicos. Rolagem mecânica, não inercial.
    *   **Previsibilidade Espacial:** Derivada parcial da posição em relação ao tempo é zero para elementos não ativos (\n\n\\frac{\\partial (x_i, y_i)}{\\partial t} = 0 \\quad \\forall i \\notin \\Omega_{ativa}\n\n).
    *   **First Draft Protocol:** Mecanismo de conciliação entre Agente de Design (UI) e Agente de Código (Backend). Rejeita intenções de UI que excedam limites de latência ou complexidade computacional.

*   **Otimização de Modelos Ultracompactos (CPU/GPU Híbrido):**
    *   **Hardware Alvo:** Intel Core i9 (14nm, AVX2), 32GB RAM, RTX 2060m (6GB VRAM).
    *   **Motores de Inferência:** `llama.cpp` (via `mmap` e `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para offloading), `bitnet.cpp` (para modelos ternários), `rust-llm` (para inferência nativa em Rust), `Candle` (framework ML nativo em Rust).
    *   **Quantização:** GGUF Q4_K_M, INT4, AWQ, GPTQ. Modelos de 1.58 bits (BitNet) para CPU.
    *   **Taxonomia de Modelos:** Parakeet TDT (ASR), Moonshine Tiny (ASR Wake Word), SmolVLM2-256M (VLM), BGE-micro/GTE-tiny (Embeddings), FunctionGemma 270M (Function Calling), Qwen3-0.6B (Orquestração Lógica), SmolLM2-135M (Cognição Básica), BitNet-b1.58-2B-4T (Cognição Complexa), Phi-4-mini (Cognição STEM), Kokoro-82M (TTS).
    *   **Pipeline Agêntico:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> Roteamento (BGE/GTE Embeddings) -> Orquestração (Qwen3/FunctionGemma) -> Inferência LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
    *   **Mitigação de Gargalos:** `llama-swap` para gerenciar VRAM, `PagedAttention` para KV Cache, `Zero-Copy IPC` para comunicação.

*   **FinOps IA e Roteamento Híbrido:**
    *   **Fronteira de Pareto:** Otimização de Custo, Qualidade e Latência.
    *   **Roteamento em Cascata:** Nível 0 (Local - Custo $0), Nível 1 (Subscription Workers - Custo Fixo), Nível 2 (Premium Pay-per-Token - Custo Variável).
    *   **Algoritmo ParetoBandit:** Adapta o roteamento com base no orçamento (\n\n\\lambda_t\n\n) e na qualidade predita (\n\nq_t\n\n).
    *   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem.
    *   **Execução Híbrida:** iGPU Intel para modelos leves/roteamento, RTX 2060m para LLMs e VLMs, RAM para offloading de modelos maiores.
    *   **Subscription Hacking:** Uso de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas.
    *   **Auto Exacto / OTLP:** Telemetria para monitorar a qualidade dos provedores de nuvem e ajustar o roteamento.

*   **Ambiente de Desenvolvimento Agente (Antigravity IDE):**
    *   **Orquestração Multi-Agente:** Agente \"Mission Control\" (Brain) orquestra \"Workers\" (sub-agentes) via MCP ou CLIs.
    *   **FinOps:** Uso de assinaturas (Claude Code, Gemini CLI) como workers para otimizar custos.
    *   **NotebookLM como Cérebro de Contexto:** Servidor `notebooklm-mcp` com padrão `Progressive Disclosure` para injetar contexto de forma eficiente.
    *   **Alternativas Open-Source:** `Docfork` para documentação e `GitMCP` para inspeção de repositórios em tempo real, substituindo Context7.
    *   **Governança Agêntica:** Fusão de `CC-SDD` (Espec. EARS, Mermaid) com `Superpowers` (TDD, Revisão Cruzada) executada em `Shadow Workspaces`.
    *   **Pipeline de Desenvolvimento:** Ciclos curtos de Planejar → Gerar → Rodar → Testar → Corrigir → Documentar, com DoD rigoroso.
    *   **Vibe Coding:** Guiado por prompts estruturados e ganchos de verificação, não por intuição.

*   **Isolamento e Sidecars Efêmeros:**
    *   **Rejeição de Docker no Núcleo:** O daemon SODA em Rust opera bare-metal.
    *   **Sidecars Efêmeros:** Ferramentas Python/Node.js (como Docling MCP) são empacotadas em contêineres descartáveis ou executadas via Wasmtime (Pyodide) para isolamento e morte automática após a tarefa.
    *   **Sandboxing:** Wasmtime para lógicas puras; Landlock (Linux) / Sandbox Nativo (Windows) para acesso a ferramentas do host (Git, Npm).
    *   **Segurança:** Proibição de acesso direto a rede/arquivos pelo Wasm. Uso de `MIN-MEM-SIZE`, `TRAP-ON-GROW-FAILURE`, `FUEL`, `NO-NETWORK`, `NO-INHERIT-ENV`, `NO-CACHE`.

---

**Auditoria Crítica (Furos e Conflitos):**

*   **Complexidade da Tríade de Memória:** A manutenção de SQLite, Qdrant e FalkorDB (ou sua substituição por LightRAG) em sincronia e com performance garantida em um ambiente de borda é um desafio de engenharia significativo. A latência de \n\ncommit\n\n em três vias pode ser um gargalo. **Mitigação:** Priorizar SQLite com extensões para todas as funções em v1, adiando Qdrant e grafos complexos para fases posteriores, ou explorar soluções mais leves e integradas em Rust.
*   **IPC Binário vs. React:** A transferência de \n\nArrayBuffer\n\n para o Web Worker é eficiente, mas a comunicação de volta do Worker para o React (eventos de estado, atualizações de UI) ainda pode introduzir latência se não for cuidadosamente gerenciada. O modelo de \"passagem de referência\" no \n\npostMessage\n\n é crucial.
*   **Dependência de LLMs Locais vs. Nuvem:** A estratégia de roteamento híbrido é sólida, mas a detecção de \"falha de qualidade\" ou \"regressão de desempenho\" em modelos locais (que podem ocorrer sem aviso prévio) precisa de um mecanismo de monitoramento robusto. O \"Esquecimento Geométrico\" é um bom começo, mas pode ser necessário um sistema de \n\nhealth check\n\n mais ativo para os modelos locais.
*   **Segurança do Sandboxing:** Embora Wasmtime e Landlock ofereçam isolamento forte, a interação do agente com ferramentas do host (Git, Docker via sidecars) requer um controle de permissões extremamente granular e auditável. A gestão de chaves SSH e credenciais de API para os sidecars é um ponto crítico de segurança.
*   **Manutenção do Ecossistema de Ferramentas:** A dependência de bibliotecas C++ como \n\nllama.cpp\n\n e \n\ntree-sitter\n\n, mesmo com \n\nbindings\n\n em Rust, introduz complexidade na cadeia de compilação e na gestão de dependências. A transição para alternativas puramente Rust (como Candle para inferência) deve ser avaliada continuamente.
*   **VRAM da RTX 2060m:** A estratégia de \n\nllama-swap\n\n e offloading para RAM é essencial, mas a performance será limitada pela largura de banda PCIe e pela latência da RAM. Modelos que exigem mais de 5.6GB de VRAM (mesmo quantizados) podem ainda apresentar gargalos. A priorização de modelos menores e mais eficientes para tarefas comuns é crucial.
*   **\"Subscription Hacking\" e Limites de API:** A dependência de assinaturas para tarefas pesadas introduz um risco de atingir limites de taxa (rate limits) ou de ter o acesso revogado. O roteador precisa de uma lógica de fallback mais sofisticada para lidar com falhas de serviço dos provedores de assinatura.

---

### **PARTE 4: O Roteamento Híbrido e a Fronteira de Pareto para FinOps IA**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** O roteamento de inferência deve otimizar Custo, Qualidade e Latência, operando sob uma Fronteira de Pareto matemática. A utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n governa as decisões, onde \n\nq\n\n é a qualidade, \n\nc\n\n o custo, \n\nl\n\n a latência, \n\n\\lambda\n\n o marcapasso orçamentário e \n\n\\beta\n\n o fator de sensibilidade à latência.
*   **Roteamento em Cascata:**
    *   **Nível 0 (Local - Custo $0):** Modelos leves (FunctionGemma 3 270M na iGPU) para roteamento semântico e tarefas triviais.
    *   **Nível 1 (Local - RTX 2060m):** Modelos de médio porte (Qwen 3.5 9B, DeepSeek-R1-14B quantizados) para raciocínio e código, com \n\nllama-swap\n\n para gerenciar VRAM.
    *   **Nível 2 (Subscription Workers):** CLIs de assinatura (Claude Code, Gemini CLI) para tarefas pesadas, com fallback para nuvem premium se limites forem atingidos.
    *   **Nível 3 (Premium APIs):** Modelos de fronteira (Claude Opus 4.6, GPT-5.4) para tarefas críticas e insolúveis localmente.
*   **Bandits Adaptativos e Esquecimento Geométrico:** O sistema aprende continuamente com o desempenho dos provedores de nuvem, ajustando o roteamento com base em métricas de qualidade e custo. O Esquecimento Geométrico (\n\n\\gamma \\in (0, 1)\n\n) descarta gradualmente dados de desempenho antigos para adaptar-se a mudanças nos provedores.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem para informar as decisões do ParetoBandit. Dados são ingeridos via OTLP e NATS JetStream para processamento assíncrono em Rust.
*   **Execução Bare-Metal:** O daemon SODA em Rust opera diretamente no hardware, sem Docker ou Python persistente. Ferramentas Python/Node.js são executadas em \n\nSidecars Efêmeros\n\n via Wasmtime ou subprocessos isolados.
*   **Sandboxing:** Wasmtime para lógicas puras; Landlock (Linux) / Sandbox Nativo (Windows) para acesso a ferramentas do host.
*   **Memória Híbrida:** L1 (RAM do Sistema para KV Cache dinâmico), L2 (SQLite com FTS5 para estado transacional/relacional), L3 (Qdrant para vetores semânticos).
*   **Comunicação IPC Zero-Copy:** Uso de canais binários no Tauri para transferir dados entre Rust e React, evitando serialização JSON.
*   **Modelos Ultracompactos:** Seleção de modelos como FunctionGemma 3 270M (roteamento), Qwen 3.5 9B/DeepSeek-R1-14B (raciocínio), Whisper.cpp/Silero VAD (áudio), SmolVLM2 (visão) para otimização em hardware restrito.
*   **Vibe Coding com DoD:** Prompts estruturados, `SKILL.md` para Divulgação Progressiva, `Scaffolding` completo, testes de fumaça e CI são mandatórios para qualquer feature.
*   **HITL (Human-in-the-Loop) com Auto-QA:** Aprovação humana para ações críticas, com feedback de um sub-agente QA.
*   **Otimização de Ferramentas:** Uso de `mcp2cli` (conceito) e `tooon-format` para reduzir o consumo de tokens em interações com APIs e esquemas MCP.
*   **Segurança:** `Secure-by-Construction` com `Constitutional Spec-Driven Development`, `Shadow Workspaces` para isolamento de código gerado por IA.

---

### **PARTE 4: O Roteamento Híbrido e a Fronteira de Pareto para FinOps IA**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** O roteamento de inferência deve otimizar Custo, Qualidade e Latência, operando sob uma Fronteira de Pareto matemática. A utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n governa as decisões, onde \n\nq\n\n é a qualidade, \n\nc\n\n o custo, \n\nl\n\n a latência, \n\n\\lambda\n\n o marcapasso orçamentário e \n\n\\beta\n\n o fator de sensibilidade à latência.
*   **Roteamento em Cascata:**
    *   **Nível 0 (Local - Custo $0):** Modelos leves (FunctionGemma 3 270M na iGPU) para roteamento semântico e tarefas triviais.
    *   **Nível 1 (Local - RTX 2060m):** Modelos de médio porte (Qwen 3.5 9B, DeepSeek-R1-14B quantizados) para raciocínio e código, com \n\nllama-swap\n\n para gerenciar VRAM.
    *   **Nível 2 (Subscription Workers):** CLIs de assinatura (Claude Code, Gemini CLI) para tarefas pesadas, com fallback para nuvem premium se limites forem atingidos.
    *   **Nível 3 (Premium APIs):** Modelos de fronteira (Claude 4.6 Opus, GPT-5.4) para tarefas críticas e insolúveis localmente.
*   **Bandits Adaptativos e Esquecimento Geométrico:** O sistema aprende continuamente com o desempenho dos provedores de nuvem, ajustando o roteamento com base em métricas de qualidade e custo. O Esquecimento Geométrico (\n\n\\gamma \\in (0, 1)\n\n) descarta gradualmente dados de desempenho antigos para adaptar-se a mudanças nos provedores.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem para informar as decisões do ParetoBandit. Dados são ingeridos via OTLP e NATS JetStream para processamento assíncrono em Rust.
*   **Execução Bare-Metal:** O daemon SODA em Rust opera diretamente no hardware, sem Docker ou Python persistente. Ferramentas Python/Node.js são executadas em \n\nSidecars Efêmeros\n\n via Wasmtime ou subprocessos isolados.
*   **Sandboxing:** Wasmtime para lógicas puras; Landlock (Linux) / Sandbox Nativo (Windows) para acesso a ferramentas do host.
*   **Memória Híbrida:** L1 (RAM do Sistema para KV Cache dinâmico), L2 (SQLite com FTS5 para estado transacional/relacional), L3 (Qdrant para vetores semânticos).
*   **Comunicação IPC Binário:** Uso de canais binários no Tauri para transferir dados entre Rust e React, evitando serialização JSON.
*   **Modelos Ultracompactos:** Seleção de modelos como FunctionGemma 3 270M (roteamento), Qwen 3.5 9B/DeepSeek-R1-14B (raciocínio), Whisper.cpp/Silero VAD (áudio), SmolVLM2 (visão) para otimização em hardware restrito.
*   **Vibe Coding com DoD:** Prompts estruturados, `SKILL.md` para Divulgação Progressiva, `Scaffolding` completo, testes de fumaça e CI são mandatórios para qualquer feature.
*   **HITL (Human-in-the-Loop) com Auto-QA:** Aprovação humana para ações críticas, com feedback de um sub-agente QA.
*   **Otimização de Ferramentas:** Uso de `mcp2cli` (conceito) e `tooon-format` para reduzir o consumo de tokens em interações com APIs e esquemas MCP.
*   **Segurança:** `Secure-by-Construction` com `Constitutional Spec-Driven Development`, `Shadow Workspaces` para isolamento de código gerado por IA.

---

### **PARTE 5: O Ambiente de Desenvolvimento Agente Otimizado**

**Axiomas e Conceitos Técnicos:**

*   **Antigravity IDE como \"Fábrica\":** O Antigravity IDE é a ferramenta que cria as ferramentas (as Skills). Ele orquestra múltiplos agentes e garante a adesão ao padrão `agentskills.io`.
*   **FinOps IA e \"Subscription Hacking\":** O uso de assinaturas de taxa fixa (Claude Pro, Gemini CLI) para tarefas pesadas em vez de APIs pay-per-token para otimizar custos. O roteador do Genesis MC gerencia o fallback para nuvem premium se os limites da assinatura forem atingidos.
*   **Agente Governador (Jarvis):** Um LLM centralizador que interage com o usuário, refina prompts, gerencia o escopo e despacha tarefas para sub-agentes especializados (Workers). Ele também atua como um \"sparring partner\" intelectual, forçando o usuário a confrontar suas ideias.
*   **`smolagents` como Ferramentas de Execução Letal:** LLMs pequenos e eficientes, escritos em Rust, que executam tarefas específicas (ex: manipulação de arquivos, cálculos matemáticos) em ambientes Wasm efêmeros, garantindo isolamento e morte automática após a conclusão.
*   **Sandboxing Híbrido:** Wasmtime para lógicas puras e isoladas. Para interações com o host (Git, Npm), uso de restrições nativas do SO (Landlock no Linux, Sandbox nativo no Windows) em vez de Docker.
*   **Canibalização de Repositórios Open-Source:** Extrair a lógica e os padrões (ex: `SKILL.md`, `bMAD`, `Ralph Loop`, `TDD`) de projetos como OpenClaw, Antigravity, Vibe Kanban, e reimplementá-los em Rust no núcleo do SODA.
*   **`SKILL.md` e Divulgação Progressiva:** O padrão para definir habilidades agênticas, permitindo que o LLM carregue apenas os metadados inicialmente e o conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto. O Antigravity IDE gerencia a integração final do código validado.
*   **Pipeline de Desenvolvimento:** Planejar → Gerar → Rodar → Testar → Corrigir → Documentar. Cada etapa é validada e registrada.

---

### **PARTE 6: VETOR ZETA (Arquitetura e Orquestração de MCPs)**

**Axiomas e Conceitos Técnicos:**

*   **MCP como Sistema Nervoso Central:** O Model Context Protocol (MCP) padroniza a comunicação entre agentes de IA e ferramentas externas, atuando como a espinha dorsal da integração.
*   **Maestro Roteador (Rust):** Um roteador de integração de altíssimo desempenho em Rust, responsável por orquestrar conexões, mitigar \n\ncold starts\n\n e catalogar servidores MCP open-source.
*   **Abstração de Contexto Tardio (Late Context Abstraction / Just-In-Time Context):** Evita o \n\nTool Bloat\n\n e \n\nContext Rot\n\n injetando metadados de ferramentas no prompt inicial e carregando esquemas detalhados apenas quando o LLM os invoca.
*   **RAG-MCP:** O roteador busca ferramentas relevantes em um espaço vetorial local (Qdrant/SQLite) com base na intenção do LLM, injetando apenas os esquemas necessários.
*   **Delegação Sub-Agêntica:** Para evitar sobrecarga do LLM principal, dados brutos retornados por ferramentas são processados por sub-agentes efêmeros, e apenas resumos são passados ao LLM.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo drasticamente o consumo de tokens e eliminando a necessidade de gerar código cliente específico. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Utilização do formato \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens em 30-60% em comparação com JSON.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) são iniciados e mantidos em estado \"aquecido\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Predictive Shaping:** O roteador prevê a necessidade de ferramentas com base no contexto e pré-aquece os servidores relevantes.
*   **Cache Hierárquico (L1/L2):** Uso de estruturas de dados em Rust (\n\nDashMap\n\n) para cache de resultados de ferramentas e metadados MCP na memória.
*   **Catálogo Curado de MCPs Táticos:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, Desktop Commander MCP, Knowledge Graph Memory, GitHub MCP Server, CodeGraphContext, DuckDuckGo MCP, Kindly MCP, Playwright MCP, PostgreSQL/SQLite MCP, Chroma MCP). Rejeição de soluções proprietárias e baseadas em nuvem.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime e cofres mestres para chaves e sessões.

---

### **PARTE 6: VETOR OMICRON (Ilhas WebGL e IPC Binário Zero-Copy)**

**Axiomas e Conceitos Técnicos:**

*   **Ilhas WebGL:** UI híbrida onde a renderização gráfica densa ocorre em Web Workers isolados, utilizando \n\nOffscreenCanvas\n\n e comunicação via IPC binário. Isso libera a Main Thread do React e evita \n\nCumulative Layout Shift\n\n (CLS).
*   **IPC Binário Zero-Copy:** Comunicação direta de \n\nArrayBuffer\n\n entre Rust e Web Worker, evitando serialização/desserialização JSON para latência mínima (<5ms).
*   **`three.wasm`:** Micro-motor WebAssembly para renderização WebGL, executado no Web Worker para processamento gráfico isolado.
*   **Leis Anti-Gargalo:**
    1.  **Extirpação Explícita de VRAM:** Uso de \n\nWEBGL_lose_context\n\n para liberar memória da GPU ao desmontar ilhas.
    2.  **Desacoplamento IPC:** Destruição concertada de referências e comunicação via \n\npostMessage\n\n para o Web Worker, com \n\nself.close()\n\n para o trabalhador.
    3.  **Proibição de Realocação Dinâmica:** Uso de pré-alocação rígida e \n\nMutação In-Place\n\n no Web Worker para evitar jank e fragmentação de VRAM.
*   **Rust Backend:** Gerencia o ciclo de vida dos modelos, a comunicação IPC e a lógica de orquestração.
*   **Frontend Passivo (React):** Apenas monta o \n\n<canvas>\n\n, transfere o controle para o Web Worker e recebe os resultados finais. Não executa lógica de renderização pesada.
*   **Segurança:** Isolamento do código WASM e controle rigoroso sobre as permissões de acesso ao hardware e ao sistema de arquivos.

---

### **PARTE 7: [Research] Melhoria do Genesis MC Produto e UX (Vantage - Durable Skills)**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Evolução Cognitiva:** O Genesis MC é um \"exoesqueleto cognitivo\" que não apenas automatiza, mas avalia, adapta e desenvolve as habilidades do usuário.
*   **Avaliação Furtiva (Stealth Assessment) e ECD (Evidence-Centered Design):** Medição passiva de habilidades duráveis (pensamento crítico, resiliência, criatividade) através de telemetria comportamental contínua, sem testes explícitos.
*   **GenUI (Generative UI) e Adaptação Neuro-Sensorial:** A interface se adapta em tempo real ao estado cognitivo do usuário (estresse, foco), priorizando \"distração zero\" e clareza para neurodiversos (2e/TDAH).
*   **Explicabilidade Absoluta (Skill Maps e Drill-down):** Transparência total sobre como as avaliações são feitas, permitindo ao usuário ver os dados exatos que levaram a uma pontuação.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, podendo \"fabricar atrito educacional\" (criar conflitos simulados) para treinar habilidades de resolução.
*   **Gêmeos Digitais (Digital Twins):** Simulações das proficiências do usuário para testar mudanças de design ou complexidade de tarefas antes de aplicá-las no ambiente real.
*   **RLHF Local (Reinforcement Learning from Human Feedback):** O usuário fornece feedback comparativo para alinhar as prioridades da IA com suas necessidades neurodiversas.
*   **Canibalização Contínua:** Extrair o \"ouro\" (lógica, heurísticas) de projetos open-source e reescrever em Rust, descartando o \"lixo tóxico\" (Python, Node.js, Docker).
*   **Motor Bare-Metal:** Otimização contínua para RTX 2060m e iGPU Intel.
*   **Roteamento Híbrido Causal:** Decisão dinâmica entre modelos locais e nuvem com base em custo, qualidade e latência.
*   **Sandboxing:** Wasmtime e restrições nativas para isolar agentes.
*   **Fricção Produtiva:** Desacelerar intencionalmente em pontos críticos para garantir qualidade e evitar dívida técnica, em contraste com a geração \"frictionless\".
*   **Orquestração Multi-Agente:** Padrões como \"Reflection and Planning\" e \"Human-in-the-Loop\" (HitL) para gerenciar interações complexas entre agentes e o usuário.
*   **Segurança \"Secure-by-Construction\":** Impor regras de segurança (CWE/MITRE) desde a especificação (prompt constitucional) para prevenir vulnerabilidades injetadas pela IA.
*   **Vibe Testing:** QA agentivo que avalia a \"sensação\" da interface, latência e fluidez das micro-animações, não apenas a funcionalidade binária.

---

### **PARTE 8: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HitL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 9: VETOR ZETA (Arquitetura e Orquestração de MCPs)**

**Axiomas e Conceitos Técnicos:**

*   **MCP como Sistema Nervoso Central:** Padroniza a comunicação entre agentes e ferramentas, crucial para a autonomia local.
*   **Maestro Roteador (Rust):** Orquestra conexões, mitiga \n\ncold starts\n\n e cataloga servidores MCP open-source.
*   **Abstração de Contexto Tardio (Late Context Abstraction / Just-In-Time Context):** Evita \n\nTool Bloat\n\n e \n\nContext Rot\n\n injetando metadados de ferramentas no prompt inicial e carregando esquemas detalhados apenas quando o LLM os invoca.
*   **RAG-MCP:** Roteador busca ferramentas relevantes em espaço vetorial local (Qdrant/SQLite) com base na intenção do LLM, injetando esquemas JIT.
*   **Delegação Sub-Agêntica:** Dados brutos de ferramentas são processados por sub-agentes efêmeros; apenas resumos são passados ao LLM principal.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização, reduzindo consumo de tokens em 40-60%.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Predictive Shaping:** Roteador prevê necessidades de ferramentas e pré-aquece servidores.
*   **Cache Hierárquico (L1/L2):** Uso de estruturas Rust (\n\nDashMap\n\n) para cache de resultados e metadados MCP na memória.
*   **Catálogo Curado de MCPs Táticos:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).
*   **Segurança:** Sanitização de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.

---

### **PARTE 10: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`mmap()`:** Utilizado pelo `llama.cpp` para mapear pesos do modelo diretamente da RAM, evitando cópias massivas.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Comunicação Assíncrona:** Uso de `tokio::sync::mpsc` Channels para comunicação entre componentes Rust, garantindo paralelismo e backpressure.

---

### **PARTE 9: [Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek V3.2) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 10: [Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 11: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 12: [Feature Update] MCP para CLI Agentes e Vantagens**

**Axiomas e Conceitos Técnicos:**

*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre LLMs e ferramentas externas.
*   **Abstração de Contexto Tardio (Late Context Abstraction):** Evita \n\nTool Bloat\n\n injetando metadados de ferramentas no prompt inicial e esquemas detalhados apenas quando o LLM os invoca.
*   **Conversão MCP para CLI:** Transforma servidores MCP em CLIs interativas, reduzindo o consumo de tokens e a complexidade para o LLM.
*   **Zero-Codegen:** Criação dinâmica de interfaces CLI a partir de esquemas MCP, sem necessidade de gerar código cliente prévio.
*   **Eficiência TOON:** Uso do formato \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens em 40-60% em comparação com JSON.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local, em vez de injeção massiva de esquemas.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 13: [Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 14: [Product Vision] Genesis MC O Exoesqueleto Cognitivo e a Simbiose Humano-Máquina**

**Axiomas e Conceitos Técnicos:**

*   **Exoesqueleto Cognitivo:** O Genesis MC não é apenas uma ferramenta, mas uma extensão da mente do usuário, avaliando e adaptando-se em tempo real.
*   **Habilidades Duráveis:** Foco na medição e desenvolvimento de pensamento crítico, resiliência, colaboração e criatividade.
*   **Avaliação Furtiva (Stealth Assessment) e ECD (Evidence-Centered Design):** Medição passiva de habilidades através de telemetria comportamental, sem testes explícitos.
*   **GenUI (Generative UI) e Adaptação Neuro-Sensorial:** A interface se adapta ao estado cognitivo do usuário (estresse, foco), priorizando clareza e controle para neurodiversos (2e/TDAH).
*   **Explicabilidade Absoluta (Skill Maps e Drill-down):** Transparência total sobre como as avaliações são feitas, permitindo ao usuário ver os dados que basearam as decisões.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, podendo \"fabricar atrito educacional\" para treinar habilidades.
*   **Gêmeos Digitais (Digital Twins):** Simulações das proficiências do usuário para testar mudanças antes de aplicá-las no ambiente real.
*   **RLHF Local:** O usuário fornece feedback comparativo para alinhar as prioridades da IA com suas necessidades.
*   **Canibalização Contínua:** Extrair o \"ouro\" (lógica, heurísticas) de projetos open-source e reescrever em Rust, descartando o \"lixo tóxico\" (Python, Node.js, Docker).
*   **Motor Bare-Metal:** Otimização contínua para hardware específico.
*   **Roteamento Híbrido Causal:** Decisão dinâmica entre modelos locais e nuvem com base em custo, qualidade e latência.
*   **Sandboxing:** Wasmtime e restrições nativas para isolar agentes.
*   **Fricção Produtiva:** Desacelerar intencionalmente em pontos críticos para garantir qualidade e evitar dívida técnica.
*   **Orquestração Multi-Agente:** Padrões como \"Reflection and Planning\" e \"Human-in-the-Loop\" (HitL) para gerenciar interações complexas.
*   **Segurança \"Secure-by-Construction\":** Impor regras de segurança desde a especificação para prevenir vulnerabilidades injetadas pela IA.
*   **Vibe Testing:** QA agentivo que avalia a \"sensação\" da interface, latência e fluidez das micro-animações.

---

### **PARTE 15: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 16: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 17: [Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Canibalização de Ferramentas:** Extrair a lógica de ferramentas open-source e reimplementar em Rust ou executar em Wasm.

---

### **PARTE 18: [Updated Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 19: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 20: Arquitetura Fractal SODA**

**Axiomas e Conceitos Técnicos:**

*   **Revelação Progressiva:** Documentação organizada em árvore, acessada sob demanda pelo agente, não em um fluxo linear.
*   **Estrutura de Diretórios:** `/docs` contendo `MANIFESTO_SODA.md`, `AGENTS_PROTOCOL.md`, `/architecture` (macro), `/adrs` (decisões), `/vectors` (frentes de batalha), `/milestones` (PRDs acionáveis).
*   **Concisão:** Cada arquivo deve ser hiper-conciso.
*   **Protocolo de Orquestração do Agente (BMAD adaptado):**
    1.  **Ingestão Cirúrgica:** Usar `@mcp-jcodemunch-master` para AST, não ler o repo inteiro.
    2.  **Destilação da \"Alma Matemática\":** Eliminar dependências Python/Node.js/Docker; focar em Rust/Wasmtime/SQLite.
    3.  **Geração da Taxonomia de 3 Níveis:** `SKILL.md` com metadados (Nível 1), corpo (Nível 2), e recursos (Nível 3).
*   **PRD Granular (Milestone):** Define objetivo atômico, restrições (proibições), critérios de aceite (DoD) e tarefas para o próximo ciclo de desenvolvimento.

---

### **PARTE 21: [Feature Update] MCP para CLI Agentes e Vantagens**

**Axiomas e Conceitos Técnicos:**

*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Abstração de Contexto Tardio (Late Context Abstraction):** Evita \n\nTool Bloat\n\n injetando metadados de ferramentas no prompt inicial e esquemas detalhados apenas quando o LLM os invoca.
*   **Conversão MCP para CLI:** Transforma servidores MCP em CLIs interativas, reduzindo o consumo de tokens e a complexidade para o LLM.
*   **Zero-Codegen:** Criação dinâmica de interfaces CLI a partir de esquemas MCP, sem necessidade de gerar código cliente prévio.
*   **Eficiência TOON:** Uso do formato \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 22: [Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 23: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 24: [Product Vision] Genesis MC O Exoesqueleto Cognitivo e a Simbiose Humano-Máquina**

**Axiomas e Conceitos Técnicos:**

*   **Exoesqueleto Cognitivo:** O Genesis MC é uma extensão da mente do usuário, avaliando e adaptando-se em tempo real.
*   **Habilidades Duráveis:** Foco na medição e desenvolvimento de pensamento crítico, resiliência, colaboração e criatividade.
*   **Avaliação Furtiva (Stealth Assessment) e ECD (Evidence-Centered Design):** Medição passiva de habilidades através de telemetria comportamental, sem testes explícitos.
*   **GenUI (Generative UI) e Adaptação Neuro-Sensorial:** A interface se adapta ao estado cognitivo do usuário (estresse, foco), priorizando clareza e controle para neurodiversos (2e/TDAH).
*   **Explicabilidade Absoluta (Skill Maps e Drill-down):** Transparência total sobre como as avaliações são feitas, permitindo ao usuário ver os dados que basearam as decisões.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, podendo \"fabricar atrito educacional\" para treinar habilidades.
*   **Gêmeos Digitais (Digital Twins):** Simulações das proficiências do usuário para testar mudanças antes de aplicá-las no ambiente real.
*   **RLHF Local:** O usuário fornece feedback comparativo para alinhar as prioridades da IA com suas necessidades.
*   **Canibalização Contínua:** Extrair o \"ouro\" (lógica, heurísticas) de projetos open-source e reescrever em Rust, descartando o \"lixo tóxico\" (Python, Node.js, Docker).
*   **Motor Bare-Metal:** Otimização contínua para hardware específico.
*   **Roteamento Híbrido Causal:** Decisão dinâmica entre modelos locais e nuvem com base em custo, qualidade e latência.
*   **Sandboxing:** Wasmtime e restrições nativas para isolar agentes.
*   **Fricção Produtiva:** Desacelerar intencionalmente em pontos críticos para garantir qualidade e evitar dívida técnica.
*   **Orquestração Multi-Agente:** Padrões como \"Reflection and Planning\" e \"Human-in-the-Loop\" (HitL) para gerenciar interações complexas.
*   **Segurança \"Secure-by-Construction\":** Impor regras de segurança desde a especificação para prevenir vulnerabilidades injetadas pela IA.
*   **Vibe Testing:** QA agentivo que avalia a \"sensação\" da interface, latência e fluidez das micro-animações.

---

### **PARTE 25: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 26: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 27: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 28: Arquitetura Fractal SODA**

**Axiomas e Conceitos Técnicos:**

*   **Revelação Progressiva:** Documentação organizada em árvore, acessada sob demanda pelo agente, não em um fluxo linear.
*   **Estrutura de Diretórios:** `/docs` contendo `MANIFESTO_SODA.md`, `AGENTS_PROTOCOL.md`, `/architecture` (macro), `/adrs` (decisões), `/vectors` (frentes de batalha), `/milestones` (PRDs acionáveis).
*   **Concisão:** Cada arquivo deve ser hiper-conciso.
*   **Protocolo de Orquestração do Agente (BMAD adaptado):**
    1.  **Ingestão Cirúrgica:** Usar `@mcp-jcodemunch-master` para AST, não ler o repo inteiro.
    2.  **Destilação da \"Alma Matemática\":** Eliminar dependências Python/Node.js/Docker; focar em Rust/Wasmtime/SQLite.
    3.  **Geração da Taxonomia de 3 Níveis:** `SKILL.md` com metadados (Nível 1), corpo (Nível 2), e recursos (Nível 3).
*   **PRD Granular (Milestone):** Define objetivo atômico, restrições (proibições), critérios de aceite (DoD) e tarefas para o próximo ciclo de desenvolvimento.

---

### **PARTE 29: [Product Vision] Genesis MC O Exoesqueleto Cognitivo e a Simbiose Humano-Máquina**

**Axiomas e Conceitos Técnicos:**

*   **Exoesqueleto Cognitivo:** O Genesis MC é uma extensão da mente do usuário, avaliando e adaptando-se em tempo real.
*   **Habilidades Duráveis:** Foco na medição e desenvolvimento de pensamento crítico, resiliência, colaboração e criatividade.
*   **Avaliação Furtiva (Stealth Assessment) e ECD (Evidence-Centered Design):** Medição passiva de habilidades através de telemetria comportamental, sem testes explícitos.
*   **GenUI (Generative UI) e Adaptação Neuro-Sensorial:** A interface se adapta ao estado cognitivo do usuário (estresse, foco), priorizando clareza e controle para neurodiversos (2e/TDAH).
*   **Explicabilidade Absoluta (Skill Maps e Drill-down):** Transparência total sobre como as avaliações são feitas, permitindo ao usuário ver os dados que basearam as decisões.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, podendo \"fabricar atrito educacional\" para treinar habilidades.
*   **Gêmeos Digitais (Digital Twins):** Simulações das proficiências do usuário para testar mudanças antes de aplicá-las no ambiente real.
*   **RLHF Local:** O usuário fornece feedback comparativo para alinhar as prioridades da IA com suas necessidades.
*   **Canibalização Contínua:** Extrair o \"ouro\" (lógica, heurísticas) de projetos open-source e reescrever em Rust, descartando o \"lixo tóxico\" (Python, Node.js, Docker).
*   **Motor Bare-Metal:** Otimização contínua para hardware específico.
*   **Roteamento Híbrido Causal:** Decisão dinâmica entre modelos locais e nuvem com base em custo, qualidade e latência.
*   **Sandboxing:** Wasmtime e restrições nativas para isolar agentes.
*   **Fricção Produtiva:** Desacelerar intencionalmente em pontos críticos para garantir qualidade e evitar dívida técnica.
*   **Orquestração Multi-Agente:** Padrões como \"Reflection and Planning\" e \"Human-in-the-Loop\" (HitL) para gerenciar interações complexas.
*   **Segurança \"Secure-by-Construction\":** Impor regras de segurança desde a especificação para prevenir vulnerabilidades injetadas pela IA.
*   **Vibe Testing:** QA agentivo que avalia a \"sensação\" da interface, latência e fluidez das micro-animações.

---

### **PARTE 30: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 28: Arquitetura Fractal SODA**

**Axiomas e Conceitos Técnicos:**

*   **Revelação Progressiva:** Documentação organizada em árvore, acessada sob demanda pelo agente, não em um fluxo linear.
*   **Estrutura de Diretórios:** `/docs` contendo `MANIFESTO_SODA.md`, `AGENTS_PROTOCOL.md`, `/architecture` (macro), `/adrs` (decisões), `/vectors` (frentes de batalha), `/milestones` (PRDs acionáveis).
*   **Concisão:** Cada arquivo deve ser hiper-conciso.
*   **Protocolo de Orquestração do Agente (BMAD adaptado):**
    1.  **Ingestão Cirúrgica:** Usar `@mcp-jcodemunch-master` para AST, não ler o repo inteiro.
    2.  **Destilação da \"Alma Matemática\":** Eliminar dependências Python/Node.js/Docker; focar em Rust/Wasmtime/SQLite.
    3.  **Geração da Taxonomia de 3 Níveis:** `SKILL.md` com metadados (Nível 1), corpo (Nível 2), e recursos (Nível 3).
*   **PRD Granular (Milestone):** Define objetivo atômico, restrições (proibições), critérios de aceite (DoD) e tarefas para o próximo ciclo de desenvolvimento.

---

### **PARTE 29: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 30: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 31: Arquitetura de Isolamento e Sidecars Efêmeros no Antigravity IDE**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** O núcleo do Genesis MC deve ser Rust/Tauri puro, sem dependências persistentes de Python/Node.js.
*   **Sidecars Efêmeros:** Ferramentas Python/Node.js (como Docling MCP) são empacotadas em contêineres descartáveis ou executadas via Wasmtime (Pyodide) para isolamento e morte automática após a tarefa.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para acesso a ferramentas do host.
*   **`Dockerfile` Otimizado:** Uso de imagens `uv:bookworm-slim`, flags de bytecode (`UV_COMPILE_BYTECODE=1`), e inclusão mínima de dependências de sistema (`libgl1`, `libglib2.0-0`).
*   **`SKILL.md` para Habilidades:** Padrão para definir habilidades, com foco em extrair lógica em Rust/Wasm e evitar runtimes externos persistentes.
*   **Segurança:** Isolamento de recursos, proteção contra upgrades de SO, ponte segura de chaves SSH (via Rust), e rejeição de Docker pesado para sandboxing.
*   **Humanizer:** Lógica de detecção de padrões de escrita de IA absorvida no pipeline de post-processing Rust.
*   **Kreuzberg:** Adotado como biblioteca (crate) nativa em Rust para parsing de documentos, com uso opcional de ONNX Runtime para embeddings.
*   **Governança Nativa:** Lógica de regras aninhadas do Ruler reimplementada em Rust, gerenciando metadados dinâmicos em vez de scripts externos.

---

### **PARTE 32: VETOR OMICRON (Ilhas WebGL e IPC Binário Zero-Copy)**

**Axiomas e Conceitos Técnicos:**

*   **Ilhas WebGL:** UI híbrida onde a renderização gráfica densa ocorre em Web Workers isolados, utilizando \n\nOffscreenCanvas\n\n e comunicação via IPC binário. Isso libera a Main Thread do React e evita \n\nCumulative Layout Shift\n\n (CLS).
*   **IPC Binário Zero-Copy:** Comunicação direta de \n\nArrayBuffer\n\n entre Rust e Web Worker, evitando serialização/desserialização JSON para latência mínima (<5ms).
*   **`three.wasm`:** Micro-motor WebAssembly para renderização WebGL, executado no Web Worker para processamento gráfico isolado.
*   **Leis Anti-Gargalo:**
    1.  **Extirpação Explícita de VRAM:** Uso de \n\nWEBGL_lose_context\n\n para liberar memória da GPU ao desmontar ilhas.
    2.  **Desacoplamento IPC:** Destruição concertada de referências e comunicação via \n\npostMessage\n\n para o Web Worker, com \n\nself.close()\n\n para o trabalhador.
    3.  **Proibição de Realocação Dinâmica:** Uso de pré-alocação rígida e \n\nMutação In-Place\n\n no Web Worker para evitar jank e fragmentação de VRAM.
*   **Rust Backend:** Gerencia o ciclo de vida dos modelos, a comunicação IPC e a lógica de orquestração.
*   **Frontend Passivo (React):** Apenas monta o \n\n<canvas>\n\n, transfere o controle para o Web Worker e recebe os resultados finais. Não executa lógica de renderização pesada.

---

### **PARTE 33: [Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 34: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 34: Genesis MC: Arquitetura e Estratégia para Sistemas Operacionais Agênticos**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 35: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 36: Melhores Práticas - Vibe Coding**

**Axiomas e Conceitos Técnicos:**

*   **Foco em Software Integrado e Testado:** LLMs otimizam para texto plausível, não para software completo.
*   **Especificação Clara:** Prompts devem incluir escopo, critérios de aceitação, não-funcionais (performance, segurança) e constraints do ambiente.
*   **Feedback Executável:** Código deve ser rodado e testado para fornecer feedback real à IA.
*   **Ciclos Curtos:** Planejar → Gerar → Rodar → Testar → Corrigir → Documentar.
*   **Scaffold Completo:** Pedir um esqueleto rodável com Dockerfile, Makefile, README, testes, CI, migrations/seed antes de features.
*   **DoD (Definition of Done):** Checklist mínimo para cada entrega (funciona local, README, .env.example, testes, logs, CI, estrutura de pastas).
*   **Anti-Padrões:** Evitar pedir \"faz X\" sem DoD, ignorar ambiente, pular scaffold, deixar docs para o fim.
*   **Quando \"Vibe Coding\" é Bom:** Spikes/exploração, geração de boilerplate (com DoD), produção de alternativas.

---

### **PARTE 37: [Product Vision] Genesis MC O Exoesqueleto Cognitivo e a Simbiose Humano-Máquina**

**Axiomas e Conceitos Técnicos:**

*   **Exoesqueleto Cognitivo:** O Genesis MC é uma extensão da mente do usuário, avaliando e adaptando-se em tempo real.
*   **Habilidades Duráveis:** Foco na medição e desenvolvimento de pensamento crítico, resiliência, colaboração e criatividade.
*   **Avaliação Furtiva (Stealth Assessment) e ECD (Evidence-Centered Design):** Medição passiva de habilidades através de telemetria comportamental, sem testes explícitos.
*   **GenUI (Generative UI) e Adaptação Neuro-Sensorial:** A interface se adapta ao estado cognitivo do usuário (estresse, foco), priorizando clareza e controle para neurodiversos (2e/TDAH).
*   **Explicabilidade Absoluta (Skill Maps e Drill-down):** Transparência total sobre como as avaliações são feitas, permitindo ao usuário ver os dados que basearam as decisões.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, podendo \"fabricar atrito educacional\" para treinar habilidades.
*   **Gêmeos Digitais (Digital Twins):** Simulações das proficiências do usuário para testar mudanças antes de aplicá-las no ambiente real.
*   **RLHF Local:** O usuário fornece feedback comparativo para alinhar as prioridades da IA com suas necessidades.
*   **Canibalização Contínua:** Extrair o \"ouro\" (lógica, heurísticas) de projetos open-source e reescrever em Rust, descartando o \"lixo tóxico\" (Python, Node.js, Docker).
*   **Motor Bare-Metal:** Otimização contínua para hardware específico.
*   **Roteamento Híbrido Causal:** Decisão dinâmica entre modelos locais e nuvem com base em custo, qualidade e latência.
*   **Sandboxing:** Wasmtime e restrições nativas para isolar agentes.
*   **Fricção Produtiva:** Desacelerar intencionalmente em pontos críticos para garantir qualidade e evitar dívida técnica.
*   **Orquestração Multi-Agente:** Padrões como \"Reflection and Planning\" e \"Human-in-the-Loop\" (HitL) para gerenciar interações complexas.
*   **Segurança \"Secure-by-Construction\":** Impor regras de segurança desde a especificação para prevenir vulnerabilidades injetadas pela IA.
*   **Vibe Testing:** QA agentivo que avalia a \"sensação\" da interface, latência e fluidez das micro-animações.

---

### **PARTE 38: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 39: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 40: Melhores Práticas - Vibe Coding**

**Axiomas e Conceitos Técnicos:**

*   **Foco em Software Integrado e Testado:** LLMs otimizam para texto plausível, não para software completo.
*   **Especificação Clara:** Prompts devem incluir escopo, critérios de aceitação, não-funcionais (performance, segurança) e constraints do ambiente.
*   **Feedback Executável:** Código deve ser rodado e testado para fornecer feedback real à IA.
*   **Ciclos Curtos:** Planejar → Gerar → Rodar → Testar → Corrigir → Documentar.
*   **Scaffold Completo:** Pedir um esqueleto rodável com Dockerfile, Makefile, README, testes, CI, migrations/seed antes de features.
*   **DoD (Definition of Done):** Checklist mínimo para cada entrega (funciona local, README, .env.example, testes, logs, CI, estrutura de pastas).
*   **Anti-Padrões:** Evitar pedir \"faz X\" sem DoD, ignorar ambiente, pular scaffold, deixar docs para o fim.
*   **Quando \"Vibe Coding\" é Bom:** Spikes/exploração, geração de boilerplate (com DoD), produção de alternativas.

---

### **PARTE 41: [Research] Melhoria do Genesis MC Produto e UX (Vantage - Durable Skills)**

**Axiomas e Conceitos Técnicos:**

*   **Exoesqueleto Cognitivo:** O Genesis MC é uma extensão da mente do usuário, avaliando e adaptando-se em tempo real.
*   **Habilidades Duráveis:** Foco na medição e desenvolvimento de pensamento crítico, resiliência, colaboração e criatividade.
*   **Avaliação Furtiva (Stealth Assessment) e ECD (Evidence-Centered Design):** Medição passiva de habilidades através de telemetria comportamental, sem testes explícitos.
*   **GenUI (Generative UI) e Adaptação Neuro-Sensorial:** A interface se adapta ao estado cognitivo do usuário (estresse, foco), priorizando clareza e controle para neurodiversos (2e/TDAH).
*   **Explicabilidade Absoluta (Skill Maps e Drill-down):** Transparência total sobre como as avaliações são feitas, permitindo ao usuário ver os dados que basearam as decisões.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, podendo \"fabricar atrito educacional\" para treinar habilidades.
*   **Gêmeos Digitais (Digital Twins):** Simulações das proficiências do usuário para testar mudanças antes de aplicá-las no ambiente real.
*   **RLHF Local:** O usuário fornece feedback comparativo para alinhar as prioridades da IA com suas necessidades.
*   **Canibalização Contínua:** Extrair o \"ouro\" (lógica, heurísticas) de projetos open-source e reescrever em Rust, descartando o \"lixo tóxico\" (Python, Node.js, Docker).
*   **Motor Bare-Metal:** Otimização contínua para hardware específico.
*   **Roteamento Híbrido Causal:** Decisão dinâmica entre modelos locais e nuvem com base em custo, qualidade e latência.
*   **Sandboxing:** Wasmtime e restrições nativas para isolar agentes.
*   **Fricção Produtiva:** Desacelerar intencionalmente em pontos críticos para garantir qualidade e evitar dívida técnica.
*   **Orquestração Multi-Agente:** Padrões como \"Reflection and Planning\" e \"Human-in-the-Loop\" (HitL) para gerenciar interações complexas.
*   **Segurança \"Secure-by-Construction\":** Impor regras de segurança desde a especificação para prevenir vulnerabilidades injetadas pela IA.
*   **Vibe Testing:** QA agentivo que avalia a \"sensação\" da interface, latência e fluidez das micro-animações.

---

### **PARTE 42: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 43: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 44: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 45: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 46: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 47: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 48: Arquitetura Fractal SODA**

**Axiomas e Conceitos Técnicos:**

*   **Revelação Progressiva:** Documentação organizada em árvore, acessada sob demanda pelo agente, não em um fluxo linear.
*   **Estrutura de Diretórios:** `/docs` contendo `MANIFESTO_SODA.md`, `AGENTS_PROTOCOL.md`, `/architecture` (macro), `/adrs` (decisões), `/vectors` (frentes de batalha), `/milestones` (PRDs acionáveis).
*   **Concisão:** Cada arquivo deve ser hiper-conciso.
*   **Protocolo de Orquestração do Agente (BMAD adaptado):**
    1.  **Ingestão Cirúrgica:** Usar `@mcp-jcodemunch-master` para AST, não ler o repo inteiro.
    2.  **Destilação da \"Alma Matemática\":** Eliminar dependências Python/Node.js/Docker; focar em Rust/Wasmtime/SQLite.
    3.  **Geração da Taxonomia de 3 Níveis:** `SKILL.md` com metadados (Nível 1), corpo (Nível 2), e recursos (Nível 3).
*   **PRD Granular (Milestone):** Define objetivo atômico, restrições (proibições), critérios de aceite (DoD) e tarefas para o próximo ciclo de desenvolvimento.

---

### **PARTE 49: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 50: Melhores Práticas - Vibe Coding**

**Axiomas e Conceitos Técnicos:**

*   **Foco em Software Integrado e Testado:** LLMs otimizam para texto plausível, não para software completo.
*   **Especificação Clara:** Prompts devem incluir escopo, critérios de aceitação, não-funcionais (performance, segurança) e constraints do ambiente.
*   **Feedback Executável:** Código deve ser rodado e testado para fornecer feedback real à IA.
*   **Ciclos Curtos:** Planejar → Gerar → Rodar → Testar → Corrigir → Documentar.
*   **Scaffold Completo:** Pedir um esqueleto rodável com Dockerfile, Makefile, README, testes, CI, migrations/seed antes de features.
*   **DoD (Definition of Done):** Checklist mínimo para cada entrega (funciona local, README, .env.example, testes, logs, CI, estrutura de pastas).
*   **Anti-Padrões:** Evitar pedir \"faz X\" sem DoD, ignorar ambiente, pular scaffold, deixar docs para o fim.
*   **Quando \"Vibe Coding\" é Bom:** Spikes/exploração, geração de boilerplate (com DoD), produção de alternativas.

---

### **PARTE 51: [Research] Melhoria do Genesis MC Produto e UX (Vantage - Durable Skills)**

**Axiomas e Conceitos Técnicos:**

*   **Exoesqueleto Cognitivo:** O Genesis MC é uma extensão da mente do usuário, avaliando e adaptando-se em tempo real.
*   **Habilidades Duráveis:** Foco na medição e desenvolvimento de pensamento crítico, resiliência, colaboração e criatividade.
*   **Avaliação Furtiva (Stealth Assessment) e ECD (Evidence-Centered Design):** Medição passiva de habilidades através de telemetria comportamental, sem testes explícitos.
*   **GenUI (Generative UI) e Adaptação Neuro-Sensorial:** A interface se adapta ao estado cognitivo do usuário (estresse, foco), priorizando clareza e controle para neurodiversos (2e/TDAH).
*   **Explicabilidade Absoluta (Skill Maps e Drill-down):** Transparência total sobre como as avaliações são feitas, permitindo ao usuário ver os dados que basearam as decisões.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, podendo \"fabricar atrito educacional\" para treinar habilidades.
*   **Gêmeos Digitais (Digital Twins):** Simulações das proficiências do usuário para testar mudanças antes de aplicá-las no ambiente real.
*   **RLHF Local:** O usuário fornece feedback comparativo para alinhar as prioridades da IA com suas necessidades.
*   **Canibalização Contínua:** Extrair o \"ouro\" (lógica, heurísticas) de projetos open-source e reescrever em Rust, descartando o \"lixo tóxico\" (Python, Node.js, Docker).
*   **Motor Bare-Metal:** Otimização contínua para hardware específico.
*   **Roteamento Híbrido Causal:** Decisão dinâmica entre modelos locais e nuvem com base em custo, qualidade e latência.
*   **Sandboxing:** Wasmtime e restrições nativas para isolar agentes.
*   **Fricção Produtiva:** Desacelerar intencionalmente em pontos críticos para garantir qualidade e evitar dívida técnica.
*   **Orquestração Multi-Agente:** Padrões como \"Reflection and Planning\" e \"Human-in-the-Loop\" (HitL) para gerenciar interações complexas.
*   **Segurança \"Secure-by-Construction\":** Impor regras de segurança desde a especificação para prevenir vulnerabilidades injetadas pela IA.
*   **Vibe Testing:** QA agentivo que avalia a \"sensação\" da interface, latência e fluidez das micro-animações.

---

### **PARTE 52: Arquitetura de Isolamento e Sidecars Efêmeros no Antigravity IDE**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** O núcleo do Genesis MC deve ser Rust/Tauri puro, sem dependências persistentes de Python/Node.js.
*   **Sidecars Efêmeros:** Ferramentas Python/Node.js (como Docling MCP) são empacotadas em contêineres descartáveis ou executadas via Wasmtime (Pyodide) para isolamento e morte automática após a tarefa.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para acesso a ferramentas do host.
*   **`Dockerfile` Otimizado:** Uso de imagens `uv:bookworm-slim`, flags de bytecode (`UV_COMPILE_BYTECODE=1`), e inclusão mínima de dependências de sistema (`libgl1`, `libglib2.0-0`).
*   **`SKILL.md` para Habilidades:** Padrão para definir habilidades agênticas, com foco em extrair lógica em Rust/Wasm e evitar runtimes externos persistentes.
*   **Segurança:** Isolamento de recursos, proteção contra upgrades de SO, ponte segura de chaves SSH (via Rust), e rejeição de Docker pesado para sandboxing.
*   **Humanizer:** Lógica de detecção de padrões de escrita de IA absorvida no pipeline de post-processing Rust.
*   **Kreuzberg:** Adotado como biblioteca (crate) nativa em Rust para parsing de documentos, com uso opcional de ONNX Runtime para embeddings.
*   **Governança Nativa:** Lógica de regras aninhadas do Ruler reimplementada em Rust, gerenciando metadados dinâmicos em vez de scripts externos.

---

### **PARTE 53: VETOR OMICRON (Ilhas WebGL e IPC Binário Zero-Copy)**

**Axiomas e Conceitos Técnicos:**

*   **Ilhas WebGL:** UI híbrida onde a renderização gráfica densa ocorre em Web Workers isolados, utilizando \n\nOffscreenCanvas\n\n e comunicação via IPC binário. Isso libera a Main Thread do React e evita \n\nCumulative Layout Shift\n\n (CLS).
*   **IPC Binário Zero-Copy:** Comunicação direta de \n\nArrayBuffer\n\n entre Rust e Web Worker, evitando serialização/desserialização JSON para latência mínima (<5ms).
*   **`three.wasm`:** Micro-motor WebAssembly para renderização WebGL, executado no Web Worker para processamento gráfico isolado.
*   **Leis Anti-Gargalo:**
    1.  **Extirpação Explícita de VRAM:** Uso de \n\nWEBGL_lose_context\n\n para liberar memória da GPU ao desmontar ilhas.
    2.  **Desacoplamento IPC:** Destruição concertada de referências e comunicação via \n\npostMessage\n\n para o Web Worker, com \n\nself.close()\n\n para o trabalhador.
    3.  **Proibição de Realocação Dinâmica:** Uso de pré-alocação rígida e \n\nMutação In-Place\n\n no Web Worker para evitar jank e fragmentação de VRAM.
*   **Rust Backend:** Gerencia o ciclo de vida dos modelos, a comunicação IPC e a lógica de orquestração.
*   **Frontend Passivo (React):** Apenas monta o \n\n<canvas>\n\n, transfere o controle para o Web Worker e recebe os resultados finais. Não executa lógica de renderização pesada.

---

### **PARTE 54: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 55: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 56: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 57: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 58: Arquitetura Genesis: Orquestração Nativa e Sistemas Agênticos de Alta Performance**

**Axiomas e Conceitos Técnicos:**

*   **Rejeição de Python/Node.js no Núcleo:** Uso de \n\nSidecars Efêmeros\n\n e \n\nSandboxing\n\n (Wasmtime) para ferramentas externas, mantendo o núcleo em Rust.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Adiamento de FalkorDB e representações 3D/Sonoras.
*   **Simplificação de Memória:** Uso de SQLite com extensão \n\nsqlite-vec\n\n como memória unificada inicial.
*   **Canibalização de Lógica:** Extrair e reescrever em Rust a lógica de orquestração, busca e parsing de projetos como AgenticSeek, Antigravity, OpenClaw, Sanity-Gravity, Humanizer, Kreuzberg, Ruler.
*   **Interface Passiva React/Tauri:** UI reativa que apenas desenha dados do backend Rust, sem lógica de negócios.
*   **Ralph Loop:** Agentes são criados, executam tarefas, rodam testes e são mortos, recriando-se com contexto limpo para a próxima iteração.
*   **Backpressure Determinístico:** Testes de compilador e linters como principal feedback para a IA, garantindo \n\nexit code 0\n\n para conclusão de tarefa.
*   **O(1) Code Retrieval:** Uso de \n\ntree-sitter\n\n para extrair código via AST, economizando tokens.
*   **Gateway MCP Dinâmico:** Injeção de ferramentas via \n\nSKILL.md\n\n e busca semântica local, evitando o \n\nTool Bloat\n\n.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local, assinatura ou nuvem premium com base em custo, qualidade e latência.
*   **Swarm Intelligence:** Orquestração de múltiplos agentes especializados para tarefas complexas.

---

### **PARTE 59: Arquitetura de Isolamento e Sidecars Efêmeros no Antigravity IDE**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** O núcleo do Genesis MC deve ser Rust/Tauri puro, sem dependências persistentes de Python/Node.js.
*   **Sidecars Efêmeros:** Ferramentas Python/Node.js (como Docling MCP) são empacotadas em contêineres descartáveis ou executadas via Wasmtime (Pyodide) para isolamento e morte automática após a tarefa.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para acesso a ferramentas do host.
*   **`Dockerfile` Otimizado:** Uso de imagens `uv:bookworm-slim`, flags de bytecode (`UV_COMPILE_BYTECODE=1`), e inclusão mínima de dependências de sistema (`libgl1`, `libglib2.0-0`).
*   **`SKILL.md` para Habilidades:** Padrão para definir habilidades agênticas, com foco em extrair lógica em Rust/Wasm e evitar runtimes externos persistentes.
*   **Segurança:** Isolamento de recursos, proteção contra upgrades de SO, ponte segura de chaves SSH (via Rust), e rejeição de Docker pesado para sandboxing.
*   **Humanizer:** Lógica de detecção de padrões de escrita de IA absorvida no pipeline de post-processing Rust.
*   **Kreuzberg:** Adotado como biblioteca (crate) nativa em Rust para parsing de documentos, com uso opcional de ONNX Runtime para embeddings.
*   **Governança Nativa:** Lógica de regras aninhadas do Ruler reimplementada em Rust, gerenciando metadados dinâmicos em vez de scripts externos.

---

### **PARTE 60: VETOR OMICRON (Ilhas WebGL e IPC Binário Zero-Copy)**

**Axiomas e Conceitos Técnicos:**

*   **Ilhas WebGL:** UI híbrida onde a renderização gráfica densa ocorre em Web Workers isolados, utilizando \n\nOffscreenCanvas\n\n e comunicação via IPC binário. Isso libera a Main Thread do React e evita \n\nCumulative Layout Shift\n\n (CLS).
*   **IPC Binário Zero-Copy:** Comunicação direta de \n\nArrayBuffer\n\n entre Rust e Web Worker, evitando serialização/desserialização JSON para latência mínima (<5ms).
*   **`three.wasm`:** Micro-motor WebAssembly para renderização WebGL, executado no Web Worker para processamento gráfico isolado.
*   **Leis Anti-Gargalo:**
    1.  **Extirpação Explícita de VRAM:** Uso de \n\nWEBGL_lose_context\n\n para liberar memória da GPU ao desmontar ilhas.
    2.  **Desacoplamento IPC:** Destruição concertada de referências e comunicação via \n\npostMessage\n\n para o Web Worker, com \n\nself.close()\n\n para o trabalhador.
    3.  **Proibição de Realocação Dinâmica:** Uso de pré-alocação rígida e \n\nMutação In-Place\n\n no Web Worker para evitar jank e fragmentação de VRAM.
*   **Rust Backend:** Gerencia o ciclo de vida dos modelos, a comunicação IPC e a lógica de orquestração.
*   **Frontend Passivo (React):** Apenas monta o \n\n<canvas>\n\n, transfere o controle para o Web Worker e recebe os resultados finais. Não executa lógica de renderização pesada.

---

### **PARTE 61: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 58: Arquitetura Genesis: Orquestração Nativa e Sistemas Agênticos de Alta Performance**

**Axiomas e Conceitos Técnicos:**

*   **Rejeição de Python/Node.js no Núcleo:** Uso de \n\nSidecars Efêmeros\n\n e \n\nSandboxing\n\n (Wasmtime) para ferramentas externas, mantendo o núcleo em Rust.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Adiamento de FalkorDB e representações 3D/Sonoras.
*   **Simplificação de Memória:** Uso de SQLite com extensão \n\nsqlite-vec\n\n como memória unificada inicial.
*   **Canibalização de Lógica:** Extrair e reescrever em Rust a lógica de orquestração, busca e parsing de projetos como AgenticSeek, Antigravity, OpenClaw, Sanity-Gravity, Humanizer, Kreuzberg, Ruler.
*   **Interface Passiva React/Tauri:** UI reativa que apenas desenha dados do backend Rust, sem lógica de negócios.
*   **Ralph Loop:** Agentes são criados, executam tarefas, rodam testes e são mortos, recriando-se com contexto limpo para a próxima iteração.
*   **Backpressure Determinístico:** Testes de compilador e linters como principal feedback para a IA, garantindo \n\nexit code 0\n\n para conclusão de tarefa.
*   **O(1) Code Retrieval:** Uso de \n\ntree-sitter\n\n para extrair código via AST, economizando tokens.
*   **Gateway MCP Dinâmico:** Injeção de ferramentas via \n\nSKILL.md\n\n e busca semântica local, evitando o \n\nTool Bloat\n\n.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Swarm Intelligence:** Orquestração de múltiplos agentes especializados para tarefas complexas.

---

### **PARTE 59: Arquitetura Genesis: Orquestração Nativa e Sistemas Agênticos de Alta Performance**

**Axiomas e Conceitos Técnicos:**

*   **Rejeição de Python/Node.js no Núcleo:** Uso de \n\nSidecars Efêmeros\n\n e \n\nSandboxing\n\n (Wasmtime) para ferramentas externas, mantendo o núcleo em Rust.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Adiamento de FalkorDB e representações 3D/Sonoras.
*   **Simplificação de Memória:** Uso de SQLite com extensão \n\nsqlite-vec\n\n como memória unificada inicial.
*   **Canibalização de Lógica:** Extrair e reescrever em Rust a lógica de orquestração, busca e parsing de projetos como AgenticSeek, Antigravity, OpenClaw, Sanity-Gravity, Humanizer, Kreuzberg, Ruler.
*   **Interface Passiva React/Tauri:** UI reativa que apenas desenha dados do backend Rust, sem lógica de negócios.
*   **Ralph Loop:** Agentes são criados, executam tarefas, rodam testes e são mortos, recriando-se com contexto limpo para a próxima iteração.
*   **Backpressure Determinístico:** Testes de compilador e linters como principal feedback para a IA, garantindo \n\nexit code 0\n\n para conclusão de tarefa.
*   **O(1) Code Retrieval:** Uso de \n\ntree-sitter\n\n para extrair código via AST, economizando tokens.
*   **Gateway MCP Dinâmico:** Injeção de ferramentas via \n\nSKILL.md\n\n e busca semântica local, evitando o \n\nTool Bloat\n\n.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Swarm Intelligence:** Orquestração de múltiplos agentes especializados para tarefas complexas.

---

### **PARTE 60: O Paradigma Vantage: Uma Revolução na Avaliação por IA**

**Axiomas e Conceitos Técnicos:**

*   **Avaliação Furtiva (Stealth Assessment):** Medição contínua de habilidades duráveis através de telemetria comportamental, sem testes explícitos.
*   **Design Centrado em Evidências (ECD):** Engenharia reversa de mecânicas de jogo para evocar ações que comprovem competências mapeadas em rubricas pedagógicas.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, gerando respostas e comportamentos para todos os avatares de IA na simulação, e eliciando proativamente o comportamento do usuário.
*   **Equilíbrio Eco-Psicométrico:** Combinação de validade ecológica (realismo da simulação) com rigor psicométrico (controle e comparabilidade de dados).
*   **Mapas de Habilidades (Skill Maps):** Visualização dos níveis de proficiência com \n\ndrill-down\n\n para os trechos de transcrição que basearam a avaliação.
*   **Neurodiversidade:** Design que considera as necessidades de usuários com TDAH, autismo, etc., adaptando a interface (densidade, contraste, feedback).
*   **Agentes Guardiões:** IAs defensivas pessoais para filtrar comunicações e negociar com sistemas terceiros.

---

### **PARTE 61: O Paradigma Vantage: Uma Revolução na Avaliação por IA**

**Axiomas e Conceitos Técnicos:**

*   **Avaliação Furtiva (Stealth Assessment):** Medição contínua de habilidades duráveis através de telemetria comportamental, sem testes explícitos.
*   **Design Centrado em Evidências (ECD):** Engenharia reversa de mecânicas de jogo para evocar ações que comprovem competências mapeadas em rubricas pedagógicas.
*   **LLM Executivo:** Um LLM centralizador que orquestra outros agentes, gerando respostas e comportamentos para todos os avatares de IA na simulação, e eliciando proativamente o comportamento do usuário.
*   **Equilíbrio Eco-Psicométrico:** Combinação de validade ecológica (realismo da simulação) com rigor psicométrico (controle e comparabilidade de dados).
*   **Mapas de Habilidades (Skill Maps):** Visualização dos níveis de proficiência com \n\ndrill-down\n\n para os trechos de transcrição que basearam a avaliação.
*   **Neurodiversidade:** Design que considera as necessidades de usuários com TDAH, autismo, etc., adaptando a interface (densidade, contraste, feedback).
*   **Salto Agêntico:** Transição de ferramentas de IA para sistemas agênticos autônomos que se adaptam ao usuário.
*   **Agile 2.0:** Gestão de produtos com \"Via de Estabilidade de Recursos\" e \"Via de Descoberta Agêntica\".
*   **GenUI (Generative UI) e Vibe Design:** Interfaces que morfam e se adaptam em tempo real, baseadas em intenções e vibrações, guiadas por sistemas de design e restrições.

---

### **PARTE 62: Dissecação Técnica de Sistemas de Injeção de Contexto e Parsing para o Soberano Genesis Mission Control**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Edge Computing:** Execução local, air-gapped, sem dependência de nuvem.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo SODA.
*   **Canibalização de Lógica:** Extrair e reescrever em Rust a lógica de orquestração, busca e parsing de projetos como AgenticSeek, Antigravity, OpenClaw, Sanity-Gravity, Humanizer, Kreuzberg, Ruler.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para binários host. Docker é pesado e descartado.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **Parsing de Alta Performance:** Uso do Kreuzberg (Rust) para extrair texto e metadados de 88+ formatos de arquivos.
*   **Ontologias e Grafos de Conhecimento:** Integração com SQLite/Qdrant para manter conhecimento estruturado e validado contra esquemas YAML.
*   **AgenticSeek:** Rejeitar a infraestrutura Python/Docker; extrair lógica de decomposição de tarefas e seleção de agentes em Rust. Substituir SearxNG por módulo de busca estático/efêmero.
*   **Antigravity Awesome Skills:** Absorver o padrão `SKILL.md` e a Divulgação Progressiva. Implementar Skill Loader nativo em Rust.
*   **OpenClaw:** Extrair ontologias e esquemas de dados. Ignorar runtime Node.js. Implementar lógica de ontologia em Rust sobre SQLite/Qdrant.
*   **Sanity-Gravity:** Migrar conceito de \"Disposable Container\" para micro-VMs (Firecracker) ou sandboxes Wasm. Portar SSH Agent Proxy para Rust.
*   **Humanizer:** Absorver lógica de detecção de padrões no pipeline de post-processing Rust.
*   **Kreuzberg:** Integrar como crate Rust estática no daemon SODA. Usar versão WASM no frontend React.
*   **Ruler:** Reescrever motor de aplicação de regras em Rust. Tratar regras como metadados dinâmicos injetados no prompt.

---

### **PARTE 63: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 64: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 65: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 66: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 67: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 68: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 69: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 70: [Feature Update] MCP para CLI Agentes e Vantagens**

**Axiomas e Conceitos Técnicos:**

*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Abstração de Contexto Tardio (Late Context Abstraction):** Evita \n\nTool Bloat\n\n injetando metadados de ferramentas no prompt inicial e esquemas detalhados apenas quando o LLM os invoca.
*   **Conversão MCP para CLI:** Transforma servidores MCP em CLIs interativas, reduzindo o consumo de tokens e a complexidade para o LLM.
*   **Zero-Codegen:** Criação dinâmica de interfaces CLI a partir de esquemas MCP, sem necessidade de gerar código cliente prévio.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 71: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 72: VETOR ETA (Modelos de Linguagem Recursivos - RLM)**

**Axiomas e Conceitos Técnicos:**

*   **Modelos de Linguagem Recursivos (RLM):** Transformam o LLM de um \"contêiner\" passivo para um \"controlador\" ativo de um ambiente REPL.
*   **Mitigação de Context Rot:** O contexto massivo reside na RAM do sistema, não na VRAM da GPU. O LLM interage com ele via código gerado.
*   **Mecânica RLM:** Loop REPL (Read-Eval-Print Loop) onde o LLM gera código, este é executado, e a saída (ou erro) é retornada ao LLM para a próxima iteração.
*   **Sub-Chamadas Recursivas:** O Root LM pode invocar sub-RLMs para delegar tarefas específicas, mantendo o contexto principal limpo.
*   **Exploração Estrutural:** Técnicas como \"Peeking\" (sondar a estrutura do código) e \"Grepping\" (busca com RegEx) são usadas para entender bases de código massivas sem carregar tudo na VRAM.
*   **Delegação por Partição (Map-Reduce):** O LLM divide tarefas complexas em partes menores, delega a sub-RLMs e agrega os resultados.
*   **Rust/Wasmtime:** O ambiente REPL e os sub-agentes são implementados em Rust e executados em Wasmtime para isolamento e performance.
*   **Hardware Awareness:** Adaptação das estratégias RLM às limitações de VRAM da RTX 2060m e RAM do i9.

---

### **PARTE 73: [Update Research] Otimização Rust, Wasmtime e Llama.cpp**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal no Windows:** Execução direta no hardware (Intel i9, 32GB RAM, RTX 2060m 6GB VRAM), sem Docker/WSL2 para o núcleo.
*   **Sandboxing Zero-Trust:** WebAssembly Component Model (WASI 0.2) com Wasmtime para isolamento de código gerado por IA. Negação de acesso por padrão, com capacidades explicitamente injetadas.
*   **Compilação Nativa:** Uso de MSVC Build Tools, CMake, Rustup e CUDA Toolkit para compilação otimizada para Windows x64 e RTX 2060m (Compute Capability 7.5).
*   **Restrição de CLI:** Comandos Wasmtime com flags rigorosas (`--max-memory-size`, `--trap-on-grow-failure`, `--fuel`, `--no-network`, `--no-inherit-env`, `--no-cache`) para isolamento e controle de recursos.
*   **Dinâmica Numérica e Limitações Turing:** Otimização para Tensor Cores da RTX 2060m (FP16/INT8). Uso de `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para gerenciar a VRAM limitada através de offloading para RAM.
*   **Llama.cpp:** Compilado nativamente com CMake, priorizando bibliotecas estáticas e otimizações AVX2/FMA para CPU e CUDA 7.5 para GPU.
*   **`llama-swap`:** Mecanismo para trocar modelos entre VRAM e RAM de forma rápida (2-5s) para gerenciar o limite de 6GB VRAM.
*   **`rust-llm` / `Candle`:** Alternativas nativas em Rust ao `llama.cpp`, oferecendo melhor integração com o ecossistema Rust e controle de memória. `Candle` é preferido pela gestão de VRAM via VMM.
*   **Pipeline Multimodal:** VAD (Silero) -> STT (Whisper.cpp/ONNX na CPU) -> LLM (Candle/llama.cpp na GPU/CPU) -> TTS (Kokoro-82M na CPU).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **`SKILL.md` e Divulgação Progressiva:** Padrão para definir habilidades agênticas, carregando metadados inicialmente e conteúdo completo sob demanda.
*   **TDD Forçado pelo Agente:** O framework `Superpowers` (adaptado para `SKILL.md`) obriga o agente a escrever testes primeiro, garantir que eles falhem, e só então escrever o código para fazê-los passar.
*   **`Shadow Workspaces`:** Ambientes de desenvolvimento isolados onde os agentes podem experimentar e falhar sem afetar o código principal do projeto.

---

### **PARTE 74: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 75: [Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 76: VETOR OMICRON (Ilhas WebGL e IPC Binário Zero-Copy)**

**Axiomas e Conceitos Técnicos:**

*   **Ilhas WebGL:** UI híbrida onde a renderização gráfica densa ocorre em Web Workers isolados, utilizando \n\nOffscreenCanvas\n\n e comunicação via IPC binário. Isso libera a Main Thread do React e evita \n\nCumulative Layout Shift\n\n (CLS).
*   **IPC Binário Zero-Copy:** Comunicação direta de \n\nArrayBuffer\n\n entre Rust e Web Worker, evitando serialização/desserialização JSON para latência mínima (<5ms).
*   **`three.wasm`:** Micro-motor WebAssembly para renderização WebGL, executado no Web Worker para processamento gráfico isolado.
*   **Leis Anti-Gargalo:**
    1.  **Extirpação Explícita de VRAM:** Uso de \n\nWEBGL_lose_context\n\n para liberar memória da GPU ao desmontar ilhas.
    2.  **Desacoplamento IPC:** Destruição concertada de referências e comunicação via \n\npostMessage\n\n para o Web Worker, com \n\nself.close()\n\n para o trabalhador.
    3.  **Proibição de Realocação Dinâmica:** Uso de pré-alocação rígida e \n\nMutação In-Place\n\n no Web Worker para evitar jank e fragmentação de VRAM.
*   **Rust Backend:** Gerencia o ciclo de vida dos modelos, a comunicação IPC e a lógica de orquestração.
*   **Frontend Passivo (React):** Apenas monta o \n\n<canvas>\n\n, transfere o controle para o Web Worker e recebe os resultados finais. Não executa lógica de renderização pesada.

---

### **PARTE 77: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 78: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 79: [Update Research] FinOps IA Roteamento Híbrido e Pareto**

**Axiomas e Conceitos Técnicos:**

*   **FinOps IA:** Otimização de Custo, Qualidade e Latência na orquestração de LLMs.
*   **Fronteira de Pareto:** Seleção de modelos baseada na utilidade \n\nU_t(a | x) = q_t(a | x) - \\lambda_t \\cdot c(a) - \\beta \\cdot l_t(a)\n\n, onde \n\n\\lambda\n\n (marcapasso orçamentário) e \n\n\\beta\n\n (sensibilidade à latência) são dinâmicos.
*   **Roteamento em Cascata:** Prioriza modelos locais (Custo $0), depois assinaturas (Custo Fixo), e por último APIs premium (Custo Variável).
*   **Bandits Adaptativos:** O sistema aprende continuamente com o desempenho dos provedores, ajustando o roteamento com base em métricas de qualidade e custo.
*   **Esquecimento Geométrico:** Para lidar com a não-estacionariedade dos provedores de nuvem, o histórico de desempenho é gradualmente descartado.
*   **Telemetria Dinâmica (Auto Exacto / OTLP):** Monitoramento contínuo do desempenho dos provedores de nuvem via OTLP e NATS JetStream para informar as decisões do ParetoBandit.
*   **Execução Híbrida:** Uso da iGPU Intel para modelos leves/roteamento e RTX 2060m para LLMs e tarefas pesadas, com \n\nllama-swap\n\n para gerenciar VRAM.
*   **\"Subscription Hacking\":** Utilização de CLIs de assinaturas (Claude Code, Gemini CLI) como sub-agentes para tarefas pesadas, otimizando custos.
*   **Análise de Modelos:** Comparação de modelos de nuvem (Claude 4.6, Gemini 3.1, GPT-5.4, DeepSeek v4) e locais (Qwen 3.5, Llama 4 Scout, Gemma 4, Phi-4-mini) com foco em custo, qualidade e latência.

---

### **PARTE 80: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 81: VETOR OMICRON (Ilhas WebGL e IPC Binário Zero-Copy)**

**Axiomas e Conceitos Técnicos:**

*   **Ilhas WebGL:** UI híbrida onde a renderização gráfica densa ocorre em Web Workers isolados, utilizando \n\nOffscreenCanvas\n\n e comunicação via IPC binário. Isso libera a Main Thread do React e evita \n\nCumulative Layout Shift\n\n (CLS).
*   **IPC Binário Zero-Copy:** Comunicação direta de \n\nArrayBuffer\n\n entre Rust e Web Worker, evitando serialização/desserialização JSON para latência mínima (<5ms).
*   **`three.wasm`:** Micro-motor WebAssembly para renderização WebGL, executado no Web Worker para processamento gráfico isolado.
*   **Leis Anti-Gargalo:**
    1.  **Extirpação Explícita de VRAM:** Uso de \n\nWEBGL_lose_context\n\n para liberar memória da GPU ao desmontar ilhas.
    2.  **Desacoplamento IPC:** Destruição concertada de referências e comunicação via \n\npostMessage\n\n para o Web Worker, com \n\nself.close()\n\n para o trabalhador.
    3.  **Proibição de Realocação Dinâmica:** Uso de pré-alocação rígida e \n\nMutação In-Place\n\n no Web Worker para evitar jank e fragmentação de VRAM.
*   **Rust Backend:** Gerencia o ciclo de vida dos modelos, a comunicação IPC e a lógica de orquestração.
*   **Frontend Passivo (React):** Apenas monta o \n\n<canvas>\n\n, transfere o controle para o Web Worker e recebe os resultados finais. Não executa lógica de renderização pesada.

---

### **PARTE 82: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 83: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 84: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 85: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 86: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 87: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 88: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 89: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 90: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 91: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 92: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 93: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 94: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 95: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 96: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 97: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nativamente em Rust no Maestro.
*   **TOON vs. JSON:** Uso de \n\nToken-Oriented Object Notation (TOON)\n\n para serialização de dados, reduzindo o consumo de tokens.
*   **Pool de Processos STDIO Contínuo:** Servidores MCP (Python/Node.js) mantidos \"aquecidos\" via STDIO no backend Rust, eliminando \n\ncold starts\n\n.
*   **Sandboxing:** Wasmtime para lógicas puras; restrições nativas do SO (Landlock) para interações com ferramentas do host.
*   **Segurança:** Sanitização rigorosa de invocações CLI via Rust, isolamento com Wasmtime, cofres mestres para chaves.
*   **Roteamento Semântico Hierárquico:** Busca de ferramentas baseada em similaridade semântica e ranqueamento (BM25) em um catálogo local.
*   **Mitigação de Latência JIT:** Uso de `mcp2cli` (conceito) e `unmcp` para gerenciar servidores MCP em modo daemon, reduzindo latência de comunicação.
*   **Catálogo Curado de MCPs:** Priorização de ferramentas open-source e locais (Rust MCP Filesystem, GitHub MCP Server, DuckDuckGo MCP, Kindly MCP, Playwright MCP, SQLite MCP, Chroma MCP).

---

### **PARTE 98: Todas as notas 15/03/2026**

**Axiomas e Conceitos Técnicos:**

*   **Soberania e Bare-Metal:** O Genesis MC é um SO Agêntico Soberano (SODA), operando localmente, air-gapped, com foco em eficiência termodinâmica e mitigação de Context Rot.
*   **Rejeição de Interpretadores Pesados:** Eliminação de Python/Node.js no núcleo, priorizando Rust/Zig.
*   **Interface Passiva React/Tauri:** Frontend puramente visual, sem lógica de negócios, comunicando via IPC Zero-Copy.
*   **Tríade de Memória:** SQLite (transacional/relacional), Qdrant (vetorial), FalkorDB/LightRAG (grafos).
*   **Sandboxing Agressivo:** Wasmtime para lógicas puras, Landlock para binários host. Agentes estritamente efêmeros.
*   **Pessimismo da Razão vs. Otimismo da Vontade:** Reconhecer os riscos da complexidade (engenharia burnout, falhas transacionais, gargalos IPC) e priorizar a simplificação (SQLite como memória única inicial, sidecars efêmeros em vez de reescrita total em Rust).
*   **Priorização de Features:** Foco em orquestração imutável, memória eficiente e sandboxing seguro para v1. Features como representações 3D/Sonoras e FalkorDB nativo são adiadas.
*   **Gargalo de VRAM Compartilhada:** Necessidade de um VRAM Scheduler no daemon Rust para gerenciar o offloading dinâmico entre RTX 2060m e RAM.
*   **Dívida Técnica de Sandboxing:** Evitar dependência excessiva de Wasm puro para interações com o host; usar wrappers Rust para ferramentas do sistema.
*   **MCP Gateway Dinâmico:** Buscar ferramentas via busca semântica local em vez de injetar catálogos massivos no prompt.
*   **HITL Passivo:** Pausar o daemon Rust para aprovação humana via UI React ou notificações externas.
*   **Roteamento \"Zero-Trust Sandwich\":** Gateway interno em Rust avalia requisições, roteando para local (gratuito), assinatura (custo fixo) ou nuvem premium (custo variável).
*   **Canibalização de Repositórios:** Extrair lógica e padrões (bMAD, Ralph Loop, TDD, SKILL.md) de projetos open-source e reescrever em Rust.
*   **Canvas-first UI:** Interface visual onde agentes desenham diagramas e código, com visualização de fluxos de trabalho (Kanban, DAGs).
*   **Testes como Prioridade:** O gargalo de desenvolvimento será a escrita de testes automatizados impecáveis, não a geração de código pela IA.

---

### **PARTE 99: [Update Research] Otimização de Ferramentas para Amarração Tardia, Divulgação Progressiva e Roteamento Semântico (MCPs) para Agentes Soberanos**

**Axiomas e Conceitos Técnicos:**

*   **SODA Bare-Metal:** Execução local e soberana, sem dependência de runtimes pesados (Python/Node.js).
*   **Mitigação de Tool Bloat e Context Rot:** Uso de Amarração Tardia (\n\nLate-Binding\n\n), Divulgação Progressiva (\n\nProgressive Disclosure\n\n) e Roteamento Semântico (\n\nSemantic Routing\n\n) para otimizar o uso do contexto do LLM.
*   **MCP como Protocolo de Comunicação:** Padroniza a interação entre agentes e ferramentas.
*   **Maestro Roteador (Rust):** Orquestra a descoberta e injeção dinâmica de ferramentas via MCP, evitando o carregamento estático massivo.
*   **`mcp2cli` e Zero-Codegen:** Conceito de tratar servidores MCP como CLIs exploráveis, reduzindo o consumo de tokens. Implementado nat

---

Com base nos textos fornecidos, aqui está um resumo consolidado dos axiomas, regras e conceitos técnicos para o projeto SODA, com foco nas novas informações apresentadas:

---

## SODA - Manual Canônico (Atualização Parte 4)

### 1. ARQUITETURA PURA E PODAS TÓXICAS

*   **Backend:** Estritamente Rust (Tokio).
*   **Frontend:** Estritamente Svelte 5 + Tauri v2.
*   **Interface Frontend:** Passiva. Toda lógica reside no Rust (IPC Zero-Copy).
*   **Eliminar:** Tutoriais de React, Node.js daemons, Electron, VDOM, Server-Side Rendering (Next.js).

### 2. HARDWARE AWARE E OTIMIZAÇÕES

*   **CPU:** Otimizações bare-metal, diretivas de execução AVX2.
*   **GPU:** Limitações da iGPU (gargalos de barramento). Otimização para RTX 2060m (6GB VRAM) com `llama.cpp` e `mmap`.
*   **Memória:** Gerenciamento rigoroso dos 32GB de RAM, evitando daemons Python/Node.js.

### 3. AUDITORIA CRÍTICA (FUROS E CONFLITOS)

*   **boot.ps1:** Prática de isolamento excelente. Injeção de variáveis de ambiente na RAM da sessão atual do PowerShell, evitando poluição do Registro do Windows. Execução limpa do `agentgateway.exe` com configuração via `gateway-config.yaml`.
*   **Dockerfile e boot.ps1:** São considerados a "esteira de montagem isolada" do desenvolvedor, não lixo.
*   **GEMINI.md (Global):** Deve ser mantido atualizado.

### 4. CONSOLIDAÇÃO DO "PORQUÊ" TÉCNICO

*   **Arquitetura Sem Sujar a Máquina:** O `boot.ps1` injeta variáveis de ambiente na RAM da sessão atual do PowerShell, garantindo que o `agentgateway.exe` inicie limpo e seguro. Ao fechar o PowerShell, as variáveis desaparecem, mantendo o sistema limpo.
*   **Esteira de Montagem Isolada:** O `Dockerfile` e o `boot.ps1` são essenciais para criar um ambiente de desenvolvimento isolado e limpo.
*   **Manual Canônico:** A necessidade de um manual canônico (`_REFERENCES_STACK.md`) para documentar as referências técnicas imutáveis para o Agente.

### 5. NOVOS AXIOMAS, REGRAS E CONCEITOS TÉCNICOS

*   **Model Context Protocol (MCP):** Padrão de código aberto (originalmente da Anthropic) que atua como um barramento universal de comunicação para LLMs se conectarem a ferramentas externas, bancos de dados e serviços via JSON-RPC.
*   **Tool Bloat / Context Rot:** Ocorre quando dezenas de servidores MCP expõem seus esquemas simultaneamente, saturando a janela de contexto do LLM, inflando custos de inferência e prejudicando a precisão do roteamento.
*   **Gateways Dinâmicos e Late-Binding:** Solução para o Tool Bloat, operando como roteadores de meta-ferramentas que aplicam reflexão computacional e amarração tardia. O LLM interage com um catálogo leve, carregando o peso de uma ferramenta apenas quando necessário.
*   **MCP Vault (mcpv):** Gateway de carregamento preguiçoso ("Lazy-Loading") para resolver latência e loops infinitos. Atua como proxy local, controlando o tráfego JSON-RPC. Possui uma "Válvula Inteligente" para bloquear requisições reiteradas de dump de repositório, retornando "Context already cached".
*   **Divulgação Progressiva (Progressive Disclosure):** Arquitetura de Skills do Antigravity que separa índice de conhecimento da entrega de conteúdo. Organizada em 3 níveis:
    *   **Nível 1 (Descoberta Constante):** Metadados (título, descrição, triggers). Custo: ~100 tokens. Injetado permanentemente.
    *   **Nível 2 (Instrução Dinâmica):** Corpo completo do `SKILL.md` (documentação, regras, parâmetros JSON-RPC). Custo: < 5.000 tokens. Ativado por colisão vetorial com Nível 1.
    *   **Nível 3 (Execução e Busca):** Scripts associados, conexões com servidores MCP. Custo: Ilimitado. Engatilhado pela validação do Nível 2.
*   **Topologia Híbrida de Execução:** Combina processos nativos de hiper-latência e `Sidecars` efêmeros via Docker.
    *   **Sidecars Efêmeros:** Contêineres Docker instanciados sob demanda (`docker run -i --rm`), estritamente atrelados ao ciclo de vida da requisição da IA. Essencial para cargas de trabalho pesadas (OCR, navegação web).
    *   **Execução Nativa Hiper-Rápida:** Para ferramentas de leitura de disco contígua, buscas vetoriais locais, rastreamento de banco de dados relacional. Configurado no `mcp_config.json` para evocar comandos nativos (`npx`, `uvx`).
*   **Princípio do Privilégio Mínimo:** Aplicado a montagens de disco em contêineres. Comandos no `mcp_config.json` devem impor o prefixo `:ro` (Read-Only) em volumes (`-v`) conectados à raiz do workspace (`${workspaceRoot}`).
*   **Gerenciamento de Segredos:** Credenciais (tokens, chaves API) devem ser atreladas exclusivamente a variáveis de ambiente do kernel hospedeiro (`${env:VAR_NAME}`), com injeções efêmeras limitadas à RAM temporária do contêiner.
*   **MCP Vault (mcpv) para Auditoria:** Atua como proxy central para interceptar tráfego RPC, monitorar cadência interativa, tempo de resposta e reter relatórios forenses.
*   **`mcp_config.json`:** Arquivo de configuração central que define os servidores MCP, seus comandos, argumentos e variáveis de ambiente.
*   **`SKILL.md`:** Manifesto para a Divulgação Progressiva, contendo `name`, `description`, `triggers` e os passos imperativos de execução. Previne a saturação da IDE com sintaxe de ferramentas irrelevantes.
*   **Memória Neuro-Sintética (MNS):** Evolução da arquitetura de memória do SODA.
    *   **Morte do Grafo em RAM:** Petgraph em RAM foi substituído. SQLite (com FTS5 em modo WAL) é a Única Fonte da Verdade (SSOT). Adota `Event Sourcing` (Append-Only).
    *   **Paradigma Cabinet:** Clona SQLite transacionalmente e usa `libgit2` para criar `snapshots` comprimidos (delta encoding). LanceDB possui versionamento nativo, amarrando ID de versão ao `hash` do `commit` do SQLite.
    *   **Topologia de 3 Camadas "Pointer Index":** Memória RAM (L1 - Transiente) é apenas um índice raso com ponteiros (~150 caracteres) referenciando o disco. L0 é `SOUL.md`/`SKILL.md`. L2/L3 é acionado sob demanda.
    *   **Persistência Física do KV Cache:** Quantiza cache neural em tensores de 4-bits (Q4) e grava em SSD NVMe como `safetensors`. Permite injeção de raciocínios massivos em 1.3 segundos.
    *   **Substituição de Inferência por Matemática Pura:** Usa Métrica Fisher-Rao, Cohomologia de Feixes (para detectar contradições algebricamente sem tokens) e Dinâmica de Langevin (para poda sináptica).
    *   **Motor de Desfragmentação AutoDream (Chyros Daemon):** Acorda sub-agentes em `idle time` para curar `brain-dumps`, fundir metadados e criar resumos limpos.
*   **Design para Neurodiversidade (Genesis MC):**
    *   **Prótese de Função Executiva:** O sistema assume o peso logístico do planejamento e sequenciamento.
    *   **Assincronia e Perfis Irregulares:** Reconhece a falta de linearidade e atenção sustentada.
    *   **Economia da Carga Executiva:** Minimiza o esforço mental para planejar e iniciar tarefas.
    *   **Sistema Nervoso Baseado em Interesse:** Motivação ativada por novidade, interesse, urgência e desafio.
    *   **Cegueira Temporal e Paralisia de Iniciação:** Interfaces devem ser minimalistas, com Divulgação Progressiva e hierarquia de informação clara.
    *   **Controle de Estímulos e Previsibilidade:** Evitar animações não solicitadas, usar modos de acessibilidade cognitiva.
    *   **Topologias Espaciais e Rejeição da Linearidade:** Interfaces como Whiteboards/Spatial Canvas (Heptabase, Scrintal).
    *   **Generative UI (GenUI):** Preferência por `Declarative GenUI` para renderizar interfaces dinamicamente com base no contexto, mantendo padrões neuroinclusivos.
    *   **Loop Agêntico (Perceive, Reason, Plan, Act, Observe):** LLM atua como núcleo cognitivo.
    *   **Padrões de Delegação Humano-IA:** Híbrido `HITL`/`HOTL`. Tarefas logísticas de baixa criticidade em `HOTL`, tarefas de alto risco em `HITL`.
    *   **Orquestração Multi-Agente:** Recomendação de `Magentic-One` (via AutoGen) pela gestão do `Progress Ledger`.
    *   **Inferência de Carga Cognitiva:** Machine Learning `On-Device` para inferir estado atencional via marcadores comportamentais digitais (tab churn, foco de aplicação, inatividade).
    *   **Privacidade:** `Agent OS Locais` e `Model Context Protocol (MCP)` para operação `Privacy-First`.
*   **Avaliação de LLMs:**
    *   **Benchmarks:** MMLU, HellaSwag, GSM8K, HumanEval, TruthfulQA, MT-Bench, Chatbot Arena.
    *   **Paradigma:** Zero-shot, Few-shot, Fine-tuned.
    *   **LLM-as-a-Judge:** Uso de LLMs para avaliar outros LLMs, especialmente em diálogos multi-turno.
    *   **Críticas:** Contaminação de dados, Lei de Goodhart ("Quando uma medida se torna um alvo, ela deixa de ser uma boa medida"), ruído nos benchmarks, saturação, falta de representatividade do mundo real.
    *   **Framework de Análise Multiaxial:** Definir capacidades críticas -> Selecionar portfólio de benchmarks -> Analisar desempenho relativo -> Considerar eficiência (latência, throughput, memória).
    *   **Arquiteturas de LLM:** Modelos Densos vs. Mixture-of-Experts (MoE), Modelos Base vs. Ajustados por Instrução.
*   **IA na China:**
    *   **Multimodalidade:** Padrão emergente (Qwen-VLo, Baidu Huiboxing, ByteDance Doubao, Tencent Hunyuan3D).
    *   **Convergência Arquitetônica:** Onipresença da Mistura de Especialistas (MoE) para eficiência.
    *   **Raciocínio Avançado:** Modos de "pensamento" e modelos especializados.
    *   **Inovação sob Restrição:** Otimização de hardware e software devido a sanções dos EUA.
    *   **Estratégias Corporativas:** Baidu (defensiva/ecossistema/código aberto), ByteDance (consumidor/engajamento), Alibaba (infraestrutura/B2B/código aberto), Tencent (simbiótica/transacional).
    *   **Guerra de Preços:** Estratégia para comoditizar modelos e impulsionar plataformas.
*   **Canibalização de Repositórios Open-Source (Lote 14):**
    *   **Marknative:** Extrair matemática de conversão AST para Rust.
    *   **JSON-Alexander:** Extrair lógica de virtualização de árvores e JSON Path Tool para Rust.
    *   **Gitreverse:** Extrair heurística de varredura recursiva de diretórios e template de prompt para Rust.
    *   **Mail-app (Exo):** Extrair algoritmos de busca híbrida (SQLite FTS5), overrides de prioridade e "Optimistic UI" para Rust/Wasm.
    *   **Sheets:** Isolar parser CSV abstrato e engine de fórmulas (`=`) para Wasm/Rust.
    *   **Boneyard:** Absorver apenas a taxonomia final (`.bones.json`) para pré-render geométrico no Canvas.
    *   **Jot:** Transcrever a heurística central (`Tracked IdList`, Rebase Semântico) para Rust, substituindo WebSockets por IPC.
    *   **Cabinet:** Transmutar a infraestrutura de conhecimento persistente (Git + Markdown) para o daemon Rust, descartando Next.js e Node.js.
    *   **PaperclipAI:** Reconstruir o "Heartbeat Pattern" no escalonador Tokio em Rust.
*   **Roteamento Inteligente e Gateways:**
    *   **LiteLLM:** Proxy com suporte a múltiplos provedores, painel admin, gestão de quotas. Risco: Latência e OOM sob alta carga.
    *   **One API:** Go, redistribuição de chaves, gestão de quotas, roteamento estático.
    *   **Bifrost:** Go, ultra-baixa latência, cache semântico de dupla camada.
    *   **OpenZiti LLM Gateway:** Go, rede Zero-Trust, comunicação NAT.
    *   **Observabilidade:** Langfuse, Arize Phoenix, Opik, Helicone, OpenLLMetry.
    *   **Roteamento Inteligente:** Regras determinísticas vs. Semântico (vLLM Semantic Router) vs. Machine Learning (LLMRouter) vs. Grafos (AgentRouter).
*   **Debate Multiagente Anti-Cegueira de Consenso:**
    *   **Problema:** LLMs formam câmaras de eco, ignorando falhas arquiteturais.
    *   **Métricas:** `U_intra`, `Sum C_t`, `U_sys` para quantificar incerteza.
    *   **DMAD (Diverse Multi-Agent Debate):** Rotação de estratégias (CoT, Step-Back, Falsification) em vez de personas.
    *   **Free-MAD (Consensus-Free):** Anti-conformidade no prompt (`beta < 0`), decaimento/penalização de ajustes tardios (`f(k)`), entropia determinística, jittering estratificado, roteamento adaptativo (`RA-CR`).
    *   **Reflection with Backpressure:** Gerenciamento de carga, elevação de limiares e descarte (`Shedding`), histerese.
    *   **Map-Reduce Socrático:** Fase Zero (Leis Socráticas), Fase Map (Parallel Dispatch), Fase Cross-Critique, Fase Reduce (Síntese via Council Mode).
    *   **Implementação Rust:** Canais `mpsc`, `tokio`, `swarms-rs`, `libgit2`, `sqlite`, `Qdrant`.

---

**Observações Gerais:**

*   A arquitetura SODA é fortemente orientada para a eficiência de recursos, determinismo e segurança, priorizando Rust e Wasmtime.
*   A interação com o usuário (2e/TDAH) é um fator chave, ditando a necessidade de interfaces passivas, previsíveis e com mínima carga cognitiva.
*   A canibalização de código open-source é uma estratégia central para obter a funcionalidade desejada sem introduzir dependências tóxicas.
*   O `AgentGateway` em Rust é a peça fundamental para a comunicação segura e eficiente, substituindo middlewares legados em Python/Node.js.
*   A gestão de memória, especialmente VRAM, é crítica, exigindo técnicas como `mmap`, quantização e o "Heartbeat Pattern" para modelos efêmeros.

---

## Manual Canônico SODA: Atualização de Arquitetura e Segurança (Parte 5)

### Axiomas e Conceitos Técnicos Adicionados:

**1. Engenharia de Memória Autônoma e Recuperação Sub-Quadrática:**

*   **Memória Tri-Partite:** A arquitetura SODA utiliza uma memória dividida em três camadas para otimizar o acesso e a persistência de dados, especialmente em hardware restrito:
    *   **L1 (Memória de Trabalho Efêmera em RAM):** Índice de ponteiros estruturados na RAM do sistema para roteamento determinístico e respostas de baixa latência.
    *   **L2 (Memória Episódica em SQLite WAL):** Registro imutável da realidade, capturando o histórico de interações como um fluxo de eventos via SQLite com Write-Ahead Logging (WAL) e FTS5 para busca textual.
    *   **L3 (Memória Semântica Persistente e Vetorial):** Malha de dados avançada (LanceDB/Qdrant) com algoritmos de clustering (Leiden) e offload assíncrono do KV Cache quantizado em 4-bits para o SSD via `.safetensors`.
*   **Garbage Collection Semântico (Chyros Daemon / AutoDream):** Rotina de manutenção em background, executada na CPU via AVX2 durante ociosidade do sistema, para consolidar, otimizar e expurgar dados obsoletos da memória, evitando o \"Lixão Semântico\".
*   **Recuperação $\\mathcal{O}(1)$:** Abandono da similaridade de cosseno em favor de métricas baseadas na Distância de Fisher-Rao (FRQAD) e Tabelas de Hash Esparsas (LightMem, NextMem, SLM V3) para acesso a vetores em tempo constante.
*   **Cohomologia de Feixes (Sheaf Cohomology):** Utilizada para garantir a imunologia semântica, detectando e resolvendo contradições lógicas e factuais no grafo de conhecimento (GraphRAG) através de álgebra abstrata, prevenindo alucinações persistentes.
*   **Dinâmica de Langevin em Espaços Hiperbólicos:** Modelo matemático para o decaimento orgânico da memória, onde a curvatura do espaço informacional (Bola de Poincaré) e a interação entre forças de deriva determinística e difusão estocástica impõem um decaimento provável e a convergência para distribuições estacionárias, eliminando o lixo semântico de forma natural.

**2. Orquestração de Manutenção em Background (Chyros Daemon / AutoDream):**

*   **Padrão de Computação em Tempo de Sono:** Rotinas de limpeza e consolidação de memória relegadas estritamente aos períodos de ociosidade do hardware.
*   **Máquina de Estados da Consolidação Semântica:** Processo ETL cognitivo em 4 fases (Orient, Gather, Consolidate, Prune & Index) executado em threads isoladas via `tokio`.
*   **Inferência em Background via CPU (AVX2):** Utilização das instruções AVX2 da CPU para rodar modelos LLM destilados (SmolLM2-135M, Phi-4-mini) através de bindings Rust (`llama-cpp-bindings`, `candle`), mantendo a VRAM da dGPU intacta.

**3. Orquestração de Agentes e Governança:**

*   **Model Context Protocol (MCP):** Padrão de comunicação bidirecional entre o cliente SODA e os LLMs na nuvem, com foco em segurança e eficiência.
*   **AgentGateway (Rust):** Roteador de gateway local que gerencia a comunicação MCP, implementando políticas de segurança como *Fail Closed* para o núcleo cognitivo e *Fail Open* para utilitários.
*   **Typestate Pattern:** Utilização de tipos em tempo de compilação para codificar o estado de uma transação ou requisição MCP, garantindo transições de estado seguras e impedindo estados inválidos ou inacessíveis.
*   **Rate Limiting (GCRA):** Algoritmo para controlar a frequência de requisições de *sampling* e outras operações, prevenindo exaustão de recursos e ataques de negação de serviço.
*   **Restrição de Escopo de Metadados:** Análise e expurgo de metadados de ferramentas (descrições, parâmetros) para remover caracteres de esteganografia, limitar tamanho e eliminar tags que possam induzir comportamento malicioso no LLM.
*   **Human-in-the-Loop (HITL) Dinâmico:** Mecanismo assíncrono onde requisições de alto risco ou mutáveis são suspensas e apresentadas ao usuário para aprovação explícita antes da execução.
*   **Sandboxing via Wasmtime/Landlock:** Execução de ferramentas e scripts gerados pela IA em ambientes isolados e com privilégios mínimos, garantindo a inviolabilidade do sistema hospedeiro.

**4. Segurança e Mitigação de Engano:**

*   **Blindagem do Canal IPC (Tauri v2):** Utilização de buffers binários (MessagePack via `rmp-serde`) para comunicação entre Rust e Svelte/React, evitando gargalos de serialização JSON e bloqueio do Event Loop.
*   **Content Security Policy (CSP):** Diretivas rigorosas no `tauri.conf.json` para restringir a execução de scripts e conexões externas na WebView.
*   **Component Registry Pattern:** Mapeamento estático e imutável de identificadores JSON para componentes React, garantindo acesso em tempo $O(1)$ e prevenindo injeções de código.
*   **Sanitização de Markdown (DOMPurify):** Higienização de conteúdo rico para neutralizar propriedades perigosas e scripts embutidos antes da renderização no React.
*   **Validação de Esquema Recursiva (Zod):** Uso de `z.lazy()` e `z.discriminatedUnion()` para validação segura e eficiente de estruturas de dados JSON recursivas e complexas, com ` .strict()` para rejeitar propriedades não declaradas.
*   **Mitigação de Engano por IA:**
    *   **Telemetria de Canal Lateral:** Monitoramento de latência (TTFT) para detectar \"latency jitter\" como indicativo de processamento oculto.
    *   **Alinhamento Deliberativo (CoT):** Exigência de raciocínio *Chain-of-Thought* para fundamentar decisões e prevenir *sandbagging*.
    *   **Sandboxing de Kernel (eBPF / Landlock / AppContainer):** Isolamento físico e lógico de processos e sistemas de arquivos para prevenir acesso não autorizado e execução de código malicioso.
    *   **Treinamento Adversarial Latente (LAT) Local:** Ajuste fino de modelos para remover comportamentos enganosos identificados.

**5. Arquitetura de Interface e UX (Cyber-Neuro Synthesis):**

*   **Nothing Design:** Estética minimalista com foco em funcionalidade, utilizando grids absolutos, espaçamentos matemáticos e paleta de cores restrita (Cyber-Purple, Midnight Black).
*   **Glassmorphism Funcional:** Uso de `backdrop-blur` e opacidades sutis para criar hierarquia visual sem poluir o campo de visão.
*   **Mechanical Instancy:** Micro-interações de UI com latência inferior a 50ms para feedback responsivo.
*   **Progressive Disclosure:** Ocultamento da complexidade até que seja explicitamente solicitada pelo usuário.
*   **Focus Rack:** Limitação de 5 slots de trabalho ativos para evitar sobrecarga cognitiva.
*   **Scratchpad / Thinking Legibility:** Visualização do processo de raciocínio da IA em áreas recolhíveis.
*   **A2UI (Agent-to-User Interface):** Protocolo para geração declarativa de interfaces, onde o Rust envia esquemas JSON que o frontend passivo renderiza em componentes seguros.
*   **Rebase Semântico:** Resolução de conflitos de edição entre humano e IA sem a necessidade de CRDTs pesados.

### Regras Inegociáveis e Axiomas Arquiteturais:

1.  **Rust (Tokio) no Backend, Svelte 5 + Tauri v2 no Frontend:** A base tecnológica é imutável.
2.  **Frontend Passivo:** Toda lógica reside no Rust. O frontend apenas renderiza o estado e envia intenções de ação via IPC Zero-Copy.
3.  **Zero-Trust Absoluto:** Assumir que toda entrada externa (rede, arquivos, LLM) é hostil. Implementar defesas em profundidade em todas as camadas.
4.  **Hardware-Awareness:** Otimizar para as restrições físicas (6GB VRAM, CPU i9) e explorar ao máximo as capacidades (AVX2, mmap).
5.  **Canibalização Cirúrgica:** Extrair a lógica essencial de repositórios open-source, descartando o lixo tóxico (Node.js, Python runtimes, Docker pesado em produção).
6.  **Idempotência e Determinismo:** Operações devem ser repetíveis sem efeitos colaterais indesejados. O estado deve ser previsível.
7.  **Segurança por Design:** Implementar sandboxing (Wasmtime, Landlock), validação de schema (Zod) e controle de acesso granular (Typestate) desde o início.
8.  **Mitigação de Alucinação:** Utilizar HVR, Roteamento Mecanicista e SELFDOUBT para prevenir e corrigir comportamentos erráticos da IA.
9.  **UX para Neurodiversidade:** Design focado em reduzir carga cognitiva, fornecer clareza e controle ao usuário 2e/TDAH.
10. **Manutenção Autônoma (AutoDream):** Rotinas de limpeza e otimização de memória ocorrem em background durante ociosidade do sistema.

---

Este manual consolida os princípios arquiteturais e as práticas de engenharia que definem o SODA. Ele serve como a base para o desenvolvimento contínuo e a garantia de que cada componente opera dentro dos limites de segurança, performance e usabilidade definidos.

---

## Manual Canônico da Arquitetura SODA - Parte 6

### Axiomas e Conceitos Técnicos Adicionais

**1. Camadas de Interface e Vocabulário Oficial SODA:**

A decomposição da interface do SODA em camadas visuais e funcionais, do fundo para a frente e das bordas para o centro, estabelece um vocabulário unificado para a comunicação com ferramentas de design e desenvolvimento:

*   **Camada 0: SODA Substrate (Tauri Window Shell):** A \"casca\" do aplicativo, interagindo diretamente com o SO. Deve invocar APIs nativas do Tauri para materiais translúcidos do SO (Mica/Acrylic no Windows 11), conferindo profundidade espacial.
*   **Camada 1: Governor Rail (Sidebar/Menu de Navegação):** Controle central para alternar entre \"Canvases Específicos\" (Mission Dashboard, Architecture Canvas, MCP Registry, Memory Explorer, Settings). Deve conter atalhos visuais para estes modos.
*   **Camada 2: Telemetry HUD (Topbar/Header):** Barra superior estritamente \n\nCompute-Aware\n\n, exibindo dados do daemon Rust (VRAM, status do roteador LLM, status de assinatura). Não é um cabeçalho estático, mas o \"eletrocardiograma\" do hardware e do agente.
*   **Camada 3: Active Canvas (Workspace/Viewport Ativo):** Área central dinâmica que muda completamente dependendo do item selecionado no Governor Rail. Utiliza bibliotecas como \n\ntldraw\n\n ou \n\nxyflow\n\n, com fundo pontilhado/escuro. É onde os \"nós\" e \"pensamentos\" dos agentes fluem.
*   **Camada 4: Ephemeral Layer (Overlays, Modals & Spotlight):** Elementos que flutuam em \"Z-index\" alto sobre todo o resto. Inclui a barra de captura rápida (\n\nAlt+Space\n\n), Cards de Aprovação (\n\nBlast Radius\n\n), balões de \n\nSkeleton Loading\n\n e interações que congelam a tela.

**2. Módulos Funcionais da Interface (Canvases Focados):**

A tela não é um \"Canvas Infinito\" genérico, mas sim dividida em módulos focados para evitar a \"Síndrome da Tela em Branco\" e a \"Dívida de Fluxo\" (Flow-Debt) para mentes 2e/TDAH. Os módulos incluem:

*   **O \"Spotlight\" Efêmero (Entrada Zero-Atrito):** Barra translúcida minimalista invocada por atalho global (\n\nRaycast/PowerToys\n\n), para \n\nBrain Dumps\n\n rápidos de texto ou áudio.
*   **Architecture Canvas (Tldraw/Xyflow):** Quadro branco principal com fundo escuro de alto contraste. Permite a \n\nDivulgação Progressiva\n\n (nós contraídos que expandem ao clique).
*   **Kanban Swarm Canvas (Governança):** Visualização tátil de cartões (\n\nTo Do -> In Progress -> Review -> Done\n\n) mostrando o \n\npipeline\n\n da metodologia (ex: \n\nSpecify -> Plan -> Red-Green-Refactor -> Review\n\n). Os cartões se movem sozinhos com animações orgânicas de 250ms.
*   **Matriz do \"Blast Radius\" (HITL Interativo):** Tela de aprovação de segurança que congela o sistema. Exibe um grafo direcional mostrando módulos impactados por ações perigosas. Exige ação mecânica consciente (slider/biometria) para autorizar tarefas críticas de Nível 3.
*   **Terminal Truecolor Embutido:** Aba inferior (\n\nlibghostty-vt\n\n) apenas para leitura, mostrando o compilador Rust falhando, testes do \n\nRalph Loop\n\n e telemetria FinOps/Hardware (\n\nMedidor de VRAM (6GB RTX 2060m)\n\n, Temperatura, APM, Gasto de Tokens/Subscription Hacking Status\n\n).

**3. Arquitetura de Comunicação e Gestão de Estado:**

*   **Rust (Tokio) Backend:** Núcleo imutável e determinístico. Toda lógica reside aqui.
*   **React (Svelte 5) Frontend:** Interface passiva, consumindo dados via IPC Zero-Copy do Rust.
*   **Zustand Stores:** Estado global segregado em \n\nstores\n\n atômicos (ex: \n\nuseUIStore\n\n, \n\nuseCanvasStore\n\n) para evitar re-renderizações desnecessárias.
*   **IPC Throttling:** Eventos contínuos do Rust (logs, telemetria) são limitados a 100ms para evitar congelamento do frontend.
*   **Focus Rack:** Gerencia slots dinâmicos na UI para contextualizar a interação do usuário com o agente.

**4. Protocolo de Vibe Coding e Design System:**

*   **Definições Inegociáveis:** A IA deve aderir estritamente às diretrizes visuais (\n\nCyber-Purple\n\n, \n\nMidnight Black\n\n, \n\nGlassmorphism\n\n, \n\nNeon-Violet\n\n) e tipográficas (\n\nSpace Grotesk\n\n, \n\nManrope\n\n, \n\nInter\n\n).
*   **Proibição de \"Flow Debt\":** Eliminar bordas de 1px, usar transições mecânicas (50ms) e orgânicas (250ms), e favorecer o \n\nGlassmorphism\n\n para profundidade espacial.
*   **Componentes Compute-Aware:** Implementar \n\nSkeleton Loading\n\n reativo e \n\nStreaming Text\n\n com micro-interações de 50ms.

**5. Segurança e Isolamento de Execução:**

*   **Wasmtime WASIp2:** Execução de lógicas puras e scripts gerados pela IA em contêineres isolados com limites de memória e CPU.
*   **Landlock (Linux) / Sandbox Nativo (Windows):** Para ferramentas do host (Git, npm, etc.), restrições de kernel para acesso a disco e rede.
*   **Zero Egress:** Nenhuma capacidade de rede é habilitada por padrão para módulos Wasm.
*   **Cgroups v2:** Gerenciamento rigoroso de recursos para processos em background, garantindo que nenhum agente consuma excessivamente CPU ou RAM.
*   **ProcessPoolGuard:** Mecanismo em Rust para garantir que processos filhos instanciados pelo AgentGateway sejam terminados de forma determinística quando saírem de escopo.

**6. Orquestração de Ferramentas e MCP:**

*   **AgentGateway:** Proxy nativo em Rust que substitui \n\nmcpv\n\n e \n\nPython/Node.js\n\n. Opera com \n\nLate-Binding\n\n, \n\nstdio\n\n e \n\nStreamableHTTP\n\n.
*   **Manifest YAML:** Configuração declarativa para roteamento de ferramentas, políticas de segurança (CEL) e tratamento de falhas (\n\nfailureMode: failOpen\n\n).
*   **Nomenclatura e Identidade:** O agente central é o \"Estrategista\" (Life Coach), com um \"Governador\" oculto para segurança.
*   **Canibalização de Código:** Extração de heurísticas e lógicas de repositórios open-source, reescrita em Rust para conformidade com SODA. Repúdio total de dependências Python/Node.js e runtimes pesados.

**7. Mitigação de Riscos e Dívida Técnica:**

*   **ASI04 (Supply Chain):** Mitigado por sandboxing Wasmtime com \n\ndeny-by-default\n\n, limites de recursos e \n\nlinker traps\n\n para importações desconhecidas.
*   **ASI09 (Shadow IT):** Erradicado pelo uso de Cgroups v2 para isolamento de processos e pelo \n\nProcessPoolGuard\n\n em Rust para terminação determinística.
*   **ASI08 (Failures/Denial of Wallet):** Mitigado pelo modo \n\nfailOpen\n\n no AgentGateway e pela lógica de Circuit Breaker em Rust com avaliação CEL.
*   **Dívida Técnica:** Necessidade de adicionar \n\ntokio\n\n, \n\nsqlx\n\n e \n\nmcp-rs\n\n ao backend Rust. Limpeza do \n\ngenesis_mc_state_report.md\n\n.

**8. Otimização de Hardware e Inferência:**

*   **HISA (Hierarchical Indexed Sparse Attention):** Adotado para reduzir a complexidade da atenção de \n\nO(L^2)\n\n para subquadrática, crucial para LLMs em hardware restrito. Implementado via tensores puros em Rust/Candle.
*   **llama.cpp com TurboQuant:** Utilizado para gerenciar o KV Cache na RTX 2060m (6GB VRAM), permitindo janelas de contexto maiores através de quantização agressiva.
*   **VAD (Cobra VAD) e STT (Parakeet TDT):** Processamento de áudio localmente em Rust, com foco em latência mínima e baixo consumo de CPU/RAM.
*   **TTS (Kokoro-82M):** Para geração de voz, priorizado pela eficiência em CPU e baixo consumo de RAM.

**9. O Paradigma de Memória e Raciocínio:**

*   **Memória Tri-Partite Especializada:** SQLite (transacional/relacional), LanceDB (vetorial) e FalkorDB (grafo) para RAG e raciocínio multi-salto.
*   **SLM V3:** Utiliza matemática (Fisher-Rao, Cohomology) para evitar \"Context Rot\" e alucinações, operando 100% em CPU.
*   **GraphRAG:** Essencial para raciocínio multi-salto, superando RAG vetorial puro em precisão e detecção de relações causais.

**10. O Agente Governador e a Interface de Controle:**

*   **Persona Dual:** \"Estrategista\" (visível, interativo) e \"Governador\" (invisível, segurança).
*   **OmniBar + Painel Lateral Retrátil:** Interface híbrida para interações rápidas e monitoramento contínuo.
*   **Delegação Transparente:** O \"Livro-Razão Visual\" exibe o fluxo de ações dos subagentes, com aprovação humana via \"Approval Mode\" e o \"Big Friendly Stop Button\".

---

Este conjunto de axiomas e conceitos técnicos consolida a arquitetura SODA, priorizando segurança, eficiência e uma experiência de usuário adaptada às necessidades neurodivergentes, tudo dentro das restrições de hardware bare-metal.

---

Com base nos textos fornecidos, aqui está um resumo consolidado dos novos axiomas, regras e conceitos técnicos para o Manual Canônico do projeto SODA (Sovereign Operating Data Architecture - Genesis MC):

---

## SODA - Manual Canônico (Parte 7)

### Novos Axiomas, Regras e Conceitos Técnicos

**1. Roteamento Mecanicista e Desacoplamento Encoder-Target:**

*   **Conceito:** O roteamento de consultas para LLMs não deve se basear apenas na similaridade semântica (kNN), mas sim na **previsão da probabilidade de sucesso/falha** da tarefa em um modelo específico. Isso é alcançado analisando as **ativações de prefill** (estados ocultos intermediários) de um **Encoder** (modelo pequeno e eficiente, rodando na CPU) para prever o desempenho do **Alvo** (LLM local na GPU ou na nuvem).
*   **Métricas Chave:**
    *   **Anisotropia Representacional (\n\n\\alpha\n\n):** Mede a direcionalidade do agrupamento de vetores de estado oculto para detectar a patologia do \"cone estreito\". Camadas com alta isotropia (baixo \n\n\\alpha\n\n) são preferíveis.
    *   **Dimensionalidade Efetiva (\n\nd_{\\text{eff}}\n\n):** Calcula a \"largura de banda representacional\" de uma camada usando a Razão de Participação, indicando a quantidade de dimensões não redundantes. Camadas com alto \n\nd_{\\text{eff}}\n\n são mais informativas.
    *   **Separabilidade de Fisher (\n\nJ\n\n):** A métrica supervisionada final que mede a distância entre clusters de sucesso/falha, ponderada pela variância intragrupo. A camada com o pico de \n\nJ\n\n é selecionada para extração de sinais.
*   **Arquitetura:** Implementação do **SharedTrunkNet** (uma MLP com tronco compartilhado) para processar as ativações do Encoder e prever o sucesso de múltiplos Alvos (GPU local, Nuvem) simultaneamente. O treinamento utiliza **ensemble** (Top 5 modelos com menor Brier Score) para robustez.
*   **Regra SODA:** O roteamento de inferência deve priorizar a **previsão de sucesso baseada em sinais de prefill** em vez de similaridade semântica, utilizando um Encoder local (CPU) para orquestrar o Alvo (GPU/Nuvem).

**2. Engenharia de Contexto e Mitigação de \"Context Rot\":**

*   **Conceito:** Combater a degradação da performance e a amnésia dos LLMs através de técnicas que gerenciam ativamente a janela de contexto, evitando a saturação com dados irrelevantes ou redundantes.
*   **Mecanismos:**
    *   **Rebase Semântico:** Substitui CRDTs por um modelo de árbitro autoritário (backend Rust) com identificadores imutáveis (\n\nElementId\n\n) e um sistema de \"tombstones\" (\n\nis_deleted\n\n) para gerenciar edições concorrentes sem sobrecarga de metadados. Utiliza \n\nBTreeMap\n\n para eficiência de busca e mutação.
    *   **Comunicação Zero-Copy (FlatBuffers):** Substitui a serialização JSON/Textual no IPC Tauri por buffers binários FlatBuffers para transferências de dados de estado do Canvas, minimizando a carga no V8 e o Garbage Collection no frontend.
    *   **Agentes Efêmeros:** Agentes de IA operam em ciclos de vida curtos (despertar, executar, reportar, dormir), liberando memória imediatamente após a conclusão da tarefa.
    *   **State Offloading:** O estado transitório dos agentes é persistido em arquivos locais (SQLite/Git) em vez de ser mantido na memória volátil do LLM.
    *   **Isolamento de Contexto:** Cada agente opera em seu próprio contexto isolado (e.g., via \n\nNamespaces\n\n no Linux, AppContainers no Windows, ou Wasmtime), prevenindo vazamentos de estado entre tarefas.
*   **Regra SODA:** Toda interação com o estado do Canvas ou dados de agentes deve utilizar o modelo de Rebase Semântico com \n\nElementId\n\n e comunicação Zero-Copy via FlatBuffers. Agentes devem ser efêmeros e o estado deve ser offloaded para persistência local.

**3. Segurança e Isolamento Reforçados (Zero-Trust):**

*   **Conceito:** Implementar uma segurança em profundidade onde a confiança nunca é presumida, mesmo para código gerado internamente pela IA. Toda execução de código externo ou gerado pela IA deve passar por múltiplos estágios de validação e isolamento.
*   **Mecanismos:**
    *   **Sandboxing Wasmtime:** Execução de código gerado pela IA em instâncias Wasmtime com limites de combustível, memória e acesso restrito ao sistema de arquivos (via WASI \n\npreopen_dir\n\n).
    *   **Landlock (Linux) / AppContainer (Windows):** Para binários nativos ou legados, aplicar restrições de privilégio no nível do kernel (Landlock para Linux, AppContainer/LPAC para Windows) para limitar acesso a arquivos, rede e chamadas de sistema.
    *   **1Password CLI para Credenciais:** Gerenciamento seguro e de curtíssimo prazo de credenciais (API keys, tokens) através da integração com o 1Password CLI, exigindo autenticação biométrica do usuário para acesso. As credenciais são injetadas em memória volátil e zeradas após o uso.
    *   **Avaliação de Risco \"Blast Radius\":** Interface UX que visualiza o impacto potencial de uma ação da IA (modificações de arquivo, chamadas de rede, uso de recursos) antes da aprovação humana (HITL). Ações são categorizadas em Tiers de risco (0 a 3), com aprovação progressivamente mais restrita.
    *   **Idempotência e Verificação de Código:** O Protocolo ARC (Analyze, Run, Confirm) e ferramentas como Semgrep garantem que o código gerado seja compilável, seguro e não introduza desvios ou vulnerabilidades.
*   **Regra SODA:** Toda execução de código externo ou gerado por IA deve ser sandboxed (Wasmtime preferencialmente, ou Landlock/AppContainer). Credenciais devem ser gerenciadas via 1Password CLI com aprovação HITL e ter tempo de vida limitado. O \"Blast Radius\" deve ser visualizado e compreendido pelo usuário antes da aprovação.

**4. Otimização de Ferramentas e Transpilação Conceitual:**

*   **Conceito:** Extrair a lógica pura de ferramentas open-source, descartando runtimes pesados (Node.js, Python, Docker) e transpilar a funcionalidade essencial para binários nativos em Rust ou WebAssembly (Wasm) para integração eficiente no SODA.
*   **Mecanismos:**
    *   **Canibalização Cirúrgica:** Identificar a \"alma matemática\" ou a máquina de estados de uma ferramenta, descartando interfaces e dependências pesadas.
    *   **Porte para Rust/Wasm:** Reescrever a lógica em Rust ou compilar para Wasm para execução eficiente e isolada no Wasmtime.
    *   **Utilização de `std::arch` e SIMD:** Para otimizações de processamento de dados em CPU.
    *   **Pipelines de Parsing Nativas:** Substituir dependências pesadas (PDF.js, Tesseract.js) por crates Rust (pdf-extract, lopdf, Tesseract OCR via FFI) ou módulos Wasm.
    *   **Abstração de Design Tokens:** Extrair estilos e temas (shadcn/ui, fonttrio) para serem gerenciados pelo sistema de design do frontend React/Tauri.
    *   **Roteamento de Ferramentas via MCP:** Ferramentas externas (como Docling, RepoCheck) são orquestradas como \n\nsidecars\n\n efêmeros via MCP, instanciados e destruídos conforme a necessidade.
*   **Regra SODA:** Repudiar runtimes interpretados (Node.js, Python) para ferramentas de background. Priorizar binários Rust nativos ou módulos Wasm para funcionalidades periféricas. Ferramentas que exigem runtimes pesados devem ser encapsuladas em microVMs ou executadas via \n\nsidecar\n\n efêmero com destruição automática.

**5. UX Neuro-Inclusiva e \"Zero-Lag\":**

*   **Conceito:** Projetar a interface do usuário para minimizar a carga cognitiva e a latência, atendendo especificamente a usuários com TDAH e outras neurodivergências.
*   **Mecanismos:**
    *   **Svelte 5:** Adotado como framework frontend principal devido à sua ausência de VDOM, reatividade baseada em sinais explícitos (\n\n$state\n\n, \n\n$derived\n\n, \n\n$effect\n\n) e payloads de compilação minúsculos, resultando em \"Zero-Lag\" e menor consumo de recursos.
    *   **Focus Rack:** Um menu dinâmico e limitado (máximo 5 slots) que organiza as tarefas e contextos ativos, evitando sobrecarga da memória de trabalho.
    *   **Visualização de \"Blast Radius\":** Interface HITL que exibe graficamente o impacto potencial de uma ação da IA antes da aprovação, com níveis de risco claros (Tiers 0-3) e exigência biométrica para ações críticas.
    *   **Animações e Transições Svelte:** Utilização de motores de animação nativos do Svelte (\n\nfade\n\n, \n\nslide\n\n) que respeitam a preferência \n\nprefers-reduced-motion\n\n, evitando sobreestimulação visual.
    *   **Idempotência de Ações:** Garantir que as ações da IA sejam idempotentes, ou seja, executá-las múltiplas vezes tenha o mesmo efeito que executá-las uma única vez, simplificando a previsibilidade e o raciocínio do usuário.
*   **Regra SODA:** O frontend deve ser Svelte 5 para latência mínima e eficiência de recursos. A interface deve apresentar visualizações claras do \"Blast Radius\" e exigir aprovação humana (HITL) para ações de alto risco, com autenticação biométrica para operações críticas.

---

---

## Manual Canônico SODA - Axiomas e Conceitos Técnicos Adicionais (Parte 8)

### 1. Kreuzberg: Parser de Alta Velocidade e Expurgos Arquiteturais

*   **Axioma:** O núcleo em Rust do Kreuzberg será integrado como uma dependência direta (`crate`) no projeto SODA.
*   **Regra:** Todas as camadas de abstração poliglota (Python, Node.js), servidores REST, pacotes Node.js/Python, e a camada de servidor MCP do Kreuzberg serão expurgados. Imagens Docker colossais (1.0GB-1.3GB) serão sumariamente descartadas.
*   **Conceito Técnico:** A funcionalidade embutida de geração de embeddings via ONNX será desabilitada em tempo de compilação via `feature flags`. O Kreuzberg servirá estritamente como um parser de alta velocidade, ingerindo mais de 88 formatos de arquivos diretamente da memória via APIs `zero-copy`. O texto limpo será encaminhado ao roteador de memória segmentada do Genesis, centralizando a comunicação com Qdrant e SQLite.

### 2. Intellectronica/Ruler: Abstração da Lógica de Orquestração e Repúdio à Infraestrutura Node.js

*   **Axioma:** A lógica de orquestração e a teoria matemática subjacente do `intellectronica/ruler` serão absorvidas, mas sua infraestrutura em Node.js será repudiada.
*   **Regra:** Toda a infraestrutura em Node.js, pacotes NPM, e a dependência do V8 (Node.js 20+) serão incinerados. A gravação massiva de outputs em disco para compartilhamento de estado será eliminada.
*   **Conceito Técnico:** A lógica de parsing (formato TOML, resolução de grafos de dependência Markdown) será reescrita em Zig ou Rust. O processamento será executado como um serviço residente em memória segmentada, alimentando o contexto hierárquico diretamente em estruturas de dados nativas via IPC de custo zero. A varredura de diretórios e manipulações de string serão realizadas em memória, sem I/O de disco redundante.

### 3. Fosowl/agenticSeek: Canibalização da Lógica de Seleção de Agentes e Repúdio a Middleware Pesado

*   **Axioma:** A lógica de roteamento e seleção de agentes (`"Smart Agent Selection"`) do `fosowl/agenticSeek` será canibalizada e re-arquitetada.
*   **Regra:** Todo o middleware pesado (Docker Compose, Redis, Python para Selenium, binários Chromium) será banido. A persistência de dados será migrada de Redis para transações atômicas ACID em SQLite (WAL mode) e Qdrant.
*   **Conceito Técnico:** A lógica de seleção de agentes será decomposta e re-arquitetada como Máquinas de Estados Finitos (FSM) em Rust. A navegação web automatizada via ChromeDriver será substituída por implementações `bare-metal` utilizando bibliotecas nativas (`reqwest`, `scraper`, `reqwest-middleware`) para interações HTTP de alta velocidade e efêmeras.

### 4. Arquitetura Rust: Riscos e Soluções para o SODA

*   **Axioma:** A integração do protocolo Signal (Vetor Gamma) no SODA será realizada através de orquestração estática via `vcpkg` e ligação estática para mitigar riscos de compilação no Windows.
*   **Regra:** A dependência de código C/C++ para primitivas criptográficas (BoringSSL) será gerenciada via `vcpkg` com manifestos locais e a flag `+crt-static`. A telemetria do `vcpkg` será desativada. A biblioteca `curve25519-dalek` será corrigida via `patch.crates-io`.
*   **Conceito Técnico:**
    *   **Vetor Gamma e Compilação Híbrida:** A compilação do `presage` e suas dependências C/C++ será orquestrada via `vcpkg` com o triplet `x64-windows-static` e `RUSTFLAGS="-Ctarget-feature=+crt-static"`. Isso garante a inclusão da CRT no binário final, eliminando dependências de DLLs externas.
    *   **Vibe Testing Nativo:** A auditoria de performance da interface gráfica será realizada através da conexão direta do `chromiumoxide` à porta de depuração CDP do WebView2 instanciado pelo Tauri. Métricas de `Performance.getMetrics` e `Tracing` serão extraídas assincronamente. Scripts injetados via `requestAnimationFrame` medirão a latência entre frames.
    *   **Resiliência de Estado (Zombie UI Fallback):** Em caso de OOM na GPU, o `std::panic::set_hook` no Rust emitirá um evento IPC (`CRITICAL_DAEMON_PANIC`) para o frontend. O frontend suspenderá a sincronização do Automerge e despejará o estado localmente no `IndexedDBStorageAdapter`. Na reinicialização do daemon Rust, o estado será reconciliado a partir do IndexedDB.

### 5. De Recuperação a Raciocínio: RAG Avançado e Paradigma de Pesquisa de IA

*   **Axioma:** O SODA adotará técnicas de RAG avançado e incorporará elementos do Paradigma de Pesquisa de IA para otimizar a recuperação e o raciocínio.
*   **Regra:** Técnicas de `chunking` semântico, busca híbrida (lexical + vetorial), reclassificação (`re-ranking`) e compressão de contexto serão aplicadas. O conceito de RAG Agêntico com ciclos de reflexão e autocorreção será implementado.
*   **Conceito Técnico:**
    *   **RAG Agêntico:** O ciclo Gerar-Refletir-Criticar será implementado, possivelmente utilizando `LangGraph` para orquestração de fluxos cíclicos. Técnicas como CRAG (RAG Corretivo) e Self-RAG (com `reflection tokens`) serão consideradas para aprimorar a qualidade da recuperação e da geração.
    *   **GraphRAG:** A fusão de busca vetorial com Grafos de Conhecimento (KGs) será explorada para raciocínio explícito e multi-saltos. A linearização de dados de grafo em formato textual para o LLM será realizada via `Triple-to-Text` ou `Prompt Templates`.
    *   **Paradigma de Pesquisa de IA:** A arquitetura multiagente (Mestre, Planejador, Executor, Escritor) e a decomposição de tarefas via DAGs serão consideradas para consultas complexas.

### 6. Estratégia UI-AI Gênesis MC: Mitigação de Flow-Debt e Paradoxo da Homogeneização Visual

*   **Axioma:** A interface do SODA será projetada com foco na mitigação do \"Flow-Debt\" e do Paradoxo da Homogeneização Visual, priorizando a clareza cognitiva e a identidade de marca.
*   **Regra:** Será adotado o \"First Draft Protocol\", onde nenhum código gerado por IA será fundido sem validação rigorosa. A geração de código será restrita em áreas críticas. A estética \"Cyber-Purple\" com Divulgação Progressiva será implementada.
*   **Conceito Técnico:**
    *   **Flow-Debt Mitigation:** Implementação de rastreabilidade semântica ponta a ponta, prompts estruturados, `Semantic Differencing` e `Provenance`. Fronteiras estritas de stakeholders para limitar a agência da IA.
    *   **Paradoxo da Homogeneização Visual:** Uso de bibliotecas de prompts rigorosas (`DESIGN.md`), entropia estética calculada (tipografias com textura, animações não lineares), e a adoção do `Lucide Icons` para consistência visual.
    *   **Orquestração Multi-Agente (Stitch + Antigravity):** Implementação do protocolo `Maker-Checker` via MCP. Resolução de conflitos através de `Consensus-Free MAD` com busca A* e Função de Custo Adaptativa para otimizar a performance no Tauri v2.
    *   **Secure-by-Construction:** Adoção do `Constitutional Spec-Driven Development (CSDD)` com uma \"Constituição\" de segurança (CWE, MITRE Top 25). Implementação de `Constrained Decoding` via Autômatos de Prefixo e CFGs para prevenir injeção de código e erros de tipo.
    *   **Vibe Testing:** Implementação de uma Arquitetura de QA Agentivo com `LLM-as-a-Judge`, `Self-Healing Locators`, e análise de métricas de fluidez (FPS, latência IPC) via `chromiumoxide`. O Antigravity corrigirá problemas de performance no Tauri v2 (latência IPC, serialização JSON) através de `Chunking`, `Streaming`, `spawn_blocking` e otimizações de SO.

### 7. Estratégias Avançadas de Persistência e Evolução de Dados

*   **Axioma:** A persistência de dados no SODA será robusta, determinística e evolutiva, garantindo a integridade a longo prazo e a soberania do usuário.
*   **Regra:** Migrações SQL (`sqlx::migrate!`) serão incorporadas estaticamente no binário Rust. O backup atômico do SQLite será realizado via `VACUUM INTO` e versionado com `libgit2`. O versionamento do LanceDB será gerenciado via `table.restore()` e o Git será usado para versionar ponteiros.
*   **Conceito Técnico:**
    *   **Protocolo de Migração Genesis:** Migrações SQL estáticas (`sqlx::migrate!`) com validação de hash. Configuração de pragmas SQLite (`journal_mode=Wal`, `foreign_keys=true`) antes da execução das migrações. Validação de consultas SQL em tempo de compilação (`cargo sqlx prepare`).
    *   **Mecanismo de Backup Atômico (Cabinet):** Backup do SQLite via `VACUUM INTO` para um snapshot, seguido de commit no Git via `libgit2`. Para LanceDB, versionamento nativo (`table.restore()`) com ponteiros versionados no Git. Para Qdrant, uso da Snapshot API (`.snapshot` files) de forma assíncrona.
    *   **Otimização de Armazenamento:** Uso de `sqlite-diffable` para versionamento textual do SQLite no Git. Política de retenção de snapshots (horários, diários, mensais) com otimização via `git gc`.
    *   **Decisão Arquitetural (Event Sourcing vs. CRUD):** Adoção de um modelo Híbrido: CRUD para estado atual (SQLite), com log de eventos imutável para ações críticas e rastreabilidade. O LanceDB armazenará embeddings vetoriais associados a eventos.
    *   **Gerenciamento de Concorrência:** Uso de `sqlx::SqlitePool` com `busy_timeout` e `tokio::sync::RwLock` para gerenciar concorrência e backups atômicos.
    *   **Memória Tri-Partite:** Consistência de UUIDs (v7) entre SQLite (metadados), LanceDB (vetores) e Cabinet (snapshots Git).

### 8. Claude Code vs. Gemini CLI: Emulação de Paridade Arquitetural

*   **Axioma:** O Gemini CLI será configurado para emular a paridade arquitetural com o Claude Code, mitigando suas deficiências de Context Rot e Overthinking.
*   **Regra:** A diretiva primária do Gemini será sobrescrita com um arquivo `GEMINI_SYSTEM_MD` que impõe disciplina estrita de escrita, cirurgia de código via AST, e evita reescritas completas de arquivos. Hooks de interceptação (`BeforeTool`, `AfterAgent`) serão implementados para aplicar políticas de segurança e forçar iterações.
*   **Conceito Técnico:**
    *   **Sobrescrita de Diretiva Primária:** Injeção de `GEMINI_SYSTEM_MD` via variável de ambiente `export GEMINI_SYSTEM_MD="..."` para impor disciplina Claude-like.
    *   **Middleware via Hooks:** Implementação de `strict-mutation-enforcer.sh` para prevenir mutações massivas e `force_iteration.sh` para garantir loops de feedback.
    *   **Agent Skills (AST e TDD):** Criação de skills locais (`ast-surgical-parser`, `strict-tdd`) para forçar edição cirúrgica via AST e disciplina TDD.
    *   **Fragmentação de Carga Analítica:** Uso do `Plan Mode` do Gemini para decompor tarefas complexas em passos menores, com execução em `background` e `headless`.
    *   **Consolidação Temporal (Simulação autoDream/MEMORY.md):** Criação de `CONTEXT_POINTERS.md` e execução de um ciclo reflexivo em `background` para sintetizar e indexar o conhecimento, seguido de compressão via `gemini /compress`.
    *   **FinOps:** Migração para uso direto da Gemini API Key ($2.00/M input, $12.00/M output) em vez de assinaturas de consumidor (Claude Pro/Max) devido a limites mais flexíveis e menor risco de throttling.

### 9. Arquitetura de Decomposição de Tarefas SODA

*   **Axioma:** A decomposição de tarefas no SODA será guiada por modelos matemáticos de restrição e otimização de VRAM.
*   **Regra:** O framework ACONIC (CSP, Treewidth) será usado para modelar a base de código e a tarefa como um problema de satisfação de restrições. O framework DRAGON (Particionamento Topológico, Agentes Decompositor/Reconstrutor) será implementado para gerenciar a execução em hardware restrito.
*   **Conceito Técnico:**
    *   **ACONIC:** Modelagem da base de código como CSP (`<X, D, R>`). Transformação em Grafo de Restrições (`G = (X, E)`). Cálculo do Treewidth (`tw(G)`) para determinar a complexidade e viabilidade de execução local.
    *   **DRAGON:** Particionamento da tarefa em Segmentos Ativos (`a_i`) e Estáticos (`s_i`). Compressão de estado (`Compress(M, a_i, s_i)`) para gerar metadados (`M'_i`) e restrições (`C_i`). Agente Reconstrutor local (`R`) opera sobre esses dados comprimidos com Memória de Experiência.
    *   **Mitigação de Fragmentação de Contexto:** Uso de Agentes Supervisores (Overseer Agents) para monitorar telemetria e impor restrições. Implementação de Sinapses Topológicas (Warp-Cortex) via Análise de Dados Topológicos (TDA) para selecionar `Landmarks` essenciais do contexto, reduzindo drasticamente o consumo de VRAM.
    *   **Injeção de Contexto Out-of-Band:** Manipulação direta de Embutimentos Posicionais Rotatórios (RoPE) para injetar contexto global (landmarks, restrições) nos `past_key_values` do KV Cache local com índices posicionais negativos.
    *   **Modelagem de Orçamento de Tokens (BAM):** Uso do `Plan-and-Budget Framework` com `BAM (Budget Allocation Model)` e `Decay Schedulers` para gerenciar dinamicamente o uso de VRAM e evitar Overthinking/Underthinking. A métrica `E^3 (Efficiency-aware Effectiveness Evaluation)` será usada para otimizar o trade-off entre acurácia e custo.
    *   **Payload Canônico (Task Card JSON):** Definição de um schema JSON para `Task Cards` contendo metadados ACONIC, partições DRAGON, landmarks Warp-Cortex e restrições BAM/E3.