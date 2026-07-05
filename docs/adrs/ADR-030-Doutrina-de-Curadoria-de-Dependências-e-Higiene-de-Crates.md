---
aliases:
  - "ADR-030: Doutrina de Curadoria de Dependências e Higiene de Crates"
version: 1.0
---

# ADR-030: Doutrina de Curadoria de Dependências e Higiene de Crates

## Status

Aceito (Ativo, Inegociável e Fundacional para SODA V4)

## Contexto Técnico e Restrições Físicas de Infraestrutura

O desenvolvimento de software guiado por agentes automatizados em regimes de auto-reparo e geração contínua de código (_Spec-Driven Development_ / _TDD_) expõe o SODA V4 a um risco severo de entropia sintática, comumente denominado pela doutrina como "Slop de Dependências".

Durante as iterações rápidas do compilador no ciclo interno, a injeção estocástica de crates externas não homologadas agride diretamente a eficiência térmica e de memória do host:

- **A Topologia de Cache do Hospedeiro:** O processador Intel Core i9 conta com um pool compartilhado e altamente sensível de Cache $L_3$. A expansão excessiva de tipos e metadados gerada por macros procedimentais obesas fragmenta a localidade espacial e temporal, provocando o fenômeno de _Cache Thrashing_.
- **O Orçamento de RAM do Sistema:** A máquina hospedeira de desenvolvimento possui exatamente $32 \text{ GB}$ de RAM física dedicados. A poluição do grafo de compilação do `Cargo` por crates redundantes que exijam múltiplos runtimes, clonagens profundas de strings ou geradores dinâmicos eleva o consumo de memória de sistema durante o ciclo incremental de build, provocando latência de disco desnecessária.
- **Garantia de Compilação Determinística:** Projetos agênticos locais operando sob o cinto de segurança bare-metal exigem que todo e qualquer binário gerado seja $100\%$ rastreável e reproduzível, impedindo desvios de comportamento sintático induzidos por atualizações opacas de sub-dependências transitórias no registro do crates.io.

## Declaração do Problema

Como estruturar a árvore de dependências do SODA V4 de forma que o tempo de macro-parsing e a pegada de metadados do compilador operem sob complexidade assintoticamente estável $O(1)$ sobre as linhas de cache $L_3$ da CPU, impedindo que upgrades silenciosos corrompam o isolamento térmico e de memória RAM da máquina host?

## Decisões Arquiteturais da SODA V4

```
                                  TOKEN STREAM DE ENTRADA (Código Rust)
                                                    |
                                                    v
                                    +-------------------------------+
                                    |     PROIBIÇÃO DO MOTOR syn    |
                                    | - Sem parsing de AST profunda |
                                    | - Sem varredura de blocos     |
                                    +-------------------------------+
                                                    |
                                                    v
                                    +-------------------------------+
                                    |   MÁXIMA EFICIÊNCIA SINTÁTICA |
                                    |                               |
                                    |   [ venial ] Outer Shell      |
                                    |   --> Lê apenas declarações   |
                                    |                               |
                                    |   [ unsynn ] Zero-Copy        |
                                    |   --> Fatiamento síncrono     |
                                    +-------------------------------+
                                                    |
                                                    v
                                    +-------------------------------+
                                    |      ESTABILIZAÇÃO DE I/O     |
                                    |  thiserror v2 (#![no_std])    |
                                    |  Version Pinning Fixo (=)     |
                                    +-------------------------------+
                                                    |
                                                    v
                                      COMPILAÇÃO BARE-METAL ULTRA-LEVE
```

### 1. O Extermínio e Banimento Terminal do `syn`

Fica sumariamente proibida a injeção ou utilização direta de macros procedimentais que dependam da crate `syn` para rotinas de parsing simples ou puramente estruturais no Data Plane do SODA V4.

- O `syn` reconstrói recursivamente a árvore sintática abstrata (AST) integral de blocos procedimentais densos. Esse comportamento inflaciona a memória durante a etapa de macro-parsing, estourando as linhas de Cache $L_3$ da CPU hospedeira.
- A pegada de memória ($M$) e a complexidade temporal ($T$) de processamento de tokens por passagem recursiva profunda do `syn` é descrita por:

$$T_{\text{syn}}(N) = \mathcal{O}(N \cdot d_{\text{AST}})$$$$M_{\text{syn}}(N) = \mathcal{O}(N \cdot d_{\text{AST}})$$

Onde $N$ representa o número cumulativo de tokens do stream analisado e $d_{\text{AST}}$ a profundidade do grafo sintático resultante.

- Sob SODA V4, o `syn` é classificado como dependência de quarentena. Sua permanência na árvore de compilação só é tolerada se encapsulada sob runtimes estáticos e isolados que não afetem o caminho crítico do ciclo de build incremental do daemon.

