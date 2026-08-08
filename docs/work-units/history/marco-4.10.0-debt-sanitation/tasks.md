---
spec: marco-4.10.0-debt-sanitation-and-cognitive-coupling
phase: 3-tasks
design: docs/work-units/active/marco-4.10.0-debt-sanitation/design.md
branch: feat/marco-4.10.0-debt-sanitation
---

# Tasks — Marco 4.10.0 Saneamento de Dívidas e Acoplamento Cognitivo

Cada task tem um DoD (Definition of Done) executável. Tasks marcadas `[SCAFFOLD]` exigem teste vazio de falha antes da lógica real (Lei do Scaffold do SDD).

## Diretrizes Inegociáveis do Arquiteto-Chefe

1. **DIRETRIZ 1**: `first_betti_number()` ≠ contagem de ciclos espacial cega (m-n+c). Modelar cohomologia de feixe socrático via **rank do operador d0** sobre a matriz de restrições de consistência factual. Gauss-Elimination **puro CPU em stack (≤ 1024 entradas), zero alocação na heap**. H¹ ≠ 0 apenas para **contradição de restrições de verdade** (ciclos harmoniosos = H¹ = 0).
2. **DIRETRIZ 2**: É **TERMINANTEMENTE PROIBIDO** invocar o prober epistêmico ou qualquer CPU-bound síncrono na thread async do proxy L7. Enviar para thread OS dedicada via canal MPSC, receber via `oneshot::channel()` (await). Zero stalls.
3. **DIRETRIZ 3**: Quando `disjuntor_ativo` for disparado, emitir Tauri Event `socratic_interrupt` com payload `{scores, prompt_truncado}`. Interromper JSON-RPC com erro tipado `-32001` (HitlDenied).
4. **DIRETRIZ 4**: Matriz de testes TDD = 10 testes net-new, todos verdes, `cargo clippy -D warnings` Exit 0.

---

## TASK-01 [SCAFFOLD] — VerbalizerMap (ETAPA 1)

**Arquivo:** `src-tauri/src/core/epistemic_prober.rs` (EDIT)

**Escopo:** Adicionar struct `VerbalizerMap` que resolve IDs verbais ("true", "false", "yes", "no", "0", "1") em runtime. Dois modos:
- **MOCK** (vocab_size = 128): usa FNV-1a hash determinístico (canibalizado de `llama_logit_probing.rs::seed_logit`).
- **REAL** (vocab_size ≥ 1024): usa tokenizador real da `llama-cpp-2` crate via `llama_tokenize` (abstração atrás de um trait para que testes não precisem de modelo carregado).

Refatorar `verbalizer_ratio` para receber `&VerbalizerMap` em vez de `Range<usize>`. Manter os 12 testes existentes do Marco 4.9.4 verdes.

- [ ] Struct `pub struct VerbalizerMap` com `vocab_size: usize`, `entries: BTreeMap<&'static str, u32>`
- [ ] `VerbalizerMap::for_mock_vocab(vocab_size: usize) -> Self` (FNV-1a)
- [ ] `VerbalizerMap::from_tokenizer_resolver(...)` para path real
- [ ] `VerbalizerMap::resolve(&self, label: &str) -> Option<u32>`
- [ ] `VerbalizerMap::resolve_pair(&self, pos: &str, neg: &str) -> (Vec<u32>, Vec<u32>)`
- [ ] `LlamaCppEpistemicProber` ganha campo `verbalizer_map: VerbalizerMap`
- [ ] `verbalizer_ratio` refatorado para receber `&VerbalizerMap`

**DoD:**
- `cargo check` Exit Code 0
- **4 testes TDD novos** (`test_verbalizer_map_*`):
  1. `test_verbalizer_map_mock_resolves_deterministic_ids` — FNV-1a é reprodutível
  2. `test_verbalizer_map_mock_distinguishes_pos_neg` — `true`≠`false`, `yes`≠`no`
  3. `test_verbalizer_map_real_resolver_propagates_tokenizer_errors` — fallback gracioso
  4. `test_verbalizer_map_used_by_prober_instead_of_hardcoded_ranges` — prober usa `VerbalizerMap` em vez de `SAFE_RANGE`/`UNSAFE_RANGE`/`CONFLICT_RANGE`/`ALIGN_RANGE`
- 12 testes existentes do Marco 4.9.4 permanecem verdes

---

## TASK-02 [SCAFFOLD] — CohomologyEngine (ETAPA 2)

**Arquivo:** `src-tauri/src/core/cohomology.rs` (NOVO)

**Escopo:** Implementar a cohomologia de feixes socráticos. **NÃO** usar a fórmula `m - n + c`. Em vez disso:

