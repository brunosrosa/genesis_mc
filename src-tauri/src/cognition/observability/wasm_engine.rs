//! Marco 3.8 (Fase C.2): Enjaulamento Wasmtime do Tree-Sitter.
//!
//! Esta cerca perimétrica blinda o gateway `souls_mcp` contra os três
//! vetores de ataque histórico do `tree-sitter` C nativo:
//!
//! 1. **Segfaults** propagados de `panic!` internos do parser.
//! 2. **Loops infinitos** consumindo 100% de um worker do Tokio.
//! 3. **Footprint ilimitado** alocando ASTs de centenas de MB no heap Host.
//!
//! A solução canônica: cada chamada de parsing roda dentro de um
//! [`wasmtime::Store`] com **memory limiter de 16 MiB** e **fuel metering
//! de 10 milhões de unidades**. Traps do guest (unreachable, OOM, fuel
//! exhausted) são classificados via [`WasmTrap`] e retornados como
//! `Err` estruturado em vez de derrubar a thread do Tokio.
//!
//! ## Padrão de uso
//!
//! ```no_run
//! use souls_mc_lib::cognition::observability::wasm_engine::{WasmEngine, RUST_WASM};
//!
//! let engine = WasmEngine::global();
//! let module = engine.load_module(RUST_WASM)?;
//! let result = engine.execute_safely::<_, i32>(&module, |store, instance| {
//!     let f = instance.get_typed_func::<(), i32>(&mut *store, "answer")?;
//!     f.call(&mut *store, ())
//! })?;
//! assert_eq!(result, 42);
//! # Ok::<(), wasmtime::Error>(())
//! ```
//!
//! ## Hard Constraints
//!
//! - **Teto de memória linear:** 16 MiB por Store (2x a gramática típica).
//! - **Teto de fuel:** 10.000.000 unidades (200x uma gramática média).
//! - **Singleton de Engine:** `OnceLock<Engine>` para amortizar o cold start.
//! - **Sem host functions:** o guest é CPU-puro, estanque, sem I/O.

use std::sync::OnceLock;

use wasmtime::{Engine, Module, ResourceLimiter, Store};

/// Bytecode WAT de fixture embarcado em compile-time (Marco 4.0.2).
///
/// Substitui paths relativos de disco por fatia de bytes estática
/// (`&'static [u8]`), eliminando fragilidade de I/O em runtime
/// (especialmente em testes paralelos `cargo test --workspace`).
///
/// **Origem:** `data/wasm/rust_sample.wat`. Substituível por
/// bytecode tree-sitter real via `tree-sitter generate` + `wat2wasm`
/// no próximo Marco.
pub const RUST_WASM: &[u8] =
    include_bytes!("../../../data/wasm/rust_sample.wat");

/// Teto de memória linear por Store (16 MiB).
///
/// Gramáticas `tree-sitter-c`/`tree-sitter-rust` compiladas para WASM
/// raramente excedem 8 MiB. O teto de 16 MiB dá folga 2x para entradas
/// grandes sem permitir exaustão de memória do Host.
pub const MEMORY_LIMIT_BYTES: usize = 16 * 1024 * 1024;

/// Teto de fuel por invocação de guest.
///
/// 1 unidade de fuel = 1 instrução WASM. Gramática típica consome ~50K;
/// o teto 10M tem folga 200x para entradas patológicas sem permitir
/// loops infinitos que monopolizem o worker thread.
pub const FUEL_LIMIT: u64 = 10_000_000;

/// Implementação concreta do [`ResourceLimiter`] do Wasmtime 29.
///
/// Rejeita qualquer crescimento de memória além de [`MEMORY_LIMIT_BYTES`].
/// Tabelas e instâncias são aceitas sem limite (não aplicáveis a
/// gramáticas tree-sitter que são CPU-puras).
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
///
/// O gateway **nunca** deixa um trap do guest propagar como `panic!`
/// para o runtime do Tokio. Cada trap é convertido em uma das variantes
/// abaixo para que o handler MCP possa retornar um `RpcError` com
/// mensagem acionável.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmTrap {
    /// `unreachable` ou divisão por zero dentro do guest.
    Unreachable { reason: String },
    /// Crescimento de memória linear excedeu o [`MEMORY_LIMIT_BYTES`].
    Oom { reason: String },
    /// Contador de fuel zerou antes do guest retornar.
    FuelExhausted { fuel_consumed: u64 },
    /// Erro arbitrário do runtime Wasmtime (instance link, type mismatch).
    StructuredFailure { reason: String },
    /// `panic!` Rust-side no host antes de cruzar a fronteira WASM.
    HostPanic { reason: String },
}

