# Tasks — MARCO 5.7.0

- [x] Task 1: Migração e DDL de Persistência SQLite (`souls_state.db`)
  - Adicionar tabelas `souls_memory_nodes` e `souls_raw_events_l0` com modo `STRICT`.
  - Garantir execução idempotente durante a inicialização do SODA.

- [x] Task 2: Algoritmo de Poda de Langevin na Bola de Poincaré (`langevin_decay.rs`)
  - Implementar cálculo de Poincaré Gradient Descent ($x_{t+1} = \text{proj\_poincare}(x_t - \eta \nabla V(x_t) + \sqrt{2D\Delta t}\xi_t)$).
  - Atualizar coordenadas de nós `EVOLVING` e marcar como `SUPERSEDED` nós com $\|x\| \ge 0.95$.

- [x] Task 3: Loop de Monitoramento de Ociosidade e Daemon (`chyros_daemon.rs`)
  - Implementar `ActivityTracker` e struct `ChyrosDaemon`.
  - Configurar disparo automático do ciclo `run_consolidation_cycle` após limiar de ociosidade.
  - Implementar freio/interrupção imediata da consolidação no momento de atividade do usuário (<100ms).

- [x] Task 4: Consolidação Cognitiva Episódica na CPU (Gemma E2B / Tier 0.5)
  - Processar eventos não processados `processed = 0` da fila `souls_raw_events_l0`.
  - Confrontar fatos L0 com premissas existentes usando `LlamaCpp4LogitEngine` na CPU (AVX2).
  - Resolver contradições, gravar tombstones `SUPERSEDED` e compilar a Visão Materializada de Memória (MMV) alinhada em múltiplos de 64 tokens.

- [x] Task 5: Suíte TDD e Validação Completa (DoD GREEN)
  - Implementar `test_chyros_daemon_idle_trigger`.
  - Implementar `test_langevin_decay_convergence`.
  - Implementar `test_jit_factual_consolidation`.
  - Implementar `test_mmv_prefix_cache_rate`.
  - Garantir `cargo check` e `cargo clippy -- -D warnings` 100% limpos.
