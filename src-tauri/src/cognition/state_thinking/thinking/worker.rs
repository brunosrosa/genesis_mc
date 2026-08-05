use crate::cognition::thinking::ops;
use crate::cognition::thinking::persistence::SocraticThought;
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

/// Diário de execução de um subagente para persistência socrática/estado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubAgentDiary {
    pub agent_id: String,
    pub session_id: String,
    pub content: String,
    pub created_at: i64,
}

/// Operação enviada no barramento MPSC do `StateDbWorker`.
#[derive(Debug)]
pub enum StateDbOp {
    // Caminhos Quentes (Exigem oneshot::Sender para destravar o agente de forma síncrona-reativa):
    WriteSocraticThought {
        session_id: String,
        thought: SocraticThought,
        reply: oneshot::Sender<Result<(), String>>,
    },
    WriteDiary {
        diary: SubAgentDiary,
        reply: oneshot::Sender<Result<(), String>>,
    },
    // Caminhos Frios (Try_send na RAM, acumulados em memória e gravados em lote de 5s):
    LogFileAccess {
        file_path: String,
        tool: String,
    },
    LogTelemetry {
        metric: String,
        value: f64,
    },
}

/// Global OnceLock para o barramento MPSC de escrita do State DB.
pub static STATE_DB_TX: OnceLock<mpsc::Sender<StateDbOp>> = OnceLock::new();

/// Tenta enviar uma operação fria no canal MPSC global.
/// Se o canal estiver cheio ou indisponível, aplica Soft Drop (descarte na RAM).
pub fn try_send_cold(op: StateDbOp) -> Result<(), String> {
    if let Some(tx) = STATE_DB_TX.get() {
        match tx.try_send(op) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                Err("SoftDrop: State DB channel full".to_string())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err("State DB channel closed".to_string())
            }
        }
    } else {
        Err("State DB channel not initialized".to_string())
    }
}

/// Retorna a cadência de flush (1s no warm-up de 30s, 5s no steady-state).
pub fn current_flush_interval(boot_time: Instant, current_time: Instant) -> Duration {
    if current_time.duration_since(boot_time) < Duration::from_secs(30) {
        Duration::from_secs(1)
    } else {
        Duration::from_secs(5)
    }
}

#[derive(Debug)]
enum ColdItem {
    FileAccess {
        file_path: String,
        tool: String,
        accessed_at: i64,
    },
    Telemetry {
        metric: String,
        value: f64,
        created_at: i64,
    },
}

/// Inicializa a thread OS dedicada para gravações exclusivas no `souls_state.db`.
pub fn init_state_db_worker(
    db_path: PathBuf,
    mut rx: mpsc::Receiver<StateDbOp>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[StateDbWorker] ERRO ao abrir banco {:?}: {e}", db_path);
                return;
            }
        };

        if let Err(e) = setup_db_connection(&mut conn) {
            eprintln!("[StateDbWorker] ERRO ao configurar PRAGMAs/Schema: {e}");
            return;
        }

        run_worker_loop(&mut conn, &mut rx);
    })
}

fn setup_db_connection(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(|e| format!("PRAGMA error: {e}"))?;

    ops::migrate_v3_to_v5(conn).map_err(|e| format!("Migration error: {e}"))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sub_agent_diaries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL
        ) STRICT;
        CREATE TABLE IF NOT EXISTS file_access_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            tool TEXT NOT NULL,
            accessed_at INTEGER NOT NULL
        ) STRICT;
        CREATE TABLE IF NOT EXISTS telemetry_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            metric TEXT NOT NULL,
            value REAL NOT NULL,
            created_at INTEGER NOT NULL
        ) STRICT;",
    )
    .map_err(|e| format!("Table creation error: {e}"))?;

    Ok(())
}

