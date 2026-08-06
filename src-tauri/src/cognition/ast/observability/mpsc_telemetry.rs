//! Marco 3.8 (Fase C.2): Ponte MPSC de telemetria sintática para o Call Graph.
//!
//! Worker em `std::thread` dedicada (NÃO `tokio::spawn`) consome
//! [`TelemetryEvent`] e atualiza [`SYMBOL_INDEX`](super::call_graph::symbol_index)
//! e [`CALL_GRAPH`](super::call_graph::call_graph) em RAM Host.
//!
//! ## Padrão HIPER-FORWARD
//!
//! As tools de mutação (`read`, `edit`, `write`) chamam
//! [`try_emit_event`]. Se o canal estiver cheio (256 eventos enfileirados
//! = workspace sob write storm), o evento é **descartado** com
//! `tracing::warn!` — o critical path do tool **nunca bloqueia**.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

use super::call_graph::{
    insert_edge, insert_symbol, remove_node, remove_symbols_for_file, SymbolEntry, SymbolKind,
};

/// Evento de telemetria sintática enviado pelo critical path das tools.
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    /// Arquivo foi criado ou modificado; `content` é o conteúdo textual
    /// bruto (UTF-8) para re-parse.
    FileMutated {
        path: PathBuf,
        content: String,
    },
    /// Arquivo foi deletado; remove do índice.
    FileDeleted {
        path: PathBuf,
    },
}

/// Capacidade do canal MPSC.
///
/// 256 eventos equivalem a ~256 mutações de arquivo. Em ritmo típico
/// de IDE (1 mutação / segundo), o canal absorve 4 minutos de backlog.
/// Em write storm, os eventos excedentes são descartados e uma métrica
/// `telemetry_events_dropped` é incrementada.
pub const CHANNEL_CAPACITY: usize = 256;

/// Contador global de eventos descartados por saturação do canal.
/// Visível em [`crate::cognition::observability::feedback`].
static EVENTS_DROPPED: AtomicU64 = AtomicU64::new(0);

/// Sender global do canal MPSC.
static TELEMETRY_TX: OnceLock<mpsc::Sender<TelemetryEvent>> = OnceLock::new();

/// Inicializa o canal MPSC e dispara o worker dedicado.
///
/// Idempotente: chamadas subsequentes são no-op. Retorna o `Sender`
/// global para uso no critical path.
pub fn init_telemetry_worker() -> &'static mpsc::Sender<TelemetryEvent> {
    TELEMETRY_TX.get_or_init(|| {
        let (tx, mut rx) = mpsc::channel::<TelemetryEvent>(CHANNEL_CAPACITY);

        // Worker em std::thread (síncrono, isolado do event loop Tokio).
        // Mesmo padrão de `mpsc_bridge.rs` (souls_graph) — espelhado
        // deliberadamente para manter previsibilidade operacional.
        std::thread::Builder::new()
            .name("souls-callgraph-telemetry".into())
            .spawn(move || {
                while let Some(event) = rx.blocking_recv() {
                    process_event(event);
                }
                tracing::info!("[CallGraphTelemetry] worker encerrou (canal fechado)");
            })
            .expect("[CallGraphTelemetry] falha ao spawnar worker thread");

        tx
    })
}

/// HIPER-FORWARD: emite um evento sem bloquear o caller.
///
/// Retorna `true` se o evento foi enfileirado, `false` se o canal estava
/// cheio (evento descartado). Use o retorno para telemetria de backpressure.
pub fn try_emit_event(event: TelemetryEvent) -> bool {
    let tx = init_telemetry_worker();
    match tx.try_send(event) {
        Ok(()) => true,
        Err(mpsc::error::TrySendError::Full(_)) => {
            let dropped = EVENTS_DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
            if dropped.is_multiple_of(64) {
                tracing::warn!(
                    "[CallGraphTelemetry] {dropped} eventos descartados (canal saturado)"
                );
            }
            false
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            tracing::error!("[CallGraphTelemetry] canal fechado — worker morreu");
            false
        }
    }
}

/// Devolve a contagem atual de eventos descartados por saturação.
pub fn events_dropped() -> u64 {
    EVENTS_DROPPED.load(Ordering::Relaxed)
}

