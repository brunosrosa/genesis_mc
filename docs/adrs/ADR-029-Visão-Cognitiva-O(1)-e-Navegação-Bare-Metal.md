---
aliases:
  - "ADR-029: Visão Cognitiva O(1) e Navegação Bare-Metal"
---

# ADR-029: Visão Cognitiva O(1) e Navegação Bare-Metal

## Status

Aceito (Ativo, Inegociável e Fundacional para SODA V4)

## Contexto Técnico e Restrições de Entrada/Saída (I/O)

A execução de agentes agênticos na nova **Arquitetura SODA V4** é governada estritamente pelo "Pessimismo da Razão" sob as restrições físicas inalteráveis da dGPU NVIDIA RTX 2060m (com exatos $6.0 \text{ GB}$ de VRAM) e do processador central Intel i9 suportado por $32 \text{ GB}$ de RAM física.

Como estabelecido no [ADR-027](https://gemini.google.com/app/ADR-027-Motor-Hibrido-VRAM.md "null"), a integridade térmica e lógica do sistema operacional agêntico exige que a VRAM seja blindada contra o transbordo (_spillover_) pelo barramento PCIe Gen3 x8, dedicando-a exclusivamente aos pesos comprimidos do modelo em `IQ3_M` e ao KV Cache sínclono em `Q4_K`.

Sistemas agênticos industriais de mercado falham catastróficamente neste limite físico por duas razões de design:

- **Abstrações Obesas de Terceiros:** Ferramentas baseadas em Playwright, Node.js, Electron ou daemons em Python introduzem picos estocásticos de alocação de heap, latência de Garbage Collection (GC) incontornável e consumo descontrolado de memória RAM.
- **Processamento Analógico de Visão (VLMs):** Modelos de visão multimodal locais exigem processamento estocástico de imagens brutas (screenshots), gerando tensores de ativação imensos na GPU e consumindo janelas de atenção que inflacionam o KV Cache além do teto de segurança [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].

## Declaração do Problema

Como estruturar os mecanismos de navegação Web autônoma e parsing de documentos de larga escala no SODA V4 de forma que operem em tempo estrito de barramento $O(1)$, sem instanciar runtimes JavaScript interpretados, sem carregar um único pixel na memória de vídeo (VRAM) e mantendo a pegada de memória RAM central imutável perante arquivos densos [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md, uploaded:SODA Theme_01]?

## Decisões Arquiteturais da SODA V4

```
                                [ WEB PAGE / CDP STREAM ]
                                            |
                                            v
                              [ chaser-oxide (Tokio Client) ]
                                            |
                                (CDP: Accessibility Tree)
                                            |
                                            v
                              [ AXTree Semântica Bruta ]
                                            |
                             (Rust Parser: Lifetimes & Arena)
                                            |
                                            v
                             [ Formato LEAN: Dot-Flattening ]
                                            |
                             - 71% Menos Tokens de Entrada
                             - Coordenadas de Clique Geométricas
                                            |
                                            v
                              [ VRAM Headroom Intacto (~0 MB) ]
```

### 1. Automação Headless Tokio-Native (`chaser-oxide`)

Fica terminantemente banido o uso de Playwright, Selenium, Node.js, Python ou qualquer daemon web-server local em background no produto final.

- A navegação e a orquestração do navegador local head-less são gerenciadas exclusivamente pela crate **`chaser-oxide`** (um fork endurecido, otimizado e compilado em Rust da biblioteca `chromiumoxide`) [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].
- A comunicação com as instâncias do Chromium ocorre diretamente no runtime assíncrono Tokio através do protocolo CDP (_Chrome DevTools Protocol_) via WebSockets rápidos locais [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].
- **Bypass de Anti-Bot Nativo:** A crate `chaser-oxide` manipula ativamente as chamadas do protocolo de transporte ao nível de assinaturas TCP e injeção sínclona de cabeçalhos HTTP na camada do cliente Rust, evitando o bloqueio por Web Application Firewalls (WAFs como Cloudflare Turnstile e Datadome) sem a sobrecarga computacional de carregar motores de emulação visual pesados ou renderizadores externos [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].

### 2. Visão Cognitiva O(1) e o Formato LEAN (LLM-Efficient Adaptive Notation)

Nossos agentes locais rejeitam a análise baseada em pixels ou imagens matriciais. Visando poupar $100\%$ do poder de processamento térmico da RTX 2060m, a visão dos agentes SODA é puramente estrutural e matemática [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].

- **Extração de AXTree:** O SODA V4 extrai síncronamente do CDP a Árvore de Acessibilidade (`Accessibility.getFullAXTree`), contendo apenas as primitivas semânticas e os elementos focáveis/interativos do layout, ignorando completamente CSS decorativo, tags de estilo vazias, mídias e scripts [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].
- **Compactação LEAN:** A AXTree bruta é processada em memória por um parser zero-copy nativo (baseado em lifetimes Rust `&'a str` e na crate `nom`) que converte a estrutura redundante da página para a especificação **LEAN** [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md, uploaded:SODA V3: Arquitetura de Inferência Híbrida].
    - _Dot-Flattening:_ O aninhamento profundo de nós DOM é aplanado em caminhos lineares diretos usando notação por pontos (ex: `root.main.form.input_user`).
    - _Booleanos Literais:_ Atributos de estado são traduzidos para caracteres únicos: `T` (verdadeiro/ativo), `F` (falso/desabilitado) e `_` (nulo ou irrelevante).

#### Equação de Deflação de Tokens de Entrada:

Seja $T_{\text{HTML}}$ o volume de tokens gerado pela representação clássica do DOM e $T_{\text{LEAN}}$ a codificação aplanada sob SODA V4. O motor impõe a seguinte restrição estatística de compressão:

$$T_{\text{LEAN}} \le 0.29 \times T_{\text{HTML}}$$

Isso representa uma deflação síncrona mínima de $71\%$ **no consumo de tokens**.

- **Mapeamento Geométrico Espacial:** Para interagir com a interface (cliques, digitação), o agente SODA V4 consulta síncronamente o modelo de caixas do elemento focado (`DOM.getBoxModel`). O ponto físico de impacto para clique é calculado geometricamente por média linear dos vértices sem rendering de tela:

$$x_{\text{clique}} = \frac{x_0 + x_2}{2}$$$$y_{\text{clique}} = \frac{y_0 + y_2}{2}$$

Os cliques operam com precisão de silício sub-milisegundo sem gastar um único byte de memória de vídeo [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].

### 3. O Bypass de OOM Documental (Kreuzberg Streaming e pdfsink-rs)

Para impedir picos letais de memória RAM central ($> 2.5\text{ GB}$) que causem o pânico do Tokio Event Loop e ativem o Linux OOM Killer no hospedeiro, o processamento de arquivos pesados (PDFs, planilhas massivas) é redimensionado para operar em fluxo sínclono linearizado.

- **Streaming via `Kreuzberg`:** O SODA V4 veta chamadas de leitura de arquivos que exijam buffering integral na RAM do host. O motor `Kreuzberg` (embarcado via static-linking com PDFium) deve ser instanciado estritamente através do mecanismo de streaming por fatias iterativas (`PdfParser::for_each_page`). Cada página é digerida, vetorizada e salva individualmente no SQLite (Fase 1.5), limpando os buffers da página anterior antes de alocar a seguinte. A pegada de memória do processo permanece estática e em complexidade constante:

$$\mathcal{O}(1)$$

- **Alocação Zero via `pdfsink-rs`:** A extração de dados brutos de tabelas duras e dados altamente indexados é feita através da crate `pdfsink-rs`. Esta biblioteca utiliza de forma massiva mapeamento de memória direta (`mmap`) e referências temporais com lifetimes (`&'a str`) diretamente conectadas ao arquivo mapeado [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md]. Não ocorrem alocações desnecessárias no Heap do Rust para strings, reduzindo picos computacionais térmicos na CPU e mantendo o throughput máximo do SSD NVMe [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].

## Consequências e Trade-offs

### Impactos Positivos:

- **Headroom de RAM e VRAM Preservado:** A dGPU RTX 2060m opera livre de tarefas gráficas de navegação ($0\text{ MB}$ de impacto), e o uso de RAM central fica abaixo do cgroup estrito do SODA.
- **Estabilidade do Tokio Event Loop:** Eliminação de latências estocásticas induzidas pelo bloqueio de threads assíncronas por parsers síncronos de arquivos gigantescos.
- **Imunidade a WAFs e Bots:** Acesso resiliente a fontes web locais-first sem depender de proxificações de nuvem lentas [DEPENDENCIES] Arquitetura Agente Rust Bare-Metal (Navegação e Parsing O(1)).md].

### Impactos Negativos:

- **Cegueira Decorativa do Agente:** O SODA é blindado contra imagens decorativas, Canvas gráficos analógicos, e folha de estilos (CSS) não estruturadas. Se uma página requer inferência baseada estritamente na renderização artística visual de imagens para ser compreendida, a tarefa falha de forma limpa.
- **Custo de CPU Local:** A CPU i9 assume a responsabilidade síncrona de rodar o parser zero-copy das árvores do Chromiumoxide, operando em pico de registradores AVX2. O impacto é absorvido pela separação lógica de núcleos (_CPU Core Affinity_) do SODA Daemon.

### Comportamento Fail-Closed

Caso o buffer de dados estruturados gerado na descompressão de uma página web ou arquivo ultrapasse o teto físico imposto pelas restrições do cgroup do processo worker, o SODA executa um descarte atômico sínclono (`SIGKILL`), cancelando a extração local e gravando um log seco na tabela `SYSTEM_TELEMETRY` antes de abortar de forma segura.