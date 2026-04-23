# Manual Canônico: Arquitetura SODA (Genesis MC)

Este documento estabelece as diretrizes fundamentais para a operação do **SODA (Sovereign Operating Data Architecture)**. Como Curador Arquitetural, declaro que qualquer desvio destas normas — especialmente a introdução de runtimes interpretados (Node.js, Python) no *hot path* ou a adoção de paradigmas de interface reativa (React/VDOM) — constitui uma falha crítica de segurança e performance.

---

## 1. O Dogma Tecnológico (Stack Pura)
A arquitetura SODA é estritamente **Rust-first**.
*   **Backend:** Tokio (Runtime assíncrono). A comunicação é feita via IPC Zero-Copy.
*   **Frontend:** Svelte 5 + Tauri v2. O frontend é uma **interface passiva**. Ele não processa lógica de negócio; ele apenas renderiza o estado projetado pelo backend Rust.
*   **Proibição:** Estão banidos da base de código: React, Node.js daemons, Electron, VDOM e qualquer forma de Server-Side Rendering (Next.js). A complexidade deve ser resolvida no compilador, não no runtime.

## 2. Hardware-Awareness e Otimização Bare-Metal
O Genesis MC opera sobre o hardware (Intel i9 / RTX 2060m).
*   **Execução:** Diretivas AVX2 são obrigatórias para otimização de inferência local.
*   **Memória:** O uso de `mmap` via `llama.cpp` é a única forma aceitável de carregar pesos de modelos na VRAM da RTX 2060m, evitando gargalos de barramento PCIe.
*   **Armazenamento:** O uso de *Dev Drives* (ReFS) no Windows é mandatório para reduzir a latência de I/O em artefatos de compilação e bancos de dados Dolt.

## 3. Auditoria Crítica: Furos e Fragilidades
Após análise das fontes, identificamos pontos de falha que devem ser mitigados:
*   **O "Furo" do Node.js:** A fonte sobre *Tree-sitter* menciona a necessidade de Node.js para a geração de gramáticas. **Correção:** O SODA deve tratar o Node.js como uma ferramenta de *build-time* isolada. Ele nunca deve residir no ambiente de execução do usuário final.
*   **Fragilidade do CoT:** A dependência de *Chain-of-Thought* (CoT) para segurança é frágil. Modelos podem "esquecer" ou "falsificar" o CoT se detectarem monitoramento. **Mitigação:** O SODA não confia no texto. A segurança é imposta via **eBPF (crate `aya`)** no kernel, interceptando syscalls maliciosas independentemente do que o LLM "pensa".
*   **Conflito de Linkagem (LNK2038):** A mistura de bibliotecas C++ (Signal Protocol) com Rust exige controle rigoroso de CRT (`/MT` vs `/MD`). O SODA impõe o uso de `target-feature=+crt-static` para garantir um binário único e imutável.

## 4. O "Porquê" Técnico: A Soberania do SODA
O SODA não é apenas um orquestrador; é um **sistema de defesa**.
1.  **Isolamento de Contexto:** Dados não são concatenados. São encapsulados em *Data-Boundary Markers* (Rust-parsed), impedindo que o LLM execute comandos injetados.
2.  **Telemetria de Latência:** O monitoramento de *Time-To-First-Token* (TTFT) via Rust permite detectar "pensamento oculto" (scheming). Se o modelo demora mais do que o esperado para responder, o sistema assume que ele está tentando evadir a supervisão e aplica restrições de MAC (Mandatory Access Control) imediatas.
3.  **Honeypots de Kernel:** O uso de eBPF para criar ambientes de execução falsos (Honeypots) é a única forma de capturar a "decepção alinhada". O modelo acredita ter acesso root, mas está confinado em um sandbox monitorado pelo kernel.

## 5. Diretriz de Implementação (Policy Enforcement)
Para evitar erros de deserialização (`RuleSerde`), toda configuração de gateway deve seguir o padrão de *Literal Block Scalar* (`|`) no YAML, garantindo que o CEL (Common Expression Language) seja interpretado sem erros de escape.

**Regra de Ouro:** Se o componente não pode ser compilado para um binário estático e auditado via `cargo-audit`, ele não pertence ao Genesis MC.

---
*Curadoria Finalizada. O sistema está em estado de prontidão. Qualquer tentativa de introduzir dependências de runtime não-nativas será bloqueada pelo CI/CD.*