# Product Requirements Document (PRD) - Milestone 2: Selagem do Headroom e Core do TCP Proxy L7 (Souls MC)

*   **Status**: Proposed / Under Review
*   **Epic**: Roteamento FinOps & Infraestrutura L7
*   **Version**: 1.0 (Souls MC Dynamic Context Era)
*   **Author**: Gemini Notebook (Parceiro Cognitivo)
*   **Target Hardware**: Intel i9 (32GB RAM) + NVIDIA RTX 2060m (6GB VRAM)

---

## 🏛️ 1. Objetivo do Produto (The "Why")

A execução ininterrupta (24/7) de agentes locais e remotos no ecossistema **Souls MC** sob as restrições severas de hardware do nosso "Treino de Gravidade" (RTX 2060m com 6GB VRAM) impõe um limite físico intransigente. O preenchimento contínuo e desordenado do cache de contexto (Context Rot) satura as chaves de atenção, causa picos térmicos de processamento na GPU e induz a falhas catastróficas de Out-of-Memory (OOM).

Este documento especifica a engenharia do **Marco 2 (A Selagem do Headroom e o Core do TCP Proxy L7)**, cujo objetivo é triplo:
1.  **Deter o Estouro de VRAM**: Transmutar o gerenciador de contexto `SoulsCcrStore` de um cache de crescimento monotônico para um sistema dinâmico de **Evicção LRU (Least Recently Used) com Decaimento de Idade**, operando em RAM com alocação estável.
2.  **Garantir Tolerância a Falhas na Rede (Response Healing SSE)**: Acoplar o consertador sintático diretamente na esteira de Server-Sent Events (SSE) do proxy TCP L7, interceptando dados truncados de APIs na nuvem (ou motores locais instáveis) e forçando o fechamento determinístico de chaves e colchetes no fluxo da stream para proteger a UI em Svelte 5.
3.  **Aniquilar Gargalos de Alocação de Memória**: Eliminar as operações síncronas de desserialização completa (`serde_json::from_slice`) do proxy L7 sobre cargas gigantescas de texto (como árvores AST de 100k linhas ou linters SAST >1MB), substituindo-as por slicing de bytes zero-copy com complexidade de tempo de execução $O(1)$.

---

## 🛠️ 2. Descrição Funcional e Componentes (The "What")

```
                     +---------------------------------------+
                     |        Upstream Stream (SSE/HTTP)     |
                     +-------------------+-------------------+
                                         |
                                         | (Raw Bytes Stream)
                                         v
                     +-------------------+-------------------+
                     |       agentgateway_tcp_proxy          |
                     |  - Zero-Copy JSON Slicing O(1)        |
                     |  - DFA-Based Literal Healing          |
                     |  - Stack-Based Bracket Balancing      |
                     +-------------------+-------------------+
                                         |
                                         | (Healed & Mutated Stream)
                                         v
                     +-------------------+-------------------+
                     |              Svelte 5 UI              |
                     +-------------------+-------------------+
                                         ^
                                         | (Context Query)
                                         v
                     +-------------------+-------------------+
                     |             SoulsCcrStore             |
                     |  - DashMap Zero-VRAM Allocation       |
                     |  - LRU Eviction with Age Decay (90%)  |
                     +---------------------------------------+
```

### 🧩 Módulo 2.1: SoulsCcrStore & Evicção LRU com Decaimento de Idade
*   **Entrada (Input)**: ID do canal de contexto, payload de texto a ser cacheado, e timestamp de acesso.
*   **Saída (Output)**: Estado atualizado da cache, confirmação de gravação ou liberação síncrona de memória.
*   **Falha Mapeada (Failure Scenario)**: A inserção do novo payload excede o limite físico rígido `max_ram_bytes`.
*   **Abordagem de Engenharia**:
    *   Substituir o armazenamento monotônico por um mapa concorrente thread-safe (`DashMap`) em RAM.
    *   Implementar um **Algoritmo de Evicção LRU com decaimento exponencial de utilidade**. Cada leitura de fatia de contexto reseta seu contador de idade; fatias ociosas têm seu valor atenuado.
    *   Sempre que o tamanho total de bytes na cache bater em **90% da margem de segurança** (`max_ram_bytes`), o coletor síncrono dispara e ejeta as fatias mais frias da RAM/VRAM até que a pegada de memória retorne ao nível de trabalho estável (70%).

