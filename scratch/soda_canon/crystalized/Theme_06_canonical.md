# Manual Canônico da Arquitetura SODA (Sovereign Operating Data Architecture - Genesis MC)

## 1. Introdução

Este manual descreve a arquitetura do projeto SODA (Sovereign Operating Data Architecture), com foco em sua implementação e princípios fundamentais. O SODA é projetado para ser uma plataforma de dados soberana, utilizando tecnologias de ponta para garantir desempenho, segurança e escalabilidade.

## 2. Princípios Arquiteturais Fundamentais

### 2.1. Linguagem e Frameworks

*   **Backend:** Estritamente **Rust** com o framework **Tokio**. Esta escolha garante alta performance, concorrência segura e gerenciamento eficiente de recursos, características essenciais para um sistema de dados robusto.
*   **Frontend:** Utiliza **Svelte 5** com **Tauri v2**. O Svelte 5 oferece um framework reativo e performático para a interface do usuário, enquanto o Tauri v2 permite a criação de aplicações desktop nativas com segurança e eficiência, aproveitando a performance do Rust.
*   **Comunicação Frontend-Backend:** A interface do frontend é **passiva**. Toda a lógica de negócios e processamento de dados reside no backend Rust. A comunicação entre frontend e backend é realizada através de **IPC (Inter-Process Communication) Zero-Copy**, minimizando a latência e a sobrecarga de serialização/desserialização de dados.

### 2.2. Otimização de Hardware e Performance

A arquitetura SODA é projetada com uma forte ênfase na otimização de hardware para maximizar o desempenho:

*   **Otimizações Bare-Metal:** O uso de Rust e Tokio permite um controle granular sobre os recursos do sistema, possibilitando otimizações em nível de "bare-metal" para tarefas computacionalmente intensivas.
*   **Gerenciamento de GPU:**
    *   **iGPU (Integrated Graphics Processing Unit):** Reconhece-se a existência de gargalos de barramento associados ao uso de iGPUs, que podem limitar a taxa de transferência de dados em operações de alta demanda.
    *   **AVX2:** Otimizações de CPU são exploradas através de diretivas de execução como AVX2, aproveitando instruções SIMD (Single Instruction, Multiple Data) para acelerar cálculos paralelos.
    *   **llama.cpp mmap:** Para cenários específicos, como a utilização de RTX 2060m, a técnica de `mmap` (memory mapping) com `llama.cpp` é considerada para otimizar o acesso à memória da GPU, especialmente em cargas de trabalho de inferência de modelos de linguagem.

### 2.3. Protocolos e Comunicação

*   **Model Context Protocol (MCP):** O SODA adota o MCP como protocolo principal para a comunicação entre agentes e serviços, permitindo a troca estruturada de contexto e dados.
*   **Agent-to-Agent (A2A) Protocol:** Para facilitar a colaboração e comunicação entre agentes autônomos, o protocolo A2A é suportado, permitindo a descoberta de capacidades e a coordenação de tarefas.

## 3. Componentes e Configuração

### 3.1. Agent Gateway (`gateway-config.yaml`, `mcp_config.json`)

O Agent Gateway atua como o ponto de entrada e orquestrador para os serviços de IA. Sua configuração define:

*   **Porta de Escuta:** O gateway escuta na porta `3000` via HTTP.
*   **Rotas e Políticas:**
    *   **CORS:** Configuração de Cross-Origin Resource Sharing.
    *   **MCP Authorization:** Regras de autorização baseadas em CEL (Common Expression Language) para controlar o acesso a ferramentas e recursos MCP. As regras permitem ou negam acesso com base em condições específicas, como a ausência ou presença de certas ferramentas (`!has(mcp.tool)`) ou a correspondência de nomes de ferramentas com padrões (`mcp.tool.name.matches(...)`).
    *   **Ferramentas MCP:** O gateway gerencia e expõe diversas ferramentas MCP, incluindo:
        *   `lean-ctx`: Para contexto de linguagem.
        *   `sequential_thinking`: Para processamento sequencial.
        *   `memory-mcp-rs`: Para funcionalidades de memória.
        *   `jcodemunch`: Utilitário genérico.
        *   `time_server`: Para obter o tempo atual.
        *   `duckduckgo_search`: Para realizar buscas na web.
        *   `github_mcp`: Para interagir com o GitHub.
        *   `notebooklm`: Para funcionalidades de linguagem em notebooks.
        *   `sqlite_soda`: Para acesso a banco de dados SQLite.
        *   `docs_scraper`: Para raspagem de documentação.
