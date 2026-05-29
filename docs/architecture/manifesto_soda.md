# 00_MANIFESTO_SODA: Sovereign Operating Data Architecture

**Versão:** 3.0 (Definitiva - Bare-Metal Precision)
**Status:** ATIVO E INEGOCIÁVEL
**Alvo da Leitura:** Agentes Orquestradores (Antigravity IDE), Engenheiros de Sistema, Arquitetos de Solução.

## 1. A NATUREZA DO SODA (O QUE SOMOS)

O **Genesis Mission Control (SODA)** não é um aplicativo web, não é um wrapper de API de nuvem e não é um chatbot glorificado.

Ele é um **Sistema Operacional Agêntico Soberano** e um **Exoesqueleto Cognitivo**. Projetado para atuar como um _Sparring Partner_ implacável e proativo, o SODA opera ininterruptamente no _Bare-Metal_ (hardware local) do usuário. Sua missão é forjar uma simbiose humano-máquina perfeita, atuando como uma Prótese de Função Executiva projetada matematicamente para mentes neurodivergentes (**2e / TDAH**), absorvendo o caos logístico para liberar o hiperfoco criativo.

## 2. A LEI DA FÍSICA: RESTRIÇÕES DE HARDWARE

Nenhuma arquitetura, biblioteca ou ferramenta pode ser sugerida ou implementada se violar os limites termodinâmicos e de memória da máquina hospedeira. **A Nuvem é considerada um ambiente hostil e secundário; o local é o Rei.**

- **CPU:** Intel Core i9 (AVX2). Usado primariamente para roteamento mecânico, orquestração assíncrona, processamento isolado de áudio (FP32) e gestão de bancos de dados locais.
- **RAM:** 32GB DDR4/DDR5. Atua como repositório de estado em repouso e expansão elástica para Offloading de tensores e KV Cache (Unified Memory).
- **GPU (O Gargalo Crítico):** NVIDIA RTX 2060m com **TETO ESTRITO DE 6GB VRAM**.
    - _Regra de Ouro:_ É fisicamente impossível carregar modelos monolíticos densos com grandes janelas de contexto estáticas aqui. Agentes devem ser **efêmeros** e a memória contextual deve ser **comprimida, dinâmica e delegada**.

## 3. OS QUATRO PILARES ARQUITETURAIS

Todo código gerado para o ecossistema SODA deve obedecer irrestritamente a estes quatro pilares:

### I. Córtex Motor (Rust & Tauri v2)

- **Zero Interpretadores em Background:** É terminantemente **PROIBIDA** a execução de daemons contínuos em Node.js, Python, Electron ou Java. Eles geram _Daemon Bloat_ e engasgam a CPU com _Garbage Collection_.
- **O Núcleo é Rust:** Todo o backend, roteamento, interceptação L7 e persistência de dados rodam em **Rust assíncrono (Tokio)**. O bloqueio da thread principal (Event Loop Starvation) é uma falha inaceitável.
- **Zero-Copy IPC:** A comunicação entre o cérebro (Rust) e a interface (SvelteKit) trafega dados binários puros. A serialização JSON pesada está banida das vias expressas.

### II. Hipocampo: Memória Neuro-Sintética (MNS)

O _Context Rot_ (Amnésia de IA) é o inimigo mortal. O SODA repudia o "RAG Ingênuo". A memória é dividida e persistida cirurgicamente:

- **L1 (RAM Transiente - Pointer Index):** O agente não lê textos longos da RAM, apenas referências e índices rasos para montar a árvore de intenção.
- **L2 (Single Source of Truth - Episódica):** Banco de dados **SQLite (FTS5 / WAL Mode)**. Operamos sob o paradigma de **Event Sourcing** (Append-only). O CRUD tradicional (UPDATE/DELETE) está banido para evitar o apagamento da ontogênese temporal de raciocínio da máquina.
- **L3 (Semântica/Grafos):** **LanceDB** embutido para similaridade vetorial sub-quadrática. Despejo do _KV Cache_ do LLM em arquivos `.safetensors` de 4-bits no SSD para retomada de contexto atômica em ~1.3s.
- **Chyros Daemon (AutoDream):** A desfragmentação da memória e a fusão de grafos semânticos ocorrem na ociosidade da madrugada, unicamente na CPU, poupando a VRAM.

### III. Sistema Nervoso e Roteamento (Zero-Trust)

A máquina presume que a rede, as ferramentas de terceiros e o próprio LLM são entidades estocásticas passíveis de alucinação e decepção.

- **Prisões Wasmtime:** Qualquer código ou script de automação não-nativo gerado pelo agente ou baixado via MCP (Model Context Protocol) roda dentro de uma sandbox **WebAssembly (Wasmtime)** sem acesso arbitrário a disco ou rede.
- **AgentGateway L7 Embutido:** O roteamento MCP não usa portas abertas vulneráveis ou proxies em linguagens interpretadas. O Gateway é compilado nativamente no Rust, usando algoritmos de interceptação léxica (_Aho-Corasick_) e Tipagem Estrita (_Typestate Pattern_) para bloquear sequestros de sessão.
- **Roteamento Mecanicista & ParetoBandit:** A triagem das tarefas obedece à _Entropia Semântica_. A dGPU processa tarefas seguras e curtas. Se o limite cognitivo ou o risco de alucinação exceder os limites, a equação do _ParetoBandit_ aplica o FinOps e roteia taticamente para a Nuvem de forma invisível.

### IV. Lente Cognitiva (Cyber-Neuro Synthesis)

A interface não é um site comercial; é um instrumento clínico, tátil e de precisão.

- **Canvas-First & Passividade:** A UI (SvelteFlow/Tailwind v4) é **estritamente passiva**. Nenhuma lógica de negócios reside no frontend.
- **Rebase Semântico Atômico:** CRDTs pesados (Yjs/Automerge) estão banidos por consumirem recursos excessivos. A concorrência de edição no Canvas é resolvida por um Árbitro Autoritário na base Rust.
- **UX de Neurodiversidade:** Implementação agressiva de **Divulgação Progressiva** e **Focus Rack** (limite rígido de contextos). O _Nothing Design_ impõe cores profundas, contraste absoluto e ausência de animações supérfluas. A instância mecânica deve responder em $< 50ms$.

## 4. GOVERNANÇA E EXECUÇÃO (A DOUTRINA DO AGENTE)

Qualquer agente autônomo atuando neste código-fonte está rigorosamente proibido de praticar _Vibe Coding_ irresponsável.

1. **Fricção Socrática e Debate Anti-Consenso:** O agente deve questionar ativamente as premissas estruturais do usuário antes de codificar. A IA atua como um _Red Teamer_ local.
2. **Spec-Driven Development (SDD):** Planejar precede executar. Nenhum componente nasce sem especificação arquitetural prévia.
3. **First Draft Protocol & GitOps:** O agente nunca injeta código diretamente no fluxo principal. O código nasce em um _Shadow Workspace_, passa pela prova matemática do compilador Rust, atende aos testes de resiliência sistêmica e submete-se à verificação humana (_Human-In-The-Loop_) no Canvas antes do _Merge_. Toda extração e canibalização de repositórios de terceiros obedece à quarentena atômica do Git Subrepo.

_Fim do Manifesto. A máquina está alinhada. O hardware é a lei. A intenção é absoluta._