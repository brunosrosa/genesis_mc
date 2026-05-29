# 03_INFERENCE_ENGINE: O Motor de Raciocínio Híbrido

**Versão:** 3.2 (Definitiva - Hardware-Aware Inference)
**Status:** ATIVO E INEGOCIÁVEL
**Alvo da Leitura:** Engenheiros de Machine Learning, Desenvolvedores Rust, Agentes Orquestradores (Antigravity).

## 1. A TERMODINÂMICA DA INFERÊNCIA E O "LLAMA-SWAP"

O Genesis Mission Control (SODA) opera em uma zona de assimetria de hardware extrema: um processador veloz (Intel Core i9 com AVX2) e muita RAM estática (32GB), porém estrangulado por uma dGPU (NVIDIA RTX 2060m) com um teto de **6GB de VRAM**.

Manter múltiplos modelos grandes carregados na placa de vídeo é fisicamente impossível. O SODA adota o paradigma de **Desagregação Computacional** via `llama.cpp` nativo (utilizando o crate `llama_cpp_rs` no backend Rust):

- **Repouso Quente (Hot Repose):** Os pesos quantizados (GGUF em 4-bits - `Q4_K_M`) dos modelos especialistas ficam armazenados nos 32GB de RAM do sistema principal.
- **Injeção Llama-swap:** Quando uma tarefa exige a dGPU, o daemon Rust ativa a flag `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1`. Os tensores fluem da RAM para os 6GB da VRAM via barramento PCIe em questão de milissegundos, processam a inferência e são sumariamente expurgados de volta para a RAM (evicção), devolvendo a GPU para o sistema operacional.

## 2. ROTEAMENTO MECANICISTA (A MORTE DA BUSCA SEMÂNTICA CEGA)

Avaliar se uma tarefa deve rodar localmente ou ir para a nuvem lendo o texto do prompt (Semantic Routing tradicional) é ineficaz; um prompt curto pode exigir um raciocínio lógico que estoure a VRAM.

O SODA implementa o **Roteamento Mecanicista (SharedTrunkNet)**:

1. O texto de entrada passa por um modelo microscópico puramente na CPU (Nível 0).
2. Em vez de avaliar a resposta, o Rust examina as **Ativações de Prefill** (os estados ocultos da rede neural).
3. Utilizando o cálculo de _Entropia Semântica Neural (SNNE)_ e _Separabilidade de Fisher_, o sistema prevê a "dificuldade" matemática da tarefa em $< 50ms$.
4. Se o score apontar alto risco de colapso de memória ou alucinação, a tarefa sofre _Fallback_ automático.

## 3. A CONSTELAÇÃO LOCAL (HIERARQUIA DE ESPECIALISTAS)

O SODA não possui um "LLM Único". Ele é uma Mistura de Especialistas (MoE) orquestrada no nível do sistema operacional:

### Nível 0: Gateway Cognitivo (Always-On)

- **Modelos:** _Qwen3-0.6B_ ou _AVALIAR_OUTROS_MODELOS.
- **Alocação:** Estritamente CPU (Instruções AVX2).
- **Missão:** Operar a latência próxima de zero. Analisar intenções, disparar chamadas de ferramentas (JSON) proativamente e rotear o fluxo sem acordar a placa NVIDIA. Custo zero de VRAM.

### Nível 1: Executores Especialistas (dGPU)

- **Modelos:** _DeepSeek-R1-Distill-Qwen (7B)_ para raciocínio denso e _Rnj-1 8B_ para codificação (Fill-in-the-middle).
- **Alocação:** NVIDIA RTX 2060m (6GB VRAM) via invocação _Llama-swap_.
- **Missão:** Executar raciocínio "Sistema 2" de forma local, privada e segura.

### Nível 2: Cloud Fallback (Subscription Hacking / ParetoBandit)

- **Ferramentas:** Cli comerciais efêmeras (_Gemini CLI_) empacotadas via sidecars Docker e expostas via protocolo MCP.
- **Missão:** O orquestrador em Rust calcula a equação _ParetoBandit_ (Custo vs Qualidade vs Latência). Se a tarefa for uma refatoração massiva (ex: 80.000 tokens), o SODA delega assincronamente a tarefa para a Nuvem utilizando a cota Flat-Rate (mensalidade já paga do usuário) das CLIs, garantindo custo marginal zero (Inference Bill Shock = 0).

## 4. A SOBREVIVÊNCIA DO KV CACHE (ATENÇÃO ESPARSA)

A memória operacional do modelo (KV Cache) cresce linearmente e devora a VRAM em sessões longas. Para sobreviver ao teto de 6GB, o SODA implementa compressão atômica:

- **Arquiteturas de Ponta:** Obrigatoriedade de uso de modelos baseados em **Multi-head Latent Attention (MLA)** (ex: arquiteturas derivadas do DeepSeek) ou integração de tensores **HISA (Hierarchical Indexed Sparse Attention)** no motor nativo.
- **Tolerância a Contexto:** A quantização do KV Cache para formatos INT8/FP8 dentro do `llama.cpp` é inegociável, permitindo que o ambiente processe contextos úteis superiores a 16.000 tokens sem sofrer falhas fatais de _Out-Of-Memory (OOM)_.

_Fim da Especificação de Inferência. O Motor Híbrido está calibrado sob leis termodinâmicas locais._