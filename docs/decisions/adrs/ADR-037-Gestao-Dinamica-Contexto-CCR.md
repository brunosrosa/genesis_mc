---
id: "ADR-037"
title: "ADR-037-Gestao-Dinamica-Contexto-CCR"
version: 2.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Governança de contexto via REPL/RLM e Prensa Elástica CCR. Impõe janela curta (4-8 turnos), Context Slicing, desidratação AST (lean_vacuum), DashMap Zero-VRAM e a Barreira de Domínio contra compressores estatísticos em código-fonte."
---

# ADR-037: Gestão Dinâmica de Contexto, Motor REPL/RLM e Prensa Elástica CCR

## Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SOULS V6)

## Contexto Técnico e o Colapso de Atenção (Context Rot)
Durante a execução de fluxos agênticos complexos, a acumulação desordenada de logs, saídas JSON de ferramentas e arquivos de código induz ao *Context Rot* (deterioração da atenção do modelo) e arrisca estouro de VRAM na RTX 2060m (6GB). 
Além disso, aplicar compressores estatísticos não-determinísticos (como `LLMLingua-2` ou PyTorch/ModernBERT) sobre código-fonte mutila identidades de variáveis, caminhos de arquivos e sintaxe de linguagem, corrompendo a lógica do sistema.

## Decisão Arquitetural (Motor REPL/RLM, Prensa CCR e Barreira de Domínio)
Fica estabelecida a governança dinâmica de contexto baseada em **4 Pilares Constitucionais**:

### 1. Arquitetura do Motor REPL/RLM e Janela Curta (SPEC-012)
*   **Single-Session Master:** O agente Master opera em sessão contínua mantendo um prompt enxuto e delegando investigações pesadas para sub-sessões RLM efêmeras enjauladas em Wasmtime.
*   **Context Slicing:** As sub-sessões RLM recebem apenas o recorte atômico estritamente necessário para sua sub-tarefa, retornando um *Reduce Sintético* em Notação LEAN.
*   **Janela Ativa Recente ($T_{\text{live\_diff}}$):** A janela de conversa ativa mantida no prompt é rigorosamente delimitada a um buffer curto de **4 a 8 turnos de texto puro ($N = 4 \text{ a } 8$)**. Interações antigas são desidratadas e consolidadas de forma assíncrona pela Tríade de Memória (L2 FrankenSQLite / L3 LanceDB).

### 2. Prensa Elástica CCR e Desidratação AST (`lean_vacuum`)
*   **Threshold Deslizante:** Qualquer bloco de dados, logs ou código que ultrapasse o limiar deslizante de **5 linhas** no pipeline de entrada é submetido à prensa elástica CCR.
*   **Poda Sintática Lossless (`lean_vacuum`):** A CPU Host (AVX2) executa a desidratação sintática lossless via `tree-sitter`, removendo ruídos e comentários mortos, mas preservando intactas assinaturas de tipos e declarações.
*   **Armazenamento RAM Host (`DashMap` Zero-VRAM):** Os corpos brutos originais omitidos são salvos 100% na memória RAM principal do computador em um `dashmap::DashMap<[u8; 16], Bytes>`, indexados por um hash hexadecimal minúsculo (Blake3/MD5). Consumo de VRAM adicional: **0.00 MB**.
*   **Injeção de Stubs de Resgate:** No prompt do modelo entra apenas o stub compacto contendo o token de resgate:
    `[SOULS CCR: bloco comprimido. Para reidratar os dados integrais, invoque headroom_retrieve(hash="a1b2c3d4e5f6")]`

### 3. Barreira de Domínio Inegociável (Proibição do LLMLingua-2)
*   Fica **TERMINANTEMENTE PROIBIDO** o uso de `LLMLingua-2`, compressores neurais/estatísticos ou chamadas a runtimes Python sobre caminhos de arquivos, nomes de símbolos ou blocos de código-fonte de qualquer linguagem de programação.
*   A compressão de código e caminhos de arquivos DEVE ser 100% determinística, orientada por AST e desidratação sintática em Rust.

### 4. Interceção Tool Loopback em $< 1\text{ms}$ no Tokio Proxy
*   Se a SLM solicitar a reidratação via `headroom_retrieve(hash="...")`, o Gateway Tokio intercepta a chamada na camada de proxy HTTP em $< 100\mu\text{s}$, busca o payload original no `DashMap` Host RAM e re-injeta o bloco no histórico sem queimar ciclos de GPU ou acionar IO de disco.

## Compliance com a Doutrina Bare-Metal SOULS
| Diretiva SOULS | Estado do ADR-037 | Garantia de Implementação |
| :--- | :--- | :--- |
| **Zero VRAM Extra** | **CONFORME** | 100% alocado em `DashMap` na RAM Host. Zero tensores na GPU. |
| **Barreira de Domínio** | **CONFORME** | `LLMLingua-2` sumariamente banido de código e filepaths. |
| **Janela Curta Chat** | **CONFORME** | Buffer $T_{\text{live\_diff}}$ travado entre 4 e 8 turnos ($N=4..8$). |
| **Prensa Lossless** | **CONFORME** | Limiar de 5 linhas com desidratação AST `lean_vacuum` e Hash Blake3. |
| **SDD / Spec-Driven** | **CONFORME** | Este ADR oficializa a física de gestão de contexto antes de qualquer código/PRD. |
| **Latência Sub-ms** | **CONFORME** | Interceção loopback $< 100 \ \mu\text{s}$ no Tokio, total $< 1.0 \text{ ms}$. |
