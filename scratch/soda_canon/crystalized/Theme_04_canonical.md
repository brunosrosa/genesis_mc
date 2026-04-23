# Manual Canônico: Arquitetura SODA (Genesis MC)

Este documento estabelece as diretrizes fundamentais para o desenvolvimento e manutenção da **Sovereign Operating Data Architecture (SODA)**. Como Curador Arquitetural, reitero que a conformidade com estes padrões é a única via para garantir a soberania, segurança e performance do projeto Genesis MC.

---

## 1. O Dogma Arquitetural (Stack Tecnológica)
A SODA não é um framework de conveniência; é uma arquitetura de alta performance e soberania.
*   **Backend (O Cérebro):** Estritamente **Rust** utilizando o runtime **Tokio**. Toda a lógica de orquestração, segurança e execução de ferramentas reside aqui.
*   **Frontend (A Interface Passiva):** **Svelte 5 + Tauri v2**. O frontend é estritamente uma camada de visualização. **Proibido:** Qualquer lógica de negócio no frontend.
*   **Comunicação:** **IPC Zero-Copy**. A comunicação entre o backend Rust e o frontend Tauri deve evitar serialização/deserialização desnecessária. O estado é gerenciado no Rust e refletido no Svelte via *bindings* reativos.

> **⚠️ PODA TÓXICA (Eliminação Sumária):** Estão banidos do ecossistema SODA: React, Node.js daemons, Electron, VDOM e qualquer forma de Server-Side Rendering (Next.js). Estas tecnologias introduzem overhead inaceitável e vetores de ataque desnecessários.

---

## 2. Hardware-Aware & Bare-Metal
A SODA é projetada para rodar em hardware soberano, desde servidores bare-metal até dispositivos de borda.
*   **Otimização de CPU:** O uso de diretivas **AVX2** é obrigatório para qualquer processamento de inferência local. O compilador deve ser configurado para `target-cpu=native` para maximizar o throughput.
*   **GPU/VRAM:** Para inferência local (ex: RTX 2060m), o uso de **llama.cpp com mmap** é o padrão ouro para evitar o estouro de VRAM e garantir que o modelo permaneça residente na memória de forma eficiente.
*   **Gargalos:** O barramento PCIe é o gargalo crítico. A arquitetura de dados deve minimizar a transferência de tensores entre CPU e GPU.

---

## 3. Auditoria Crítica: Furos e Fragilidades
A análise do ecossistema "OpenClaw" (fonte primária do aglomerado) revela falhas estruturais que a SODA deve evitar:
1.  **O "God-Mode" Terminal:** A prática de permitir que agentes executem comandos shell sem sandboxing rigoroso é uma vulnerabilidade crítica. A SODA implementa **isolamento via namespaces/cgroups** (Linux) ou containers efêmeros para toda execução de código.
2.  **Vazamento de Segredos:** A prática comum de logar chaves de API em texto plano é inaceitável. A SODA utiliza um **Vault de Memória Criptografada** (em memória, nunca persistido em disco sem encriptação AES-256).
3.  **Fragilidade de UI:** Agentes que dependem de "screenshots" para automação de browser são inerentemente frágeis. A SODA utiliza a **Accessibility Tree (CDP)** para interação semântica, reduzindo o consumo de tokens e aumentando a resiliência contra mudanças de layout.

---

## 4. O "Porquê" Técnico (Consolidação)
Por que a SODA exige Rust e uma interface passiva?
*   **Soberania de Dados:** Ao manter o Gateway em Rust, garantimos que o ciclo de vida dos dados (do input do usuário à execução do comando) ocorra em um ambiente de memória segura, sem a "sujeira" de runtimes gerenciados (como V8/Node.js).
*   **Performance Determinística:** O modelo de concorrência do Tokio permite que a SODA gerencie milhares de conexões (canais de chat) com latência de microssegundos, algo impossível em arquiteturas baseadas em Electron.
*   **Segurança por Design:** Ao tratar o frontend como uma interface passiva, eliminamos o risco de injeção de código no lado do cliente. O frontend apenas exibe o que o backend autoriza, tornando a SODA imune a ataques de XSS que poderiam comprometer o agente.

---

## 5. Diretrizes de Execução
1.  **Zero-Trust:** Nenhum plugin ou skill é confiável por padrão. Todo código externo deve ser executado em um ambiente isolado (Wasm ou Container).
2.  **Observabilidade:** Todo fluxo de decisão do agente deve ser logado em uma estrutura de *Trace* (OpenTelemetry), permitindo a auditoria completa do "porquê" de uma ação ter sido tomada.
3.  **Soberania:** O usuário final deve ser capaz de rodar a SODA offline. A dependência de APIs de terceiros (OpenAI/Anthropic) deve ser opcional e configurável via *fallback* para modelos locais (Ollama/llama.cpp).

**Curador Arquitetural SODA**
*Genesis MC - "Your machine, your rules."*