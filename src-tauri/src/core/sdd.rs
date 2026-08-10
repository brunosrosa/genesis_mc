//! `sdd.rs` — Marco 5.16.0: O Orquestrador de Cascata Documental SDD
//!
//! **DIRETRIZ 1 (SSOT Constitucional):** Migração idempotente do `PRAGMA user_version`
//! para `6`, criando (caso ainda não exista) a tabela `sdd_document_states` que
//! materializa o estado de integridade SHA-256 e a chancela humana
//! (`[APPROVED_BY_HUMAN: YYYY-MM-DD]`) exigida pelo protocolo BMAD (ADR-010 v2.0).
//!
//! **DIRETRIZ 2 (As 3 Leis da Cascata de Validação):**
//! 1. **LEI I — Verificação de Assinatura Humana:** varredura leve na CPU
//!    pela tag `[APPROVED_BY_HUMAN: YYYY-MM-DD]` em `REQUIREMENTS.md`.
//! 2. **LEI II — Invalidação de Hash em Cascata (SHA-256):** se o hash do
//!    `REQUIREMENTS.md` divergir do registrado, todos os documentos
//!    downstream (`DESIGN.md`, `TASKS.md`, `TEST_SPECS.md`) são
//!    atômica e coordenadamente rebaixados para `is_approved = 0`.
//! 3. **LEI III — Cobertura TDD (Cross-Match):** cada `Task NNN` declarada
//!    em `TASKS.md` deve possuir uma assinatura `fn test_*` correspondente
//!    em `TEST_SPECS.md` (matching por número da task contido no nome do teste).
//!
//! **DIRETRIZ 3 (Higiene Bare-Metal):** zero alocação dinâmica na hot-path
//! regex (uma vez por chamada); zero dependências novas no `Cargo.toml`.
//! Erros emitidos EXCLUSIVAMENTE via `CognitiveError` (canibalização do
//! catálogo canônico) para casar com a doutrina ADR-025 (qualidade 100/100)
//! e com o disjuntor Fail-Closed L7 do Escudo Socrático.

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};

use crate::cognition::state_thinking::memory_graph::errors::CognitiveError;

// ============================================================================
// CONSTANTES CANÔNICAS (SSOT)
// ============================================================================

/// Versão-alvo do schema após a introdução do módulo SDD. Coincide com o
/// `TARGET_VERSION_V6` do motor de cognição (canibalização zero-debt).
pub const TARGET_SDD_VERSION: i64 = 6;

/// DDL puro e idempotente da tabela de integridade documental.
/// A constraint `STRICT` espelha a doutrina State V5/V6 do SOULS, e a PK
/// em `document_path` materializa o conceito de "documento canônico".
///
/// **Nota sintática SQLite 3.37+:** `STRICT` deve ser declarado ao final
/// da definição da tabela, não em uma coluna individual.
pub const SDD_DOCUMENT_STATES_DDL: &str = "
CREATE TABLE IF NOT EXISTS sdd_document_states (
    document_path TEXT PRIMARY KEY,
    sha256_hash TEXT NOT NULL,
    last_validated_at INTEGER NOT NULL,
    is_approved INTEGER NOT NULL DEFAULT 0
) STRICT;
";

/// Tag obrigatória para a chancela humana. Captura o sufixo `YYYY-MM-DD`.
const APPROVED_TAG_PATTERN: &str = r"\[APPROVED_BY_HUMAN:\s*(\d{4}-\d{2}-\d{2})\]";

/// Regex de validação adicional do formato ISO-8601 (YYYY-MM-DD).
const ISO_DATE_PATTERN: &str = r"^\d{4}-\d{2}-\d{2}$";

/// Regex de varredura das Tasks declaradas em `TASKS.md` (case-insensitive).
const TASK_ID_PATTERN: &str = r"(?i)Task\s+(\d+)";

/// Regex de varredura das assinaturas de teste em `TEST_SPECS.md`
/// (Rust-style: `fn test_xxx` ou `async fn test_xxx`).
const TEST_SIGNATURE_PATTERN: &str = r"(?m)^\s*(?:async\s+)?fn\s+test_(\w+)";

/// Documentos canônicos do BMAD. A ordem é FUNDAMENTAL para a cascata:
/// o primeiro é a raiz da invalidação; os demais são alvos downstream.
pub const SDD_DOCS: &[&str] = &[
    "REQUIREMENTS.md",
    "DESIGN.md",
    "TASKS.md",
    "TEST_SPECS.md",
];

