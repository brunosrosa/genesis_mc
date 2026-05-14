# SODA ETL V3: Current State Dossier & Architectural Blueprint
**Versão:** 3.5 (SODA Canon v2.0) | **Status:** Red Teaming Audit - REFACTOR REQUIRED

Este documento serve como a **única fonte da verdade** para a reconstrução do pipeline ETL do Genesis MC. O sistema atual é diagnosticado como uma infraestrutura de transição ("ETL de Brinquedo") e deve ser reescrito seguindo os axiomas de integridade agêntica abaixo.

---

## 1. Gargalos Arquiteturais e Requisitos SODA Canon (O Verdadeiro V3)

Esta seção detalha as mutações inegociáveis que devem ser implementadas na próxima iteração de código.

### A. Integração de Aterramento Semântico (Fase 2 - NotebookLM)
*   **Problema Atual:** A Fase 2 opera com prompts estáticos, limitada ao contexto da Fase 1, perdendo a "Mente Colmeia" do projeto.
*   **Requisito:** O pipeline **DEVE** invocar o sidecar `notebooklm-mcp-cli` através do método `cross_notebook_query`.
*   **Objetivo:** Cruzar o contexto do repositório extraído com as diretrizes arquiteturais residentes no NotebookLM do Genesis MC antes da síntese final.
*   **Fail-Fast:** Qualquer erro de autenticação ou cookie no NotebookLM deve resultar no **SIGKILL imediato** do lote para evitar análises cegas.

### B. Orquestração de Extração Tripla (Fase 1 - Ingestão Densa)
*   **Problema Atual:** Inanição de dados por dependência exclusiva de `httpx` e `README.md`.
*   **Requisito:** A Fase 1 deve coordenar três motores de extração local-first:
    1.  **JCodeMunch (AST):** Extração de assinaturas e estrutura via `get_file_outline`.
    2.  **Webcrawl (Scraping):** Captura de CodeWiki e Documentação Externa.
    3.  **HTTPX (Raw):** Metadados da API do GitHub e README bruto.

### C. Gestão de Sidecars Efêmeros e Higiene de RAM
*   **Requisito:** Todas as ferramentas MCP (escritas em Python ou Node) devem ser instanciadas como subprocessos isolados.
*   **Protocolo de Morte:** Ao término de cada fase, o orquestrador deve disparar um `SIGKILL` atômico nos subprocessos, erradicando processos zumbis que sequestram a VRAM da RTX 2060m.

### D. Diretrizes FinOps e Truncagem Rigorosa
*   **Budget de Contexto:** Limite rígido de **15.000 caracteres** (~4.5k tokens) para o payload da Fase 1.
*   **Distribuição:**
    *   **README:** Máximo 8.000 caracteres.
    *   **AST Outline:** Máximo 6.500 caracteres.
    *   **Metadados API:** Protegido (500 chars).

---

## 2. Arquitetura das 4 Fases (Revisada)

O sistema opera sob o paradigma **Secure-by-Construction**, onde a observabilidade é total.

1.  **Fase 1 (Extração Tripla):** Validação de existência via API GitHub (Anti-404) -> Ingestão densa via AST + Scrape.
2.  **Fase 2 (Enxame de Lentes + NotebookLM):** Map-Phase onde as Lentes especialistas consultam o Oráculo (NotebookLM) para aterramento de diretrizes SODA.
3.  **Fase 3 (Sintetizador Pydantic):** Geração do objeto `RepoHeuristic` validado contra o contrato de 45 colunas.
4.  **Fase 4 (Persistence & UPSERT):** Injeção atômica no SQLite (Vault) e sincronização destrutiva via `batchUpdate` no Google Sheets.

---

## 3. Protocolo de Execução: A "Janela de Vidro"

A observabilidade no Ambiente de Desenvolvimento é inegociável para o Arquiteto:

