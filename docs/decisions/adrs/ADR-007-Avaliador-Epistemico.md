---
id: "ADR-007"
title: "ADR-007-Avaliador-Epistemico"
version: 2.0
status: Ativo_Inegociavel
epic: "Cognição"
description: "Formaliza o Hipocampo Epistêmico via Logit Probing em Forward Pass e decodificação restrita via llguidance em CPU (AVX2). Estabelece a Mesa de Triagem de SLMs, amostragem Min-P e Few-Shot Rígido."
---

### ADR-007: Avaliador Epistêmico, Logit Probing, Decodificação Restrita e Hipocampo CPU

#### Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SOULS V6)

#### Contexto Técnico e a Falência da Decodificação Livre na Triagem
Em sistemas agênticos de alta performance, delegar triagem, verificação de risco ou extração sintática diretamente ao modelo principal na GPU via geração de texto livre é uma falha arquitetural grave. Essa prática induz a dois gargalos letais:
1. **Flow-Debt e Entropia de Tokens:** Forçar o modelo a decodificar tokens autorregressivos (ex.: gerar prosas para responder "SIM" ou "NÃO") consome ciclos computacionais valiosos e introduz alucinações sintáticas inevitáveis.
2. **Asfixia de VRAM e PCIe:** Concorre diretamente pelos 6GB de VRAM da dGPU RTX 2060m e congestiona o barramento PCIe, elevando a latência e correndo risco de estouro térmico ou OOM.

#### Decisão Arquitetural (O Hipocampo Epistêmico O(1) e a Mesa de Triagem)
Fica decretada a separação física, espacial e matemática entre a Ação (GPU) e a Avaliação Epistêmica (CPU Host). O SOULS estabelece o Hipocampo Epistêmico governado pelas seguintes leis constitucionais imutáveis:

##### 1. Taxonomia Unificada de Modelos e Mesa de Triagem (SPEC-013)
A infraestrutura de inferência classifica e isola os modelos em 4 camadas de capacidades estritas:
*   **Encoders / Micro-Models (< 500M):** Executados estritamente na CPU Host (AVX2). Destinados a extração $O(1)$, NLU, NER e classificação inicial de risco (ex.: GliClass). Latência alvo $< 5\text{ms}$.
*   **Small Language Models - SLM (1B a 8B):** Hospedados na GPU Local (4-8GB VRAM) ou RAM. Responsáveis por Live Chat, REPL local, chamadas de ferramentas e raciocínio de borda.
*   **Intermediate LLM - Mid-LLM (14B a 70B):** Alocados em workstations de memória unificada para processamento local denso e análise profunda offline.
*   **Frontier / Massive LLMs (100B a 2.3T+):** Oráculos externos na nuvem acessados estritamente por exceção (Spill-over FinOps) para destilação sintética e resolução de impasses epistêmicos.

##### 2. Decodificação Restrita Assíncrona via `llguidance` em CPU (AVX2)
*   Todo processamento de triagem e extração de dados estruturados na CPU (Intel i9) utiliza **Constrained Decoding** via engine `llguidance` alocada no núcleo Rust.
*   A engine intercepta e mascara a matriz de logits em hardware, impedindo fisicamente a emissão de qualquer token fora da gramática estrita (JSON Schema / Notação LEAN).
*   A latência de decodificação restrita por token é limitada deterministicamente ao teto de **50 microssegundos por token (50 µs/token)**.

##### 3. Amostragem Estatística Min-P Inegociável
*   Na esteira de inferência do `LlamaCpp4LogitEngine` (Tier 0.5 na CPU), fica imposta a obrigatoriedade do amostrador estatístico **Min-P** ($\text{Min-P} \ge 0.05$).
*   O Min-P atua como barreira inegociável para cortar a cauda longa de probabilidade e mitigar o achatamento de distribuição gerado pela quantização IQ3_M do modelo local, garantindo que apenas tokens com massa de probabilidade relativa real sejam considerados no mascaramento de logits.

##### 4. Regra do Few-Shot Rígido (2 Exemplos Limpos)
*   É proibido submeter prompts de extração ou classificação zeroshot estocásticos.
*   Toda requisição ao Hipocampo Epistêmico deve obrigatoriamente injetar **exatamente 2 exemplos limpos (Few-Shot Rígido)** no prompt de grounded extraction, alinhando a atenção do modelo e erradicando desvios comportamentais.

##### 5. Extração Numérica via Logit Probing (Forward Pass)
*   Para verificações de risco binário, o sistema adota o paradigma *ProbeLogits*.
*   O motor interrompe a decodificação no final do *Forward Pass* inicial (fase de *prefill*).
*   O núcleo Rust lê o delta da distribuição angular de logits dos tokens âncora diretamente na RAM Host, calculando a certeza matemática em $< 150\text{ms}$ sem decodificar prosas.

#### Consequências Operacionais e Trade-offs
*   **Impacto Positivo:** Desoneração de 100% da dGPU (RTX 2060m) durante processos de triagem e guardrails. Eliminação de alucinações sintáticas via `llguidance` (50 µs/token) e blindagem de cauda via Min-P.
*   **Impacto Negativo:** Exige manutenção paranoica dos bindings C-FFI do `llama-cpp-4` no Tokio para evitar segfaults na RAM Host, além de manter o dataset de 2 exemplos Few-Shot rigorosamente higienizado.
