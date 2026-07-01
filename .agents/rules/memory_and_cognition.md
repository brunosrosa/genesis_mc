---
trigger: always_on
---

Revisado: 2026-07-01

###### CONSTITUIÇÃO COGNITIVA SODA
**Objetivo:** Erradicar a "Cegueira Temporal" e o "Context Rot" sem estourar os 6GB de VRAM da RTX 2060m. O SODA não usa bancos de dados tradicionais em nuvem; ele opera uma arquitetura neuro-simulada estritamente local e soberana.

###### 1. A TRÍADE DE MEMÓRIA (O CÉREBRO)
É TERMINANTEMENTE PROIBIDO o uso de PostgreSQL, Neo4j, FAISS ou bancos vetoriais em nuvem. A persistência cognitiva opera em três camadas:
*   **L1 (Efêmera):** RAM do sistema e KV Cache dinâmico para roteamento e respostas de latência zero.
*   **L2 (Episódica/Transacional):** **FrankenSQLite** com controle MVCC e Write-Ahead Logging (WAL). Armazena eventos e estados transacionais com escrita atômica, suportando múltiplos gravadores sem travar (`SQLITE_BUSY`). Para escrita concorrente, CONFIGURE EXPLICITAMENTE `busy_timeout` para evitar deadlocks.
*   **L3 (Semântica/Ontológica):** **LanceDB** (rodando via `mmap` direto do SSD NVMe) acoplado ao **LadybugDB** (banco de grafos 100% Rust para relações causais).
*   **A Métrica de Distância (FRQAD):** O uso da Similaridade de Cosseno está BANIDO. O cálculo vetorial DEVE usar a **Distância de Fisher-Rao Quantizada (FRQAD)**. Ela penaliza matematicamente vetores comprimidos, atingindo 100% de precisão onde o cosseno falharia na compressão agressiva.

###### 2. RAG TEMPORAL E A CURA DA CEGUEIRA (ANTI-RECENCY BIAS)
O SODA repudia a hipercomplexidade do *Temporal Graph RAG* (TG-RAG) para não asfixiar a CPU.
*   **Taxonomia de Sobrevivência:** Todo dado ingerido recebe a tag `STABLE` (regras e fundamentos que ignoram o tempo) ou `EVOLVING` (logs, chats, que possuem caducidade). Fatos `STABLE` NUNCA são apagados ou sobrepostos por "fatos recentes".
*   **Extração Temporal O(1):** O cálculo de intenção de datas ("sexta passada") ocorre nativamente em Rust (`temps` / `natural-date-parser`), resolvendo a string na CPU em 1ms. O LLM atua apenas como fallback via *Function Calling* se a CPU falhar.
*   **Pré-filtragem B-Tree e Proteção de Índice:** O LanceDB aplica filtros de tempo via *Hard SQL* *antes* da busca vetorial. Se a janela de tempo retornar menos de 1000 linhas, é OBRIGATÓRIO o uso do comando `bypass_vector_index()` para forçar a busca de força bruta (kNN Exato) e evitar o colapso do índice ANN.
*   **Busca Híbrida e Contextual Chunks:** A *string* da data deve ser injetada no corpo do texto antes de gerar o vetor. Exija Busca Híbrida (Vetor + BM25) para ancorar o raciocínio em saltos temporais (Multi-hop).

###### 3. O HIPOCAMPO EPISTÊMICO (LOGIT PROBING)
O SODA NÃO GERA TEXTO para avaliar risco, moralidade ou ambiguidade da fala do usuário (isso esgotaria a VRAM local).
*   **O Bisturi Analítico:** Usamos um SLM quantizado ultraleve (ex: Gemma-4-E2B ou Phi-4-mini). As LoRAs destes modelos DEVEM obrigatoriamente ser treinadas com **Classification Head Trimming** (extirpando o vocabulário gerador de texto no Unsloth para poupar 1GB de RAM).
*   **Logit Probing:** O SODA executa ESTRITAMENTE a passagem direta (*forward pass*). Lemos as probabilidades matemáticas brutas (*logits*) da camada oculta via `llama-cpp-4` ou `mistral.rs` para extrair scores exatos de "ambiguidade" ou "risco relacional" em <150ms.
*   **Isolamento de Thread:** Para não paralisar o motor Tokio, a inferência do Hipocampo RODA OBRIGATORIAMENTE em *Dedicated Worker Threads* isoladas (com canais MPSC), preservando o alinhamento vetorial AVX2 e o cache L1/L2 da CPU.

###### 4. MATURIDADE SIMBIÓTICA E PERSISTÊNCIA ASSÍNCRONA
A IA amadurece localmente, moldando sua personalidade sem usar algoritmos RLHF exaustivos.
*   **Métricas ELO/EMA e X-LoRA:** O comportamento reativo da IA é ajustado por pesos numéricos via Médias Móveis Exponenciais (EMA/ELO). Se o regime de diálogo exigir, o sistema fará o *Hot-Swapping* de matrizes LoRA quantizadas *in-flight* (em voo) diretamente na VRAM, mudando a atitude do agente sem recarregar o modelo base.
*   **Persistência Cabinet (Gitoxide):** O histórico imutável das mudanças de estado e memória estrutural NUNCA é salvo com bibliotecas C (Libgit2). O SODA usa o **gitoxide** (100% Rust) em *background* para versionar snapshots atômicos assincronamente, consumindo zero performance da thread principal.

###### 5. CEMITÉRIO SEMÂNTICO E DECAIMENTO ORGÂNICO
O lixo semântico não é apagado abruptamente por temporizadores (TTL); ele sofre deriva topológica.
*   **O Paradigma NextPlaid para Código:** Códigos-fonte e funções NUNCA devem ser esmagados em vetores monolíticos. O SODA os fatia obrigatoriamente em vetores menores baseados na Árvore de Sintaxe Abstrata (AST), garantindo que lógicas úteis não desapareçam na busca densa.
*   **Dinâmica de Langevin (PGD):** Durante a ociosidade da madrugada, o *Chyros Daemon* aplica o *Poincaré Gradient Descent*. Arquivos `EVOLVING` ociosos sofrem deriva hiperbólica em direção às bordas matemáticas do disco até serem arquivados ou esquecidos.
*   **Índice de Phronesis:** Contradições lógicas gravadas na memória não são lidas por IA generativa, mas encontradas via *Cohomologia de Feixes Celulares* $\mathcal{O}(N \log N)$. Se for detectado um conflito matemático ($H^1 \neq 0$), um paradoxo é sinalizado para auditoria humana no Canvas.