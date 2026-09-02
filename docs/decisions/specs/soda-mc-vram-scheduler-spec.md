# ESPECIFICAÇÃO TÉCNICA E CADERNO DE TDD: MARCO 5.12.0 (TASK 137)

## 🥶 1. Task 137 — O VRAM Scheduler Dinâmico no Model Manager

### 1.1 Racional do Design (A Cura do PCIe Bandit Spillover)
O Souls MC opera sob a restrição física inegociável de uma GPU dedicada **NVIDIA RTX 2060m** dotada de exatos **6.144 MB of VRAM** [118]. No entanto, seu sistema conta com **32 GB of Host RAM** [368]. 
Se múltiplos subagentes cognitivos tentarem instanciar e alocar simultaneamente diferentes modelos locais na VRAM (ex: o Qwen Coder 4B para a geração síncrona [1170] e o Gemma 4 E2B para o prober epistêmico [1169]), a dGPU sofrerá de estouro de capacidade física. O driver do sistema operacional responderá ativando o *paging* de memória (offloading/spillover) através do barramento **PCIe Gen3** [368, 413].
Como a largura de banda do PCIe Gen3 é ordens de magnitude inferior à taxa interna da VRAM, a inferência local despencará instantaneamente de **45 tokens/s** para velocidades de **1 a 3 tokens/s**, gerando asfixia de I/O, estresse térmico severo e congelamento do chat ativo [413, 1170].

A **Task 137 (VRAM Scheduler Dinâmico)** resolve esta tensão de forma determinística por meio de um **Gerenciador de Alocação de VRAM Ativo com Evicção LRU**. O sistema calcula a pegada total da alocação de forma Ahead-Of-Time (AOT), monitora os limites do barramento e executa de forma atômica o hot-swapping de modelos na VRAM (via **`mmap`** com as bindings nativas do `llama.cpp`) [413], garantindo que a memória de vídeo dedicada permaneça sempre otimizada e fria.

### 1.2 A Equação Matemática do Orçamento de VRAM
O Scheduler proíbe qualquer alocação estocástica. Antes de enviar o sinal de carregamento de pesos ao interpretador do `llama.cpp`, a pegada total necessária para a execução estável é calculada pela fórmula clássica:

$$M_{\text{total}} = M_{\text{model}} + M_{\text{KV}} + \delta_{\text{safe}} \le M_{\text{limit}}$$

Onde:
*   $M_{\text{model}}$ é o peso estático do arquivo quantizado GGUF em disco (lido de forma $O(1)$ pelo parser bare-metal) [23].
*   $\delta_{\text{safe}}$ é a margem de proteção térmica e de tensores temporários de ativação, fixada rigidamente em **512 MB** [118].
*   $M_{\text{limit}}$ é o limite de alocação de segurança, fixado em **5.632 MB** (preservando o teto físico de 6.144 MB contra colapsos de contexto do OS).
*   $M_{\text{KV}}$ é a pegada dinâmica teórica calculada pelo tamanho de lote ($b = 1$) [118], as camadas do modelo ($l$), o número de cabeças de atenção ($h$), a dimensão da cabeça ($d_{\text{head}}$), a precisão em bytes ($p$, ex: 1 para INT8) [119] e o tamanho do contexto do prompt ($s$):

$$M_{\text{KV}} = 2 \times b \times s \times l \times h \times d_{\text{head}} \times p$$

---

## ⚙️ 2. A Máquina de Estados e a Evicção LRU

```
                 [Requisição de Modelo: Model B]
                               │
                               ▼
            ┌──────────────────────────────────────┐
            │ Calcular M_total(B) e Folga de VRAM  │
            └──────────────────┬───────────────────┘
                               │
                ┌──────────────┴──────────────┐
                ▼                             ▼
        [Folga Suficiente]            [VRAM Saturada]
                │                             │
                │                             ▼
                │                  ┌──────────────────────┐
                │                  │  Selecionar Inativo  │
                │                  │  via Heurística LRU  │
                │                  └──────────┬───────────┘
                │                             │
                │                             ▼
                │                  ┌──────────────────────┐
                │                  │   Unload do Modelo   │
                │                  │  Inativo da dGPU     │
                │                  └──────────┬───────────┘
                │                             │
                ▼                             ▼
        ┌─────────────────────────────────────────┐
        │  Carregar Model B na VRAM via Tokio     │
        │  spawn_blocking (Isolamento de Threads) │
        └─────────────────────────────────────────┘
```

### 2.1 Os Três Estados Físicos de Modelos
1.  **`Active`**: Carregado na VRAM da dGPU com aceleração de hardware ativa.
2.  **`Standby`**: Carregado na Host RAM usando arquivos mapeados via `mmap` read-only.
3.  **`Unloaded`**: Totalmente descarregado do sistema, liberando ponteiros e buffers.

### 2.2 O Algoritmo de Evicção LRU (Least Recently Used)
Se a soma de $M_{\text{total}}$ do novo modelo requisitado colidir com o limite máximo configurado na GPU ($M_{\text{limit}}$), o Scheduler entra em modo defensivo:
1.  Busca na tabela relacional `model_registry` o modelo com o estado `Active` que apresenta o maior timestamp desde a última chamada de inferência ativa [1093, 1256].
2.  Executa de forma atômica o método de descarregamento (`unload_model`), derrubando a sua ocupação de VRAM.
3.  Após a confirmação da liberação física dos buffers da GPU, dispara o carregamento do novo modelo requisitado.
4.  Para evitar picos de latência e starvation (fome de threads) no loop assíncrono do Tokio, toda a rotina de swap de mmap e carregamento do llama.cpp é despachada de forma sínclona em uma thread de trabalho dedicada via **`tokio::task::spawn_blocking`** [127, 600, 1087, 1088].

---

## 🚦 3. Caderno de Testes TDD (DoD GREEN)

Escreveremos e rodaremos os seguintes testes sob **`cargo test --bin souls_mcp_server`**:

1.  **`test_vram_scheduler_budget_calculation`**: Prova que o cálculo do peso do modelo e estimativa teórica de KV cache mapeia com exatidão de bytes as equações do Bedrock e do MLOps [118, 119], sem provocar estouros imprevistos.
2.  **`test_lru_eviction_under_pressure`**: Configura um limite simulado de VRAM de 5.000 MB. Tenta carregar sequencialmente 3 modelos sintéticos com peso de 2.000 MB cada. Assevera que o modelo mais antigo (LRU) é ejetado da memória de forma sequencial e determinística para abrir espaço para o terceiro item, sem causar segfaults ou travas de I/O.
3.  **`test_vram_concurrency_tokio_blocking`**: Submete requisições simultâneas de carregamento em múltiplos threads concorrentes do Tokio, provando que o VRAM Scheduler serializa e enfileira as transições de carregamento, processando-as de forma segura dentro do pool de bloqueio isolado, mantendo a thread assíncrona principal livre de latências de cauda [128, 1087, 1088].
