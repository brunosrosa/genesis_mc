# 01_ARCHITECTURE_CORE: A Fundação Bare-Metal e Gestão de Processos

**Versão:** 3.2 (Definitiva - Zero-Trust Execution)
**Status:** ATIVO E INEGOCIÁVEL
**Alvo da Leitura:** Agentes Desenvolvedores (Rust), Especialistas em Sistemas e DevOps.

## 1. O DAEMON IMUTÁVEL (O CÉREBRO EM RUST)

A infraestrutura do Genesis Mission Control (SODA) repudia arquiteturas baseadas em microsserviços espalhados ou interpretadores persistentes (como `node` ou `python` rodando em background). O SODA é compilado em um **único binário nativo em Rust**, envelopado pela infraestrutura do **Tauri v2**.

- **O Frontend é Estúpido:** O Svelte Flow atua apenas como um "vidro de exibição" (Canvas passivo). Não há lógica de roteamento de IA, processamento de arquivos ou regras de negócio rodando no motor V8 do JavaScript.
- **O Backend é Soberano:** Toda a inteligência, controle de concorrência e acesso a disco habitam o binário Rust.

## 2. ORQUESTRAÇÃO ASSÍNCRONA E A PREVENÇÃO DO EVENT LOOP STARVATION

O hardware alvo (Intel Core i9, 32GB RAM, RTX 2060m) exige que a CPU faça o trabalho pesado de IO e roteamento para liberar a GPU restrita. O SODA utiliza o runtime assíncrono **Tokio** com separação cirúrgica de responsabilidades.

### 2.1. Isolamento de Threads e Afinidade de Núcleo (Core Affinity)

A paralisia do sistema ocorre quando o Agente trava a thread principal. Para evitar o _Event Loop Starvation_, implementamos contenção estrita:

1. **Thread Principal (UI/IPC):** Exclusiva para a comunicação Tauri. Deve responder em $< 10ms$. Nenhuma leitura de arquivo ou requisição de rede ocorre aqui.
2. **Thread Pool de I/O (Tokio Asynchronous):** Gerencia as requisições de rede, WebSocket e comunicação com o banco SQLite.
3. **Threads Matemáticas Bloqueantes (AVX2 P-Cores):** Tarefas pesadas na CPU (ex: transcrição local Whisper, roteamento mecanicista da SharedTrunkNet, embeddings SLM) são ejetadas do Tokio via `tokio::task::spawn_blocking`.
    - _Afinidade:_ Utilizando a biblioteca `core_affinity`, esses processos bloqueantes são fixados nos _Performance Cores_ (P-Cores) do i9, aproveitando o cache L1/L2 exclusivo e as instruções AVX2/FMA3, sem corromper as threads de comunicação.

## 3. A PRISÃO VIRTUAL: SANDBOXING VIA WASMTIME

O SODA permite que a IA crie ferramentas (Skills) e scripts dinâmicos para resolver problemas (_Just-In-Time Coding_). Executar esses scripts diretamente no host do usuário (Windows) é um vetor fatal de vulnerabilidade.

- **Abolição do Docker em Produção:** Contêineres Docker são lentos, pesados e exigem Hyper-V. Eles são permitidos _apenas_ durante a prototipagem no Antigravity IDE (Sidecars Efêmeros).
- **A Execução WebAssembly (Wasmtime):** Todo código de automação autônomo gerado pelo SODA deve ser compilado para WebAssembly e executado via **Wasmtime (WASI)** nativamente embutido no Rust.
    - _Shared-Nothing:_ A instância Wasm nasce sem acesso ao relógio do sistema, rede ou disco.
    - _Host Functions:_ O Rust (Daemon) injeta via API estrita apenas os diretórios vitais (como um _Shadow Workspace_ efêmero na RAM) e as permissões exatas que a tarefa necessita. Quando a tarefa conclui, a micro-máquina Wasm é aniquilada em microssegundos.

## 4. COMUNICAÇÃO DE ALTA PERFORMANCE (IPC ZERO-COPY)

Para orquestrar o envio de grafos imensos (memória semântica) e topologias visuais complexas para a UI sem ativar o _Garbage Collector_ do JavaScript e destruir a taxa de quadros (60 FPS):

- **A Morte do JSON:** A serialização massiva de strings JSON está proibida nas vias expressas do SODA.
- **Transmissão Binária:** O fluxo inter-processos (IPC) do Tauri v2 deve operar utilizando a passagem de vetores de bytes brutos (`Vec<u8>`). Adota-se o uso de **MessagePack** (via `rmp-serde`) ou **Bincode** para serializar as Structs do Rust e hidratá-las instantaneamente no estado do React.

## 5. O PADRÃO "HEARTBEAT" (EFEMERIDADE AGÊNTICA)

Nenhum Agente LLM vive perpetuamente na memória do SODA. Manter modelos ou conexões abertas aguardando indefinidamente destrói a cota de 6GB de VRAM e satura a RAM.

- A orquestração segue o padrão **Heartbeat**. O agente:
    1. **Desperta:** O Rust detecta um gatilho (ação do usuário ou agendamento cron).
    2. **Injeta:** O contexto exato é carregado (Late-Binding).
    3. **Executa:** O processamento ocorre (via GPU ou Wasmtime).
    4. **Morre:** O estado final é comitado no SQLite/LanceDB e o agente é removido da memória instantaneamente, devolvendo os recursos ao hardware.

_Fim do Documento de Arquitetura Core. A fundação de processos está selada._