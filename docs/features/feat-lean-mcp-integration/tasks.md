# Tasks — Canibalização Tipo A: Saco a Vácuo Nativo (Fase 3)

**Lei do Scaffold:** Cada task tem DoD rigoroso, com teste Red antes da lógica Green.
**Ralph Loop:** Teto de 3 tentativas por task. Ao bater o teto, acionar `soda-ralph-loop`.

---

## T0 — Atualizar SDD (W1)

**Arquivos:**
- `docs/features/feat-lean-mcp-integration/design.md` (Fase 3)
- `docs/features/feat-lean-mcp-integration/tasks.md` (este arquivo)

### T0.1 — DoD
- [x] `design.md` contém diagrama Mermaid da Canibalização Tipo A.
- [x] `tasks.md` lista T1/T2/T3 com DoD atômico.

---

## T1 — Criar `cognition/lean_vacuum/` (W2)

**Arquivos novos (5):**
- `src-tauri/src/cognition/lean_vacuum/mod.rs`
- `src-tauri/src/cognition/lean_vacuum/dot_flatten.rs`
- `src-tauri/src/cognition/lean_vacuum/ansi_filter.rs`
- `src-tauri/src/cognition/lean_vacuum/myers_diff.rs`
- `src-tauri/src/cognition/lean_vacuum/text_compress.rs`

**Arquivo modificado (1):**
- `src-tauri/src/core/mod.rs` — adicionar `similar = "=2.7.0"` como dep direta.
- `src-tauri/src/cognition/mod.rs` — adicionar `pub mod lean_vacuum;`

### T1.1 — `ansi_filter.rs` (TDD Red→Green)

**Teste Red:**
```rust
#[test]
fn strip_ansi_removes_color_codes() {
    let raw = "\x1b[31mERROR\x1b[0m: failed at \x1b[1mline 42\x1b[0m";
    let cleaned = strip_ansi(raw);
    assert_eq!(cleaned, "ERROR: failed at line 42");
}
```

**Green:**
- `pub fn strip_ansi(s: &str) -> String` — varre chars, remove de `\x1b` até ASCII alpha.
- `pub fn ansi_density(s: &str) -> f64` — `count('\x1b') / s.len() as f64`.

**DoD:**
- [ ] 2 testes passando.
- [ ] `cargo check --lib` Exit 0.

### T1.2 — `dot_flatten.rs` (TDD Red→Green)

**Teste Red:**
```rust
#[test]
fn dot_flatten_simple_object() {
    let v = json!({"a": {"b": {"c": 42}}});
    assert_eq!(dot_flatten(&v), "a.b.c=42");
}

#[test]
fn dot_flatten_with_booleans_literal() {
    let v = json!({"enabled": true, "verbose": false});
    assert_eq!(dot_flatten(&v), "enabled=true\nverbose=false");
}

#[test]
fn dot_flatten_array() {
    let v = json!({"items": [1, 2, 3]});
    assert_eq!(dot_flatten(&v), "items[0]=1\nitems[1]=2\nitems[2]=3");
}

#[test]
fn dot_flatten_nested_array_of_objects() {
    let v = json!({"users": [{"name": "alice"}, {"name": "bob"}]});
    assert_eq!(dot_flatten(&v), "users[0].name=alice\nusers[1].name=bob");
}

#[test]
fn dot_flatten_string_with_special_chars_preserves_value() {
    let v = json!({"msg": "true = false : end"});
    assert_eq!(dot_flatten(&v), "msg=\"true = false : end\"");
}
```

**Green:**
- `pub fn dot_flatten(value: &serde_json::Value) -> String`
- Encoder recursivo. Para Object: `{k}.{nested}` ou `[{i}]` para Array.
- Booleanos literais `true`/`false` (característica LEAN canônica).
- Strings com aspas duplas para preservar espaços e caracteres especiais.

**DoD:**
- [ ] 5 testes passando.
- [ ] `cargo check --lib` Exit 0.