fn run_worker_loop(conn: &mut Connection, rx: &mut mpsc::Receiver<StateDbOp>) {
    let boot_time = Instant::now();
    let mut last_flush = Instant::now();
    let mut cold_buffer: Vec<ColdItem> = Vec::with_capacity(50);

    loop {
        let current_time = Instant::now();
        let interval = current_flush_interval(boot_time, current_time);

        let op_opt = if cold_buffer.is_empty() {
            match rx.blocking_recv() {
                Some(op) => Some(op),
                None => break,
            }
        } else {
            if last_flush.elapsed() >= interval || cold_buffer.len() >= 50 {
                let _ = flush_cold_buffer(conn, &mut cold_buffer);
                last_flush = Instant::now();
                continue;
            }

            match rx.try_recv() {
                Ok(op) => Some(op),
                Err(mpsc::error::TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    let _ = flush_cold_buffer(conn, &mut cold_buffer);
                    break;
                }
            }
        };

        if let Some(op) = op_opt {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or_default();

            match op {
                StateDbOp::WriteSocraticThought {
                    session_id,
                    thought,
                    reply,
                } => {
                    let res = flush_jit_hot_thought(conn, &mut cold_buffer, &session_id, &thought, now);
                    let _ = reply.send(res);
                    last_flush = Instant::now();
                }
                StateDbOp::WriteDiary { diary, reply } => {
                    let res = flush_jit_hot_diary(conn, &mut cold_buffer, &diary);
                    let _ = reply.send(res);
                    last_flush = Instant::now();
                }
                StateDbOp::LogFileAccess { file_path, tool } => {
                    cold_buffer.push(ColdItem::FileAccess {
                        file_path,
                        tool,
                        accessed_at: now,
                    });
                    if cold_buffer.len() >= 50 {
                        let _ = flush_cold_buffer(conn, &mut cold_buffer);
                        last_flush = Instant::now();
                    }
                }
                StateDbOp::LogTelemetry { metric, value } => {
                    cold_buffer.push(ColdItem::Telemetry {
                        metric,
                        value,
                        created_at: now,
                    });
                    if cold_buffer.len() >= 50 {
                        let _ = flush_cold_buffer(conn, &mut cold_buffer);
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }

    if !cold_buffer.is_empty() {
        let _ = flush_cold_buffer(conn, &mut cold_buffer);
    }
}

fn flush_cold_buffer(conn: &mut Connection, cold_buffer: &mut Vec<ColdItem>) -> Result<(), String> {
    if cold_buffer.is_empty() {
        return Ok(());
    }

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    for item in cold_buffer.drain(..) {
        match item {
            ColdItem::FileAccess {
                file_path,
                tool,
                accessed_at,
            } => {
                tx.execute(
                    "INSERT INTO file_access_logs (file_path, tool, accessed_at) VALUES (?1, ?2, ?3)",
                    params![file_path, tool, accessed_at],
                )
                .map_err(|e| e.to_string())?;
            }
            ColdItem::Telemetry {
                metric,
                value,
                created_at,
            } => {
                tx.execute(
                    "INSERT INTO telemetry_metrics (metric, value, created_at) VALUES (?1, ?2, ?3)",
                    params![metric, value, created_at],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn flush_jit_hot_thought(
    conn: &mut Connection,
    cold_buffer: &mut Vec<ColdItem>,
    session_id: &str,
    thought: &SocraticThought,
    now: i64,
) -> Result<(), String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    for item in cold_buffer.drain(..) {
        match item {
            ColdItem::FileAccess {
                file_path,
                tool,
                accessed_at,
            } => {
                tx.execute(
                    "INSERT INTO file_access_logs (file_path, tool, accessed_at) VALUES (?1, ?2, ?3)",
                    params![file_path, tool, accessed_at],
                )
                .map_err(|e| e.to_string())?;
            }
            ColdItem::Telemetry {
                metric,
                value,
                created_at,
            } => {
                tx.execute(
                    "INSERT INTO telemetry_metrics (metric, value, created_at) VALUES (?1, ?2, ?3)",
                    params![metric, value, created_at],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    ops::upsert_socratic_session(&tx, session_id, now, "{}").map_err(|e| e.to_string())?;
    ops::upsert_socratic_thought(&tx, thought).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn flush_jit_hot_diary(
    conn: &mut Connection,
    cold_buffer: &mut Vec<ColdItem>,
    diary: &SubAgentDiary,
) -> Result<(), String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    for item in cold_buffer.drain(..) {
        match item {
            ColdItem::FileAccess {
                file_path,
                tool,
                accessed_at,
            } => {
                tx.execute(
                    "INSERT INTO file_access_logs (file_path, tool, accessed_at) VALUES (?1, ?2, ?3)",
                    params![file_path, tool, accessed_at],
                )
                .map_err(|e| e.to_string())?;
            }
            ColdItem::Telemetry {
                metric,
                value,
                created_at,
            } => {
                tx.execute(
                    "INSERT INTO telemetry_metrics (metric, value, created_at) VALUES (?1, ?2, ?3)",
                    params![metric, value, created_at],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    tx.execute(
        "INSERT INTO sub_agent_diaries (agent_id, session_id, content, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![diary.agent_id, diary.session_id, diary.content, diary.created_at],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::thinking::persistence::{BranchId, ThoughtType};
    use tempfile::tempdir;

    #[test]
    fn test_mpsc_backpressure_drop() {
        let (tx, _rx) = mpsc::channel::<StateDbOp>(2);

        // Fill channel to capacity with cold messages
        assert!(tx.try_send(StateDbOp::LogFileAccess {
            file_path: "file1.rs".into(),
            tool: "read".into()
        }).is_ok());

        assert!(tx.try_send(StateDbOp::LogFileAccess {
            file_path: "file2.rs".into(),
            tool: "read".into()
        }).is_ok());

        // 3rd cold message fails try_send (Soft Drop)
        let cold_res = tx.try_send(StateDbOp::LogTelemetry {
            metric: "cpu".into(),
            value: 42.0
        });

        assert!(cold_res.is_err());
        match cold_res {
            Err(mpsc::error::TrySendError::Full(op)) => {
                if let StateDbOp::LogTelemetry { metric, .. } = op {
                    assert_eq!(metric, "cpu");
                } else {
                    panic!("wrong op returned");
                }
            }
            _ => panic!("expected TrySendError::Full"),
        }
    }

    #[test]
    fn test_exponential_slicing_warmup() {
        let boot = Instant::now();
        // Warmup period (< 30s)
        let early = boot + Duration::from_secs(5);
        assert_eq!(current_flush_interval(boot, early), Duration::from_secs(1));

        let boundary = boot + Duration::from_secs(29);
        assert_eq!(current_flush_interval(boot, boundary), Duration::from_secs(1));

        // Steady state (>= 30s)
        let late = boot + Duration::from_secs(31);
        assert_eq!(current_flush_interval(boot, late), Duration::from_secs(5));
    }

    #[test]
    fn test_jit_jumper_immediate_flush() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("souls_state_test.db");

        let (tx, rx) = mpsc::channel::<StateDbOp>(64);
        let _worker = init_state_db_worker(db_path.clone(), rx);

        // Enqueue cold operations
        tx.try_send(StateDbOp::LogFileAccess {
            file_path: "src/main.rs".into(),
            tool: "edit".into(),
        })
        .unwrap();

        // Enqueue hot operation with oneshot channel
        let (ack_tx, ack_rx) = oneshot::channel();
        let start = Instant::now();

        tx.try_send(StateDbOp::WriteSocraticThought {
            session_id: "s_jit".into(),
            thought: SocraticThought {
                thought_id: "th_jit_1".into(),
                session_id: "s_jit".into(),
                branch_id: BranchId::from("main"),
                parent_thought_id: None,
                thought_type: ThoughtType::Regular,
                content: "JIT Jumper test".into(),
                step_number: 1,
                duration_ms: 5,
                created_at: 1000,
            },
            reply: ack_tx,
        })
        .unwrap();

        // Wait for oneshot reply (must be ACKed in < 5ms under normal thread execution)
        let res = ack_rx.blocking_recv().expect("ack received");
        let elapsed = start.elapsed();
        println!("JIT Jumper ACK elapsed time: {:?}", elapsed);
        assert!(elapsed < Duration::from_millis(50), "ACK devia responder em tempo recorde, demorou {:?}", elapsed);
        assert!(res.is_ok(), "thought must be saved successfully");
        // Verify SQLite database contains both cold log and hot thought
        let conn = Connection::open(&db_path).expect("open db");
        let cold_count: i64 = conn.query_row("SELECT COUNT(*) FROM file_access_logs WHERE file_path = 'src/main.rs'", [], |r| r.get(0)).unwrap();
        assert_eq!(cold_count, 1, "cold log must be flushed by JIT Jumper");

        let thought_count: i64 = conn.query_row("SELECT COUNT(*) FROM socratic_thoughts WHERE thought_id = 'th_jit_1'", [], |r| r.get(0)).unwrap();
        assert_eq!(thought_count, 1, "hot thought must be flushed by JIT Jumper");
    }
}
