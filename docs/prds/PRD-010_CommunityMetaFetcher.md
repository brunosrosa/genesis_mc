# PRD-010: CommunityMetaFetcher (N10)

## 1. Visão Geral
**Status:** Fase B (Especificação)
**Nó do DAG:** N10
**Objetivo:** Coletar métricas sociais e de comunidade de repositórios externos (ex: contagem de Issues, Pull Requests abertos/fechados, datas de última atualização) para ancorar os dados do SODA SSOT. Este nó atua como um coletor passivo, executando I/O de rede e traduzindo falhas de API em graceful degradation (Fail-Soft).

## 2. Contrato de I/O (Interface)

### 2.1. Entradas (Input)
A execução requer a passagem estrita de referências imutáveis para garantir isolamento e prevenir esgotamento de APIs.
- `repo_url: &Url`: URL validada do repositório alvo (ex: GitHub, GitLab).
- `rate_limiter: &RateLimiter`: Dependência injetada globalmente no Harvester para orquestrar backpressure e prevenir banimento de IP ou estouro de cota nas APIs externas.

### 2.2. Saídas (Output)
A função de extração deverá retornar uma assinatura formalizada:
`Result<CommunityMetaPayload, FetchError>`

#### 2.2.1. `CommunityMetaPayload` (Sucesso)
Struct padronizada, imutável e agnóstica à plataforma de origem:
- `open_issues_count: u32`
- `open_prs_count: u32`
- `last_commit_date: Option<DateTime<Utc>>`
- `last_release_date: Option<DateTime<Utc>>`

#### 2.2.2. `FetchError` (Falha)
Enum que mapeia restritamente as anomalias de rede e resposta.

## 3. Cenário Principal de Falha (Fail-Closed to Fail-Soft)

**O Problema:** A rede é inerentemente instável e imprevisível. O GitHub pode retornar 404 (repositório privado/apagado), 403 (Rate Limit estourado) ou a conexão pode sofrer Timeout.
**O Paradigma:** Fail-Soft Silencioso.
**A Solução:** Qualquer variação de erro (`FetchError` gerado por Timeout, erro de rede, HTTP 4xx/5xx) **NÃO PODE** propagar e derrubar a thread do Harvester ou o Event Loop do Tokio. O extrator interno pode até produzir o `FetchError`, mas a interface pública do `CommunityMetaFetcher` deve interceptar a falha, registrar o evento via Ghost Telemetry/log, e retornar graciosamente métricas vazias (ou falhar graciosamente e ser tratado pelo orquestrador como dados nulos) — garantindo que o pipeline de ETL avance ininterrupto para o próximo nó.

## 4. Invariantes de Arquitetura e Proibições Tóxicas

A implementação do nó N10 deve respeitar de forma absoluta as seguintes regras:

### 4.1. PT-3 (Zero Bloqueio)
É EXPRESSAMENTE PROIBIDO executar qualquer I/O síncrono. Toda interação de rede, parsing de headers ou leitura de stream de resposta DEVE ser estritamente assíncrona, usando `.await` para não bloquear o Worker Pool do Tokio.

### 4.2. PT-META-1 (Zero SDK Bloat)
É TERMINANTEMENTE PROIBIDO importar SDKs monolíticos e pesados (ex: `octocrab`, `hubcaps` ou similares) apenas para buscar contadores. Essa abstração asfixia o tamanho do binário e aumenta a superfície de ataque/dependências sem justificativa de performance.
**Solução Exigida:** A extração deve ser operada EXCLUSIVAMENTE via requisições HTTP REST cruas e diretas usando a crate `reqwest` (com suporte a async/json), OU interagindo com a CLI oficial `gh api` via `tokio::process` devidamente isolada.

## 5. Critérios de Conclusão (Definition of Done - DoD)
- [ ] A interface pública do nó recebe `repo_url` e `rate_limiter` injetados por referência.
- [ ] O tipo de retorno interno utiliza `Result<CommunityMetaPayload, FetchError>`, mas as falhas de API (Timeout, 404, Rate Limit) são traduzidas num payload vazio/Fail-Soft na borda do nó, nunca causando crash (`unwrap()` ou `panic!`).
- [ ] A implementação **não possui** dependências de SDKs do GitHub no `Cargo.toml`.
- [ ] Uso exclusivo de `reqwest` HTTP direto ou `tokio::process` (gh api) para a coleta de dados.
- [ ] Testes unitários em Rust (Fase C) provam, via mocks, que chamadas com Timeout e 404 retornam `Ok` com métricas zeradas (ou um resultado seguro equivalente que evite a quebra do pipeline).
