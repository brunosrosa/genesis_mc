# RUST BARE METAL PATTERNS

## Premissas

Este documento ancora o repertorio de late-binding da skill `soda-rust-expert`. O foco e Rust bare-metal sob carga local severa, com rejeicao explicita de abstrações confortaveis que escondem custo termodinamico, ambiguidade de ownership ou colapso de throughput. O objetivo nao e escrever Rust "idiomatico de tutorial", mas sim sobreviver a arquiteturas reais com Tokio, IPC binario, inferencia local, sidecars nativos, latencia previsivel e soberania de memoria.

As 4 leis abaixo devem ser tratadas como heuristicas operacionais de projeto, revisao e refatoracao. Elas existem para impedir que o sistema pareca correto em benchmarks pequenos, mas desabe sob burst, starvation, deadlock, false sharing, reclaim indefinido ou serializacao acidental.

## 1. A Evolucao Do Borrow Checker

### A Era Polonius

O NLL classico melhorou drasticamente o Rust ergonomico, mas permanece conservador em cenarios relacionais mais sutis. Em topologias com grafos mutaveis, borrowers temporariamente disjuntos, pipelines assíncronos ou estados auto-referenciados, o compilador tradicional frequentemente recusa programas que sao semanticamente validos, porque sua modelagem de aliasing ainda opera por aproximacao lexical e nao por raciocinio relacional completo.

Polonius representa a direcao correta: ownership e lifetimes tratados como fatos derivados de relacoes entre loans, uses e invalidations, e nao apenas como intervalos lexicais aproximados. A consequencia pratica para engenharia bare-metal e simples:

- modelos de dados devem nascer preparados para analise fina de aliasing;
- refs emprestadas devem ser curtas, cirurgicas e associadas ao menor escopo funcional possivel;
- estruturas que exigem mutabilidade difusa devem ser redesenhadas antes de serem "curadas" com `Arc<Mutex<T>>`.

### Implicacao De Projeto

Quando um desenho exige proliferacao de `Arc<Mutex<_>>`, `RwLock`, `RefCell` ou canais apenas para contornar o borrow checker, o default correto e suspeitar da topologia, nao do compilador. Em producao local sob carga, travas dinamicas escondem custo real:

- serializam throughput;
- introduzem inversao de prioridade;
- criam deadlocks por ordem de aquisicao;
- induzem contention explosiva em paths quentes;
- mascaram fronteiras de ownership que deveriam ser topologicamente explicitas.

O criterio bare-metal e:

- preferir ownership por particao de dominio;
- preferir message passing com responsabilidade exclusiva por shard;
- preferir ECS, arena indexing, slot maps ou state machines explicitas a locks difusos;
- preferir fases de leitura/escrita bem demarcadas a aliasing mutavel permanente.

### Regra Operacional

Se um problema parece "resolver" com `Arc<Mutex<HashMap<...>>>`, pare e redesenhe. O objetivo e que o compilador aprove a topologia porque ela esta correta, nao porque foi anestesiada com sincronizacao pessimista.

## 2. A Farsa Do Lock-Free E A Ascensao Do Wait-Free

### CAS Nao E Magia

Muito codigo vendido como "lock-free" apenas trocou mutex visivel por complexidade invisivel. CAS puro (`compare_exchange`) resolve exclusao em nivel atomico, mas introduz o Problema ABA: um valor pode sair de `A`, passar por `B` e voltar para `A`, enganando algoritmos que observam apenas equivalencia final. O programa aparenta estabilidade enquanto a integridade temporal ja foi corrompida.

Em estruturas concorrentes reais, isso contamina stacks lock-free, freelists, reclamacao de nos e filas MPMC. Sem tagged pointers, hazard pointers ou outro protocolo de reclamacao, o "lock-free" degrada para comportamento nao deterministico, use-after-free potencial ou loops de retry sem limite significativo.

### O Limite Da EBR

Epoch-Based Reclamation (EBR) foi tratada por anos como resposta elegante: simples, rapida e adequada para cargas throughput-oriented. O problema e brutal em sistemas locais com threads heterogeneas, sidecars, inferencia e event loops mistos:

- se uma thread trava;
- se uma thread fica preemptada demais;
- se um worker entra em starved state;
- se um runtime conserva epoch antiga por tempo indefinido;

entao a memoria "aposentada" deixa de ser coletada. O resultado e vazamento estrutural, backlog de objetos mortos e crescimento sem limite previsivel.

### A Exigencia Atual: Wait-Free Memory Reclamation

A vanguarda relevante e reclamacao wait-free, como a linha representada por Kovan: o progresso de reclaim nao pode depender da boa vontade temporal de outra thread. O criterio duro deixa de ser "na media e rapido" e passa a ser "cada thread termina seu protocolo em limite estrito de passos".

Para o ecossistema SODA, a heuristica e:

- lock-free sem estrategia explicita contra ABA e arquitetura incompleta;
- EBR e aceitavel apenas como compromisso local, nunca como verdade final;
- em estruturas centrais e quentes, a direcao correta e reclaim wait-free com limite estrito de CPU por operacao.

### Regra Operacional

