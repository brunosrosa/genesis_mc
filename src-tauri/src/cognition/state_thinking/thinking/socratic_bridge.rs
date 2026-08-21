//! Barramento Assíncrono Socrático (Marco 3.9 Fase E.2).
//!
//! Padrão Orchestrator-Worker para gravações socráticas:
//! - **Produtor** (Tokio runtime, critical path MCP/Tauri): empacota
//!   `SocraticOp` e despacha via `mpsc::Sender::try_send` (HIPER-FORWARD).
//!   Nunca bloqueia esperando ACK. Se o bounded channel estiver saturado,
//!   `try_send` retorna `TrySendError::Full` e o produtor decide entre
//!   log+drop (modo audit) ou propagar erro (modo estrito).
//! - **Consumidor** (`std::thread` dedicada): loop `blocking_recv`,
//!   aplica `migrate_v3_to_v5` no boot, executa cada `SocraticOp` no
//!   SQLite WAL de forma sequencial. Idempotente (todas as variantes
//!   usam `INSERT OR REPLACE`).
//!
//! **Lei do NUNCA-BLOQUEAR:** `try_send` é o disjuntor (backpressure).
//! Bounded(512) = 512 mensagens em vôo. Acima disso, backoff natural
//! para o produtor.
//!
//! **Compatibilidade:** espelha o padrão de `memory_graph::mpsc_bridge`,
//! mas com capacity 512 (não 100) para absorver o cenário de stress
//! de 10k pensamentos do Marco 3.9 Fase E.2.

use crate::cognition::thinking::ops;
use crate::cognition::thinking::persistence::SocraticThought;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// Capacidade do canal MPSC. 512 mensagens = ~10k pensamentos podem ser
/// despachados em rajadas de até 512 antes do backpressure natural.
///
/// Lei do NUNCA-BLOQUEAR: o canal é bounded para que o produtor saiba
/// quando o consumidor está em sobrecarga (em vez de acumular memória
/// indefinidamente).
pub const SOCRATIC_CHANNEL_CAPACITY: usize = 512;

/// Tamanho do lote (micro-batch) que o worker acumula antes de
/// persistir em uma transação única. Em Windows ReFS, cada `fsync`
/// de WAL custa ~1-2ms; com `BATCH_SIZE = 64` reduzimos o overhead
/// de 10k inserts em ~150x.
pub const SOCRATIC_BATCH_SIZE: usize = 64;

/// Tipo de retorno de uma operação síncrona (com ACK) para o produtor.
pub type SocraticResult = Result<Value, String>;

/// Envelope opaco das operações socráticas. Cada variante é processada
/// de forma sequencial pelo `SocraticWriteWorker` (single-thread, sem
/// race em SQLite).
///
/// **Hiper-Forward:** as variantes `*Fire` (sem `reply`) são fire-and-forget
/// — não bloqueiam o produtor esperando ACK. Usar no critical path
/// (e.g., `merge_sessions`).
#[derive(Debug)]
pub enum SocraticOp {
    /// `UpsertSession` síncrono (com ACK). Retorna JSON canônico
    /// confirmando a operação. Usar em fluxos que precisam de auditoria.
    UpsertSession {
        session_id: String,
        created_at: i64,
        metadata: String,
        reply: oneshot::Sender<SocraticResult>,
    },
    /// `UpsertThought` síncrono (com ACK). Idem.
    UpsertThought {
        thought: SocraticThought,
        reply: oneshot::Sender<SocraticResult>,
    },
    /// `UpsertSession` Hiper-Forward (sem ACK). Para o critical path
    /// onde perda tolerável (logs de auditoria).
    UpsertSessionFire {
        session_id: String,
        created_at: i64,
        metadata: String,
    },
    /// `UpsertThought` Hiper-Forward (sem ACK). Idem.
    UpsertThoughtFire {
        thought: SocraticThought,
    },
}