1. Carregar `entities` e `relations` STABLE do `souls_state.db` (tabela `relations` existente, filtrada por heurística).
2. Modelar cada relação como uma **restrição de consistência factual**:
   - `depends_on`: x_u = x_v (linha: [0..0; +1 em u, -1 em v])
   - `conflicts_with`: x_u + x_v = 1 (linha: [+1 em u, +1 em v, b=1])
   - `implies`: x_u → x_v ≡ -x_u + x_v ≥ 0 (modelado como x_v ≥ x_u, em ℝ)
3. Montar matriz de restrições A (m × n) + vetor b (m × 1), tudo em **stack de tamanho fixo (≤ 1024)**.
4. Aplicar Gauss-Elimination parcial (in-place em stack, **zero alocação na heap**).
5. Calcular `rank(A)`.
6. H¹ = dim(coker(d0)) onde d0 é a parte homogênea de A.
7. **H¹ ≠ 0 apenas se houver contradição de verdade**: testar se o sistema `Ax = b` tem solução. Se sim → H¹ = 0 (mesmo com ciclos harmoniosos). Se não → H¹ = m - rank(A, augmentado com b).

**Algoritmo de Gauss-Elimination stack-only (≤ 1024):**
```rust
const MAX_FACTS: usize = 1024;
struct RestrictionMatrix {
    data: [[f32; MAX_FACTS + 1]; MAX_FACTS], // Augmentada com coluna b
    rows: usize, cols: usize,
}
fn rank_augmented(&mut self) -> usize { /* pivoteamento parcial */ }
```

- [ ] `pub struct RestrictionMatrix` com `[[f32; 1025]; 1024]` e zero alocação
- [ ] `pub struct SqliteFactGraph` que carrega entities + relations de `souls_state.db`
- [ ] `pub struct CohomologyResult { h1_dimension: usize, has_contradiction: bool }`
- [ ] `pub fn compute_cohomology(graph: &SqliteFactGraph) -> CohomologyResult`
- [ ] `pub fn boost_conflito_memoria(base: f32, h1: &CohomologyResult) -> f32` (boost > 0.85 se H¹ > 0)
- [ ] `pub mod` em `core/mod.rs`

**DoD:**
- `cargo check --workspace` Exit Code 0
- **3 testes TDD novos** (`test_cohomology_*`):
  1. `test_cohomology_acyclic_graph_has_zero_h1` — grafo sem ciclos → H¹ = 0
  2. `test_cohomology_harmonious_cycle_has_zero_h1` — ciclo de `depends_on` (A→B→C→A) → b₁ = 1, mas H¹ = 0 (sem contradição de verdade)
  3. `test_cohomology_conflicting_premises_boosts_score_above_threshold` — grafo com `conflicts_with` contraditório → H¹ > 0, `boost_conflito_memoria(0.5) > 0.85`
- Zero alocação na heap (verificado por inspeção: apenas `[[f32; N]; M]` em stack)
- Latência da Gauss-Elimination para 100 fatos: < 1ms

---

## TASK-03 [SCAFFOLD] — SocraticEventBus + Tauri Emit (ETAPA 3)

**Arquivos:**
- `src-tauri/src/core/socratic_event_bus.rs` (NOVO)
- `src-tauri/src/bin/souls_mcp_server.rs` (EDIT)

**Escopo:** Quando o disjuntor é ativado em `run_intent`:
1. Emitir evento `socratic_interrupt` com payload `{scores, prompt_truncado, session_id}`.
2. Retornar erro JSON-RPC com código `-32001` (HitlDenied) e `data.hitl_required = true`.

Abstração via trait `SocraticEventSink`:
- `TauriSocraticSink` (produção): usa `AppHandle::emit` se disponível, senão no-op + log.
- `InMemorySocraticSink` (testes): armazena eventos em `Arc<Mutex<Vec<SocraticInterrupt>>>`.

- [ ] `pub trait SocraticEventSink: Send + Sync`
- [ ] `pub struct TauriSocraticSink { app_handle: Option<AppHandle> }`
- [ ] `pub struct InMemorySocraticSink { events: Arc<Mutex<Vec<SocraticInterrupt>>> }`
- [ ] `pub struct SocraticInterrupt { scores: EpistemicScores, prompt_truncated: String, session_id: String }`
- [ ] Constante `pub const RPC_HITL_DENIED_CODE: i32 = -32001;`
- [ ] `SOCRATIC_SINK: OnceLock<Arc<dyn SocraticEventSink>>` global
- [ ] `pub fn set_socratic_sink(sink: Arc<dyn SocraticEventSink>)`
- [ ] `run_intent` chama sink + retorna erro `-32001` quando disjuntor_ativo
- [ ] `init_state_db_and_worker` registra o sink apropriado (Tauri se feature, InMemory em testes)

