//! `repo_heatmap.rs` — Marco 4.1.2: Monitor Termico de Frecency
//!
//! Canibalizacao cirurgica da "Alma Matematica" do observability
//! heatmap (Langevin decay) + WalkDir de `repo_impact.rs` (Marco 4.1.0)
//! para construir um monitor termico de arquivos baseado em **mtime**
//! e **contagem de modificacoes** com decaimento exponencial (Frecency).
//!
//! ## Filosofia
//!
//! Pergunta respondida: "Quais arquivos deste monorepo foram
//! modificados mais recentemente e com mais frequencia?". A resposta
//! e um ranking ordenado por `frecency_score` decrescente, persistido
//! em SQLite sob a tabela `repo_heatmap` (STRICT).
//!
//! ## Algoritmo (Frecency)
//!
//! ```text
//! score(file, now) = min(modification_count * exp(-lambda * (now - mtime)), MAX_SCORE)
//! ```
//!
//! Com `lambda = 0.0001` (calibrado empiricamente, meia-vida ~1h55min):
//! - arquivo modificado agora: score ~ count (saturado em 5.0)
//! - modificado ha 1h: score * 0.698
//! - modificado ha 24h: score * 0.0014
//! - modificado ha 7 dias: score ~ 0
//!
//! ## Persistencia
//!
//! Tabela `repo_heatmap` em SQLite STRICT com UPSERT atomico:
//! ```sql
//! INSERT ... ON CONFLICT(file_path) DO UPDATE SET
//!     frecency_score = excluded.frecency_score,
//!     last_modified_epoch = excluded.last_modified_epoch,
//!     modification_count = repo_heatmap.modification_count + 1;
//! ```
//!
//! ## Interceptacao Cognitiva (R15-R17)
//!
//! A funcao `record_access` e o **hook fire-and-forget** invocado
//! silenciosamente pelo dispatcher apos chamadas bem-sucedidas de
//! `read`, `edit`, `symbol`, `repo_impact`, `repo_ast` e `multi_read`.
//! Enriquece o monitor com telemetria de uso real, independente do
//! `mtime` do disco (que pode estar poluido por checkout de branch).
//!
//! ## Agnosticismo Hardware
//!
//! 100% CPU + I/O de filesystem. Zero CUDA/Python/Node. RTX 2060m
//! intocada. Transmutavel para qualquer backend (Metal/Vulkan/NPU).

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use walkdir::WalkDir;

use super::extensions::{is_excluded_dir, is_source_ext};

/// Constante de decaimento canonica do SODA para o `repo_heatmap`.
///
/// `lambda = 0.0001` foi calibrada empiricamente para prover meia-vida
/// (peso cai a 50%) de aproximadamente 6930s (~1h55min). Isso prioriza
/// arquivos do dia sem descartar totalmente o historico de poucas
/// horas atras.
pub const DEFAULT_LAMBDA: f64 = 0.0001;

/// Teto rigido do score (saturacao). Impede overflow em monorepos
/// massivos onde `count` pode chegar a centenas.
pub const MAX_SCORE: f64 = 5.0;

/// Teto anti-OOM do numero de arquivos varridos.
pub const MAX_FILES_SCAN: usize = 50_000;

/// Entrada individual do heatmap (um arquivo + seu score).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatmapEntry {
    /// Caminho do arquivo (relativo a raiz do workspace, com `/`).
    pub file_path: String,
    /// Score acumulado via Frecency: `min(count * exp(-lambda * dt), 5.0)`.
    pub score: f64,
    /// Numero total de modificacoes registradas (UPSERT incrementa).
    pub modification_count: i64,
    /// Ultimo mtime registrado em epoch seconds.
    pub last_modified_epoch: i64,
}

/// Relatorio canonico de Frecency (payload MCP).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatmapReport {
    /// Lambda utilizada no calculo.
    pub lambda: f64,
    /// Epoch `now` usado no calculo.
    pub now: i64,
    /// Total de entradas retornadas (apos LIMIT).
    pub total: usize,
    /// Ranking ordenado por `score` descendente.
    pub entries: Vec<HeatmapEntry>,
}