*   **Configuração de Servidores MCP (`mcp_config.json`):**
    *   **`agent-gateway`:** Configurado para ser iniciado via `npx mcp-remote`, com variáveis de ambiente como `AG_BRIDGE_URL`, `GPU_RASTERIZATION` e `ZERO_COPY_MODE` ativadas.
    *   **`docling_extractor`:** Executado via Docker, montando o diretório de projetos e utilizando `stdio` como transporte.
*   **Dockerfile (`dockerfile.docling`):** Define a imagem base (`ghcr.io/astral-sh/uv:bookworm-slim`), otimizações de compilação (`UV_COMPILE_BYTECODE=1`, `UV_LINK_MODE=copy`), dependências de sistema (`libgl1`, `libglib2.0-0`) e a instalação do `docling-mcp-server`.

### 3.2. Common Expression Language (CEL)

O CEL é amplamente utilizado no Agent Gateway para definir políticas dinâmicas e flexíveis. Ele permite a avaliação de expressões em tempo de execução com base no contexto da requisição, incluindo headers, claims de JWT e atributos de LLM.

*   **Variáveis Disponíveis:** `request`, `response`, `env`, `jwt`, `apiKey`, `basicAuth`, `llm`, `source`, `mcp`, `backend`, `extauthz`, `extproc`, `metadata`.
*   **Funções:** Inclui funções para manipulação de JSON (`json`, `toJson`), strings (`contains`, `split`, `regexReplace`), e outras utilidades (`default`, `coalesce`, `uuid`).
*   **Casos de Uso:**
    *   **Fallback de Valores:** `default(request.headers["x-user-id"], "anonymous")`
    *   **Autorização:** `jwt.sub == "test-user" && mcp.tool.name == "add"`
    *   **Rate Limiting:** Baseado em headers ou claims de JWT.

## 4. Considerações de Segurança e Governança

*   **Centralização de Chaves:** O gateway centraliza o gerenciamento de chaves de provedores de LLM, com controle de acesso granular.
*   **Guardrails:** Implementação de guardrails inline para mitigar ataques de prompt e vazamentos de dados.
*   **Auditoria:** Rastreamento completo de interações de agentes e chamadas de ferramentas para fins de conformidade e depuração.
*   **Políticas de Acesso:** Aplicação de políticas globais de rate limiting, cotas e controle de acesso.

## 5. Furos e Pontos de Atenção

*   **Dependência de `npx` e `uvx.exe`:** A configuração do Agent Gateway depende da disponibilidade de `npx` e `uvx.exe`. A ausência ou incompatibilidade dessas ferramentas pode impedir o funcionamento dos MCP servers.
*   **Gerenciamento de Dependências:** A configuração `mcp_config.json` especifica o uso de `docker` para `docling_extractor`. A correta instalação e configuração do Docker é crucial.
*   **Segurança de `GITHUB_PAT`:** A variável de ambiente `GITHUB_PAT` para autenticação no GitHub deve ser gerenciada de forma segura, evitando exposição em logs ou controle de versão.
*   **Otimização de iGPU:** Embora mencionado, o impacto exato e as estratégias de mitigação para gargalos de barramento de iGPU em cargas de trabalho intensivas de IA podem requerer otimizações adicionais ou hardware dedicado.
*   **Complexidade da Configuração:** A configuração do Agent Gateway é extensa e detalhada, exigindo atenção cuidadosa para garantir a correta interconexão dos serviços.

## 6. Conclusão

A arquitetura SODA, com seu foco em Rust/Tokio e Svelte/Tauri, juntamente com o uso de protocolos como MCP e A2A, posiciona-se como uma solução de alta performance e segurança para o desenvolvimento de aplicações de IA soberanas. A clara separação de responsabilidades entre backend e frontend, e a comunicação eficiente via IPC Zero-Copy, são pilares desta arquitetura. A atenção aos detalhes de configuração e às considerações de hardware é fundamental para a sua correta implementação e operação.

---

## SODA (Sovereign Operating Data Architecture) - Manual Canônico - Parte 2

Este documento detalha os axiomas, regras e conceitos técnicos que regem a arquitetura SODA, com foco nas funcionalidades de políticas e backends.

### Axiomas e Regras Fundamentais

