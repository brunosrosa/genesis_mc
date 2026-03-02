# 📋 PRODUCT VISION: Genesis - Mission Control

## O Que É

O **Genesis - Mission Control** é um Sistema Operacional Agêntico Soberano (Life OS e Orquestrador de IA). Ele atua como um *daemon* contínuo local e um painel de controle de interface gráfica Desktop, projetado para coordenar sub-agentes de IA na execução de tarefas cotidianas e desenvolvimento de software.

## O "Porquê" (Filosofia Central)

1. **Soberania e Privacidade:** Os dados do usuário (memória, tokens, logs) nunca saem da máquina local, exceto via APIs de LLM estritamente autorizadas.
2. **Acessibilidade Cognitiva:** Eliminar o "Log Fatigue". A interface substitui terminais verbosos por abstrações espaciais (Kanbans visuais, Árvores de Dependência, Chat Generativo) para acomodar perfis neurodivergentes de alto desempenho.
3. **Pessimismo da Razão, Otimismo da Vontade:** O sistema assume que a IA falha. Por isso, utiliza salvaguardas mecânicas (Human-in-the-loop, Ralph Loops, Memória Persistente) para blindar a execução.

## Arquitetura em Alto Nível

- **O Motor (Daemon):** Desenvolvido em **Rust** (arquitetura ZeroClaw). Responsável pelo roteamento de LLMs (APIs vs Locais), integração MCP (Model Context Protocol) e gestão de estado.
- **A Lente (Frontend):** Desenvolvida em **React/Vite via Tauri v2**. Atua exclusivamente como uma vitrine de renderização gráfica "burra" que reage passivamente aos Eventos IPC do Rust.
- **Memória de Dois Cérebros:** Separação estrita entre Memória Episódica/Processual (Kanbans via `Beads` paradigm) e Memória Semântica (Vector DB / RAG).
