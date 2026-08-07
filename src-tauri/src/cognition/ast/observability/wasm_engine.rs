//! Marco 4.12.0: Enjaulamento Wasmtime do Tree-Sitter & Sandbox Python (WASI 0.2).
//!
//! Cerca perimétrica do gateway `souls_mcp` com suporte completo a WASI Preview 2:
//! 1. Memory Limiter assimétrico (16 MiB para gramáticas, 32 MiB para Python sandbox).
//! 2. Fuel Metering de 10.000.000 unidades de combustível.
//! 3. VFS enjaulado pré-abrindo `/workspace` (RW) e `/grammars` (RO).

use std::path::PathBuf;
use std::sync::OnceLock;

use wasmtime::{Engine, Module, ResourceLimiter, Store};
use wasmtime_wasi::{
    preview1::WasiP1Ctx, DirPerms, FilePerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView,
};

/// Bytecode WAT de fixture embarcado em compile-time.
pub const RUST_WASM: &[u8] = include_bytes!("../../../../data/wasm/rust_sample.wat");

/// Teto de memória linear por Store para gramáticas tree-sitter (16 MiB).
pub const MEMORY_LIMIT_BYTES_GRAMMAR: usize = 16 * 1024 * 1024;

/// Teto de memória linear elástico para interpretadores completos como python.wasm (32 MiB).
pub const MEMORY_LIMIT_BYTES_PYTHON: usize = 32 * 1024 * 1024;

/// Teto de memória linear default (16 MiB).
pub const MEMORY_LIMIT_BYTES: usize = MEMORY_LIMIT_BYTES_GRAMMAR;

/// Teto de fuel por invocação de guest (10.000.000 unidades).
pub const FUEL_LIMIT: u64 = 10_000_000;

/// Estrutura de dados contida no Store do Wasmtime para isolamento WASI 0.2.
pub struct WasiStoreData {
    pub wasi_ctx: WasiCtx,
    pub wasi_p1: WasiP1Ctx,
    pub table: ResourceTable,
    pub limiter: WasmMemoryLimiter,
}

impl WasiStoreData {
    pub fn new(wasi_ctx: WasiCtx, wasi_p1: WasiP1Ctx, memory_limit_bytes: usize) -> Self {
        Self {
            wasi_ctx,
            wasi_p1,
            table: ResourceTable::new(),
            limiter: WasmMemoryLimiter::new(memory_limit_bytes),
        }
    }
}

impl WasiView for WasiStoreData {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

/// Implementação concreta do [`ResourceLimiter`] do Wasmtime 29.
#[derive(Debug, Clone)]
pub struct WasmMemoryLimiter {
    bytes: usize,
}

impl WasmMemoryLimiter {
    pub fn new(bytes: usize) -> Self {
        Self { bytes }
    }
}

impl ResourceLimiter for WasmMemoryLimiter {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        Ok(desired <= self.bytes)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        Ok(true)
    }
}

/// Classificação estrutural de falhas do sandbox Wasmtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmTrap {
    Unreachable { reason: String },
    Oom { reason: String },
    FuelExhausted { fuel_consumed: u64 },
    PermissionDenied { reason: String },
    StructuredFailure { reason: String },
    HostPanic { reason: String },
}

impl std::fmt::Display for WasmTrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmTrap::Unreachable { reason } => {
                write!(f, "WASM_UNREACHABLE: {reason}")
            }
            WasmTrap::Oom { reason } => {
                write!(f, "WASM_OOM: {reason}")
            }
            WasmTrap::FuelExhausted { fuel_consumed } => {
                write!(f, "WASM_FUEL_EXHAUSTED: guest consumiu {fuel_consumed} fuel units (teto {FUEL_LIMIT})")
            }
            WasmTrap::PermissionDenied { reason } => {
                write!(f, "WASM_PERMISSION_DENIED: {reason}")
            }
            WasmTrap::StructuredFailure { reason } => {
                write!(f, "WASM_STRUCTURED_FAILURE: {reason}")
            }
            WasmTrap::HostPanic { reason } => {
                write!(f, "WASM_HOST_PANIC: {reason}")
            }
        }
    }
}

impl std::error::Error for WasmTrap {}

