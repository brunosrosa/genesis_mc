# Tasks Specification — MARCO 5.14.0 (Drift Sentinel)

- [ ] **Task 1: Non-blocking UDP connectivity detector in `telemetry.rs`**
  - Implement `pub async fn is_internet_active() -> bool` in `src-tauri/src/telemetry.rs`
- [ ] **Task 2: StateDbOp extension & SQL Handlers in `souls_mcp_server.rs`**
  - Define `RepoDriftCandidate`
  - Add `GetRepositoriesForDriftCheck` & `UpdateRepositoryDrift` to `StateDbOp`
  - Implement SQL handlers in `StateDbWorker`
- [ ] **Task 3: Async Background Drift Sentinel Worker in `souls_mcp_server.rs`**
  - Implement `pub async fn start_reactive_drift_checker`
  - Handle rate limit headers (`GITHUB_TOKEN`, `SOULS_GITHUB_TOKEN`)
  - Execute HTTP polling loop & dispatch drift updates via MPSC
- [ ] **Task 4: TDD Integration Test Suite**
  - `test_drift_sentinel_offline_bypass`
  - `test_drift_calculation_and_state_transition`
  - `test_drift_time_cooldown_gate`
