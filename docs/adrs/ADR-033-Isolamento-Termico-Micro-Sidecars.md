---
id: "ADR-033"
title: "ADR-033-Isolamento-Termico-Micro-Sidecars-em-CPU-Afinidade-de-Nucleo"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Impõe a execução de micro-sidecars (Embeddings e TTS/STT) estritamente na CPU via afinidade de núcleos (taskset/pinning), preservando VRAM e o Event Loop do Tokio."
---

# ADR-033: Isolamento Térmico de Micro-Sidecars em CPU (Afinidade de Núcleo)

## Status

Aceito (Ativo, Inegociável e Fundacional para SOULS V5)

## Contexto Técnico e Restrições de Concorrência

A operação de um Sistema Operacional Agêntico Local (SOULS V5) exige a execução concorrente de múltiplos serviços utilitários além do modelo de linguagem principal. Estes incluem:

- **Encoders de Embeddings Locais:** Responsáveis pela vetorização semântica de curto prazo e busca RAG em memória (ex: `bge-small-en-v1.5`, `all-MiniLM-L6-v2`).
- **Módulos de Áudio (STT / TTS):** Motores de transcrição de fala (`Moonshine`, `Whisper-Tiny`) e síntese de voz (`Kokoro-82M`).

Quando esses micro-sidecars são incorretamente alocados na VRAM da dGPU NVIDIA RTX 2060m ou tentam compartilhar o *stream* CUDA com o LLM principal, surgem graves patologias de sistema:

1. **Fragmentação de VRAM & Picos de OOM:** Pequenas matrizes alocadas dinamicamente na VRAM fragmentam a memória de vídeo, reduzindo a janela contígua necessária para estender a cache KV do modelo base.
2. **Context Switching na GPU & Stuttering:** Disputas pelo CUDA stream paralisam temporariamente a inferência autorregressiva, provocando surtos imprevisíveis de latência no *Time-To-First-Token* (TTFT) e micro-congelamentos na síntese de áudio em tempo real.
3. **Contenção no Event Loop Assíncrono:** A execução de tarefas intensivas de CPU/GPU em threads genéricas do runtime assíncrono Tokio bloqueia os *workers* de I/O, aumentando a latência de respostas do gateway Rust.

## Declaração do Problema

Como garantir a execução contínua de micro-sidecars utilitários (Embeddings e Áudio) com alta throughput e baixa latência sem alocar um único megabyte de VRAM, sem induzir *stuttering* na inferência do LLM e sem degradar a estabilidade do Event Loop Tokio?

## Decisões Arquiteturais da SOULS V5

```
                                [ PROCESSADOR INTEL / AMD ]
                                             |
     +---------------------------------------+---------------------------------------+
     |                                                                               |
     v                                                                               v
[ NÚCLEOS 0-3: Taskset Dedicated ]                                 [ NÚCLEOS 4-N: Tokio Event Loop ]
(Micro-Sidecars em CPU Host RAM)                                    (Orquestração & GPU Stream Manager)
  - Embeddings: bge-small (~130 MB RAM)                                              |
  - TTS: Kokoro-82M (~200 MB RAM)                                                    v
  - RTF <= 0.25 | TTFA Sub-100ms                                  [ dGPU RTX 2060m: 100% VRAM Dedicada ]
                                                                    (LLM Qwen 3.5 4B + KV Cache)
```

### 1. Offloading Obrigatório de Micro-Sidecars para a CPU

Fica terminantemente **PROIBIDO** o carregamento e a execução de encoders de embeddings locais e motores de síntese/transcrição de áudio na VRAM da GPU.

- **Embeddings:** O SOULS V5 padroniza a vetorização local no modelo **`bge-small-en-v1.5`** (ou `BGE-micro-v2`), executado estritamente via runtime CPU (`ONNX Runtime` CPU EP ou `sentence-transformers` via C++ bindings).
  - *Desempenho Comprovado:* Atinge $>93\%$ da acurácia do `text-embedding-3-small` da OpenAI no benchmark MTEB com pegada de apenas $\sim 130 \text{ MB}$ na RAM do host.
- **Síntese de Voz (TTS):** O motor **`Kokoro-82M`** deve ser instanciado em modo `ONNX CPU`, consumindo $\sim 200 \text{ MB}$ de RAM e mantendo um *Real-Time Factor* $\text{RTF} \le 0,25$.

### 2. Isolamento Físico via Afinidade de Núcleo (Thread Pinning / `taskset`)

Para eliminar o risco de contaminação cruzada de cache L3 e contenção de processamento na CPU, os workers de micro-sidecars DEVEM ser fisicamente isolados em núcleos de CPU específicos via **Afinidade de Núcleo** (_Core Affinity_):

- **Linux OS:** Utilização da utilidade `taskset` ou chamadas de sistema `sched_setaffinity` no momento do *fork/spawn* dos processos sidecar.
  ```bash
  # Invocação isolada do worker TTS Kokoro-82M nos núcleos 0 a 3 da CPU
  taskset -c 0-3 python -m kokoro_runner \
    --model kokoro-82m.onnx \
    --threads 4
  ```
- **Windows OS:** Definição compulsória da máscara de afinidade de processo via `SetProcessAffinityMask` na inicialização do worker sidecar pelo daemon Rust.
- **Preservação do Event Loop do Tokio:** Os núcleos alocados para os micro-sidecars (ex: Cores 0-3) são expressamente excluídos da piscina de threads do runtime Tokio (`tokio::runtime::Builder::new_multi_thread`), garantindo que o orquestrador agêntico mantenha latência determinística.

### 3. Métricas de Aceitação Termodinâmica de Áudio e Embeddings

A integridade dos micro-sidecars operando em CPU DEVE ser validada sob os seguintes limites operacionais:

1. **Fator de Tempo Real (TTS):**
   $$\text{RTF} = \frac{\text{Tempo de Processamento do Áudio (s)}}{\text{Duração Total do Áudio (s)}} \le 0,25$$
2. **Tempo até o Primeiro Áudio (TTFA):** $\text{TTFA} \le 150 \text{ ms}$ a partir do recebimento da primeira frase emitida pelo LLM.
3. **Latência de Embeddings:** $\text{Latência} \le 15 \text{ ms}$ por chunk de texto vetorizado na CPU.

## Consequências e Trade-offs

### Impactos Positivos:

- **VRAM Totalmente Livre para o LLM:** $100\%$ da memória de vídeo ($6.0 \text{ GB}$) fica reservada exclusivamente para os pesos do modelo de linguagem (Qwen 3.5 4B) e expansão da cache KV.
- **Zero Stuttering e Zero Jitter:** Síntese de voz fluida em tempo real sem engasgos causados por rajadas de computação da GPU.
- **Isolamento de Falhas (Fail-Isolate):** Um travamento ou pico de consumo no worker de áudio/embedding em CPU não afeta o pipeline de inferência da GPU nem o runtime principal em Rust.

### Impactos Negativos:

- **Carga Contínua na CPU Host:** Os núcleos isolados (ex: 0 a 3) operarão em pico durante a síntese de áudio ou reindexação RAG. Essa carga é absorvida pelo dimensionamento do processador e mitigada pela execução em lote de requisições de embeddings.

### Comportamento Fail-Closed

Se o daemon do SOULS detectar que um micro-sidecar foi instanciado sem a máscara de afinidade de núcleo válida ou se o worker tentar alocar contexto no dispositivo CUDA, o processo worker sofre `SIGKILL` imediato, gerando alerta crítico na tabela `SYSTEM_AUDIT`.
