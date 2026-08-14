---
spec: marco-iv-watchdog-thermal-vram
phase: 3-tasks
design: docs/work-units/active/marco-iv-watchdog-thermal/design.md
branch: feat/marco-iv-watchdog-thermal
---

# Tasks — MARCO IV (Watchdog Térmico + VRAM Scheduler + LoRA Hot-Swap)

Cada task tem DoD executável. `[SCAFFOLD]` exige teste de falha antes da lógica real (Lei do Scaffold).

## TASK-01 [SCAFFOLD] — `hardware_watchdog.rs` stub + teste vermelho

**Arquivo:** `src-tauri/src/core/hardware_watchdog.rs` (NOVO)

- [ ] Struct `pub struct HardwareWatchdog` (campos privados `running: Arc<AtomicBool>`, `handle: Option<JoinHandle>`)
- [ ] `pub fn new() -> Self` constrói mas **não inicia**
- [ ] `pub fn start(&mut self) -> &'static Arc<AtomicU64>` inicia thread nativa
- [ ] `static WATCHDOG_STATE: OnceLock<Arc<AtomicU64>>` global
- [ ] `pub fn get_state() -> Option<Arc<AtomicU64>>` (acesso lock-free)
- [ ] Constantes de bit-mask: `MASK_VRAM`, `MASK_RAM`, `MASK_CPU_TEMP`, `MASK_GPU_TEMP`, `MASK_FLAGS`
- [ ] Funções puras: `pack_state(vram_mb, ram_mb, cpu_temp, gpu_temp, flags) -> u64`
- [ ] Funções puras: `decode_vram_mb`, `decode_ram_mb`, `decode_cpu_temp_c`, `decode_gpu_temp_c`, `decode_thermal_flag`
- [ ] **Teste vermelho**: `test_watchdog_state_bit_pack_roundtrip` falha até implementação correta

**DoD:**
- `cargo check` Exit 0
- `test_watchdog_state_bit_pack_roundtrip` passa

## TASK-02 — `hardware_watchdog.rs` thread nativa S.O. com sysinfo

- [ ] `std::thread::Builder::new().name("souls-hardware-watchdog")` + `spawn`
- [ ] Loop com `std::thread::sleep(Duration::from_millis(1000))`
- [ ] `sysinfo::System::new_all()` + `refresh_all()`
- [ ] Lê `used_memory()` (RAM), `global_cpu_info().cpu_usage()` (opcional)
- [ ] Lê VRAM via NVML guard-gated com `#[cfg(feature = "llama_backend")]`
- [ ] Lê temperatura CPU via `components()` e dGPU via NVML
- [ ] Pack + `WATCHDOG_STATE.store(packed, Ordering::Release)` a cada 1s
- [ ] **Teste**: `test_watchdog_telemetry_polling` valida < 5ms para pack + store

**DoD:**
- `cargo check` Exit 0
- `test_watchdog_telemetry_polling` passa (< 5ms)

## TASK-03 [SCAFFOLD] — `vram_scheduler.rs` extension stub: `KvCacheSwapController`

**Arquivo:** `src-tauri/src/core/vram_scheduler.rs` (EDIT — preservar LRU existente)

- [ ] Trait `pub trait VramPressureSink: Send + Sync` com `on_vram_pressure(&self, pct: f32) -> VramAction`
- [ ] Enum `pub enum VramAction { SwapOut, SwapIn, Hold }`
- [ ] Struct `pub struct KvCacheSwapController` com:
  - `threshold_high_pct: f32` (default 90.0)
  - `threshold_low_pct: f32` (default 80.0)
  - `consecutive_samples: AtomicU32`
  - `current_state: AtomicU8` (0=Hold, 1=SwappedOut, 2=SwappedIn)
- [ ] `pub fn new() -> Self`
- [ ] `pub fn evaluate(&self, vram_pct: f32) -> VramAction` com lógica anti-flap (2 amostras consecutivas)
- [ ] **Teste vermelho**: `test_kv_swap_controller_hysteresis` (high=92%, low=79%, exige 2 amostras)

**DoD:**
- `cargo check` Exit 0
- LRU existente intacto

## TASK-04 — `vram_scheduler.rs` swap-out / swap-in com guarda assíncrona

- [ ] `pub async fn swap_out_kv_cache_q4k(&self) -> Result<(), String>` — simula DMA para Host RAM
- [ ] `pub async fn swap_in_kv_cache_q4k(&self) -> Result<(), String>` — simula DMA de retorno para VRAM
- [ ] `pub fn is_swapped_out(&self) -> bool` (lê `current_state` atômico)
- [ ] `tokio::task::spawn_blocking` para isolar syscalls bloqueantes
- [ ] **Teste**: `test_vram_scheduler_eviction_trigger` simula 92% e valida SwapOut após 2 amostras

