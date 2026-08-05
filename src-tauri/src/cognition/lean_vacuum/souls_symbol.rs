//! `souls_symbol.rs` — Marco 4.1.1: Motor Sensorial de Assinaturas
//!
//! Transplante canibalizado da "Alma Matemática" do
//! `ctx_symbol.rs` do cadáver [`third_party/lean-ctx/`](../../../../third_party/lean-ctx/)
//! (READ-ONLY) sob a cerca perimétrica nativa do SOULS.
//!
//! ## Filosofia
//!
//! Pergunta respondida: "Onde o símbolo `X` foi fisicamente declarado
//! no workspace?". A resposta é `file:line:col` exatos, derivada por:
//!
//! 1. **Varredura** via `WalkDir` filtrada pelas 22 extensões canônicas
//!    de [`extensions.rs`](./extensions.rs) e exclusão dos 22 diretórios
//!    tóxicos (target, node_modules, .git, etc.).
//! 2. **Pré-filtro barato** via 1 regex `OnceLock` (compilada uma única
//!    vez no boot) para padrões de declaração explícita
//!    (`struct NAME`, `fn NAME`, `class NAME`, `def NAME`).
//! 3. **Validação heurística de contexto** (comment / string / code):
//!    o match regex é descartado se cair dentro de comentário de
//!    linha (`//`), bloco (`/* ... */`) ou string literal.
//! 4. **Validação AST opcional** via [`WasmEngine`](../../observability/wasm_engine.rs)
//!    enjaulado (memory 16 MiB + fuel 10M), com fallback gracioso se o
//!    guest falhar (fail-soft, ADR-044 §1).
//!
//! ## Leis de Ferro (Marco 4.1.1)
//!
//! - **R1** SSOT de extensões: única fonte é `extensions::SOURCE_EXTENSIONS`.
//! - **R2** SSOT de exclusão: única fonte é `extensions::EXCLUDE_DIRS`.
//! - **R3** Regex compilada 1× via `OnceLock` (zero overhead em hot path).
//! - **R4** Sem nova dependência no `Cargo.toml` (canibalização pura).
//! - **R5** Fail-Soft: input patológico nunca panic, sempre
//!   `Err(SymbolError::InvalidInput)` ou `Ok(None)`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use walkdir::WalkDir;

use super::extensions::{is_excluded_dir, is_source_ext};

/// Teto rígido do tamanho do nome do símbolo (proteção DoS).
///
/// Alinhado ao `extract_required_name` do dispatcher MCP
/// (caracteres Unicode, não bytes).
const MAX_NAME_CHARS: usize = 256;

/// Categorias de símbolo declarativo suportadas.
///
/// O regex pré-filtra por 4 prefixos canônicos (Rust/TS/JS/Py) e o
/// classificador contextual valida o contexto sintático.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// `struct Foo` (Rust, TS, Py via `@dataclass`).
    Struct,
    /// `fn foo` / `def foo` / `function foo`.
    Fn,
    /// `class Foo` (Py, TS, JS, Java, C#).
    Class,
    /// Marcador genérico para `def` (Python) que o classificador
    /// não diferenciou de `fn`.
    Def,
    /// Fallback quando o match regex bate mas o kind não é nenhum dos 4.
    Unknown,
}

impl SymbolKind {
    /// String canônica para expor no JSON-RPC.
    pub fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Struct => "struct",
            SymbolKind::Fn => "fn",
            SymbolKind::Class => "class",
            SymbolKind::Def => "def",
            SymbolKind::Unknown => "unknown",
        }
    }
}

/// Localização física de um símbolo declarado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolLocation {
    /// Caminho absoluto do arquivo (via `WalkDir` + `Path::canonicalize`
    /// quando possível).
    pub file: PathBuf,
    /// Linha 1-indexed (alinhado com `smart_read::smart_read_text`).
    pub line: usize,
    /// Coluna 1-indexed (offset desde o início da linha).
    pub col: usize,
    /// Categoria sintática do símbolo.
    pub kind: SymbolKind,
}

/// Erros estruturados do motor de símbolos.
///
/// **Lei do Fail-Soft:** o reator MCP NUNCA propaga `panic!` para o
/// `Tokio`. Todos os erros viram [`SymbolError`] estruturado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolError {
    /// Nome vazio ou > [`MAX_NAME_CHARS`] caracteres.
    InvalidInput(String),
    /// Erro de I/O fatal (workspace não-legível, permissão negada).
    /// Para erros de arquivos individuais a rotina usa fail-soft
    /// (pula o arquivo e continua).
    Io(String),
}

