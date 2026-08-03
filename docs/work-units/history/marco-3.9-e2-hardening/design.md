# Design — Marco 3.9 Fase E.2: Hardening E2E e Barramento Assíncrono Socrático

> Emenda ao [design-marco-3.9-e.md](design-marco-3.9-e.md).
> Referência normativa: [ADR-045](adrs/ADR-045-Persistencia-da-Alma-Socratica.md).

## 1. Contexto e Motivação

O Marco 3.9 Fase E (PR #21) introduziu a persistência socrática (V5) com 4 testes TDD
protegidos por um **mutex global síncrono** (`MARCO_39_FASE_E_LOCK`) que serializava
a suíte. Esse padrão é uma **dívida técnica inaceitável** sob três óticas:

1. **Fricção Cognitiva (SODA):** locks síncronos no critical path do Tokio event loop
   geram contenção invisível que aparece apenas sob carga (Backpressure de 2ª ordem).
2. **Test Pollution:** o lock global mascara race conditions reais entre os 4 testes
   (todos batem no mesmo banco `:memory:` por meio de canais diferentes).
3. **Agnosticismo de Hardware:** o padrão "1 mutex global" não escala para Apple
   Silicon (cores M-series) nem para Linux NUMA. Ele é um beco de 1 thread.

## 2. Topologia FinOps — Barramento Assíncrono Socrático

```mermaid
graph LR
    subgraph Cliente["Cliente MCP (Trae IDE / Svelte 5)"]
        LLM[LLM Agent]
        Svelte[Svelte 5 Renderer<br/>invoke() tauri::command]
    end

    subgraph Gateway["Gateway MCP (souls_mcp_server)"]
        Dispatcher[Dispatcher / tools/call]
        Merge[merge_sessions<br/>Hiper-Forward]
        Export[export_session<br/>read-only]
        Analyze[analyze_session<br/>read-only]
    end

    subgraph Tauri["Backend Tauri v2 (main.rs)"]
        IPC1[#[tauri::command]<br/>socratic_export_session]
        IPC2[#[tauri::command]<br/>socratic_analyze_session]
        IPC3[#[tauri::command]<br/>socratic_merge_sessions]
    end

    subgraph Bridge["SocraticWriteWorker (std::thread dedicada)"]
        Channel["tokio::sync::mpsc<br/>bounded(512)<br/>SocraticOp::UpsertThought*"]
        Worker["Worker loop<br/>blocking_recv<br/>INSERT 1×1 (txn)"]
    end

    subgraph StateDB[".souls_data/souls_state.db (WAL STRICT v5)"]
        Sessions[socratic_sessions<br/>STRICT TEXT PK]
        Thoughts[socratic_thoughts<br/>FK CASCADE]
    end

    LLM -->|tools/call merge_sessions| Dispatcher
    Svelte -->|invoke socratic_merge_sessions| IPC3
    Dispatcher --> Merge
    IPC1 --> Export
    IPC2 --> Analyze
    IPC3 --> Merge

    Merge -->|try_send SocraticOp::UpsertThought| Channel
    Export -.->|SELECT| Thoughts
    Analyze -.->|SELECT| Thoughts

    Channel -->|blocking_recv| Worker
    Worker -->|INSERT OR REPLACE<br/>txn batched| Thoughts
    Worker -->|PRAGMA user_version = 5| Sessions
```

## 3. Padrão Orchestrator-Worker (Hiper-Forward)

- **Orchestrator (Tokio runtime):** O `Dispatcher` MCP e os comandos Tauri `IPC1-3`
  compartilham a mesma função `run_souls_merge_sessions`, que constrói o envelope
  `SocraticOp` e dispara via `mpsc::Sender::try_send`. **Nunca** bloqueia esperando
  ACK (fire-and-forget para escritas; ACK explícito apenas em fluxos de auditoria HITL).
- **Worker (std::thread `SocraticWriteWorker`):** Consome `SocraticOp` via
  `blocking_recv` numa thread dedicada, mantendo `rusqlite` síncrono e isolado do
  event loop. Bounded channel (512) = **backpressure natural** — se o consumidor
  travar, o produtor recebe `TrySendError::Full` e pode fazer backoff ou HITL.
- **Tabela de decisão:**
  - `export_session` / `analyze_session` → read direto (SELECT), sem MPSC.
  - `merge_sessions` (escrita) → `try_send` (Hiper-Forward), worker serializa.
  - Cenário de stress 10k pensamentos → 10k `try_send` sequenciais devem completar
    em **< 200ms** (latência média de `try_send` < 20µs).

## 4. Agnosticismo de Hardware

- **Piso de Validação:** RTX 2060m (6GB VRAM, 16GB RAM, Windows ReFS). O canal MPSC
  usa `std::thread::spawn` que mapeia 1:1 em qualquer arquitetura; tokio mpsc é
  platform-agnostic (usa `crossbeam` por baixo).
- **Teto Agnóstico:** O padrão "bounded mpsc + worker dedicado" é o **canônico Rust**
  e roda idêntico em:
  - Apple Silicon M-series (cores P+E, 1 thread ainda é 1 thread).
  - Linux NUMA (worker thread fica no node local; produtor pode pinar via `core_affinity`).
  - WebAssembly (com `wasm32-wasi` e adaptação do `std::thread` → future).
- **Zero `#cfg(target_arch)`:** o design é puramente logical; não há código
  arquitetural-condicional.

## 5. Estado e Transações

| Operação | Tipo | Garantia | Lock |
|----------|------|----------|------|
| `merge_sessions` (write) | via MPSC `SocraticOp::UpsertThought` | INSERT OR REPLACE (idempotente) | `try_send` (não-bloqueante) |
| `SocraticWriteWorker` consume | single-thread loop | WAL writer, busy_timeout 5s | row-level (WAL) |
| `export_session` (read) | direto via `Connection` | SELECT puro | shared lock (read) |
| `analyze_session` (read) | direto via `Connection` | SELECT + compute | shared lock (read) |
| `migrate_v3_to_v5` (cold) | `Connection::execute_batch` | idempotente | exclusive (transient) |

**Lei do NUNCA-BLOQUEAR:** o `try_send` é o disjuntor. Se o canal está saturado
(> 512 mensagens não-consumidas), o produtor **NÃO** espera — recebe
`TrySendError::Full` e decide:
1. Logar warning + descartar (Hiper-Forward, sem ACK).
2. Sinalizar erro ao cliente MCP (apenas em caminhos síncronos críticos).
3. HITL pause (apenas em fluxos de auditoria onde perda é inaceitável).

## 6. Custos e Hot Paths (Estimados em RTX 2060m)

| Operação | Custo Esperado | Observação |
|----------|----------------|------------|
| `SocraticOp::UpsertThought::try_send` | ~5-20µs | Memcpy do envelope + enqueue atômico |
| `Worker::INSERT` (1 thought) | ~30-80µs | WAL fsync mínimo, índice hit |
| 10k `try_send` (loop síncrono) | < 200ms | 20µs × 10k = 200ms, banda IO 50k/s |
| 10k `Worker::INSERT` (background) | ~3-5s | I/O-bound, paralelismo com Tokio event loop |
| `cargo test --bin souls_mcp_server` | < 10s | Higiene Térmica: --no-default-features |

## 7. Lei do Scaffold (DoD Pré-Codificação)

Cada tarefa em `tasks.md` tem teste vermelho antes da lógica real.
Definição de Done estrita:

- [x] `cognition/thinking/socratic_bridge.rs` compila sem warnings.
- [x] `MARCO_39_FASE_E_LOCK` extirpado: `grep -r MARCO_39_FASE_E_LOCK src/` = 0.
- [x] `SOCRATIC_TX: OnceLock<mpsc::Sender<SocraticOp>>` injetado no init.
- [x] Comandos Tauri `socratic_*` registrados em `main.rs::invoke_handler`.
- [x] `test_socratic_load_10k_thoughts` verde; ≥ 95% dos pensamentos lidos de volta.

## 8. Riscos e Mitigações (Pessimismo da Razão)

| Risco | Mitigação |
|-------|-----------|
| `try_send` sempre retorna Full em pico | Backpressure observability log + métrica `socratic_oversized_queue` |
| Worker thread trava em I/O síncrono | `busy_timeout(5000ms)` evita deadlock SQLite, e thread `JoinHandle` permite watchdog futuro |
| 10k thoughts excedem memória | Teste usa banco `:memory:` (zero I/O), mas valida que o envio é 100% não-bloqueante |
| Tauri command retorna JSON inválido | `Result<Value, String>` mapeia `RpcError → String` na fronteira, Svelte 5 captura gracefully |
| Concorrência entre múltiplos processos Tauri | `OnceLock` garante **um** worker por processo; multi-processo usaria file-lock no SQLite (WAL já blinda) |

## 9. Compatibilidade com a Lei 32/120 (ADR-041)

| Tool Tauri | Tamanho | Descrição |
|------------|---------|-----------|
| `socratic_export_session` | 23 chars | "Exporta arvore socratica SQLite como JSON/MD. Result<Value,String>." |
| `socratic_analyze_session` | 25 chars | "Calcula FinOps cognitivos (revisao, ramificacao, latencia) por sessao." |
| `socratic_merge_sessions` | 23 chars | "Fusao atomica last-write-wins entre sessoes com remap de parent FK." |

Todas ≤ 32 chars. Descrições ≤ 120 chars. Servername soberano `souls_mcp` (futuro).

## 10. Invariantes de Compile-Time (Blindagem)

```rust
// 32 chars max para nome Tauri
const _: () = assert!(
    "socratic_export_session".len() <= 32,
    "Lei 32/120 violada: nome Tauri excede 32 chars"
);
```

(Em Rust 1.79+: `const _: () = assert!(...);` aceita apenas expressões estáticas —
não format strings. Validação literal.)