### T1.3 — `myers_diff.rs` (TDD Red→Green)

**Teste Red:**
```rust
#[test]
fn myers_diff_no_changes() {
    let out = myers_diff("a\nb\nc", "a\nb\nc");
    assert_eq!(out, "(no changes)");
}

#[test]
fn myers_diff_single_insertion() {
    let out = myers_diff("a\nb", "a\nx\nb");
    assert!(out.contains("+2: x"));
    assert!(out.contains("+1/-0 lines"));
}

#[test]
fn myers_diff_deletion_and_insertion() {
    let out = myers_diff("a\nb\nc", "a\nX\nc");
    assert!(out.contains("-2: b"));
    assert!(out.contains("+2: X"));
    assert!(out.contains("+1/-1 lines"));
}
```

**Green:**
- `pub fn myers_diff(before: &str, after: &str) -> String`
- Usa `similar::TextDiff::from_lines` + `iter_all_changes()`.
- Formato: `+{n}: {text}`, `-{n}: {text}`, footer `\ndiff +{adds}/-{dels} lines`.
- Early-return `"(no changes)"` se `before == after`.

**DoD:**
- [ ] 3 testes passando.
- [ ] `cargo check --lib` Exit 0.

### T1.4 — `text_compress.rs` (TDD Red→Green)

**Teste Red:**
```rust
#[test]
fn aggressive_compress_strips_rust_line_comments() {
    let raw = "fn main() {\n    // debug print\n    println!(\"hi\");\n}";
    let out = aggressive_compress(raw, Some("rs"));
    assert!(!out.contains("// debug print"));
    assert!(out.contains("fn main()"));
}

#[test]
fn lightweight_cleanup_collapses_brace_runs() {
    let raw = "fn a() {\n}\nfn b() {\n}\n";
    let out = lightweight_cleanup(raw);
    // 3+ braces colapsam em uma linha sumária
    assert!(out.lines().count() < raw.lines().count() || out.contains("collapsed"));
}
```

**Green:**
- `pub fn aggressive_compress(content: &str, ext: Option<&str>) -> String`
- Remove linhas de comentário (`//`, `#`, `--`, `/* */`, `<!-- -->`) por extensão.
- Colapsa runs de `}` / `);`.
- `pub fn lightweight_cleanup(content: &str) -> String` — colapsa blank lines,
  colapsa runs de `};` se > 5.

**DoD:**
- [ ] 2 testes passando.
- [ ] `cargo check --lib` Exit 0.

### T1.5 — `mod.rs` (orquestrador)

- `pub mod ansi_filter; pub mod dot_flatten; pub mod myers_diff; pub mod text_compress;`
- `pub fn compress_to_lean(text: &str, ext: Option<&str>) -> String`:
  - Aplica `strip_ansi` → `aggressive_compress` → `lightweight_cleanup`.
- `pub fn read_to_lean(path: &Path) -> std::io::Result<String>`:
  - Lê arquivo (5MB hard cap), aplica `compress_to_lean`.

**DoD:**
- [ ] `cargo check --lib` Exit 0.
- [ ] Todos os testes de T1.1–T1.4 passando.

### T1.6 — Adicionar `similar` como dep direta

**Arquivo:** `src-tauri/Cargo.toml`

```toml
similar = "=2.7.0"  # SODA-CANIBALIZED Fase 3: Myers diff nativo (já estava em lean-ctx como dep transitiva)
```

**DoD:**
- [ ] `cargo check --lib` Exit 0.
- [ ] `cargo tree -p souls_mc_lib | grep -E "^├─ similar|^└─ similar"` mostra a crate.

---

## T2 — Transplante das 2 Ferramentas Vitais (W3)

**Arquivo:** `src-tauri/src/bin/soda_mcp_server.rs`

### T2.1 — `souls_read` real

