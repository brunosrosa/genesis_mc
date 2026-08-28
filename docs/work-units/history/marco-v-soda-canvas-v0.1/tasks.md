---
spec: marco-v-soda-canvas-v0.1
phase: 4-tasks
design: docs/work-units/active/marco-v-soda-canvas-v0.1/design.md
branch: feat/marco-v-soda-canvas-v0.1
---

# Tasks — Marco V (SODA Canvas v0.1 + Tauri IPC Zero-Copy Telemetry Bridge)

Cada task tem DoD (Definition of Done) executável. TDD atômico (Red-Green-Refactor) — `cargo check` + `pnpm build` limpos antes do HITL Gate.

## TASK-01 — `telemetry/watchdog_ipc.rs` (binário puro 8 bytes)

**Arquivo:** `src-tauri/src/telemetry/watchdog_ipc.rs` (NOVO)

- [ ] `pub fn spawn_watchdog_channel(app: tauri::AppHandle, channel: tauri::ipc::Channel<Vec<u8>>)` em < 60 linhas
- [ ] `tokio::spawn` task que poll `WATCHDOG_STATE` a 1000ms
- [ ] `Ordering::Relaxed` no load (lock-free, sem starvation do Tokio)
- [ ] `state.to_le_bytes()` → `channel.send(bytes.to_vec())` (8 bytes exatos)
- [ ] `select!` com cancelamento via `tokio::sync::watch` ou `tokio::time::interval` para parar quando `app` for derrubado
- [ ] Teste TDD: `pack_state_roundtrip_le_bytes` valida 8 bytes LE exatos

**DoD:**
- `cargo check --features tauri-app` → Exit 0, zero warnings
- Teste: `test_pack_state_little_endian_bytes` valida que `state.to_le_bytes()` bate byte-a-byte com o esperado

## TASK-02 — `telemetry/blast_bridge.rs` (re-emite ImpactReport)

**Arquivo:** `src-tauri/src/telemetry/blast_bridge.rs` (NOVO)

- [ ] `pub async fn emit_blast_pending(app: &tauri::AppHandle, report: serde_json::Value)` 
- [ ] `app.emit("blast_radius_pending", report)` (1× por mudança)
- [ ] `tracing::debug!` no `stderr` (NUNCA `stdout` — ADR-003)
- [ ] Documenta no header que o JSON aqui é ≤ 50KB (relatório estático) — não viola ADR-003 R1 porque é evento de controle discreto, NÃO Data Plane contínuo

**DoD:**
- `cargo check --features tauri-app` → Exit 0, zero warnings
- Teste: `test_emit_blast_pending_uses_event_api` (mock com `tauri::test::mock_app()`)

## TASK-03 — `lib.rs` + `main.rs` (registra command + setup)

**Arquivos:** `src-tauri/src/lib.rs` (EDIT) + `src-tauri/src/main.rs` (EDIT)

- [ ] `pub mod watchdog_ipc;` e `pub mod blast_bridge;` em `lib.rs`
- [ ] `pub mod telemetry;` em `lib.rs` mantém subdir pattern (Rust 2018+): `telemetry.rs` é a raiz e os subdirs ficam em `telemetry/`
- [ ] Em `telemetry.rs`: `pub mod watchdog_ipc; pub mod blast_bridge;`
- [ ] Em `main.rs`: novo `#[tauri::command] async fn start_watchdog_stream(app: AppHandle, channel: Channel<Vec<u8>>)`
- [ ] `tauri::generate_handler![..., start_watchdog_stream]`
- [ ] `.setup()` inicializa o watchdog (cria `Interval` + `select!`)

**DoD:**
- `cargo check --features tauri-app` → Exit 0

## TASK-04 — `capabilities/default.json` (permissão de Channel customizado)

**Arquivo:** `src-tauri/capabilities/default.json` (EDIT)

- [ ] Adicionar `"core:channel:default"` em `permissions`
- [ ] Adicionar `"core:event:default"` em `permissions`
- [ ] Adicionar `"core:event:allow-emit"` para o evento `blast_radius_pending`

**DoD:**
- `cargo check --features tauri-app` → Exit 0
- Tauri compila sem warning de permissão faltando

## TASK-05 — `src/lib/stores/telemetry.svelte.ts` (Runes + DataView)

**Arquivo:** `src/lib/stores/telemetry.svelte.ts` (NOVO)

- [ ] `export const telemetry = $state({ vram_mb: 0, ram_mb: 0, cpu_temp: 0, gpu_temp: 0 })`
- [ ] `export const thermal_status = $derived(...)` — "PRESSAO_CRITICA" se vram > 5000 MB, "OCIOSO" caso contrário
- [ ] `export function decode_payload(arrayBuffer: ArrayBuffer)` — usa `DataView` + `getBigUint64(0, true /* little-endian */)` + máscaras de bit idênticas ao Rust
- [ ] `export function bind_channel_to_runes()` — usa `requestAnimationFrame` para throttlar updates (decoupling 1Hz Rust ↔ 60Hz UI)
- [ ] Usa `@tauri-apps/api/core::invoke` para iniciar o canal