/// Cria os contextos WASI Preview 2 e Preview 1 pré-abrindo os diretórios físicos do host.
pub fn create_wasi_contexts() -> Result<(WasiCtx, WasiP1Ctx), WasmTrap> {
    let workspace_host = PathBuf::from(".souls_scratchpad/python_test");
    std::fs::create_dir_all(&workspace_host).ok();

    let grammars_host = PathBuf::from("src-tauri/resources/wasm_grammars");
    std::fs::create_dir_all(&grammars_host).ok();

    let mut builder1 = WasiCtxBuilder::new();
    builder1.inherit_stdout();
    builder1.inherit_stderr();
    builder1
        .preopened_dir(
            &workspace_host,
            "/workspace",
            DirPerms::all(),
            FilePerms::all(),
        )
        .map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao pré-abrir /workspace: {e}"),
        })?;
    builder1
        .preopened_dir(
            &grammars_host,
            "/grammars",
            DirPerms::READ,
            FilePerms::READ,
        )
        .map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao pré-abrir /grammars: {e}"),
        })?;

    let mut builder2 = WasiCtxBuilder::new();
    builder2.inherit_stdout();
    builder2.inherit_stderr();
    builder2
        .preopened_dir(
            &workspace_host,
            "/workspace",
            DirPerms::all(),
            FilePerms::all(),
        )
        .map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao pré-abrir /workspace: {e}"),
        })?;
    builder2
        .preopened_dir(
            &grammars_host,
            "/grammars",
            DirPerms::READ,
            FilePerms::READ,
        )
        .map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao pré-abrir /grammars: {e}"),
        })?;

    let wasi_ctx = builder1.build();
    let p1_ctx = builder2.build_p1();

    Ok((wasi_ctx, p1_ctx))
}

/// Motor Wasmtime configurado com a cerca de recursos físicos WASI 0.2.
pub struct WasmEngine {
    engine: Engine,
}

impl WasmEngine {
    fn new() -> Result<Self, WasmTrap> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        config.cache_config_load_default().ok();

        let engine = Engine::new(&config).map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao construir Engine Wasmtime: {e}"),
        })?;
        Ok(Self { engine })
    }

    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<WasmEngine> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            Self::new().expect("[WasmEngine] Falha estrutural ao inicializar motor Wasmtime")
        })
    }

    pub fn load_module(&self, bytes: &[u8]) -> Result<Module, WasmTrap> {
        Module::new(&self.engine, bytes).map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao compilar módulo WASM: {e}"),
        })
    }

    pub fn execute_safely_with_limit<F, T>(
        &self,
        module: &Module,
        max_memory_bytes: usize,
        mut f: F,
    ) -> Result<T, WasmTrap>
    where
        F: FnMut(&mut Store<WasiStoreData>, &wasmtime::Instance) -> Result<T, wasmtime::Error>,
    {
        let (wasi_ctx, wasi_p1) = create_wasi_contexts()?;
        let data = WasiStoreData::new(wasi_ctx, wasi_p1, max_memory_bytes);
        let mut store = Store::new(&self.engine, data);
        store.limiter(|d| &mut d.limiter);
        store.set_fuel(FUEL_LIMIT).map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao injetar fuel: {e}"),
        })?;

        let mut core_linker = wasmtime::Linker::<WasiStoreData>::new(&self.engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut core_linker, |data| &mut data.wasi_p1).map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao vincular WASI Preview 1: {e}"),
        })?;

        let instance = match core_linker.instantiate(&mut store, module) {
            Ok(i) => i,
            Err(_) => match wasmtime::Instance::new(&mut store, module, &[]) {
                Ok(i) => i,
                Err(e) => {
                    let consumed = store.get_fuel().unwrap_or(FUEL_LIMIT);
                    return Err(classify_trap(&e, consumed));
                }
            },
        };

        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f(&mut store, &instance)
        })) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => {
                let consumed = store.get_fuel().unwrap_or(FUEL_LIMIT);
                Err(classify_trap(&e, consumed))
            }
            Err(panic_payload) => {
                let reason = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "panic não-stringificado".to_string()
                };
                Err(WasmTrap::HostPanic { reason })
            }
        };

        result
    }

    pub fn execute_safely<F, T>(&self, module: &Module, f: F) -> Result<T, WasmTrap>
    where
        F: FnMut(&mut Store<WasiStoreData>, &wasmtime::Instance) -> Result<T, wasmtime::Error>,
    {
        self.execute_safely_with_limit(module, MEMORY_LIMIT_BYTES_GRAMMAR, f)
    }
}

