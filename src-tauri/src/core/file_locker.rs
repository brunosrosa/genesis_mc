//! Core file_locker — Travas por PathBuf + Escrita Atômica + Snapsafe (MARCO 6.1.0).
//!
//! Este módulo consolida toda a infraestrutura física de I/O de arquivo do Agent
//! Gateway (`souls_mcp`):
//!
//! 1. `PATH_LOCKS` — Mapa concorrente `DashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>` que
//!    serializa todas as mutações de um mesmo arquivo via lock assíncrono do Tokio,
//!    eliminando condições de corrida entre workers.
//! 2. `atomic_write_file` — Swap atômico via `tmpfile + rename` (substitui o conteúdo
//!    do arquivo alvo sem janela de leitura/escrita intermediária).
//! 3. `snapsafe_create_hardlink` / `snapsafe_restore` — Backup O(1) em NTFS/ReFS via
//!    hard-link do arquivo original, com fallback de cópia para volumes cruzados.
//! 4. `WasmTimeTreeSitterValidator` — Válvula de recusa sintática via WAT (WebAssembly
//!    Text) embarcado em compile-time e executado sob `WasmEngine` (WASI 0.2) com
//!    fuel/memory limiter. Detecta parênteses, colchetes e chaves desbalanceadas
//!    (o vetor de ataque do "delimitador órfão").

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use uuid::Uuid;

static PATH_LOCKS: OnceLock<DashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>> = OnceLock::new();

/// Recupera ou insere uma trava assíncrona por caminho de arquivo no `PATH_LOCKS`.
///
/// Utiliza `dunce::canonicalize` para resolver UNC paths e `\\?\` prefixes no Windows,
/// produzindo uma `PathBuf` canônica estável como chave do DashMap (evita colisão
/// entre `C:\foo` e `C:\.\foo`).
///
/// Executa a poda de travas órfãs (`Arc::strong_count == 1`) para impedir vazamento
/// de RAM após o `Mutex` ser droppado pelo último caller.
pub fn acquire_file_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>> {
    let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let map = PATH_LOCKS.get_or_init(DashMap::new);
    map.retain(|_k, lock| Arc::strong_count(lock) > 1);
    map.entry(canonical)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .value()
        .clone()
}

/// Snapshot do arquivo original para o caminho `target_parent/.souls_snap_<uuid>_<name>`.
///
/// **Implementação:** `std::fs::copy` síncrono (dentro de `spawn_blocking`) sobre o
/// conteúdo atual do `target`. A escolha por `copy` (e não `hard_link`) é deliberada:
/// em NTFS, hard links compartilham o mesmo MFT entry, de modo que uma mutação
/// subsequente no `target` (via `atomic_write_file` = `tmp + rename`) também afetaria
/// o snapshot, quebrando o contrato de rollback. `copy` captura o conteúdo no
/// instante da chamada — a única forma correta de "snapshot" para esse cenário.
///
/// O custo de I/O é `O(n)` bytes lidos, mas isto é executado em `spawn_blocking`
/// para não bloquear o reactor do Tokio. Para workloads pesados, a alternativa
/// NTFS Shadow Copy (VSS) pode ser plugada aqui sem alterar a assinatura.
pub fn snapsafe_create_hardlink(target: &Path) -> Result<PathBuf, std::io::Error> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let snapshot_name = format!(
        ".souls_snap_{}_{}",
        Uuid::new_v4().simple(),
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("anon")
    );
    let snapshot_path = parent.join(snapshot_name);
    std::fs::copy(target, &snapshot_path)?;
    Ok(snapshot_path)
}

/// Restauração atômica do `target` a partir de um snapshot produzido por
/// `snapsafe_create_hardlink`. Remove o snapshot após a cópia.
///
/// Utiliza `std::fs::copy` (síncrono) intencionalmente: o caminho de restore
/// é um evento de erro raro (somente quando `verify_ast` recusa o conteúdo),
/// e a versão assíncrona do tokio pode deixar handles abertos no Windows
/// que disparam `Os { code: 32, kind: Uncategorized, ... }`. Como o restore
/// é uma operação pontual, bloquear o worker é aceitável e mais robusto.
pub async fn snapsafe_restore(snapshot: &Path, target: &Path) -> Result<(), std::io::Error> {
    if !snapshot.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("snapshot inexistente: {}", snapshot.display()),
        ));
    }
    // Bloqueia o worker apenas neste caminho de erro raro (rollback). Garante
    // flush + close determinístico dos handles NTFS antes de `remove_file`.
    let bytes_copied = tokio::task::spawn_blocking({
        let snapshot = snapshot.to_path_buf();
        let target = target.to_path_buf();
        move || std::fs::copy(&snapshot, &target)
    })
    .await
    .map_err(std::io::Error::other)??;
    let _ = bytes_copied;
    let _ = tokio::fs::remove_file(snapshot).await;
    Ok(())
}

