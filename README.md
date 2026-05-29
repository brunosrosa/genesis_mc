# 🛰️ Genesis Mission Control (SODA)

**Sovereign Operating Data Architecture (SODA)** não é um "wrapper" de IA ou um chatbot glorificado. É um **Sistema Operacional Agêntico Local** construído do zero no "Metal Nu" (Bare-Metal). Ele atua como um exoesqueleto cognitivo invisível e subjacente, desenhado para orquestrar inteligência autônoma diretamente no hardware do usuário, garantindo simbiose humana, privacidade criptográfica e eficiência termodinâmica.

**Status Atual:** MILESTONE 1 - Fundação Bare-Metal & Tauri IPC.

---

## 🧠 Perfil e Hardware Alvo
Esta arquitetura foi forjada com restrições e objetivos matematicamente estritos:
- **Hardware Alvo:** Intel i9, 32GB RAM, GPU NVIDIA RTX 2060m (Teto rígido de 6GB VRAM).
- **Perfil Cognitivo (UX):** Otimizado para **2e / TDAH** (Dupla Excepcionalidade). A interface atua como um *Sparring Partner* e *Life Coach* proativo, blindada contra sobrecarga sensorial através do "Modo Zen" e de renderização passiva (Zero Layout Shifts).

---

## 🏗️ Dogmas Arquiteturais (A Stack Imutável)

O Genesis MC repudia a execução de interpretadores pesados em *background* (como daemons contínuos em Node.js ou Python) para preservar a VRAM e a CPU estritamente para a inferência local de IA.

1. **Backend (O Cérebro):** Rust + Tokio (Assíncrono). Gerencia todo o I/O, persistência local e orquestração de Agentes.
2. **Desktop Bridge:** Tauri v2. Garante um binário leve, seguro e com comunicação IPC (Inter-Process Communication) orientada a buffers binários de Zero-Copy.
3. **Frontend (O Terminal Burro):** React 19 + TypeScript + Tailwind CSS v4 + Xyflow/Tldraw. Uma UI estritamente passiva (Canvas-First) que apenas renderiza os estados processados pelo Rust.
4. **Memória L2 (O Hipocampo):** SQLite operando em modo WAL com a extensão FTS5, garantindo histórico episódico ultrarrápido sem servidores externos pesados.

---

## ⚙️ Instalação e Quickstart (Ambiente Local)

### Pré-requisitos
- [Rust Toolchain](https://rustup.rs/) (cargo, rustc).
- [Node.js](https://nodejs.org/) (v20+ LTS) e **pnpm** instalado globalmente.
- C++ Build Tools (MSVC no Windows) para a compilação das bibliotecas C/C++ nativas.

### Setup do Workspace
1. **Clone o repositório e instale as dependências de interface:**
   ```bash
   pnpm install
   ```

2. **Inicie o ambiente de desenvolvimento (Tauri Dev):**
   ```bash
   pnpm tauri dev
   ```
   *O Tauri fará o build do backend em Rust e iniciará o servidor HMR do Vite para o React simultaneamente.*

---

## 📂 Topologia de Diretórios (Onde vive a inteligência)

A estruturação do projeto é regida pela metodologia **Spec-Driven Development (SDD)** e divisão estrita de responsabilidades:

- `/src-tauri/` -> O santuário do **Rust**. Toda a regra de negócio, invocação de processos Wasmtime e gerenciamento de banco de dados vive aqui.
- `/src/` -> O **React Frontend**. Contém a interface *Cyber-Purple*, componentes Shadcn UI e o ecossistema passivo do Canvas.
- `.agents/` -> O **Córtex de Contexto**. 
  - `rules/`: Leis de governança imutáveis e de sintaxe (Cursorrules).
  - `skills/`: O ecossistema de habilidades em markdown (`SKILL.md`) usadas pelos agentes sob o princípio de **Divulgação Progressiva** (Progressive Disclosure).
- `/docs/` -> Memória Semântica de longo prazo (Architecture Decision Records - ADRs e Especificações SDD).

---

## 🛡️ Segurança (Zero-Trust & HITL)
Nenhum agente autônomo rodando no Genesis MC possui permissão de escrita livre no disco principal. Alterações destrutivas ou invocações de terminais operam sob o conceito de **Shadow Workspaces** e dependem de aprovação **Human-In-The-Loop (HITL)** via interface gráfica antes da mutação real.

---
*“Pessimismo da razão, otimismo da vontade.”*