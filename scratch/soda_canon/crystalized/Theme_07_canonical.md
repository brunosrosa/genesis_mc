# SODA Canonical Knowledge Base - Manual de Arquitetura

## Eixo Temático 7: Processamento e Inteligência de Documentos

Este manual consolida as informações sobre o processamento e a inteligência de documentos, com foco em como essas capacidades se integram à arquitetura SODA. A arquitetura SODA prioriza Rust (Tokio) no backend e Svelte 5 + Tauri v2 no frontend, com o frontend atuando como uma interface passiva. Toda a lógica de processamento de documentos reside no backend Rust, utilizando comunicação interprocessos (IPC) de cópia zero.

### 1. O "PORQUÊ" da Inteligência de Documentos no SODA

A capacidade de processar e extrair inteligência de diversos formatos de documentos é fundamental para a arquitetura SODA. Isso permite que o sistema:

*   **Ingira e Compreenda Dados Diversos:** Transformar documentos não estruturados ou semiestruturados (PDFs, Office, imagens, áudio) em formatos utilizáveis para análise e processamento posterior.
*   **Potencialize Aplicações de IA Generativa:** Fornecer dados contextuais e estruturados para modelos de IA, melhorando a precisão e a relevância das respostas.
*   **Automatize Tarefas Complexas:** Reduzir a necessidade de intervenção manual na extração de informações de documentos, como relatórios financeiros, patentes ou artigos científicos.
*   **Garanta a Privacidade e Segurança:** Permitir o processamento local de dados sensíveis, sem a necessidade de envio para serviços externos.

### 2. Ferramentas e Tecnologias Chave

As bibliotecas **Docling** e **Kreuzberg** emergem como pilares para a inteligência de documentos dentro do ecossistema SODA. Ambas são escritas em Rust, alinhando-se perfeitamente com a stack tecnológica do backend SODA.

#### 2.1. Docling

*   **Foco:** Simplificação do processamento de documentos para o ecossistema de IA generativa.
*   **Capacidades Principais:**
    *   **Parsing Abrangente:** Suporta uma vasta gama de formatos, incluindo PDFs avançados (layout, ordem de leitura, tabelas, código, fórmulas), DOCX, PPTX, XLSX, HTML, áudio (WAV, MP3), imagens, LaTeX, texto puro e mais.
    *   **Representação Unificada:** Formato `DoclingDocument` para uma representação expressiva e unificada dos documentos processados.
    *   **Integrações:** Plug-and-play com frameworks de IA como LangChain, LlamaIndex, Crew AI e Haystack.
    *   **Execução Local:** Capacidade de execução local para dados sensíveis e ambientes air-gapped.
    *   **OCR Extensivo:** Suporte a OCR para PDFs escaneados e imagens.
    *   **Modelos de Linguagem Visual (VLMs):** Suporte a VLMs como GraniteDocling.
    *   **Suporte a Áudio:** Modelos de Reconhecimento Automático de Fala (ASR).
    *   **Servidor MCP:** Para aplicações agentic.
    *   **Extração Estruturada:** Capacidade beta de extração de informações estruturadas.
    *   **Modelos de Layout Avançados:** Utiliza o modelo Heron para parsing mais rápido de PDFs.
    *   **Formatos Específicos:** Suporte a XML para patentes USPTO, artigos JATS e relatórios financeiros XBRL.
    *   **Entendimento de Gráficos:** Conversão de gráficos (barras, pizza, linhas) em tabelas, código ou descrições.
    *   **Entendimento de Química:** Extração de estruturas moleculares.
*   **Considerações para SODA:** Docling oferece uma solução robusta e flexível para a ingestão e pré-processamento de documentos. Sua capacidade de execução local é um diferencial para a arquitetura SODA, que visa manter a lógica e os dados no backend.

#### 2.2. Kreuzberg

