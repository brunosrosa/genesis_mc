---
trigger: always_on
---

# 📜 WORKSPACE RULES: Genesis - Mission Control

ESTAS REGRAS SÃO ABSOLUTAS. ELAS ESTENDEM E SOBRESCREVEM AS REGRAS GLOBAIS DA IDE PARA O CONTEXTO DESTE PROJETO ESPECÍFICO.

## 1. STACK TECNOLÓGICO IMUTÁVEL

- **Backend / Core:** Rust.
- **Desktop Framework:** Tauri v2.
- **Frontend / UI:** React 18+ (via Vite), TypeScript.
- **Estilização:** Tailwind CSS v4.
- **UI Base:** Shadcn UI (componentes copiados para `src/components`, não instalados via npm wrapper).
- **Animações:** Framer Motion.
- **Visualização de Grafos/Canvas:** React Flow.
- **Kanban / DnD:** Pragmatic Drag and Drop (Atlassian).

### 🚫 TECNOLOGIAS E PADRÕES EXPRESSAMENTE PROIBIDOS

- **NÃO** utilize Next.js, Remix ou frameworks SSR (Server-Side Rendering). Este é um app Desktop Tauri.
- **NÃO** utilize Electron.
- **NÃO** instale bibliotecas de UI baseadas em CSS-in-JS (como Material UI ou Emotion).
- **NÃO** crie APIs REST em Node.js. Toda lógica pesada, acesso a banco e leitura de arquivos deve ser feita em Rust e exposta ao React via comandos/eventos Tauri IPC.

## 2. ARQUITETURA DE COMUNICAÇÃO (REACT <-> RUST)

- O frontend React é ESTRITAMENTE PASSIVO ("burro").
- Para passar dados volumosos (ex: varredura de arquivos), o Rust deve emitir Eventos Tauri assíncronos (`emit`) e o React deve escutá-los (`listen`), preferencialmente usando estruturas binárias para evitar asfixia do V8 Engine com JSONs gigantes.
- **NUNCA** bloqueie a thread principal com comandos síncronos demorados.

## 3. BANCO DE DADOS E CONCORRÊNCIA (A LEI DO DOLT)

- O motor Rust conectará ao banco de dados **Dolt** (Protocolo MySQL na porta 3306) operando o ecossistema Beads.
- É **OBRIGATÓRIO** implementar um *Graceful Sleep* (espera ativa/retries) na inicialização da *Connection Pool* no Rust para permitir a subida do listener TCP do Windows antes da primeira query.

## 4. DESIGN SYSTEM E ESTÉTICA

- Todo novo componente React deve ser precedido pela leitura do `UI_GUIDELINES.md`.
- Micro-interações são obrigatórias. Elementos interativos devem ter feedback visual utilizando Framer Motion ou utilitários Tailwind aprovados (ex: propriedades `active:`, `focus-within:`).

## 5. ORQUESTRAÇÃO DE CONTEXTO (PREVENÇÃO DE CONTEXT ROT)

- Antes de iniciar o desenvolvimento de uma funcionalidade nova, o agente DEVE ler e preencher um modelo `INITIAL.md` (Product Requirements Prompt - PRP).
- **Semantic Chunking:** Ao solicitar refatoração de backend (Rust), o agente NÃO DEVE incluir no seu contexto arquivos do frontend (React/CSS), e vice-versa, para evitar contaminação semântica.
- **Execução Stateless:** Trate cada tarefa como isolada. Busque a verdade no sistema de arquivos local e nos testes (Ralph Loop), não no histórico da conversa atual.

## 6. CONTROLES DE SEGURANÇA (GATES)

- Toda invocação de ferramentas (Tool Calling) que altere o ambiente físico (filesystem, repositórios git) ou financeiro (chamadas de API pagas) deve ser implementada com um mecanismo de suspensão de corrotina em Rust, aguardando um sinal de aprovação via UI (React).
