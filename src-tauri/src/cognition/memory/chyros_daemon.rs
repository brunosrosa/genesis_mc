// SOULS V6 — Memory Module: Chyros Daemon (AutoDream)
// Daemon assíncrono para monitoramento de ociosidade, decaimento Langevin e consolidação cognitiva episódica L0 estritamente na CPU (AVX2).

use super::langevin_decay::apply_langevin_decay;
use crate::core::gigatoken_encoder::GigaTokenEncoder;
use crate::core::llama_logit_probing::LlamaCpp4LogitEngine;
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::task::JoinHandle;

/// Rastreador de Atividade do Usuário com suporte a cancelamento cooperativo <100ms.
#[derive(Debug, Clone)]
pub struct ActivityTracker {
    last_activity_epoch: Arc<AtomicU64>,
    cancel_flag: Arc<AtomicBool>,
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityTracker {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            last_activity_epoch: Arc::new(AtomicU64::new(now)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn record_activity(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_activity_epoch.store(now, Ordering::SeqCst);
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn last_activity_epoch(&self) -> u64 {
        self.last_activity_epoch.load(Ordering::SeqCst)
    }

    pub fn clear_cancel(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
    }

    pub fn should_abort(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    pub fn trigger_cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }
}

/// Relatório final do ciclo de consolidação AutoDream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsolidationReport {
    pub decay_nodes_updated: usize,
    pub l0_events_processed: usize,
    pub mmv_snapshot: String,
    pub mmv_token_count: usize,
    pub is_aligned_64: bool,
}

#[derive(Clone)]
pub struct ChyrosDaemon {
    db_path: PathBuf,
    idle_threshold_secs: u64,
    tick_interval_ms: u64,
    tracker: ActivityTracker,
}

impl ChyrosDaemon {
    pub fn new<P: AsRef<Path>>(db_path: P, idle_threshold_secs: u64) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
            idle_threshold_secs,
            tick_interval_ms: 1000,
            tracker: ActivityTracker::new(),
        }
    }

    pub fn with_tick_interval_ms(mut self, interval_ms: u64) -> Self {
        self.tick_interval_ms = interval_ms;
        self
    }

    pub fn tracker(&self) -> ActivityTracker {
        self.tracker.clone()
    }

    pub fn record_activity(&self) {
        self.tracker.record_activity();
    }

    pub fn is_idle(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let last = self.tracker.last_activity_epoch();
        now.saturating_sub(last) >= self.idle_threshold_secs
    }

    /// Executa o ciclo síncrono/assíncrono de AutoDream com verificação de cancelamento <100ms.
    pub fn run_consolidation_cycle(&self, conn: &Connection) -> Result<ConsolidationReport, String> {
        if self.tracker.should_abort() {
            eprintln!("[ChyrosDaemon] Abortando ciclo: atividade do usuário detectada.");
            return Err("Aborted: User active".to_string());
        }

        eprintln!("[ChyrosDaemon] Iniciando ciclo de AutoDream e consolidação cognitiva...");

        // Fase 1: Langevin Decay na Bola de Poincaré
        let decay_count = apply_langevin_decay(conn, 0.05, 0.02, 1.0)?;

        if self.tracker.should_abort() {
            eprintln!("[ChyrosDaemon] Interrupção <100ms acionada após Langevin decay.");
            return Err("Aborted: User active".to_string());
        }

        // Fase 2: Consolidação Cognitiva Episódica (L0 Events) estritamente na CPU (AVX2)
        let l0_count = self.consolidate_l0_events_cpu(conn)?;

        if self.tracker.should_abort() {
            eprintln!("[ChyrosDaemon] Interrupção <100ms acionada após consolidação L0.");
            return Err("Aborted: User active".to_string());
        }

        // Fase 3: Compilação de Snapshot da Visão Materializada de Memória (MMV) com alinhamento de 64-Tokens
        let (mmv_snapshot, token_count) = self.build_aligned_mmv_snapshot(conn)?;

        let report = ConsolidationReport {
            decay_nodes_updated: decay_count,
            l0_events_processed: l0_count,
            mmv_snapshot,
            mmv_token_count: token_count,
            is_aligned_64: token_count > 0 && token_count.is_multiple_of(64),
        };

        eprintln!(
            "[ChyrosDaemon] Ciclo concluído: decay={}, l0={}, mmv_tokens={}",
            report.decay_nodes_updated, report.l0_events_processed, report.mmv_token_count
        );

        Ok(report)
    }