/// Processa um evento de telemetria.
///
/// **Stub consciente:** enquanto o grammar WASM canônico
/// (`tree-sitter-c.wasm`) ainda não está integrado, esta função extrai
/// símbolos e arestas via regex leve (mesma heurística usada por
/// `observability::impact::build_import_graph`). Quando o grammar for
/// compilado, este switch vira `WasmEngine::execute_safely(...)`.
fn process_event(event: TelemetryEvent) {
    match event {
        TelemetryEvent::FileMutated { path, content } => {
            // Remove entradas antigas deste arquivo (idempotência).
            remove_symbols_for_file(&path);
            remove_node(&path.to_string_lossy());

            // Extração via regex stub (Marco 3.8 Fase D substituirá por Wasm).
            let symbols = extract_symbols_stub(&content, &path);
            for sym in symbols {
                insert_symbol(sym);
            }
            // Arestas de call: a→b quando a função/método é chamada.
            let edges = extract_call_edges_stub(&content, &path);
            let now = now_epoch_seconds();
            for (caller, callee) in edges {
                insert_edge(&caller, &callee, now);
            }
        }
        TelemetryEvent::FileDeleted { path } => {
            remove_symbols_for_file(&path);
            remove_node(&path.to_string_lossy());
        }
    }
}

/// Stub de extração de símbolos via regex.
///
/// Substituirá o WasmEngine quando o grammar canônico for compilado.
/// Heurística: linhas que começam com `pub fn `, `fn `, `pub struct `,
/// `struct `, `pub enum `, `enum `, `pub trait `, `trait `, `pub const `,
/// `const `, `pub static `, `static `.
fn extract_symbols_stub(content: &str, path: &std::path::Path) -> Vec<SymbolEntry> {
    let mut out = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let (kind, name) = if let Some(rest) = trimmed.strip_prefix("pub fn ") {
            (SymbolKind::Fn, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("fn ") {
            (SymbolKind::Fn, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("pub struct ") {
            (SymbolKind::Struct, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("struct ") {
            (SymbolKind::Struct, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("pub enum ") {
            (SymbolKind::Enum, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("enum ") {
            (SymbolKind::Enum, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("pub trait ") {
            (SymbolKind::Trait, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("trait ") {
            (SymbolKind::Trait, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("pub const ") {
            (SymbolKind::Const, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("const ") {
            (SymbolKind::Const, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("pub static ") {
            (SymbolKind::Static, first_ident_token(rest))
        } else if let Some(rest) = trimmed.strip_prefix("static ") {
            (SymbolKind::Static, first_ident_token(rest))
        } else {
            continue;
        };

        if name.is_empty() || !is_valid_identifier(name) {
            continue;
        }

        out.push(SymbolEntry {
            qualified_name: format!("{}::{}", path.display(), name),
            kind,
            file_path: path.to_path_buf(),
            line: (idx + 1) as u32,
            column: 0,
        });
    }
    out
}

/// Stub de extração de arestas de call.
///
/// Heurística: para cada `fn name(...)` no arquivo, todas as chamadas
/// `other_fn(...)` no corpo viram arestas `name → other_fn`.
fn extract_call_edges_stub(content: &str, path: &std::path::Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut current_fn: Option<String> = None;
    let mut brace_depth: i32 = 0;
    let path_prefix = path.to_string_lossy().to_string();

    for line in content.lines() {
        // Detecta início de uma nova função.
        if current_fn.is_none() {
            let trimmed = line.trim_start();
            let name = trimmed
                .strip_prefix("pub fn ")
                .or_else(|| trimmed.strip_prefix("fn "))
                .and_then(|r| r.split('(').next())
                .map(str::trim);
            if let Some(name) = name {
                if is_valid_identifier(name) {
                    current_fn = Some(format!("{path_prefix}::{name}"));
                    brace_depth = 0;
                }
            }
        }

        // Profundidade de chaves (rastreamento simplificado).
        for c in line.chars() {
            match c {
                '{' => brace_depth += 1,
                '}' => {
                    let d = brace_depth - 1;
                    if d <= 0 {
                        current_fn = None;
                        brace_depth = 0;
                    } else {
                        brace_depth = d;
                    }
                }
                _ => {}
            }
        }

        // Extrai chamadas se estamos dentro de uma função.
        if let Some(ref caller) = current_fn {
            // Procura padrões `name(` que não sejam declaração.
            let mut chars = line.char_indices().peekable();
            while let Some((i, c)) = chars.next() {
                if c.is_alphabetic() || c == '_' {
                    let start = i;
                    let mut end = i + c.len_utf8();
                    while let Some((j, nc)) = chars.peek().copied() {
                        if nc.is_alphanumeric() || nc == '_' {
                            end = j + nc.len_utf8();
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // Próximo char não-espaço deve ser `(`.
                    let after = line[end..].trim_start();
                    if after.starts_with('(') {
                        // Filtra keywords comuns (`if`, `while`, etc).
                        let name = &line[start..end];
                        if !is_keyword(name) && is_valid_identifier(name) {
                            out.push((caller.clone(), name.to_string()));
                        }
                    }
                }
            }
        }
    }
    out
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Extrai o primeiro token que seja um identificador Rust válido.
///
/// Tolera delimitadores não-Whitespace (`(`, `<`, `:`, `=`, `;`, `,`,
/// `{`, `}`, `[`, `]`, `&`, `*`, `'`, `<=`, `->`, `=>`, `::`) que
/// `split_whitespace` não separaria, preservando o nome puro do
/// símbolo. Ex.: `"LIMIT: usize = 100;"` → `"LIMIT"`.
fn first_ident_token(rest: &str) -> &str {
    let mut end = 0;
    let mut started = false;
    for (i, c) in rest.char_indices() {
        if c.is_alphabetic() || c == '_' {
            started = true;
            end = i + c.len_utf8();
        } else if started {
            break;
        }
    }
    &rest[..end]
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "else"
            | "while"
            | "for"
            | "loop"
            | "match"
            | "return"
            | "let"
            | "mut"
            | "fn"
            | "struct"
            | "enum"
            | "trait"
            | "impl"
            | "use"
            | "mod"
            | "pub"
            | "self"
            | "Self"
            | "as"
            | "in"
            | "where"
            | "type"
            | "const"
            | "static"
            | "ref"
            | "move"
    )
}

fn now_epoch_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_emit_event_never_blocks_when_channel_full() {
        // Satura o canal com 256+1 eventos sem consumir; nenhum deve
        // bloquear o critical path.
        let mut sent = 0u32;
        let mut dropped = 0u32;
        for i in 0..CHANNEL_CAPACITY + 50 {
            let evt = TelemetryEvent::FileMutated {
                path: PathBuf::from(format!("/tmp/saturate_{i}.rs")),
                content: format!("fn func_{i}() {{}}"),
            };
            if try_emit_event(evt) {
                sent += 1;
            } else {
                dropped += 1;
            }
        }
        // Em alguma janela, todos os primeiros CHANNEL_CAPACITY entram;
        // os próximos caem em backpressure.
        assert!(sent > 0, "alguns eventos devem entrar");
        assert!(dropped > 0, "excesso deve cair em descarte HIPER-FORWARD");
        assert!(events_dropped() >= dropped as u64);
    }

    #[test]
    fn test_extract_symbols_stub_finds_rust_fns() {
        let code = r#"
            pub fn hello() -> i32 { 42 }
            fn internal() {}
            pub struct Foo { x: i32 }
            pub enum Bar { A, B }
            pub trait Baz { fn quux(&self); }
            pub const LIMIT: usize = 100;
        "#;
        let path = std::path::Path::new("test.rs");
        let symbols = extract_symbols_stub(code, path);
        let names: Vec<&str> = symbols.iter().map(|s| {
            s.qualified_name.rsplit("::").next().unwrap_or("")
        }).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"internal"));
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"Bar"));
        assert!(names.contains(&"Baz"));
        assert!(names.contains(&"LIMIT"));
    }

    #[test]
    fn test_extract_call_edges_stub_finds_call_relations() {
        let code = r#"
            fn caller() {
                helper_a();
                helper_b(42);
                other_function();
            }
        "#;
        let path = std::path::Path::new("test.rs");
        let edges = extract_call_edges_stub(code, path);
        let targets: Vec<String> = edges.iter().map(|(_, c)| c.clone()).collect();
        assert!(targets.contains(&"helper_a".to_string()));
        assert!(targets.contains(&"helper_b".to_string()));
        assert!(targets.contains(&"other_function".to_string()));
    }
}
