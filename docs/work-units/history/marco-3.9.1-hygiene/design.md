# Design — Marco 3.9.1: Faxina de Higiene + Detecção de Backpressure (Zero-Slop)

> Emenda ao [design-marco-3.9-e2-hardening.md](design-marco-3.9-e2-hardening.md).
> Referência normativa: auditoria P1-P4 do Arquiteto-Chefe (2026-08-02).

## 1. Contexto e Motivação

A auditoria P1-P4 pós-PR #22 identificou **4 pontos de atrito** no chassi:

| # | Severidade | Tipo | Local |
|---|-----------|------|-------|
| P1 | ⚠️ Baixa | Dead code literal | [bin/souls_mcp_server.rs:2958-3017](file:///z:/souls_mc/src-tauri/src/bin/souls_mcp_server.rs#L2958-L3017) |
| P2 | ⚠️ Baixa | Duplicação de helpers | [bin/souls_mcp_server.rs:1690-1813](file:///z:/souls_mc/src-tauri/src/bin/souls_mcp_server.rs#L1690-L1813) |
| P3 | ⚡ Média (potencial) | `MutexGuard` mantido em `await` (warning `clippy::await_holding_lock`) | [bin/souls_mcp_server.rs:2980](file:///z:/souls_mc/src-tauri/src/bin/souls_mcp_server.rs#L2980) |
| P4 | ⚠️ Baixa | Telemetria de backpressure adormecida | [socratic_bridge.rs:129-131](file:///z:/souls_mc/src-tauri/src/cognition/thinking/socratic_bridge.rs#L129-L131) |

**Pessimismo da Razão:** Cada warning de compilação é uma bomba-relógio. Em algum
refator futuro, alguém vai tentar usar o código morto, ou o `await_holding_lock`
vai explodir quando alguém mover a chamada para um contexto async. Devemos
**extirpar** o código morto e **ativar** os disjuntores adormecidos.

## 2. Topologia FinOps — Faxina Cirúrgica

```mermaid
graph LR
    subgraph Antes["Estado Pré-Faxina (PR #22)"]
        A1[init_state_db_for_testing]
        A2[TEST_STATE_DB_TX_OVERRIDE]
        A3[open_socratic_state_db bin]
        A4[build_socratic_tree bin]
        A5[render_socratic_markdown bin]
        A6[is_under_backpressure adormecido]
        A7[warning: await_holding_lock]
        A8[warning: function never used]
    end

    subgraph Depois["Estado Pós-Faxina (Marco 3.9.1)"]
        D1[Handlers canônicos em cognition::thinking::handlers]
        D2[Helpers socráticos movidos para módulo test_helpers sob #[cfg(test)]]
        D3[is_under_backpressure instrumentado em run_souls_merge_sessions]
        D4[telemetry_logs: tool=socratic_backpressure_active]
        D5[Zero warnings no código novo]
    end

    A1 -.extirp.-> NULO1((🪦))
    A2 -.extirp.-> NULO2((🪦))
    A3 -.refactor.-> D2
    A4 -.refactor.-> D2
    A5 -.refactor.-> D2
    A6 -.wire.-> D3
    A3 -->|antes| D1
    A4 -->|antes| D1
    A5 -->|antes| D1
    A7 -.auto-cura.-> D5
    A8 -.auto-cura.-> D5
```

## 3. Padrão Orchestrator-Worker (Mantido)

A arquitetura do Marco 3.9 Fase E.2 é **preservada**:
- `SocraticWriteWorker` continua consumindo MPSC bounded(512) + micro-batching.
- `socratic_handle()` continua sendo o `Option<SocraticWriteHandle>` para o critical path.

A faxina apenas:
1. **Remove** o que está morto (P1).
2. **Centraliza** o que estava duplicado (P2) em `cognition::thinking::test_helpers`.
3. **Ativa** o disjuntor adormecido (P4) via instrumentação pós-despacho.

## 4. Decisões Canônicas (Lei Zero do Trator Mecânico)

### D1 — Extinção total de P1
- **Deleta:** `init_state_db_for_testing` (60 linhas).
- **Deleta:** `TEST_STATE_DB_TX_OVERRIDE` (4 linhas + a static).
- **Justificativa:** Função nunca chamada em qualquer teste. É literalmente
  código morto que introduz risco (assinatura pode ter mudado, dialeto de
  migração pode ter ficado obsoleto). Conformidade com **ADR-005 §Zero-Slop**.
- **Efeito cascata:** `cargo clippy` para de reportar
  `clippy::await_holding_lock` (P3) e `dead_code` (P1).

### D2 — Centralização dos helpers socráticos (P2)
- **Cria:** `cognition::thinking::test_helpers` (submódulo sob `#[cfg(test)]`).
- **Move:** `build_socratic_tree`, `render_socratic_markdown` para o novo
  submódulo (visíveis apenas em testes).
- **Mantém:** `open_socratic_state_db` no bin (usado por T-bootstrap, e o
  pattern `RpcError` é específico do bin).
- **Justificativa:** Single source of truth para reconstrução de árvore.
  Impede drift de formato entre bin e lib se o schema V5 evoluir.

### D3 — Instrumentação ativa de backpressure (P4)
- **Cria:** `try_log_socratic_backpressure()` no bin.
  - Lê `socratic_handle().is_under_backpressure()`.
  - Se `true`: chama `try_log_telemetry("socratic_backpressure_active", 0, 0, 0.0, 0, 0.0)`.
  - Se `false`: chama `try_log_telemetry("socratic_backpressure_inactive", 0, 0, 0.0, 0, 1.0)` (cardinalidade para o Prometheus).
- **Chama:** A partir de `run_souls_merge_sessions` (único path de escrita socrática
  que usa o worker). Custo: 1 read atômico + 1 try_send (best-effort).
- **Justificativa:** O disjuntor foi projetado (Marco 3.9 Fase E.2) mas nunca
  teve observability. Sem essa métrica, **ninguém saberá** se o barramento está
  saturado até o próximo log de erro de I/O.

### D4 — Cura `tools/list` (P5)
- **Verifica:** Registry via os 2 testes existentes:
  - `tools_list_fill_unico_sem_duplicata` (L5540)
  - `server_info_name_is_souls_mcp` (L5553)
  - `test_tools_list_includes_headroom_retrieve` (L4787)
- **Confirma:** `souls_stub_fill` aparece apenas como alias de back-compat no
  dispatcher (L778), **NUNCA** no `tools/list`. A duplicata já está exterminada.
- **Ação:** Nenhuma mudança no registry (está limpo). Documenta o status no
  log do commit.

## 5. Agnosticismo de Hardware (Mantido)

- **Piso:** RTX 2060m (Windows ReFS, 16GB RAM) — não muda.
- **Teto:** Apple Silicon M-series / Linux NUMA — o pattern `try_log_telemetry`
  + `is_under_backpressure` é puramente `std::sync::atomic` + `mpsc`, então
  é 100% agnóstico.

## 6. Hot Paths e Custos

| Operação | Custo | Observação |
|----------|-------|------------|
| Extinção P1 (compile-time) | -57 linhas, -0 bytes runtime | Code shrink, 0 overhead |
| D2 (mover para test_helpers) | 0 bytes runtime, mais legibilidade | 0 overhead produção |
| `try_log_socratic_backpressure` (best-effort) | ~2µs/read + try_send | Sumido no I/O de merge |
| 1 métrica/telemetria/merge | ~50 bytes/registro em `telemetry_logs` | FinOps-safe |

## 7. Lei do Scaffold (DoD Pré-Codificação)

- [x] `init_state_db_for_testing` deletado + `grep -r init_state_db_for_testing src/` = 0.
- [x] `TEST_STATE_DB_TX_OVERRIDE` deletado + `grep -r TEST_STATE_DB_TX_OVERRIDE src/` = 0.
- [x] `cognition::thinking::test_helpers` criado com 2 funções movidas.
- [x] `try_log_socratic_backpressure` implementado e chamado em `run_souls_merge_sessions`.
- [x] `cargo test --bin souls_mcp_server --no-default-features` 41/41 verde.
- [x] `cargo clippy --bin souls_mcp_server --no-default-features --tests` sem warnings
      relativos ao código novo.

## 8. Riscos e Mitigações

| Risco | Mitigação |
|-------|-----------|
| P2 move quebra T2 (que usa `build_socratic_tree` e `render_socratic_markdown`) | Submódulo `#[cfg(test)]` mantém as funções visíveis aos testes |
| P4 vaza canais em produção (try_send excessivo) | `try_send` é best-effort, bounded(100), e a métrica é pequena (50 bytes) |
| T-bootstrap falha sem `open_socratic_state_db` no bin | Mantido (não movido para lib) — usado apenas pelo T-bootstrap |
| `tools/list` ainda tem resíduo | 2 testes existentes validam a cura |

## 9. Compatibilidade com a Lei 32/120 (ADR-041)

Nenhuma tool nova é introduzida nesta faxina. Compatibilidade integral.

## 10. Invariantes de Compile-Time (Blindagem)

```rust
// D2: helpers de teste ficam isolados, garantindo que produção não os importe.
#[cfg(test)]
mod test_helpers {
    pub fn build_socratic_tree<'a>(...) -> ...
    pub fn render_socratic_markdown(...) -> String
}
```

Nenhuma nova invariante necessária — o `#[cfg(test)]` é a blindagem canônica.