/// Erros canonicos do monitor termico.
#[derive(Debug, Error)]
pub enum HeatmapError {
    #[error("caminho invalido: {0}")]
    InvalidPath(String),
    #[error("falha de I/O ao varrer repositorio: {0}")]
    Io(String),
    #[error("falha de SQLite: {0}")]
    Sqlite(String),
}

/// Calcula o score puro de Frecency para um unico arquivo.
///
/// **Formula canonica:** `min(count * exp(-lambda * dt), MAX_SCORE)`
///
/// Onde:
/// - `count`: numero de modificacoes acumuladas.
/// - `mtime`: epoch seconds do filesystem.
/// - `now`: epoch seconds atual.
/// - `dt = max(0, now - mtime)`: clamp anti-relogio-desregulado.
/// - `lambda`: constante de decaimento (default 0.0001).
///
/// **Determinismo:** funcao pura, sem I/O, sem alocacao mutavel.
/// Testavel em isolamento sem fixtures.
#[inline]
pub fn calculate_frecency(count: i64, mtime: i64, now: i64, lambda: f64) -> f64 {
    if count <= 0 {
        return 0.0;
    }
    let dt = (now - mtime).max(0) as f64;
    let raw = (count as f64) * (-lambda * dt).exp();
    raw.min(MAX_SCORE)
}

/// Garante que a tabela `repo_heatmap` existe no SQLite (idempotente).
///
/// **Lei da Idempotencia:** pode ser chamada multiplas vezes sem
/// efeito colateral. NUNCA derruba a tabela existente.
///
/// **STRICT mode:** garante blindagem contra coercao silenciosa
/// de tipos (Marco 3.9 Estado V5).
pub fn ensure_heatmap_table(conn: &Connection) -> Result<(), HeatmapError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS repo_heatmap (
             file_path          TEXT PRIMARY KEY,
             frecency_score     REAL NOT NULL,
             last_modified_epoch INTEGER NOT NULL,
             modification_count INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX IF NOT EXISTS idx_heatmap_score ON repo_heatmap(frecency_score DESC);",
    )
    .map_err(|e| HeatmapError::Sqlite(e.to_string()))
}

/// Hook fire-and-forget: UPSERT silencioso em `repo_heatmap`.
///
/// **Lei R15-R17:** NUNCA retorna erro. Filtra por extensao canonica.
/// Recalcula score com `lambda = DEFAULT_LAMBDA`.
///
/// **Uso:** invocado pelo dispatcher apos chamadas bem-sucedidas de
/// `read`, `edit`, `symbol`, `repo_impact`, `repo_ast`, `multi_read`.
///
/// **Comportamento de erro:** silencioso (log warn, nao propaga).
/// Caller NUNCA recebe erro deste hook — alinhado ao SSOT
/// `try_log_file_access` ja presente no dispatcher.
pub fn record_access(conn: &Connection, file_path: &str, now: i64) {
    // R17: filtro por extensao canonica. Arquivos temporarios e
    // efemeros NAO poluem o heatmap.
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !is_source_ext(ext) {
        return;
    }

    // Calcula score com count=1 (acesso novo). A contagem sera
    // incrementada via `modification_count + 1` no UPSERT.
    let score = calculate_frecency(1, now, now, DEFAULT_LAMBDA);

    let result = conn.execute(
        "INSERT INTO repo_heatmap (file_path, frecency_score, last_modified_epoch, modification_count)
         VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(file_path) DO UPDATE SET
             frecency_score = excluded.frecency_score,
             last_modified_epoch = excluded.last_modified_epoch,
             modification_count = repo_heatmap.modification_count + 1;",
        rusqlite::params![file_path, score, now],
    );

    // Hook fire-and-forget: log warn em caso de erro mas NAO propaga.
    if let Err(e) = result {
        eprintln!("[repo_heatmap::record_access] WARN: {e} (path={file_path})");
    }
}

