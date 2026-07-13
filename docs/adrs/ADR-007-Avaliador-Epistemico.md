---
id: "ADR-007"
title: "ADR-007-Avaliador-Epistemico"
version: 2.0
status: Ativo_Inegociavel
epic: "Cognição"
description: "Formaliza o Hipocampo Epistêmico via Logit Probing em Forward Pass. SLMs na CPU (AVX2) avaliam risco e ambiguidade em <150ms sem gerar texto."
---

### ADR-007: Avaliador Epistêmico, Logit Probing e o Hipocampo na CPU

#### Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SODA V4)

#### Contexto Técnico e o Gargalo do Raciocínio Reflexivo
Em sistemas agênticos, antes de delegar uma execução (ex: mutação de código, leitura pesada) ao modelo principal, a máquina precisa avaliar o "Risco Relacional", a "Ambiguidade" e a probabilidade da ação ser destrutiva [5].
Realizar essa triagem invocando o LLM primário na placa de vídeo gera um gargalo duplo letal:
1. Força a geração autorregressiva de texto (ex: forçar a IA a responder "SIM" ou "NÃO" passo a passo), o que drena recursos e tempo calculando a decodificação sequencial de *tokens* [4].
2. Concorre diretamente pela VRAM crítica (6GB) da dGPU RTX 2060m e asfixia o barramento PCIe, criando um severo "Flow-Debt" termodinâmico [1].

#### Decisão Arquitetural (O Hipocampo Híbrido O(1))
Fica decretada a separação física e matemática entre a "Ação" (GPU) e a "Avaliação Rápida" (CPU). O SODA institui a figura do "Hipocampo Epistêmico", operando sob as seguintes leis imutáveis:

**Módulo 1: O Fim da Geração Autorregressiva para Triagem**
*   É sumariamente proibido usar ciclos de decodificação para classificar se um comando inicial é seguro, ambíguo ou necessita de intervenção [4].
*   O motor de avaliação de risco deve interromper a sua operação de forma compulsória no exato nanossegundo em que a fase de *prefill* (leitura em bloco do *prompt*) é concluída [4].

**Módulo 2: Extração Numérica via Logit Probing (Forward Pass)**
*   O sistema implementará o paradigma *ProbeLogits* [4].
*   No final do *Forward Pass* único, o núcleo Rust lerá diretamente a matriz de distribuição de *logits* residindo na memória RAM [4].
*   Medindo o delta da probabilidade estrita de *tokens* âncora (ex: calculando a diferença angular/matemática de ativação entre os *logits* mapeados para risco vs. segurança), o SODA extrai a certeza da IA de forma determinística, sem nunca decodificar uma string.

**Módulo 3: Isolamento Físico na CPU (AVX2)**
*   Este motor validador será suportado por SLMs ultra-quantizados (ex: Phi-4-mini ou famílias hiper-leves). Eles estão terminantemente proibidos de tocar na placa de vídeo [6].
*   A execução dessas matrizes matemáticas ocorrerá unicamente na CPU (Intel i9) utilizando o poder intrínseco das instruções vetoriais **AVX2** [2, 3].
*   Para prover acesso direto aos ponteiros da memória de *logits*, o SODA fará o uso cirurgicamente algemado de bibliotecas focadas em *prefill* (ex: via injeção C-FFI de `llama-cpp-4` contida ou suporte especializado), assegurando extrações atômicas [2, 6, 7].
*   O limite máximo de latência orçamentado para esse "reflexo epistêmico" é estritamente **< 150ms** [2, 6].

#### Consequências Operacionais e Defesa contra o Slop (Trade-offs)
*   **Impacto Positivo:** Desoneração de 100% da dGPU (RTX 2060m) durante os processos de triagem e *guardrails* [1]. A resposta do sistema perante intenções hostis ou ambíguas adquire latência sobre-humana, abortando ciclos inúteis de processamento longo em menos de 150 milissegundos e protegendo o motor de inferência massiva [2, 5].
*   **Impacto Negativo (Complexidade C-FFI e Dados):** A manipulação direta de tensores de saída introduz o risco do gerenciamento C-FFI (`llama-cpp-4`) no Rust, obrigando contenção paranoica contra *segfaults* para não contaminar o Tokio [7]. Adicionalmente, este módulo exigirá no futuro a aplicação do Epic 7.1, demandando o preparo artesanal de *Golden Datasets* e *Fine-Tuning* de adaptadores LoRA direcionados estritamente ao comportamento dessas respostas matemáticas de probabilidade [8, 9].