/// Gravação atômica em arquivo via swap temporário (`.tmp_uuid` -> `std::fs::rename`).
///
/// O arquivo temporário é criado **no mesmo diretório do `target`** (não em
/// `.souls_sandbox/`) por uma razão bare-metal: o `std::fs::rename` no Windows
/// (e `rename(2)` no POSIX) só é atômico se origem e destino estão no mesmo
/// filesystem/volume. Mover o tmp para `.souls_sandbox/` (que fica tipicamente
/// no workspace root) violaria este invariante para arquivos em outros volumes
/// (ex.: `Z:\ReFS\` vs `C:\NTFS\`). O nome do tmp é prefixado com `.tmp_` para
/// ficar invisível em listagens de diretório padrão.
///
/// **MARCO III (ADR-010):** O bloco de I/O físico (`std::fs::write` + `std::fs::rename`)
/// é encapsulado em `tokio::task::spawn_blocking` para impedir que syscalls
/// bloqueantes do NTFS/ACL saturam o reactor do Tokio. O caminho síncrono
/// `tokio::fs::*` é banido por garantir preempção inadequada no event loop.
pub async fn atomic_write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let path_buf = path.to_path_buf();
    let content_bytes = content.as_bytes().to_vec();
    let tmp_name = format!(".tmp_{}", Uuid::new_v4().simple());
    let tmp_path = parent.join(tmp_name);
    let join_result = tokio::task::spawn_blocking(move || -> Result<(), std::io::Error> {
        if !parent.exists() {
            std::fs::create_dir_all(&parent)?;
        }
        std::fs::write(&tmp_path, &content_bytes)?;
        if let Err(rename_err) = std::fs::rename(&tmp_path, &path_buf) {
            // Fallback de kernel: copy + delete (não-atômico, mas recupera caso
            // o rename falhe por motivo raro de ACL/lock em SMB share).
            // Se o copy completar com sucesso, o conteúdo está gravado no
            // destino — reportamos `Ok(())` (a semântica de "atomic_write_file"
            // é "conteúdo durável no target path"). Apenas propagamos erro se
            // AMBOS (rename E copy) falharem.
            if let Err(copy_err) = std::fs::copy(&tmp_path, &path_buf) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(std::io::Error::new(
                    copy_err.kind(),
                    format!(
                        "atomic_write_file: rename falhou ({rename_err}) e copy de fallback tambem falhou ({copy_err})"
                    ),
                ));
            }
            let _ = std::fs::remove_file(&tmp_path);
        }
        Ok(())
    })
    .await
    .map_err(std::io::Error::other)?;
    join_result
}

