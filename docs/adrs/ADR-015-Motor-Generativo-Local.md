# ADR-015-Motor-Generativo-Local

## Status
Aceito (Ativo e Inegociável)

## Contexto
Depender de servidores externos pesados em contêineres Docker ou CLIs de daemons monolíticos (como Ollama, LM Studio ou LocalAI) rodando continuamente em segundo plano causa sobrecarga na CPU i9, gera contenção de rede IPC e introduz riscos severos de vazamento de controle de processos. Além disso, a placa de vídeo de consumidor do usuário (NVIDIA RTX 2060m com 6GB VRAM) é fisicamente incapaz de manter múltiplos modelos de grande porte carregados quentes na VRAM simultaneamente com o ambiente gráfico.

## Decisão
Implementar o **Motor Generativo Local** do SODA incorporado nativamente ao core de backend em Rust:
1. **Inferência Embarcada via Candle e Mistral.rs:** O core gerador de IA do SODA é compilado em Rust Bare-Metal utilizando os ecossistemas **Candle** (Hugging Face) e **mistral.rs**. Fica terminantemente proibido o uso de Ollama ou daemons JSON-REST externos para inferência no código final de produção.
2. **Desagregação Computacional (Llama-swap):** Os pesos quantizados dos modelos especialistas locais (`GGUF` no formato `Q4_K_M`, tais como DeepSeek-R1-Distill-Qwen 7B ou Rnj-1 8B) permanecem armazenados quentes na RAM principal (32GB DDR4/DDR5) do host (Hot Repose).
3. **Mecânica de Troca PCIe:** Quando uma tarefa é despachada para a dGPU pelo ParetoBandit, o SODA ativa a flag `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1`. Os tensores fluem em milissegundos via barramento PCIe para os 6GB de VRAM da dGPU, realizam a inferência e sofrem evicção atômica de volta para a RAM física após a conclusão, zerando o barramento VRAM imediatamente.
4. **Atenção Latente e Compressão de KV Cache:** A compressão do contexto na GPU exige o suporte a arquiteturas baseadas em **Multi-head Latent Attention (MLA)** e a quantização do KV Cache para formatos compactados **INT8/FP8**, viabilizando contextos úteis superiores a 16.000 tokens sem risco de Out-of-Memory (OOM).

## Consequências
- **Privacidade Soberana Total:** Os modelos e os pesos rodam fisicamente na máquina local, garantindo confidencialidade absoluta e independência de conexões externas à internet.
- **Gerenciamento Unificado de Processos:** Sem processos zumbis de IA comendo ciclos de CPU do host de forma oculta; o ciclo de vida do motor de IA é amarrado à própria execução da janela do Tauri.
- **Eficiência Computacional:** A RTX 2060m é liberada imediatamente para jogos ou renderização gráfica assim que a IA cessa a inferência ativa.

## Restrições Bare-Metal
- **Teto Crítico de VRAM:** A alocação local máxima do modelo gerador na RTX 2060m é limitada ao teto estrito de **4.5GB** de VRAM ativa.
- **Latência de Swap PCIe:** A transferência de tensores do host para a dGPU via PCIe deve executar em menos de **1.5 segundos**.
- **Quantização Obrigatória:** Fica proibida a execução de modelos locais em formatos de precisão f32/f16; a quantização padrão obrigatória para modelos locais na GPU de 6GB é o formato `Q4_K_M`.
- **Batching Sequencial (Anti-Spillover):** É terminantemente proibido carregar múltiplos modelos generativos em paralelo na RTX 2060m; o Motor deve operar estritamente com **Batching Sequencial** e reciclagem de contexto via **FastSwitch**, prevenindo derrame de VRAM para o barramento DDR4.