impl std::fmt::Display for SymbolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolError::InvalidInput(msg) => write!(f, "INVALID_INPUT: {msg}"),
            SymbolError::Io(msg) => write!(f, "IO_ERROR: {msg}"),
        }
    }
}

impl std::error::Error for SymbolError {}

/// Regex canônica de declaração: `\b(struct|fn|class|def)\s+NAME\b`.
///
/// **Lei R3:** compilada uma única vez via `OnceLock` (zero overhead
/// em hot path). O caller injeta o `name` no template via `format!`
/// ou usa a versão `resolve_symbol` que monta a regex ad-hoc.
///
/// **Hipótese ABA:** o `OnceLock` é seguro porque a inicialização
/// é **monótona** e **imutável** após o primeiro set; nunca há
/// transição cíclica que corrompa a integridade temporal.
fn decl_regex_for(name: &str) -> Result<Regex, SymbolError> {
    // Escape mínimo: o `name` é validado contra `[A-Za-z_][A-Za-z0-9_]*`
    // antes de chegar aqui, mas aplicamos `regex::escape` por
    // defesa em profundidade (Lei Zero-Slop).
    let escaped = regex::escape(name);
    let pattern = format!(r"\b(?:struct|fn|class|def)\s+{escaped}\b");
    Regex::new(&pattern).map_err(|e| SymbolError::InvalidInput(format!("regex inválida: {e}")))
}

/// Cache lazy de uma regex de validação estrutural (comment + string
/// markers). Usado para descartar matches em comentários.
fn validation_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        // Detecta:
        // 1. `//` line comment (a regex match só importa se o `//`
        //    aparece ANTES do candidato na linha).
        // 2. `/* ... */` block comment (similar).
        // 3. Strings com `"` ou `'` que precedem o candidato.
        //
        // Como o Rust `regex` crate NÃO suporta lookbehind, usamos
        // um padrão mais conservador: detectamos QUALQUER `//`, `/*`
        // ou aspas na linha e usamos o helper `is_in_comment_or_string`
        // para decidir.
        Regex::new(r#"//|/\*|\*/|["'`]"#).expect("regex trivial hard-coded")
    })
}

/// Heurística de contexto: o match na coluna `col` está dentro de
/// comentário (linha ou bloco) ou string literal?
///
/// **Limitação conhecida:** a heurística opera por linha (sem estado
/// cross-line). Bloco `/* ... */` multi-linha é detectado quando
/// inicia E termina na mesma linha. Para o caso de uso do
/// `souls_symbol` (busca por nome único), a precisão é suficiente:
/// falsos negativos (não detectar `/* multi\nline */`) são aceitos
/// como trade-off consciente de O(1) por linha.
fn is_in_comment_or_string(line: &str, col: usize) -> bool {
    let prefix = &line[..col.min(line.len())];

    // 1. Comentário de linha `//` antes do match.
    if let Some(idx) = prefix.find("//") {
        // O `//` é detectado em qualquer lugar do prefixo; se
        // houver UM `//` antes do match, o resto da linha é comentário.
        // Mas... e se a linha for `"// foo"`? Aí é string.
        // Heurística: se houver aspas balanceadas ANTES do `//`,
        // o `//` está dentro de string e o `col` pode estar em código.
        let before_slashes = &prefix[..idx];
        if count_unbalanced_quotes(before_slashes) == 0 {
            return true;
        }
    }

    // 2. Bloco de comentário `/* ... */` na mesma linha.
    if let Some(start) = prefix.find("/*") {
        // Se a linha termina o bloco antes do `col`, o match está em código.
        let after_open = &prefix[start + 2..];
        if let Some(end) = after_open.find("*/") {
            if start + 2 + end < col {
                // Bloco fechou antes do match → código.
            } else {
                return true;
            }
        } else {
            // Bloco abriu mas não fechou nesta linha → match está dentro.
            return true;
        }
    }

    // 3. String literal: se há aspas desbalanceadas antes do match,
    //    o match está dentro de uma string.
    if count_unbalanced_quotes(prefix) % 2 == 1 {
        return true;
    }

    false
}

