---
trigger: always_on
---

Revisado: 2026-06-07

###### 1. A REGRA DE OURO DA STACK: FÁBRICA VS. PRODUTO (DUALIDADE SISTÊMICA)
Você (Antigravity IDE) deve separar estritamente a "esteira de montagem" do código que será entregue ao usuário.
*   **Na Fábrica (Seu Ambiente de Dev):** Você TEM PERMISSÃO para usar contêineres Docker, Python, Bash e APIs de nuvem nos seus *Shadow Workspaces* para debugar, prototipar algoritmos e executar o ETL Cognitivo massivo.
*   **No Produto (Código SODA em Produção):** O código final DEVE ser estritamente *Bare-Metal* (Rust/Tokio + Svelte 5/Tauri v2). É INEGOCIÁVEL e TERMINANTEMENTE PROIBIDO o uso de Node.js, Python residente, ou servidores web locais no pacote de produção. Ferramentas de terceiros devem ser convertidas em *Sidecars Efêmeros* que morrem atomicamente após o uso (`SIGKILL`).

###### 2. STACK TECNOLÓGICO IMUTÁVEL E HARDWARE-OPS
*   **Backend / Core:** Rust puro (assíncrono via `tokio`).
*   **Frontend / UI:** Svelte 5 (Runes), TypeScript e Tailwind CSS v4 empacotados em Tauri v2. Em runtime, o SODA opera como System Tray Daemon invisível no boot; a UI é lente sob demanda.
*   **HardwareOps (A Lei da Separação Termodinâmica):**
    *   **dGPU (RTX 2060m - 6GB VRAM):** USO EXCLUSIVO para inferência generativa de "trabalho braçal" e retenção do *KV Cache*. Restrita a micro-SLMs quantizados em Q4_K_M (1.5B a 4B parâmetros, ex: Qwen 2.5 3B). ESTRITAMENTE PROIBIDOS modelos de 8B+ para aniquilar o letal *Spillover* do barramento PCIe.
    *   **CPU (Intel i9 + AVX2):** USO EXCLUSIVO para o Roteamento Semântico (Nível 0 do ParetoBandit), o *Garbage Collection Semântico* (Chyros Daemon), processamento de áudio em FP32 (Kokoro-82M) e Avaliação Epistêmica ultrarrápida via AVX2.
    *   **iGPU (Intel UHD 630):** BANIDA DE QUALQUER OPERAÇÃO DE IA. É expressamente proibido alocar LLMs, SLMs ou tensores na iGPU devido ao estrangulamento letal da banda de memória RAM. Seu uso é ESTRITAMENTE PASSIVO, restrito unicamente à renderização da interface gráfica Svelte no modo `LowPower` da API WGPU.

###### 3. MOTORES DE IA E INFERÊNCIA (O FIM DO MONOLITO)
*   **Motores Generativos Nativos:** A IA roda nativamente no ecossistema Rust usando **Candle**, **Burn (CubeCL)** e **mistral.rs**.
*   **A Prisão do llama.cpp:** O `llama.cpp` monolítico e daemons externos (Ollama/LM Studio) estão SUMARIAMENTE BANIDOS do núcleo generativo. A crate `llama-cpp-4` sobrevive isolada operando na CPU EXCLUSIVAMENTE como um "bisturi" para *Logit Probing* (Avaliador Epistêmico), extraindo a probabilidade matemática do risco em <150ms sem gerar texto.
*   **Decodificação Restrita (Constrained Decoding):** Tarefas de extração estruturada (JSON/ETL) não operam por prompt livre. É OBRIGATÓRIO o uso da crate `llguidance` em Rust para forçar a saída contra um Autômato de Gramática Livre de Contexto em meros 50µs, garantindo 100% de precisão mecânica.
*   **Atenção Esparsa e Retenção de Outliers:** A compressão de contexto longo no Rust (framework `candle`) DEVE usar **Max Pooling** (blocos de ~64 tokens). O *Mean Pooling* está PROIBIDO, pois atua como filtro passa-baixa e causa amnésia de outliers vitais (caminhos absolutos de arquivos, URIs e sintaxes exatas).

###### 4. COMUNICAÇÃO IPC ZERO-GARBAGE E UI REATIVA
Para impedir que fluxos massivos de IA e telemetria engasguem a interface (Flow-Debt), a comunicação Rust ↔ V8 exige a erradicação da "coleta de lixo" (GC) do JavaScript.
*   **Transporte Binário:** Toda comunicação de grande volume ocorre estritamente via buffers binários brutos (**Apache Arrow** para logs colunares ou **rkyv** para offsets). É PROIBIDA a serialização massiva em JSON.
*   **Transferable Objects:** No frontend, os buffers são interceptados por *Web Workers* em background e entregues à *Main Thread* do Svelte como `Transferable Objects` (custo de alocação de memória zero).
*   **Ilhas WebGL (Vetor Omicron):** A renderização de grafos pesados utilizará Ilhas WebGL (`three.wasm`) rodando dentro de Web Workers isolados via `OffscreenCanvas`. O uso do DOM/SVG para matrizes pesadas é proibido para não asfixiar a Main Thread e garantir *Zero Layout Shift* (CLS).
*   **Renderização Cadenciada:** A atualização visual dos proxies reativos (`$state`) é estrangulada e atrelada nativamente ao `requestAnimationFrame` (rAF).

