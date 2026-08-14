//! SOULS MC — Marco I · v6.1: Telemetry Dispatcher (MPSC → SQLite async)
//!
//! Recebe eventos de telemetria (TTFT, cost_usd, tokens, PeakEWMA) do hot-path
//! assíncrono do proxy via `tokio::sync::mpsc` e os grava na tabela
//! `telemetry_logs` do `souls_state.db` em uma **thread dedicada** (NÃO
//! bloqueia o Event Loop do Tokio).
//!
//! ## Topologia
//!
//! ```text
//!  agentgateway_tcp_proxy (Tokio task)
//!       │ try_send (não-bloqueante, cap 256)
//!       ▼
//!  TelemetryEvent channel (MPSC, cap 256)
//!       │  blocking_recv (em std::thread::spawn nomeada)
//!       ▼
//!  DispatcherWorker (std::thread "souls-telemetry-dispatcher")
//!       │  rusqlite Connection (WAL V5, batch via transaction a cada flush)
//!       ▼
//!  souls_state.db (SQLite STRICT)
//! ```
//!
//! ## Leis
//!
//! - **ADR-010 (Escrita atômica):** Cada batch de telemetria é uma
//!   `Connection::transaction` (auto-rollback via Drop em panic).
//! - **ADR-027 (Termodinâmica VRAM):** Zero GPU, zero alocação no hot-path
//!   (apenas `try_send`).
//! - **ADR-030 (Higiene):** Apenas deps já presentes (`rusqlite`, `tokio`).
//! - **Marco I (Fail-Soft):** Se o canal encheu (`try_send` falha), o
//!   evento é logado em warn e descartado — telemetria é best-effort.
//!
//! ## Schema (telemetry_logs v4)
//!
//! ```sql
//! CREATE TABLE telemetry_logs (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     tool TEXT NOT NULL,
//!     tokens_in INTEGER NOT NULL DEFAULT 0,
//!     tokens_out INTEGER NOT NULL DEFAULT 0,
//!     cost_usd REAL NOT NULL DEFAULT 0.0,
//!     duration_ms INTEGER NOT NULL DEFAULT 0,
//!     accuracy_score REAL NOT NULL DEFAULT 1.0,
//!     created_at INTEGER NOT NULL
//! ) STRICT;
//! ```

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread;

use rusqlite::{params, Connection, OpenFlags};
use tokio::sync::mpsc;

/// Evento de telemetria que flui do hot-path assíncrono para o worker SQLite.
///
/// Note: `peak_ewma_ms` e `ttft_ms` são `Option<f64>` porque nem todo
/// evento carrega latência (e.g., eventos de finops não têm TTFT).
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub tool: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub duration_ms: i64,
    pub accuracy_score: f64,
    pub peak_ewma_ms: Option<f64>,
    pub ttft_ms: Option<f64>,
    pub session_id: Option<String>,
}

impl TelemetryEvent {
    pub fn new(tool: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            duration_ms: 0,
            accuracy_score: 1.0,
            peak_ewma_ms: None,
            ttft_ms: None,
            session_id: None,
        }
    }
}

/// Sender do canal MPSC. Cloneable, múltiplos producers permitidos.
#[derive(Clone)]
pub struct TelemetrySender {
    tx: mpsc::Sender<TelemetryEvent>,
}

