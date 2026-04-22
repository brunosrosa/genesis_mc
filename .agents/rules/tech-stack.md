---
trigger: always_on
---

### 📜 WORKSPACE RULES: Genesis - Mission Control (SODA)
**Versão:** 1.1 (Definitiva)
ESTAS REGRAS SÃO ABSOLUTAS. ELAS ESTENDEM E SOBRESCREVEM AS REGRAS GLOBAIS DA IDE PARA O CONTEXTO DESTE PROJETO ESPECÍFICO.

#### 1. STACK TECNOLÓGICO IMUTÁVEL (BARE-METAL CORE)
* **Backend / Core:** Rust (assíncrono via tokio).
* **UI e Sincronização:** React via Tauri v2. PROIBIDO o uso de CRDTs (Yjs/Automerge) pesados. A sincronização de concorrência multi-agente no Canvas utilizará ESTRITAMENTE o **Rebase Semântico Atômico** (arquitetura IPC Zero-Copy).
* **Gestão de VRAM e Inferência:** Modelos de texto densos rodam exclusivamente na dGPU (RTX 2060m 6GB) via `Candle Framework` (HuggingFace) gerenciando a VMM dinamicamente. PROIBIDO o uso de vLLM ou llama.cpp no núcleo de produção para evitar fragmentação de memória.
* **Processamento Analógico (Áudio):** A iGPU da Intel (UHD 630) é de uso EXCLUSIVO para processamento analógico auditivo via OpenVINO (Cobra VAD, Parakeet TDT para STT e Kokoro-82M para TTS). PROIBIDO rotear modelos de linguagem (LLMs) para a iGPU.
* **Desktop Framework:** Tauri v2.
* **Frontend / UI:** React 18+ (via Vite), TypeScript.
* **Estilização:** Tailwind CSS v4.
* **UI Base:** Shadcn UI (componentes copiados para src/components, não instalados via npm wrapper).
* **Animações:** Framer Motion (apenas para micro-interações, priorizando performance).
* **Visualização de Grafos/Canvas:** React Flow / Tldraw.
* **Sandboxing de Execução:** Wasmtime (para execução isolada de lógicas ou scripts gerados).

##### 🚫 TECNOLOGIAS E PADRÕES EXPRESSAMENTE PROIBIDOS
* **NÃO** utilize Next.js, Remix ou frameworks SSR (Server-Side Rendering). Este é um app Desktop Tauri.
* **NÃO** utilize Electron.
* **NÃO** instale bibliotecas de UI baseadas em CSS-in-JS (como Material UI ou Emotion).
* **NÃO** crie APIs REST em Node.js. Toda lógica pesada, acesso a banco e leitura de arquivos DEVE ser feita em Rust.

#### 2. ARQUITETURA DE COMUNICAÇÃO (IPC ZERO-COPY)
* O frontend React é ESTRITAMENTE PASSIVO ("burro"). Ele não possui regras de negócios.
* A comunicação entre Rust e React deve priorizar o envio de **buffers binários nativos** (ou JSON estritamente tipado) via Eventos Tauri assíncronos (emit / listen) para evitar asfixia do motor V8 com serializações gigantes.
* **NUNCA** bloqueie a thread principal com comandos síncronos demorados. O I/O pesado deve rodar em `tokio::task::spawn_blocking`.

#### 3. BANCO DE DADOS E MEMÓRIA (O FIM DO DOLT/MYSQL)
* A memória L2 transacional do agente utilizará EXCLUSIVAMENTE o **SQLite (via `rusqlite` ou `sqlx`)** operando em modo WAL (*Write-Ahead Logging*).
* O uso de instâncias externas pesadas (MySQL, Dolt, Postgres) está proibido para garantir a soberania "Local-First".
* Consultas lexicais utilizarão a extensão nativa **FTS5** do SQLite.

#### 4. A EXCEÇÃO DO DOCKER (SIDECARS EFÊMEROS)
* **Atenção à Regra Bare-Metal:** O produto final do SODA jamais usará contêineres.
* **Exceção de Desenvolvimento:** Exclusivamente durante o desenvolvimento neste workspace, **É PERMITIDO** o uso de Docker apenas para orquestrar "Sidecars Efêmeros" via MCP (ex: docling-mcp, browser-use).
* Estes contêineres devem obrigatoriamente rodar com a flag `--rm` para serem sumariamente destruídos após a extração do JSON-RPC, devolvendo a memória à máquina hospedeira.

#### 5. ORQUESTRAÇÃO DE CONTEXTO E GITOPS (SHADOW WORKSPACES)
* **Spec-Driven Development (SDD):** Nunca codifique às cegas. Antes de gerar código, você DEVE estruturar o problema usando a metodologia BMAD (criar proposal.md, design.md e tasks.md).
* **Shadow Workspaces:** Para refatorações críticas ou execuções autônomas, **NÃO aplique commits diretos na branch main**. Trabalhe em ramificações ou pastas temporárias isoladas, gerando um patch (Diff) para a aprovação estrita do Arquiteto (Human-in-the-loop).
* **Execução Stateless:** Trate cada tarefa como isolada. O sucesso não é definido pelo chat, mas por *Exit Code 0* nos testes locais (cargo check / cargo test).

#### 6. CONTROLES DE SEGURANÇA (GATES & HITL)
* Toda invocação de ferramentas (Tool Calling) que altere o ambiente físico (filesystem, repositórios git) ou financeiro deve ser implementada com um mecanismo de suspensão de corrotina em Rust.
* O *daemon* em Rust "congela" a thread e exibe um Card de Aprovação no Canvas (React), aguardando a confirmação explícita humana antes de aplicar a mutação no disco.

#### 7. ARQUITETURA DE MEMÓRIA (NEURO-SINTÉTICA - MNS)
* **Proibição de RAG Simplista:** É proibido tratar a memória como um banco vetorial único. O sistema utiliza a Memória Tri-Partite Especializada embutida no Daemon Rust:
  1. **Transacional (Curto Prazo/Event Sourcing):** SQLite em modo WAL com FTS5.
  2. **Semântica (Longo Prazo/Zero-Copy):** LanceDB (vetores colunares Apache Arrow no SSD).
  3. **Relacional/Episódica:** FalkorDB (Grafos estruturais estáticos em RAM).
* O "Esquecimento" é gerenciado matematicamente por Geometria Diferencial (Métrica Fisher-Rao) processada na CPU, nunca gastando tokens de LLM para "decidir" o que lembrar.