---
id: "ADR-027"
title: "ADR-027: Arquitetura de Motor de Inferência Híbrido Isolado e Termodinâmica da VRAM"
version: 1.0
status: Ativo_Inegociavel
epic: "Cognição"
description: "Governa a termodinâmica da VRAM de 6GB, limitando o spillover de PCIe com micro-SLMs quantizados locais."
---

# ADR-027: Arquitetura de Motor de Inferência Híbrido Isolado e Termodinâmica da VRAM

## Status

Aceito (Ativo, Inegociável e Fundacional para SOULS V4)

## Contexto Técnico e Restrições de Silício

A Arquitetura SOULS V4 opera sob limitações físicas severas e fixas no hardware host do usuário final:

- **Unidade de Processamento Gráfico (dGPU):** NVIDIA RTX 2060m contendo exatamente $6.0 \text{ GB}$ ($6144 \text{ MB}$) de VRAM física dedicada.
- **Largura de Banda do Barramento:** Conexão móvel limitada via barramento PCIe Gen3 x8. Qualquer transferência dinâmica de pesos (offloading) ou troca de tensores entre a RAM do sistema e a VRAM durante a fase autorregressiva (decodificação) degrada a performance de geração, reduzindo a taxa de processamento para menos de $2 \text{ a } 8 \text{ tokens/s}$.
- **Unidade de Processamento Central (CPU):** Intel Core i9 auxiliada por $32 \text{ GB}$ de memória RAM física do sistema.
- **Requisito de UX Cognitiva:** Operadores neurodivergentes (2e/TDAH) exigem latência sub-milisegundo no eco das teclas e feedback mecânico instantâneo (< 100ms). Flutuações na latência de geração decorrentes de ciclos estocásticos de Garbage Collection (GC) ou travamento do loop de eventos são consideradas falhas críticas do sistema.

## Declaração do Problema

O processamento de contextos longos em regime de análise de repositórios locais (janelas de contexto de até $30.000$ tokens) impõe duas barreiras físicas intransponíveis caso se utilize um motor de inferência monolítico e persistente:

1. **A Assincronia do Prefill versus Decode:** A fase de ingestão primária (_prefill_) exige processamento massivo em paralelo para devorar documentos longos e árvores sintáticas. O motor `llama-cpp-2` (C++ nativo encapsulado em Rust) falha gravemente nesta etapa, operando em baixa eficiência de pipeline sob prompts longos. Por outro lado, runtimes de ultra-velocidade como `mistral.rs` ( CUDA kernels fundidos e Flash Attention 2) alcançam até $937 \text{ tps}$ em prefill, mas sofrem de severos vazamentos de memória virtual sob payloads dinâmicos prolongados, inviabilizando a sua persistência em background como daemon 24/7.
2. **A Inflação Linear do KV Cache:** Em janelas de contexto longas ($30\text{k}$ tokens), a retenção tradicional de chaves e valores (Key-Value) em precisão de 16-bits (FP16 ou BF16) consome espaço de armazenamento de vídeo que excede a capacidade física da RTX 2060m.

### Equação de Asfixia Térmica e Transbordo de VRAM:

A alocação de memória de vídeo de um modelo de $8\text{B}$ de parâmetros sem compressão estrita do contexto de dados é descrita por:

$$V_{\text{total}} = V_{\text{pesos}} + V_{\text{kv}} + V_{\text{runtime\_overhead}}$$

Para um modelo LLM padrão de $8\text{B}$ parametrizado com arquitetura de atenção baseada em Grouped-Query Attention (GQA) contendo $N_{\text{layers}} = 32$, $N_{\text{kv\_heads}} = 8$, dimensão de cabeça $D_{\text{head}} = 128$ e comprimento de contexto $L = 30.000$ tokens:

#### Cenário FP16 (Sem quantização de cache):

$$V_{\text{kv\_fp16}} = 2 \times N_{\text{layers}} \times N_{\text{kv\_heads}} \times D_{\text{head}} \times L \times 2 \text{ bytes}$$$$V_{\text{kv\_fp16}} = 2 \times 32 \times 8 \times 128 \times 30.000 \times 2 \approx 3.93 \text{ GB}$$

Se os pesos do modelo compactados ocuparem $3.78 \text{ GB}$ ($V_{\text{weights\_IQ3\_M}}$) e o runtime exigir $200 \text{ MB}$ de VRAM fixa para contexto CUDA:

$$V_{\text{total}} = 3.78 \text{ GB} + 3.93 \text{ GB} + 0.20 \text{ GB} = 7.91 \text{ GB}$$