1.  **Rust (Tokio) Backend, Svelte 5 + Tauri v2 Frontend:** A espinha dorsal da SODA é construída com Rust, utilizando o framework Tokio para concorrência assíncrona. A interface do usuário é desenvolvida com Svelte 5, empacotada em Tauri v2.
2.  **Frontend Passivo:** A interface do usuário (Svelte) atua estritamente como um canal de apresentação de dados. Toda a lógica de negócios, processamento e tomada de decisão reside no backend Rust.
3.  **IPC Zero-Copy:** A comunicação entre o frontend e o backend é otimizada através de Inter-Process Communication (IPC) com zero-copy, garantindo a máxima eficiência na transferência de dados.
4.  **Exclusão de Tecnologias Não Conformes:** Tutoriais ou menções a React, Node.js daemons, Electron, VDOM ou Server-Side Rendering (Next.js) são estritamente proibidos e devem ser ignorados.

### Conceitos Técnicos e Diretrizes

#### Políticas de Segurança e Processamento de IA

A SODA implementa um sistema robusto de políticas para gerenciar e proteger as interações com modelos de IA e outros serviços.

*   **`policies[].policy.ai.promptGuard`**: Configurações para proteger prompts de IA.
    *   **`response`**: Define as ações a serem tomadas com base na análise de prompts.
        *   **`azureContentSafety`**: Integração com o Azure AI Content Safety para detecção de conteúdo nocivo e tentativas de jailbreak.
            *   **`analyzeText`**: Configuração para análise de texto, incluindo categorias de severidade (`severityThreshold`), versões de API (`apiVersion`), listas de bloqueio (`blocklistNames`) e lógica de interrupção (`haltOnBlocklistHit`).
            *   **`detectJailbreak`**: Configuração específica para detecção de jailbreak, com sua própria `apiVersion`.
        *   **`rejection`**: Define a resposta a ser retornada quando uma política de segurança é violada, incluindo corpo (`body`), status HTTP (`status`) e cabeçalhos (`headers`).
    *   **`defaults`**: Configurações padrão aplicáveis a todas as políticas de IA.
    *   **`overrides`**: Permite substituir configurações padrão para políticas específicas.
    *   **`transformations`**: Aplica transformações aos prompts antes do processamento.
    *   **`prompts`**: Permite a adição (`append`) ou preposição (`prepend`) de conteúdo aos prompts, com definição de `role` e `content`.
    *   **`modelAliases`**: Mapeamento de aliases para modelos de IA.
    *   **`promptCaching`**: Configurações para cache de prompts, incluindo opções para cache de sistema (`cacheSystem`), mensagens (`cacheMessages`), ferramentas (`cacheTools`), tokens mínimos (`minTokens`) e offset de mensagens (`cacheMessageOffset`).
    *   **`routes`**: Define rotas específicas para o processamento de IA.

*   **`policies[].policy.backendTLS`**: Configurações para comunicação segura via TLS com backends. Inclui opções para certificados (`cert`), chaves (`key`), CAs raiz (`root`), hostname (`hostname`), modo inseguro (`insecure`, `insecureHost`), ALPN (`alpn`) e Subject Alternative Names (`subjectAltNames`).

*   **`policies[].policy.backendTunnel`**: Configuração para tunelamento de tráfego para backends.
    *   **`proxy`**: Define o endereço do proxy, permitindo a especificação de um `service` (com `name` contendo `namespace` e `hostname`, e `port`), um `host` direto, ou uma referência a um `backend` definido globalmente.

*   **`policies[].policy.backendAuth`**: Mecanismos de autenticação para backends.
    *   **`passthrough`**: Passagem de credenciais de autenticação existentes, especificando a `location` (header, queryParameter, cookie) e seus detalhes (`name`, `prefix`).
    *   **`key`**: Autenticação usando uma chave, com opções para valor (`value` via `file`) e `location` (header, queryParameter, cookie).
    *   **`gcp`**: Autenticação com Google Cloud, suportando `idToken` ou `accessToken`, com especificação de `audience`.
    *   **`aws`**: Autenticação com AWS, permitindo `accessKeyId`, `secretAccessKey`, `region` e `sessionToken`.
    *   **`azure`**: Autenticação com Azure, com opções para configuração explícita (`explicitConfig` com `clientSecret`, `managedIdentity`, `workloadIdentity`), implícita de desenvolvedor (`developerImplicit`) ou implícita geral (`implicit`).