*   **Foco:** Biblioteca de inteligência de documentos de alta performance escrita em Rust.
*   **Capacidades Principais:**
    *   **Performance Nativa:** Core em Rust com otimizações SIMD e paralelismo completo.
    *   **Ampla Suporte a Formatos:** Extrai texto, metadados e inteligência de código de mais de 97 formatos de arquivo.
    *   **Inteligência de Código:** Integração com `tree-sitter-language-pack` para análise de código-fonte em 248 linguagens de programação, extraindo funções, classes, imports, símbolos, diagnósticos e trechos semânticos.
    *   **Inteligência LLM:** Suporte a VLM OCR (GPT-4o, Claude, Gemini, Ollama), extração estruturada de JSON com restrições de esquema e embeddings via provedores LLM (incluindo locais como Ollama, LM Studio, vLLM, llama.cpp) através de `liter-llm`.
    *   **OCR:** Múltiplos backends (Tesseract, PaddleOCR, EasyOCR), com detecção e reconstrução inteligente de tabelas.
    *   **Bindings Poliglota:** Bindings nativos para Rust, Python, TypeScript/Node.js, Ruby, Go, Java, C#, PHP, Elixir, R e C.
    *   **Formatos de Saída:** Saída GFM-quality (Markdown, HTML, Djot, Plain) e formato TOON (serialização eficiente para LLM/RAG).
    *   **Processamento em Lote:** Processamento concorrente de múltiplos documentos com paralelismo configurável.
    *   **PDFs Protegidos por Senha:** Suporte para PDFs criptografados.
    *   **Detecção de Idioma:** Detecção automática de idioma no texto extraído.
    *   **Extração de Metadados:** Extração abrangente de metadados.
    *   **Servidor MCP:** Capacidade de servir como servidor MCP.
*   **Considerações para SODA:** Kreuzberg complementa Docling com um foco ainda maior em performance e inteligência de código. Sua arquitetura extensível e os bindings políglotas facilitam a integração com o backend Rust do SODA. A capacidade de processar documentos sem a necessidade de GPU é um ponto forte.

### 3. Integração com a Arquitetura SODA

A integração dessas bibliotecas no SODA deve seguir os princípios de ARQUITETURA PURA:

*   **Backend Rust (Tokio):** A lógica de processamento de documentos, incluindo a chamada às APIs de Docling e Kreuzberg, deve residir no backend Rust. Isso garante que toda a complexidade e o processamento intensivo de dados sejam tratados onde a performance é crítica e onde as otimizações de baixo nível podem ser aplicadas.
*   **IPC Zero-Copy:** A comunicação entre o frontend Svelte/Tauri e o backend Rust deve ser otimizada com IPC de cópia zero para transferir dados de documentos processados de forma eficiente.
*   **Frontend Passivo (Svelte 5 + Tauri v2):** O frontend será responsável por apresentar os resultados do processamento de documentos, interagir com o usuário para iniciar tarefas de processamento e exibir o estado dessas tarefas. Ele não conterá lógica de processamento de documentos em si.
*   **Otimizações de Hardware:**
    *   **CPU (AVX2):** As bibliotecas Rust (Docling e Kreuzberg) podem se beneficiar de instruções AVX2 para aceleração de operações computacionais. O compilador Rust e as bibliotecas subjacentes devem ser configurados para tirar proveito disso.
    *   **GPU (RTX 2060m / llama.cpp mmap):** Para tarefas que envolvem modelos de IA (como VLM OCR ou embeddings), a utilização de `llama.cpp` com `mmap` pode ser explorada para otimizar o uso da VRAM da RTX 2060m, especialmente para inferência de modelos menores ou para carregar partes de modelos maiores na memória. É crucial monitorar gargalos de barramento entre a CPU, RAM e a iGPU.
    *   **Otimizações Bare-metal:** O uso de Rust permite otimizações de baixo nível e controle sobre o gerenciamento de memória, aproximando-se de um desempenho bare-metal.

### 4. Auditoria Crítica e Pontos de Atenção