impl TelemetrySender {
    /// Despacha um evento para o worker de forma **não-bloqueante** (try_send).
    /// Se a fila estiver cheia, registra warning e descarta (fail-soft).
    pub fn dispatch(&self, event: TelemetryEvent) {
        match self.tx.try_send(event) {
            Ok(_) => {}
            Err(mpsc::error::TrySendError::Full(ev)) => {
                tracing::warn!(
                    "TelemetryDispatcher: fila cheia (cap 256), descartando evento: {}",
                    ev.tool
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::error!("TelemetryDispatcher: canal fechado, evento perdido");
            }
        }
    }

    /// Helper de conveniência para eventos simples.
    pub fn dispatch_simple(
        &self,
        tool: &str,
        tokens_in: i64,
        tokens_out: i64,
        cost_usd: f64,
        duration_ms: i64,
    ) {
        let mut ev = TelemetryEvent::new(tool);
        ev.tokens_in = tokens_in;
        ev.tokens_out = tokens_out;
        ev.cost_usd = cost_usd;
        ev.duration_ms = duration_ms;
        self.dispatch(ev);
    }

    /// Helper para eventos de latência (TTFT + PeakEWMA).
    /// ADR-025: aceita `LatencyPayload` para sanar `clippy::too_many_arguments`
    /// (a função original tinha 8 args com &self, acima do limite de 7).
    pub fn dispatch_latency(&self, payload: LatencyPayload) {
        let mut ev = TelemetryEvent::new(payload.tool);
        ev.tokens_in = payload.tokens_in;
        ev.tokens_out = payload.tokens_out;
        ev.cost_usd = payload.cost_usd;
        ev.duration_ms = payload.ttft_ms as i64;
        ev.peak_ewma_ms = Some(payload.peak_ewma_ms);
        ev.ttft_ms = Some(payload.ttft_ms);
        ev.session_id = payload.session_id;
        self.dispatch(ev);
    }
}

/// Payload para eventos de latência (TTFT + PeakEWMA).
/// ADR-025: agrupamento dos 7 parâmetros de `dispatch_latency` para sanar
/// `clippy::too_many_arguments` (limite = 7, função tinha 8 com `&self`).
/// Owned types para zero lifetimes explícitos (ADR-025, needless_lifetimes).
#[derive(Debug, Clone)]
pub struct LatencyPayload {
    pub tool: String,
    pub ttft_ms: f64,
    pub peak_ewma_ms: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub session_id: Option<String>,
}

/// Estado global do dispatcher. `OnceLock` garante inicialização única.
static TELEMETRY_SENDER: OnceLock<TelemetrySender> = OnceLock::new();

/// Inicializa o dispatcher global. Idempotente: chamadas subsequentes
/// são no-ops (retornam o sender existente). Retorna o `TelemetrySender`
/// global para uso direto.
///
/// **Concorrência:** múltiplos `init_telemetry_dispatcher` em paralelo são
/// serializados via o `OnceLock::get_or_init` interno. Apenas um worker
/// thread é spawnado.
pub fn init_telemetry_dispatcher(db_path: &Path) -> Result<&'static TelemetrySender, String> {
    // Garante que o schema v4 está materializado antes de aceitar eventos.
    // (Idempotente: CREATE TABLE IF NOT EXISTS.)
    bootstrap_schema(db_path)?;

    let (tx, mut rx) = mpsc::channel::<TelemetryEvent>(256);
    let db_path_owned = db_path.to_path_buf();

    thread::Builder::new()
        .name("souls-telemetry-dispatcher".to_string())
        .spawn(move || {
            let Ok(mut conn) = Connection::open_with_flags(
                &db_path_owned,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
            ) else {
                eprintln!(
                    "[TelemetryDispatcher] ERRO: falha ao abrir {}",
                    db_path_owned.display()
                );
                return;
            };

            // PRAGMA WAL: permite leitores concorrentes durante writes.
            let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");

            let mut batch: Vec<TelemetryEvent> = Vec::with_capacity(32);
            loop {
                // Block no primeiro item, depois drena o que tiver na fila.
                match rx.blocking_recv() {
                    Some(ev) => {
                        batch.push(ev);
                        // Drena o restante sem bloquear.
                        while let Ok(extra) = rx.try_recv() {
                            batch.push(extra);
                            if batch.len() >= 64 {
                                break;
                            }
                        }
                        flush_batch(&mut conn, &mut batch);
                    }
                    None => {
                        // Canal fechado: flush final e exit.
                        flush_batch(&mut conn, &mut batch);
                        break;
                    }
                }
            }
        })
        .map_err(|e| format!("Falha ao spawnar thread souls-telemetry-dispatcher: {e}"))?;

