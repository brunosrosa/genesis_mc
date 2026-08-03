# PRODUCT REQUIREMENTS DOCUMENT (PRD) - MILESTONE 2 (V2)
## A Selagem do Headroom, Resiliência do Proxy L7 e a Orquestra Multimotor
**Versão:** 2.0  
**Autor:** Gemini Notebook (Parceiro Cognitivo)  
**Status:** APROVADO / PRONTO PARA EXECUÇÃO  
**Target Hardware:** Intel i9 (32GB RAM) | NVIDIA GeForce RTX 2060m (6GB VRAM)  
**Doutrina:** Spec-Driven Development (SDD) | Realismo Dialético  

---

## 1. INTRODUÇÃO & DIREÇÃO DE VOO (REORDENAMENTO FINANCEIRO)
Após uma triagem empírica impiedosa da Arena do Tier 1, constatou-se que tentar acoplar o roteador **ParetoBandit** de forma ativa neste momento é uma falha metodológica grave de *overengineering* e uma ilusão temporal. O ParetoBandit baseia sua utilidade ($E^3$) na Média Móvel Exponencial (EMA) de latências reais (TTFT/TPOT) e taxas de sucesso de JSONs estruturados. Como os motores locais ainda não estão operando sob regimes híbridos estáveis no host e a fauna de modelos exóticos causa colapsos de grafos, **congelamos temporariamente o acoplamento do ParetoBandit**.

O foco absoluto do **Milestone 2 (V2)** passa a ser a **soldagem da infraestrutura de baixo nível**: consolidar a resiliência de rede do TCP Proxy L7, selar termodinamicamente o headroom de RAM/VRAM, estabilizar a decodificação de streams SSE e estruturar a **Orquestra Multimotor** (os adapters e traits de hot-swap que permitirão aos modelos rodar no motor correto, gerando dados legítimos para, futuramente, alimentar o ParetoBandit).

---

## 2. ESCOPO TÉCNICO & DIRETRIZES DE ENGENHARIA

### 2.1. Selagem do Headroom: Evicção LRU Dinâmica (`headroom_engine.rs`)
O cofre de contexto local (`SoulsCcrStore`) armazena payloads e buffers em RAM concorrente (`DashMap`). Atualmente, o crescimento é puramente monotônico via acréscimo atômico (`fetch_add`), o que asfixiará o host (32GB) sob execução de enxames prolongados 24/7.

*   **Requisito:** Implementar uma política de evicção **LRU (Least Recently Used) com Decaimento de Utilidade por Idade** no `SoulsCcrStore`.
*   **Mecânica:**
    *   Adicionar um mapa de controle ou estender a chave do `DashMap` para rastrear a tupla `(Payload, Timestamp de Último Acesso)`.
    *   Configurar uma marca de **Maré Alta (90% de `max_ram_bytes`)** lida dinamicamente da env `SOULS_CCR_MAX_RAM_MB`.
    *   Sempre que o método `store()` for invocado e a maré alta for atingida, disparar uma rotina de evicção síncrona/atômica.
    *   A evicção deve ejetar registros em lote baseado no tempo do último acesso, decrementando `current_ram_bytes` até que a alocação de memória recue para o limiar de **Maré Baixa (80% de `max_ram_bytes`)**.

### 2.2. Resiliência do Proxy L7: Proteção contra Fragmentação TCP (`agentgateway_tcp_proxy.rs`)
A lógica antiga lia pacotes em buffers crus de 8KB e aplicava `.contains("headroom_retrieve")`. Isso quebra catastroficamente caso os pacotes TCP quebrem a string ao meio (fragmentação física de pacotes de rede).

*   **Requisito:** Implementar um **Acumulador Assíncrono de Stream Baseado em Linhas SSE**.
*   **Mecânica:**
    *   Utilizar um buffer de linhas e quebras de delimitação do protocolo Server-Sent Events (`\n\n`) via `tokio::io::BufReader`.
    *   O loop assíncrono só deve aplicar verificações, substituições e o *Response Healing* sintático sobre eventos SSE (`data: ...`) completamente reconstruídos e estáveis em memória.
    *   Qualquer chunk quebrado na fronteira do pacote deve ser retido no buffer interno do proxy até que o delimitador de encerramento do frame chegue do socket upstream.

