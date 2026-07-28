---
id: "ADR-003"
title: "ADR-003-Zero-Copy-IPC"
version: 2.0
status: Ativo_Inegociavel
epic: "IPC"
description: "Impõe a adoção do iceoryx2 (Lock-Free O(1)) para o Data Plane e UDS/stdio para o Control Plane de MCPs, erradicando o JSON e a alocação dinâmica no núcleo."
---

### ADR-003: Comunicação Zero-Copy IPC e Arquitetura de Data/Control Plane

#### Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SODA V4)

#### Contexto Técnico e o Gargalo de Serialização
Durante o fluxo de inferência local e telemetria profunda, o núcleo Rust do SODA despeja fluxos contínuos e massivos de dados (estados de grafos, logs estruturados e blocos de AST/texto de modelos SLM). Serializar essa montanha de dados em cadeias de texto JSON para transportá-los até a interface gráfica (Svelte 5 / Tauri) ou entre agentes gera gargalos drásticos de I/O [2]. 

A constante alocação e desalocação de objetos JSON no motor JavaScript aciona ciclos agressivos do *Garbage Collector* (GC) do V8. Na dGPU RTX 2060m e CPU i9 sob estresse máximo, esse overhead de memória não-nativa se converte em "Flow-Debt" (micro-congelamentos de tela e engasgos operacionais), o que é intolerável para a interface reativa e neuro-inclusiva do SODA [2, 6].

#### Decisão Arquitetural (A Separação Data Plane / Control Plane)
Fica decretada a extinção da serialização JSON nas rotinas centrais de alta performance do *runtime*, adotando um modelo estrito e bipartido de comunicação *Inter-Process Communication* (IPC):

**Módulo 1: O Data Plane Absoluto (`iceoryx2`)**
*   Para transporte de cargas massivas (como tensores, grandes grafos do *LadybugDB*, matrizes do *LanceDB* e árvores de sintaxe vetorial), impõe-se a adoção da crate **`iceoryx2`** [4].
*   Esta tecnologia provê um *middleware* *Lock-Free* verdadeiro baseado em memória compartilhada POSIX, garantindo latência de transporte $\mathcal{O}(1)$ inferior a 1 microssegundo, independentemente do tamanho do payload [4].
*   O formato binário bruto dos dados trafegados na memória compartilhada será estritamente mapeado via **Apache Arrow** (para matrizes e métricas) ou **`rkyv`** (para serialização atômica zero-copy de estruturas Rust) [1, 2].

**Módulo 2: O Control Plane Restrito (UDS / stdio)**
*   Para evitar a fragmentação excessiva da memória física e do *kernel* com múltiplas tabelas de páginas (*mmap*), o `iceoryx2` fica PROIBIDO para a passagem de mensagens simples e sinais de controle [5].
*   A sinalização de estado de agentes (acordar, dormir, telemetria leve) utilizará **Unix Domain Sockets (UDS) não bloqueantes** suportados por primitivas de evento do SO (*epoll/eventfd* no Linux, Named Pipes no Windows) [3].
*   **A Alfândega MCP:** Para a integração com ferramentas e servidores do ecossistema *Model Context Protocol* (MCP), o SODA operará os fluxos de controle através de **`stdio`** e JSON-RPC 2.0 [5, 7]. A fronteira do JSON fica restrita exclusivamente à "alfândega externa" do sistema, isolada do barramento de dados principal.
*   **Emenda Constitucional: A Lei do Isolamento do Stdio (Sanidade do MCP):**
    *   Nos binários e sidecars do ecossistema SODA que operam como servidores ou proxies MCP via `stdio` (ex: `soda_mcp_server`, `mcp_stdio_guard`, `lean-ctx`, `agentgateway`), **o fluxo de saída padrão (`stdout`) é considerado um recurso sagrado e exclusivo do barramento JSON-RPC / LSP**.
    *   **Proibição Absoluta:** É terminantemente PROIBIDO instanciar bibliotecas de logging ou telemetria (como `tracing-subscriber`, `env_logger`, `log`) ou rotinas manuais de escrita (`println!`, `print!`) que despejem registros no `stdout`. Qualquer byte de log emitido no `stdout` corrompe o enquadramento de payload JSON-RPC (`Content-Length`), provocando falha catastrófica de desserialização e encerramento imediato do canal MCP no cliente (`upstream closed on receive`).
    *   **Ancoragem Mandatória no `stderr`:** Todo e qualquer evento de observabilidade, telemetria e diagnóstico (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`) DEVE ser obrigatoriamente ancorado no fluxo de erro padrão (`stderr`). A inicialização de formatadores e subscritores de log deve impor a trava explícita de escrita para `stderr` (ex: `.with_writer(std::io::stderr)` ou `eprintln!`).

**Módulo 3: Expurgo de Filas e Message Brokers Obesos**
*   Fica terminantemente banido o uso de *message brokers* genéricos (como NATS JetStream, RabbitMQ) para IPC local intra-sistema [5, 8]. Tais sistemas incorrem em *overhead* de *loopback* de rede desnecessário. Adicionalmente, implementações instáveis de canais baseadas em experimentação (como `shaq` ou `Promisqs`) são reprovadas em favor da maturidade automotiva do `iceoryx2` [8].

#### Consequências Operacionais e Defesa contra o Slop (Trade-offs)
*   **Impacto Positivo:** Desoneração total do *Garbage Collector* do V8 [2]. A latência de transporte se torna estritamente matemática e isolada. Prevenção absoluta contra engasgos de tela (Flow-Debt), garantindo a estabilidade mental e operacional do usuário final [6].
*   **Impacto Negativo (Rigidez e Debug):** A programação com o `iceoryx2` e memória compartilhada *lock-free* não tolera o menor descuido. Vazamentos de memória não serão limpos por GC e problemas de dessincronização de ponteiros de leitura/escrita podem corromper os dados (exigindo extrema cautela com *unsafe Rust* nas bordas do *framework*). Ferramentas tradicionais de debug baseadas em interceptação de rede local (como Wireshark) tornam-se cegas, exigindo *profilers* nativos e ferramentas de DTrace/eBPF.