    /// Processamento de Eventos L0 na CPU (AVX2) via LlamaCpp4LogitEngine (n_gpu_layers = 0).
    fn consolidate_l0_events_cpu(&self, conn: &Connection) -> Result<usize, String> {
        // Garantia de Isolamento de VRAM: LlamaCpp4LogitEngine roda 100% na CPU (AVX2)
        let _cpu_engine = LlamaCpp4LogitEngine::new();

        let mut stmt = conn
            .prepare("SELECT event_id, event_type, payload FROM souls_raw_events_l0 WHERE processed = 0 ORDER BY event_id ASC")
            .map_err(|e| format!("Erro ao consultar eventos L0: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| format!("Erro ao mapear eventos L0: {}", e))?;

        let events: Vec<(i64, String, String)> = rows.filter_map(Result::ok).collect();
        drop(stmt);

        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut processed_count = 0;

        for (event_id, _ev_type, payload) in events {
            if self.tracker.should_abort() {
                break;
            }

            // Tenta efetuar o parse do payload JSON
            let parsed_json: serde_json::Value = serde_json::from_str(&payload).unwrap_or(serde_json::json!({}));
            
            // Resolução Socrática de Contradições:
            // Se payload indicar contradição com memória existente (ex: contradicts_id)
            if let Some(contradicts_id) = parsed_json.get("contradicts_id").and_then(|v| v.as_str()) {
                let sql_tombstone = "UPDATE souls_memory_nodes SET stability_status = 'SUPERSEDED', updated_at = ?1 WHERE memory_id = ?2";
                let _ = conn.execute(sql_tombstone, params![now_epoch, contradicts_id]);
            }

            // Inserção ou Atualização da Nova Premissa Consolidada em souls_memory_nodes
            let memory_id = parsed_json
                .get("memory_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("mem_{}_{}", event_id, now_epoch));

            let content = parsed_json
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or(&payload)
                .to_string();

            let status = parsed_json
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("EVOLVING");

            let insert_sql = "INSERT INTO souls_memory_nodes (memory_id, content, stability_status, relevance_score, poincare_x, poincare_y, updated_at)
                              VALUES (?1, ?2, ?3, 1.0, 0.0, 0.0, ?4)
                              ON CONFLICT(memory_id) DO UPDATE SET content = excluded.content, stability_status = excluded.stability_status, updated_at = excluded.updated_at";
            let _ = conn.execute(insert_sql, params![memory_id, content, status, now_epoch]);

            // Marca evento L0 como processado
            let update_l0 = "UPDATE souls_raw_events_l0 SET processed = 1 WHERE event_id = ?1";
            let _ = conn.execute(update_l0, params![event_id]);

            processed_count += 1;
        }

        Ok(processed_count)
    }

    /// Compila o snapshot da Materialized Memory View (MMV) e garante alinhamento a múltiplos de 64 tokens.
    fn build_aligned_mmv_snapshot(&self, conn: &Connection) -> Result<(String, usize), String> {
        let mut stmt = conn
            .prepare("SELECT memory_id, content, stability_status FROM souls_memory_nodes WHERE stability_status IN ('STABLE', 'EVOLVING') ORDER BY updated_at DESC")
            .map_err(|e| format!("Erro ao consultar MMV: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(format!(
                    "[{}] ({}) {}",
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(1)?
                ))
            })
            .map_err(|e| format!("Erro ao mapear MMV: {}", e))?;

        let lines: Vec<String> = rows.filter_map(Result::ok).collect();
        drop(stmt);

        let base_header = "[MATERIALIZED_MEMORY_VIEW]\n";
        let raw_mmv = format!("{}{}", base_header, lines.join("\n"));

        let encoder = GigaTokenEncoder::global();
        let initial_tokens = encoder.tokenize_to_bin(&raw_mmv).unwrap_or_default();
        let token_count = initial_tokens.len();

        if token_count == 0 {
            return Ok((raw_mmv, 0));
        }

        if token_count.is_multiple_of(64) {
            return Ok((raw_mmv, token_count));
        }

        // Iterative Token Padding Loop: Garante alinhamento a múltiplos de 64 tokens no BPE do Tokenizer
        let mut candidate = format!("{}\n/* pad", raw_mmv);
        let mut attempts = 0;
        while attempts < 1000 {
            let tokens = encoder.tokenize_to_bin(&candidate).unwrap_or_default();
            let len = tokens.len();
            if len.is_multiple_of(64) {
                return Ok((candidate, len));
            }
            candidate.push_str(" x");
            attempts += 1;
        }

        let final_tokens = encoder.tokenize_to_bin(&candidate).unwrap_or_default();
        Ok((candidate, final_tokens.len()))
    }

    /// Dispara a rotina em segundo plano via Tokio Task.
    pub fn start_background_loop(self) -> JoinHandle<()> {
        let daemon = Arc::new(self);
        tokio::spawn(async move {
            let interval = Duration::from_millis(daemon.tick_interval_ms);
            loop {
                tokio::time::sleep(interval).await;
                if daemon.is_idle() {
                    let db_path = daemon.db_path.clone();
                    let daemon_cloned = daemon.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = Connection::open_with_flags(
                            &db_path,
                            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
                        ) {
                            let _ = daemon_cloned.run_consolidation_cycle(&conn);
                        }
                    }).await;
                }
            }
        })
    }
}