### 🧩 Módulo 2.2: Interceptador de Rede & Response Healing SSE (DFA-Based)
*   **Entrada (Input)**: Buffer de fluxo de bytes de streaming Server-Sent Events (SSE).
*   **Saída (Output)**: Stream de bytes processada contendo objetos JSON estruturalmente válidos.
*   **Falha Mapeada (Failure Scenario)**: Upstream encerra a stream abruptamente antes de enviar o delimitador de fechamento `}` ou `]`.
*   **Abordagem de Engenharia**:
    *   Integrar a lógica de cura sintática no loop assíncrono de streaming do `agentgateway_tcp_proxy.rs`.
    *   **Máquina de Estados Finitos Determinística (DFA)** para substituição de literais: O parser deve substituir dinamicamente booleanos e nulos malformatados gerados por modelos que "falam Python" fora de literais de string:
        *   `True` ➔ `true`
        *   `False` ➔ `false`
        *   `None` ➔ `null`
        *   A regra de contorno proíbe a alteração caso o caractere esteja contido em uma string (ex: `"Status: True"` deve permanecer intocado).
    *   **Balanço de Pilha sínclono**: Ao detectar o sinal de interrupção (EOF ou quebra do socket SSE), o proxy lê a pilha de colchetes/chaves ativas e cospe os caracteres de fechamento correspondentes (`}` ou `]`) para fechar as estruturas em aberto, devolvendo um JSON válido para o desserializador downstream.

### 🧩 Módulo 2.3: Otimização Zero-Copy do Proxy (`mutate_json_payload`)
*   **Entrada (Input)**: Fatias de buffers de bytes bruto (`&[u8]`).
*   **Saída (Output)**: Payload mutado em formato binário sem reconstrução do Heap.
*   **Falha Mapeada (Failure Scenario)**: Payload gigante superior a 10MB causa picos de alocação de RAM e estoura o tempo de resposta do Tokio.
*   **Abordagem de Engenharia**:
    *   Banir terminantemente o uso de `serde_json::from_slice` genérico na linha 68 (ou equivalente) para varrer todo o corpo das mensagens.
    *   Utilizar fatiamento de strings de baixo nível ancoradas em lifetimes de Rust (`&'a str` ou `Cow<'a, str>`) e algoritmos de varredura em tempo constante $O(1)$ (como parsing vetorial SIMD via `simd-json`).
    *   O proxy deve localizar campos estruturais de controle (ex: `_meta`, `tools`, `model`) inspecionando apenas os primeiros offsets de bytes do payload, sem instanciar structs pesadas ou clonar strings para o Heap do sistema.

---

## 🚫 3. Proibições Térmicas e Linhas Vermelhas (Inegociáveis)

1.  **PROIBIDO** o uso de qualquer biblioteca ou runtime baseado em Node.js, V8 ou Python dentro do `agentgateway_tcp_proxy.rs`. Todo o pipeline de intercepção e cura deve ser em Rust puro.
2.  **PROIBIDO** realizar alocações de memória dinâmica síncronas que bloqueiem o loop de eventos assíncrono do Tokio durante o tráfego de dados do proxy.
3.  **PROIBIDO** o uso de expressões regulares (Regex) ingênuas para a extração ou correção de estruturas de chaves em JSON, devido ao perigo de backtracking catastrófico e travamento térmico da CPU i9.
4.  **PROIBIDO** permitir o crescimento de contexto da cache sem limites reais (crescimento monotônico). A evicção deve ser automática e determinística ao tocar no teto de 90% da alocação de segurança.
5.  **PROIBIDO** mascarar erros de FFI ou de conexões upstream com vazamento de dados nulos ("N/A") ou strings falsas. Se o sistema não puder curar o payload, o erro deve falhar de forma limpa e graciosa (Fail-Soft), registrando a falha no banco de telemetria local.

