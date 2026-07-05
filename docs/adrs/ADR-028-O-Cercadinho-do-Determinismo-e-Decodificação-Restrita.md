---
aliases:
  - "ADR-028: O Cercadinho do Determinismo e Decodificação Restrita"
---

# ADR-028: O Cercadinho do Determinismo e Decodificação Restrita

## Status

Aceito (Ativo, Inegociável e Fundacional para SODA V4)

## Contexto Técnico e Impacto da Quantização

Sob as restrições térmicas e de memória de vídeo fixadas no [ADR-027](https://gemini.google.com/app/ADR-027-Motor-Hibrido-VRAM.md "null") (NVIDIA RTX 2060m de $6.0 \text{ GB}$ de VRAM), o SODA V4 adota o regime de quantização extrema `IQ3_M` via matrizes de importância (`imatrix`) para modelos de linguagem de escala $8\text{B}$ de parâmetros.

A compressão dos pesos para o patamar de $\approx 3.66 \text{ bpw}$ (bits por peso) acarreta uma degradação severa na coerência lógica fina do modelo:

- **Colapso de Schema JSON:** O modelo perde a capacidade natural de seguir sintaxes rígidas em regime nativo livre (_Zero-Shot_), falhando no fechamento de delimitadores estruturais (chaves `{}` e colchetes `[]`), escapando caracteres de controle ou gerando chaves inexistentes no esquema de dados de 82 colunas da tabela `MASTER_SOLUTIONS`.
- **Estresse de Desserialização no Rust Backend:** Interfaces de acoplamento direto baseadas na crate `serde_json` falham catastroficamente (`panic` ou interrupção do thread de controle do Tokio) ao receber strings JSON malformadas [DEPENDENCIES] Otimização de Inferência Rust Bare-Metal SLMs e Decodificação Restrita (Fase de Destilação).md, uploaded:SODA Theme_16].
- **Sobrecarga de Recuperação Lógica (Anti-Patterns Banned):** Rejeita-se categoricamente a re-execução de prompts síncronos na dGPU para "correção de JSON" ou laços de validação sintática complexos que sobrecarreguem o barramento PCIe ou gerem novos picos de latência.

## Declaração do Problema

Como garantir $100\%$ de conformidade sintática à especificação de saída JSON para um modelo $8\text{B}$ quantizado em `IQ3_M`, processando contextos de até $30.000$ tokens, sem introduzir overhead térmico na dGPU e sem expor a esteira de I/O em Rust a exceções de tempo de execução [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md, uploaded:SODA Fase 0: Harvester Nativo e Fluxo Canônico V6.0]?

## Decisões Arquiteturais da SODA V4

```
                                  VETOR DE LOGITS BRUTOS
                                            |
                                            v
                              [  dGPU: llama-cpp-2 Engine  ]
                                            |
                                            v
                              [ Amostrador Min-P (0.05-0.08) ]
                                            |
                                            v
                     ===============================================
                       CONTROL PLANE (CPU i9): MASCARAMENTO AVX2
                     ===============================================
                     |                                             |
                     |  llguidance Parser (Context-Free Grammar)   |
                     |  - Varre o estado sintático do token        |
                     |  - Compara com a árvore AST do JSON         |
                     |  - Executa filtragem paralela SIMD (<50us)   |
                     |                                             |
                     ===============================================
                                            |
                         Aplica logits_processor: mask -> -∞
                                            |
                                            v
                               TOKEN DETERMINÍSTICO SELECIONADO
                                            |
                                            v
                            [  Tokio Event Loop: serde_json ] -> Sem Panics!
```

### 1. A Algema da CPU via `llguidance` e Vetorização AVX2

Bane-se qualquer processamento de máscaras de Gramática Livre de Contexto (CFG - _Context-Free Grammar_) ou inferência lógica de ordenação estrutural na dGPU NVIDIA RTX 2060m.

- O SODA V4 utiliza a crate Rust `llguidance` integrada diretamente ao loop de geração sínclono do `llama-cpp-2`.
- A validação de tokens e o avanço dos autômatos determinísticos de gramática são descarregados integralmente para os núcleos de alta frequência da CPU (Intel Core i9).
- A CPU intercepta o vetor de logits brutos retornado pela dGPU antes da seleção de amostragem. Emprega-se paralelismo SIMD nativo através de instruções vetoriais AVX2 de 256 bits (`is_x86_feature_detected!("avx2")`) para escanear e invalidar de forma concorrente a tabela de probabilidade do vocabulário do tokenizer.

#### Operação Matemática de Constraint de Logits:

Seja $L_{\text{raw}} \in \mathbb{R}^{V}$ o vetor de logits brutos para um vocabulário de tamanho $V$. A CPU calcula uma máscara booleana $M_{\text{CFG}} \in \{0, 1\}^{V}$ baseada no estado atual do autômato JSON em tempo de execução. O vetor modificado de logits $L_{\text{constrained}}$ é computado na CPU por:

$$L_{\text{constrained}, i} = \begin{cases} L_{\text{raw}, i} & \text{se } M_{\text{CFG}, i} = 1 \\ -\infty & \text{se } M_{\text{CFG}, i} = 0 \end{cases}$$

- **Headroom de Tempo:** A interceptação, o mascaramento de gramática por autômatos e a injeção do vetor corrigido na esteira de geração devem operar em um teto máximo e estrito de $50 \text{ microssegundos}$ **(**$\le 50\mu\text{s}$**)** por token, tornando a operação indetectável perante a latência de decodificação geral.

### 2. Supressão de Ruído de Cauda via Amostragem Min-P

Fica terminantemente proibido o uso isolado ou prioritário de amostradores baseados em contagem linear estocástica acumulada ou truncamentos estáticos de tamanho (tais como `Top-P` e `Top-K`).

A compressão severa de pesos em `IQ3_M` achata artificialmente as curvas de probabilidade calculadas pela camada Softmax, elevando scores estatísticos de tokens espúrios (lixo gramatical) à vizinhança imediata dos tokens corretos [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].

- O SODA V4 impõe o amostrador **Min-P** como o disjuntor estatístico primário do motor gerador [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].
- O Min-P atua podando de forma adaptativa e proporcional todos os tokens cuja probabilidade seja inferior a um limite dinâmico calculado a partir do token de maior probabilidade ($p_{\text{max}}$) [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].

#### Equação do Limiar de Poda Estocástica:

$$p_{\text{threshold}} = p_{\text{max}} \times m_{\text{min\_p}}$$

- O coeficiente de escala de amostragem é fixado no intervalo estrito de:

    $$m_{\text{min\_p}} \in [0.05, 0.08]$$
- Qualquer token $i$ cuja probabilidade normalizada satisfaça $p_i < p_{\text{threshold}}$ é sumariamente eliminado do espaço de busca [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md]. Esta compressão impede que o gerador de gramática CPU gaste ciclos processando logits de alta entropia decorrentes da quantização destrutiva da GPU [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].

### 3. Ancoragem de Atenção por In-Context Learning (Few-Shot Rígido)

O modelo hiper-quantizado operando em contexts longos ($30\text{k}$ tokens) sofre de degradação rápida no vetor de atenção espacial [DEPENDENCIES] Otimização de Inferência Rust Bare-Metal SLMs e Decodificação Restrita (Fase de Destilação).md]. Forçar o modelo a deduzir a semântica da resposta em regime _Zero-Shot_ consome recursos cognitivos residuais escassos dos pesos comprimidos [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].

- O SODA V4 institui a obrigatoriedade da **Ancoragem Dupla de Prompt com ICL (In-Context Learning)** [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].
- Toda requisição enviada ao motor de inferência pela Fase 1.5 e Fase 3 de processamento deve conter, de forma imutável no topo de seu harness de prompt, exatamente **2 exemplos perfeitos de Entrada/Saída JSON** [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].
- O modelo é induzido a atuar puramente por mimetismo geométrico e ativação espacial de padrões preexistentes na janela de contexto [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].
- Essa simplificação cognitiva economiza a capacidade de representação do modelo quantizado, focando $100\%$ de seu poder de atenção exclusivamente na _extração semântica_ das propriedades do código local analisado, enquanto o plano de controle de Rust cuida da rigidez estrutural externa.

## Consequências e Trade-offs

### Impactos Positivos:

- **Conformidade Estrutural Absoluta (**$100\%$**):** Erradicação completa de falhas de desserialização e panics sistêmicos na camada `serde_json` no backend Rust [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].
- **Isenção de Carga na GPU:** Desoneração total da placa de vídeo dedicada RTX 2060m quanto a processamentos lógicos de gramática, preservando recursos computacionais estritamente para aceleração matricial de tensores e exibição de tela do host.
- **Previsibilidade Operacional:** Mitigação de desvios e comportamentos estocásticos indesejados (_schema drift_) decorrentes do achatamento de probabilidades gerado pela compressão `IQ3_M` [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md].

### Impactos Negativos:

- **Aumento do Prompt Base:** A injeção síncrona dos 2 exemplos do Few-Shot no Harness do prompt consome cerca de $350 \text{ a } 500 \text{ tokens}$ adicionais de entrada fixos [DEPENDENCIES] Otimização Inferência Rust Bare-Metal e FinOps Local.md]. Aceita-se esta penalidade em favor da economia cognitiva total e precisão de saída do modelo.
- **Sobrecarga de CPU:** O uso contínuo de instruções SIMD AVX2 na CPU consome ciclos de processamento do processador Core i9. Contudo, a CPU possui orçamento térmico suficiente e opera em regime assíncrono sobre as threads de controle, sem travar o laço de eventos principal do Tokio [Epic 07] Arquitetura SLM Rust para Avaliação Epistêmica.md].