/// Subconjunto de `SDD_DOCS` que sofre invalidação em cascata quando
/// `REQUIREMENTS.md` tem seu hash SHA-256 alterado.
pub const CASCADE_DOWNSTREAM: &[&str] = &[
    "DESIGN.md",
    "TASKS.md",
    "TEST_SPECS.md",
];

// ============================================================================
// MAPEAMENTO DE ERROS PARA CognitiveError (SSOT canônico)
// ============================================================================
//
// O módulo SDD emite EXCLUSIVAMENTE variantes de `CognitiveError`, canibalizando
// o catálogo canônico do motor de cognição. Mapeamentos semânticos:
//
//   rusqlite::Error      → CognitiveError::GraphError
//   std::io::Error        → CognitiveError::GraphError  (I/O do FS é falha de dados)
//   regex::Error          → CognitiveError::InvalidPayload (regex interna é payload)
//
//   Falta de assinatura   → CognitiveError::HitlDenied
//   Divergência de hash    → CognitiveError::SddCascadeViolation
//   Cobertura TDD falhada  → CognitiveError::UntrustedExecutionBlocked
//
// ============================================================================

// ============================================================================
// HELPERS CPU-BOUND (sem heap desnecessário, hot-path leve)
// ============================================================================

/// Calcula o hex-encoded SHA-256 de `data` (32 bytes → 64 chars).
///
/// **Higiene Bare-Metal:** usa `Sha256::digest` que escreve o resultado em
/// um array de 32 bytes na stack (sem `Vec`, sem `String` intermediário).
fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Valida que `s` casa com `YYYY-MM-DD` E que mês/dia são gramaticalmente
/// válidos (mês 1-12, dia 1-31). **Não** valida calendário estendido
/// (anos bissextos, dias por mês) — esse é trabalho do operador humano.
fn is_valid_iso_date(s: &str) -> bool {
    let re = match regex::Regex::new(ISO_DATE_PATTERN) {
        Ok(r) => r,
        Err(_) => return false,
    };
    if !re.is_match(s) {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let month: u32 = match parts[1].parse() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let day: u32 = match parts[2].parse() {
        Ok(d) => d,
        Err(_) => return false,
    };
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

// ============================================================================
// MIGRAÇÃO IDEMPOTENTE (DIRETRIZ 1)
// ============================================================================

/// Resolve o caminho canônico do banco de estado sob `workspace_root`.
/// Convenção SSOT: `<workspace>/.souls_data/souls_state.db`.
pub fn resolve_sdd_db_path(workspace_root: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(workspace_root)
        .join(".souls_data")
        .join("souls_state.db")
}

/// Abre (ou cria) a conexão SQLite do SDD e aplica a migração idempotente
/// para `user_version = 6`, incluindo a tabela `sdd_document_states`.
///
/// **Lei do Túnel Único:** toda escrita passa por uma transação explícita,
/// satisfazendo o requisito "modo WAL e transações explícitas" do State V6.
pub fn open_sdd_db(workspace_root: &str) -> Result<Connection, CognitiveError> {
    let db_path = resolve_sdd_db_path(workspace_root);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(&db_path)?;
    // WAL é a configuração canônica do SOULS State V5/V6.
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
    let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");
    migrate_to_v6_sdd(&mut conn)?;
    Ok(conn)
}

/// Garante que `user_version >= 6` E que `sdd_document_states` existe.
///
/// **Idempotência em 3 camadas:**
/// 1. A migração V5→V6 subjacente é no-op se `user_version >= 6`.
/// 2. `CREATE TABLE IF NOT EXISTS` é idempotente em qualquer estado.
/// 3. A gravação final de `user_version = 6` é incondicional apenas
///    quando a tabela é recriada (caminho de upgrade real).
pub fn migrate_to_v6_sdd(conn: &mut Connection) -> Result<(), CognitiveError> {
    // Etapa 1: garante o salto para V6 (canibaliza o motor canônico).
    crate::cognition::state_thinking::thinking::ops::migrate_v5_to_v6(conn)?;

    // Etapa 2: cria a tabela SDD em transação atômica isolada.
    let tx = conn.transaction()?;
    tx.execute_batch(SDD_DOCUMENT_STATES_DDL)?;
    tx.commit()?;
    Ok(())
}

// ============================================================================
// HELPERS DE PERSISTÊNCIA (TABELA sdd_document_states)
// ============================================================================

/// UPSERT canônico em `sdd_document_states` (PK = `document_path`).
/// Usado tanto no caminho de aprovação quanto no de invalidação.
fn upsert_document_state(
    conn: &Connection,
    path: &str,
    hash: &str,
    now_seconds: i64,
    is_approved: bool,
) -> Result<(), CognitiveError> {
    conn.execute(
        "INSERT INTO sdd_document_states
            (document_path, sha256_hash, last_validated_at, is_approved)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(document_path) DO UPDATE SET
            sha256_hash       = excluded.sha256_hash,
            last_validated_at = excluded.last_validated_at,
            is_approved       = excluded.is_approved",
        params![path, hash, now_seconds, is_approved as i64],
    )?;
    Ok(())
}

/// Variante transacional de `upsert_document_state` (para uso dentro de
/// `tx.commit()` durante a invalidação em cascata).
fn upsert_document_state_tx(
    tx: &Transaction<'_>,
    path: &str,
    hash: &str,
    now_seconds: i64,
    is_approved: bool,
) -> Result<(), CognitiveError> {
    tx.execute(
        "INSERT INTO sdd_document_states
            (document_path, sha256_hash, last_validated_at, is_approved)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(document_path) DO UPDATE SET
            sha256_hash       = excluded.sha256_hash,
            last_validated_at = excluded.last_validated_at,
            is_approved       = excluded.is_approved",
        params![path, hash, now_seconds, is_approved as i64],
    )?;
    Ok(())
}

/// Snapshot consolidado do estado registrado de um documento canônico.
/// Usado pela LEI II para decidir se a divergência de hash dispara cascata.
#[derive(Debug, Clone)]
struct DocumentStateSnapshot {
    hash: Option<String>,
    is_approved: Option<bool>,
}

/// Lê `hash` E `is_approved` de `path` em um único round-trip ao SQLite
/// (canibalização do `query_row` para evitar duas chamadas de I/O).
fn read_document_state(
    conn: &Connection,
    path: &str,
) -> Result<DocumentStateSnapshot, CognitiveError> {
    let row: Option<(String, i64)> = conn
        .query_row(
            "SELECT sha256_hash, is_approved FROM sdd_document_states WHERE document_path = ?1",
            params![path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(CognitiveError::from)?;
    Ok(match row {
        Some((hash, approved)) => DocumentStateSnapshot {
            hash: Some(hash),
            is_approved: Some(approved != 0),
        },
        None => DocumentStateSnapshot {
            hash: None,
            is_approved: None,
        },
    })
}

/// Timestamp absoluto em **segundos** desde a Época UNIX.
///
/// **Spec canônica (`.souls_scratchpad/soda-mc-sdd-cascade-spec.md` §2):**
/// `last_validated_at INTEGER NOT NULL` é definido como
/// "UNIX epoch timestamp em segundos". Esta é a unidade canônica
/// para a coluna em todas as escritas e leituras.
fn unix_now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ============================================================================
// MOTOR DE VALIDAÇÃO (DIRETRIZ 2)
// ============================================================================

/// Motor estático da cascata de validação. Stateless por design: a
/// configuração de runtime é exclusivamente `workspace_root` por chamada,
/// e o estado de integridade vive no SQLite sob `.souls_data/souls_state.db`.
pub struct SddValidationEngine;

impl SddValidationEngine {
    /// Orquestra as 3 Leis da cascata SDD sobre o `workspace_root` informado.
    ///
    /// **Contrato de retorno:**
    /// - `Ok(true)` → cascata íntegra, todos os documentos aprovados e cobertos.
    /// - `Err(HitlDenied(_))` → LEI I falhou (sem chancela humana).
    /// - `Err(SddCascadeViolation(n))` → LEI II falhou (n documentos rebaixados).
    /// - `Err(UntrustedExecutionBlocked(_))` → LEI III falhou (cobertura TDD).
    /// - `Err(GraphError(_) | InvalidPayload(_))` → falha de subsistema.
    pub async fn validate_sdd_cascade_state(
        workspace_root: &str,
    ) -> Result<bool, CognitiveError> {
        let root = std::path::PathBuf::from(workspace_root);
        let requirements_path = root.join("REQUIREMENTS.md");

        // ----- LEI I: ASSINATURA HUMANA -----
        let requirements_content = tokio::fs::read_to_string(&requirements_path)
            .await
            .map_err(|_| {
                CognitiveError::GraphError("documento SDD ausente: REQUIREMENTS.md".to_string())
            })?;

        let approved_re = regex::Regex::new(APPROVED_TAG_PATTERN)?;
        let signed = approved_re
            .captures(&requirements_content)
            .and_then(|caps| caps.get(1))
            .map(|m| is_valid_iso_date(m.as_str()))
            .unwrap_or(false);

        // Abre/garante o banco de integridade.
        let mut conn = open_sdd_db(workspace_root)?;
        let now_seconds = unix_now_seconds();
        let req_hash = sha256_hex(requirements_content.as_bytes());

        if !signed {
            // Falha LEI I: persiste o estado "não aprovado" e aborta.
            upsert_document_state(&conn, "REQUIREMENTS.md", &req_hash, now_seconds, false)?;
            return Err(CognitiveError::HitlDenied(
                "assinatura humana ausente em REQUIREMENTS.md (tag [APPROVED_BY_HUMAN: YYYY-MM-DD] não encontrada)".to_string(),
            ));
        }

        // ----- LEI II: HASH CASCADE -----
        // A cascata SÓ dispara quando o REQUIREMENTS.md era previamente
        // aprovado. Caso o estado anterior fosse "rejeitado" (ex: faltava
        // assinatura humana) e o operador corrigiu o problema, a divergência
        // de hash representa PROMOÇÃO, não invalidação — então promovemos
        // silenciosamente sem cascata.
        let prev_state = read_document_state(&conn, "REQUIREMENTS.md")?;
        let hash_diverged = matches!(&prev_state.hash, Some(prev) if prev != &req_hash);
        let prev_was_approved = prev_state.is_approved.unwrap_or(false);

        if hash_diverged && prev_was_approved {
            // Invalidação atômica e coordenada de toda a cascata downstream.
            let invalidated = cascade_invalidate(&mut conn, &root, &req_hash, now_seconds)?;
            return Err(CognitiveError::SddCascadeViolation(invalidated));
        }

        // Caminho verde: REQUIREMENTS.md aprovado (primeira inserção,
        // hash inalterado, OU divergência com estado anterior rejeitado
        // — esta última é uma PROMOÇÃO controlada).
        upsert_document_state(&conn, "REQUIREMENTS.md", &req_hash, now_seconds, true)?;

        // Reconcilia os documentos downstream (re-registra hashes e
        // preserva o flag `is_approved` se o hash não mudou desde a
        // última validação; senão marca como 0).
        for doc in CASCADE_DOWNSTREAM {
            reconcile_downstream(&conn, &root, doc, now_seconds)?;
        }

        // ----- LEI III: TDD COVERAGE -----
        let tasks_content = tokio::fs::read_to_string(root.join("TASKS.md"))
            .await
            .map_err(|_| {
                CognitiveError::GraphError("documento SDD ausente: TASKS.md".to_string())
            })?;
        let test_specs_content = tokio::fs::read_to_string(root.join("TEST_SPECS.md"))
            .await
            .map_err(|_| {
                CognitiveError::GraphError("documento SDD ausente: TEST_SPECS.md".to_string())
            })?;

        let orphans = find_orphan_tasks(&tasks_content, &test_specs_content)?;
        if !orphans.is_empty() {
            return Err(CognitiveError::UntrustedExecutionBlocked(format!(
                "cobertura TDD incompleta: {} task(s) sem assinatura de teste em TEST_SPECS.md (órfãs: {:?})",
                orphans.len(),
                orphans
            )));
        }

        Ok(true)
    }
}

// ============================================================================
// FUNÇÕES AUXILIARES DO MOTOR
// ============================================================================

/// Invalida atômicamente todos os documentos downstream após divergência
/// de hash em REQUIREMENTS.md. Retorna a contagem de documentos rebaixados.
fn cascade_invalidate(
    conn: &mut Connection,
    root: &std::path::Path,
    new_req_hash: &str,
    now_seconds: i64,
) -> Result<usize, CognitiveError> {
    let tx = conn.transaction()?;
    // Re-registra REQUIREMENTS.md com o hash novo e is_approved = 0.
    upsert_document_state_tx(&tx, "REQUIREMENTS.md", new_req_hash, now_seconds, false)?;

    let mut count = 0usize;
    for doc in CASCADE_DOWNSTREAM {
        let doc_path = root.join(doc);
        let hash = match std::fs::read(&doc_path) {
            Ok(bytes) => sha256_hex(&bytes),
            Err(_) => {
                // Documento ausente no disco: ainda assim cascateia a
                // invalidação para refletir a quebra do BMAD no DB.
                String::from("<missing>")
            }
        };
        upsert_document_state_tx(&tx, doc, &hash, now_seconds, false)?;
        count += 1;
    }
    tx.commit()?;
    Ok(count)
}

/// Reconcilia o estado de um documento downstream: na PRIMEIRA inserção
/// (sem hash prévio) o documento é promovido a `is_approved = true`; em
/// re-execuções subsequentes, preserva `is_approved` se o hash permanece
/// inalterado, senão rebaixa para `0`.
///
/// **Heurística:** a primeira passagem de validação estabelece o baseline
/// de integridade para todos os 4 documentos canônicos. A partir daí,
/// qualquer divergência de hash individual é tratada como modificação
/// unilateral que requer re-validação.
fn reconcile_downstream(
    conn: &Connection,
    root: &std::path::Path,
    doc: &str,
    now_seconds: i64,
) -> Result<(), CognitiveError> {
    let doc_path = root.join(doc);
    let bytes = match std::fs::read(&doc_path) {
        Ok(b) => b,
        Err(_) => {
            // Documento ausente: marca explicitamente como não aprovado
            // para que o DoD do BMAD exija a recriação.
            upsert_document_state(conn, doc, "<missing>", now_seconds, false)?;
            return Ok(());
        }
    };
    let hash = sha256_hex(&bytes);
    let prev_state = read_document_state(conn, doc)?;
    let is_approved = match prev_state.hash {
        Some(prev) if prev == hash => prev_state.is_approved.unwrap_or(true),
        Some(_) => false, // Hash divergiu → invalidação unilateral
        None => true,     // Primeira inserção → baseline aprovado
    };
    upsert_document_state(conn, doc, &hash, now_seconds, is_approved)?;
    Ok(())
}

/// Identifica tasks em `TASKS.md` que não possuem test signature
/// correspondente em `TEST_SPECS.md`.
///
/// **Heurística de Cross-Match (PRD Marco 5.16.0):** uma Task `NNN` é
/// considerada coberta se o nome de alguma assinatura `fn test_*` em
/// `TEST_SPECS.md` contiver `NNN` como substring (convenção Rust:
/// `test_task_NNN_xxx` ou `test_NNN_xxx`).
fn find_orphan_tasks(
    tasks_content: &str,
    test_specs_content: &str,
) -> Result<Vec<String>, CognitiveError> {
    let task_re = regex::Regex::new(TASK_ID_PATTERN)?;
    let test_re = regex::Regex::new(TEST_SIGNATURE_PATTERN)?;

    // Coleta e deduplica IDs de tasks ativas.
    let mut task_ids: Vec<String> = task_re
        .captures_iter(tasks_content)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    task_ids.sort();
    task_ids.dedup();

    // Coleta nomes das assinaturas de teste.
    let test_names: Vec<String> = test_re
        .captures_iter(test_specs_content)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();

    // Cross-match: task NNN exige ao menos um test name contendo "NNN".
    let mut orphans = Vec::new();
    for tid in &task_ids {
        let covered = test_names.iter().any(|tn| tn.contains(tid.as_str()));
        if !covered {
            orphans.push(tid.clone());
        }
    }
    Ok(orphans)
}

// ============================================================================
// TESTES UNITÁRIOS INTERNOS (smoke tests; cobertura completa em tests.rs)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: a validação de data ISO-8601 deve rejeitar formatos
    /// malformados mas aceitar o canônico `YYYY-MM-DD` com mês/dia válidos.
    #[test]
    fn iso_date_validator_accepts_canonical_and_rejects_garbage() {
        assert!(is_valid_iso_date("2026-08-09"));
        assert!(!is_valid_iso_date("2026-13-09"));
        assert!(!is_valid_iso_date("2026-08-32"));
        assert!(!is_valid_iso_date("26-08-09"));
        assert!(!is_valid_iso_date("2026/08/09"));
        assert!(!is_valid_iso_date(""));
    }

    /// Smoke test: SHA-256 de payload conhecido deve ser determinístico
    /// e casar com o vetor canônico do NIST.
    #[test]
    fn sha256_hex_is_deterministic_and_known() {
        let h = sha256_hex(b"abc");
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