*   **Terminal Dedicado:** Execução obrigatória em terminal visível e destacado (ex: Split Pane na IDE).
*   **Streaming de Raciocínio:** Logs coloridos via `Rich` devem exibir cada chamada de subprocesso e o status do heartbeat do pipeline.
*   **Auditoria ao Vivo:** O streaming permite a intervenção humana imediata em caso de desvio semântico.

---

## 4. Schema SODA V3 (Mapeamento de 45 Colunas)

O contrato `RepoHeuristic` (Pydantic) permanece a âncora de dados, mas as Lentes devem ser enriquecidas pelo NotebookLM.

1.  **Identidade:** `project_name`, `repo_url`, `lote_id`, `data_ultima_analise`, `analise_origem`.
2.  **Scores (0-10):** Fit Geral, Philosophical Fit, Bare-Metal Fit, Extractability, Operability.
3.  **Análise Humana (PT-BR):** Executive Verdict, Justificativa de Decisão, Proposta Original.
4.  **Riscos:** Entropy, Design Misuse, Ethics, Linha Vermelha.
5.  **Deep Architecture:** Categoria, Nuance Técnica, Tipo de Integração, Papel Exato, Must Components, Ouro a Extrair, Deep Pattern, Ação de Canibalização.

---

## 5. Dependências e Segurança

*   **Segredos:** `GITHUB_PAT`, `SHEETS_ID`, `OPENROUTER_API_KEY`, `NOTEBOOKLM_COOKIE`.
*   **Anti-SDC:** `BEGIN IMMEDIATE` no SQLite. Proibido o uso de `std::fs` ingênuo; mutações via `atomic-write-file`.
*   **Observabilidade:** Tabela `etl_run_log` e `etl_errors` para telemetria forense.

---
**Exit Code 0. Dossiê SODA V3 Blindado. A máquina reconhece as falhas atuais e possui a planta-baixa completa para a refatoração do pipeline.**

-----

# SODA ETL V3: Current State Dossier
**Versão:** 3.1 | **Data:** 2026-05-02 | **Status:** Tactical Freeze

Este documento consolida a arquitetura técnica e o estado de prontidão do pipeline **SODA ETL V3** para transição de contexto.

## 1. Arquitetura do Pipeline (As 3 Fases)

O sistema opera sob o paradigma **Local-First Context Ingestion**, garantindo que as IAs não operem "cegas".

*   **Fase 1: Kimi K2 (Anchored Triagem)**
    *   **Mecanismo:** Extração física do `README.md` via `httpx` (GitHub API com `GITHUB_PAT`) antes da inferência.
    *   **Modelo:** `moonshotai/kimi-k2.5` (via OpenRouter).
    *   **Objetivo:** Identificar linguagem, domínio e complexidade para ancorar o Enxame.
*   **Fase 2: Enxame Cognitivo (Map-Phase)**
    *   **Mecanismo:** Despacho paralelo de 3 lentes especialistas via `asyncio.gather`.
    *   **Política:** **Fail-Fast Ativo** (uma falha derruba o processo para evitar dados corrompidos/N/A).
    *   **Lentes:**
        *   **Lente A (UX/Produto):** `anthropic/claude-opus-4.7` — Foco em atrito humano e utilidade.
        *   **Lente B (Arq/Bare-Metal):** `deepseek/deepseek-v4-pro` — Foco em Rust, 6GB VRAM e Canibalização O(1).
        *   **Lente C (Operacional):** `z-ai/glm-5.1` — Foco em sustentação 24/7 e entropia de manutenção.
*   **Fase 3: Síntese e Validação (Reduce-Phase)**
    *   **Mecanismo:** Sintetizador executivo e formatador JSON estrito.
    *   **Modelo:** `deepseek/deepseek-chat` (Âncora de baixo custo).
    *   **Output:** Objeto `RepoHeuristic` validado via Pydantic.
