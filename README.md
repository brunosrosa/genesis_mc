# 👻 Souls MC (Mission Control) (_SODA_)

> **Axioma:** *"O silício é o nosso limite; a soberania é o nosso dogma; o silêncio é a nossa estética."*

**Sovereign Operating Data Architecture (SODA)** não é um "wrapper" de IA ou um chatbot glorificado. É um **Sistema Operacional Agêntico Local** — um **Exoesqueleto Cognitivo** e **Prótese de Função Executiva** — construído do zero no *Metal Nu* (Bare-Metal). Ele orquestra inteligência autônoma diretamente no hardware do usuário, garantindo simbiose humana, privacidade criptográfica, eficiência termodinâmica e mitigação de *Flow-Debt* (Dívida de Fluxo) em mentes neurodivergentes (2e/TDAH).

**Status Atual:** **ERA V6 — Souls MC.** Backend Rust/Tokio maduro (Harvester F0–F5, SAST pipeline, MCP gateway `soda-agent-gateway`, sandboxing Windows AppContainer + Wasmtime). Frontend Svelte 5 em transição. Constituição ativa: [`docs/SODA_CANON_MANIFEST.md`](docs/SODA_CANON_MANIFEST.md).

---

## 🧠 Perfil e Hardware Alvo (O *Treino de Gravidade*)

A arquitetura foi forjada com restrições matematicamente estritas. Toda tecnologia candidata deve sobreviver a este piso de validação antes de entrar no ecossistema.

- **Hardware Alvo:** Intel i9, 32 GB RAM, GPU NVIDIA **RTX 2060m** (teto rígido de **6 GB de VRAM**).
- **Perfil Cognitivo (UX):** Otimizado para **2e / TDAH** (Dupla Excepcionalidade). A interface atua como *Sparring Partner* e *Life Coach* proativo, blindada contra sobrecarga sensorial via *Modo Zen* e renderização passiva (Zero Layout Shifts).
- **Agnosticismo Matemático:** A "alma matemática" do sistema é **transmutável** — projetada para rodar tanto em CPU (AVX2) quanto em GPU (Burn / CubeCL / Candle), sem engessamento de fornecedor.

---

## 🏗️ Dogmas Arquiteturais (A Stack Imutável)

O Souls MC repudia a execução de interpretadores pesados em *background* (Node.js, Electron, Python, JVM) para preservar a VRAM e a CPU estritamente para a inferência local de IA. Cada regra abaixo é uma **Lei da Física**, não uma convenção.

1. **Backend (O Cérebro) — Rust + Tokio (Assíncrono).** Gerencia todo o I/O, persistência local, orquestração de Agentes, gateway MCP, harvester de repositórios, distillers e SAST pipeline.
2. **Comunicação Zero-Garbage — IPC Zero-Copy.** Toda massa de dados entre processos e UI trafega via **FlatBuffers / Apache Arrow / rkyv**. Serialização JSON pesada é proibida em hot paths.
3. **Sandboxing Tripartite.** Ferramentas de terceiros e scripts gerados por IA rodam como **Sidecars Efêmeros** — enjaulados via *Wasmtime* (lógica pura) ou *Landlock / AppContainer* (host) e destruídos com **SIGKILL atômico** após o uso. Asfixia de RAM é vetada por design.
4. **Frontend (O Terminal Burro) — Svelte 5 (Runes) + Tauri v2 + Tailwind v4.** UI estritamente passiva (Zero-VDOM). Frameworks baseados em Virtual DOM (React, Vue) estão **banidos** — a "Morte do Virtual DOM" é lei.
5. **Tiling Window Manager 2D & Zero Layout Shift.** Janelas flutuantes caóticas, "Liquid Glass" (blur) e modais que causam refusão de layout são proibidos. Adotamos planaridade ortogonal e **GenUI Efêmera** (injeção de interface sem quebrar o espaço cognitivo).
6. **Memória Evolutiva (A Tríade).** L2 Episódico (SQLite WAL MVCC + FTS5) + L3 Semântico (LanceDB) + Grafos Ontológicos (LadybugDB / Kùzu). Toda mutação passa por **Event Sourcing** via *Gitoxide* (Rust) — o usuário nunca perde uma versão.
7. **LLM Wiki (vs RAG).** Repúdio ao RAG de busca cega. A cognição opera sob o paradigma *LLM Wiki*, com **Logit Probing** direto em SLMs locais (Mistral.rs / llama-cpp-4 via AVX2) — zero gasto de VRAM gerando texto para julgar a realidade. Conflitos com crenças raiz disparam auditoria via *Cohomologia de Feixes* e distância *Fisher-Rao*.

---

## ⚙️ Instalação e Quickstart (Ambiente Local)

> **Aviso de Ambiente — Fábrica vs Produto.** `pnpm` e o ecossistema Node são **DEV-ONLY** (Shadow Workspace / fábrica de iteração). O produto final entregue é **binário estático 100% Rust**, sem dependências interpretadas em produção. Nunca embarcar runtime interpretado no release.

