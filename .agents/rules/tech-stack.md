---
trigger: always_on
---

##### 📜 WORKSPACE RULES: Genesis - Mission Control (SODA V2)
**Versão:** 2.1 (Canon V2 Definitivo) ESTAS REGRAS SÃO ABSOLUTAS E SOBRESCREVEM QUALQUER PREMISSA GLOBAL DA IDE PARA O CONTEXTO DESTE PROJETO.

###### 1. STACK TECNOLÓGICO IMUTÁVEL (BARE-METAL CORE)
*   **Backend / Core:** Rust puro (assíncrono via `tokio`).
*   **Frontend / UI:** Svelte 5 (Runes), TypeScript e Tailwind CSS v4 empacotados em Tauri v2.
*   **HardwareOps (A Lei da Separação de Hardware):**
    *   **dGPU (RTX 2060m - 6GB VRAM):** USO EXCLUSIVO para inferência generativa pesada e retenção do *KV Cache*. Restrita a micro-SLMs quantizados em `Q4_K_M` (1.5B a 4B parâmetros, ex: Qwen 2.5 3B, Llama 3.2 3B). Proibidos modelos de 8B+ para aniquilar o letal *Spillover* do barramento PCIe.
    *   **iGPU (Intel UHD 630):** USO EXCLUSIVO para renderizar a interface gráfica Svelte no modo `LowPower` da API WGPU. Proibida de encostar em tensores de IA.
    *   **CPU (Intel i9 + AVX2):** Roteamento, Garbage Collection Semântico (Chyros Daemon), processamento de áudio em FP32 (Kokoro-82M) e Avaliação Epistêmica ultrarrápida.

###### 2. MOTORES DE IA E INFERÊNCIA (O FIM DO MONOLITO)
*   **Motor Generativo Principal:** A IA roda nativamente no ecossistema Rust usando **Candle**, **Burn (CubeCL)** e **mistral.rs**.
*   **A Prisão do llama.cpp:** O `llama.cpp` monolítico e daemons externos (Ollama/LM Studio) estão BANIDOS. A crate `llama-cpp-4` sobrevive isolada operando na CPU EXCLUSIVAMENTE para *Logit Probing* (Avaliador Epistêmico), extraindo a probabilidade matemática do risco sem gerar texto.
*   **Decodificação Restrita (Constrained Decoding):** Tarefas de extração JSON (ETL) não operam por prompt livre. OBRIGATÓRIO o uso da crate `llguidance` em Rust para forçar a saída contra um Autômato de Gramática Livre de Contexto em 50µs.
*   **Atenção Esparsa:** A compressão de contexto no Rust (framework `candle`) DEVE usar **Max Pooling** (blocos de ~64 tokens), sendo proibido o *Mean Pooling*. Isso preserva outliers vitais como caminhos absolutos de arquivos e URIs.

###### 3. COMUNICAÇÃO ZERO-GARBAGE E UI REATIVA
*   **O Frontend é Passivo:** O Svelte 5 atua estritamente como lente de exibição via Runes (`$state`, `$derived`). Lógica de negócios no cliente é PROIBIDA.
*   **Ilhas WebGL:** A renderização de grafos pesados não usará bibliotecas baseadas em DOM (Svelte Flow). Utilizaremos **Ilhas WebGL** (`three.wasm`) em Web Workers isolados para não engasgar a *Main Thread*.
*   **IPC Zero-Copy (A Barreira do Tauri v2):** PROIBIDO trafegar volumes de dados em JSON. A comunicação via `tauri::ipc::Channel` exige buffers binários nativos: **Apache Arrow** ou **rkyv** (via *Transferable Objects* do Worker para a Main Thread).

###### 4. A TRÍADE DE MEMÓRIA E PERSISTÊNCIA COGNITIVA
Proibido tratar a memória como um banco vetorial cego e único.
1.  **L1 (Transiente):** Índices em RAM via Tokio.
2.  **L2 (Relacional/Episódica):** **SQLite** (Modo WAL, MVCC, FTS5) atuando como fonte da verdade em *Event Sourcing*.
3.  **L3 (Grafos / Semântica):** **LadybugDB** (100% Rust) para grafos causais multi-hop (FalkorDB e KùzuDB estão BANIDOS). **LanceDB** via `mmap` direto do SSD para busca vetorial de documentos.
*   **RAG Temporal:** O TG-RAG está banido. O tempo é resolvido via **Pré-filtragem B-Tree Hard SQL** no LanceDB associada a *Contextual Chunks*. Usa-se a taxonomia `STABLE` vs `EVOLVING` para proteger o conhecimento núcleo do viés de recência.

###### 5. SANDBOXING E ISOLAMENTO EFÊMERO (ZERO-TRUST)
*   **A Exceção do Docker:** O uso de Docker é restrito EXCLUSIVAMENTE ao Antigravity IDE (Ambiente de Desenvolvimento/Fábrica). No produto SODA final, contêineres Docker são proibidos.
*   **Sidecars Efêmeros em Produção:** 
    *   Lógicas puras isoladas via **Wasmtime (WASI 0.2/0.3)**.
    *   Scripts pesados (Python/OCR) rodam em **Micro-VMs (KVM)** bifurcadas via *Copy-on-Write (Clone VMM)*, atreladas a Cgroups v2. Destruição atômica (SIGKILL) imediata após o uso garantida pelo `Drop` trait no Rust (*Process Pool Guard*).
    *   Acesso a ferramentas do host é contido estritamente via **Landlock** (Linux) e **AppContainer** (Windows).
    *   Comunicação com Micro-VMs usa memória compartilhada POSIX via crate `iceoryx2` (Zero-Copy).

###### 6. SEGURANÇA DE WORKSPACE E PREVENÇÃO SDC
*   **Rebase Semântico Atômico:** PROIBIDO usar CRDTs pesados (Yjs/Automerge) para edição concorrente. Edições paralelas de arquivos usam *Tracked IdList* e tombstones arbitrados atomicamente pelo Mutex assíncrono do Tokio.
*   **Defesa de Arquivos em O(1):** PROIBIDO usar `std::fs::File::write` ingênuo. A mutação de arquivos no workspace exige *Hard Links* instantâneos (crate `snapsafe`) pareados com escrita temporária (crate `atomic-write-file`). Isso aniquila a Corrupção Silenciosa de Dados (SDC).
*   **Agent Inbox (HITL):** A IA está proibida de corromper arquivos silenciosamente em background. Edições não-triviais geram *Pull Requests* para a *Agent Inbox*, exigindo exibição da "Matriz do Blast Radius" e aprovação explícita do usuário.