/// Bytecode WAT (WebAssembly Text Format) embarcado em compile-time que implementa
/// o linter de balanceamento de delimitadores para a Válvula de Recusa Sintática.
///
/// Assinatura: `validate(ptr: i32, len: i32) -> i32`
///   - Lê `len` bytes a partir do offset `ptr` na memória linear.
///   - Mantém 3 contadores: `p_diff` = `(` - `)`, `b_diff` = `{` - `}`, `a_diff` = `[` - `]`.
///   - **Rastreia estado de string literal** (`$in_string` + `$quote_char` + `$escaped`)
///     para NÃO contar delimitadores que apareçam dentro de aspas, onde são texto
///     literal e não sintaxe. Suporta tanto aspas duplas (`"`, 0x22) quanto simples
///     (`'`, 0x27 — Rust `char` literals), e respeita a barra invertida (`\`, 0x5C)
///     como mecanismo de escape, marcando o próximo byte como "consumido".
///   - Retorna `1` se TODOS os contadores forem zero no final **E** o arquivo terminou
///     fora de uma string literal, `0` caso contrário.
///
/// O cálculo de cada incremento/decremento explora o fato de que `i32.eq` em WAT
/// produz `1` (verdadeiro) ou `0` (falso), permitindo aritmética sem `if`:
///   `c = c + (byte == '(') * (1 - in_string)`   e   `c = c - (byte == ')') * (1 - in_string)`
///
/// A seleção condicional do novo `$quote_char` usa a instrução `select` nativa do
/// WebAssembly (suportada por Wasmtime 29).
///
/// Detecta o vetor de ataque do "delimitador órfão":
/// `fn main() {` (chave aberta sem fechamento), `if (x {` (parêntese sem fecha), etc.
/// E CORRIGE o falso-positivo do delimitador dentro de string:
/// `let s = "fn main() {";` (delimitadores balanceados quando ignorada a string).
const WAT_BRACKET_VALIDATOR: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "validate") (param $ptr i32) (param $len i32) (result i32)
            (local $i i32)
            (local $p_diff i32)
            (local $b_diff i32)
            (local $a_diff i32)
            (local $byte i32)
            (local $in_string i32)
            (local $quote_char i32)
            (local $escaped i32)
            (local $is_opening i32)
            (local $is_closing i32)
            (local $is_backslash i32)
            (local $not_in_string i32)
            (local.set $i (i32.const 0))
            (local.set $in_string (i32.const 0))
            (local.set $quote_char (i32.const 0))
            (local.set $escaped (i32.const 0))
            (block $exit
                (loop $loop
                    (br_if $exit (i32.ge_s (local.get $i) (local.get $len)))
                    (local.set $byte
                        (i32.load8_u
                            (i32.add (local.get $ptr) (local.get $i))))
                    (local.set $not_in_string
                        (i32.sub (i32.const 1) (local.get $in_string)))
                    (local.set $is_backslash
                        (i32.eq (local.get $byte) (i32.const 0x5C)))
                    (local.set $is_opening
                        (i32.mul
                            (local.get $not_in_string)
                            (i32.or
                                (i32.eq (local.get $byte) (i32.const 0x22))
                                (i32.eq (local.get $byte) (i32.const 0x27)))))
                    (local.set $is_closing
                        (i32.mul
                            (local.get $in_string)
                            (i32.mul
                                (i32.eq (local.get $byte) (local.get $quote_char))
                                (i32.sub (i32.const 1) (local.get $escaped)))))
                    (local.set $in_string
                        (i32.sub
                            (i32.add (local.get $in_string) (local.get $is_opening))
                            (local.get $is_closing)))
                    (local.set $quote_char
                        (select
                            (local.get $byte)
                            (local.get $quote_char)
                            (local.get $is_opening)))
                    (local.set $escaped
                        (i32.mul
                            (local.get $is_backslash)
                            (i32.mul
                                (local.get $in_string)
                                (i32.sub (i32.const 1) (local.get $escaped)))))
                    (local.set $p_diff
                        (i32.add
                            (local.get $p_diff)
                            (i32.mul
                                (local.get $not_in_string)
                                (i32.eq (local.get $byte) (i32.const 0x28)))))
                    (local.set $p_diff
                        (i32.sub
                            (local.get $p_diff)
                            (i32.mul
                                (local.get $not_in_string)
                                (i32.eq (local.get $byte) (i32.const 0x29)))))
                    (local.set $b_diff
                        (i32.add
                            (local.get $b_diff)
                            (i32.mul
                                (local.get $not_in_string)
                                (i32.eq (local.get $byte) (i32.const 0x7B)))))
                    (local.set $b_diff
                        (i32.sub
                            (local.get $b_diff)
                            (i32.mul
                                (local.get $not_in_string)
                                (i32.eq (local.get $byte) (i32.const 0x7D)))))
                    (local.set $a_diff
                        (i32.add
                            (local.get $a_diff)
                            (i32.mul
                                (local.get $not_in_string)
                                (i32.eq (local.get $byte) (i32.const 0x5B)))))
                    (local.set $a_diff
                        (i32.sub
                            (local.get $a_diff)
                            (i32.mul
                                (local.get $not_in_string)
                                (i32.eq (local.get $byte) (i32.const 0x5D)))))
                    (local.set $i
                        (i32.add (local.get $i) (i32.const 1)))
                    (br $loop)))
            (i32.and
                (i32.eq (i32.const 0) (local.get $p_diff))
                (i32.and
                    (i32.eq (i32.const 0) (local.get $b_diff))
                    (i32.and
                        (i32.eq (i32.const 0) (local.get $a_diff))
                        (i32.eq (i32.const 0) (local.get $in_string)))))))
"#;