    let sender = TelemetrySender { tx };
    // Tenta inserir. Se já foi inicializado por outra thread, ignora e
    // retorna a instância existente. O `tx` extra é dropped (sender zumbi
    // sem receivers — coletado pelo GC eventualmente).
    let _ = TELEMETRY_SENDER.set(sender);
    TELEMETRY_SENDER
        .get()
        .ok_or_else(|| "TELEMETRY_SENDER não pôde ser inicializado".to_string())
}

/// Cria um dispatcher **privado** (sem tocar o `TELEMETRY_SENDER` global),
/// vinculado a um `db_path` específico. Usado por testes que precisam de
/// isolamento de path. Spawna worker thread dedicada nomeada
/// `souls-telemetry-test-{pid}-{n}`.
#[doc(hidden)]
pub fn init_telemetry_dispatcher_private(db_path: &Path) -> Result<TelemetrySender, String> {
    bootstrap_schema(db_path)?;

    let (tx, mut rx) = mpsc::channel::<TelemetryEvent>(256);
    let db_path_owned = db_path.to_path_buf();

    thread::Builder::new()
        .name(format!("souls-telemetry-test-{}-{}", std::process::id(), line!()))
        .spawn(move || {
            let Ok(mut conn) = Connection::open_with_flags(
                &db_path_owned,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
            ) else {
                return;
            };
            let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");

            let mut batch: Vec<TelemetryEvent> = Vec::with_capacity(32);
            loop {
                match rx.blocking_recv() {
                    Some(ev) => {
                        batch.push(ev);
                        while let Ok(extra) = rx.try_recv() {
                            batch.push(extra);
                            if batch.len() >= 64 {
                                break;
                            }
                        }
                        flush_batch(&mut conn, &mut batch);
                    }
                    None => {
                        flush_batch(&mut conn, &mut batch);
                        break;
                    }
                }
            }
        })
        .map_err(|e| format!("Falha ao spawnar thread privada: {e}"))?;

    Ok(TelemetrySender { tx })
}

/// Retorna o sender global, ou `None` se o dispatcher não foi inicializado.
pub fn telemetry_sender() -> Option<&'static TelemetrySender> {
    TELEMETRY_SENDER.get()
}

/// Resolve o path canônico do `souls_state.db` para o proxy standalone.
/// Sobe a árvore até `.souls_data/souls_state.db` (até 6 níveis).
pub fn resolve_state_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("SOULS_STATE_DB_PATH") {
        return PathBuf::from(p);
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut candidate = cwd.as_path();
        for _ in 0..6 {
            let path = candidate.join(".souls_data").join("souls_state.db");
            if path.is_file() {
                return path;
            }
            match candidate.parent() {
                Some(p) => candidate = p,
                None => break,
            }
        }
    }
    crate::core::workspace_root().join(".souls_data").join("souls_state.db")
}

/// Materializa o schema v4 (telemetry_logs) antes de aceitar eventos.
fn bootstrap_schema(db_path: &Path) -> Result<(), String> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(|e| format!("open({}): {e}", db_path.display()))?;

    // DDL idempotente (cf. cognition/ast/observability/ops.rs).
    let ddl = "
        CREATE TABLE IF NOT EXISTS telemetry_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tool TEXT NOT NULL,
            tokens_in INTEGER NOT NULL DEFAULT 0,
            tokens_out INTEGER NOT NULL DEFAULT 0,
            cost_usd REAL NOT NULL DEFAULT 0.0,
            duration_ms INTEGER NOT NULL DEFAULT 0,
            accuracy_score REAL NOT NULL DEFAULT 1.0,
            created_at INTEGER NOT NULL
        ) STRICT;
        CREATE INDEX IF NOT EXISTS idx_telemetry_tool_time
            ON telemetry_logs(tool, created_at);
        CREATE INDEX IF NOT EXISTS idx_telemetry_time
            ON telemetry_logs(created_at);
        ";
    conn.execute_batch(ddl)
        .map_err(|e| format!("DDL telemetry_logs falhou: {e}"))?;
    Ok(())
}

