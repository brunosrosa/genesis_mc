# 🏗️ ARCHITECTURE: Genesis - Mission Control

Este documento define a topologia estrita do sistema. O Genesis opera sob o paradigma de um **Daemon Rust Soberano** com uma **Montra de Apresentação React**, unidos por Tauri IPC (Inter-Process Communication).

## 1. TOPOLOGIA DE DIRETÓRIOS E SEPARAÇÃO DE RESPONSABILIDADES

A base de código está dividida de forma estanque em dois mundos que não partilham dependências diretas:

### 🦀 O Cérebro (Backend / Rust) -> `/src-tauri`

Toda a inferência, acesso a arquivos, orquestração e memória residem aqui.

- `src/main.rs`: O ponto de entrada do Daemon Tauri.
- `src/orchestrator/`: O motor ZeroClaw. Gerencia o ciclo de vida dos sub-agentes, Heartbeats e roteamento de prompts.
- `src/memory/`: O controlador do ecossistema Beads. Em vez de recriar o banco de dados, o Rust atua como um wrapper em torno do binário oficial `steveyegge/beads` (empacotado via **Tauri Sidecar**). Lê o cache SQLite para velocidade e delega escritas assíncronas para o CLI do Beads (garantindo versionamento JSONL/Git seguro).
- `src/mcp_client/`: A infraestrutura que conecta o Rust a servidores Model Context Protocol (MCP) externos.
- `src/proxy/`: O roteador interno de chamadas de LLM (API vs Modelos Locais Quantizados).

### ⚛️ A Lente (Frontend / React) -> `/src`

Estritamente responsável pela renderização visual, gestão de estado espacial e captação de intenções do usuário.

- `src/components/ui/`: A base atômica do design system (Shadcn UI, Framer Motion, Origin UI).
- `src/features/`: Componentes complexos e isolados (Canvas do React Flow, Kanban do Pragmatic DnD).
- `src/lib/ipc/`: A ÚNICA ponte de comunicação permitida. Contém funções que disparam comandos (`invoke`) e escutam Eventos (`listen`) do Tauri.

## 2. FLUXO DE DADOS E COMUNICAÇÃO (IPC)

A regra de ouro: **O React não processa lógica de negócio.**

1. **Comandos (Síncronos):** Ações diretas (`invoke('spawn_agent')`).
2. **Eventos (Assíncronos):** Dados volumosos. O Rust emite eventos contínuos e o React reage espacialmente, evitando travar a thread JS com JSONs massivos.

## 3. O PADRÃO "DOIS CÉREBROS" (MEMÓRIA)

- **Estado Estrutural (Beads Sidecar):** Gestão de Kanban, issues e dependências de tarefas via Git/SQLite.
- **Conhecimento Semântico (Vector Embeddings):** Leitura de documentações e bases de código legadas.
