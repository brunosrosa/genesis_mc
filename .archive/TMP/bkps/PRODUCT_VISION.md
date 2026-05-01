# 📋 PRODUCT VISION: Genesis - Mission Control

## O QUE É O GENESIS?

O **Genesis - Mission Control** é um Sistema Operacional Agêntico Soberano (*Life OS* e Orquestrador de IA). Ele atua simultaneamente como um *daemon* contínuo operando em background (Backend) e um painel de controle espacial Desktop (Frontend), projetado para coordenar sub-agentes de IA na execução de tarefas cotidianas, análise de dados e desenvolvimento de software.

## O "PORQUÊ" (FILOSOFIA CENTRAL)

As três pilastras inegociáveis do produto:

1. **Soberania e Privacidade:** Os dados do usuário (memória, tokens, logs, arquivos) NUNCA saem da máquina local, exceto via APIs de LLM estritamente autorizadas no momento da execução. O cérebro é local.
2. **Acessibilidade Cognitiva (A "Manager Surface"):** Eliminar o *Log Fatigue*. A interface rejeita terminais verbosos que cospem texto infinito. Em vez disso, traduz estados de máquina em **abstrações espaciais fluidas** (Kanbans visuais, Árvores de Dependência, Gráficos). A interface foi desenhada para mentes de alto desempenho que exigem organização visual imediata.
3. **Pessimismo da Razão, Otimismo da Vontade:** O sistema opera sob a premissa de que a *IA invariavelmente vai falhar ou alucinar*. Por isso, utiliza salvaguardas mecânicas implacáveis (Human-in-the-loop, Ralph Loops em TDD, Memória Persistente) para blindar a execução e evitar catástrofes físicas ou financeiras.

## ARQUITETURA EM ALTO NÍVEL

- **O Motor (Daemon Soberano):** Desenvolvido em **Rust** (operando o núcleo *ZeroClaw*). Responsável pelo roteamento de LLMs, integração MCP (Model Context Protocol) e execução pesada em Windows Nativo.
- **A Lente (Frontend Passivo):** Desenvolvida em **React 18 / Vite via Tauri v2**. Atua EXCLUSIVAMENTE como uma vitrine gráfica "burra" que reage passivamente aos Eventos IPC do Rust.
- **Memória Bitemporal (GraphRAG / Dolt):** Separação estrita entre Memória Processual (Kanbans e tarefas orquestradas via ecossistema `Beads` sobre o banco de dados `Dolt` no protocolo MySQL) e Memória Semântica (Vector Embeddings de documentações).
