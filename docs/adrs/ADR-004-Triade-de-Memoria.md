---
id: "ADR-004"
title: "ADR-004-Triade-de-Memoria"
version: 2.0
status: Ativo_Inegociavel
epic: "Memória"
description: "Institui a Tríade de Memória isolada (SQLite, LanceDB, LadybugDB) operando estritamente via Event Sourcing (WAL Append-Only) e compressão zstd para preservação do SSD."
---

### ADR-004: Tríade de Memória, Event Sourcing e Compressão Bare-Metal

#### Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SODA V4)

#### Contexto Técnico e Ameaça Operacional (O Gargalo Físico)
Sistemas agênticos convencionais tentam combater a "Amnésia Sistêmica" (Context Rot) utilizando bancos de dados únicos e monolíticos para armazenar metadados, grafos e vetores massivos de embeddings. Quando operando no host do usuário, o disparo contínuo de milhares de micro-transações geradas por agentes (logs, reflexões, RAG) resulta em esquizofrenia de I/O.
Na prática, o uso ingênuo de comandos `UPDATE` e `DELETE` frequentes asfixia a taxa de transferência do disco, eleva o bloqueio de threads e, criticamente, destrói a vida útil física (Terabytes Written - TBW) do SSD NVMe do usuário. Além disso, o uso de algoritmos de compressão legados (como `zip`) para o arquivamento de vetores antigos ("arquivo frio") penaliza a CPU com ciclos de Garbage Collection e alocações dinâmicas inaceitáveis.

#### Decisão Arquitetural (A Matriz de Persistência O(1))
Fica decretado o uso da "Tríade de Memória" segmentada, operada sob regras rígidas de acesso e compressão, desenhadas para garantir longevidade de hardware:

**Módulo 1: Segmentação Obrigatória (A Tríade)**
*   **LadybugDB:** Assume exclusivamente a topologia causal e as relações gráficas estruturais da ontologia do usuário.
*   **LanceDB:** Opera como banco vetorial mapeado diretamente na memória física (via `mmap`), dispensando o carregamento integral dos índices na RAM do sistema.
*   **FrankenSQLite:** Atua como o cofre transacional e relacional absoluto para metadados, estados de tarefas (Kanban) e telemetria profunda.

**Módulo 2: O Motor de Event Sourcing e Proteção de SSD**
*   Para acomodar o tráfego de alta frequência de logs agênticos sem matar o SSD, o **FrankenSQLite** deve operar rigorosamente no modo **WAL (Write-Ahead Logging)** acoplado à arquitetura de **Event Sourcing (Append-Only)**.
*   É estritamente **PROIBIDO** o uso de comandos destrutivos diretos (`UPDATE` ou `DELETE`) nas tabelas primárias de eventos da IA. Os dados devem nascer como registros históricos imutáveis.
*   **O Pipeline MPSC:** A escrita em disco não deve bloquear o *Event Loop* do Tokio. As requisições são postadas via canais `mpsc` na RAM, onde uma *Dedicated Worker Thread* (Background Worker) acumula os lotes e realiza gravações sequenciais (*Batched Writes*), amortizando o atrito de I/O no NVMe.

**Módulo 3: Higiene de Cemitério e Compressão Vetorial (`zstd`)**
*   Quando a "Dinâmica de Langevin" (arquivamento de memórias marginais) empurrar vetores antigos para as bordas hiperbólicas do arquivo frio, a compressão dos dados não utilizará algoritmos padrão.
*   A adoção da crate `zip` fica **banida** das engrenagens de compressão de memória contínua. Fica imposto o uso estrito do motor **`zstd`** (Zstandard), garantindo compressão e descompressão de dicionários com características **Zero-Copy**, velocidade extrema e economia brutal de espaço de armazenamento sem gargalos na CPU i9.

#### Consequências Operacionais e Defesa contra o Slop (Trade-offs)
*   **Impacto Positivo:** Desgaste do SSD (TBW) reduzido a níveis mínimos através de gravações em lote. Sobrevivência 24/7 do daemon sem congelamento do Tokio por I/O. Recuperação perfeita do histórico da IA sem corrupção silenciosa (SDC) e otimização radical de espaço no "arquivo frio" via `zstd`.
*   **Impacto Negativo (Manutenção de Estado):** O paradigma *Append-Only* gera crescimento de banco teoricamente infinito. A arquitetura exige que o desenvolvedor crie *Snapshots* periódicos de estado e implemente as engrenagens ativas de "Slimming" (esquecimento orgânico e poda) para compactar os diários passados durante o tempo de inatividade da máquina (madrugada), o que adiciona complexidade à orquestração do *Chyros Daemon*.
