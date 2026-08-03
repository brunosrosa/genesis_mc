# Tasks — Disjuntores BFS no `repo_radar` (Resgate blob_03)

> Branch: `fix/blob03-bfs-circuit-breaker`
> Pai do design: [docs/fixes/blob03-bfs-circuit-breaker/design.md](file:///Z:/souls_mc/docs/fixes/blob03-bfs-circuit-breaker/design.md)
> DoD global: `cargo check` exit 0 + `cargo test repo_radar::tests` exit 0 + 0 regressão nos 5 callers.

---

## T1 — TDD Red: Teste de regressão do Circuit Breaker
**Arquivo:** [src-tauri/src/harvester/repo_radar.rs](file:///Z:/souls_mc/src-tauri/src/harvester/repo_radar.rs) (apêndice de `mod tests` ao final)
**DoD:** ✓ 3 funções de teste presentes (`test_scan_repo_radar_circuit_breaker_caps_files`, `test_scan_repo_radar_respects_max_depth`, `test_scan_repo_radar_does_not_follow_symlinks` com `#[cfg(unix)]`).

## T2 — TDD Green: Injeção dos 3 disjuntores em `scan_repo_radar`
**Arquivo:** [src-tauri/src/harvester/repo_radar.rs:62-145](file:///Z:/souls_mc/src-tauri/src/harvester/repo_radar.rs#L62-L145)
**DoD:** ✓ `MAX_RADAR_FILES: usize = 5_000` no topo. ✓ `builder.max_depth(Some(4))` + `builder.follow_links(false)`. ✓ Contador `visited` + `if visited > MAX_RADAR_FILES { break; }`. ✓ `warn!` Fail-Soft.

## T3 — Validação Bare-Metal
**Comandos executados:**
```powershell
cd src-tauri
cargo check -p souls_mc --lib
cargo test -p souls_mc --lib repo_radar::tests -- --nocapture
```
**Resultado:**
- `cargo check` → **exit 0** ✓
- `cargo test repo_radar::tests` → **2 passed; 0 failed** ✓ (3º teste é `#[cfg(unix)]`)

## T4 — Relatório HITL
**Entrega:** este diretório + diff do `repo_radar.rs` na Agent Inbox. **Sem merge, sem commit.** Aguardando aprovação biométrica/tátil do Arquiteto.

---

## Notas de Risco
- `follow_links(false)` é mudança de comportamento para os 5 callers. Nenhum depende de symlinks.
- `max_depth(Some(4))` pode cortar árvores legítimas em monorepos muito profundos. Disjuntor de 5.000 absorve o pior caso sem perda total.
- Circuit breaker é decisão consciente de retornar dados parciais (Fail-Soft).
