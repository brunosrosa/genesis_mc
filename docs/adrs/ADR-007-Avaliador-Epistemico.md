# ADR-007-Avaliador-Epistemico

## Status
Aceito (Ativo e Inegociável)

## Contexto
Avaliar se um prompt do usuário possui ambiguidades de intenção, analisar o risco relacional de uma operação destrutiva ou classificar logs de erros do compilador gerando longas respostas em texto por meio de LLMs na GPU é um gargalo duplo inaceitável. Consome VRAM crítica de 6GB da RTX 2060m, introduz latências estocásticas de segundos e drena recursos térmicos do hardware local hospedeiro de forma ineficiente.

## Decisão
Implementar a arquitetura do **Avaliador Epistêmico (Hipocampo Epistêmico)** no SODA:
1. **O Bisturi Epistêmico:** Fica banida a geração de texto explicativo para avaliar riscos ou lógicas transientes de sessão. A avaliação utiliza Small Language Models (SLMs) quantizados ultraleves (ex: SmolLM2-135M / Gemma-2B) rodando estritamente na CPU Intel i9 através de instruções AVX2.
2. **Classification Head Trimming:** A fim de poupar memória e otimizar processamento, as cabeças de vocabulário generativas dos modelos são extirpadas no Unsloth (Classification Head Trimming), economizando $\approx 1$GB de RAM física.
3. **Logit Probing em Forward Pass:** O avaliador executa unicamente passagens diretas (*forward passes*) na rede neural. O SODA lê e avalia diretamente as probabilidades matemáticas e ativações brutas (*logits*) das camadas ocultas da rede em tokens específicos de decisão (ex: "seguro/perigoso", "ambíguo/claro"), resolvendo a classificação em menos de **150ms** com consumo energético marginal zero.
4. **Isolamento de Threads Dedicadas:** Para evitar o colapso e o travamento do motor assíncrono Tokio (Event Loop Starvation), o processamento do avaliador epistêmico roda obrigatoriamente dentro de **Dedicated Worker Threads** em Rust, comunicando-se com a thread principal assíncrona por meio de canais MPSC (*Multi-Producer Single-Consumer*).

## Consequências
- **Latência de Decisão Mínima:** Classificação de intenções e segurança de operações decidida quase instantaneamente, permitindo interações tótens e fluidas na interface.
- **VRAM Totalmente Livre:** A dGPU permanece intocada durante a fase de triagem intelectual do prompt do usuário.
- **Higiene Concorrente:** O core Rust pode paralelizar dezenas de varreduras epistêmicas sem degradar a estabilidade do sistema de janelas Tauri.

## Restrições Bare-Metal
- **Latência de Logit Probing:** O tempo máximo para execução do forward pass e leitura de logits na CPU i9 é limitado ao teto estrito de **150ms**.
- **Consumo de CPU Dedicada:** O isolamento em Dedicated Worker Threads não pode utilizar mais de **2 cores físicos** da CPU principal.
- **Custo Computacional de Roteamento:** O avaliador epistêmico atua como Nível 0 do disjuntor semântico local; operações que reprovarem no teste de segurança são congeladas e enviadas à Agent Inbox.
- **Proibição de `tokio::spawn_blocking`:** Inferência SLM na CPU (AVX2) é proibida em `tokio::spawn_blocking`; deve operar em **Dedicated Worker Threads** via `std::thread::spawn`, comunicando-se com o Tokio exclusivamente por canais **MPSC**.