*   **`policies[].policy.localRateLimit`**: Implementação de rate limiting com estado local. Configurações incluem `maxTokens`, `tokensPerFill`, `fillInterval` e `type` (requests ou tokens).

*   **`policies[].policy.remoteRateLimit`**: Implementação de rate limiting com estado gerenciado remotamente.
    *   **`service`**: Define o serviço de rate limiting (com `name`, `namespace`, `hostname`, `port`).
    *   **`host`**: Endereço do host do serviço de rate limiting.
    *   **`backend`**: Referência explícita a um backend.
    *   **`domain`**: Domínio associado ao rate limiting.
    *   **`policies`**: Políticas adicionais para a comunicação com o serviço de rate limiting, incluindo modificadores de cabeçalho (`requestHeaderModifier`), transformações (`transformations`), TLS (`backendTLS`) e autenticação (`backendAuth`).
    *   **`descriptors`**: Define os descritores para o rate limiting, com `entries` (key/value), `type` e `limitOverride`.
    *   **`failureMode`**: Comportamento em caso de falha do serviço de rate limiting (`failClosed` ou `failOpen`).

*   **`policies[].policy.jwtAuth`**: Autenticação baseada em JSON Web Tokens (JWT).
    *   **`mode`**: Modo de validação (`strict`, `optional`, `permissive`).
    *   **`location`**: Onde o JWT é esperado (header, queryParameter, cookie).
    *   **`providers`**: Lista de provedores de JWT, cada um com `issuer`, `audiences`, `jwks` (file ou url) e `jwtValidationOptions` (incluindo `requiredClaims`).

*   **`policies[].policy.oidc`**: Autenticação via OpenID Connect (OIDC) para fluxos de navegador.
    *   Configurações incluem `issuer`, `discovery` (file ou url), `authorizationEndpoint`, `tokenEndpoint`, `tokenEndpointAuth`, `jwks` (file ou url), `clientId`, `clientSecret`, `redirectURI` e `scopes`.

*   **`policies[].policy.basicAuth`**: Autenticação básica com arquivo htpasswd.
    *   Configurações incluem `htpasswd` (file), `realm`, `mode` (strict, optional) e `authorizationLocation`.

*   **`policies[].policy.apiKey`**: Autenticação baseada em chaves de API.
    *   Configurações incluem `keys` (lista de chaves com metadados), `mode` (strict, optional) e `location` (header, queryParameter, cookie).

*   **`policies[].policy.extAuthz`**: Autorização externa via chamadas a um servidor de autorização.
    *   Configurações incluem `service` (com `name`, `namespace`, `hostname`, `port`), `host`, `backend`, `policies` (semelhantes a `backendTunnel` e `backendAuth`), `protocol` (gRPC ou HTTP), `failureMode` e opções de inclusão de cabeçalhos (`includeRequestHeaders`) e corpo (`includeRequestBody`).

*   **`policies[].policy.extProc`**: Extensão com processadores externos.
    *   Configurações incluem `service`, `host`, `backend`, `policies` (semelhantes a `backendTunnel` e `backendAuth`), `failureMode`, `metadataContext` e `requestAttributes`/`responseAttributes`.

*   **`policies[].policy.transformations`**: Aplicação de transformações em requisições e respostas.
    *   **`request`**: Modificações no cabeçalho (`add`, `set`, `remove`), corpo (`body`) e metadados (`metadata`).
    *   **`response`**: Modificações no cabeçalho (`add`, `set`, `remove`), corpo (`body`) e metadados (`metadata`).

*   **`policies[].policy.csrf`**: Proteção contra Cross-Site Request Forgery (CSRF) com validação de origens.

*   **`policies[].policy.timeout`**: Configuração de timeouts para requisições e backends.

*   **`policies[].policy.retry`**: Política de retentativas para requisições, com configurações de `attempts`, `backoff`, `codes`.

#### Backends e Grupos de IA

A SODA suporta a definição de múltiplos backends, incluindo aqueles para cargas de trabalho de IA.

