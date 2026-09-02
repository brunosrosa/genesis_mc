---
version: 7.0
description: SOULS Workspace Map & Territorial Governance (Canon v7.0 - Marco V6 Bare-Metal Cargo Workspace)
---

# SOULS WORKSPACE MAP & TERRITORIAL GOVERNANCE

> [!IMPORTANT]
> **REGRA MATRIZ (FAIL-CLOSED):**
> É terminantemente proibido criar novas pastas ou ejetar arquivos fora das zonas delimitadas abaixo. A árvore é estática e monitorada contra entropia.
> Toda criação de arquivo deve ser precedida de consulta a este mapa e à skill `souls-territorial-compliance` (Marco 3.10 / Marco V6).

---

## 1. ZONAS DO TERRITÓRIO

### [ZONA 1: A FÁBRICA & AMBIENTE DO AGENTE] *(Ignorados no Git Principal)*
- `.agents/` -> Armazenamento global das engrenagens da IA.
- `.agents/rules/` -> Diretrizes e manifestos de contexto (DESIGN, tech-stack).
- `.agents/sidecars/` -> Dockerfiles e isolamentos efêmeros de desenvolvimento.
- `.agents/skills/` -> Catálogo de habilidades e servidores MCP locais do SOULS.
- `.agents/bin/` -> Binários de release transplantados (`souls_ui_shell.exe`, `souls_mcp_server.exe`, `agentgateway_tcp_proxy.exe`, `mcp_stdio_guard.exe`).
- `.trae/` -> Habilidades e configurações exclusivas da IDE Trae PRO+.
- `.antigravitycli/` -> Logs de execução e sessões do Antigravity CLI.
- `.vscode/` -> Perfis de workspace, tarefas e debugger local.

### [ZONA 2: ESTADO DA MÁQUINA E CACHE] *(Ignorados no Git Principal)*
- `.souls_data/` -> [L2/L3 Memory] SQLite transacional (`souls_state.db` / `souls_heuristic_vault.db`) e LanceDB vetorial. **Nota Marco 3.9.2:** `souls_heuristic_vault.db` é **auto-curativo** — `Connection::open` (rusqlite) recria o arquivo se ausente, e `ensure_repo_heuristics_schema` materializa a tabela `repo_heuristics` (84 colunas) on first use.
- `.souls/config/` -> [Marco I · v6.1] Configuração soberana do usuário (BYOK, rotas, FinOps). **SSO canônico:** `.souls/config/souls-gateway.jsonc` (JSONC). Variáveis `${VAR}` expandidas via `std::env::var` no parse-time. `gitignored` (não versionado).
- `.souls_cache/` -> Chunks temporários, hashes de arquivos e tokens.
- `.souls_sandbox/` -> Sandboxing para execução segura de módulos externos.
- `.souls_scratchpad/` -> **5 zonas efêmeras (Marco 3.10):**
  - `logs/{cargo,git,adr,marco,misc}/` -> Logs de execução, separados por origem.
  - `commits/` -> Mensagens de commit em rascunho (`commit_msg_*.md`).
  - `scripts/` -> Scripts utilitários efêmeros de inspeção (`.py`).
  - `reports/` -> Relatórios textuais da IA (`_PHASE_REPORT_...txt`).
  - `.archive/` -> Logs com mais de 30 dias (limpeza periódica).

### [ZONA 3: O CÂNONE E MEMÓRIA DE PRODUTO] *(Repositório Rígido de Documentação — Marco 3.10)*

#### 3.1. `docs/` (Zona Documental Canônica)
**Manifesto de entrada:** [SOULS_CANON_MANIFEST.md](docs/SOULS_CANON_MANIFEST.md).

- `docs/work-units/` -> **Work units ativas e históricas** (Marco 3.10).
  - `active/` -> Work units em curso (ex: `refactor-workspace-and-codebase-360/`).
  - `history/` -> Work units concluídas (ex: `marco-3.7-observability/`, `marco-3.8-c2/`, `marco-3.9-e/`, `marco-v-soda-canvas-v0.1/`).
  - `_templates/` -> Templates canônicos (`design.md`, `tasks.md`).
- `docs/planning/` -> **Planejamento e especificação de produto** (Marco 3.10).
  - `prds/` -> Product Requirement Documents vivos e `.archive/` histórico.
  - `roadmap/` -> PRDs 10.x (milestones de longo prazo).
- `docs/decisions/` -> **Decisões arquiteturais canônicas** (Marco 3.10).
  - `adrs/` -> Architecture Decision Records (45 ADRs).
  - `architecture/` -> Manuais macro (Inference, Memory, Gateway, Core Daemon, Governance, Manifesto).
  - `architecture/canibalization_essence/` -> Post-mortems de canibalização (ex: `essence-post-mortem-lean-ctx.md`).
  - `specs/` -> Dicionários tabulares e especificações fixas.
- `docs/observability/` -> **Telemetria, auditoria e estado** (Marco 3.10).
  - `audits/` -> Auditorias (4 subzonas: `blobs/`, `crates/`, `mcp_inventory/`, `quality/`).
  - `state/` -> Monitoramento canônico de crates/dependências/realidade.
  - `reports/` -> Relatórios canônicos de longo prazo.
