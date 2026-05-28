# ADR-004-Triade-de-Memoria

## Status
Aceito (Ativo e Inegociável)

## Contexto
O *Context Rot* (Apodrecimento de Contexto) e a amnésia sistêmica são problemas endêmicos em assistentes de IA que operam injetando logs extensos e arquivos cru via prompts monolíticos. Esse padrão de RAG ingênuo inunda a VRAM estrita de 6GB da RTX 2060m, gerando perda severa de atenção no modelo e falhas catastróficas de Out-of-Memory (OOM). Adicionalmente, bancos de dados corporativos tradicionais em nuvem ou rodando via contêineres pesados (ex: PostgreSQL, Redis, Neo4j, FAISS) impõem contenção térmica, latência de rede e grande sobrecarga de RAM local.

## Decisão
Implementar a **Memória Neuro-Sintética (MNS)** do SODA como uma arquitetura tripartite em disco e RAM, estritamente embutida e local-first:
1. **L1 (Efêmera - Pointer Index Layer):** Alocada na RAM transiente. Não retém arquivos densos; gerencia apenas um índice raso de ponteiros e offsets geográficos ($\approx 150$ caracteres por pointer) apontando cirurgicamente para as camadas de disco.
2. **L2 (Episódica - FrankenSQLite):** Banco de dados **SQLite** em modo **WAL (Write-Ahead Logging)** com extensão **FTS5** ativa para buscas textuais. Opera rigidamente sob o paradigma de **Event Sourcing** (Append-only). Ficam expressamente proibidas operações de `UPDATE` e `DELETE` no núcleo da memória para evitar o apagamento do histórico e do raciocínio causal da máquina.
3. **L3 (Semântica e Ontológica - LanceDB & LadybugDB):**
   - **LanceDB:** Embutido nativamente no Rust para similaridade vetorial via Apache Arrow, efetuando buscas diretamente no SSD (Zero-Copy) sem consumir o cache de alocação da CPU i9.
   - **LadybugDB:** Banco de grafos 100% Rust para mapeamento de conexões semânticas cruzadas e dependências sistêmicas.
4. **Métrica de Similaridade:** Fica decretado o banimento da Similaridade de Cosseno no cálculo de vetores compactados. O SODA utilizará a **Distância de Fisher-Rao Quantizada (FRQAD)** para penalizar desvios estatísticos na compressão semântica agresiva, mantendo precisão matemática em cenários nos quais a métrica de cosseno falharia.

## Consequências
- **Amnésia Zero:** A IA reconstrói e audita sua própria trilha histórica de pensamentos passados com consistência transacional absoluta.
- **Eficiência Computacional:** Sem daemons adicionais rodando em background; a persistência em disco consome 0 bytes de memória de trabalho quando ociosa.
- **Rigor de Dados:** O modelo de dados evolui puramente por snapshots de eventos adicionais, blindando as trilhas de dados contra corrupção silenciosa de dados (SDC).

## Restrições Bare-Metal
- **Latência de Recuperação L2/L3 Híbrida:** A união entre busca textual SQLite FTS5 e busca vetorial LanceDB SSD deve executar em menos de **20ms**.
- **Segurança de Gravação SQLite:** O FrankenSQLite opera estritamente com *MVCC* e **Serializable Snapshot Isolation (SSI)** para evitar contenção por escrita concorrente e mitigar `SQLITE_BUSY`.
- **Manutenção LanceDB em Background:** Compactação de blocos e ordenação vetorial do LanceDB são proibidas no Event Loop do Tokio; devem ocorrer exclusivamente em Background Worker Threads com prioridade mínima.
- **Reserva de Memória L3:** As operações com LanceDB em memória transiente são limitadas ao teto máximo rígido de **512MB** de memória RAM ativa.