*   **`backends`**: Lista de definições de backends.
    *   **`host`**: O hostname do backend.
    *   **`mcp`**: Configurações para o Management Cloud Platform (MCP).
        *   **`targets`**: Lista de alvos para o MCP, incluindo `sse`, `mcp`, `stdio` e `openapi`.
        *   **`statefulMode`**: Modo de operação do MCP (`stateless` ou `stateful`).
        *   **`prefixMode`**: Modo de prefixo do MCP (`always`, `conditional`, `null`).
        *   **`failureMode`**: Comportamento em caso de falha do MCP (`failClosed` ou `failOpen`).
    *   **`ai`**: Configurações específicas para backends de IA.
        *   **`name`**: Nome do backend de IA.
        *   **`provider`**: Define o provedor de IA (`openAI`, `gemini`, `vertex`, `anthropic`, `bedrock`, `azure`), com configurações específicas para cada um (ex: `model`, `region`, `projectId`, `resourceName`, `resourceType`, `projectName`).
        *   **`hostOverride`**: Permite sobrescrever o host do provedor de IA.
        *   **`pathOverride`**: Permite sobrescrever o path do provedor de IA.
        *   **`pathPrefix`**: Permite definir um prefixo de path para o provedor de IA.
        *   **`tokenize`**: Habilita a tokenização para estimativa de custo e rate limiting.
        *   **`policies`**: Políticas aplicáveis ao backend de IA, incluindo modificações de cabeçalho, transformações, TLS, autenticação, health checks, autorização MCP, A2A, roteamento de inferência e proteção de prompt (`promptGuard`).
            *   **`promptGuard`**: Detalha as políticas de proteção de prompt para requisições e respostas, incluindo `regex`, `webhook`, `openAIModeration`, `bedrockGuardrails` e `googleModelArmor`.
            *   **`azureContentSafety`**: Configuração específica para Azure Content Safety, com `endpoint`, `analyzeText` e `detectJailbreak`.
        *   **`groups`**: Permite agrupar múltiplos provedores de IA sob um único backend, com políticas e configurações de fallback.

#### Roteamento e Políticas de Rota

A SODA utiliza um sistema de roteamento flexível para direcionar requisições e aplicar políticas.

*   **`routeGroups`**: Agrupamento de rotas.
    *   **`name`**: Nome do grupo de rotas.
    *   **`routes`**: Lista de rotas dentro do grupo.
        *   **`name`**: Nome da rota.
        *   **`namespace`**: Namespace associado à rota.
        *   **`ruleName`**: Nome da regra de roteamento.
        *   **`hostnames`**: Lista de hostnames que a rota deve corresponder (pode incluir wildcards).
        *   **`matches`**: Critérios de correspondência para a rota, incluindo `headers`, `path` (exact, pathPrefix, regex), `method` e `query`.
        *   **`policies`**: Políticas aplicáveis a esta rota, cobrindo:
            *   Modificação de cabeçalhos (`requestHeaderModifier`, `responseHeaderModifier`).
            *   Redirecionamento de URL (`requestRedirect`).
            *   Reescrita de URL (`urlRewrite`).
            *   Espelhamento de requisições (`requestMirror`).
            *   Resposta direta (`directResponse`).
            *   CORS (`cors`).
            *   Autorização (`mcpAuthorization`, `authorization`, `mcpAuthentication`).
            *   Processamento de IA (`ai` com `promptGuard`, `bedrockGuardrails`, `googleModelArmor`, `azureContentSafety`).
            *   Políticas de rate limiting (`localRateLimit`, `remoteRateLimit`).
            *   TLS e tunelamento de backend (`backendTLS`, `backendTunnel`).
            *   Autenticação de backend (`backendAuth`).
            *   Timeouts (`timeout`).
            *   Retentativas (`retry`).
        *   **`backends`**: Lista de backends para onde a rota pode ser encaminhada, com configurações específicas para MCP e IA.

### Considerações de Hardware e Otimização

*   **Otimizações Bare-Metal:** A arquitetura SODA visa otimizações de baixo nível para maximizar o desempenho.
*   **iGPU Gargalos:** A arquitetura deve considerar as limitações de barramento da iGPU, que podem se tornar um gargalo em cargas de trabalho intensivas.
*   **Instruções AVX2:** O uso de instruções AVX2 na CPU é recomendado para otimizar o processamento de dados.
*   **`llama.cpp` mmap para RTX 2060m:** Para cenários específicos de inferência de LLM em hardware como a RTX 2060m, a utilização de `mmap` com `llama.cpp` pode ser uma estratégia de otimização eficaz.

### Auditoria Crítica e Análise de Furos