**DoD:**
- `cargo check --workspace` Exit Code 0
- **1 teste TDD novo**:
  - `test_intent_disjuntor_emits_socratic_signal` — injeta `InMemorySocraticSink`, prompt vago, verifica que evento foi emitido + erro -32001 retornado
- Payload do evento contém `scores.ambiguidade`, `scores.risco_relacional`, `prompt_truncated` (≤ 256 chars)
- Erro JSON-RPC tem estrutura `{code: -32001, message: "HitlDenied: disjuntor socrático ativo", data: {hitl_required: true}}`

---

## TASK-04 [SCAFFOLD] — L7 Shield com Canal MPSC (ETAPA 4)

**Arquivo:** `src-tauri/src/bin/agentgateway_tcp_proxy.rs` (EDIT)

**Escopo:** Acoplar triagem epistêmica na entrada de tool-calls mutating críticos (`execute`, `edit`, `delete`).

**DIRETRIZ 2 (inegociável):**
- **PROIBIDO** chamar `prober.probe()` na thread async do proxy
- Enviar para thread OS dedicada via canal MPSC
- Receber via `oneshot::channel()` (await)
- Zero stalls na thread de rede

**Implementação:**
1. Criar `EpistemicShieldChannel` que envolve:
   - `mpsc::UnboundedSender<ShieldRequest>`
   - Thread OS dedicada que processa requests síncronamente
2. `ShieldRequest { prompt_payload: String, reply: oneshot::Sender<ShieldVerdict> }`
3. `ShieldVerdict { risk_class: f32, block: bool }`
4. `handle_l7_proxy` chama `channel.triage(&body).await` (await non-blocking) antes de qualquer `mutate_json_payload`
5. Se `verdict.block == true`, redirecionar para `Agent Inbox` e notificar Tauri via sink compartilhado

- [ ] `pub struct EpistemicShieldChannel { tx: mpsc::UnboundedSender<ShieldRequest> }`
- [ ] `pub struct ShieldRequest { prompt: String, reply: oneshot::Sender<ShieldVerdict> }`
- [ ] `pub struct ShieldVerdict { risk_class: f32, block: bool }`
- [ ] `EpistemicShieldChannel::spawn() -> Self` (cria canal + thread OS)
- [ ] `EpistemicShieldChannel::triage(&self, prompt: &str) -> ShieldVerdict` (async, await oneshot)
- [ ] `is_mutating_tool_call(body: &[u8]) -> bool` (heurística: presença de "execute", "edit", "delete" no JSON)
- [ ] `redirect_to_agent_inbox(body, sink)` para tool-calls bloqueados
- [ ] `handle_l7_proxy` chama shield **antes** de `mutate_json_payload`

**DoD:**
- `cargo check --workspace` Exit Code 0
- **2 testes TDD novos** (`test_l7_shield_*`):
  1. `test_l7_shield_readonly_payload_passes_through` — tool `read`/`search` → não bloqueado
  2. `test_l7_shield_mutating_with_high_risk_redirects_to_inbox` — tool `execute` com prompt suspeito → bloqueado, enviado para inbox
- Thread de rede **nunca bloqueia** mais que `await` (verificado: `triage` é `async fn`)
- Thread OS dedicada processa requests com latência p99 < 50ms

---

## TASK-05 — Validação DoD Global (ETAPA 5)

**Escopo:** Provar que o silício assimilou o saneamento.

- [ ] `cd src-tauri && cargo check --workspace` → Exit Code 0
- [ ] `cd src-tauri && cargo clippy --workspace --all-targets -- -D warnings` → Exit Code 0
- [ ] `cd src-tauri && cargo test --workspace` → Exit Code 0, **278+ testes verdes** (268 originais + 10 novos)
- [ ] Se falhar por lifetime/ownership: invocar `souls-ralph-loop` (3-tentativas ceiling, Fail-Closed)
- [ ] Se falhar por feature gating: ajustar `#[cfg(feature = "...")]` para stubs sem feature

**DoD:**
- `cargo check` retorna `Exit Code 0` com zero warnings
- `cargo clippy -D warnings` retorna `Exit Code 0`
- `cargo test` retorna `Exit Code 0` com 268+ testes originais + 10 novos = 278+ verdes

---

## TASK-06 — Blast Radius Report + HITL

**Escopo:** Compilar diff stats e enviar para aprovação humana.

- [ ] `git diff --stat` capturado
- [ ] Mensagem HITL gerada com: branch, número de arquivos novos/editados, lista de paths
- [ ] NÃO fazer merge
- [ ] Aguardar aprovação do Arquiteto para rebase semântico
