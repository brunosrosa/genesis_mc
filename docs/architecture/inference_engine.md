# 03_INFERENCE_ENGINE: O Motor de Raciocínio Híbrido

**Versão:** 3.2 (Definitiva - Hardware-Aware Inference)
**Status:** ATIVO E INEGOCIÁVEL
**Alvo da Leitura:** Engenheiros de Machine Learning, Desenvolvedores Rust, Agentes Orquestradores (Antigravity).

## 1. A TERMODINÂMICA DA INFERÊNCIA E O "CANDLE-FIRST"

O Genesis Mission Control (SOULS) opera em uma zona de assimetria de hardware extrema: um processador veloz (Intel Core i9 com AVX2) e muita RAM estática (32GB), porém estrangulado por uma dGPU (NVIDIA RTX 2060m) com um teto de **6GB de VRAM**.

Manter múltiplos modelos grandes carregados na placa de vídeo é fisicamente impossível. O SOULS adota o paradigma de **Desagregação Computacional** com o **Candle** como motor central de inferência em Rust puro, preservando o controle bare-metal do pipeline:

- **Repouso Quente (Hot Repose):** Os pesos quantizados (GGUF em 4-bits - `Q4_K_M`) dos modelos especialistas ficam armazenados nos 32GB de RAM do sistema principal.
- **Injeção Candle-First:** Quando uma tarefa exige a dGPU, o daemon Rust desloca os tensores do worker soberano para os 6GB da VRAM de forma efêmera, processa a inferência e expurga o estado quente assim que a tarefa termina, devolvendo a GPU ao restante do sistema.

## 2. ROTEAMENTO MECANICISTA (A MORTE DA BUSCA SEMÂNTICA CEGA)

Avaliar se uma tarefa deve rodar localmente ou ir para a nuvem lendo o texto do prompt (Semantic Routing tradicional) é ineficaz; um prompt curto pode exigir um raciocínio lógico que estoure a VRAM.

O SOULS implementa o **Roteamento Mecanicista (SharedTrunkNet)**:

1. O texto de entrada passa por um modelo microscópico puramente na CPU (Nível 0).
2. Em vez de avaliar a resposta, o Rust examina as **Ativações de Prefill** (os estados ocultos da rede neural).
3. Utilizando o cálculo de _Entropia Semântica Neural (SNNE)_ e _Separabilidade de Fisher_, o sistema prevê a "dificuldade" matemática da tarefa em $< 50ms$.
4. Se o score apontar alto risco de colapso de memória ou alucinação, a tarefa sofre _Fallback_ automático.

## 3. A CONSTELAÇÃO LOCAL (HIERARQUIA DE ESPECIALISTAS)

O SOULS não possui um "LLM Único". Ele é uma Mistura de Especialistas (MoE) orquestrada no nível do sistema operacional:

### Nível 0: Gateway Cognitivo (Always-On)

- **Modelos:** _Qwen3-0.6B_ ou _AVALIAR_OUTROS_MODELOS.
- **Alocação:** Estritamente CPU (Instruções AVX2).
- **Missão:** Operar a latência próxima de zero. Analisar intenções, disparar chamadas de ferramentas (JSON) proativamente e rotear o fluxo sem acordar a placa NVIDIA. Custo zero de VRAM.

### Nível 1: Executores Especialistas (dGPU)

- **Modelos:** _DeepSeek-R1-Distill-Qwen (7B)_ para raciocínio denso e _Rnj-1 8B_ para codificação (Fill-in-the-middle).
- **Alocação:** NVIDIA RTX 2060m (6GB VRAM) via worker soberano baseado em **Candle**.
- **Missão:** Executar raciocínio "Sistema 2" de forma local, privada e segura.

### Nível 2: Cloud Fallback (Subscription Hacking / ParetoBandit)

- **Ferramentas:** Cli comerciais efêmeras (_Gemini CLI_) empacotadas via sidecars Docker e expostas via protocolo MCP.
- **Missão:** O orquestrador em Rust calcula a equação _ParetoBandit_ (Custo vs Qualidade vs Latência). Se a tarefa for uma refatoração massiva (ex: 80.000 tokens), o SOULS delega assincronamente a tarefa para a Nuvem utilizando a cota Flat-Rate (mensalidade já paga do usuário) das CLIs, garantindo custo marginal zero (Inference Bill Shock = 0).

## 4. A SOBREVIVÊNCIA DO KV CACHE (ATENÇÃO ESPARSA)

A memória operacional do modelo (KV Cache) cresce linearmente e devora a VRAM em sessões longas. Para sobreviver ao teto de 6GB, o SOULS implementa compressão atômica:

- **Arquiteturas de Ponta:** Obrigatoriedade de uso de modelos baseados em **Multi-head Latent Attention (MLA)** (ex: arquiteturas derivadas do DeepSeek) ou integração de tensores **HISA (Hierarchical Indexed Sparse Attention)** no motor nativo.
- **Tolerância a Contexto:** A gestão do KV Cache e a compressão do contexto longo devem permanecer sob o motor nativo em Rust, priorizando **Candle** no caminho central. `llama-cpp-4` pode sobreviver apenas como bisturi isolado de *Logit Probing*, nunca como pilar do fluxo gerativo principal.

## 5. LEIS DE INFERÊNCIA E SGR (DEEPSEEK V4 PRO NO OPENROUTER)

Estas leis são obrigatórias para qualquer requisição HTTP ao OpenRouter usando **deepseek/deepseek-v4-pro** em tarefas de Structured Outputs / Schema-Guided Reasoning (SGR).

- **LEI 1 (O Gatilho Semântico):** A palavra **"JSON"** DEVE estar explicitamente presente no `system_prompt` ou no `user_prompt`. Sem isso, o modelo tende a ignorar o contrato e degradar a resposta.
- **LEI 2 (Strict Schema):** A requisição HTTP DEVE usar `response_format` com `{"type":"json_schema"}` e a flag `"strict": true` (com `additionalProperties: false` e `required` completo). JSON Mode básico é frágil em payloads densos.
- **LEI 3 (Controle de Truncamento):** `max_tokens` DEVE ser suficientemente amplo para acomodar a sintaxe completa do JSON (chaves, aspas, vírgulas, colchetes). Faixa típica: **1500 a 16000**, calibrada pelo tamanho do schema e número de campos.
- **LEI 4 (Cognição Preservada):** É PROIBIDO forçar `reasoning_effort="low"` ao pedir ENUMs ou lógica densa. Use `high`/`xhigh` (ou o padrão do provedor) para o modelo conseguir planejar o JSON corretamente.
- **LEI 5 (Supressão de Transporte / Caminho do Meio):** Para evitar timeouts de rede e tráfego inútil de raciocínio, a requisição DEVE suprimir os tokens de pensamento no transporte. Aplique sempre `reasoning: { "exclude": true }` e/ou `include_reasoning: false`. O modelo pensa na nuvem, mas o cliente faz download APENAS do JSON final.

_Fim da Especificação de Inferência. O Motor Híbrido está calibrado sob leis termodinâmicas locais._