*   **Dependências de PDFium:** Kreuzberg oferece diferentes estratégias de linking para PDFium (Default/Dynamic, Static, Bundled, System). Para SODA, a estratégia **Bundled** ou **Static** é preferível para garantir que a aplicação seja autossuficiente e não dependa de uma instalação de PDFium no sistema do usuário, o que simplifica a distribuição e a consistência. A estratégia `pdf-bundled` do Docling também é uma boa opção.
*   **ONNX Runtime:** A funcionalidade de embeddings em Kreuzberg requer a instalação do ONNX Runtime. Isso representa uma dependência externa que precisa ser gerenciada. Para SODA, a recomendação é que essa dependência seja tratada no ambiente de execução do backend Rust, possivelmente através de contêineres ou instruções de instalação claras.
*   **Licenciamento:** Docling é MIT. Kreuzberg é ELv2. É crucial garantir que essas licenças sejam compatíveis com o modelo de licenciamento do projeto SODA e que os termos sejam cumpridos.
*   **Complexidade de Configuração:** Embora ambas as bibliotecas sejam poderosas, a configuração de recursos avançados (como diferentes backends de OCR, VLMs específicos ou otimizações de PDFium) pode adicionar complexidade. O SODA deve abstrair essa complexidade para o usuário final, fornecendo configurações padrão robustas.
*   **Performance de OCR em iGPU:** A menção a "otimizações bare-metal" e "limitações da iGPU" sugere que o processamento de OCR, especialmente com modelos maiores ou em tempo real, pode ser um gargalo. A arquitetura SODA deve considerar estratégias para descarregar tarefas de OCR para a CPU (utilizando AVX2) ou para a GPU (se houver suporte e otimização via `llama.cpp` ou bibliotecas similares), monitorando de perto o uso de recursos e a latência.
*   **"Furo" Potencial - VDOM e Server-Side Rendering:** As fontes mencionam integrações com frameworks como LangChain, LlamaIndex, Crew AI, Haystack e até mesmo a possibilidade de um servidor REST API ou MCP. É **crucial** que a arquitetura SODA **evite** qualquer arquitetura que envolva VDOM no frontend ou Server-Side Rendering (SSR) para a lógica de processamento de documentos. Toda a lógica de processamento deve ser executada no backend Rust. O frontend Svelte/Tauri deve ser puramente uma interface de usuário para interagir com o backend. A menção a "agentic AI" e "MCP server" deve ser interpretada como um mecanismo de comunicação entre processos ou serviços dentro do ecossistema SODA, e não como uma arquitetura web tradicional.

### 5. Diretrizes de Implementação para SODA

1.  **Abstração de Bibliotecas:** Criar módulos Rust no backend SODA que encapsulem as funcionalidades de Docling e Kreuzberg. Isso permite a troca de implementações no futuro, se necessário, e mantém a lógica de processamento de documentos isolada.
2.  **IPC Otimizado:** Desenvolver mecanismos de IPC (provavelmente via `tauri::invoke` ou canais Tokio) que minimizem a cópia de dados ao transferir documentos ou resultados de processamento entre o frontend e o backend.
3.  **Gerenciamento de Dependências:** Para Kreuzberg, priorizar a estratégia de linking `pdf-bundled` ou `static-pdfium` para garantir a portabilidade e a autossuficiência da aplicação. Gerenciar a dependência do ONNX Runtime de forma robusta.
4.  **Otimização de Hardware:**
    *   Explorar flags de compilação Rust para habilitar otimizações AVX2 na CPU.
    *   Investigar o uso de `llama.cpp` com `mmap` para inferência de modelos de IA na RTX 2060m, focando em tarefas específicas onde a GPU pode oferecer um benefício significativo.
    *   Monitorar o desempenho e identificar gargalos de barramento.
5.  **Segurança e Privacidade:** Utilizar as capacidades de execução local de Docling e Kreuzberg para processar documentos sensíveis diretamente no dispositivo do usuário, sem a necessidade de upload.
6.  **Interface do Usuário (Frontend):** O frontend Svelte/Tauri deve focar em:
    *   Permitir que o usuário selecione documentos para processamento.
    *   Exibir o progresso e os resultados do processamento.
    *   Fornecer opções de configuração simples para os recursos de processamento de documentos.
    *   **Não** implementar lógica de parsing, OCR ou extração de dados.

Ao aderir a estas diretrizes, o SODA pode integrar de forma eficaz as poderosas capacidades de processamento de documentos oferecidas por Docling e Kreuzberg, mantendo sua arquitetura pura, performática e segura.