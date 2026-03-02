# 🗺️ DEVELOPMENT PLAN: Genesis - Mission Control

Esta é a ordem cronológica estrita de desenvolvimento. Um agente NUNCA deve iniciar uma fase subsequente sem que a fase atual esteja completada, empiricamente testada (exit code zero) e com commit realizado.

## Fase 1: Fundação e Esqueleto (Scaffolding) [ PENDENTE ]

- [ ] Inicializar o repositório Git.
- [ ] Criar o projeto base `Tauri v2 + Vite + React + TypeScript` (utilizando `cmd /c`).
- [ ] Configurar o **Tailwind CSS v4** e as fundações do design system.
- [ ] Implementar as bibliotecas core: `framer-motion`, `lucide-react` e inicializar o `shadcn-ui`.
- [ ] Estabelecer a ponte IPC ("Ping-Pong" entre Rust e React) provando a comunicação síncrona/assíncrona sem travar a thread.

## Fase 2: O Cérebro (Backend Rust & Memória Dolt) [ BLOQUEADO ]

*Depende do sucesso da Fase 1.*

- [ ] Estruturar o motor `ZeroClaw` base em `src-tauri/src/orchestrator`.
- [ ] Configurar a inicialização da conexão com o banco **Dolt** (porta 3306), implementando o *Graceful Sleep* obrigatório para a *Connection Pool*.
- [ ] Criar os bindings em Rust para disparar comandos do CLI do `Beads` (`cmd /c bd ...`).
- [ ] Implementar os *Human Gates* (mecanismo de suspensão de corrotinas Rust via Tauri Events).

## Fase 3: A Lente (Frontend e Acessibilidade Cognitiva) [ BLOQUEADO ]

*Depende do sucesso da Fase 2.*

- [ ] Desenvolver a infraestrutura espacial de navegação (Sidebar animada, painéis com Framer Motion).
- [ ] Construir o **Kanban Pessoal** lendo dados do Dolt/Beads via Rust IPC (usando `Pragmatic Drag and Drop`).
- [ ] Construir o **Workflow Editor** lógico (usando `React Flow`).
- [ ] Implementar a *Manager Surface* (Chat de orquestração com renderização tardia/lazy rendering do Monaco Editor para código).

## Fase 4: Integração de Habilidades (Skills) [ BLOQUEADO ]

*Depende do sucesso da Fase 3.*

- [ ] Configurar o cliente nativo Model Context Protocol (MCP) no Rust.
- [ ] Instanciar e isolar sub-agentes (ex: `agent-browser` via Docker com `host-gateway`).
- [ ] Validar rotinas autônomas e telemetria local.