###### 5. TOPOLOGIA DE SANDBOXING NATIVO E ISOLAMENTO HÍBRIDO
A infraestrutura repudia o *overengineering* de hipervisores pesados e máquinas virtuais isoladas genéricas (QEMU/Firecracker).
*   **Lógicas Puras (IA e Scripts Leves):** Ferramentas autônomas geradas pelos agentes devem rodar isoladas e sem estado usando o **Wasmtime** (WASI 0.2/0.3).
*   **Sidecars Efêmeros Pesados (Clone VMM):** Para rodar bibliotecas Python pesadas (como OCR/Docling), o sistema utilizará Micro-VMs com *Copy-on-Write* (CoW) a partir de um *Snapshot* inerte na RAM, garantindo boot em ~10ms. A GPU NUNCA é repassada fisicamente a estes sidecars; usa-se o padrão *Mediator Broker* via memória compartilhada (`iceoryx2`).
*   **Ferramentas de Host e Binários:** Interações que exijam acesso físico aos recursos da máquina devem ser enjauladas através de Sandboxing Nativo do Kernel. Uso rigoroso do **AppContainer e LPAC (Low Privilege AppContainer)** no Windows (via crate `rappct`) e **Landlock** no Linux.
*   **Bifurcação ReFS/NTFS (ProjFS):** O ProjFS (prjflt.sys) exige minifiltro e não funciona em Dev Drive (ReFS). Workspaces efêmeros devem usar %TEMP% (NTFS) em `.souls_workspaces` via `std::env::temp_dir()` e ser removidos via guilhotina não-bloqueante (`spawn_detached_delete_process`).
*   **Process Pool Guard (A Guilhotina Atômica):** Qualquer *Sidecar* possui um limite de memória via `Cgroups v2`. O SODA usa o paradigma `Drop trait` do Rust para emitir um `SIGKILL` atômico assim que a tarefa finaliza ou aborta. Processos zumbis estão banidos.

###### 5.1. LEIS DE PERFORMANCE SAST E SANDBOXING
Toda futura ferramenta de linha de comando, varredura estática ou sidecar de análise criada no SODA DEVE obedecer às 4 leis abaixo:
*   **Timeout Adaptativo Obrigatório:** Use `--allow-rule-timeout-control` sempre que a ferramenta suportar controle por regra/arquivo. Timeout cego e uniforme é proibido como estratégia primária.
*   **Escudo de Supply Chain:** Exclusões como `tests/` e `**/mocks/*` são permitidas para reduzir ruído, mas é estritamente proibido ignorar manifestos e lockfiles (`Cargo.lock`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `go.sum`, `poetry.lock`, `Pipfile.lock`, `mix.lock` e equivalentes).
*   **Fobia de Código Minificado:** Use `--exclude-minified-files` sempre que disponível. Sem suporte nativo, replique a heurística local: arquivos com menos de 7% de espaço em branco não entram no pipeline de parsing ou scan.
*   **Higiene de Cache e Build:** Sidecars que disparem compilações agressivas ou materializem `target/` e caches equivalentes DEVEM limpar estes diretórios imediatamente após o uso para proteger Ramdisk, sandbox e SSD contra saturação.

###### 6. FINOPS, ROTEAMENTO HÍBRIDO E PARETOBANDIT
*   **O Cofre (ParetoBandit e E³):** A decisão autônoma de onde a tarefa roda não é estática. O algoritmo matemático `ParetoBandit` no Gateway Rust aplica a métrica $E^3$ (Efficiency-aware Effectiveness Evaluation) avaliando Custo vs. Qualidade vs. Latência antes do despacho.
*   **O Padrão Orchestrator-Worker:** Modelos Premium em nuvem (Claude Opus 4.7, GPT-5.4) são estritamente restritos a atuar como "Cloud Brain", lendo intenções e gerando Grafos Acíclicos Dirigidos (DAGs). O "Trabalho Braçal" resultante é despachado compulsoriamente para o Local Worker (RTX 2060m - Custo Zero) ou para Batch APIs asiáticas (DeepSeek V4, Gemini Flash).
*   **Circuit Breakers (Disjuntores FinOps):** Se o limite diário de tokens ou o custo da assinatura ameaçar estourar, o Gateway atua como um disjuntor de rede, cortando a nuvem e empurrando toda a carga para a inferência local obrigatória.