*   **Consistência de Políticas:** É crucial garantir que as políticas de segurança e processamento de IA sejam aplicadas de forma consistente em todos os backends e rotas. A granularidade das políticas permite um controle fino, mas a complexidade pode levar a configurações inconsistentes se não gerenciada adequadamente.
*   **Gerenciamento de Estado em Rate Limiting Remoto:** A dependência de um servidor remoto para rate limiting introduz um ponto de falha potencial. A estratégia `failOpen` pode levar a sobrecarga se o serviço de rate limiting estiver indisponível, enquanto `failClosed` pode negar acesso legítimo. A escolha deve ser cuidadosamente avaliada com base no caso de uso.
*   **Segurança de Credenciais de Backend:** A configuração de autenticação de backend, especialmente para chaves e segredos, requer um gerenciamento rigoroso para evitar exposição. O uso de arquivos ou variáveis de ambiente seguras é essencial.
*   **Complexidade de Configuração de IA:** A vasta gama de opções para backends de IA e suas políticas pode levar a uma complexidade de configuração significativa. Uma documentação clara e ferramentas de validação são cruciais para evitar erros.
*   **Latência de `extAuthz` e `extProc`:** A introdução de chamadas externas para autorização e processamento pode introduzir latência adicional. A arquitetura deve considerar estratégias para mitigar isso, como caching ou processamento assíncrono, onde aplicável.
*   **Gerenciamento de Dependências de Rust:** A dependência de bibliotecas Rust e suas versões deve ser gerenciada cuidadosamente para garantir a estabilidade e a segurança do sistema.

Este manual serve como um guia para a construção e manutenção da arquitetura SODA, enfatizando a pureza arquitetural, a eficiência e a segurança.

---

## Manual Canônico da Arquitetura SODA (Sovereign Operating Data Architecture) - Genesis MC

Este manual descreve os axiomas, regras e conceitos técnicos que sustentam a arquitetura SODA, com foco em sua implementação utilizando Rust (Tokio) no backend e Svelte 5 + Tauri v2 no frontend.

### Axiomas Fundamentais

1.  **Backend em Rust (Tokio):** Toda a lógica de negócios, processamento de dados e orquestração reside no backend, implementado exclusivamente em Rust com o framework assíncrono Tokio.
2.  **Frontend Passivo (Svelte 5 + Tauri v2):** O frontend serve como uma interface de usuário passiva, responsável apenas pela apresentação de dados e pela captura de interações do usuário. Toda a lógica de processamento e tomada de decisão é delegada ao backend Rust.
3.  **Comunicação Zero-Copy (IPC):** A comunicação entre o frontend e o backend é realizada através de Inter-Process Communication (IPC), otimizada para zero-copy, garantindo a máxima eficiência na transferência de dados.
4.  **Hardware Awareness:** A arquitetura é projetada com consciência do hardware subjacente, permitindo otimizações de baixo nível. Isso inclui considerações sobre gargalos de barramento de iGPU e a utilização de diretivas de execução AVX2 para CPUs ou `mmap` para inferência via `llama.cpp` em GPUs como a RTX 2060m.

### Regras de Arquitetura

1.  **Rust-Centric Backend:** O backend deve ser escrito inteiramente em Rust, utilizando Tokio para concorrência e gerenciamento de tarefas assíncronas.
2.  **Svelte/Tauri Frontend:** O frontend deve ser desenvolvido utilizando Svelte 5 para a framework e Tauri v2 para a construção da aplicação desktop, garantindo uma interface de usuário leve e eficiente.
3.  **Lógica no Backend:** Qualquer lógica de processamento de dados, validação, orquestração de LLM, ou interação com ferramentas externas deve ser implementada no backend Rust. O frontend não deve conter lógica de negócios complexa.
4.  **Proibição de Tecnologias Não Conformes:**
    *   **Eliminação de React:** Tutoriais ou menções a frameworks frontend como React devem ser sumariamente ignorados.
    *   **Eliminação de Node.js Daemons:** O uso de Node.js como daemon ou para lógica de backend é estritamente proibido.
    *   **Eliminação de Electron:** O framework Electron para desenvolvimento de aplicações desktop é proibido.
    *   **Eliminação de VDOM:** Abordagens que dependem de Virtual DOM (VDOM) no frontend são proibidas.
    *   **Eliminação de SSR (Next.js):** Arquiteturas de Server-Side Rendering (SSR), como as implementadas por Next.js, são incompatíveis com a filosofia de frontend passivo do SODA.
