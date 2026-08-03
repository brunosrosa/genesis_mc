---
id: "ADR-034"
title: "ADR-034-Injecao-de-Vetores-de-Controle-RepE-em-Runtime"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Impõe o uso de Engenharia de Representação (RepE) via interceptação de cb_eval para modulação de ativação em camadas intermediárias."
---

# ADR-034: Injeção de Vetores de Controle (RepE) em Runtime

## Status

Aceito (Ativo, Inegociável e Fundacional para SOULS V5)

## Contexto Técnico e Restrições de Quantização

A execução de Small Language Models (SLMs de 3B a 4B parâmetros, como Qwen 3.5 4B) em hardware de consumo com 6GB de VRAM exige a aplicação de quantização agressiva em bloco (`Q4_K_M` ou `IQ3_M`). Embora a quantização reduza a pegada estática de memória de ~9GB para ~2.5GB de VRAM, ela introduz perturbações numéricas nos tensores de pesos:

1. **Distorção de Trajetória de Atenção:** Pequenos erros de arredondamento nos pesos quantizados alteram a projeção dos tensores de ativação intermediários ($h_l$) ao longo das camadas do Transformer.
2. **Surtos de Recusas Indevidas e Alucinações:** A degradação estatística faz com que o modelo perca o rastreio de regras complexas de sistema, levando a alucinações sintáticas, perda de contexto agêntico ou recusas indevidas (*refusal trigger*).
3. **Inviabilidade de Fine-Tuning Tradicional:** Re-treinar ou aplicar fine-tuning pesado sobre o modelo base para corrigir desvios sintáticos/comportamentais exige infraestrutura massiva de GPU e cria múltiplos checkpoints gigantescos, inviabilizando a flexibilidade de manutenção no ecossistema local.

## Declaração do Problema

Como recuperar a acurácia comportamental, suprimir alucinações e alinhar a execução de um SLM 4B quantizado aos níveis de um modelo de 7B/8B sem aumentar o footprint de VRAM, sem re-treinar os pesos base e com impacto insignificante no throughput de inferência?

## Decisões Arquiteturais da SOULS V5

```
                                [ PROMPT ENTRADA ]
                                         |
                                         v
                      [ Tensor de Entrada h_l (Camada l) ]
                                         |
                                         v
                         [ llama.cpp / mistral.rs cb_eval ]
                         (Interceptação de Callback em Rust)
                                         |
                         h_l' = h_l + alpha * v_l (GGUF Vector)
                         (alpha in [0.15, 0.6] | Layers 10-24)
                                         |
                                         v
                      [ Projeção de Ativação Modulada h_l' ]
                                         |
                                         v
                        [ Saída Fluida & Livre de OOM ]
```

### 1. Interceptação em Runtime via Callbacks (`cb_eval`)

O SOULS V5 impõe o uso de **Engenharia de Representação** (*Representation Engineering* - RepE) para modulação de ativação em tempo de execução.

- O backend de inferência em Rust (`llama-cpp-2` / `mistral.rs` bindings) DEVE registrar um manipulador na função de callback de avaliação de tensores (`cb_eval`).
- A cada passagem direta (*forward pass*), o manipulador intercepta os tensores de ativação ocultos das camadas intermediárias ($h_l$) e injeta um vetor de deslocamento comportamental ($v_l$) derivado de um arquivo de vetor de controle em formato GGUF:

$$h_l' = h_l + \alpha \cdot v_l$$

### 2. Parâmetros e Janelas de Camadas Estritas

- **Faixa de Camadas de Interceptação:** A injeção de vetores de controle DEVE ocorrer exclusivamente nas **camadas intermediárias (faixa de 10 a 24)** em modelos de 32 camadas.
  - *Proibição:* É terminantemente **PROIBIDO** aplicar vetores de controle nas primeiras camadas (0 a 9), sob pena de corromper a extração de primitivas sintáticas e léxicas básicas.
- **Fator de Escala Dinâmico ($\alpha$):**
  - Para modelos densos (ex: Qwen 3.5 4B), o fator de escala DEVE ser parametrizado no intervalo $\alpha \in [0,15; 0,60]$.
  - Para arquiteturas Mixture-of-Experts (MoE), o fator de escala fica restrito ao intervalo $\alpha \in [0,01; 0,05]$.

### 3. Proibição de Fine-Tuning Pesado para Correção Comportamental

Fica expressamente **PROIBIDO** instanciar rotinas de fine-tuning (LoRA pesado ou full fine-tuning) com o propósito exclusivo de alinhar regras de raciocínio passo a passo ou suprimir recusas indevidas em modelos base. A modulação comportamental DEVE ser tratada como um *sidecar* computacional em tempo de execução via vetores de controle RepE em formato GGUF (tamanho $< 1\text{ MB}$).

## Consequências e Trade-offs

### Impactos Positivos:

- **Impacto Nulo na VRAM:** O vetor de controle GGUF ocupa menos de $1\text{ MB}$ de memória e adiciona zero peso à GPU.
- **Recuperação Cognitiva Instantânea:** Aumento de $+10\%$ a $+20\%$ na consistência lógica e alinhamento a regras agênticas complexas.
- **Desempenho de Inferência:** A adição de vetor em C++/Rust via AVX2/CUDA tem sobrecarga irrisória ($< 2\%$ de impacto no throughput de tokens).

### Impactos Negativos:

- **Necessidade de Profiling de Fator $\alpha$:** Exige a calibração prévia da escala $\alpha$ por família de modelo para evitar distorções semânticas se $\alpha$ for configurado acima do teto de estabilidade.

### Comportamento Fail-Closed

Se o callback `cb_eval` falhar ao aplicar o vetor ou se o fator $\alpha$ induzir divergência numérica (detectada por NaNs nos logits), o Gateway reduzirá $\alpha$ instantaneamente para $0.0$ (fallback monolítico) e emitirá um log de alerta na tabela `SYSTEM_TELEMETRY`.