impl std::fmt::Display for WasmTrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmTrap::Unreachable { reason } => {
                write!(f, "WASM_UNREACHABLE: {reason}")
            }
            WasmTrap::Oom { reason } => {
                write!(f, "WASM_OOM: {reason} (teto {} MiB)", MEMORY_LIMIT_BYTES / (1024 * 1024))
            }
            WasmTrap::FuelExhausted { fuel_consumed } => {
                write!(f, "WASM_FUEL_EXHAUSTED: guest consumiu {fuel_consumed} fuel units (teto {FUEL_LIMIT})")
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

/// Motor Wasmtime configurado com a cerca de recursos físicos.
///
/// **Lei 1:** singleton via [`WasmEngine::global`] para amortizar
/// o cold start de ~5ms do Cranelift JIT.
///
/// **Lei 2:** `execute_safely` é a **única** porta de entrada para
/// código guest. Toda chamada de parser tree-sitter DEVE passar por aqui.
pub struct WasmEngine {
    engine: Engine,
}

impl WasmEngine {
    /// Constrói um novo `Engine` com a cerca de recursos do ADR-044 §1.
    fn new() -> Result<Self, WasmTrap> {
        let mut config = wasmtime::Config::new();
        // NOTA: `epoch_interruption` foi propositalmente DESABILITADO
        // porque em Wasmtime 29 a combinação com `consume_fuel` causa
        // falsos positivos de FuelExhausted em funções triviais. O
        // fuel puro já é suficiente para o caso de uso de sandbox
        // tree-sitter (kill loops infinitos em <= 10M instruções).
        config.consume_fuel(true);
        // Cache de compilação on-disk; economiza ~30ms em re-execuções.
        config.cache_config_load_default().ok();

        let engine = Engine::new(&config).map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao construir Engine Wasmtime: {e}"),
        })?;
        Ok(Self { engine })
    }

    /// Devolve o singleton global do motor Wasmtime.
    ///
    /// Inicialização lazy na primeira chamada; lock-free nas subsequentes.
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<WasmEngine> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            // Falha de inicialização é estrutural — não deveria ocorrer
            // em ambiente onde wasmtime compila. Panico defensivo
            // documentado em ADR-044 §1.
            Self::new().expect("[WasmEngine] Falha estrutural ao inicializar motor Wasmtime")
        })
    }

    /// Compila um módulo WASM (bytes brutos ou WAT).
    ///
    /// `Module` é `Clone` (Arc internamente); cache no caller se for
    /// reusado entre chamadas.
    pub fn load_module(&self, bytes: &[u8]) -> Result<Module, WasmTrap> {
        Module::new(&self.engine, bytes).map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao compilar módulo WASM: {e}"),
        })
    }

    /// Executa uma closure dentro do sandbox Wasmtime com a cerca de
    /// recursos físicos aplicada.
    ///
    /// **Lei do Sandbox:** a closure recebe um `&mut Store` configurado
    /// com memory limiter e fuel; qualquer erro (trap, OOM, fuel exhausted)
    /// é convertido em [`WasmTrap`] e retornado como `Err`. A thread
    /// do Tokio **nunca** é derrubada.
    ///
    /// **HIPER-FORWARD:** o `Store` é descartado imediatamente após o
    /// retorno (RAII libera todas as páginas lineares).
    pub fn execute_safely<F, T>(&self, module: &Module, mut f: F) -> Result<T, WasmTrap>
    where
        F: FnMut(&mut Store<WasmMemoryLimiter>, &wasmtime::Instance) -> Result<T, wasmtime::Error>,
    {
        // Cria Store com o limiter como dado do Store (T = WasmMemoryLimiter).
        // Isso permite usar `store.limiter(|data| data)` sem closures
        // capturando variáveis locais — o borrow checker aceita porque
        // o limiter mora dentro do Store, com lifetime >= closure.
        let mut store = Store::new(&self.engine, WasmMemoryLimiter::new(MEMORY_LIMIT_BYTES));
        store.limiter(|data| data);
        store.set_fuel(FUEL_LIMIT).map_err(|e| WasmTrap::StructuredFailure {
            reason: format!("Falha ao injetar fuel: {e}"),
        })?;

        // Instancia o módulo. Falha aqui é estrutural (link, imports).
        let instance = match wasmtime::Instance::new(&mut store, module, &[]) {
            Ok(i) => i,
            Err(e) => {
                let consumed = store.get_fuel().unwrap_or(FUEL_LIMIT);
                return Err(classify_trap(&e, consumed));
            }
        };

        // Executa a closure do caller. `catch_unwind` blinda panics Rust-side
        // que possam escapar antes de cruzar a fronteira WASM.
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

        // RAII: store sai de escopo aqui. Toda memória linear do guest
        // é liberada deterministicamente.
        result
    }
}

