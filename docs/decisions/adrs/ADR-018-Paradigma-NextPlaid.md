---
id: "ADR-018"
title: "ADR-018-Paradigma-NextPlaid-e-Enjaulamento-AST"
version: 2.0
status: Ativo_Inegociavel
epic: "Memória"
description: "Impõe o fatiamento de código O(1) via AST (ast-grep) e o enjaulamento compulsório de todas as gramáticas C do tree-sitter em WebAssembly (wasmtime) para prevenir segfaults."
---

### ADR-018: Paradigma NextPlaid, Parsing AST e Enjaulamento em WebAssembly

#### Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SOULS V4)

#### Contexto Técnico e Ameaça Operacional (O Risco do C-FFI)
Historicamente, o fatiamento (*chunking*) de código-fonte baseado em delimitadores cegos (caracteres ou quebras de linha) destrói as assinaturas de funções e corrompe o entendimento do Modelo de Linguagem Grande (LLM) durante tarefas de Retrieval-Augmented Generation (RAG) [2]. A adoção da Árvore de Sintaxe Abstrata (AST) via ecossistema `tree-sitter` curou esse problema, permitindo extrações semânticas precisas.

Contudo, a adoção do `tree-sitter` abriu um vetor de colapso estrutural gravíssimo: as mais de 66 gramáticas oficiais do `tree-sitter` (para linguagens como Python, Go, JS, Rust) são escritas em C e C++ [1]. Executar código C nativo no *backend* por meio de *Foreign Function Interface* (C-FFI) significa que se o agente tentar analisar um arquivo exótico, malicioso ou com a sintaxe severamente quebrada, a biblioteca C pode sofrer uma falha de segmentação (*segfault*). No ecossistema *bare-metal* em Rust, um *segfault* originado no C não gera um `Result::Err` capturável; ele age como uma guilhotina que aniquila o processo inteiro de forma silenciosa, derrubando imediatamente o *Event Loop* assíncrono do Tokio [1].

#### Decisão Arquitetural (A Matriz de Parsing Blindada)
Fica decretado o fim do fatiamento cego e a blindagem cirúrgica do *parsing* AST, operando sob as seguintes leis inegociáveis:

**Módulo 1: O Fim do Fatiamento Estocástico**
*   Fica terminantemente proibido o uso de *chunking* cego ou por limites arbitrários de caracteres para ingestão de bases de código.
*   Todo o código-fonte deve ser interpretado estruturalmente, garantindo que as assinaturas das funções, suas implementações e *docstrings* correspondentes permaneçam atômicas e inseparáveis na base vetorial. O SOULS utilizará o `ast-grep` (nativo em Rust) para consultas semânticas no repositório [1].

**Módulo 2: A Jaula de Silício (Enjaulamento Wasmtime)**
*   Para fornecer suporte poliglota com segurança, é **ESTRITAMENTE PROIBIDO** carregar as gramáticas em C do `tree-sitter` como bibliotecas dinâmicas ou estáticas nativas vinculadas diretamente ao processo do Rust.
*   O SOULS exigirá que todas as gramáticas sintáticas sejam previamente compiladas para o formato **WebAssembly (WASM)** [1].
*   A execução do `tree-sitter` no SOULS operará obrigatoriamente algemada dentro do *runtime* `wasmtime`. Toda extração e leitura de AST de terceiros acontecerá dentro de *sandboxes* Wasm isoladas do ambiente hospedeiro.

**Módulo 3: Interceptação de Pânicos e Sobrevivência do Tokio**
*   Através deste enjaulamento, instaura-se o mecanismo de "Fail-Safe". Se a análise de um arquivo *slop* ou corrompido desencadear um *segfault* no código C do *parser*, o erro resultará apenas no colapso da *sandbox* WebAssembly.
*   A máquina virtual Wasm "morre silenciosamente", permitindo que o Rust capture o evento como um erro de execução padrão (`Trap`), salte a etapa daquele arquivo defeituoso e mantenha a esteira do *Event Loop* do Tokio girando impenetrável e ininterrupta [1].

#### Consequências Operacionais e Defesa contra o Slop (Trade-offs)
*   **Impacto Positivo:** Blindagem termodinâmica absoluta contra quedas do servidor causadas por código corrompido que o Agente ou o usuário decidam raspar. A estabilidade do SOULS V4 se aproxima dos 100%, garantindo o *uptime* necessário para operações 24/7 de um Sistema Operacional Agêntico [1].
*   **Impacto Negativo (Dívida de Toolchain):** Exigirá um esforço tático de compilação. Em vez de simplesmente baixar bibliotecas do mercado, o SOULS obriga a manutenção de *pipelines* que compilem as gramáticas C do `tree-sitter` em pacotes `.wasm`. Além disso, a troca de contexto entre o Rust e a *sandbox* Wasm introduz um *overhead* mínimo (em microssegundos), perfeitamente aceitável perante o ganho colossal de resiliência.