### Pré-requisitos
- [Rust Toolchain](https://rustup.rs/) (`cargo`, `rustc`) — **obrigatório**.
- [Node.js](https://nodejs.org/) (v20+ LTS) + **pnpm** — **somente para o shell do Tauri em modo dev**.
- C++ Build Tools (MSVC no Windows) para crates com bindings C/C++ nativos (`crc32fast`, etc).

### Setup
1. Clone o repositório e instale as dependências de interface:
   ```bash
   pnpm install
   ```
2. Inicie o ambiente de desenvolvimento (Tauri Dev — HMR do shell Svelte + build do backend Rust simultâneo):
   ```bash
   pnpm tauri dev
   ```
3. Build de produção — binário estático, sem dependências externas:
   ```bash
   cd src-tauri && cargo build --release
   ```

---

## 📂 Topologia de Diretórios (Onde vive a inteligência)

A estruturação é regida pelo **Spec-Driven Development (SDD)** e divisão estrita de responsabilidades.

- **`/src-tauri/`** — O **Coração Rust**. Toda a regra de negócio, IPC Zero-Copy, Harvester (F0–F5), SAST pipeline (semgrep / opengrep), MCP gateway (`souls_mcp_server`, `soda-agent-gateway`), persistência SQLite e gestão de subprocessos vive aqui.
- **`/src/`** — O **Shell Svelte 5** (em transição do legado). Interface *Cyber-Purple*, Tiling 2D, GenUI Efêmera e ecossistema passivo do Canvas.
- **`.agents/`** — O **Córtex de Contexto** dos Agentes.
  - `rules/`: Leis de governança imutáveis e sintaxe (`project_rules.md`).
  - `skills/`: Ecossistema de habilidades em Markdown (`SKILL.md`) sob o princípio de **Divulgação Progressiva** (Progressive Disclosure).
- **`/docs/`** — Memória Semântica de longo prazo.
  - `SODA_CANON_MANIFEST.md` — **A Constituição**. Toda decisão arquitetural deve referenciá-la.
  - `adrs/` — *Architecture Decision Records* (ADRs).
  - `specs/` — Especificações SDD.
  - `prds/` — Product Requirements Documents.
  - `state/`, `audits/`, `context_dumps/` — Snapshots frozen da evolução do projeto (preservados como cronologia, **não devem ser editados**).

---

## 🛡️ Segurança (Zero-Trust, HITL e Rebase Semântico)

O Souls MC implementa **três camadas de defesa complementares**:

- **Zero-Trust & HITL.** Nenhum agente autônomo possui permissão de escrita livre no disco principal. Alterações destrutivas operam em **Shadow Workspaces** isolados e dependem de aprovação **Human-In-The-Loop** via Agent Inbox antes de qualquer mutação real.
- **Agent Inbox (Rebase Semântico).** Operações que tocam código, memória ou superfície de produção são capturadas como *patches semânticos* e jogadas numa gaveta de aprovação humana. **Event Sourcing** via *Gitoxide* preserva todas as versões concorrentes — o sistema fala apenas quando tem algo determinístico a dizer.
- **Ghost Telemetry.** A máquina se comunica silenciosamente no rodapé, **banindo spinners** geradores de ansiedade. Telemetria predatória é repudiada.
- **Sandbox Efêmero + SIGKILL.** Wasmtime + Landlock / AppContainer protegem contra RCE em sidecars. Toda execução de terceiros é finita e determinística.

---

## 🐝 Mandato do Enxame (As Três Lentes)

Quando agentes são invocados para analisar, auditar ou canibalizar tecnologias externas, **três perspectivas antagônicas** devem dialogar antes de qualquer síntese. A conclusão resulta em **Ouro a Extrair** (o que assimilar) e **Lixo a Expurgar** (o que amputar cirurgicamente).

- **Lente A — Sentido (Produto / UX — Otimismo da Vontade).** A inovação mitiga o *Flow-Debt*? Oferece um momento "UAU" neuro-inclusivo? Qual heurística de interface deve ser extraída para a GenUI sem poluir o silício?
- **Lente B — Estrutura (Arquiteto Bare-Metal — Física da Máquina).** A lógica é extraível em O(1)? A "alma matemática" é independente de linguagens tóxicas e recompilável nativamente em Rust? Sobrevive ao Treino de Gravidade da RTX 2060m?
- **Lente C — Realidade (Auditor Operacional — Pessimismo da Razão).** Como a solução envelhece? Impõe DevOps contínuo, telemetria predatória ou dependência letal de APIs pagas (FinOps)? O que deve ser isolado ou expurgado?

---

## 📜 Constituição & Contribuição

Este projeto opera sob **Spec-Driven Development** e **TDD atômico (Red-Green-Refactor)**. Toda nova lógica:

1. Debate arquitetura com o Arquiteto Humano via `/grill-me` antes de codificar.
2. É precedida por `docs/design.md` (com diagrama Mermaid **Orchestrator-Worker** e prova de Agnosticismo Hardware) + `docs/tasks.md` (com DoD atômico por tarefa).
3. Escreve o teste vazio de falha (Red) antes da implementação (Green).
4. Passa pelo **Ralph Loop** (teto de 3 tentativas, *Fail-Closed*) se o compilador quebrar.
5. Aguarda aprovação HITL antes de qualquer rebase semântico em direção à `main`.

A Constituição completa vive em [`docs/SODA_CANON_MANIFEST.md`](docs/SODA_CANON_MANIFEST.md). Em caso de divergência entre este README e o Canôn, **o Canôn prevalece**.

---

*“Pessimismo da razão, otimismo da vontade.”*