Como $7.91 \text{ GB} > 6.0 \text{ GB}$, o sistema sofre transbordo imediato para o barramento PCIe, forçando o driver a realizar paginação no disco/RAM, congelando a interface e derrubando o sistema por esgotamento de recurso de hardware.

## Decisões Arquiteturais da SOULS V4

Para romper as limitações de hardware sem comprometer a soberania local e o desempenho térmico da máquina host, a Arquitetura SOULS V4 adota as seguintes regras de implementação de baixo nível:

```
+-------------------------------------------------------------------------+
|                         SOULS V4 - DUAL ENGINE LAYOUT                    |
+-------------------------------------------------------------------------+
|                                                                         |
|  [Fase 1.5 - Destilação]                                                |
|  mistral.rs (CUDA, Flash Attention 2)                                   |
|   |--> Prefill Massivo (~937 tps) em Payload de 30k                     |
|   |--> Cospe estruturado JSON localmente                                |
|   |--> [SINAL SÍNCRONO: SIGKILL]                                        |
|        |                                                                |
|        v (VRAM descarregada instantaneamente para 0 MB)                 |
|                                                                         |
|  [Fase 2 - Geração Perene]                                              |
|  llama-cpp-2 (Core)                                                     |
|   |--> Headroom de Hardware Segurado (V_total <= 4.8 GB)                |
|   |--> Modelo 8B quantizado via imatrix em IQ3_M (~3.78 GB)             |
|   |--> KV Cache comprimido síncronamente em Q4_K (~937.5 MB)            |
|   |--> Alocação estática mmap virtual contígua (O(1) no Heap)           |
|                                                                         |
+-------------------------------------------------------------------------+
```

### 1. A Dualidade de Motores com Descarte Atômico

Bane-se o uso de daemons de inferência persistentes e instáveis na GPU. SOULS V4 adota um pipeline híbrido composto por dois motores distintos operando em tempos de vida mutuamente exclusivos:

- **O Trator de Ingestão de Alta Voltagem (`mistral.rs`):** Instanciado em tempo de execução de forma estritamente efêmera e isolada. O orquestrador em Rust (`souls-etl-worker`) dispara o binário `mistral.rs` como um subprocesso filho focado exclusivamente em digerir e gerar as essências da Fase 1.5. Beneficia-se da velocidade de kernels CUDA e Flash Attention 2 nativo para processar prompts colossais de código em menos de $40$ segundos.
- **A Guilhotina Atômica do Sistema:** Imediatamente após a conclusão da extração sintática e entrega do JSON estruturado ao SQLite pela Fase 1.5, o orquestrador despacha um sinal síncrono do sistema (`SIGKILL`) ao processo do `mistral.rs`. A memória física da placa de vídeo dedicada é descarregada à força e retorna para $0 \text{ MB}$, expurgando vazamentos residuais e limpando os registradores para a próxima etapa.
- **O Cérebro Contínuo Baseado em C++ (`llama-cpp-2`):** Responsável por guiar a interface, executar a decodificação de logit-probing e as ações geradoras de feedback em tempo real. O `llama-cpp-2` carrega modelos utilizando memória virtual mapeada (`mmap`), garantindo previsibilidade determinística e alocação estática e contígua em tempo $O(1)$.

### 2. Imposição Matemática de Limites de KV Cache (Quantização Casada)

Para assegurar o regime de estabilidade operacional contínua na dGPU RTX 2060m, o SOULS V4 fixa a seguinte equação regulatória inegociável:

$$\text{VRAM}_{\text{pesos}} + \text{VRAM}_{\text{kv\_cache\_quant}} \le 4.8 \text{ GB}$$

Esta meta orçamentária é cumprida síncronamente pela aplicação de duas técnicas de compactação de tensores de precisão na inicialização dos buffers:

1. **Quantização dos Pesos em IQ3_M com Matriz de Importância (imatrix):** Todos os modelos de escala $8\text{B}$ de parâmetros devem ser convertidos e carregados estritamente no formato IQ3_M calibrado com dados representativos de código fonte. O tamanho total do modelo em memória virtual fica limitado a exatamente $3.78 \text{ GB}$ ($3870 \text{ MB}$).
2. **Quantização Mandatória do KV Cache a 4-Bits (`Q4_K`):** O cache de contexto longo do SOULS é processado com quantização de 4 bits em vez de flutuação em 16-bits.

#### Cálculo de VRAM do KV Cache quantizado em Q4_K:

$$V_{\text{kv\_q4}} = 2 \times N_{\text{layers}} \times N_{\text{kv\_heads}} \times D_{\text{head}} \times L \times 0.5 \text{ bytes}$$$$V_{\text{kv\_q4}} = 2 \times 32 \times 8 \times 128 \times 30.000 \times 0.5 \text{ bytes} \approx 937.5 \text{ MB}$$