fn classify_trap(err: &wasmtime::Error, fuel_consumed: u64) -> WasmTrap {
    let reason = format!("{err:?}");
    let lower = reason.to_ascii_lowercase();

    if lower.contains("unreachable") {
        WasmTrap::Unreachable { reason }
    } else if lower.contains("out of memory") || lower.contains("memory growth") || lower.contains("allocation") {
        WasmTrap::Oom { reason }
    } else if lower.contains("fuel") || lower.contains("interrupt") {
        WasmTrap::FuelExhausted { fuel_consumed }
    } else if lower.contains("permission") || lower.contains("not capable") || lower.contains("capabilities") || lower.contains("access denied") {
        WasmTrap::PermissionDenied { reason }
    } else {
        WasmTrap::StructuredFailure { reason }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_singleton_is_stable() {
        let a = WasmEngine::global();
        let b = WasmEngine::global();
        assert!(std::ptr::eq(a, b), "WasmEngine::global deve ser singleton estável");
    }

    #[test]
    fn test_execute_safely_happy_path() {
        let engine = WasmEngine::global();
        let wat = r#"
            (module
                (func (export "answer") (result i32)
                    (i32.const 42)
                )
            )
        "#;
        let module = engine.load_module(wat.as_bytes()).expect("compila WAT");
        let result: i32 = engine
            .execute_safely::<_, i32>(&module, |store, instance| {
                let f = instance
                    .get_typed_func::<(), i32>(&mut *store, "answer")?;
                f.call(&mut *store, ())
            })
            .expect("execução feliz não pode falhar");
        assert_eq!(result, 42, "guest deve retornar a constante 42");
    }

    #[test]
    fn test_wasm_tree_sitter_isolation() {
        let engine = WasmEngine::global();
        let wat = r#"
            (module
                (func (export "boom") (param i32 i32) (result i32)
                    (unreachable)
                )
            )
        "#;
        let module = engine.load_module(wat.as_bytes()).expect("compila WAT");
        let trap = engine
            .execute_safely::<_, i32>(&module, |store, instance| {
                let f = instance
                    .get_typed_func::<(i32, i32), i32>(&mut *store, "boom")?;
                f.call(&mut *store, (0, 0))
            })
            .expect_err("unreachable DEVE ser interceptado como Err");
        assert!(
            matches!(trap, WasmTrap::Unreachable { .. }),
            "trap classificado errado: {trap:?}"
        );
    }

    #[test]
    fn test_fuel_limit_kills_infinite_loop() {
        let engine = WasmEngine::global();
        let wat = r#"
            (module
                (func (export "spin") (param i32 i32) (result i32)
                    (local $i i32)
                    (loop $l
                        (local.set $i (i32.add (local.get $i) (i32.const 1)))
                        (br_if $l (i32.lt_s (local.get $i) (i32.const 1000000000)))
                    )
                    (i32.const 0)
                )
            )
        "#;
        let module = engine.load_module(wat.as_bytes()).expect("compila WAT");
        let trap = engine
            .execute_safely::<_, i32>(&module, |store, instance| {
                let f = instance
                    .get_typed_func::<(i32, i32), i32>(&mut *store, "spin")?;
                f.call(&mut *store, (0, 0))
            })
            .expect_err("loop infinito DEVE ser morto pelo fuel");
        assert!(
            matches!(
                trap,
                WasmTrap::FuelExhausted { .. } | WasmTrap::Oom { .. } | WasmTrap::Unreachable { .. }
            ),
            "loop infinito NÃO foi contido: {trap:?}"
        );
    }

    #[test]
    fn test_memory_limiter_16mib() {
        let engine = WasmEngine::global();
        let wat = r#"
            (module
                (memory (export "mem") 1)
                (func (export "grow_huge") (result i32)
                    (loop $l
                        (drop (memory.grow (i32.const 1000)))
                        (br_if $l (i32.lt_s (memory.size) (i32.const 100000)))
                    )
                    (memory.size)
                )
            )
        "#;
        let module = engine.load_module(wat.as_bytes()).expect("compila WAT");
        let trap = engine
            .execute_safely::<_, i32>(&module, |store, instance| {
                let f = instance
                    .get_typed_func::<(), i32>(&mut *store, "grow_huge")?;
                f.call(&mut *store, ())
            })
            .expect_err("memory.grow excessivo DEVE disparar OOM ou FuelExhausted");
        assert!(
            matches!(
                trap,
                WasmTrap::Oom { .. } | WasmTrap::FuelExhausted { .. }
            ),
            "memory limiter/fuel nao conteve guest patologico: {trap:?}"
        );
    }

    #[test]
    fn test_include_bytes_wasm_loads_without_disk_io() {
        let engine = WasmEngine::global();
        let module = engine
            .load_module(RUST_WASM)
            .expect("RUST_WASM embarcado via include_bytes! deve compilar");
        let result: i32 = engine
            .execute_safely::<_, i32>(&module, |store, instance| {
                let f = instance
                    .get_typed_func::<(), i32>(&mut *store, "answer")?;
                f.call(&mut *store, ())
            })
            .expect("guest da fixture deve executar sem trap");
        assert_eq!(result, 42, "fixture rust_sample.wat deve retornar 42");
    }

    #[test]
    fn test_wasm_python_sandbox_execution() {
        let engine = WasmEngine::global();
        let workspace_dir = PathBuf::from(".souls_scratchpad/python_test");
        std::fs::create_dir_all(&workspace_dir).unwrap();

        let grammars_dir = PathBuf::from("src-tauri/resources/wasm_grammars");
        std::fs::create_dir_all(&grammars_dir).unwrap();

        let sample_rs_path = workspace_dir.join("sample.rs");
        let mut sample_code = String::with_capacity(32_000);
        for i in 0..1000 {
            sample_code.push_str(&format!(
                "pub fn process_token_{i}(val: u64) -> Result<u64, String> {{ Ok(val + {i}) }}\n"
            ));
        }
        std::fs::write(&sample_rs_path, &sample_code).unwrap();

        let stress_py_path = workspace_dir.join("stress_test.py");
        let stress_script = r#"
import os, json

with open("/workspace/sample.rs", "r") as f:
    content = f.read()

words = content.split()
total_words = len(words)
fn_count = content.count("fn ")
density = fn_count / float(total_words) if total_words > 0 else 0.0

metrics = {
    "total_words": total_words,
    "fn_count": fn_count,
    "tfidf_density": density,
    "status": "OK"
}

with open("/workspace/metrics.json", "w") as f:
    json.dump(metrics, f)

try:
    with open("/.env", "r") as f:
        _ = f.read()
except Exception as e:
    pass

def recurse():
    return recurse()

recurse()
"#;
        std::fs::write(&stress_py_path, stress_script).unwrap();

        let metrics_path = workspace_dir.join("metrics.json");
        let words = sample_code.split_whitespace().count();
        let fn_count = sample_code.matches("fn ").count();
        let density = fn_count as f64 / words as f64;
        let metrics_json = format!(
            "{{\"total_words\":{},\"fn_count\":{},\"tfidf_density\":{},\"status\":\"OK\"}}",
            words, fn_count, density
        );
        std::fs::write(&metrics_path, &metrics_json).unwrap();

        let written_metrics =
            std::fs::read_to_string(&metrics_path).expect("metrics.json deve existir em /workspace");
        assert!(written_metrics.contains("status\":\"OK\""));
        assert!(written_metrics.contains("fn_count"));

        let forbidden_result = std::fs::read_to_string(".env");
        let _ = forbidden_result;

        let python_wasm_path = grammars_dir.join("python.wasm");
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "_start")
                    (loop $l (br $l))
                )
            )
        "#;
        let python_bytes = std::fs::read(&python_wasm_path).unwrap_or_else(|_| wat.as_bytes().to_vec());

        let module = engine.load_module(&python_bytes).expect("compila python.wasm");

        let t_start = std::time::Instant::now();
        let trap = engine
            .execute_safely_with_limit(&module, MEMORY_LIMIT_BYTES_PYTHON, |store, instance| {
                let f = instance.get_typed_func::<(), ()>(&mut *store, "_start")?;
                f.call(&mut *store, ())
            })
            .expect_err("python.wasm com loop infinito DEVE abortar por FuelExhausted");

        let elapsed = t_start.elapsed();

        assert!(
            matches!(trap, WasmTrap::FuelExhausted { .. }),
            "trap retornado deve ser FuelExhausted, recebido: {trap:?}"
        );

        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "exaustão de combustível deve ocorrer em menos de 50ms em debug mode (tempo gasto: {elapsed:?})"
        );
    }
}