*   **Fase 4: Carga e UPSERT (Persistence-Phase)**
    *   **Mecanismo:** UPSERT destrutivo via `batchUpdate` no Google Sheets (Script `phase4_sheets_loader.py`).
    *   **Lógica:** Localiza a `repo_url` na Coluna C e sobrescreve a linha exata (A-AS) com 45 colunas.
    *   **Garantia:** Zero duplicidade. Preservação de índices originais da planilha.

## 2. Dependências e Ambiente (.env)

O orquestrador exige as seguintes variáveis configuradas no arquivo `.env`:

*   **GITHUB_PAT:** Token de acesso pessoal do GitHub (necessário para evitar 403/429 na Fase 1).
*   **SHEETS_ID:** ID da planilha do Google Sheets (MASTER_SOLUTIONS_v3).
*   **OPENROUTER_API_FAST/HEAVY:** Chaves de API para os modelos Kimi (Fast) e Enxame (Heavy).
*   **OPENAI_BASE_URL:** `https://openrouter.ai/api/v1`.

## 2. Orquestração e Fluxo de Dados

O orquestrador (`etl_orchestrator.py`) gerencia o ciclo de vida do lote.

*   **Fluxo:** Sequencial (One-by-One) dentro de micro-lotes de 5 repositórios.
*   **Telemetria:** Logging rico no STDOUT e registro de erros na tabela `etl_errors` (SQLite).
*   **Resfriamento:** `await asyncio.sleep(2)` entre cada repositório para preservação de cota e rate-limit TCP.
*   **Persistência Atômica:**
    *   **L2 (SQLite):** Gravação imediata após a Fase 3 no arquivo `soda_heuristic_vault.db`.
    *   **WAL Mode:** Habilitado para concorrência de leitura e escrita.
    *   **Integridade:** Uso de `BEGIN IMMEDIATE` para garantir que o Sheets só seja atualizado se o SQLite confirmar a transação.

## 3. Segurança e Controle Anti-SDC (Silent Data Corruption)

O pipeline implementa proteções rigorosas contra corrupção de dados:

*   **Fail-Fast (Fase 2):** O `asyncio.gather` sem `return_exceptions` garante que falhas de rede/API não gerem linhas vazias ou "N/A" que poluiriam o banco.
*   **Truncagem de VRAM:** Limitação estrita de caracteres (800 para análises, 400 para vereditos) antes da inserção no SQLite, protegendo a memória em consultas futuras.
*   **Validation Strict:** Uso do Pydantic V2 para forçar tipos (scores float, datas ISO) e impedir a injeção de lixo semântico.
*   **Marca d'Água:** Injeção da string `SODA ETL V3 Auto` na coluna 43 para evitar reprocessamento infinito e facilitar auditorias de lote.
    1.  **L2 (SQLite):** `BEGIN IMMEDIATE` + `COMMIT` para evitar Silent Data Corruption (SDC).
    2.  **Lote (Sheets):** UPSERT destrutivo via `batchUpdate` acionado somente após sucesso no SQLite.
*   **Flags CLI:**
    *   `--lote-id`: Filtro obrigatório (ex: `LOTE_19`).
    *   `--batch-size`: Tamanho do micro-lote (Default: 5).
    *   `--dry-run`: Simulação total sem persistência.
    *   `--vault-db`: Caminho do banco SQLite.

## 3. Schema SODA V3 (Mapeamento de 45 Colunas)

O contrato `RepoHeuristic` (Pydantic) impõe a seguinte estrutura:

1.  **Identidade:** `project_name`, `repo_url`, `lote_id`, `data_ultima_analise` (ISO-8601), `analise_origem`.
2.  **Scores (0-10):** `score_final`, `score_fit_geral_soda`, `score_philosophical_fit`, `score_bare_metal_fit`, `score_architectural_extractability`, `score_operability`, `score_creep_risk`.
3.  **Análise Humana (PT-BR):** `declared_description`, `justificativa_decisao`, `executive_verdict`, `proposta_original_resumo`.
4.  **Lentes:** `lente_a_sentido_ux`, `lente_b_estrutura_arq`, `lente_c_realidade_ops`.
5.  **Riscos e Classificação:** `entropy_risk`, `design_misuse_risk`, `intrinsic_ethics_risk`, `risco_principal`, `risco_linha_vermelha`, `classificacao_terminal`.
6.  **Arquitetura Deep:** `categoria_arquitetural`, `categoria_nuance_tecnica`, `tipo_integracao`, `integracao_papel_exato`, `must_components`, `ouro_a_extrair`, `deep_pattern`, `acao_de_canibalizacao`, `transplantable_core`, `logic_math_heuristic`, `bare_metal_fit`, `discipline_dependency`, `extractability_level`, `operability_level`, `where_ai_should_not_enter`, `do_not_absorb`.

## 4. Ponto de Parada e Backlog de Erros

### O que funciona perfeitamente:
*   [x] Persistência Atômica no SQLite (Vault).
*   [x] UPSERT em lote no Google Sheets (Evita flicagem e preserva histórico).
*   [x] Enxame Cognitivo (Fase 2) com modelos de ponta e Fail-Fast.
*   [x] Tradução mandatória para PT-BR em todos os campos descritivos.

### O que está QUEBRADO (Prioridade para próxima sessão):
*   [ ] **Inanição de Dados na Fase 1:** Atualmente depende apenas de `httpx` para o `README.md`. Repositórios sem README ou com README opaco causam falhas.
*   [ ] **Extração Tripla (Upgrade):** Implementar orquestração de extração usando `jcodemunch` (AST), `webcrawl` (Deep Search) e `httpx` (Raw README) para garantir contexto denso.
*   [ ] **Resiliência 404/403:** O Short-Circuit atual é muito agressivo. Precisa de lógica de retry ou fallback para domínios inacessíveis que ainda possam ser analisados via metadata.
*   [ ] **Regressão à Média:** Otimizar o prompt da Fase 3 para evitar scores medianos (8.5) quando não houver clareza técnica.
*   [ ] **Alinhamento de Scripts:** O script `audit_vault.py` possui nomes de colunas obsoletos (V2) e precisa ser sincronizado com o Schema V3 (ex: `score_final`).

## 5. Observabilidade e Debug

Ferramentas disponíveis para monitoramento em tempo real:

*   **Terminal STDOUT:** Logs coloridos via `RichHandler` (ou fallback padrão) exibindo o progresso de cada fase e requisições HTTP.
*   **Tabela etl_run_log:** Registra o status de cada execução (`RUNNING`, `COMPLETED`, `PARTIAL`, `FAILED`), timestamps e contagem de sucessos/erros.
*   **Tabela etl_errors:** Log detalhado de exceções (fase, tipo de erro, mensagem) permitindo triagem pós-morte sem abortar o lote.
*   **Audit Vault:** Script `etl/audit_vault.py` para inspeção rápida dos scores e vereditos diretamente no terminal (requer atualização de schema).

## 6. Diretrizes para a Reconstrução da Fase 1 (Extração Tripla)

O Ambiente de Desenvolvimento prioriza a **densidade de contexto** sobre a restrição de VRAM. O foco de otimização é o consumo de tokens (FinOps).

### A. Validação de Existência (Protocolo Anti-404)
*   **Ação:** O script deve realizar um `GET` no endpoint `api.github.com/repos/{owner}/{repo}` antes de qualquer fase de IA.
*   **Lógica:**
    *   **Status 200:** Extrair metadados vitais (`stargazers_count`, `forks_count`, `language`, `description`, `topics`).
    *   **Status 404/403:** Registrar o erro no `etl_errors`, marcar o repositório como `DEAD_LINK` e disparar o `Short-Circuit` imediato. **Proibido gastar tokens em repositórios inexistentes.**

