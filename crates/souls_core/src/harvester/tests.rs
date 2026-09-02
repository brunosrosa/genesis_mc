//! `tests.rs` — Suíte de testes TDD para Sandbox Wasmtime e AST Tree-Sitter

use std::path::PathBuf;
use std::time::Instant;

use crate::cognition::ast::observability::call_graph::{insert_symbol, lookup_symbol, SymbolEntry, SymbolKind};
use crate::harvester::ast_parser::{get_wasm_engine, ParserStoreData, WasmMemoryLimiter};

#[test]
fn test_wasm_treesitter_sandbox_oom_prevention() {
    let engine = get_wasm_engine();
    let wat = r#"
        (module
            (memory (export "mem") 1)
            (func (export "grow_cyclic_oom") (result i32)
                (loop $l
                    (drop (memory.grow (i32.const 1000)))
                    (br_if $l (i32.lt_s (memory.size) (i32.const 100000)))
                )
                (memory.size)
            )
        )
    "#;
    let module = wasmtime::Module::new(engine, wat.as_bytes()).expect("compila WAT de teste");
    let store_data = ParserStoreData {
        limiter: WasmMemoryLimiter::new(16 * 1024 * 1024), // 16MB limit
    };
    let mut store = wasmtime::Store::new(engine, store_data);
    store.limiter(|d| &mut d.limiter);
    store.set_fuel(10_000_000).expect("injetar fuel");

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instancia guest");
    let grow_fn = instance
        .get_typed_func::<(), i32>(&mut store, "grow_cyclic_oom")
        .expect("obtem funcao");

    let err = grow_fn.call(&mut store, ()).expect_err("deve falhar por OOM ou Fuel");
    let err_str = format!("{err:?}").to_ascii_lowercase();

    assert!(
        err_str.contains("out of memory")
            || err_str.contains("memory growth")
            || err_str.contains("allocation")
            || err_str.contains("fuel")
            || err_str.contains("interrupt")
            || err_str.contains("unreachable")
            || err_str.contains("trap"),
        "OOM ou Fuel não foi interceptado como Trap: {err:?}"
    );
}

#[test]
fn test_wasm_treesitter_fuel_limit_abort() {
    let engine = get_wasm_engine();
    let wat = r#"
        (module
            (func (export "infinite_loop") (result i32)
                (local $i i32)
                (loop $l
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))
                    (br_if $l (i32.lt_s (local.get $i) (i32.const 2000000000)))
                )
                (local.get $i)
            )
        )
    "#;
    let module = wasmtime::Module::new(engine, wat.as_bytes()).expect("compila WAT de loop");
    let store_data = ParserStoreData {
        limiter: WasmMemoryLimiter::new(16 * 1024 * 1024),
    };
    let mut store = wasmtime::Store::new(engine, store_data);
    store.limiter(|d| &mut d.limiter);
    store.set_fuel(10_000_000).expect("injetar 10M fuel");

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instancia guest");
    let loop_fn = instance
        .get_typed_func::<(), i32>(&mut store, "infinite_loop")
        .expect("obtem funcao");

    let start = Instant::now();
    let err = loop_fn.call(&mut store, ()).expect_err("deve abortar por exaustao de fuel");
    let elapsed = start.elapsed();

    let err_str = format!("{err:?}").to_ascii_lowercase();
    assert!(
        err_str.contains("fuel") || err_str.contains("interrupt"),
        "falha não foi por exaustão de combustível: {err:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "abort por fuel deve ocorrer em menos de 50ms (levou {elapsed:?})"
    );
}

#[test]
fn test_wasm_grammar_payload_size_sanast() {
    let root = crate::core::workspace_root();
    let grammars_dir = if root.join("resources").join("wasm_grammars").exists() {
        root.join("resources").join("wasm_grammars")
    } else {
        root.join("crates").join("souls_core").join("resources").join("wasm_grammars")
    };

    assert!(
        grammars_dir.exists(),
        "diretório de gramáticas WASM deve existir: {}",
        grammars_dir.display()
    );

    let entries = std::fs::read_dir(&grammars_dir)
        .expect("ler diretório wasm_grammars")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("wasm"))
        .collect::<Vec<_>>();

    assert!(
        !entries.is_empty(),
        "deve haver arquivos .wasm em wasm_grammars"
    );

    for entry in &entries {
        let metadata = std::fs::metadata(entry.path()).expect("ler metadata");
        let size = metadata.len();
        assert!(
            size >= 50 * 1024,
            "arquivo '{}' possui apenas {} bytes (< 50KB). Proibido stubs de 67 bytes.",
            entry.path().display(),
            size
        );
    }
}

#[test]
#[allow(non_snake_case)]
fn test_souls_symbol_resolution_O1() {
    let symbol_name = "EngineCoreProcessor";
    let file_path = PathBuf::from("src/core/engine.rs");

    insert_symbol(SymbolEntry {
        qualified_name: symbol_name.to_string(),
        kind: SymbolKind::Struct,
        file_path: file_path.clone(),
        line: 42,
        column: 0,
    });

    let start = Instant::now();
    let entry = lookup_symbol(symbol_name).expect("deve resolver símbolo no SYMBOL_INDEX");
    let elapsed = start.elapsed();

    assert_eq!(entry.qualified_name, symbol_name);
    assert_eq!(entry.file_path, file_path);
    assert_eq!(entry.line, 42);
    assert_eq!(entry.kind, SymbolKind::Struct);

    assert!(
        elapsed < std::time::Duration::from_millis(1),
        "resolução no SYMBOL_INDEX deve ser sub-milissegundo (< 1ms), levou {:?}",
        elapsed
    );
}