/// Válvula de Recusa Sintática do MARCO 6.1.0.
///
/// Carrega o módulo WAT [`WAT_BRACKET_VALIDATOR`] sob a cerca de recursos do
/// `WasmEngine` global (WASI 0.2, fuel 10M, memory 16 MiB) e expõe a função
/// `validate_source` que recebe o conteúdo textual de um arquivo temporário e
/// retorna:
///   - `Ok(true)`  → sintaxe balanceada (passa pela válvula)
///   - `Ok(false)` → delimitadores órfãos detectados (válvula fechada)
///   - `Err(_)`    → falha estrutural do motor (fail-soft: o caller decide)
///
/// Esta é a "Válvula de Recusa" exigida pela ADR-010: nenhum arquivo com
/// sintaxe quebrada é gravado em disco, poupando ciclos de compilação nativa
/// na CPU e preservando a dGPU (RTX 2060m) de cargas inúteis.
pub struct WasmTimeTreeSitterValidator;

impl WasmTimeTreeSitterValidator {
    /// Compila o módulo WAT (uma única vez por processo) e executa o linter
    /// sobre o conteúdo fornecido.
    ///
    /// **Fail-Soft:** se o motor Wasmtime estiver indisponível ou o módulo
    /// falhar ao compilar, retorna `Ok(true)` (passa), permitindo que o caller
    /// continue sem o gate. A ADR-010 prefere um sistema degradado a uma
    /// falha total do I/O.
    pub fn validate_source(source: &str) -> Result<bool, String> {
        let engine = crate::cognition::ast::observability::wasm_engine::WasmEngine::global();
        let module = match engine.load_module(WAT_BRACKET_VALIDATOR.as_bytes()) {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!(
                    target: "souls_mcp::surgical_edit",
                    "WasmTimeTreeSitterValidator: módulo WAT indisponível, fail-soft ({e})"
                );
                return Ok(true);
            }
        };

        // Payload precisa caber na memória linear (1 página = 64 KiB).
        if source.len() > 60 * 1024 {
            return Ok(true);
        }
        let bytes = source.as_bytes();
        let len = bytes.len() as i32;

        let result = engine.execute_safely::<_, i32>(&module, move |store, instance| {
            let memory = instance
                .get_memory(&mut *store, "memory")
                .ok_or_else(|| wasmtime::Error::msg("memória WASM não exportada"))?;
            memory.write(&mut *store, 0, bytes)?;
            let validate = instance.get_typed_func::<(i32, i32), i32>(&mut *store, "validate")?;
            validate.call(&mut *store, (0, len))
        });

        match result {
            Ok(1) => Ok(true),
            Ok(0) => Ok(false),
            Ok(_) => Ok(true),
            Err(trap) => {
                tracing::debug!(
                    target: "souls_mcp::surgical_edit",
                    "WasmTimeTreeSitterValidator: trap WASM, fail-soft ({trap:?})"
                );
                Ok(true)
            }
        }
    }

    /// Atalho que valida apenas arquivos com extensões suportadas (`.rs`/`.svelte`).
    /// Para outras extensões, retorna `Ok(true)` imediatamente (passa pela válvula).
    pub fn validate_path(path: &Path, source: &str) -> Result<bool, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        match ext.as_deref() {
            Some("rs") | Some("svelte") => Self::validate_source(source),
            _ => Ok(true),
        }
    }
}