- `docs/runtime/` -> **Artefatos vivos de design e runtime** (Marco 3.10).
  - `dags/` -> Grafos Acíclicos Dirigidos das Fases de Design/Ingestão.
  - `context_dumps/` -> Snapshots compilados do estado atual (`_ADRs_ALL.txt`, etc.).
  - `scripts/` -> Compiladores e utilitários (`souls_adr_compiler.py`, `souls_branches_sync.ps1`, etc.).
- `docs/debugs/` -> Documentos operacionais ad-hoc (debugs históricos).

#### 3.2. `.archive/` (Arquivo Frio Unificado — Marco 3.10, gitignored)
Único arquivo frio do workspace, contém tudo que está fora do caminho crítico do Produto:
- `.archive/bkps/` -> Backups de Rules, Design, UI Guidelines, etc.
- `.archive/docs-rules/` -> Regras históricas (`AGENTS.md`, `DESIGN.md`, `trae_project_rules.md`, etc.).
- `.archive/etl-blueprint/` -> Blueprints de ETL Cognitivo (kebab-case desde Marco 3.10).
- `.archive/factory-scripts/` -> Scripts de fábrica desativados (`.disabled`) e probes de linguagens.
- `.archive/soda-canon/` -> Cânone de cristalização temática (`clean_sources/`, `cold_storage/`, `crystalized/`, `raw/`, `raw_sources/`).
- `.archive/soda-neuro-genesis/` -> Genesis de neuro-temas históricos.

### [ZONA 4: O PRODUTO - CARGO WORKSPACE BARE-METAL RUST] *(Marco V6)*
- `Cargo.toml` -> Manifesto raiz do Cargo Workspace com SSOT rígido de dependências (ADR-030) e profile dev unificado (ADR-039).
- `crates/souls_protocol/` -> DTOs Serde serializáveis, envelopes IPC bidirecionais, payloads de telemetria, eventos socráticos, streaming de terminal e contratos `FrontendCommand` / `BackendResponse`.
- `crates/souls_core/` -> Motor assíncrono Tokio puro de alta performance:
  - `src/engine.rs` -> `CoreEngine` que processa comandos do frontend e despacha eventos assíncronos.
  - `src/telemetry_collector.rs` -> Coletor de telemetria do sistema em 5Hz (CPU/RAM).
  - `src/cognition/` -> Orquestração de SLMs locais, AST Tree-Sitter e engenharia de contexto.
  - `src/finops/` -> Roteadores em cascata e controle orçamentário (Iron Cost / Pareto Bandit).
  - `src/harvester/` -> Motor determinístico O(1) de clonagem, SAST e extração AST.
  - `src/persist/` -> Camada transacional de leitura/escrita e injeção no Google Sheets SSOT.
  - `src/core/` -> Governança de hardware, sandboxing Win32 AppContainer, VRAM scheduler e inferência.
  - `src/bin/` -> Binários de linha de comando (`souls_mcp_server`, `agentgateway_tcp_proxy`, `mcp_stdio_guard`, `souls_vanguard_worker`, `souls_arena_cli`, `soda_mcp_tester_cli`).
- `crates/souls_ui_shell/` -> Chassi gráfico nativo de janela e overlay:
  - `src/dwm.rs` -> Injeção FFI de material Desktop Acrylic (`DWMSBT_TRANSIENTWINDOW = 3`) via `DwmSetWindowAttribute`.
  - `src/hotkey.rs` -> Registro Win32 `RegisterHotKey` (Shift + Caps Lock) e auto-desativação atômica de Caps Lock via `GetKeyState` + `SendInput`.
  - `src/suspend.rs` -> `ComApartmentGuard` (`COINIT_APARTMENTTHREADED`) e suspensão do WebView2 quando oculto.
  - `src/ipc.rs` -> Ponte IPC bidirecional zero-copy `window.ipc.postMessage` <-> `evaluate_script`.
- `crates/anthropophagy/` -> Motor de canibalização cognitiva, swarm multi-agente e síntese O(1) de repositórios.
- `resources/` -> Recursos estáticos canônicos empacotados (`specs/`, `wasm_grammars/`).
- `vendor/` -> Dependências de código de terceiros e fontes isoladas.
- `bin/` -> Utilitários e executáveis auxiliares.
- `models/` -> Modelos e quantizações locais (ONNX / GGUF).
- `data/` -> Fixtures e recursos WASM de tempo de compilação.
- `boot.ps1` -> Orquestrador de ignição bare-metal com ancoragem forçada em iGPU (`GpuPreference=1;`) para blindagem total dos 6GB VRAM da dGPU (RTX 2060m).