**DoD:**
- `pnpm vitest run src/lib/stores` → Exit 0
- Teste: `telemetry.test.ts` valida decoder bate byte-a-byte com payload sintético

## TASK-06 — `src/lib/stores/blast.svelte.ts` (Runes para ImpactReport)

**Arquivo:** `src/lib/stores/blast.svelte.ts` (NOVO)

- [ ] `export const pendingBlast = $state<ImpactReport | null>(null)`
- [ ] `export function listen_for_blast_radius()` — `listen` da `@tauri-apps/api/event` para o evento `blast_radius_pending`

**DoD:**
- `pnpm run build` → Exit 0
- Compila sem erros TS

## TASK-07 — `src/lib/components/AgentInbox.svelte` (Gaveta + Heatmap + Slider)

**Arquivo:** `src/lib/components/AgentInbox.svelte` (NOVO)

- [ ] Gaveta lateral `position: fixed; right: 0; top: 0; transform: translateX(100%); transition: 250ms`
- [ ] Renderiza `HeatmapCell` para cada `pendingBlast.affected_files`
- [ ] Renderiza Myers Diff em fonte mono dentro de container com `contain: layout size`
- [ ] `<input type="range" min="0" max="100" class="glow-slider" style="--value: 50%">` 
- [ ] `bind:value` + `on:change` para despachar HITL (invoke `approve_blast_radius` ou `reject_blast_radius`)
- [ ] Visível apenas quando `pendingBlast` não é null

**DoD:**
- `pnpm run build` → Exit 0
- Compila sem erros Svelte/TS

## TASK-08 — `src/lib/components/HeatmapCell.svelte` (célula térmica)

**Arquivo:** `src/lib/components/HeatmapCell.svelte` (NOVO)

- [ ] Props: `severity: 0..1` (0 = verde, 1 = vermelho)
- [ ] Background `oklch(0.55 0.25 30 * severity)` interpolado
- [ ] `aria-label` descritivo

**DoD:**
- `pnpm run build` → Exit 0

## TASK-09 — `src/index.css` (Ghost Borders + Glow Slider + tema)

**Arquivo:** `src/index.css` (EDIT)

- [ ] `--background: oklch(0.12 0 0);` (preto absoluto)
- [ ] `--font-mono: ui-monospace, "JetBrains Mono", monospace`
- [ ] `--font-sans: "Space Grotesk", "Inter", system-ui, sans-serif`
- [ ] `.ghost-border` com `border: 1px solid transparent; background: linear-gradient(var(--background), var(--background)) padding-box, var(--ghost-gradient) border-box; animation: ghost-breathe 250ms ...`
- [ ] `.ghost-border--thinking` (roxo-néon)
- [ ] `.ghost-border--compiling` (cobre-laranja)
- [ ] `.ghost-border--idle` (cinza estático)
- [ ] `.glow-slider` com `box-shadow: 0 0 calc(var(--value, 0) * 0.12px) oklch(0.85 0.18 296)`
- [ ] Proibido `transition` em propriedades que causam reflow (`width`, `height`, `top`, `left`)

**DoD:**
- `pnpm run build` → Exit 0
- CSS validado sem warnings

## TASK-10 — `src/App.svelte` (integração)

**Arquivo:** `src/App.svelte` (EDIT)

- [ ] Importa `AgentInbox`
- [ ] `onMount` chama `bind_channel_to_runes()` e `listen_for_blast_radius()`
- [ ] `<AgentInbox />` no final do `<main>`
- [ ] `<svelte:head>` define fonts via `<link>` para Google Fonts (Space Grotesk + JetBrains Mono)

**DoD:**
- `pnpm run build` → Exit 0

## TASK-11 — `package.json` (vitest)

**Arquivo:** `package.json` (EDIT)

- [ ] Adicionar `"test": "vitest run"` em scripts
- [ ] Adicionar `"vitest": "^2.1.0"` em devDependencies

**DoD:**
- `pnpm install` → Exit 0
- `pnpm test` → Exit 0

## TASK-12 — Validação Final

- [ ] `cd src-tauri && cargo check --features tauri-app` → Exit 0, zero warnings
- [ ] `cd src-tauri && cargo test --features tauri-app telemetry::` → Exit 0
- [ ] `pnpm install` → Exit 0
- [ ] `pnpm run build` → Exit 0
- [ ] `pnpm test` → Exit 0
- [ ] Sem regressão nos 489 testes existentes do chassi bare-metal

## TASK-13 — Blast Radius Report + HITL Gate

- [ ] `git add -A && git status` capturado
- [ ] Diff stats enviado para Agent Inbox (chat)
- [ ] Sem merge em main; aguarda aprovação do Arquiteto