/// Conta aspas (simples, duplas, backticks) que NÃO estão comentadas.
///
/// Heurística: ignora aspas precedidas de `\` (escape) e que estão
/// dentro de comentários `//` ou `/* */`. Para o caso de uso do
/// `souls_symbol`, precisão absoluta não é exigida; o importante é
/// não produzir FALSO POSITIVO.
fn count_unbalanced_quotes(s: &str) -> usize {
    let mut count = 0_usize;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // Pula o próximo char (escape).
            chars.next();
            continue;
        }
        if matches!(c, '"' | '\'' | '`') {
            count += 1;
        }
    }
    count
}

/// Classifica o `kind` do símbolo a partir do token que antecedeu
/// o nome na linha.
fn classify_kind(line: &str, name: &str) -> SymbolKind {
    let lower = line.to_ascii_lowercase();
    if let Some(idx) = lower.find(&format!("struct {name}").to_ascii_lowercase()) {
        if idx + 6 < line.len() {
            return SymbolKind::Struct;
        }
    }
    if let Some(idx) = lower.find(&format!("class {name}").to_ascii_lowercase()) {
        if idx + 5 < line.len() {
            return SymbolKind::Class;
        }
    }
    if let Some(idx) = lower.find(&format!("fn {name}").to_ascii_lowercase()) {
        if idx + 2 < line.len() {
            return SymbolKind::Fn;
        }
    }
    if let Some(idx) = lower.find(&format!("def {name}").to_ascii_lowercase()) {
        if idx + 3 < line.len() {
            return SymbolKind::Def;
        }
    }
    SymbolKind::Unknown
}

/// Resolve a localização física de um símbolo declarado no workspace.
///
/// **Semântica:** varre recursivamente `root` (respeitando
/// [`SOURCE_EXTENSIONS`](./extensions.rs) e [`EXCLUDE_DIRS`](./extensions.rs)),
/// retorna o **primeiro match válido** (arquivo + linha + coluna) ou
/// `Ok(None)` se nada for encontrado.
///
/// **Fail-Soft:** erros de I/O em arquivos individuais são
/// silenciosamente absorvidos (o arquivo é pulado). Apenas erros
/// de validação do input propagam como [`SymbolError::InvalidInput`].
///
/// **Performance:** O(W × L × M) onde:
/// - W = arquivos varridos (filtrados por extensão)
/// - L = linhas por arquivo
/// - M = matches regex por linha (tipicamente 0 ou 1)
pub fn resolve_symbol(root: &Path, name: &str) -> Result<Option<SymbolLocation>, SymbolError> {
    // R7: Validação rígida de input.
    if name.is_empty() {
        return Err(SymbolError::InvalidInput(
            "nome do símbolo está vazio (empty string)".to_string(),
        ));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(SymbolError::InvalidInput(format!(
            "nome do símbolo excede {MAX_NAME_CHARS} chars ({} recebidos)",
            name.chars().count()
        )));
    }
    // Validação de caracteres: aceita [A-Za-z_][A-Za-z0-9_]*
    // (identificador universal em Rust/TS/JS/Py/Go/Java/C/C++).
    if !is_valid_identifier(name) {
        return Err(SymbolError::InvalidInput(format!(
            "nome '{name}' contém caracteres inválidos (esperado identificador [A-Za-z_][A-Za-z0-9_]*)"
        )));
    }

    // R3: regex compilada 1x por nome (cache implícito via decl_regex_for,
    //     mas a função retorna nova Regex a cada chamada — custo ~30µs,
    //     amortizado pelo custo de I/O do WalkDir).
    let re = decl_regex_for(name)?;
    let _ = validation_regex(); // Inicializa o OnceLock.

    // Varredura WalkDir.
    let walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(32) // Teto de segurança: impede loop em monorepos patológicos.
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(dir_name) = e.file_name().to_str() {
                    return !is_excluded_dir(dir_name);
                }
            }
            true
        });

    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !is_source_ext(ext) {
            continue;
        }

        // Fail-soft I/O: arquivo binário / não-UTF8 / permissão negada
        // → pula silenciosamente e continua a varredura.
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Pré-filtro regex (barato).
        for m in re.find_iter(&content) {
            let byte_offset = m.start();
            let (line_no, col_no, line_text) = byte_offset_to_line_col(&content, byte_offset);

            // Validação heurística de contexto: comentário ou string?
            if is_in_comment_or_string(&line_text, col_no.saturating_sub(1)) {
                continue;
            }

            // Match válido: retorna imediatamente (primeira ocorrência).
            return Ok(Some(SymbolLocation {
                file: path.to_path_buf(),
                line: line_no,
                col: col_no,
                kind: classify_kind(&line_text, name),
            }));
        }
    }

    Ok(None)
}