### [ZONA 5: A JANELA DE VIDRO - FRONTEND SVELTE 5] *(Zero-VDOM & Apple Minimalist + Cyberpunk Sleek)*
- `src/lib/services/ipc.ts` -> Cliente TypeScript IPC nativo reativo com subscrição via `soulsIpc.onEvent` e `soulsIpc.invokeCommand`.
- `src/lib/stores/` -> Svelte 5 Runes `$state` stores desacopladas (telemetria, governança, pensamentos socráticos).
- `src/components/` -> Micro-widgets encapsulados com Container Queries estritas (`@container inline-size`) e estética Apple Minimalist + Cyberpunk Sleek (fundo transparente pass-through para o DWM, bordas 1px sutis com 8% opacidade, glows difusos Digital Cyan `#00E5FF` e Lilac `#BA9EFF`).
- `src/routes/` -> Telas e roteamento estático da interface.
- `src/index.css` -> Design System com tokens HSL, remoção de CSS blur e garantia de transparência para o material DWM Acrylic nativo.

### [ZONA 6: ZONA EXTERNA EFÊMERA (HOST %TEMP%)] *(Ignorada no Git Principal)*
- `%TEMP%/.souls_workspaces/` -> Raízes efêmeras do ProjFS e workspaces de extração (exigem NTFS/mini-filtro).
  - *Criação:* Zero-Config via `std::env::temp_dir()` + `std::fs::create_dir_all`.
  - *Teardown:* Deleção não-bloqueante via `spawn_detached_delete_process` (fora do repositório host).

---

## 2. REGRAS DE FORMA E HIGIENE DIGITAL

### 2.1. Nomenclatura
- **Pastas:** `kebab-case` (ex: `work-units/`, `decision/`, `etl-blueprint/`). NUNCA `snake_case` para pastas top-level.
- **Arquivos de docs:** Prefixos canônicos (`ADR-NNN-`, `PRD-NNN-`, `DAG_`, `debug-*`).
- **Pastas internas (públicas):** Podem usar `snake_case` apenas para agrupamentos técnicos (ex: `clean_sources/`, `cold_storage/` dentro de `soda-canon/`).
- **Pastas reservadas com prefixo underscore:** Indicam zonas especiais (ex: `_templates/`, `_archive/`, `_audit/`).

### 2.2. Compliance Territorial (FAIL-CLOSED)
- **PRDs em andamento** devem residir em `docs/planning/prds/` com prefixo da Fase Temporal (ex: `PRD_00X_Fase_X_...`).
- **ADRs novos** devem ser criados em `docs/decisions/adrs/` com numeração sequencial.
- **Work units ativas** devem usar `docs/work-units/active/<tipo>-<slug>/{design.md, tasks.md}`.
- **Logs de execução** devem ser ejetados em `.souls_scratchpad/logs/<origem>/` (origem: `cargo`, `git`, `adr`, `marco`, `misc`).
- **Mensagens de commit** em rascunho devem ir para `.souls_scratchpad/commits/commit_msg_*.md`.
- **Scripts de inspeção efêmeros** devem ir para `.souls_scratchpad/scripts/`.
- **Outputs textuais da IA** (relatórios de fase) devem ir para `.souls_scratchpad/reports/`.

### 2.3. Política de Retenção
- **`.souls_scratchpad/logs/`:** Rotação a cada 30 dias. Logs antigos migrados para `.souls_scratchpad/.archive/`.
- **`.archive/`:** Append-only. Nada é removido. Apenas renomeações kebab-case e unificações são permitidas.
- **`docs/planning/prds/.archive/`:** PRDs históricos ficam para referência. Podem ser compactados anualmente.

### 2.4. Auditoria
- O agente deve, ao iniciar cada sessão, **ler este `_WORKSPACE_MAP.md`** e consultar a skill `souls-territorial-compliance` antes de criar qualquer arquivo fora de zonas conhecidas.
- O script `docs/runtime/scripts/audit_workspace_compliance.py` deve ser invocado em auditorias periódicas para detectar violações territoriais (arquivos órfãos, paths hardcoded quebrados, etc.).

---

## 3. CHANGELOG TERRITORIAL

- **v7.0 (Marco V6):** Transposição de domínio e desacoplamento bare-metal completo.
  - Extinção definitiva do monólito `src-tauri/`.
  - Migração para Cargo Workspace Multi-Crate soberano (`crates/souls_protocol`, `crates/souls_core`, `crates/souls_ui_shell`, `crates/anthropophagy`).
  - Chassi gráfico nativo Winit 0.30 + Wry 0.55 com Desktop Acrylic DWM FFI (`DWMSBT_TRANSIENTWINDOW = 3`).
  - Hotkey global Win32 Shift + Caps Lock com desligamento atômico de LED via `SendInput`.
  - Ponte IPC zero-copy via canais MPSC Tokio (`window.ipc.postMessage` / `evaluate_script`).
  - Ancoragem forçada do executável na iGPU (`GpuPreference=1;`) via `boot.ps1` para poupar 100% dos 6GB de VRAM da RTX 2060m.
- **v6.0 (Marco 3.10):** Reorganização completa da ZONA 3 (work-units, planning, decisions, observability, runtime).
- **v5.1 (Marco 3.9.2):** Nota sobre `souls_heuristic_vault.db` auto-curativo.
- **v5.0 (Marco 3.9):** Adoção de `.souls_data` e reorganização de zonas.
- **v4.x e anteriores:** Estrutura legada pré-`souls`.

---

> Última revisão: Marco V6 — `refactor-baremetal-workspace-v6`
