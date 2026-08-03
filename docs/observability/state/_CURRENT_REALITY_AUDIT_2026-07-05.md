# Auditoria Cega do Estado Atual (Reality Check) — SOULS / Souls MC
**Data da Auditoria:** 2026-07-05
**Auditor:** Antigravity (SOULS Bare-Metal Engine)

---

## 1. O Estado Real do ETL Cognitivo e Dados

### Conjunto de Scripts e Arquitetura Física
O pipeline cognitivo do SOULS é implementado através de 4 binários principais (fases) programados inteiramente em Rust (dentro de `src-tauri/src/bin/`):
1. **`f0_harvester_cli.rs` (Fase 0 - Harvester):**
   - **O que faz:** Realiza a varredura e colheita do repositório físico. Clona/atualiza via Git, executa ferramentas de análise estática e linters (`oxlint`, `cppcheck`, `opengrep`, `govulncheck`, `biome`), salvando as saídas brutas compactadas como blobs na tabela `artefatos_brutos`.
2. **`f1_distiller_cli.rs` (Fase 1 - Distiller):**
   - **O que faz:** Lê os artefatos brutos da Fase 0 e destila a "essência" arquitetural, identificando contratos e dependências críticas. Salva as saídas nas tabelas `artefatos_destilados` e `pacotes_destilados`.
3. **`f2_swarm_cli.rs` (Fase 2 - Swarm / Enxame Cognitivo):**
   - **O que faz:** Executa debates de IA usando a API do OpenRouter. Passa as informações destiladas por 3 Lentes analíticas distintas em paralelo:
     - **Lens A (ProductUX):** Inovação, valor do produto, neuro-inclusão e Canvas UI.
     - **Lens B (Architecture):** Portabilidade, agnosticismo hardware (transmutabilidade para CubeCL/Burn) e núcleo matemático.
     - **Lens C (Operations):** Auditoria FinOps, rate limits, observabilidade e toxicidade de dependências.
     - Salva os resultados JSON na tabela `debates_enxame`.
4. **`f3_synthesizer_cli.rs` (Fase 3 - Synthesizer):**
   - **O que faz:** Coordena a síntese final. Orquestra a execução das 5 fases de prompts do LLM, calcula pontuações matemáticas de conformidade e ajusta prioridades (eixo SOULS Fit). Salva o resultado consolidado em `repo_heuristics` e despacha para a planilha do Google Sheets via `SsotInjector`.

**Comunicação entre as Fases:**
As fases comunicam-se de forma assíncrona **exclusivamente via banco de dados local SQLite** (`.souls_data/souls_heuristic_vault.db`). Não existem scripts intermediários em Python ou Node.js controlando o fluxo no produto de produção. Todo o ecossistema é Rust nativo.

**Qualidade Geral do Código:**
- O código em Rust apresenta boa maturidade estrutural: uso de tipos fortemente tipados, tratamento de erros explícito com `thiserror`, controle de concorrência com travas transacionais no SQLite (`rusqlite::Transaction`) e retentativas dinâmicas com jitter para erros de concorrência (`DatabaseBusy`/`DatabaseLocked`).
- **Gargalo Identificado:** O mecanismo de sincronização com o Google Sheets realiza leituras redundantes e ineficientes célula a célula (HTTP roundtrips múltiplos) antes e depois de gravar a linha consolidada.

---

