use rusqlite::Connection;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::ratings;
use crate::cognition::memory::init_memory_schema;

#[derive(Debug, Clone)]
pub struct FeedbackTask {
    pub feedback_id: String,
    pub target_id: String,
    pub feedback_type: String,
    pub source_action: String,
    pub reward_value: f64,
    pub created_at: i64,
}

pub struct WeEvolveEngine {
    sender: mpsc::Sender<FeedbackTask>,
    db_conn: Arc<Mutex<Connection>>,
}

static GLOBAL_ENGINE: OnceLock<WeEvolveEngine> = OnceLock::new();

impl WeEvolveEngine {
    /// Returns reference to global `WeEvolveEngine` singleton instance
    pub fn global() -> &'static WeEvolveEngine {
        GLOBAL_ENGINE.get_or_init(WeEvolveEngine::new_in_memory)
    }

    /// Initializes `WeEvolveEngine` with an in-memory SQLite database
    pub fn new_in_memory() -> Self {
        let conn = Connection::open_in_memory().expect("Must open in-memory SQLite for WeEvolve");
        init_memory_schema(&conn).expect("Must init memory schema for WeEvolve");
        Self::new_with_conn(conn)
    }

    /// Initializes `WeEvolveEngine` with an existing SQLite connection
    pub fn new_with_conn(conn: Connection) -> Self {
        let (sender, receiver) = mpsc::channel::<FeedbackTask>();
        let db_conn = Arc::new(Mutex::new(conn));
        let db_conn_worker = Arc::clone(&db_conn);

        thread::spawn(move || {
            while let Ok(task) = receiver.recv() {
                let lock_res = db_conn_worker.lock();
                if let Ok(conn) = lock_res {
                    if let Err(e) = ratings::ensure_rating_target(&conn, &task.target_id) {
                        eprintln!("[WeEvolve Error] Failed to ensure rating target: {e}");
                        continue;
                    }

                    let ins_res = conn.execute(
                        "INSERT INTO weevolve_feedbacks (feedback_id, target_id, feedback_type, source_action, reward_value, created_at)
                         VALUES (?, ?, ?, ?, ?, ?)",
                        rusqlite::params![
                            task.feedback_id,
                            task.target_id,
                            task.feedback_type,
                            task.source_action,
                            task.reward_value,
                            task.created_at
                        ],
                    );

                    if let Err(e) = ins_res {
                        eprintln!("[WeEvolve Error] Failed to insert feedback: {e}");
                    } else if let Err(e) = ratings::update_rating_in_db(&conn, &task.target_id, task.reward_value) {
                        eprintln!("[WeEvolve Error] Failed to update rating in DB: {e}");
                    }
                }
            }
        });

        Self { sender, db_conn }
    }

    /// Records implicit physical telemetry signal from user action
    /// Action Mappings:
    /// - 'git_rollback': reward -1.5 (IMPLICIT_NEGATIVE)
    /// - 'compilation_failure': reward -1.0 (IMPLICIT_NEGATIVE)
    /// - 'test_success': reward +1.2 (IMPLICIT_POSITIVE)
    /// - 'manual_edit_distance': reward -0.8 (IMPLICIT_NEGATIVE)
    pub fn record_implicit_signal(
        &self,
        target_id: &str,
        action: &str,
        outcome: Result<(), String>,
    ) -> Result<(), String> {
        let (reward_value, feedback_type) = match action {
            "git_rollback" => (-1.5, "IMPLICIT_NEGATIVE"),
            "compilation_failure" => (-1.0, "IMPLICIT_NEGATIVE"),
            "test_success" => (1.2, "IMPLICIT_POSITIVE"),
            "manual_edit_distance" => (-0.8, "IMPLICIT_NEGATIVE"),
            _ => match outcome {
                Ok(()) => (1.0, "IMPLICIT_POSITIVE"),
                Err(_) => (-1.0, "IMPLICIT_NEGATIVE"),
            },
        };

        let feedback_id = uuid::Uuid::new_v4().to_string();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let task = FeedbackTask {
            feedback_id,
            target_id: target_id.to_string(),
            feedback_type: feedback_type.to_string(),
            source_action: action.to_string(),
            reward_value,
            created_at,
        };

        eprintln!(
            "[WeEvolve Telemetry] action: {}, target: {}, reward: {:.2}, type: {}",
            action, target_id, reward_value, feedback_type
        );

        self.sender
            .send(task)
            .map_err(|e| format!("MPSC send error: {e}"))?;

        Ok(())
    }

    /// Retrieves current (elo_rating, ema_score) for given target_id
    pub fn get_rating(&self, target_id: &str) -> (f64, f64) {
        let conn = self.db_conn.lock().unwrap();
        ratings::get_rating_from_db(&conn, target_id).unwrap_or((1200.0, 1.0))
    }

    /// Synchronously overrides rating for test suite pacing validation
    pub fn set_rating_override(&self, target_id: &str, elo: f64, ema: f64) -> Result<(), String> {
        let conn = self.db_conn.lock().unwrap();
        ratings::ensure_rating_target(&conn, target_id)
            .map_err(|e| format!("DB error: {e}"))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "UPDATE weevolve_ratings SET elo_rating = ?, ema_score = ?, updated_at = ? WHERE target_id = ?",
            rusqlite::params![elo, ema, now, target_id],
        )
        .map_err(|e| format!("DB update error: {e}"))?;
        Ok(())
    }

    /// Helper to wait briefly for background worker MPSC processing to flush
    pub fn wait_for_flush(&self) {
        thread::sleep(Duration::from_millis(50));
    }
}
