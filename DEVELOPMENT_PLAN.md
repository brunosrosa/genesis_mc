# 🗺️ DEVELOPMENT PLAN: Genesis - Mission Control

Este documento orienta a ordem cronológica estrita de desenvolvimento. Um agente NUNCA deve iniciar uma fase subsequente sem que a fase atual esteja completada, testada e com commit realizado.

## Fase 1: Fundação e Esqueleto (Scaffolding) [ ATUAL ]

- [ ] Inicializar repositório Git.
- [ ] Criar o projeto base `Tauri v2 + Vite + React + TypeScript`.
- [ ] Configurar o Tailwind CSS (v3/v4).
- [ ] Implementar as bibliotecas base: `framer-motion`, inicialização do `shadcn/ui`.
- [ ] Estabelecer a ponte IPC básica ("Ping-Pong" entre Rust e React) para provar a comunicação síncrona e assíncrona.

## Fase 2: O Cérebro (Backend Rust & Memória Beads)

- [ ] Estruturar o motor `ZeroClaw` base em `src-tauri`.
- [ ] Configurar a arquitetura **Tauri Sidecar** para embarcar o executável oficial do `steveyegge/beads`.
- [ ] Criar os bindings em Rust (módulo `src/memory/`) para consultar o banco SQLite do Beads em modo leitura e disparar comandos CLI para inserção de dados.
- [ ] Implementar os *Gates de Segurança* (mecanismo de suspensão de corrotinas).
- [ ] Criar o cliente Model Context Protocol (MCP) nativo em Rust.

## Fase 3: A Lente (Frontend UI)

- [ ] Desenvolver a infraestrutura espacial de navegação (Sidebar, painéis expansíveis).
- [ ] Construir o **Kanban Pessoal** (lendo dados do Beads via Rust IPC) utilizando `Pragmatic Drag and Drop`.
- [ ] Construir o **Workflow Editor** utilizando `React Flow`.
- [ ] Implementar o Chat Multimodal (Generative UI).

## Fase 4: Integração de Habilidades (Skills) e Polimento

- [ ] Instanciar o `agent-browser` (Headless browser via MCP) para automação e testes visuais.
- [ ] Otimização de consumo de memória (lazy rendering do Monaco Editor).
- [ ] Implementação de telemetria e observabilidade local (Heartbeats).
