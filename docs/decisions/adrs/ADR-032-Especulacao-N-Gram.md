---
id: "ADR-032"
title: "ADR-032-A-Guilhotina-da-Especulacao-Neural-e-Adocao-N-Gram"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Proíbe modelos de rascunho neurais (MTP/EAGLE-3) na VRAM e impõe especulação N-Gram na RAM do Host para hardware restrito de 6GB."
---

# ADR-032: A Guilhotina da Especulação Neural e Adoção N-Gram

## Status

Aceito (Ativo, Inegociável e Fundacional para SOULS V5)

## Contexto Técnico e Restrições de Hardware

A especulação de tokens e a predição multi-token (MTP) foram concebidas para contornar o gargalo de largura de banda de memória na fase autorregressiva de LLMs. No entanto, a implementação dessa técnica em hardware restrito como a dGPU NVIDIA RTX 2060m ($6.0 \text{ GB}$ VRAM, barramento PCIe Gen3 x8) impõe desafios térmicos e de memória severos:

1. **Modelos de Rascunho Neurais e Cabeças EAGLE-3/MTP (`draft-simple`, `draft-eagle3`):** Exigem a alocação de um segundo modelo neural na VRAM (ex: 0.5B a 1.5B parameters) ou cabeças de projeção conectadas aos hidden states.
2. **O Problema da Cache KV Dupla:** A especulação neural obriga o motor de inferência (`llama.cpp` / `mistral.rs`) a gerenciar **duas caches Key-Value independentes na VRAM** (uma para o modelo principal de 4B/7B e outra para o modelo de rascunho).
3. **Contenção no Barramento e Aceleração Negativa:** O consumo de VRAM estática pelos pesos do rascunho comprimem o headroom disponível para a KV Cache do modelo base. Além disso, a verificação paralela e o chaveamento constante de tensores no barramento GDDR6 resultam em razões de aceleração nulas ou **negativas** ($S < 1,0$), onde o sistema opera mais devagar do que a inferência monolítica simples, mesmo sob taxas de aceitação razoáveis ($\alpha > 70\%$).

## Declaração do Problema

Como obter aceleração na taxa de geração de tokens ($S > 1.0$) em estruturas altamente padronizadas (JSONs estruturados, chamadas de função, sintaxe de código) dentro do limite físico inflexível de $6.0 \text{ GB}$ de VRAM, sem consumir memória de vídeo adicional e sem induzir paginação para a memória RAM do sistema através do barramento PCIe?

## Decisões Arquiteturais da SOULS V5

```
                                [ PROMPT ENTRADA ]
                                         |
                                         v
                         [ Host RAM: Trie / Hash Table ]
                         (ngram-mod: Pegada ~16 MB RAM)
                                         |
                       (Rascunho de Sequências Repetitivas)
                                         |
                                         v
                         [ VRAM: LLM Principal (Qwen 3.5 4B) ]
                         (Verificação Paralela em 1 Passo)
                                         |
                                         v
                         [ Taxa de Aceleração S = 1.15x - 1.45x ]
                         [ VRAM Footprint Especulativo = 0 MB ]
```

### 1. A Guilhotina da Especulação Neural (`.mtp` / `draft-simple`)

Fica terminantemente **PROIBIDO** o carregamento de arquivos de rascunho neurais (`.gguf` secundários), cabeças MTP ou adaptadores EAGLE-3 (`.mtp`) na VRAM em qualquer componente da Arquitetura SOULS V5.

- Qualquer tentativa de inicializar o motor de inferência com a flag `--spec-type draft-simple` ou `-md <draft_model>` em dGPUs de 6GB é barrada no carregamento pelo Gateway Rust.
- O carregamento de rascunhos neurais introduz risco iminente de OOM e degradação térmica por desacoplamento de cache KV.

### 2. Adoção Compulsória da Especulação N-Gram (`ngram-mod`)

O SOULS V5 adota **exclusivamente** a especulação baseada em n-gramas (`ngram-mod`) para aceleração de geração autorregressiva.

- **Mecanismo:** O algoritmo `ngram-mod` analisa em tempo real as janelas de contexto geradas, construindo tabelas de hash dinâmicas baseadas na frequência de n-gramas (tamanho de correspondência min/max configurável).
- **Zero VRAM Footprint:** As tabelas de n-gramas residem estritamente na memória RAM central do sistema (pegada estática insignificante de $\sim 16 \text{ MB}$), sem alocar um único byte na VRAM da RTX 2060m.
- **Sem Concorrência de CUDA Stream:** Não há execução de redes neurais auxiliares. O GPU CUDA stream permanece $100\%$ dedicado à passagem direta do modelo de linguagem principal.

### 3. Parâmetros Canônicos de Execução (`llama-cli` / `mistral.rs` Bindings)

A invocação do motor de inferência para tarefas estruturadas DEVE utilizar os seguintes parâmetros de especulação n-gram:

```bash
llama-cli -m qwen3.5-4b-instruct-q4_k_m.gguf \
  -c 4096 \
  -ngl 999 \
  --spec-type ngram-mod \
  --spec-ngram-mod-n-match 24 \
  --spec-ngram-mod-n-min 48 \
  --spec-ngram-mod-n-max 64 \
  --perf \
  -p "<PROMPT_ESTRUTURADO>"
```

- **Métrica de Validação:** A flag `--perf` deve emitir relatórios contínuos de profilação. Se o speedup estatístico $S = \frac{\text{Tokens/s}_{\text{espec}}}{\text{Tokens/s}_{\text{base}}}$ cair abaixo de $1,0$ em tarefas específicas, o mecanismo especulativo é desativado dinamicamente via fallback para geração monolítica simples.

## Consequências e Trade-offs

### Impactos Positivos:

- **Preservação de VRAM:** Liberação imediata de $800\text{ MB}$ a $1.5\text{ GB}$ de VRAM que seriam desperdiçados por modelos de rascunho neurais.
- **Aceleração Termodinâmica Real:** Ganhos comprovados de $15\%$ a $45\%$ ($S = 1,15 \text{ a } 1,45$) na velocidade de saída para geração de JSONs, ASTs, SQL e schemas estruturados.
- **Simplicidade de Execução:** Eliminação da complexidade de sincronização de múltiplos modelos e artefatos de rascunho na esteira de deploy.

### Impactos Negativos:

- **Ineficiência em Prosa Não Estruturada:** Em tarefas de escrita criativa ou respostas abertas sem padrões repetitivos, a taxa de aceitação de n-gramas pode cair. No entanto, por ter custo zero de VRAM e processamento de hash insignificante na CPU, essa desativação virtual não impõe qualquer penalidade de OOM ou desaceleração do sistema.

### Comportamento Fail-Closed

Se o profiling NVML ou a telemetria do motor Rust detectar qualquer tentativa de alocação de modelo secundário na GPU durante a fase de especulação, o Gateway abortará o processo síncronamente via `SIGKILL`, registrando a violação no log de auditoria `FINOPS_TELEMETRY`.
