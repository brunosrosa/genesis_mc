# ADR-005-RAG-Temporal

## Status
Aceito (Ativo e Inegociável)

## Contexto
A busca vetorial ingênua em RAG convencional sofre de "Recency Bias" (Viés de Recorrência) e contaminação de ruído semântico. Além disso, índices de busca aproximada por vizinho mais próximo (ANN) sofrem de perda severa de precisão (recall) quando filtros de metadados estreitos são aplicados pós-busca ou quando o volume total pesquisado na janela temporal é muito pequeno. Para mentes neurodivergentes, a IA necessita recuperar com precisão cirúrgica referências exatas de datas, e-mails e trechos específicos sem misturar contextos passados obsoletos.

## Decisão
Implementar a arquitetura de **RAG Temporal Híbrido** no SODA:
1. **Taxonomia de Sobrevivência:** Todo chunk de informação ingerido recebe compulsoriamente a marcação `STABLE` (regras estruturais, chaves de arquitetura e conceitos lógicos imutáveis que não sofrem degradação temporal) ou `EVOLVING` (logs de sessão, conversas passadas e estados voláteis suscetíveis a obsolescência).
2. **Pré-Filtragem Hard SQL B-Tree:** Antes de acionar qualquer motor de busca vetorial aproximada (ANN), o motor em Rust executa uma pré-filtragem rígida em SQL utilizando índices B-Tree no SQLite/LanceDB. A string temporal de entrada (ex: "semana passada") é previamente convertida em offsets absolutos pela CPU na camada de parsing lógico local.
3. **Trava de Proteção de Índice (bypass_vector_index):** Se a janela temporal filtrada retornar **menos de 1000 linhas**, fica estabelecida a obrigatoriedade de ignorar o índice vetorial aproximado e invocar `bypass_vector_index()`. O motor executa uma busca linear exata (kNN Exato) na dGPU/CPU, prevenindo falsos negativos de indexes aproximados sub-populados.
4. **Decaimento Orgânico via Dinâmica de Langevin:** Na inatividade noturna, o **Chyros Daemon** executa a consolidação de memória em CPU i9. Ele aplica a Dinâmica de Langevin (*Poincaré Gradient Descent*) para empurrar chunks `EVOLVING` frios e ociosos em direção às bordas topológicas de compressão, aglutinando-os em sumários ontológicos ontogenéticos densos, reduzindo a pegada física no banco. Chunks `STABLE` são blindados e imunes ao decaimento.
5. **Compressão de Cemitério:** Dados arquivados pela Dinâmica de Langevin não podem consumir IOPS úteis do SSD. Devem ser obrigatoriamente comprimidos usando a crate nativa **zstd** (Zstandard Zero-Copy). Embeddings inativos sofrem **Quantização Extrema 2-bits (Polar)** para minimizar o footprint de armazenamento.

## Consequências
- **Recuperação Cirúrgica:** Erradicação total de alucinações e perda de precisão em consultas contendo filtros de datas específicas.
- **Redução do Context Rot:** O contexto injetado para o LLM contém apenas "ouro matemático" sumarizado de forma limpa, poupando a VRAM termodinâmica.
- **Higiene Semântica:** Expurgo contínuo de lixo cognitivo e repetições caóticas sem perda de informações estruturais e pilares de decisão.

## Restrições Bare-Metal
- **Bypass Trigger:** Execução obrigatória do `bypass_vector_index()` para qualquer volume sob busca filtrada $< 1000$ nós.
- **Latência de Parsing Temporal:** A conversão de termos linguísticos para offsets absolutos no parser local de tempo em Rust deve executar em menos de **2ms**.
- **Custo Computacional Noturno:** A rotina do Chyros Daemon limita o consumo de energia da CPU i9 a no máximo **40% de utilização** térmica, operando estritamente em segundo plano.
- **Manutenção LanceDB em Background:** Compactação de blocos e ordenação vetorial do LanceDB são proibidas no Event Loop do Tokio; devem ocorrer exclusivamente em Background Worker Threads com prioridade mínima.
- **Persistência Transacional (L2):** A camada transacional é estritamente o **FrankenSQLite** operando com *MVCC* e **Serializable Snapshot Isolation (SSI)**; motores relacionais externos e locks de I/O que induzam `SQLITE_BUSY` são proibidos.
- **Compressão de Cemitério (zstd):** Dados arquivados pela Dinâmica de Langevin devem ser armazenados comprimidos com **zstd** (Zero-Copy) para não consumir IOPS úteis do SSD.
- **Quantização Extrema (2-bits Polar):** Embeddings inativos devem sofrer quantização extrema de **2-bits (Polar)** para minimizar o footprint persistente.