### Colunas de Dados Mapeadas (SSOT)
O backend em Rust mapeia **exatamente 82 colunas** de metadados para cada repositório. O catálogo canônico é definido no array estático `MASTER_SOLUTIONS_CANONICAL_COLUMNS` em [synthesizer.rs](file:///z:/genesis_mc/src-tauri/src/cognition/synthesizer.rs#L3347-L3430) e espelhado na struct `MasterSolutionsRow`.

---

### Escrita e Leitura no Google Sheets
A escrita final de cada repositório é feita em lote em um único range atômico que cobre a linha inteira da planilha (`A{row}:{end_col}{row}`) usando a API `values:batchUpdate`.
No entanto, **a leitura hoje é ineficiente e dispersa**:
1. **Antes de Escrever (Busca de Overrides e Lote):**
   - O código lê individualmente a célula de `proposta_original_resumo` via `read_sheet_cell` (1 chamada HTTP).
   - Lê individualmente a célula de `categoria_arquitetural` via `read_sheet_cell` (1 chamada HTTP).
   - Lê individualmente a célula de `lote_id` (1 chamada HTTP).
2. **Após Escrever (Confirmação da Escrita):**
   - O método `confirm_cloud_write_projection` executa 4 chamadas `read_sheet_cell` individuais e consecutivas para validar os campos `"project_name"`, `"repo_url"`, `"score_final"`, e `"analise_origem"`.
- **Diagnóstico:** Há no mínimo 7 requisições de leitura HTTP individuais por ciclo de sincronização, o que atrasa a execução e causa rate-limiting.

---

### Status de Andamento Gravados no SQLite
A governança dos estados de processamento é mantida por enums lógicos convertidos em strings gravadas no banco de dados:

1. **`status_processamento`** (na tabela `repositorios`):
   - `"APROVADO_PARA_HARVESTER"`: Repositório liberado para início da colheita.
   - `"F0_OK"`: Harvester concluído com sucesso.
   - `"DEGRADADO_F0"`: Concluído com falha em linter ou analisador estático (fail-soft).
   - `"ERRO_F0"`: Falha catastrófica no harvester (timeout/erro de spawn).
   - `"FASE_2_RUNNING"`: Enxame Cognitivo ativo e processando.
   - `"F2_OK"`: Debate e análises das Lentes A/B/C concluídos e salvos.
   - `"ERRO_F2"`: Falha no debate de IA.
   - `"CONCLUIDO"`: Síntese e injeção no Google Sheets finalizadas.
   - `"ERRO_FASE_4"`: Falha de rede ou validação no Google Sheets.

2. **`status_atualizacao` e `status_fase`** (na tabela `repo_heuristics`):
   - `status_atualizacao`: `"CONCLUIDO_AGUARDANDO"`, `"PENDENTE_FASE_0"`, ou `"REJEITADO_..."` (caso caia no disjuntor da red line).
   - `status_fase`: `"FASE_0_HARVESTER_OK"`, `"FASE_0_DEGRADADA"`, `"FASE_2_RUNNING"`, `"FASE_3_SYNTHESIZER_OK"`, `"FASE_4_SHEETS_UPDATED"`, `"FASE_4_CLOUD_FAILED"`, `"ERRO_FASE_4"`.

---

## 2. O Estado Real do Souls MC (UX/UI)

### Componentes de Telemetria e Feedback na UI
- **NÃO IMPLEMENTADO NO CÓDIGO.**
- O frontend em Svelte 5 / Tauri encontra-se em hibernação total. O arquivo físico `src/App.svelte` é um scaffold básico contendo a mensagem:
  ```svelte
  <main class="min-h-dvh bg-[oklch(0.12_0_0)] text-[oklch(0.985_0_0)]">
    <div class="mx-auto max-w-[92ch] px-6 py-10">
      <h1 class="text-2xl font-semibold tracking-tight">SOULS</h1>
      <p class="mt-3 font-mono text-sm opacity-70">UI em hibernação intencional. Prioridade: ETL (Fase 1.5).</p>
    </div>
  </main>
  ```
- Componentes como "Ghost Telemetry", "Ghost Borders" ou qualquer indicador dinâmico de progresso passivo são inteiramente inexistentes no código.

### Componentes Bloqueantes
- **NÃO IMPLEMENTADO NO CÓDIGO.**
- Não existem spinners de carregamento, modais ou overlays na interface do usuário, visto que a janela principal do Tauri é ocultada imediatamente no boot (`setup` em `main.rs` faz `window.hide()`).

---

## 3. O Estado Real de Governança e Sandboxing

### Protocolo HITL (Human-in-the-Loop)
- **NÃO IMPLEMENTADO NO CÓDIGO.**
- Não existe suporte físico no código para "Agent Inbox", controle de concorrência ou criação de branchs isoladas para aprovação de diffs cognitivos pelo humano.
- **Apenas dois ganchos rudimentares existem:**
  1. Uma regra descritiva em texto de canibalização e doutrina inserida em `src-tauri/src/harvester/canon.rs`.
  2. Um atalho de fluxo em `src-tauri/src/bin/f3_synthesizer_cli.rs` (linhas 3071-3084): se o status da planilha for `"PENDENTE_FASE_0"`, o orquestrador executa apenas o Harvester (F0) e encerra o processo, impedindo o envio das informações para o LLM.

---

### Implementação Física de Sandboxing
- **Parcialmente Implementado via Job Objects e Validação Lógica de Caminho.**
- **Isolamento de Kernel (Landlock, AppContainer, LPAC):** **NÃO IMPLEMENTADO NO CÓDIGO.** Não existem chamadas a APIs nativas de sandbox do kernel (Linux/Windows).
- **O que existe fisicamente em `src-tauri/src/harvester/sandbox.rs`:**
  1. **Controle de Processos Órfãos (Windows Job Objects):** Associa cada processo de ferramenta externa colhida (sidecar) a um Job Object com `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Isso garante a destruição física (SIGKILL) de subprocessos órfãos quando a aplicação principal Tauri morre.
  2. **Cercas Lógicas de Diretório (Path Validation):** O sandbox atua como um inspetor de I/O de arquivos. Ele intercepta os argumentos do comando e variáveis de ambiente e valida se apontam para diretórios fora dos escopos de trabalho permitidos (ex: verifica se o arquivo alvo está contido dentro do diretório do repositório ou de caches temporários autorizados como `.souls_sandbox` ou `.souls_semgrep`). Se violar, spawna um erro lógico (`SandboxError::PolicyViolation` / `SandboxError::PrivilegeError`).
