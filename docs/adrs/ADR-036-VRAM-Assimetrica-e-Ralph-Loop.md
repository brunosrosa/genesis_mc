---
id: "ADR-036"
title: "ADR-036-VRAM-Assimetrica-e-Ralph-Loop-Sequencial"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Proíbe amostragem paralela (Best-of-N) na GPU e impõe o Ralph Loop Sequencial com Critic desacoplado em CPU RAM e KV-Cache Prefix Reuse."
---

# ADR-036: VRAM Assimétrica e Ralph Loop Sequencial

## Status

Aceito (Ativo, Inegociável e Fundacional para SOULS V5)

## Contexto Técnico e Gargalo da KV Cache Paralela

O paradigma de *Test-Time Compute* (TTC) demonstra que atribuir tempo adicional de raciocínio no momento da inferência eleva a capacidade lógica de modelos compactos. No entanto, em literatura de nuvem, o TTC é frequentemente implementado por meio de amostragem paralela (ex: *Best-of-N*, *Self-Consistency*, *Majority Voting*), onde $N$ sequências concorrentes são geradas e avaliadas simultaneamente.

Em hardware restrito como a dGPU NVIDIA RTX 2060m ($6.0 \text{ GB}$ VRAM):

1. **Multiplicação da KV Cache:** Gerar $N$ caminhos paralelos multiplica a alocação de memória da KV Cache por um fator $N$. Para um contexto de 4096 tokens, $N=4$ exige $>4.4\text{ GB}$ apenas para tensores de atenção, estourando instantaneamente o teto físico da VRAM e forçando paginação para a RAM via PCIe.
2. **Degradação Térmica:** O descarte das $N-1$ respostas rejeitadas desperdiça ciclos de GPU e energia térmica em execuções redundantes.

## Declaração do Problema

Como aplicar *Test-Time Compute* e ciclos de reflexão de auto-correção para resgatar erros lógicos complexos em SLMs de 3B a 4B parâmetros sem estourar o limite de 6GB de VRAM, sem induzir paginação PCIe e sem desacelerar a inferência com re-prefills integrais?

## Decisões Arquiteturais da SOULS V5

```
                                [ PROMPT INICIAL ]
                                         |
                                         v
                         [ VRAM (GPU): Modelo Primário 4B ]
                         (KV Cache Prefix Preservado)
                                         |
                                         v
                         [ Resposta Preliminar Emitida ]
                                         |
                                         v
                         [ RAM Host (CPU AVX2): Critic ]
                         (Zero VRAM | Validador Desacoplado)
                                         |
                       +-----------------+-----------------+
                       |                                   |
                (Aprovado: Sucesso)                (Rejeitado: Erro)
                       |                                   |
                       v                                   v
               [ Output Final ]                 [ Traço de Erro Sintetizado ]
                                                           |
                                                           v
                                                [ Reinjeção de Contexto ]
                                                (KV Cache Prefix Reuse)
```

### 1. Proibição Absoluta de Amostragem Paralela (`Best-of-N`)

Fica terminantemente **PROIBIDO** o uso de estratégias de amostragem paralela (*Best-of-N*, *Self-Consistency*, *Beam Search* paralelo) que aloquem múltiplas sequências de KV Cache simultâneas na VRAM da GPU.

- O motor de inferência SOULS V5 operará sob amostragem estritamente sequencial.

### 2. O Padrão "VRAM Assimétrica e Ralph Loop Sequencial"

O SOULS V5 padroniza a execução do **Ralph Loop Sequencial**, governado por uma arquitetura de VRAM assimétrica:

- **Modelo Primário (3B-4B):** Alocado exclusivamente na VRAM da GPU (~2.5GB a 3.2GB), encarregado da geração rápida de hipóteses e raciocínio.
- **Modelo Verificador (*Critic Desacoplado*):** Fica estritamente **proibido** alocar o modelo *Critic* na VRAM. O verificador (seja um validador heurístico determinístico em Rust ou um SLM ultraleve como `GLiClass` / `Phi-4-mini` de 0.5B-1.5B em GGUF) DEVE rodar exclusivamente na memória RAM do Host, processado via CPU através de instruções vetoriais AVX2 / AVX-512.

### 3. Reutilização Obrigatória de Prefixos de KV Cache (`Prefix Reuse`)

Quando o *Critic* na CPU detectar uma falha semântica ou quebra de invariante de negócio, a auto-correção DEVE seguir o protocolo de **KV Cache Prefix Reuse**:

1. O traço do erro é formatado sinteticamente e anexado ao final do contexto.
2. O Gateway Rust envia a nova tentativa reaproveitando os tensores de atenção já computados do prompt original na VRAM.
3. É terminantemente **PROIBIDO** zerar o buffer da KV Cache ou refazer o *prefill* dos tokens iniciais, garantindo que o custo computacional da re-geração se limite estritamente aos novos tokens de resposta e correção.

## Consequências e Trade-offs

### Impactos Positivos:

- **Imunidade a OOM de VRAM:** O consumo da KV Cache permanece de complexidade $\mathcal{O}(1)$ em relação ao paralelismo de amostragem.
- **Eficiência de CPU/GPU:** A GPU executa apenas o modelo principal; a CPU absorve a validação sem afetar os registradores CUDA.
- **Aumento Cognitivo:** Elevação de $+20\%$ a $+35\%$ na taxa de sucesso de tarefas lógicas e agênticas difíceis sem alterar o peso do modelo base.

### Impactos Negativos:

- **Latência Linear por Passo de Reflexão:** Cada ciclo de reflexão sequencial adiciona a latência da re-geração de tokens. Isso é mitigado pelo limite rígido de no máximo **3 iterações de reflexão** por tarefa (Fail-Closed).

### Comportamento Fail-Closed

Se o Ralph Loop atingir a 3ª iteração sem obter aprovação do *Critic*, a execução é abortada, descarregando o contexto de reflexão e registrando o snapshot na tabela `SYSTEM_AUDIT` para auditoria do usuário no Agent Inbox (HITL).
