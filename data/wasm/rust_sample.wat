;; Fixture WAT para testes do WasmEngine.
;;
;; Este arquivo existe para satisfazer a doutrina SDD/Marco 4.0.2 de
;; eliminar I/O frágil em runtime: o bytecode (em formato texto WAT
;; aceito pelo Wasmtime 29 via feature `wat`) eh embarcado no binario
;; Rust em compile-time via `include_bytes!`, sem dependencia de
;; caminho de disco em runtime.
;;
;; Quando o grammar canonico tree-sitter-rust for compilado (proximo
;; Marco), substitua este stub pelo bytecode real gerado por
;; `tree-sitter generate` + `wat2wasm`.
(module
    (func (export "answer") (result i32)
        (i32.const 42)
    )
)