/// Handle público do `SocraticWriteWorker`.
///
/// Encapsula o `Sender` MPSC e um contador atômico de operações
/// despachadas, permitindo que os testes TDD validem o flush completo
/// do canal sem precisar de polling no SQLite.
#[derive(Clone)]
pub struct SocraticWriteHandle {
    tx: mpsc::Sender<SocraticOp>,
    /// Contador atômico de operações processadas pelo worker. Atualizado
    /// a cada `SocraticOp` consumido. Permite `assert!(handle.processed() >= N)`
    /// em testes de stress.
    pub processed: Arc<AtomicUsize>,
}

impl SocraticWriteHandle {
    /// Despacha uma operação via `try_send` (HIPER-FORWARD).
    ///
    /// Retorna `Ok(())` se o envelope foi enfileirado, ou
    /// `Err(Box::new(op))` se o canal está saturado (backpressure natural).
    /// O caller decide entre `log+drop` ou propagar erro.
    ///
    /// **Box no Err:** o tipo de retorno precisa caber em 2 words para
    /// que a função seja trivialmente inlinável. Como `SocraticOp` carrega
    /// `SocraticThought` (com strings grandes), o `Err` é boxed para
    /// satisfazer o lint `clippy::result_large_err` e para evitar cópias
    /// desnecessárias no caminho de erro.
    pub fn try_send(&self, op: SocraticOp) -> Result<(), Box<SocraticOp>> {
        self.tx.try_send(op).map_err(|e| match e {
            mpsc::error::TrySendError::Full(op) => Box::new(op),
            mpsc::error::TrySendError::Closed(op) => Box::new(op),
        })
    }

    /// Despacha síncrono (bloqueia até enfileirar). Apenas para
    /// shutdown ordenado (drain do worker).
    pub async fn send(&self, op: SocraticOp) -> Result<(), mpsc::error::SendError<SocraticOp>> {
        self.tx.send(op).await
    }

    /// Total de operações processadas pelo worker (counter atômico).
    pub fn processed(&self) -> usize {
        self.processed.load(Ordering::Relaxed)
    }

    /// True se o canal está saturado (>= 1/2 da capacidade). Útil para
    /// métricas FinOps e circuit breaker.
    pub fn is_under_backpressure(&self) -> bool {
        self.tx.capacity() < SOCRATIC_CHANNEL_CAPACITY / 2
    }
}

impl crate::cognition::thinking::persistence::SocraticPersist for SocraticWriteHandle {
    fn persist_thought(&self, thought: SocraticThought) -> Result<(), String> {
        self.try_send(SocraticOp::UpsertThoughtFire { thought })
            .map_err(|_| "SocraticWriteWorker saturado: canal MPSC rejeitou pensamento".to_string())
    }

    fn persist_session(&self, session_id: &str, metadata: &str) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default();
        self.try_send(SocraticOp::UpsertSessionFire {
            session_id: session_id.to_string(),
            created_at: now,
            metadata: metadata.to_string(),
        })
        .map_err(|_| "SocraticWriteWorker saturado: canal MPSC rejeitou sessão".to_string())
    }
}