/// Flush de um batch via transação atômica.
fn flush_batch(conn: &mut Connection, batch: &mut Vec<TelemetryEvent>) {
    if batch.is_empty() {
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut attempts = 0u8;
    let tx = loop {
        match conn.transaction() {
            Ok(t) => break t,
            Err(e) => {
                attempts += 1;
                if attempts >= 3 {
                    eprintln!("[TelemetryDispatcher] Falha persistente em transaction: {e}");
                    batch.clear();
                    return;
                }
                std::thread::yield_now();
            }
        }
    };
    for ev in batch.iter() {
        if let Err(e) = tx.execute(
            "INSERT INTO telemetry_logs \
                (tool, tokens_in, tokens_out, cost_usd, duration_ms, accuracy_score, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                ev.tool,
                ev.tokens_in,
                ev.tokens_out,
                ev.cost_usd,
                ev.duration_ms,
                ev.accuracy_score,
                now,
            ],
        ) {
            eprintln!(
                "[TelemetryDispatcher] INSERT falhou (evento={}): {e}. Rollback do batch.",
                ev.tool
            );
            // Drop da tx → rollback automático.
            batch.clear();
            return;
        }
    }
    if let Err(e) = tx.commit() {
        eprintln!("[TelemetryDispatcher] commit falhou: {e}");
    }
    batch.clear();
}

/// Soma o `cost_usd` do dia atual (UTC) na tabela `telemetry_logs`.
/// Usado pelo `IronCostBreaker` antes de aprovar uma rota cloud.
pub fn sum_today_cost_usd(db_path: &Path) -> Result<f64, String> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| format!("open({}): {e}", db_path.display()))?;

    // Início do dia UTC em epoch seconds.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let day_start = now - (now % 86_400);

    let total: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM telemetry_logs WHERE created_at >= ?1",
            params![day_start],
            |r| r.get(0),
        )
        .map_err(|e| format!("SUM(cost_usd) falhou: {e}"))?;
    Ok(total)
}

// ============================================================================
// Helper de boot para o binário proxy
// ============================================================================

/// Inicializa o dispatcher usando o path configurado em `GatewayConfig`
/// (resolve `${SOULS_STATE_DB_PATH}` no parse-time). Idempotente.
///
/// ## Defesa em profundidade (Issue: literal `${SOULS_STATE_DB_PATH}` no FS)
///
/// Se o path do JSONC contiver um placeholder NÃO-expandido (ex: o
/// `GatewayConfig::global()` foi setado com `${SOULS_STATE_DB_PATH}`
/// literal porque a env var não estava setada no momento do load), este
/// helper **intercepta** o placeholder e cai no `resolve_state_db_path()`,
/// que faz fallback inteligente para `.souls_data/souls_state.db`.
///
/// Isso evita o bug histórico onde o SQLite era aberto em
/// `z:\souls_mc\${SOULS_STATE_DB_PATH}` (path literal) criando arquivos
/// fantasma no filesystem.
pub fn init_from_gateway_config() -> Result<&'static TelemetrySender, String> {
    let cfg = crate::core::gateway_config::GatewayConfig::global();
    let raw_path = &cfg.telemetry.sqlite_path;

    // Detecta placeholder literal não-expandido.
    let is_unexpanded_placeholder = raw_path.contains("${") || raw_path.trim().is_empty();

    let path = if is_unexpanded_placeholder {
        tracing::warn!(
            target: "telemetry_dispatcher",
            "GatewayConfig::global().telemetry.sqlite_path contém placeholder literal ('{}'). \
             Caindo no resolve_state_db_path() (fallback inteligente).",
            raw_path
        );
        resolve_state_db_path()
    } else {
        PathBuf::from(raw_path)
    };

    init_telemetry_dispatcher(&path)
}

