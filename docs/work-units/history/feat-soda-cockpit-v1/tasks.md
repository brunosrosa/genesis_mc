# Tasks: SODA Canvas v0.1 Cockpit Integrado (Svelte 5 / Tauri v2)

## Definition of Done (DoD)
1. Design System configurado estritamente com tokens SODA em Tailwind v4 (`src/index.css`).
2. Arquitetura Geométrica de 5 Camadas funcional e responsiva em `src/App.svelte` e subcomponentes.
3. Spotlight Zen conversacional (Camada 4) acessível via `Alt+Space` com suporte a micro-comandos e expansão socrática.
4. Fiação IPC de telemetria operando em Zero-Copy Stream com rAF micro-batching e `$state.snapshot` garantido.
5. Banimento total de animações de spinners em favor de Ambient Status com Ghost Borders.
6. Compilação offline via `boot.ps1 -Build` com Exit Code 0 absoluto e sem warnings no clippy.
7. Logs de compilação direcionados para `.souls_scratchpad/logs/cargo/clippy_soda_cockpit.log`.

---

## Plano de Tarefas Atômicas

- [x] **TASK-01: Governança Territorial & Work-Unit**
  - Registrar design e tasks em `docs/work-units/active/feat-soda-cockpit-v1/`.
  - Configurar diretório de log `.souls_scratchpad/logs/cargo/`.

- [x] **TASK-02: Design System & Tokens Tailwind v4 (@theme)**
  - Auditar e refinar `src/index.css` com tokens SODA (oklch 0% 0 0, mica glass 20px, ghost borders, fontes Space Grotesk / JetBrains Mono / Space Mono).
  - Incluir curvas de transição `cubic-bezier(0.2, 0.8, 0.2, 1)`.

- [x] **TASK-03: Arquitetura de 5 Camadas Planar**
  - Auditar `src/App.svelte` (Camada 0 Substrate Shell).
  - Validar `GovernorRail.svelte` (Camada 1: w-16 navigation).
  - Validar `TelemetryHUD.svelte` (Camada 2: Real-time ECG topbar).
  - Validar Active Canvas (Camada 3: Dashboard, Socratic, Inbox).
  - Validar `SpotlightZen.svelte` (Camada 4: Ephemeral Zen Layer).
  - Validar `TerminalDrawer.svelte` (Camada 5: 250ms GPU drawer com tombstone virtual).

- [x] **TASK-04: Soldagem do Spotlight Zen e Atalho Alt+Space**
  - Configurar captura de `Alt+Space` e comunicação de foco no backend e frontend.
  - Assegurar micro-comandos JIT, expansão socrática e expurgo 100% da RAM na finalização.

- [x] **TASK-05: Fiação IPC de Telemetria (Zero-Copy & Micro-Batching)**
  - Assegurar decodificação de 8 bytes `u64` little-endian via `DataView` no `telemetry.svelte.ts`.
  - Assegurar isolamento de proxy com `$state.snapshot()`.
  - Manter loop `requestAnimationFrame` estável a 60 FPS.
  - Eliminar qualquer componente de spinner em favor de Ambient Status com Ghost Borders.

- [x] **TASK-06: Homologação TDD & Build Offline**
  - Executar checagem de clippy com saída para `.souls_scratchpad/logs/cargo/clippy_soda_cockpit.log`.
  - Rodar `boot.ps1 -Build` ou `cargo test` / `vite build` para validação de produção.
