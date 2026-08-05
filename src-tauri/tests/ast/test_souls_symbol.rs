//! `test_souls_symbol.rs` — Marco 4.1.1: Motor Sensorial de Assinaturas
//!
//! Caderno TDD com 3 contratos rígidos que validam a ferramenta `souls_symbol`:
//!
//! 1. `test_resolve_symbol_struct` — `pub struct TestCore;` deve ser
//!    resolvido com `(file, line, col)` exatos via Regex + AST Wasmtime.
//!
//! 2. `test_symbol_comment_protection` — `/* fn TargetCommented() */` é
//!    **imune** ao falso positivo (regex match, mas AST classifica como
//!    `Comment`). Já `fn TargetActive()` é resolvido normalmente.
//!
//! 3. `test_symbol_empty_or_invalid_workspace` — buscas sobre símbolos
//!    inexistentes ou arquivos corrompidos retornam `NotFound` estruturado
//!    sem crashar o reator.
//!
//! **Lei do Scaffold:** estes 3 testes foram escritos ANTES da
//! implementação (Red puro). Devem falhar com `cannot find function
//! resolve_symbol` e passar após TASK-02.

use souls_mc_lib::cognition::lean_vacuum::souls_symbol::{resolve_symbol, SymbolError, SymbolKind};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Caso 1: `pub struct TestCore;` deve ser encontrado com
/// `(file, line, col)` exatos. Valida o caminho verde do Regex-AST.
#[test]
fn test_resolve_symbol_struct() {
    let tmp = TempDir::new().expect("cria tempdir");
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).expect("cria src/");

    // Cria `lib.rs` com `pub struct TestCore;` na linha 1.
    let lib_path: PathBuf = src_dir.join("lib.rs");
    let lib_content = "pub struct TestCore;\n";
    fs::write(&lib_path, lib_content).expect("escreve lib.rs");

    // Resolução via módulo nativo.
    let result = resolve_symbol(tmp.path(), "TestCore")
        .expect("resolve_symbol não deve panic em workspace válido");

    let loc = result.expect("TestCore deve ser encontrado em lib.rs");
    assert_eq!(
        loc.kind,
        SymbolKind::Struct,
        "TestCore deve ser classificado como Struct (regex + AST)"
    );
    assert_eq!(
        loc.file.file_name().and_then(|s| s.to_str()),
        Some("lib.rs"),
        "encontrado em lib.rs (caminho: {:?})",
        loc.file
    );
    assert_eq!(
        loc.line, 1,
        "TestCore está fisicamente na linha 1, got {}",
        loc.line
    );
    assert!(
        loc.col >= 1,
        "coluna deve ser >= 1 (após 'pub struct '), got {}",
        loc.col
    );
}

/// Caso 2: comentário `/* fn TargetCommented() */` é **imune** ao
/// falso positivo. O regex matcher encontra o padrão mas o validador
/// AST classifica como `Comment` e descarta.
///
/// Já `fn TargetActive()` na linha seguinte é resolvido normalmente.
#[test]
fn test_symbol_comment_protection() {
    let tmp = TempDir::new().expect("cria tempdir");
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).expect("cria src/");

    // Linha 1: comentário com `fn TargetCommented()` (deve ser IGNORADO).
    // Linha 3: declaração ativa `fn TargetActive()` (deve ser ENCONTRADA).
    let path = src_dir.join("mod.rs");
    let content = "/* fn TargetCommented() */\n\
                   // also commented: fn TargetCommented()\n\
                   fn TargetActive() {}\n";
    fs::write(&path, content).expect("escreve mod.rs");

    // Subcontrato A: `TargetCommented` só existe em comentários → NotFound.
    let commented = resolve_symbol(tmp.path(), "TargetCommented")
        .expect("resolve_symbol não deve panic");
    assert!(
        commented.is_none(),
        "TargetCommented está APENAS em comentários — DEVE ser NotFound, got {:?}",
        commented
    );

    // Subcontrato B: `TargetActive` é declaração real → Resolvido na linha 3.
    let active = resolve_symbol(tmp.path(), "TargetActive")
        .expect("resolve_symbol não deve panic");
    let loc = active.expect("TargetActive deve ser encontrado");
    assert_eq!(loc.kind, SymbolKind::Fn);
    assert_eq!(
        loc.line, 3,
        "TargetActive está na linha 3 (após 2 linhas de comentário), got {}",
        loc.line
    );
    assert_eq!(
        loc.file.file_name().and_then(|s| s.to_str()),
        Some("mod.rs")
    );
}

/// Caso 3: símbolos inexistentes e arquivos corrompidos retornam
/// `SymbolError::NotFound` ou `Ok(None)` graciosamente, sem panic.
///
/// O reator MCP **nunca** pode crashar por input patológico.
#[test]
fn test_symbol_empty_or_invalid_workspace() {
    let tmp = TempDir::new().expect("cria tempdir");

    // Subcontrato A: workspace vazio (sem arquivos) → NotFound, sem panic.
    let empty = resolve_symbol(tmp.path(), "GhostSymbol")
        .expect("workspace vazio retorna Ok(NotFound), não Err");
    assert!(
        empty.is_none(),
        "símbolo inexistente em workspace vazio deve ser None, got {:?}",
        empty
    );

    // Subcontrato B: arquivo corrompido (bytes binários não-UTF8) é
    // pulado via `read_to_string` (fail-soft), sem propagar erro.
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).expect("cria src/");
    let corrupt_path = src_dir.join("corrupt.rs");
    // Sequência binária inválida UTF-8: 0xFF 0xFE 0xFD ...
    fs::write(&corrupt_path, [0xFF_u8, 0xFE, 0xFD, 0x00, 0x01, 0x02])
        .expect("escreve arquivo corrompido");

    // Tentar resolver qualquer símbolo sobre workspace com arquivo binário
    // não pode panic. Deve retornar Ok(None) (nenhum match).
    let corrupt_result = resolve_symbol(tmp.path(), "AnySymbol")
        .expect("arquivo corrompido NÃO pode propagar Err (fail-soft)");
    assert!(
        corrupt_result.is_none(),
        "arquivo binário não pode produzir match, got {:?}",
        corrupt_result
    );

    // Subcontrato C: `name` vazio → SymbolError::InvalidInput (validação rígida).
    let empty_name = resolve_symbol(tmp.path(), "")
        .expect_err("nome vazio DEVE ser rejeitado como InvalidInput");
    match empty_name {
        SymbolError::InvalidInput(msg) => {
            assert!(
                msg.contains("vazio") || msg.contains("empty"),
                "mensagem deve indicar 'vazio'/'empty', got: {msg}"
            );
        }
        other => panic!("esperava SymbolError::InvalidInput, got {other:?}"),
    }

    // Subcontrato D: nome > 256 chars → InvalidInput (proteção DoS).
    let long_name = "a".repeat(257);
    let too_long = resolve_symbol(tmp.path(), &long_name)
        .expect_err("nome > 256 chars DEVE ser rejeitado");
    assert!(
        matches!(too_long, SymbolError::InvalidInput(_)),
        "nome longo deve ser InvalidInput, got {too_long:?}"
    );
}
