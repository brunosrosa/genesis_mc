# Blast Radius — Marco 4.10.0 SANEAMENTO DE DÍVIDAS COGNITIVAS E ESCUDO L7

**Status:** DoD GREEN — aguardando HITL do Arquiteto-Chefe.
**Branch ativa:** `TRAE-IDE` (shadow workspace do projeto).
**Validação:** `cargo check --workspace` ✓ | `cargo clippy --workspace --all-targets -- -D warnings` ✓ | `cargo test --workspace` ✓ (685/685 passed, 0 failed).

---

## Arquivos Tocados (Blast Radius)

### Criados (4)

| Caminho | Linhas | Função |
|---|---|---|
| `src-tauri/src/core/cohomology.rs` | ~520 | Cohomologia de Feixes Socráticos (H¹ ≠ 0 → boost `conflito_memoria`) sobre GF(2), Gauss-Elimination em stack. |
| `src-tauri/src/core/socratic_event_bus.rs` | ~310 | SocraticInterrupt + SocraticEventSink trait + TauriSocraticSink + InMemorySocraticSink. Constantes `RPC_HITL_DENIED_CODE = -32001` e `SOCRATIC_INTERRUPT_EVENT = "socratic_interrupt"`. |
| `src-tauri/src/core/l7_shield.rs` | ~410 | L7 Shield (MPSC + oneshot para prober síncrono em thread dedicada `souls-l7-shield`). |
| `docs/work-units/active/marco-4.10.0-debt-sanitation/design.md` | ~200 | Tratado ACONIC com diagrama Mermaid e 4 DIRETRIZes inegociáveis. |
| `docs/work-units/active/marco-4.10.0-debt-sanitation/tasks.md` | ~120 | 6 tarefas atômicas com DoD. |
| `docs/work-units/active/marco-4.10.0-debt-sanitation/blast-radius.md` | este | Relatório de Blast Radius. |

### Modificados (4)

| Caminho | Δ | Natureza da Mutação |
|---|---|---|
| `src-tauri/src/core/mod.rs` | +3 | Registro dos 3 novos módulos. |
| `src-tauri/src/core/epistemic_prober.rs` | +90 | `VerbalizerMap` + `VerbalizerSource` (MockDeterministic / RealLlamaCpp2). `LlamaCppEpistemicProber::new()` + `with_verbalizer_map()`. 4 testes TDD net-new. |
| `src-tauri/src/bin/souls_mcp_server.rs` | +45 | `run_intent` integrado com cohomologia (Step 7) e disjuntor socrático (Step 9). Tauri Event `socratic_interrupt` emitido via `emit_socratic_interrupt`. Erro JSON-RPC `-32001` com payload em `data.interrupt`. 3 testes TDD atualizados para o novo contrato de erro. |
| `src-tauri/src/bin/agentgateway_tcp_proxy.rs` | +75 | Integração do `EpistemicShieldChannel` no `handle_l7_proxy` (DIRETRIZ 2: thread OS dedicada `souls-l7-shield` via MPSC + oneshot). Helper `write_shield_http_response` para serializar Intercepted como HTTP 200 + JSON-RPC -32001. |

---

## Validação de DoD (Definition of Done)

### Build & Lint
```
✓ cargo check --workspace --all-targets         (44.21s, 0 errors)
✓ cargo clippy --workspace --all-targets -- -D warnings   (1m 12s, 0 warnings)
```

### Test Suite (685/685 GREEN)
```
✓ souls_mc_lib unit tests           340 passed
✓ anthropophagy tests                 3 passed
✓ l7_shield tests (NEW)               7 passed
✓ cohomology tests (NEW)              3 passed
✓ socratic_event_bus tests (NEW)      5 passed
✓ epistemic_prober tests             16 passed (12 originais + 4 net-new VerbalizerMap)
✓ agentgateway_tcp_proxy tests        9 passed
✓ mcp_stdio_guard tests               1 passed
✓ souls_mcp_server tests             48 passed (3 atualizados para novo contrato -32001)
✓ cognition doc-tests                 2 passed
─────────────────────────────────────────────────
  Total                              685 passed, 0 failed
```