Se um algoritmo concorrente so e seguro enquanto todas as threads colaboram perfeitamente, ele nao e bare-metal suficiente para carga real. O design deve sobreviver a pausas arbitrarias, starvation localizada e workers zumbificados sem crescer memoria indefinidamente.

## 3. A Violencia Do Zero-Copy Total

### Serializar E Um Crime Termodinamico

Em sistemas locais de alta pressao, serializacao nao e detalhe: e conversao desnecessaria de energia em latencia, alocacao, cache miss e copia. O fluxo ideal nao "traduz" dados entre camadas; ele preserva layout, alinhamento e ownership desde a origem ate o consumidor.

Sempre que uma estrutura sai de Rust, vira JSON textual, entra em JS, vira objeto, volta para bytes e retorna ao backend, houve crime termodinamico. Houve custo de parse, custo de formatacao, custo de heap, custo de GC e custo de perda de previsibilidade.

### Ferramentas Obrigatorias

O repertorio bare-metal deve favorecer:

- `rkyv` para persistencia e transporte com layout arquivado e sem desserializacao obrigatoria;
- `bytemuck` para casts seguros entre tipos POD e fatias de bytes;
- `zerocopy` para leitura estruturada diretamente sobre buffers;
- slices, views e offsets em vez de materializacao de copias intermediarias.

### Consequencias Arquiteturais

O design de DTOs deve considerar desde o inicio:

- alinhamento;
- padding;
- endianess;
- ownership do buffer;
- validade do lifetime do mapa em memoria;
- ausencia de campos que exijam heap boxing desnecessario;
- capacidade de leitura incremental sem parse integral.

Isso altera o proprio desenho da API. Em vez de "retornar structs convenientes", o backend deve preferir blocos binarios legiveis por view. Em vez de "desserializar tudo para trabalhar", o consumidor deve projetar consultas por offset, coluna ou fatia.

### Regra Operacional

Se uma camada serializa para texto apenas para outra camada desserializar imediatamente, ha desperdicio de CPU e memoria. O padrao correto e deserialization-free data flow.

## 4. O Fim Do Monopolio Do #[tokio::main]

### Um Runtime Unico Nao E Topologia

`#[tokio::main]` e um bootstrap conveniente, nao um dogma arquitetural. Em sistemas locais com:

- I/O de disco;
- pipes IPC;
- scraping;
- analise AST;
- inferencia local;
- sidecars pesados;
- telemetria;
- UI reativa;

colocar tudo no mesmo runtime e pedir que cargas heterogeneas compartilhem a mesma fisiologia de escalonamento. O resultado tende a ser:

- starvation de tasks leves por tasks pesadas;
- jitter em telemetria;
- latencia nao deterministica em IPC;
- collapse de throughput por work stealing inadequado;
- contaminacao do event loop principal por jobs de CPU.

### Isolamento Fisico De Runtimes

O padrao bare-metal exige runtime segregation:

- runtime principal para orquestracao e eventos leves;
- worker threads dedicadas para inferencia/AVX2;
- pools separados para blocking I/O;
- executores especializados para sidecars ou parsing pesado;
- filas claras entre dominios, em vez de sharing oportunista.

Quando necessario, o runtime deve ser construido manualmente com `tokio::runtime::Builder`, com controle de:

- numero de worker threads;
- enablement de I/O e time;
- limites de blocking threads;
- naming de threads;
- instrumentacao;
- afinidade de nucleo.

### CPU Pinning E Afinidade

Em alta carga local, afinidade de nucleo deixa de ser micro-otimizacao e vira blindagem de previsibilidade. Threads de inferencia, OCR, parsers massivos ou pipelines com cache quente nao devem migrar arbitrariamente competindo com telemetria, watchdogs e reatividade da UI.

CPU pinning permite:

- preservar locality de cache;
- reduzir jitter de escalonamento;
- proteger o event loop principal;
- evitar que workloads AVX2/FP32 contaminem latencias sensiveis.

### Regra Operacional

Nao compartilhar o mesmo pool entre tarefas de natureza termodinamica distinta. Thread de inferencia nao e thread de telemetria. Thread de I/O pesado nao e thread do event loop. O bootstrap conveniente deve ceder lugar ao desenho explicito do runtime.

## Checklist De Sobrevivencia

- evite `Arc<Mutex<_>>` como default; redesenhe ownership primeiro;
- trate CAS puro como inicio do problema, nao como prova de lock-free robusto;
- questione EBR em estruturas centrais onde pausas arbitrarias podem ocorrer;
- favoreca reclaim wait-free quando o requisito for previsibilidade real;
- elimine serializacao textual entre camadas locais;
- use `rkyv`, `bytemuck`, `zerocopy` e views binarios como arquitetura, nao como otimização tardia;
- construa runtimes explicitamente quando a carga misturar I/O, CPU e inferencia;
- isole workers pesados e aplique afinidade de nucleo quando a previsibilidade for requisito.

## Conclusao Operacional

Rust bare-metal de vanguarda nao e apenas "codigo sem GC". E controle topologico de ownership, memoria, reclaim, layout e escalonamento. O sistema correto nao luta contra o compilador, nao terceiriza invariantes para mutex, nao romantiza CAS e nao serializa por comodidade. Ele explicita fronteiras, corta copias, fixa responsabilidades e transforma custo oculto em desenho mecânico visivel.