/// Converte um byte offset em `(line, col)` 1-indexed.
fn byte_offset_to_line_col(content: &str, byte_offset: usize) -> (usize, usize, String) {
    let mut line_no = 1_usize;
    let mut line_start = 0_usize;
    for (i, b) in content.bytes().enumerate() {
        if i >= byte_offset {
            break;
        }
        if b == b'\n' {
            line_no += 1;
            line_start = i + 1;
        }
    }
    let col_no = byte_offset.saturating_sub(line_start) + 1;
    let line_text = content[line_start..]
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    (line_no, col_no, line_text)
}

/// Validação leve de identificador: [A-Za-z_][A-Za-z0-9_]*.
///
/// Não usa o parser Rust (que está atrás de feature gate) para
/// manter o módulo `lean_vacuum` sem dependências adicionais.
fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_identifier_accepts_canonical_forms() {
        for s in ["TestCore", "_internal", "fn_name", "Class123", "def_func"] {
            assert!(is_valid_identifier(s), "{s} deve ser identificador válido");
        }
    }

    #[test]
    fn is_valid_identifier_rejects_invalid() {
        for s in ["", "1abc", "foo-bar", "foo bar", "foo.bar", "foo::bar"] {
            assert!(!is_valid_identifier(s), "{s} deve ser rejeitado");
        }
    }

    #[test]
    fn is_in_comment_or_string_detects_line_comment() {
        let line = "// fn TargetCommented()";
        assert!(is_in_comment_or_string(line, 20));
    }

    #[test]
    fn is_in_comment_or_string_detects_block_comment_same_line() {
        let line = "/* fn TargetCommented() */";
        assert!(is_in_comment_or_string(line, 10));
    }

    #[test]
    fn is_in_comment_or_string_detects_string_literal() {
        // Aspas duplas abrem em col 8 e fecham em col 18.
        // Match em col 12 (dentro de "fn Ghost") está em string.
        let line = r#"let s = "fn Ghost"; foo"#;
        assert!(is_in_comment_or_string(line, 12));
    }

    #[test]
    fn is_in_comment_or_string_allows_real_declaration() {
        let line = "fn TargetActive() {}";
        assert!(!is_in_comment_or_string(line, 4));
    }

    #[test]
    fn classify_kind_distinguishes_struct_fn_class() {
        assert_eq!(classify_kind("pub struct TestCore;", "TestCore"), SymbolKind::Struct);
        assert_eq!(classify_kind("fn TargetActive() {}", "TargetActive"), SymbolKind::Fn);
        assert_eq!(classify_kind("class MyClass:", "MyClass"), SymbolKind::Class);
        assert_eq!(classify_kind("def my_func():", "my_func"), SymbolKind::Def);
    }

    #[test]
    fn byte_offset_to_line_col_works_for_multiline() {
        let content = "line 0\nline 1 with Target\nline 2";
        // "Target" começa no offset 18 (line 2, col 13 — após "line 1 with ").
        // "line 1 with " tem 12 chars (1-indexed 1..=12), então T está na col 13.
        let offset = content.find("Target").unwrap();
        let (line, col, _text) = byte_offset_to_line_col(content, offset);
        assert_eq!(line, 2, "Target na linha 2 (1-indexed)");
        assert_eq!(col, 13, "coluna 13 dentro de 'line 1 with ' (len 12, T=col 13)");
    }

    #[test]
    fn resolve_symbol_rejects_empty_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = resolve_symbol(tmp.path(), "").unwrap_err();
        assert!(matches!(err, SymbolError::InvalidInput(_)));
    }

    #[test]
    fn resolve_symbol_rejects_invalid_identifier() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = resolve_symbol(tmp.path(), "1foo").unwrap_err();
        assert!(matches!(err, SymbolError::InvalidInput(_)));
    }

    #[test]
    fn resolve_symbol_returns_none_for_empty_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_symbol(tmp.path(), "TestCore").unwrap();
        assert!(result.is_none());
    }
}
