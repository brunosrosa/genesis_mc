---
id: "ADR-003"
title: "ADR-003-Zero-Copy-IPC"
version: 1.0
status: Ativo_Inegociavel
epic: "IPC"
description: "Adota buffers binários estruturados (rkyv/Arrow) para comunicação IPC de alta performance, eliminando o garbage collection do V8."
---

# ADR-003-Zero-Copy-IPC

## Status
Aceito (Ativo e Inegociável)

## Contexto
Durante o fluxo de inferência local e telemetria profunda, o core Rust despeja fluxos contínuos e massivos de dados (estados de grafos, logs estruturados da CPU e chunks de texto de SLMs). Serializar essa montanha de dados em strings JSON no Rust e fazer o parse no JavaScript (V8) gera gargalos de I/O drásticos, asfixiando a thread principal da interface. Mais criticamente, a constante alocação e desalocação de strings curtas e objetos JSON no motor de JavaScript aciona ciclos agressivos do *Garbage Collector* (GC) do navegador, causando micro-congelamentos de tela ("Flow-Debt") intoleráveis para a interface reativa e neuro-inclusiva do SODA.

## Decisão
Fica decretado que toda comunicação IPC de alto throughput e transmissão contínua de dados entre a base Rust e a Janela de Vidro (Svelte 5/Tauri v2) deve operar estritamente sobre buffers binários estruturados:
1. **Dados Tabulares e Histórico de Telemetria:** Serializados e empacotados usando o formato colunar **Apache Arrow** no Rust.
2. **Mensagens Estruturadas e DTOs Complexos:** Serializados com a biblioteca **rkyv** em Rust, gerando buffers estruturados baseados em offsets binários exatos.
3. **Mecânica de Transporte e Consumo JS:** O Tauri v2 encaminhará os dados brutos como `ArrayBuffer`. Web Workers no JavaScript interceptam esses buffers e utilizam a semântica de **Transferable Objects** para transferir a propriedade de memória física para a thread do Svelte sem cópia real de dados. A desserialização é executada sob demanda por offsets binários de leitura instantânea ($\mathcal{O}(1)$), bloqueando alocações supérfluas no heap do JavaScript.

## Consequências
- **Eliminação de Latência IPC:** Desserialização praticamente instantânea, mitigando o tempo de processamento IPC a quase zero.
- **Zero Pressão no GC:** Redução dramática na taxa de alocações na RAM no frontend V8, erradicando micro-congelamentos de tela.
- **Padrão Estrito de Interfaces:** Fica estritamente proibido o envio de strings ou payloads JSON massivos para a camada IPC principal. A tipagem das mensagens de comunicação deve ser definida por esquemas binários compartilhados.

## Restrições Bare-Metal
- **IPC Zero-Copy Real:** É obrigatório usar **iceoryx2** (POSIX Shared Memory) como transporte Zero-Copy; é proibido introduzir dupla serialização via Arrow FFI (o frontend recebe apenas descritores/offsets para buffers).
- **Purificação Reativa (Svelte 5):** Antes de qualquer tráfego IPC, objetos complexos devem ser materializados via `$state.snapshot()`; é proibido trafegar Proxies reativos do Svelte/V8.
- **Transferable Objects (Web Workers):** Buffers binários devem atravessar Workers $\rightarrow$ Main Thread exclusivamente como **Transferable Objects**, evitando cópias e instâncias massivas no heap do V8.
- **Latência de Desserialização JavaScript:** Leituras estruturadas sobre o buffer rkyv/Arrow de tamanho normal no JS devem executar em menos de **1ms**.
- **Cópia na RAM:** O número de cópias físicas de buffers de dados superiores a **10KB** na passagem Rust $\rightarrow$ JS deve ser zero.
- **Frequência de Reflow:** O batching na interface gráfica deve limitar atualizações IPC a no máximo **60 vezes por segundo**, atrelado dinamicamente ao `requestAnimationFrame` (rAF) do navegador.