Sob esta topologia de compressão casada, o consumo consolidado em VRAM do modelo $8\text{B}$ com contexto de $30\text{k}$ tokens é reduzido para:

$$V_{\text{total\_souls}} = 3.78 \text{ GB} + 0.91 \text{ GB} + 0.20 \text{ GB} = 4.89 \text{ GB}$$

A compressão garante um headroom de segurança física de $1.11 \text{ GB}$ **(**$1136 \text{ MB}$**)** livre na GPU RTX 2060m para gerenciar picos de inicialização de tensores e manipulação gráfica da interface do host (Tauri v2 e sistema operacional local).

### 3. Mecanismo de Salvaguarda "Fail-Closed", `EngineCascade` e Isolamento Fiscais de Workers C++

Fica terminantemente proibido inicializar qualquer modelo local de inteligência artificial de forma cega ou baseada em heurísticas opinativas de configuração do usuário, assim como o uso de qualquer whitelist estática de strings de nomes de arquitetura.

- **EngineCascade e TopologyFeatures Dinâmico ($\mathcal{O}(1)$):** O SOULS adota o mecanismo `EngineCascade` para orquestrar e selecionar dinamicamente o motor de inferência. As características de topologia (`TopologyFeatures`) do modelo são extraídas dinamicamente via mapeamento de memória (`mmap`) do cabeçalho GGUF em tempo constante $\mathcal{O}(1)$ durante a fase de boot, sem necessidade de inferências prévias ou tabelas estáticas de strings.
- **Enjaulamento de Motores FFI C++ em Subprocessos Trabalhadores (`souls_vanguard_worker.exe`):** Motores FFI em C++ não-confiáveis, instáveis ou experimentais devem ser compulsoriamente isolados em subprocessos trabalhadores independentes (`souls_vanguard_worker.exe`), comunicando-se via IPC desidratado e blindado contra interrupções.
- **Tratamento de Crashes Nativos sem Panic:** Se o subprocesso trabalhador (`souls_vanguard_worker.exe`) sofrer um crash devido a exceções C++ nativas (tais como *vector out-of-bounds* ou *Access Violation*), o processo pai gerenciado pelo Tokio deve capturar a falha sem sofrer panic, marcar o modelo afetado como **INATIVO** na SSOT SQLite (FrankenSQLite) e retornar um erro estruturado `InferenceError`. O fallback *in-process* é **TERMINANTEMENTE PROIBIDO** para falhas estruturais de carregamento de tensores.
- **Renderização Dinâmica via `minijinja` (Zero-Allocation):** Integra-se a adoção da crate `minijinja` (compilada com `default-features = false`) no *hot-path* de bootstrapping do worker para renderização dinâmica e com zero-alocação de `chat_templates` contidos diretamente nos metadados extraídos do arquivo GGUF.
- **Comportamento Fail-Closed:** Se a soma projetada de memória de vídeo exceder $5.4 \text{ GB}$ ($90\%$ da VRAM total de $6\text{GB}$), o SOULS aborta síncronamente a operação de carregamento local, emite um log seco em terminal para o operador e desvia o roteamento de inferência do `ParetoBanditRouter` para o Fallback na Nuvem Soberana. Evita-se assim pânicos imprevisíveis no driver do sistema operacional do host.

## Consequências e Trade-offs

### Impactos Positivos:

- **Imunidade a Travamentos:** Erradicação absoluta de erros fatais do tipo CUDA Out-of-Memory (OOM) e congelamento de interface gráfica do host.
- **Previsibilidade Financeira:** Roteamento local-first garantido em até $90\%$ das requisições gerais graças à compressão agressiva de contexto na VRAM.
- **Segurança Física:** Resfriamento dinâmico da RTX 2060m por evitar que a placa opere continuamente com consumo de energia no teto elétrico de hardware devido à paginação PCIe.

### Impactos Negativos:

- **Latência de Cold Start:** O Boot inicial do sidecar efêmero do `mistral.rs` adiciona uma penalidade de carregamento de $1.2$ segundos antes do prefill primário. Aceita-se este custo em troca do isolamento térmico e de memória de vídeo.
- **Perda de Fidelidade Fina:** A quantização severa (IQ3_M) dos pesos do modelo pode introduzir degradação semântica e erros sintáticos. Esta degradação deve ser corrigida externamente pela aplicação obrigatória de decodificação restrita na CPU e amostragem Min-P, conforme formalizado na subsequente ADR-028.