### B. Orquestração de Contexto O(1) (JCodeMunch & Web)
*   **Extração AST:** Utilizar o método `get_file_outline` via `jcodemunch-mcp` para capturar assinaturas de funções e nomes de classes em arquivos estruturantes (ex: `main.rs`, `Cargo.toml`, `package.json`).
*   **Scraping Resiliente:** Utilizar `webcrawl-mcp` (ou `read_url_content`) para capturar o README e a CodeWiki (se disponível).

### C. Gestão de Budget Cognitivo (FinOps)
*   **Teto de Ingestão:** O payload enviado ao Kimi K2 na Fase 1 deve ser limitado a **15.000 caracteres** (~4k tokens).
*   **Prioridade de Truncagem:**
    1.  Metadados de API (Protegido - 100%)
    2.  README.md (Truncagem em 8.000 chars)
    3.  AST Outline / Tree (Truncagem em 6.500 chars)

### D. Execução de Sidecars Efêmeros
*   Os MCPs devem ser acionados como subprocessos efêmeros que morrem após a entrega do dado, mantendo o orquestrador Python como o único processo persistente.

## 7. Protocolo de Execução: A "Janela de Vidro"

Toda execução de lote a partir desta data deve seguir o rigor de observabilidade do Arquiteto:

1.  **Terminal Dedicado:** O orquestrador deve ser rodado em um terminal destacado e visível na IDE (ex: `Split Pane` à direita).
2.  **Streaming de Logs:** O nível de log deve garantir que cada chamada a MCP e cada transição de Fase seja exibida no STDOUT com timestamps claros.
3.  **Auditoria ao Vivo:** O Arquiteto reserva o direito de interromper o processo (`Ctrl+C`) se detectar alucinação ou inanição de contexto durante o streaming.

## 8. Scripts Utilitários (.agents/scratch) — Senior Architect Insight

A pasta de rascunhos abriga o maquinário de suporte e automação local, projetado para resiliência e baixa fricção no hardware alvo (RTX 2060m).

*   **run_orchestrator.py (The Entrypoint):**
    *   **Arquitetura:** Wrapper compatível com `uv run` (PEP 723) que resolve dependências efêmeras em tempo de execução.
    *   **Injeção:** Manipula o `sys.path` para permitir importações do núcleo `etl/` e reconfigura o encoding do `stdout` para `utf-8` (bypass de crash em terminais Windows legados).
*   **soda_synth.py (Knowledge Crystallizer):**
    *   **Pipeline Tri-modal:** Suporta inferência via API Gemini, CLI Wrapper e **Local Bare-Metal** (via llama.cpp).
    *   **Hardware Ops:** No modo `local`, força o offload total para a dGPU (`-ngl 99`) com uso mandatório de `--mmap` e `GGML_CUDA_ENABLE_UNIFIED_MEMORY=1` para prevenir falhas de barramento PCIe na RTX 2060m.
    *   **Chunking Logic:** Aplica uma janela deslizante (`CHUNK_SIZE`) adaptativa para cada backend (800K chars para API vs 24K chars para inferência local Phi-4).
*   **kill_zombies.py (Process Guard):**
    *   **Mecanismo:** Atua como um "Janitor" de processos, identificando e terminando instâncias órfãs de `python.exe`, `gemini.cmd` ou `llama-cli.exe` que travam o KV Cache na VRAM após falhas de execução.
*   **inspect_db.py (SQL Auditor):**
    *   **Mecanismo:** Utiliza `sqlite3.Row` para auditoria rápida em formato de dicionário, permitindo verificações de sanidade no `soda_heuristic_vault.db` sem overhead de ORM.
*   **update_sheets_v3.py (Manual Sync):**
    *   **Mecanismo:** Expõe a lógica da Fase 4 de forma isolada, permitindo o re-disparo de UPSERTs atômicos para correções cirúrgicas de linhas corrompidas ou alinhamento de headers.

---
**Exit Code 0. Contexto cristalizado.**