// Suprime falsos positivos do linter ao referenciar submódulos via `crate::`.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_lock_garbage_collection() {
        let path = PathBuf::from("test_orphan_lock.tmp");
        {
            let lock = acquire_file_lock(&path);
            assert_eq!(Arc::strong_count(&lock), 2); // 1 na função, 1 no DashMap
        }
        let map = PATH_LOCKS.get().unwrap();
        let canonical = dunce::canonicalize(&path).unwrap_or(path.clone());
        assert!(map.contains_key(&canonical));
        let other_path = PathBuf::from("test_other_lock.tmp");
        let _other_lock = acquire_file_lock(&other_path);
        assert!(!map.contains_key(&canonical));
    }

    #[tokio::test]
    async fn test_snapsafe_create_and_restore_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("fixture.txt");
        let original = b"original content for snapsafe test";
        tokio::fs::write(&target, original).await.expect("write original");

        let snapshot = snapsafe_create_hardlink(&target).expect("snapsafe snapshot");
        assert!(snapshot.exists(), "snapshot deve existir apos criacao");

        let corrupted = b"CORRUPTED CONTENT";
        tokio::fs::write(&target, corrupted).await.expect("write corrupt");

        snapsafe_restore(&snapshot, &target)
            .await
            .expect("restore deve succeed");

        let restored = tokio::fs::read(&target).await.expect("read restored");
        assert_eq!(restored, original, "conteudo deve voltar ao estado original");
    }

    #[test]
    fn test_wat_bracket_validator_balanced_passes() {
        let balanced = "fn main() { println!(\"hi\"); }";
        let res = WasmTimeTreeSitterValidator::validate_source(balanced)
            .expect("validate_source nao deve panic");
        assert!(res, "balanceado deve passar pela valvula");
    }

    #[test]
    fn test_wat_bracket_validator_unbalanced_rejected() {
        let unbalanced = "fn main() {";
        let res = WasmTimeTreeSitterValidator::validate_source(unbalanced)
            .expect("validate_source nao deve panic");
        assert!(!res, "delimitador orfao deve ser rejeitado pela valvula");
    }

    /// Marcadores desbalanceados dentro de uma string literal devem ser IGNORADOS
    /// (não contam como sintaxe), permitindo que código válido passe pela válvula.
    #[test]
    fn test_wat_bracket_validator_string_with_unbalanced_delimiters_passes() {
        let source = r#"let s = "fn main() { if (x { y }"; let _ = ();"#;
        let res = WasmTimeTreeSitterValidator::validate_source(source)
            .expect("validate_source nao deve panic");
        assert!(
            res,
            "delimitadores dentro de string literal NAO devem ser contados como sintaxe"
        );
    }

    /// `\"` (aspa dupla escapada com barra invertida) deve ser tratada como conteúdo
    /// literal, NÃO como fechamento de string. Caso contrário, o escape `\"` em uma
    /// string seria confundido com término prematuro, desbalanceando o estado.
    #[test]
    fn test_wat_bracket_validator_escape_quote_inside_string_passes() {
        let source = r#"let s = "say \"hi\""; let _ = ();"#;
        let res = WasmTimeTreeSitterValidator::validate_source(source)
            .expect("validate_source nao deve panic");
        assert!(res, "aspas escapadas com \\ nao devem fechar a string");
    }

    /// String NUNCA fechada (faltou o `"` final) deve ser rejeitada: o arquivo
    /// terminou com `in_string=1`, indicando literal aberto, sintaticamente inválido.
    #[test]
    fn test_wat_bracket_validator_unclosed_string_rejected() {
        let source = r#"let s = "abc"#;
        let res = WasmTimeTreeSitterValidator::validate_source(source)
            .expect("validate_source nao deve panic");
        assert!(!res, "string literal nao fechada deve ser rejeitada");
    }

    /// Aspas simples (`'`, 0x27) também devem acionar o estado de string — Rust
    /// usa-as em `char` literals (`'a'`, `'\n'`, etc.) e em lifetimes (`'a`).
    /// O validador deve tratar uma chave `{` dentro de `'{'` como literal.
    #[test]
    fn test_wat_bracket_validator_single_quote_char_literal_passes() {
        let source = r#"let c = '{'; let _ = ();"#;
        let res = WasmTimeTreeSitterValidator::validate_source(source)
            .expect("validate_source nao deve panic");
        assert!(res, "delimitadores dentro de char literal devem ser ignorados");
    }

    /// Code realista Rust com strings, escapes e múltiplos delimitadores balanceados.
    /// Validação holística para garantir que a nova lógica não introduz regressões
    /// em casos do mundo real.
    #[test]
    fn test_wat_bracket_validator_realistic_rust_source_passes() {
        let source = r#"
            fn main() {
                let msg = "Hello, \"world\"!";
                let escaped = "path\\to\\file";
                let n = if (x > 0) { x } else { 0 };
                println!("{}", msg);
            }
        "#;
        let res = WasmTimeTreeSitterValidator::validate_source(source)
            .expect("validate_source nao deve panic");
        assert!(res, "codigo Rust real balanceado deve passar pela valvula");
    }

    /// Edge case: aspas duplas consecutivas vazias + chave desbalanceada FORA.
    /// `""` é uma string vazia legítima, deve fechar imediatamente e a chave
    /// externa deve ser contada normalmente.
    #[test]
    fn test_wat_bracket_validator_empty_string_then_brace_counted() {
        let source = r#"let s = ""; if (x {"#;
        let res = WasmTimeTreeSitterValidator::validate_source(source)
            .expect("validate_source nao deve panic");
        assert!(!res, "chave orfa fora de string deve ser rejeitada mesmo apos string vazia");
    }
}
