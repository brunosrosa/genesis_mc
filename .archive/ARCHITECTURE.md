# 🏗️ ARCHITECTURE: Genesis - Mission Control

Este documento define a topologia estrita do sistema. O Genesis opera sob o paradigma de um **Daemon Rust Soberano** com uma **Lente de Apresentação React**, unidos por Tauri IPC (Inter-Process Communication).

## 1. TOPOLOGIA DE DIRETÓRIOS E SEPARAÇÃO DE RESPONSABILIDADES

A base de código está dividida de forma estanque em dois mundos que não partilham dependências diretas:

### 🦀 O Cérebro (Backend / Rust) -> `/src-tauri`

Toda a inferência, acesso a arquivos, orquestração e memória residem aqui.

- `src/main.rs`: O ponto de entrada do Daemon Tauri.
- `src/orchestrator/`: O motor ZeroClaw. Gerencia o ciclo de vida dos sub-agentes, roteamento de prompts e orquestração de chamadas (via `llm-chain` ou `rig-rs`).
- `src/memory/`: O controlador do ecossistema Beads. Conecta-se ao banco de dados **Dolt** (protocolo MySQL na porta 3306) utilizando um *Graceful Sleep* na inicialização da *Connection Pool*. Delega escritas assíncronas para o CLI do Beads (garantindo versionamento Git seguro).
- `src/mcp_client/`: A infraestrutura que conecta o Rust a servidores Model Context Protocol (MCP) externos ou em Docker.
- `src/proxy/`: O roteador de chamadas de LLM (API cloud vs Modelos Locais).

### ⚛️ A Lente (Frontend / React) -> `/src`

Estritamente responsável pela renderização visual, gestão de estado espacial e captação de intenções do usuário.

- `src/components/ui/`: A base atômica do design system (Tailwind v4, Shadcn UI, Framer Motion).
- `src/features/`: Componentes complexos e isolados (Canvas do React Flow, Kanban do Pragmatic DnD).
- `src/lib/ipc/`: A ÚNICA ponte de comunicação permitida. Contém funções que disparam comandos síncronos (`invoke`) e escutam Eventos assíncronos (`listen`) do Tauri.

## 2. FLUXO DE DADOS E COMUNICAÇÃO (IPC)

A regra de ouro: **O React não processa lógica de negócio nem retém estado denso.**

1. **Comandos (Síncronos):** Ações diretas e rápidas (ex: `invoke('ping_daemon')`).
2. **Eventos (Assíncronos):** Dados volumosos (varreduras de disco, logs de IA). O Rust emite eventos contínuos e o React reage espacialmente.

## 3. O PADRÃO "DOIS CÉREBROS" (MEMÓRIA)

- **Estado Estrutural e Processual:** Gestão de Kanban, issues e dependências de tarefas operando no ecossistema **Beads/Dolt**.
- **Conhecimento Semântico:** Vector Embeddings e RAG de documentações.