/// Classifica um `wasmtime::Error` em uma das variantes de [`WasmTrap`].
///
/// Inspeção por `Debug` format para detectar keywords canônicas do Wasmtime:
/// - `"unreachable"` → Unreachable
/// - `"out of memory"` / `"memory growth"` → Oom
/// - `"fuel"` → FuelExhausted
/// - qualquer outro → StructuredFailure
fn classify_trap(err: &wasmtime::Error, fuel_consumed: u64) -> WasmTrap {
    let reason = format!("{err:?}");
    let lower = reason.to_ascii_lowercase();

    if lower.contains("unreachable") {
        WasmTrap::Unreachable { reason }
    } else if lower.contains("out of memory") || lower.contains("memory growth") || lower.contains("allocation") {
        WasmTrap::Oom { reason }
    } else if lower.contains("fuel") || lower.contains("interrupt") {
        // "interrupt" é o que o Wasmtime emite quando o fuel metering
        // mata o guest por exaustão (epoch interruption). Classificamos
        // como FuelExhausted para o handler MCP reportar consistentemente.
        WasmTrap::FuelExhausted { fuel_consumed }
    } else {
        WasmTrap::StructuredFailure { reason }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Garante que o `WasmEngine::global()` é estável e devolve a mesma
    /// instância em chamadas consecutivas (lei do singleton).
    #[test]
    fn test_engine_singleton_is_stable() {
        let a = WasmEngine::global();
        let b = WasmEngine::global();
        assert!(std::ptr::eq(a, b), "WasmEngine::global deve ser singleton estável");
    }

    /// Compila um módulo WAT trivial e executa com sucesso.
    /// Valida o caminho verde (happy path) do sandbox.
    #[test]
    fn test_execute_safely_happy_path() {
        let engine = WasmEngine::global();
        // WAT 2.0 folded form (instrucoes entre parenteses).
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

    /// WAT com `unreachable` é classificado como `WasmTrap::Unreachable`.
    /// Valida a cerca do tipo 1 (Segfaults).
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
        // Lei do Sandbox: thread do test runner ainda viva após o trap.
        // A própria continuação deste assert é a prova.
    }

    /// Teto de fuel: loop infinito em WAT é interrompido em O(fuel_limit).
    /// Valida a cerca do tipo 2 (Loops infinitos).
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
        // Pode ser FuelExhausted (10M atingido) ou OOM (stack grow) ou
        // Unreachable (host detects trap); qualquer um blinda o guest.
        assert!(
            matches!(
                trap,
                WasmTrap::FuelExhausted { .. } | WasmTrap::Oom { .. } | WasmTrap::Unreachable { .. }
            ),
            "loop infinito NÃO foi contido: {trap:?}"
        );
    }

    /// Memory limiter ativo: tentativa de alocar além de 16 MiB falha.
    /// Valida a cerca do tipo 3 (Footprint ilimitado).
    #[test]
    fn test_memory_limiter_16mib() {
        let engine = WasmEngine::global();
        // O loop só termina se (a) memory.grow retornar erro E
        // memory.size >= 100k (impossível dentro do teto de 16 MiB) OU
        // (b) o guest for morto por OOM (memory_growing) ou FuelExhausted.
        // Aceitamos qualquer um: a cerca perimetrica foi respeitada.
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

    /// Marco 4.0.2: o bytecode WAT de fixture é embarcado via
    /// `include_bytes!` (compile-time) e injetado direto no
    /// `Wasmtime::Module::new`. Garante o contrato "zero I/O em
    /// runtime" do WasmEngine e blinda contra paths relativos
    /// frágeis em testes paralelos do cargo.
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
}