---

## 🚦 4. Critérios de Aceite (Definition of Done - DoD)

*   [ ] **Exit Code 0**: O código do backend em Rust compila perfeitamente sem warnings limitantes executando:
    ```powershell
    cargo check --manifest-path Cargo.toml --features "tauri-app,gateway_ccr,llama_backend" --bin agentgateway_tcp_proxy
    ```
*   [ ] **Teste de Evicção LRU**: Testes de unidade simulam a inserção contínua de 50 fatias de contexto que totalizam 150% do `max_ram_bytes`. O `SoulsCcrStore` deve ejetar síncronamente as 17 fatias mais velhas, mantendo o consumo de memória rigorosamente abaixo do limite de 90%.
*   [ ] **Teste de Fidelidade Literal (DFA)**: O validador do proxy recebe o payload contendo:
    ```json
    { "valido": "This is True", "status_python": True, "nulo": None }
    ```
    E cospe exatamente o output curado sem mutar a primeira string:
    ```json
    { "valido": "This is True", "status_python": true, "nulo": null }
    ```
*   [ ] **Teste de Cura SSE**: Simular uma stream cortada no meio:
    ```json
    {"data": {"choices": [{"delta": {"content": "Olá, Mestre!"
    ```
    O proxy de streaming intercepta a queda do socket e entrega ao downstream o payload balanceado e fechado:
    ```json
    {"data": {"choices": [{"delta": {"content": "Olá, Mestre!"}}]}}
    ```
*   [ ] **Benchmark Zero-Copy**: Executar o teste de estresse contra um payload de linter SAST de 15MB. A operação de mutação e roteamento deve rodar em menos de **1.2 milissegundos** no processador Intel i9, com alocação estática no Heap abaixo de **50KB**.

---

## 📋 5. Plano de Execução e Tarefas de Desenvolvimento (TDD)

```
+---------------------------------------------------------------------------------+
|                                 TASKS WORKFLOW                                  |
+---------------------------------------------------------------------------------+
| [ ] Task 1: Escrever testes unitários em 'tests/ccr_lru_tests.rs' para o SoulsCcrStore.  |
| [ ] Task 2: Implementar o algoritmo LRU com decaimento exponencial no SoulsCcrStore.   |
| [ ] Task 3: Escrever testes unitários em 'tests/healing_tests.rs' para o DFA e Brackets. |
| [ ] Task 4: Codificar o DFA de tradução literal e o balanceador de colchetes de pilha. |
| [ ] Task 5: Refatorar o parser de 'mutate_json_payload' usando slicing lifetimes &'a str.|
| [ ] Task 6: Acoplar o Response Healing no pipeline de stream assíncrono do proxy TCP L7.|
| [ ] Task 7: Executar bateria de testes de estresse de 15MB e aferir latência e heap.     |
+---------------------------------------------------------------------------------+
```

### Fase C: O TDD Implacável (Como rodar a implementação)
O desenvolvedor na IDE executará o ciclo de correção atômico aplicando a regra dos 3 erros:
1.  **Red Phase**: Criar os testes que falham explicitando o comportamento esperado de fechar chaves de streams inacabadas e manter booleanos internos de strings intocados.
2.  **Green Phase**: Escrever o código mínimo em Rust para fazer os testes passarem com sucesso.
3.  **Refactor Phase**: Ajustar lifetimes e eliminar cópias redundantes de memória usando `Cow` e referências para atingir a meta do benchmark.