**Pipeline:**
1. Validar `arguments.path` é String não-vazio.
2. `let path = PathBuf::from(...)`; verificar `path.exists()` e `path.is_file()`.
3. `lean_vacuum::read_to_lean(&path)?` → string comprimida.
4. Retornar `content[0].text = markdown` + `structuredContent` com métricas.

**DoD:**
- [ ] `cargo check --bin soda_mcp_server` Exit 0.
- [ ] `cargo test --bin soda_mcp_server` (se houver teste) Exit 0.

### T2.2 — `souls_delta_diff` real

**Pipeline:**
1. Validar `arguments.before` e `arguments.after` são Strings.
2. `lean_vacuum::myers_diff::myers_diff(before, after)`.
3. Retornar `content[0].text` + `structuredContent` com counts.

**DoD:**
- [ ] `cargo check --bin soda_mcp_server` Exit 0.

---

## T3 — Renomear 15 Stubs para `not_implemented_yet` (W4)

**Arquivo:** `src-tauri/src/bin/soda_mcp_server.rs`

### T3.1 — Renomear função

```rust
fn stub_not_implemented_yet(tool_name: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!(
                "not_implemented_yet: tool '{}' reconhecida no cânone SODA. \
                 Aguardando Fase 4+ para transplante da lógica adicional.",
                tool_name
            )
        }],
        "is_error": true
    })
}
```

### T3.2 — Atualizar match

Trocar `Ok(stub_not_implemented(tool_name))` → `Ok(stub_not_implemented_yet(tool_name))`.

### T3.3 — Atualizar `tools/list` no router

Trocar `"STUB: ..."` → `"not_implemented_yet: ..."` em cada uma das 15 descrições.

**DoD:**
- [ ] `grep -c "STUB:" src/bin/soda_mcp_server.rs` retorna `0`.
- [ ] `grep -c "not_implemented_yet" src/bin/soda_mcp_server.rs` retorna `15` (descrições) + 1 (fn) = 16.

---

## T4 — Validação Final (Fase 5)

- [ ] `cargo check --lib` Exit 0.
- [ ] `cargo check --bin soda_mcp_server` Exit 0.
- [ ] `cargo test --lib cognition::lean_vacuum` Exit 0 (≥ 6 testes).
- [ ] `git diff Cargo.toml | grep "lean-ctx"` retorna vazio.
- [ ] `git diff --stat` mostra ≤ 6 arquivos modificados e ≤ 6 arquivos criados.
- [ ] `third_party/lean-ctx/` permanece intocado (`git status third_party/lean-ctx/` mostra clean).
- [ ] Blast Radius diff atômico compilado e enviado para Agent Inbox.

---

**Modo de Execução:** Sequencial (T1 → T2 → T3 → T4).
**Ralph Loop:** Teto de 3 tentativas por task.

---

## Fase 4 — Rename infraestrutural do binário MCP

### T5 — Atualizar SDD do rename

- [ ] `design.md` documenta o rename `soda_mcp_server` -> `souls_mcp_server`.
- [ ] `tasks.md` enumera T6/T7.

### T6 — Rename físico + callers

**Arquivos-alvo**
- `src-tauri/src/bin/soda_mcp_server.rs` -> `src-tauri/src/bin/souls_mcp_server.rs`
- `src-tauri/Cargo.toml`
- `gateway-config.yaml`
- `boot.ps1`
- `src-tauri/soda_ETL_ignition.ps1`
- `src-tauri/src/core/mcp_transport.rs`

**DoD**
- [ ] nenhum arquivo restante com nome físico `soda_mcp_server.rs`
- [ ] nenhum `cmd:` em YAML apontando para `soda_mcp_server.exe`
- [ ] nenhum `.ps1` chamando `--bin soda_mcp_server` ou matando processo com esse nome

### T7 — Validação do compilador

**DoD**
- [ ] `cargo check --bin souls_mcp_server` Exit 0
- [ ] blast radius listado no feedback final