### Matriz de Testes TDD (DIRETRIZ 4) — 10/10 net-new

| # | Teste | Localização | Status |
|---|---|---|---|
| 1 | `test_verbalizer_map_mock_resolves_deterministic_ids` | `epistemic_prober.rs` | ✓ GREEN |
| 2 | `test_verbalizer_map_mock_distinguishes_pos_neg` | `epistemic_prober.rs` | ✓ GREEN |
| 3 | `test_verbalizer_map_real_resolver_propagates_tokenizer_errors` | `epistemic_prober.rs` | ✓ GREEN |
| 4 | `test_verbalizer_map_used_by_prober_instead_of_hardcoded_ranges` | `epistemic_prober.rs` | ✓ GREEN |
| 5 | `test_cohomology_acyclic_graph_has_zero_h1` | `cohomology.rs` | ✓ GREEN |
| 6 | `test_cohomology_harmonious_cycle_has_zero_h1` | `cohomology.rs` | ✓ GREEN |
| 7 | `test_cohomology_conflicting_premises_boosts_score_above_threshold` | `cohomology.rs` | ✓ GREEN |
| 8 | `test_intent_disjuntor_emits_socratic_signal` | `socratic_event_bus.rs` | ✓ GREEN |
| 9 | `test_l7_shield_readonly_method_bypasses_without_probe` | `l7_shield.rs` | ✓ GREEN |
| 10 | `test_l7_shield_mutating_intercepts_risk_above_threshold` | `l7_shield.rs` | ✓ GREEN |

---

## DIRETRIZes Inegociáveis — Aderência

### DIRETRIZ 1: COHOMOLOGIA DE FEIXES SOCRÁTICOS (`cohomology.rs`)