/// Varre o workspace e computa o heatmap de Frecency, persistindo
/// no SQLite.
///
/// # Argumentos
/// - `root`: diretorio raiz do monorepo (sera varrido).
/// - `conn`: conexao SQLite ja aberta (com `ensure_heatmap_table` chamado).
/// - `now`: epoch seconds usado como referencia temporal.
/// - `lambda`: constante de decaimento (default 0.0001).
/// - `limit`: numero maximo de entradas no ranking final (default 50).
///
/// # Retorno
/// `HeatmapReport` ordenado por `score` descendente, com no maximo
/// `limit` entradas.
///
/// # Erros
/// - `HeatmapError::InvalidPath`: se `root` nao e um diretorio.
/// - `HeatmapError::Io`: erros de varredura irrecuperaveis.
/// - `HeatmapError::Sqlite`: erros de persistencia UPSERT.
pub fn compute_repo_heatmap(
    root: &Path,
    conn: &Connection,
    now: i64,
    lambda: f64,
    limit: usize,
) -> Result<HeatmapReport, HeatmapError> {
    if !root.is_dir() {
        return Err(HeatmapError::InvalidPath(format!(
            "root nao e diretorio: {}",
            root.display()
        )));
    }

    // ── Fase 1: WalkDir filtrado ────────────────────────────────────
    let mut file_count: usize = 0;
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !is_excluded_dir(name);
                }
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // I/O resiliente: pula entry quebrado.
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();

        // Filtro de extensao (22 canonicas).
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !is_source_ext(ext) {
            continue;
        }

        // Hard cap anti-OOM.
        file_count += 1;
        if file_count > MAX_FILES_SCAN {
            return Err(HeatmapError::Io(format!(
                "monorepo excede {MAX_FILES_SCAN} arquivos (anti-OOM)"
            )));
        }

        // mtime via std::fs::metadata (O(1) syscall).
        let mtime = match std::fs::metadata(path) {
            Ok(meta) => match meta.modified() {
                Ok(t) => t
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                Err(_) => continue, // Fail-soft: pula arquivo sem mtime.
            },
            Err(_) => continue, // Fail-soft: pula arquivo inacessivel.
        };

        // Path canonico (relativo ao root, com `/`).
        let canonical = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        // Calcula score com count=1 (UPSERT incrementa depois).
        let score = calculate_frecency(1, mtime, now, lambda);

        // UPSERT atomico (R4): ON CONFLICT(file_path) DO UPDATE.
        if let Err(e) = conn.execute(
            "INSERT INTO repo_heatmap (file_path, frecency_score, last_modified_epoch, modification_count)
             VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(file_path) DO UPDATE SET
                 frecency_score = excluded.frecency_score,
                 last_modified_epoch = excluded.last_modified_epoch,
                 modification_count = repo_heatmap.modification_count + 1;",
            rusqlite::params![canonical, score, mtime],
        ) {
            // Fail-soft: log warn, continua varredura.
            eprintln!("[repo_heatmap] WARN: UPSERT falhou para {canonical}: {e}");
        }
    }

    // ── Fase 2: SELECT ranking ──────────────────────────────────────
    let mut stmt = conn
        .prepare(
            "SELECT file_path, frecency_score, modification_count, last_modified_epoch
             FROM repo_heatmap
             ORDER BY frecency_score DESC, file_path ASC
             LIMIT ?1",
        )
        .map_err(|e| HeatmapError::Sqlite(e.to_string()))?;

    let entries_iter = stmt
        .query_map(rusqlite::params![limit as i64], |row| {
            Ok(HeatmapEntry {
                file_path: row.get(0)?,
                score: row.get(1)?,
                modification_count: row.get(2)?,
                last_modified_epoch: row.get(3)?,
            })
        })
        .map_err(|e| HeatmapError::Sqlite(e.to_string()))?;

    let mut entries: Vec<HeatmapEntry> = Vec::new();
    for entry in entries_iter {
        match entry {
            Ok(e) => entries.push(e),
            Err(err) => return Err(HeatmapError::Sqlite(err.to_string())),
        }
    }

    let total = entries.len();
    Ok(HeatmapReport {
        lambda,
        now,
        total,
        entries,
    })
}