### 2.3. Poda de I/O: Otimização Zero-Copy $O(1)$ (`agentgateway_tcp_proxy.rs`)
A função `mutate_json_payload` hoje desserializa o corpo inteiro de mensagens de chat de completions via `serde_json::from_slice` para ler parâmetros de streaming. Para payloads massivos (como árvores AST de monorepos complexos superiores a 1MB), isso causa picos de alocação de Heap inaceitáveis.

*   **Requisito:** Substituir a desserialização pesada por uma varredura seletiva e linear de memória.
*   **Mecânica:**
    *   Inspecionar o buffer bruto buscando offsets e assinaturas das chaves `"stream"` e `"model"` de forma direta, sem carregar toda a árvore JSON para o Heap.
    *   Utilizar lifetimes de strings (`&'a str` ou `Cow<'a, str>`) para ler e mutar o payload, garantindo alocação zero no Heap nas rotinas críticas de proxy.

### 2.4. A Orquestra Multimotor: Trait de Hot-Swap & Adapters de Isolamento
A inferência no Souls MC opera hoje de forma rigidamente monolítica sobre o `llama-cpp-2`. Modelos ternários (BitNet) ou de atenção recorrente (Mamba/Zamba) causam crashes imediatos de FFI.

*   **Requisito:** Abstrair o mecanismo de inferência e os fallbacks em traits estanques, preparando o terreno para a multi-engine física.
*   **Mecânica:**
    *   Definir a trait unificada `EphemeralInferEngine` (com assinaturas assíncronas para carregar, inferir e descarregar).
    *   Implementar o roteamento de fallbacks ("Fail-Soft"):
        *   **`llama-cpp-2` (Geração Primária na GPU):** Drivers com cache KV assimétrico (K=FP16 / V=Q4_K) e limites RoPE para o Phi-4-mini (arquitetura `phi3` no GGUF).
        *   **`bitnet.cpp` / Modelos Ternários:** Interceptar quantizações do tipo `i2_s` ou `i1_s` da família BitNet e desviar graciosamente retornando o status `'PENDING_ENGINE'` com mensagem clara ao SQLite, impedindo panics na FFI principal.
        *   **`mistral.rs` (Híbridos / Mamba):** Isolar no roadmap a orquestração do sidecar efêmero de prefill rápido (937 tps) que sofrerá encerramento abrupto via **`SIGKILL`** para evitar vazamento persistente de VRAM e RAM.

### 2.5. Cura do `CodeCompressor` (`headroom_engine.rs`)
O compressor atual é um stub perigoso que faz contagem manual cega de chaves `{}` para simular stubs AST.
*   **Requisito:** Refatorar a lógica para um analisador linear resiliente de escopos, impedindo que chaves internas dentro de literais de strings (`"{}"`) ou comentários quebrem a integridade da substituição do arquivo de código Rust.

---

## 3. DEFINITION OF DONE (DoD) MATEMÁTICO
Para declarar as tarefas do Milestone 2 concluídas, a suíte de testes deve cravar:
1.  **Zero-Leak RAM:** Teste de estresse gerando 1000 payloads consecutivos no `SoulsCcrStore` deve acionar a evicção LRU, mantendo o consumo de memória estritamente abaixo do limite configurado de `SOULS_CCR_MAX_RAM_MB`.
2.  **TCP Fragment Proof:** Um simulador de rede fatiando strings de stream SSE a cada 10 bytes não deve causar falhas de parseamento no proxy.
3.  **Compilation Safety:** Compilação limpa sob a flag `--features "tauri-app,gateway_ccr,llama_backend"` com Exit Code 0, sem avisos de clippy em Rust.
