---
id: "ADR-002"
title: "ADR-002-Sandboxing-Hibrido"
version: 2.0
status: Ativo_Inegociavel
epic: "Sandboxing"
description: "Institui a matriz de contenção bare-metal: Wasmtime para lógica pura, AppContainer/LPAC (Windows) e Landlock (Linux) para sidecars nativos, e WSB/ProjFS para workspaces efêmeros."
---

### ADR-002: Sandboxing Híbrido e Matriz de Contenção Bare-Metal

#### Status
Aceito (Ativo, Inegociável e Fundacional para SOULS V4)

#### Contexto Técnico e Ameaça Operacional
O ecossistema SOULS permite que agentes de IA autônomos planejem, gerem e executem códigos, *scripts* e ferramentas (MCPs) interagindo diretamente com o sistema operacional hospedeiro [1]. Deixar essa execução livre representa o risco máximo de *Remote Code Execution* (RCE), deleção arbitrária de arquivos do host, corrupção silenciosa de dados e vazamento de chaves de API [1]. 

Em contrapartida, utilizar Hypervisors pesados, conteinerização de mercado baseada em *daemons* (como Docker completo) ou Máquinas Virtuais tradicionais (ex: QEMU, Firecracker) para estabelecer esse isolamento "sufocaria" irremediavelmente o hardware do usuário (Intel i9 / 32GB RAM) [1]. Isso introduziria concorrência desleal pelo barramento PCIe e disputaria recursos de CPU com a dGPU (RTX 2060m) durante a inferência [1]. O sistema de segurança do SOULS não pode introduzir *Flow-Debt* (micro-congelamentos na UX) provocado pela inicialização de contêineres obesos e ineficientes [1, 2]. 

#### Decisão Arquitetural (A Matriz de Contenção Híbrida)
Fica decretado o abandono sumário de estratégias genéricas de isolamento. O SOULS institui uma barreira de quatro camadas executada estritamente no nível do *kernel* do sistema operacional e no tempo de compilação, pautada exclusivamente no "Pessimismo da Razão" [2, 3]:

**Módulo 1: O Enjaulamento de Lógica Pura (Wasmtime e WASI 0.3)**
*   Todo processamento lógico determinístico, *parsers* propensos a riscos e falhas de memória (ex: gramáticas C/C++ carregadas pelo `tree-sitter`) e execução de *plugins* de terceiros ou gerados dinamicamente pela IA devem ser estritamente enjaulados em WebAssembly [2, 4].
*   O *runtime* adotado é o `wasmtime`, operando sob os padrões do *WebAssembly Component Model* (WASI 0.2/0.3) [2, 4].
*   A inicialização destas sandboxes Wasm ocorre em menos de `5ms`, isolando falhas de segmentação (*segfaults*) e pânicos lógicos que, de outra forma, derrubariam o *Event Loop* do Tokio no núcleo do Rust [2, 5]. O ambiente opera sob a política *Zero-Trust*, com negação de acesso à rede e ao disco por padrão, exigindo injeção explícita de *Capabilities* [4, 5].

**Módulo 2: Isolamento de Sidecars Nativos (AppContainer/LPAC e Landlock)**
*   Processos binários nativos ou interpretadores inevitáveis (como Python local) que precisem ser instanciados pelo SOULS atuarão como *Sidecars Efêmeros*, governados pela regra do Menor Privilégio Imposto pelo Kernel [5, 6].
*   **No Windows (Host Primário):** É OBRIGATÓRIA a adoção de `AppContainer` com restrição cirúrgica de tokens SID através de **LPAC** (*Less Privileged AppContainer*) e **MXC** (*Microsoft Execution Containers*) [2, 3]. Essas invocações são feitas de forma nativa via API Win32 [3]. Qualquer acesso à rede externa ou a discos globais é bloqueado fisicamente pelo kernel do Windows [2].
*   **No Linux:** Adoção de auto-enjaulamento nativo via `Landlock` (Linux Security Module). As restrições de caminhos de arquivos são aplicadas monotonicamente logo no boot do processo, garantindo tempo atômico de resposta [2, 5]. 

**Módulo 3: Workspaces Efêmeros e Preservação SSD (ProjFS e RAMDisk)**
*   Agentes de IA estão terminantemente proibidos de realizar mutações de E/S diretamente nos repositórios produtivos reais do usuário sem passar pelo funil de aprovação na *Agent Inbox* (*Human-In-The-Loop*) [7, 8].
*   Para o desenvolvimento autônomo, o SOULS instanciará *Shadow Workspaces* virtuais e transientes utilizando o **ProjFS** (*Projected File System* - Windows/NTFS) [9, 10]. 
*   Para proteger o tempo de vida útil do hardware e evitar o desgaste de células físicas de SSD (economia de *Terabytes Written* - TBW), os clones e edições efêmeras dos repositórios ocorrerão em um disco virtual montado na RAM (**RAMDisk / ImDisk**) dinamicamente alocado pelo Daemon Rust [10]. 
*   O estado mutável utiliza arquitetura *Copy-on-Write* (CoW); se o agente falhar ou sofrer a guilhotina do `SIGKILL`, o RAMDisk é desmontado e a sessão "vira fumaça" sem corromper a matriz original do usuário [9, 11].