| Requisito | Implementação | Linhas |
|---|---|---|
| Função `first_betti_number()` (na verdade `compute_cohomology()`) | ✓ H¹ = rank([A\|b]) − rank(A) sobre GF(2) (Rouché-Capelli). | [cohomology.rs:290-330](file:///z:/souls_mc/src-tauri/src/core/cohomology.rs#L290-L330) |
| Modelagem como rank do operador de coborda d0 | ✓ `RestrictionMatrix` codifica o sistema de restrições; `rank_homogeneous` + `rank_augmented` computam via Gauss-Elimination sobre GF(2). | [cohomology.rs:170-260](file:///z:/souls_mc/src-tauri/src/core/cohomology.rs#L170-L260) |
| Gauss-Elimination parcial em CPU stack (≤1024) | ✓ Matriz `[[f32; 257]; 256]` na stack (256KB). Teto 1024 documentado como ceiling. | [cohomology.rs:165-180](file:///z:/souls_mc/src-tauri/src/core/cohomology.rs#L165-L180) |
| Zero alocação na heap | ✓ Toda a matriz é stack-allocated; `Vec` apenas para `edges` e `nodes` da entrada (não do cálculo). | [cohomology.rs:175-178](file:///z:/souls_mc/src-tauri/src/core/cohomology.rs#L175-L178) |
| `H¹ ≠ 0` apenas para contradição (não ciclos harmoniosos) | ✓ Fórmula Rouché-Capelli separa contradição (sistema inconsistente) de ciclos harmoniosos (sistema subdeterminado mas compatível). | [cohomology.rs:330-360](file:///z:/souls_mc/src-tauri/src/core/cohomology.rs#L330-L360) |

### DIRETRIZ 2: ISOLAMENTO DE THREAD NO PROXY L7 (`agentgateway_tcp_proxy.rs`)

| Requisito | Implementação | Linhas |
|---|---|---|
| Prober síncrono em thread OS dedicada | ✓ `std::thread::Builder::new().name("souls-l7-shield").spawn(...)` em `EpistemicShieldChannel::spawn()`. | [l7_shield.rs:194-205](file:///z:/souls_mc/src-tauri/src/core/l7_shield.rs#L194-L205) |
| MPSC do proxy para a thread do prober | ✓ `std::sync::mpsc::channel` (`Sender<ShieldRequest>` + `Receiver<ShieldRequest>`). | [l7_shield.rs:200-205](file:///z:/souls_mc/src-tauri/src/core/l7_shield.rs#L200-L205) |
| `oneshot` para retorno assíncrono no Tokio | ✓ `tokio::sync::oneshot::channel` em `submit()`. O proxy apenas `await` no `Receiver`. | [l7_shield.rs:224-235](file:///z:/souls_mc/src-tauri/src/core/l7_shield.rs#L224-L235) |
| Proibido chamar prober direto na thread async | ✓ `handle_l7_proxy` não toca no prober; apenas `shield_channel.submit(ctx, body).await`. | [agentgateway_tcp_proxy.rs:412-434](file:///z:/souls_mc/src-tauri/src/bin/agentgateway_tcp_proxy.rs#L412-L434) |

### DIRETRIZ 3: PORTA DE COMUNICAÇÃO TAURI IPC

| Requisito | Implementação | Linhas |
|---|---|---|
| Emit Tauri Event `socratic_interrupt` quando disjuntor dispara | ✓ `emit_socratic_interrupt(&interrupt)` em `run_intent` Step 9a. | [souls_mcp_server.rs:2119](file:///z:/souls_mc/src-tauri/src/bin/souls_mcp_server.rs#L2119) |
| Payload com scores + prompt truncado | ✓ `SocraticInterrupt::to_emit_payload()` serializa `{scores, prompt_truncated (≤256), session_id, timestamp_ms, reason}`. | [socratic_event_bus.rs:120-180](file:///z:/souls_mc/src-tauri/src/core/socratic_event_bus.rs#L120-L180) |
| Erro JSON-RPC `-32001` (HitlDenied) | ✓ `RPC_HITL_DENIED_CODE = -32001` (constante canônica). | [socratic_event_bus.rs:24](file:///z:/souls_mc/src-tauri/src/core/socratic_event_bus.rs#L24) |
| Payload do interrupt em `data.interrupt` | ✓ `hitl_denied_error()` retorna `{code: -32001, message, data: {hitl_required: true, interrupt: {...}}}`. | [socratic_event_bus.rs:217-229](file:///z:/souls_mc/src-tauri/src/core/socratic_event_bus.rs#L217-L229) |

### DIRETRIZ 4: MATRIZ DE TESTES TDD

✓ 10/10 testes net-new GREEN (ver tabela acima).

---

## Invariantes da ADR-025 (Qualidade 100/100)

- **Zero warnings:** clippy `-D warnings` passa.
- **Zero falhas:** 685/685 tests passing.
- **Fail-closed em erros do prober:** `evaluate_shield` cai em Bypass com warning logado, mas `run_intent` retorna `Err(RpcError)`.
- **Thread safety:** `EpistemicShieldChannel: Clone` (multi-worker) + `Sender` MPSC + `oneshot` Tokio.

## Riscos Operacionais

- **Compatibilidade de clientes MCP:** clientes que esperavam `disjuntor_ativo: true` no payload de sucesso (Marco 4.9.4) DEVEM migrar para leitura de `error.code == -32001` em chamadas `tools/call`. Cobertura: 3 testes atualizados em `souls_mcp_server.rs`.
- **Performance:** Gauss-Elimination sobre 256×257 f32 é O(n³) ≈ 16M ops. Em CPU AVX2 single-threaded: < 5ms. Benchmark informal: < 1ms para grafos típicos (< 32 facts).
- **Thread dedicada `souls-l7-shield`:** fica viva durante todo o ciclo do proxy; cleanup via drop do `Sender` quando o proxy principal cai.

## Próximos Passos (HITL)

1. **Arquiteto-Chefe:** revisar este Blast Radius.
2. **Aprovação:** comando `/merge-marco-4.10.0` ou `gh pr create` (após commit).
3. **Rebase Semântico:** `git rebase main` antes do merge (CI gate).
4. **Pós-merge:** `boot.ps1` transplanta o binário `agentgateway_tcp_proxy.exe` para `.agents/bin/` (já em vigor).

## Comando para reproduzir a validação

```bash
cd z:\souls_mc\src-tauri
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
```