/// Constrói o canal MPSC e devolve o `SocraticWriteHandle` + dispara
/// o worker dedicado.
///
/// **Padrão Canônico SOULS (Marco 3.5):** `std::thread::spawn` +
/// `blocking_recv`, mantendo `rusqlite` síncrono e isolado do event
/// loop do Tokio. `Connection::open` é síncrono, então qualquer
/// uso de `tokio::task::spawn_blocking` adicionaria overhead sem
/// benefício.
///
/// O canal é bounded em 512 (vs 100 do `MemGraphOp`) para absorver
/// o cenário de stress 10k pensamentos.
pub fn spawn_socratic_write_worker(
    db_path: PathBuf,
) -> Result<SocraticWriteHandle, Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel::<SocraticOp>(SOCRATIC_CHANNEL_CAPACITY);
    let processed = Arc::new(AtomicUsize::new(0));
    let processed_clone = processed.clone();

    std::thread::spawn(move || {
        let mut conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[SocraticWriteWorker] ERRO ao abrir banco: {e}");
                return;
            }
        };
        // Tríade canônica SOULS: WAL, FK ON, busy_timeout 5s.
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");
        let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));

        // Migração V3→V5 no boot do worker (idempotente, requer &mut).
        if let Err(e) = ops::migrate_v3_to_v5(&mut conn) {
            eprintln!("[SocraticWriteWorker] ERRO na migração V3→V5: {e}");
        }

        // Sequência do canal: cada SocraticOp é processado em ordem.
        // O loop termina quando o `Sender` é droppado (canal fechado).
        //
        // **Micro-batching (Marco 3.9 Fase E.2):** acumulamos até
        // `SOCRATIC_BATCH_SIZE` ops em uma única transação para evitar
        // o overhead de 1 fsync por insert (WAL + Windows ReFS). Sem
        // batching, 10k inserts = 10k fsyncs = ~30s de stress test.
        // Com BATCH_SIZE=64, 10k inserts = ~157 fsyncs = ~2s.
        let mut batch: Vec<SocraticOp> = Vec::with_capacity(SOCRATIC_BATCH_SIZE);
        loop {
            // Block até a próxima op chegar (ou canal fechar).
            let first = match rx.blocking_recv() {
                Some(op) => op,
                None => {
                    // Canal fechado: drena o que sobrou.
                    if !batch.is_empty() {
                        process_batch(&conn, &mut batch, &processed_clone);
                    }
                    break;
                }
            };
            batch.push(first);
            // Tenta drenar mais sem bloquear até encher o batch.
            while batch.len() < SOCRATIC_BATCH_SIZE {
                match rx.try_recv() {
                    Ok(op) => batch.push(op),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => break,
                }
            }
            // Processa o batch em uma única transação.
            process_batch(&conn, &mut batch, &processed_clone);
        }
    });

    Ok(SocraticWriteHandle { tx, processed })
}

