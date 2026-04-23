# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 1 | Fontes Puras: 1 | Temas Identificados: 1

## 🧩 Eixo Temático 23: Arquitetura de Conhecimento Evolutivo com LLMs

### 1. O Problema com RAG Tradicional e a Necessidade de Persistência

A abordagem convencional de Retrieval-Augmented Generation (RAG) para interagir com LLMs, onde documentos são recuperados e fragmentos relevantes são apresentados ao modelo para gerar uma resposta, sofre de uma falha fundamental: a falta de **acumulação e síntese persistente de conhecimento**. Cada consulta é tratada como um evento isolado, forçando o LLM a "redescobrir" informações a cada interação. Isso resulta em uma ausência de memória real e na ineficiência de refazer o trabalho intelectual de conexão e síntese repetidamente.

### 2. A Proposta: LLM Wiki - Um Wiki Pessoal Autogerido

A arquitetura SODA adota o padrão "LLM Wiki", inspirado por Andrej Karpathy, para superar as limitações do RAG tradicional. Em vez de simplesmente recuperar documentos brutos, o LLM é empregado para **construir e manter ativamente um wiki persistente**. Este wiki, composto por arquivos Markdown estruturados e interligados, atua como uma camada intermediária entre as fontes originais e o usuário.

**O "PORQUÊ" da persistência:** Ao invés de apenas recuperar, o LLM, ao processar novas fontes, realiza as seguintes ações:
*   **Extração e Integração:** Identifica informações-chave e as integra ao wiki existente.
*   **Atualização Contextual:** Modifica páginas de entidades relacionadas para refletir as novas informações.
*   **Revisão e Síntese:** Atualiza sínteses existentes e sinaliza contradições entre fontes.

Essa abordagem transforma o LLM de uma ferramenta de consulta pontual em um **parceiro de curadoria intelectual contínua**. O wiki se torna um artefato composto, onde referências cruzadas e identificação de contradições são persistentes, eliminando a necessidade de refazer esse trabalho a cada consulta. O conhecimento cresce de forma **compounding**, ou seja, por uso e enriquecimento progressivo, não apenas por acumulação passiva.

### 3. Camadas da Arquitetura LLM Wiki

A arquitetura é dividida em três camadas distintas:

*   **Fontes Brutas:** Os documentos originais (artigos, notas, dados) que são imutáveis. O LLM apenas lê estas fontes.
*   **O Wiki:** Um diretório de arquivos Markdown gerados e mantidos pelo LLM. Contém resumos, páginas de entidades, sínteses temáticas e conexões entre conceitos. Esta é a representação estruturada e evolutiva do conhecimento.
*   **O Schema (CLAUDE.md):** Um documento de configuração que define as regras para o LLM, incluindo a estrutura do wiki, convenções de nomenclatura e fluxos de trabalho esperados.

### 4. Operações Principais do LLM Wiki

O padrão define três operações cruciais:

*   **Ingest:** Processamento de novas fontes. O LLM cria uma página de resumo, atualiza o índice e modifica páginas correlatas, automatizando um trabalho que seria intensivo para humanos.
*   **Query:** Ao receber uma pergunta, o LLM busca páginas relevantes **dentro do wiki** (e não nas fontes brutas), sintetiza uma resposta com citações e pode arquivar novos achados como páginas, promovendo o crescimento do conhecimento por exploração.
*   **Lint:** Uma operação de manutenção que verifica a saúde do wiki, identificando contradições, claims desatualizados, páginas órfãs e lacunas de cobertura. Funciona como um "compilador" para o conhecimento.

### 5. Arquivos de Controle Essenciais

Dois arquivos especiais gerenciam o wiki:

*   **index.md:** Um catálogo orientado ao conteúdo, organizado por categoria, fornecendo uma visão geral do wiki.
*   **log.md:** Um registro cronológico e append-only de todas as operações, permitindo auditoria detalhada.

### 6. Aplicações e Implicações

O padrão LLM Wiki é aplicável a diversos cenários, como rastreamento pessoal, pesquisa aprofundada, organização de leituras, wikis de equipe e análise competitiva. A relevância reside na sua capacidade de emular e expandir a memória humana através de associações automáticas, liberando o humano para focar na curadoria de fontes e na formulação de perguntas estratégicas. O LLM, por sua vez, executa o trabalho intensivo de síntese, arquivamento e manutenção de consistência de forma incansável e eficiente.

---
**Nota do Curador:** A proposta do LLM Wiki alinha-se perfeitamente com a filosofia SODA de um backend robusto e inteligente. A arquitetura de conhecimento evolutivo, onde o LLM atua como um agente de curadoria e síntese persistente, complementa a necessidade de um sistema que não apenas armazena dados, mas os enriquece e os torna semanticamente mais ricos ao longo do tempo. A persistência e a capacidade de "linting" do conhecimento são cruciais para a integridade e a evolução da base de dados soberana que o SODA visa construir.