5.  **Otimização de Desempenho:**
    *   **Otimizações Bare-Metal:** Sempre que possível, explorar e implementar otimizações de baixo nível ("bare-metal") no backend Rust para maximizar o desempenho.
    *   **Gerenciamento de Recursos de Hardware:** Considerar e mitigar gargalos de hardware, como os associados a iGPUs e barramentos de memória.
    *   **Instruções de CPU/GPU:** Utilizar instruções otimizadas como AVX2 para processamento na CPU e `mmap` para acesso eficiente à memória da GPU (especificamente para `llama.cpp` em cenários como a RTX 2060m).

### Conceitos Técnicos e Diretrizes

*   **LLM Gateway:**
    *   **Unificação de APIs:** O SODA atuará como um gateway unificado para interagir com diversos provedores de LLM (OpenAI, Anthropic, Gemini, Bedrock, etc.), apresentando uma API compatível com OpenAI para simplificar a integração.
    *   **Controle de Orçamento e Gasto:** Implementar mecanismos de controle de custos e orçamento por agente ou por rota, utilizando métricas de uso de tokens e rate limiting.
    *   **Enriquecimento de Prompts:** Capacidade de enriquecer prompts antes de enviá-los aos LLMs, possivelmente através de políticas configuráveis no backend.
    *   **Balanceamento de Carga e Failover:** Implementar estratégias de balanceamento de carga e failover para múltiplos provedores de LLM ou instâncias de modelos.
    *   **Configuração de Modelos:** Definir modelos LLM específicos, com a capacidade de mapear nomes de modelos de requisição para nomes de modelos de provedores, e configurar parâmetros específicos (`apiKey`, `awsRegion`, `vertexRegion`, `azure*`, `hostOverride`, `pathOverride`, `tokenize`, etc.).
    *   **Gerenciamento de Sessão (MCP):** O SODA deve gerenciar nativamente sessões MCP, com a flexibilidade de configurar sessões como stateless para backends que não necessitam de persistência de estado. A estratégia de hashing consistente para garantir afinidade de sessão em instâncias distribuídas do SODA é uma consideração importante.
    *   **Multiplexação MCP:** Suporte à multiplexação de requisições MCP para múltiplos backends, incluindo a reescrita de nomes de ferramentas para incluir o nome do backend.
    *   **OpenAPI para MCP:** Capacidade de converter APIs RESTful em ferramentas MCP utilizando especificações OpenAPI, atuando como um forwarder MCP-to-RESTful-API.
    *   **Autenticação e Autorização:** Suporte robusto para autenticação (JWT, API Keys, OAuth, OIDC, Basic Auth) e autorização (RBAC com CEL, `extAuthz`, `networkAuthorization`) em múltiplos níveis (frontend, LLM Gateway, MCP).
    *   **Guardrails:** Implementação de guardrails multi-camada para filtragem de conteúdo e proteção contra prompt injection, incluindo:
        *   Regex-based guardrails (`promptGuard.request.regex`).
        *   Webhooks para guardrails customizados.
        *   Integração com serviços de moderação de terceiros (OpenAI Moderation, AWS Bedrock Guardrails, Google Model Armor).
        *   Detecção de jailbreak (`azureContentSafety.detectJailbreak`).
    *   **Observabilidade:** Geração de métricas, logs e traces via OpenTelemetry para todas as interações de agentes, LLMs e ferramentas. As métricas devem incluir detalhes sobre uso de tokens, custos, tempos de resposta e identidade do agente.
    *   **Rate Limiting:** Implementação de rate limiting local e remoto, com capacidade de definir limites baseados em tokens e requisições, e a possibilidade de usar expressões CEL para definir limites dinâmicos.
    *   **Transformações e Políticas:** Capacidade de modificar requisições e respostas através de políticas configuráveis, incluindo `requestHeaderModifier`, `responseHeaderModifier`, e `transformations` com expressões CEL.
    *   **TLS e Tunneling:** Configuração granular de TLS para conexões com backends (`backendTLS`) e suporte a tunelamento (`backendTunnel`) para conexões seguras.
    *   **Gerenciamento de Memória:** Embora não seja uma função direta do gateway, a arquitetura de agentes deve considerar um sistema de memória persistente (ex: arquivos markdown com busca vetorial) para manter o contexto entre as interações. O gateway pode facilitar o acesso a esses dados através de ferramentas MCP.