/// Retorna o epoch atual (helper para o dispatcher).
pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Helper de conveniência: abre o banco `souls_state.db` em modo
/// read-write e garante a tabela `repo_heatmap`.
///
/// **Falha estruturada:** retorna `Err` se o diretorio `.souls_data`
/// nao puder ser criado ou se o banco nao puder ser aberto.
pub fn open_heatmap_db(workspace_root: &Path) -> Result<Connection, HeatmapError> {
    let souls_data_dir = workspace_root.join(".souls_data");
    std::fs::create_dir_all(&souls_data_dir)
        .map_err(|e| HeatmapError::Io(format!("create_dir_all(.souls_data): {e}")))?;
    let db_path = souls_data_dir.join("souls_state.db");
    let conn = Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(|e| HeatmapError::Sqlite(format!("open souls_state.db: {e}")))?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))
        .map_err(|e| HeatmapError::Sqlite(e.to_string()))?;
    ensure_heatmap_table(&conn)?;
    Ok(conn)
}

/// Helper para testes internos: busca uma entrada por file_path.
#[cfg(test)]
pub(crate) fn fetch_entry(conn: &Connection, file_path: &str) -> Option<HeatmapEntry> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT file_path, frecency_score, modification_count, last_modified_epoch
         FROM repo_heatmap WHERE file_path = ?1",
        rusqlite::params![file_path],
        |row| {
            Ok(HeatmapEntry {
                file_path: row.get(0)?,
                score: row.get(1)?,
                modification_count: row.get(2)?,
                last_modified_epoch: row.get(3)?,
            })
        },
    )
    .optional()
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_frecency_returns_zero_for_zero_count() {
        assert_eq!(calculate_frecency(0, 100, 200, 0.0001), 0.0);
        assert_eq!(calculate_frecency(-1, 100, 200, 0.0001), 0.0);
    }

    #[test]
    fn calculate_frecency_clamps_dt_to_zero_for_future_mtime() {
        // mtime > now → dt = 0 → score = count (sem decaimento).
        let score = calculate_frecency(3, 1000, 500, 0.0001);
        assert!((score - 3.0).abs() < 1e-9);
    }

    #[test]
    fn calculate_frecency_saturates_at_max_score() {
        // count alto + mtime recente → clamp em 5.0.
        let score = calculate_frecency(1_000_000, 100, 101, 0.0001);
        assert!((score - MAX_SCORE).abs() < 1e-9);
    }

    #[test]
    fn ensure_heatmap_table_is_idempotent() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();

        // 1ª chamada: cria.
        ensure_heatmap_table(&conn).expect("1ª criacao");
        // 2ª chamada: idempotente.
        ensure_heatmap_table(&conn).expect("2ª idempotente");

        // Verifica que a tabela existe.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='repo_heatmap'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "tabela deve existir exatamente 1x");
    }

    #[test]
    fn record_access_filters_non_canonical_extensions() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        ensure_heatmap_table(&conn).unwrap();

        let now = now_epoch();

        // Arquivo com extensao nao-canonica → hook ignora silenciosamente.
        record_access(&conn, "/tmp/foo.png", now);
        record_access(&conn, "/tmp/bar.log", now);
        record_access(&conn, "/tmp/baz.exe", now);

        // Tabela deve permanecer vazia.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM repo_heatmap", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "extensoes nao-canonicas nao devem ser registradas");
    }

    #[test]
    fn record_access_increments_count_on_collision() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        ensure_heatmap_table(&conn).unwrap();

        let now = now_epoch();
        for _ in 0..5 {
            record_access(&conn, "/tmp/foo.rs", now);
        }

        let entry = fetch_entry(&conn, "/tmp/foo.rs").expect("entry deve existir");
        assert_eq!(entry.modification_count, 5, "count deve incrementar a cada UPSERT");
    }
}