/// Processa um micro-batch de `SocraticOp` em uma única transação SQLite.
///
/// Se a transação falhar no `BEGIN`, todas as ops do batch são perdidas
/// (contadas no counter atômico, mas o conteúdo não foi persistido).
/// Se falhar no meio do batch, cada op reporta o erro individualmente
/// (log + drop) e o batch continua — preserva o fail-soft da Hipótese
/// Hiper-Forward. Se falhar no `COMMIT`, o batch inteiro é desfeito
/// (rollback implícito).
fn process_batch(
    conn: &Connection,
    batch: &mut Vec<SocraticOp>,
    processed: &Arc<AtomicUsize>,
) {
    if batch.is_empty() {
        return;
    }

    // BATCH_SIZE é tipicamente 64 — 10k inserts cabe em ~157 batches.
    let batch_len = batch.len();

    // Para batches puros de leitura (não há nenhuma nesse worker —
    // todos são write), não abriríamos transação. Aqui todos são
    // write, então BEGIN imediato.
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[SocraticWriteWorker] BEGIN falhou: {e}");
            for _ in batch.drain(..) {
                processed.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
    };

    for op in batch.drain(..) {
        match op {
            SocraticOp::UpsertSession {
                session_id,
                created_at,
                metadata,
                reply,
            } => {
                let r = ops::upsert_socratic_session(&tx, &session_id, created_at, &metadata)
                    .map(|_| {
                        json!({
                            "ok": true,
                            "session_id": session_id,
                            "status": "upserted"
                        })
                    })
                    .map_err(|e| e.to_string());
                let _ = reply.send(r);
            }
            SocraticOp::UpsertThought { thought, reply } => {
                let tid = thought.thought_id.clone();
                let r = ops::upsert_socratic_thought(&tx, &thought)
                    .map(|_| {
                        json!({
                            "ok": true,
                            "thought_id": tid,
                            "status": "upserted"
                        })
                    })
                    .map_err(|e| e.to_string());
                let _ = reply.send(r);
            }
            SocraticOp::UpsertSessionFire {
                session_id,
                created_at,
                metadata,
            } => {
                if let Err(e) =
                    ops::upsert_socratic_session(&tx, &session_id, created_at, &metadata)
                {
                    eprintln!(
                        "[SocraticWriteWorker] UpsertSessionFire falhou para '{session_id}': {e}"
                    );
                }
            }
            SocraticOp::UpsertThoughtFire { thought } => {
                if let Err(e) = ops::upsert_socratic_thought(&tx, &thought) {
                    eprintln!(
                        "[SocraticWriteWorker] UpsertThoughtFire falhou para '{}': {e}",
                        thought.thought_id
                    );
                }
            }
        }
        processed.fetch_add(1, Ordering::Relaxed);
    }

    if let Err(e) = tx.commit() {
        eprintln!(
            "[SocraticWriteWorker] COMMIT falhou: {e} (batch de {batch_len} ops desfeito via rollback)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::thinking::persistence::{BranchId, SocraticThought, ThoughtType};
    use tempfile::{TempDir, tempdir};

    /// Helper: cria um banco temporário e devolve (handle, db_path).
    fn fresh_worker() -> (SocraticWriteHandle, PathBuf, TempDir) {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("socratic_test.db");
        let handle = spawn_socratic_write_worker(db_path.clone()).expect("spawn worker");
        // CRÍTICO: devolver o `TempDir` para o caller. Caso contrário
        // o `Drop` do TempDir é disparado no retorno desta função e
        // deleta o diretório — o worker thread, em seguida, tenta
        // `Connection::open(&db_path)` em uma pasta inexistente, com
        // "unable to open database file". Marco 4.0.2 (T2): SQLite
        // isolado por teste, com diretório de teste vivo até o final.
        (handle, db_path, dir)
    }

    fn make_thought(session_id: &str, step: u32, parent: Option<&str>) -> SocraticThought {
        SocraticThought {
            thought_id: format!("th_{step}"),
            session_id: session_id.to_string(),
            branch_id: BranchId::from("main"),
            parent_thought_id: parent.map(str::to_string),
            thought_type: ThoughtType::Regular,
            content: format!("thought #{step}"),
            step_number: step,
            duration_ms: 10,
            created_at: 1000 + step as i64,
        }
    }

    #[test]
    fn test_try_send_succeeds_under_capacity() {
        let (h, _p, _dir) = fresh_worker();
        // Pequena espera para o worker inicializar e migrar o schema.
        std::thread::sleep(std::time::Duration::from_millis(20));

        // UpsertSession com ACK.
        let (ack_tx, ack_rx) = oneshot::channel();
        h.try_send(SocraticOp::UpsertSession {
            session_id: "s1".into(),
            created_at: 1000,
            metadata: "{}".into(),
            reply: ack_tx,
        })
        .expect("try_send Ok");

        let r = ack_rx.blocking_recv().expect("ack").expect("Ok");
        assert_eq!(r["ok"], serde_json::Value::Bool(true));
        assert_eq!(r["session_id"], "s1");
    }

    #[test]
    fn test_upsert_thought_fire_and_forget_persists() {
        let (h, db_path, _dir) = fresh_worker();
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Sessão.
        h.try_send(SocraticOp::UpsertSessionFire {
            session_id: "s_fire".into(),
            created_at: 2000,
            metadata: "{}".into(),
        })
        .unwrap();

        // 3 pensamentos em fire-and-forget.
        for step in 1..=3 {
            h.try_send(SocraticOp::UpsertThoughtFire {
                thought: make_thought("s_fire", step, if step == 1 { None } else { Some("th_1") }),
            })
            .unwrap();
        }

        // Espera o worker drenar.
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(h.processed() >= 4, "processou ≥ 4 ops, got {}", h.processed());

        // Reabre o banco e verifica.
        let conn = Connection::open(&db_path).expect("abre para verificação");
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM socratic_thoughts WHERE session_id = 's_fire'",
                [],
                |r| r.get(0),
            )
            .expect("COUNT");
        assert_eq!(n, 3, "3 pensamentos persistidos");
    }
}