*   **Multi-Agent Orchestration:**
    *   **Coordinator e Specialists:** A arquitetura de agentes deve seguir um padrão de coordinator-specialist, onde um agente central gerencia a interação com o usuário e roteia tarefas para agentes especializados.
    *   **Stateless Specialists:** Os agentes especialistas devem ser projetados para serem stateless e descartáveis, sendo instanciados por tarefa e recebendo contexto do coordinator.
    *   **Isolamento de Ferramentas:** As permissões de acesso a ferramentas devem ser rigorosamente controladas através de políticas de RBAC no gateway (`mcpAuthorization`), garantindo que cada agente só possa acessar as ferramentas para as quais foi explicitamente autorizado.
    *   **Separação de Responsabilidades:** O coordinator é responsável pela lógica de decisão e roteamento, enquanto o gateway é responsável pela aplicação de políticas de segurança, custo e governança.

*   **Segurança e Governança:**
    *   **Kill Switch:** O gateway deve fornecer um mecanismo centralizado para desativar agentes ou todo o sistema em caso de comportamento anômalo (ex: escalando o deployment do gateway para zero réplicas em Kubernetes).
    *   **RBAC Granular:** Utilizar JWTs com escopos definidos e políticas de autorização baseadas em CEL para impor controle de acesso granular a LLMs e ferramentas.
    *   **Proteção contra Jailbreaking:** Implementar guardrails e políticas de autorização que impeçam agentes de contornar restrições de segurança ou invocar ferramentas não autorizadas.
    *   **Auditoria:** Todas as interações devem ser auditadas e rastreáveis através de telemetria, permitindo a identificação do agente responsável por cada ação.

*   **Considerações de Implementação:**
    *   **Rust e Tokio:** A escolha de Rust e Tokio para o backend garante segurança de memória, concorrência eficiente e alto desempenho, essenciais para lidar com protocolos stateful e conexões de longa duração.
    *   **Svelte 5 e Tauri v2:** A combinação de Svelte 5 e Tauri v2 para o frontend oferece uma experiência de usuário responsiva e eficiente, sem a sobrecarga de frameworks VDOM ou Node.js.
    *   **xDS e YAML:** A configuração do gateway pode ser gerenciada via xDS (para cenários mais dinâmicos) ou YAML (para configurações mais estáticas), oferecendo flexibilidade na implantação.
    *   **Integração com Kubernetes:** A implantação em Kubernetes é recomendada para aproveitar recursos como health checks, escalonamento e rolling updates, garantindo a resiliência do próprio plano de controle. A gestão do agente que gerencia o cluster (ex: Cloud Agent) requer cuidado especial com a segmentação de namespaces para evitar dependências circulares.

### Análise Crítica e Pontos de Atenção

*   **Complexidade da Configuração:** A vasta gama de políticas e opções de configuração pode levar a uma complexidade significativa na definição e manutenção das regras de segurança e roteamento. Uma documentação clara e exemplos práticos são cruciais.
*   **Gerenciamento de Estado do MCP:** Embora o SODA tenha como objetivo simplificar o gerenciamento de estado do MCP, a abordagem "stateful por padrão" pode ser um gargalo em cenários distribuídos sem um backend de armazenamento de sessão remoto. A flexibilidade para configurar sessões stateless é um avanço importante.
*   **Incompletude da Documentação:** Conforme observado em análises externas, a documentação pode apresentar lacunas ou desatualizações em relação à implementação real. É essencial validar a documentação com o código-fonte e os testes.
*   **Desempenho em Cenários de Alta Concorrência:** Embora Rust e Tokio ofereçam excelente desempenho, a arquitetura deve ser cuidadosamente avaliada sob cargas de trabalho de alta concorrência para identificar potenciais gargalos, especialmente em operações de IPC e gerenciamento de estado.
*   **Integração com LLM Providers:** A compatibilidade com diferentes provedores de LLM e suas APIs específicas (ex: `/v1/chat/completions` vs `/v1/messages`) requer um esforço contínuo de adaptação e pode apresentar limitações em funcionalidades não totalmente suportadas (ex: output estruturado).
*   **Otimização de iGPU:** A menção a otimizações de iGPU e `mmap` para `llama.cpp` sugere um foco em cenários de inferência local ou em hardware com recursos limitados. A arquitetura deve garantir que essas otimizações sejam aplicáveis e eficazes dentro do ecossistema SODA. A interdependência entre a CPU e a iGPU, e os gargalos de barramento, devem ser considerados na alocação de recursos e no design das tarefas de inferência.

Este manual serve como um guia para a construção e operação de sistemas baseados na arquitetura SODA, enfatizando a aderência aos princípios de engenharia de software robusta e eficiente.