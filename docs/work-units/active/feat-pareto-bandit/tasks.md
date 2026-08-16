# Task Breakdown — Feat: SODA Arena, Telemetria Real e Roteamento ParetoBandit

## Marcos e Definition of Done (DoD)

### Tarefa 1: SODA Arena Engine & CLI (`src-tauri/src/bin/soda_arena_cli.rs`)
- [x] Implementar o motor de profiling e stress-test de inferência local.
- [x] Invocar `EphemeralInferEngine`/`LlamaCppEngine` com medição precisa em microssegundos de TTFT e TPOT.
- [x] Gravar registros empíricos na tabela `telemetry_logs` do `souls_state.db` com timestamp UNIX.
- [x] Configurar entrada no `Cargo.toml` para o binário `soda_arena_cli`.
- **DoD**: Executável compila sem warnings e persiste dados reais na tabela SQLite.

### Tarefa 2: ParetoBandit Router Real (`src-tauri/src/finops/pareto_bandit.rs`)
- [x] Implementar a função utilitária matemática:
  $$U_t(a \mid x) = q_t(a \mid x) - \lambda_t \cdot c(a) - \beta \cdot l_t(a)$$
- [x] Implementar recuperação de métricas históricas de $q_t$ e $l_t$ do banco de telemetria SQLite.
- [x] Implementar escalonamento dinâmico de marcapasso $\lambda_t$ sob teto de 95% do orçamento ou barramento PCIe estrangulado.
- [x] Conectar ao roteador e expor feedback ao ecossistema MCP.
- **DoD**: Roteador chaveia com base em dados empíricos reais e desvia requisições quando orçamento atinge o limiar.

### Tarefa 3: Integração da Métrica de Eficiência E3 (`src-tauri/src/cognition/ast/observability/feedback.rs`)
- [x] Implementar fórmula de eficiência E3 constitucional:
  $$E3 = \frac{\text{Acurácia\_Tarefa\_Arena}}{\text{Custo\_Financeiro\_USD} + \text{Latência\_Total\_Segundos}}$$
- [x] Atualizar cálculo e agregação em tempo de execução para penalizar overthinking e tempos inflacionados.
- **DoD**: E3 degrada monotonicamente com excesso de tokens e latência, forçando desvio local.

### Tarefa 4: Suíte de Testes TDD Mandatória
- [x] Criar testes unitários em `src-tauri/src/finops/pareto_bandit.rs`:
  - `test_pareto_bandit_routing_decision_real_telemetry`
  - `test_e3_metric_calculation_penalizes_overthinking`
  - `test_bandit_lagrangian_budget_pacing`
- [x] Validar compilação e execução limpa com `cargo test --bin souls_mcp_server` e `cargo clippy`.
- [x] Gravar logs de clippy em `.souls_scratchpad/logs/cargo/clippy_pareto_bandit.log`.
- **DoD**: 100% dos testes passando (Exit Code 0) e 0 warnings no Clippy.
