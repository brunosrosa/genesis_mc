# MANUAL CANÔNICO: SODA (Sovereign Operating Data Architecture)
**Versão:** Genesis MC - Protocolo de Integridade Arquitetural
**Status:** Consolidado

---

### 1. O PORQUÊ TÉCNICO: A SOBERANIA DO DADO
A arquitetura SODA não é um "framework de conveniência"; é uma camada de execução soberana. O erro fundamental das ferramentas analisadas (Trellis, Vibe-Kanban) reside na **dependência de runtimes interpretados (Node.js/Python)** e na **fragmentação de estado**.

O SODA rejeita a "orquestração via scripts" (Trellis) e a "gestão via web-server local" (Vibe-Kanban). Nossa missão é a **imutabilidade do estado de execução**. Enquanto o mercado tenta "colar" agentes com `npm install`, o SODA consolida a lógica em um binário Rust estático, eliminando o overhead de I/O entre processos e garantindo que o contexto do agente seja um mapeamento de memória, não um arquivo JSON lido por um daemon.

---

### 2. DIRETRIZES DE ARQUITETURA PURA (HARDWARE-AWARE)

*   **Backend (O Cérebro):** Exclusivamente **Rust (Tokio)**. A comunicação com o frontend é via **IPC Zero-Copy**. Proibido qualquer uso de WebSockets para comunicação interna; o estado deve ser compartilhado via memória mapeada (mmap) ou canais de alta performance.
*   **Frontend (A Interface Passiva):** **Svelte 5 + Tauri v2**. O frontend é um renderizador de estado, não um processador. Toda lógica de negócio, validação de specs e orquestração de agentes reside no binário Rust.
*   **Otimização Bare-Metal:**
    *   **Execução:** Otimização de instruções **AVX2** obrigatória para o escalonamento de inferência local.
    *   **Memória:** Uso de `llama.cpp` com `mmap` para carregar pesos de modelos diretamente na VRAM da RTX 2060m, evitando o gargalo do barramento PCIe.
    *   **Gargalos:** O barramento da 2060m é o limitador crítico. O SODA deve implementar *quantização dinâmica* baseada na carga de barramento detectada em tempo real.

---

### 3. AUDITORIA CRÍTICA: FALHAS E VETORES DE FRAGILIDADE

Ao analisar as fontes (Trellis e Vibe-Kanban), identificamos "buracos" que o SODA deve evitar:

1.  **A Fragilidade do "Node.js Daemon":** O Vibe-Kanban exige `pnpm run dev` e um backend em Node.js. Isso introduz um ponto de falha (o runtime) e latência de GC (Garbage Collection). **SODA ignora esta abordagem.** O SODA compila para um binário único.
2.  **A Ilusão da "Configuração via Arquivo":** O Trellis propõe `.trellis/spec/`. Isso é volátil. O SODA deve tratar essas especificações como **Schema de Dados Binário (FlatBuffers)**, garantindo que o agente consuma o contexto sem *parsing* de texto, reduzindo o consumo de tokens e aumentando a velocidade de leitura.
3.  **Conflito de Worktrees:** O uso de `git worktrees` para agentes paralelos (Trellis) é uma solução de "força bruta" que causa inconsistência no estado do sistema de arquivos. O SODA deve implementar **Virtual File System (VFS)** em Rust, permitindo que múltiplos agentes operem sobre o mesmo repositório sem a necessidade de múltiplos diretórios físicos.

---

### 4. DIRETIVAS DE EXECUÇÃO (PROIBIÇÕES)

*   **NÃO UTILIZAR:** React, Next.js, Electron, VDOM, ou qualquer framework que dependa de *reconciliation* no cliente.
*   **NÃO UTILIZAR:** Daemons em Node.js para gerenciar estados de agentes.
*   **ELIMINAR:** Qualquer dependência que exija `npm install` no ambiente de produção do Genesis MC.

---

### 5. SÍNTESE DO PROTOCOLO GENESIS MC
O SODA não "gerencia" agentes; ele **hospeda a inteligência**. 
*   **Entrada:** O contexto é injetado via `mmap` diretamente no espaço de endereçamento do modelo.
*   **Processamento:** Rust gerencia o ciclo de vida do agente com latência sub-milissegundo.
*   **Saída:** O Svelte 5 reflete o estado do sistema via *Runes*, garantindo que a interface seja apenas um espelho da realidade do backend.

**Curador Arquitetural:** *A soberania reside na ausência de camadas desnecessárias. Se o código não é Rust, ele é ruído.*