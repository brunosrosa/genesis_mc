# PRD_MILESTONE_01: O Esqueleto Bare-Metal (Fundação SODA)

**Versão:** 3.2 (Definitiva - Tactical Execution)
**Status:** ATIVO E PRONTO PARA EXECUÇÃO
**Alvo da Leitura:** Agentes Codificadores (Antigravity IDE), Engenheiros Rust/Svelte 5.

## 1. OBJETIVO DO MILESTONE 01

Construir a "Planta Baixa" mecânica do Genesis Mission Control (SODA).

Neste _Milestone_, **NÃO HAVERÁ NENHUMA INFERÊNCIA DE IA, LLMS OU MCPs**. O objetivo é estritamente subir o daemon em Rust, configurar a ponte de comunicação (IPC Zero-Copy), instanciar a base SQLite (WAL) e renderizar a Casca Visual (App Shell) passiva no Tauri v2.

- **Critério de Sucesso Master:** O aplicativo deve compilar nativamente via `cargo tauri dev`, o banco de dados `genesis.db` deve ser gerado no disco sem bloqueios, e o Svelte 5 deve receber um "Ping" binário do Rust em menos de 5ms, exibindo a interface base a cravados 60 FPS.

## 2. REGRAS DE GOVERNANÇA (BMAD) PARA ESTE MILESTONE

O Agente encarregado de executar estas tarefas está **PROIBIDO** de pular etapas ou dar commit na `main`.

- **Ação Obrigatória:** Crie uma branch chamada `feature/m1-bare-metal-core`.
- Toda execução de código deve focar em uma única _Task_ por vez.

## 3. TASKS DE EXECUÇÃO SEQUENCIAL

### TASK 1: Scaffolding e Limpeza do Workspace (Rust/Tauri)

- **Ação:** Inicializar o projeto padrão do Tauri v2 utilizando Vite + Svelte 5 + TypeScript.
- **Limpeza:** Excluir todo o código de boilerplate gerado (logos, contadores, CSS padrão).
- **Configuração de UI:** Instalar e configurar `Tailwind CSS v4` e `lucide-svelte` (apenas estes).
- **Restrição:** Certifique-se de que o `package.json` NÃO contenha dependências pesadas de backend Node.js. Toda a infraestrutura deve ser estritamente voltada para compilação estática do Svelte 5.

### TASK 2: O Córtex Motor (Tokio Runtime)

- **Ação:** No diretório `src-tauri`, refatorar o `main.rs` para utilizar o runtime assíncrono `tokio`.
- **Estruturação:** Criar a separação de _threads_. O Tauri deve rodar na thread principal, enquanto o Tokio gerenciará os futuros canais (MPSC) em background.
- **IPC Zero-Copy:** Criar o módulo `src-tauri/src/ipc.rs`. Definir uma função (Tauri Command) básica `system_ping` que retorna uma struct Rust nativa serializada, provando que a ponte está ativa.

### TASK 3: O Hipocampo Transacional (SQLite WAL + FTS5)

- **Ação:** Adicionar as crates `rusqlite` (ou `sqlx`) e `serde` ao `Cargo.toml`.
- **Configuração:** Criar o módulo `src-tauri/src/db.rs`.
- **Execução:** Codificar a rotina de inicialização (Boot) que cria o arquivo `genesis.db` no diretório local de dados da aplicação (`AppLocalData`).
- **PRAGMAS Obrigatórios:** A inicialização DEVE executar os comandos SQL para ativar o modo `WAL` (Write-Ahead Logging), modo `SYNCHRONOUS = NORMAL` e criar a primeira tabela de eventos em `FTS5` (Event Sourcing), mesmo que vazia.

### TASK 4: A Casca Visual (Cyber-Neuro App Shell)

- **Ação:** No diretório `src`, deletar o `App.svelte` padrão e construir o layout estrito baseado no _DESIGN.md_ (Nothing Design + Glassmorphism).
- **Layout Fixo:** Implementar o grid principal usando Flexbox/Grid que contenha:
    - `TopHeader.svelte` (h-12, fixo no topo).
    - `GovernorRail.svelte` (Menu esquerdo, w-16 ou w-64 expansível via `transform: translateX`).
    - `Nexus.svelte` (Área central expandida `flex-1`).
    - `TerminalFooter.svelte` (h-8, fixo no rodapé).
- **Estado:** Configurar o `Runes` do `Svelte 5` (`$state`, `$derived`) para gerenciar a abertura/fechamento do _Governor Rail_ sem causar _Layout Shifts_ (sem mexer na largura da área central).
- **Teste IPC:** O `Nexus.svelte` deve invocar o `system_ping` do Rust (Task 2) ao ser montado (`onMount`) e exibir o resultado latencial na tela em fonte monoespaçada.

## 4. DEFINIÇÃO DE PRONTO (DoD - DEFINITION OF DONE)

O Agente Orquestrador deve verificar estes portões de qualidade antes de pedir autorização para finalizar o Milestone:

1. [ ] Execução do comando `cargo check` não retorna erros de _Borrow Checker_.
2. [ ] Execução do comando `npm run tauri dev` abre a janela nativa limpa, sem erros no console do navegador (DevTools).
3. [ ] O arquivo `genesis.db` foi criado fisicamente e os PRAGMAs de WAL foram confirmados.
4. [ ] O design da tela possui fundo abissal e reflete a paleta _Cyber-Neuro Synthesis_.
5. [ ] O terminal exibe a latência IPC (Rust -> Svelte 5) provando a simbiose.

Se os critérios acima forem atingidos, o Milestone 1 está selado. O sistema está preparado para receber os Agentes LLM e o AgentGateway MCP no Milestone 2.