**Módulo 4: Quarentena Dinâmica de Código Hostil (Windows Sandbox - WSB)**
*   Para a auditoria de *linters* pesados não confiáveis ou na execução compulsória de código *vibecoded* de extrema hostilidade por parte dos agentes, a execução *bare-metal* será vetada na raiz [2, 12].
*   O SOULS orquestrará a geração dinâmica de arquivos XML `.wsb` via código Rust para invocar instantaneamente o **Windows Sandbox (Hyper-V)** [2, 12].
*   Esta micro-VM descartável embute os diretórios do repositório de análise mapeados estritamente no modo *Read-Only* (Somente-Leitura). O tempo de *boot* gira em torno de ~2 segundos [12].
*   Ao encerrar o escopo da tarefa, o Rust destrói a instância do Sandbox; o sistema aniquila completamente os resquícios em nível de hipervisor, garantindo a esterilização terminal de eventuais persistências malignas [6, 12].

#### Consequências Operacionais e Defesa contra o Slop (Trade-offs)
*   **Impacto Positivo:** O risco de Execução Remota de Código (RCE) no hospedeiro cai para virtualmente zero. As taxas de sobrecarga na CPU i9 e a ocupação da RAM de 32GB advindas da adoção do *Wasmtime* e das amarras do *Landlock/LPAC* são matematicamente invisíveis ($\mathcal{O}(1)$), erradicando qualquer atrito com o barramento do núcleo de inferência da GPU RTX 2060m [2, 6]. Os *sidecars* morrem de forma atômica e limpa.
*   **Impacto Negativo (Rigidez):** O esforço contínuo de manutenção da infraestrutura exige domínio profundo de engenharia de chamadas de baixo nível (*low-level Syscalls*) e domínio contínuo da API Win32 para a alocação dos tokens de segurança (LPAC/MXC) [3]. *Plugins* ou ferramentas criadas por desenvolvedores que não declararem perfeitamente seus manifestos de rede ou acesso a pastas tomarão bloqueios letais irreversíveis por parte do kernel (código de erro `-EPERM` / *Access Denied*), demandando uma engenharia de *Capabilities* extremamente engessada e disciplinada.

### Limitações Conhecidas e Riscos Residuais (O Fio Desencapado)
A implementação da "Gaiola de Silício" via AppContainer/LPAC isola perfeitamente o sistema de arquivos (NTFS) e a rede, mas deixa três vetores de risco latentes que exigem vigilância arquitetural extrema. Sob a doutrina do *Fail-Closed*, as seguintes proibições e mitigações estão ativas:
**A) Vulnerabilidade a Timing Attacks em Blocos `unsafe`**
A injeção de permissões de Kernel e manipulação de SIDs exigiu o uso extensivo de blocos `unsafe` e ponteiros brutos (C-FFI via `windows-sys`). Qualquer futura lógica de validação de chaves, tokens ou credenciais confidenciais dentro ou nas fronteiras deste escopo DEVE obrigatoriamente utilizar verificação de tempo constante (ex: crate `constant_time_eq`). Validações condicionais ingênuas podem vazar os limites de isolamento via análise de ciclos de CPU (*side-channel timing attacks*).
**B) Proibição de Acesso Concorrente à GPU**
A jaula LPAC não resolve a virtualização de hardware de vídeo. Os processos *sidecars* enjaulados nascem absolutamente cegos para a placa de vídeo. É expressamente proibido tentar mapear drivers de GPU diretamente para múltiplos *sidecars* efêmeros em paralelo. O processo pai SOULS atuará unicamente como um "Mediator Broker" restrito. Pedidos de inferência pesada devem ser serializados no *pipeline*, e apenas o processo mestre em Rust tocará na VRAM, prevenindo a asfixia da dGPU.
**C) O Limite do Zero-Copy Purista em WebViews**
Apesar da ponte de IPC via *Named Pipes* estar liberada com a DACL `ALL APPLICATION PACKAGES` para comunicação com o host, é arquiteturalmente proibido tentar mapear a memória compartilhada da jaula diretamente para a interface visual (Svelte 5) na tentativa de burlar o IPC do Tauri. WebViews modernas (como WebView2) exigem isolamento rigoroso de origem cruzada (COOP/COEP) e punem mapeamentos de memória não-policiados. Tentativas de "Zero-Copy purista" direto na UI resultarão em *seg-faults* violentos no processo de renderização. O fluxo de dados deve trafegar obrigatoriamente pelos dutos binários pré-estabelecidos (`rkyv` / `Apache Arrow`).