// ============================================================================
// Testes TDD (Marco I · v6.1 — Telemetria Async)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;

    fn unique_db_path() -> PathBuf {
        let mut p = env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("souls_telemetry_test_{nanos}.db"));
        p
    }

    #[test]
    fn test_init_creates_schema_and_returns_sender() {
        let path = unique_db_path();
        let sender = init_telemetry_dispatcher(&path).expect("init deve succeed");
        // Verifica que schema existe.
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='telemetry_logs'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "telemetry_logs table deve existir");
        // Sender deve ser usável.
        sender.dispatch_simple("test", 100, 50, 0.001, 250);
        // Cleanup.
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_dispatch_non_blocking_when_full() {
        let path = unique_db_path();
        let sender = init_telemetry_dispatcher(&path).unwrap();
        // Enche a fila rapidamente (cap 256, mas dispatch é try_send).
        for i in 0..512 {
            sender.dispatch_simple("burst", i, 0, 0.0, 1);
        }
        // Nenhum panic; nenhum bloqueio síncrono.
        std::thread::sleep(Duration::from_millis(50));
        std::fs::remove_file(&path).ok();
    }

    /// TDD — Issue: literal `${SOULS_STATE_DB_PATH}` no filesystem.
    /// Garante que `resolve_state_db_path()` NUNCA retorna um path
    /// com placeholder literal não-expandido, mesmo quando a env var
    /// está unsetada. Defens em profundidade contra o bug histórico.
    #[test]
    fn test_resolve_state_db_path_never_returns_literal_placeholder() {
        // Garante que SOULS_STATE_DB_PATH não está setada no test.
        std::env::remove_var("SOULS_STATE_DB_PATH");
        let resolved = resolve_state_db_path();
        let resolved_str = resolved.to_string_lossy();
        assert!(
            !resolved_str.contains("${"),
            "resolve_state_db_path() NUNCA deve conter placeholder literal ${{}}: {}",
            resolved_str
        );
        assert!(
            !resolved_str.is_empty(),
            "resolve_state_db_path() deve retornar path não-vazio: {}",
            resolved_str
        );
    }

    #[test]
    fn test_event_builder_basic() {
        let mut ev = TelemetryEvent::new("agentgateway");
        ev.tokens_in = 100;
        ev.tokens_out = 50;
        ev.cost_usd = 0.0015;
        ev.duration_ms = 250;
        ev.peak_ewma_ms = Some(245.3);
        ev.ttft_ms = Some(180.7);
        assert_eq!(ev.tool, "agentgateway");
        assert_eq!(ev.tokens_in, 100);
        assert_eq!(ev.peak_ewma_ms, Some(245.3));
    }

    #[test]
    fn test_sum_today_cost_usd_zero_initially() {
        let path = unique_db_path();
        // Sem init, schema não existe → falha esperada ou retorna 0.
        // Aqui testamos APÓS init.
        let _sender = init_telemetry_dispatcher(&path).unwrap();
        // Aguarda worker inicializar.
        std::thread::sleep(Duration::from_millis(20));
        let total = sum_today_cost_usd(&path).expect("SUM deve succeed");
        assert_eq!(total, 0.0);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_idempotent_init() {
        let path = unique_db_path();
        let s1 = init_telemetry_dispatcher(&path).unwrap();
        let s2 = init_telemetry_dispatcher(&path).unwrap();
        assert!(std::ptr::eq(s1, s2), "init deve ser idempotente");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_dispatch_latency_helper() {
        let path = unique_db_path();
        // Usa dispatcher privado (isolado) para evitar race com init global de outros testes.
        let sender = init_telemetry_dispatcher_private(&path).unwrap();
        sender.dispatch_latency(LatencyPayload {
            tool: "agentgateway_ttft".to_string(),
            ttft_ms: 180.5,
            peak_ewma_ms: 245.7,
            tokens_in: 1000,
            tokens_out: 500,
            cost_usd: 0.015,
            session_id: Some("sess-test".to_string()),
        });
        // Aguarda flush do batch.
        std::thread::sleep(Duration::from_millis(50));
        let total = sum_today_cost_usd(&path).unwrap();
        assert!(total >= 0.015, "cost deve ser gravado: {}", total);
        std::fs::remove_file(&path).ok();
    }
}