**DoD:**
- `cargo check` Exit 0
- `test_vram_scheduler_eviction_trigger` passa

## TASK-05 [SCAFFOLD] — `llama_lora_adapter.rs` stub gated em `llama_backend`

**Arquivo:** `src-tauri/src/core/llama_lora_adapter.rs` (NOVO, gated)

- [ ] `#[cfg(feature = "llama_backend")]` guard
- [ ] Enum `pub enum LoraSpecialty { Coder, Socratic, Heuristic }`
- [ ] Struct `pub struct LlamaLoraAdapter`:
  - `registered: DashMap<LoraSpecialty, PathBuf>` (paths pré-registrados)
  - `applied: Mutex<Option<LoraSpecialty>>` (apenas 1 ativo por contexto)
  - `last_swap_ns: AtomicU64` (telemetria)
- [ ] `pub fn pre_register(&self, specialty: LoraSpecialty, path: PathBuf)` — Host RAM inert
- [ ] `pub fn apply_lora_adapter_in_flight(&self, ctx_ptr: *mut c_void, specialty: LoraSpecialty, scale: f32) -> Result<(), LoraError>` (stub FFI)
- [ ] **Teste vermelho**: `test_lora_hot_swap_under_5ms` falha até implementação correta

**DoD:**
- `cargo check --features llama_backend` Exit 0
- Stub compila limpo (sem chamar FFI real, mas com `unsafe extern "C"` declarada)

## TASK-06 — `llama_lora_adapter.rs` hot-swap FFI com ik_llama.cpp hooks

- [ ] `unsafe extern "C" { fn souls_ik_llama_lora_apply(ctx: *mut c_void, path: *const c_char, scale: f32) -> i32; }`
- [ ] `#[link(name = "ik_llama", kind = "dylib")]` (best-effort, fail-soft)
- [ ] `apply_lora_adapter_in_flight` chama FFI sob `unsafe`, valida `applied != target`, faz swap atômico
- [ ] Mede latência com `Instant::now()`, atualiza `last_swap_ns`
- [ ] `pub fn release_previous(&self) -> Result<(), LoraError>` — descarrega adaptador anterior
- [ ] **Teste**: `test_lora_hot_swap_performance` valida < 5ms end-to-end (sem FFI real, via `mock_apply_fn` injetável)

**DoD:**
- `cargo check --features llama_backend` Exit 0
- `test_lora_hot_swap_performance` passa (< 5ms)

## TASK-07 — `core/mod.rs` exposição dos novos módulos

**Arquivo:** `src-tauri/src/core/mod.rs` (EDIT)

- [ ] `pub mod hardware_watchdog;` (sempre disponível)
- [ ] `#[cfg(feature = "llama_backend")] pub mod llama_lora_adapter;`

**DoD:**
- `cargo check` Exit 0
- Nenhum warning de "unused"

## TASK-08 — Testes TDD em `tests/vram_scheduler_tests.rs` (NOVO)

**Arquivo:** `src-tauri/tests/vram_scheduler_tests.rs`

- [ ] `test_watchdog_telemetry_polling` (TASK-02)
- [ ] `test_vram_scheduler_eviction_trigger` (TASK-04)
- [ ] `test_lora_hot_swap_performance` (TASK-06)

**DoD:**
- `cargo test --test vram_scheduler_tests` Exit 0 (3/3 contratos)

## TASK-09 — Validação final: `cargo clippy`

**Comando:**
```bash
cd src-tauri && cargo clippy --features "tauri-app,gateway_ccr,llama_backend" -- -D warnings
```

- [ ] Se Exit 0: marco concluído.
- [ ] Se falhar pelo issue CUDA pré-existente do `llama-cpp-2 v0.1.154` (documentado em memory): usar workaround `--no-default-features --features "tauri-app,gateway_ccr"` e reportar o desvio ao Arquiteto.

**DoD:**
- `cargo clippy` retorna Exit 0 (com ou sem workaround documentado)

## TASK-10 — Blast Radius + HITL

- [ ] `git diff --stat` capturado
- [ ] Mensagem HITL com: branch, 3 arquivos novos, 1 arquivo editado, 1 test novo
- [ ] NÃO fazer merge
- [ ] Aguardar aprovação do Arquiteto