### 2. Adoção Cirúrgica de Motores Superficiais (`venial` e `unsynn`)

Para toda e qualquer necessidade de metaprogramação, macro-parsing ou extração sintática de atributos estruturais no SODA V4, o compilador deve utilizar abordagens que limitem a varredura lógica à casca externa das declarações.

- **Outer-Shell Parsing via `venial`:** O SODA V4 padroniza a crate `venial` para decodificar assinaturas e estruturas externas de structs, enums e funções. A crate `venial` cessa o parsing sintático imediatamente ao cruzar as chaves `{` de corpos de implementação, tratando o interior como um bloco opaco de tokens brutos.
- **Token Slicing via `unsynn`:** A manipulação e casamento de padrões de sequências de tokens em macros procedimentais internas deve ser realizada através da crate `unsynn`. Esta crate opera sob o paradigma _Zero-Copy_, fatiando ponteiros de memória e referências de tokens diretamente a partir do compilador, com consumo de alocação no Heap próximo a zero:

$$T_{\text{venial}}(N) = \mathcal{O}(N_{\text{outer}})$$$$M_{\text{venial}}(N) = \mathcal{O}(1) \quad (\text{sem alocações redundantes})$$

Onde $N_{\text{outer}}$ representa unicamente os tokens periféricos de declaração da assinatura de tipos ($N_{\text{outer}} \ll N$).

### 3. Padronização de Mapeamento de Erros via `thiserror v2`

Fica proibido o acoplamento de engines de erro excessivamente verbosas, dinâmicas ou baseadas em alocações pesadas no Heap para representação de falhas de I/O e lógica de negócios.

- O SODA V4 adota estritamente a crate **`thiserror` (versão 2)** para todo o mapeamento e tratamento de erros tipados no backend em Rust.
- Exige-se que os erros definidos via macro procedimental derivados do `thiserror` operem em total compatibilidade com ambientes freestanding (**`#![no_std]`**). A representação e o mapeamento dos enums de erro devem ser resolvidos de forma estática em tempo de compilação através de referências imutáveis do tipo `&'static str`.
- Evita-se assim a formatação dinâmica de estruturas de string redundantes e fragmentações residuais de memória no Heap dos 32GB de RAM central do hospedeiro, garantindo que o canal de controle (_Control Plane_) do IPC permaneça limpo e leve.

### 4. O Dogma Hermético do Version Pinning Dinâmico

Fica terminantemente proibido o uso de operadores de resolução de versão flexíveis, frouxos ou flutuantes (tais como caret `^`, asterisco `*` ou til `~`) para qualquer dependência declarada nos manifestos `Cargo.toml` do ecossistema SODA V4.

- **Versionamento Estrito (=):** Toda declaração de dependência deve utilizar obrigatoriamente a ancoragem literal de igualdade com precisão cirúrgica de patch (ex: `thiserror = "=2.0.0"`).
- **Workspace Centralizado:** Todas as dependências externas compartilhadas pelos sub-crates (como `llama-cpp-2`, `chaser-oxide` ou `llguidance`) devem ter suas versões travadas de forma unificada no bloco `[workspace.dependencies]` localizado no `Cargo.toml` raiz do projeto.
- Essa blindagem impede flutuações e quebras de pipeline sínclonas induzidas pela rede durante boots de agentes ou compilações incrementais locais, assegurando que o sistema opere sempre sob a exata combinação física de bytes validada e homologada em ambiente de teste de hardware.

## Consequências e Trade-offs

### Impactos Positivos:

- **Compilação de Alta Frequência:** Redução drástica e comprovada no tempo de build incremental do daemon em Rust, otimizando o tempo de feedback do _Ralph Loop_ no TDD para menos de $2$ segundos.
- **Isolamento de Cache L3:** Erradicação absoluta de surtos de latência induzidos por _Cache Thrashing_ no processador Intel i9 durante a execução concorrente de tarefas pesadas do Tokio.
- **Estabilidade de Pipeline:** Bloqueio terminal contra a introdução acidental de dependências transitórias "tóxicas" ou quebras de comportamento sintático provocadas por upgrades de bibliotecas na nuvem.

### Impactos Negativos:

- **Custo de Manutenção Manual:** A atualização de qualquer biblioteca no workspace exige alteração explícita e cirúrgica do Arquiteto no manifesto principal, seguida por auditoria e recompilação completa do banco de testes local.
- **Restrição de Flexibilidade Sintática:** Macros procedimentais internas do SODA V4 tornam-se sintaticamente mais simples e focadas, abrindo mão da capacidade de ler corpos internos complexos de funções em tempo de compilação. Esse trade-off é aceito em prol da higiene e da localidade de cache da CPU.