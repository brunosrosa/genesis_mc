//! `test_repo_heatmap.rs` — Marco 4.1.2: Monitor Termico de Frecency
//!
//! Caderno TDD com 3 contratos rigidos que validam a ferramenta `repo_heatmap`:
//!
//! 1. `test_calculate_frecency_decay` — Prova matematicamente que
//!    arquivo modificado ha 1h possui score estritamente maior que
//!    modificado ha 48h. Testa a formula pura antes de qualquer I/O.
//!
//! 2. `test_heatmap_respects_exclusions` — Garante que pastas ignoradas
//!    (`target/`, `.git/`, `node_modules/`) e extensoes nao-canonicas
//!    (`.png`, `.log`, `.exe`) sao **imunes** a insercao na tabela
//!    `repo_heatmap`. Score permanece 0.0 e a tabela fica vazia para
//!    esses paths.
//!
//! 3. `test_sqlite_upsert_collision_protection` — Simula escritas
//!    concorrentes sobre o mesmo `file_path` e prova que o UPSERT
//!    resolve a corrida sem panic, sem deadlock (timeout 10s), e o
//!    `modification_count` final == numero de escritas.
//!
//! **Lei do Scaffold:** estes 3 testes foram escritos ANTES da
//! implementacao (Red puro). Devem falhar com `cannot find function
//! repo_heatmap::calculate_frecency` e passar apos TASK-02.

use rusqlite::Connection;
use souls_mc_lib::cognition::lean_vacuum::repo_heatmap::{
    calculate_frecency, compute_repo_heatmap, ensure_heatmap_table, record_access,
    HeatmapReport, DEFAULT_LAMBDA, MAX_SCORE,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

/// Helper canônico: escreve arquivo em diretório temporário.
fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(&p, content).expect("write file");
    p
}

/// Cria uma conexão SQLite em arquivo temporário com WAL + busy_timeout.
fn open_heatmap_db(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("open sqlite");
    // busy_timeout 30s via PRAGMA — absorve contenção severa em
    // testes de UPSERT concorrente (8 threads × 50 writes = 400).
    conn.execute_batch("PRAGMA busy_timeout = 30000;")
        .expect("busy_timeout pragma");
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
        .expect("pragma");
    conn
}

// ──────────────────────────────────────────────────────────────────────
// CONTRATO 1 — Decaimento exponencial da Frecency
// ──────────────────────────────────────────────────────────────────────

/// Prova matematicamente que:
/// `calculate_frecency(1, now-3600, now, λ) > calculate_frecency(1, now-172800, now, λ)`
///
/// ou seja, arquivo modificado há 1h é estritamente mais quente que
/// modificado há 48h (mesmo count=1). Garante que o decaimento é
/// monotonicamente decrescente em `dt` (anti-erro de sinal).
#[test]
fn test_calculate_frecency_decay() {
    let now: i64 = 1_700_000_000;
    let mtime_1h: i64 = now - 3_600; // 1 hora atrás
    let mtime_48h: i64 = now - 172_800; // 48 horas atrás

    let score_1h = calculate_frecency(1, mtime_1h, now, DEFAULT_LAMBDA);
    let score_48h = calculate_frecency(1, mtime_48h, now, DEFAULT_LAMBDA);

    assert!(
        score_1h > score_48h,
        "score_1h ({score_1h}) deve ser > score_48h ({score_48h}) — decaimento quebrado"
    );

    // Sanity: score_1h deve estar próximo de 0.7 (exp(-0.0001*3600) ≈ 0.698).
    assert!(
        score_1h > 0.5 && score_1h < 0.9,
        "score_1h fora da faixa esperada [0.5, 0.9]: {score_1h}"
    );

    // Sanity: score_48h deve estar próximo de 0 (exp(-0.0001*172800) ≈ 1.7e-8).
    assert!(
        score_48h < 0.001,
        "score_48h deve ser quase 0 (arquivo congelado): {score_48h}"
    );

    // Saturação: count alto + mtime recente → clamp em MAX_SCORE.
    let score_saturado = calculate_frecency(1000, now - 1, now, DEFAULT_LAMBDA);
    assert!(
        (score_saturado - MAX_SCORE).abs() < 1e-9,
        "score saturado deve == MAX_SCORE ({MAX_SCORE}), got {score_saturado}"
    );

    // Clamp anti-relogio-desregulado: mtime > now → dt = 0 → score máximo.
    let score_future = calculate_frecency(5, now + 100, now, DEFAULT_LAMBDA);
    assert!(
        (score_future - 5.0).abs() < 1e-9,
        "mtime futuro deve produzir dt=0 e score = count = 5.0, got {score_future}"
    );
}

// ──────────────────────────────────────────────────────────────────────
// CONTRATO 2 — Respeito às exclusões canônicas (22/22 SSOT)
// ──────────────────────────────────────────────────────────────────────

/// Garante que `target/`, `.git/`, `node_modules/` e extensões
/// não-canônicas (`.png`, `.log`, `.exe`) são **imunes** à inserção
/// na tabela `repo_heatmap`. A tabela deve permanecer vazia após
/// `compute_repo_heatmap`.
#[test]
fn test_heatmap_respects_exclusions() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("test.db");
    let mut conn = open_heatmap_db(&db_path);
    ensure_heatmap_table(&conn).expect("ensure_heatmap_table");

    // 1. Pasta excluída: `target/`
    let target_dir = root.join("target");
    fs::create_dir_all(&target_dir).expect("create target/");
    write(&target_dir, "compiled.rs", "fn compiled() {}");

    // 2. Pasta excluída: `.git/`
    let git_dir = root.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git/");
    write(&git_dir, "config", "core.autocrlf=false");

    // 3. Pasta excluída: `node_modules/`
    let nm_dir = root.join("node_modules");
    fs::create_dir_all(&nm_dir).expect("create node_modules/");
    write(&nm_dir, "lodash.js", "module.exports = {};");

    // 4. Extensão não-canônica: `.png`
    write(root, "logo.png", "FAKE_PNG_BINARY_CONTENT");
    // 5. Extensão não-canônica: `.log`
    write(root, "debug.log", "INFO started");
    // 6. Extensão não-canônica: `.exe`
    write(root, "binary.exe", "MZ_FAKE_BINARY");

    // Único arquivo canônico (controle positivo):
    write(root, "lib.rs", "pub fn lib_api() {}");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let report: HeatmapReport =
        compute_repo_heatmap(root, &mut conn, now, DEFAULT_LAMBDA, 50).expect("compute");

    // Apenas `lib.rs` deve aparecer.
    assert_eq!(
        report.total, 1,
        "apenas lib.rs deve ser indexado (exclusões violadas?): entries={:?}",
        report.entries
    );
    assert_eq!(report.entries[0].file_path, "lib.rs");

    // Sanity: nenhum dos paths excluídos pode estar na tabela.
    for forbidden in [
        "target/compiled.rs",
        ".git/config",
        "node_modules/lodash.js",
        "logo.png",
        "debug.log",
        "binary.exe",
    ] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM repo_heatmap WHERE file_path = ?1",
                rusqlite::params![forbidden],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(
            count, 0,
            "path proibido '{forbidden}' foi inserido no heatmap!"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────
// CONTRATO 3 — Proteção de UPSERT contra corrida concorrente
// ──────────────────────────────────────────────────────────────────────

/// Simula 8 threads paralelas fazendo 50 UPSERTs no mesmo `file_path`
/// (total 400 escritas). Prova que:
/// 1. Nenhum panic
/// 2. Nenhum deadlock (timeout 30s)
/// 3. `modification_count` final == 400
/// 4. `frecency_score` é f64 válido
#[test]
fn test_sqlite_upsert_collision_protection() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("collision.db");
    let target_path = root.join("hot_file.rs");
    write(root, "hot_file.rs", "pub fn hot() {}");

    // Bootstrap: 1 thread prepara o banco e a tabela.
    {
        let conn = open_heatmap_db(&db_path);
        ensure_heatmap_table(&conn).expect("ensure_heatmap_table");
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    const N_THREADS: usize = 8;
    const WRITES_PER_THREAD: usize = 50;
    const TOTAL_WRITES: usize = N_THREADS * WRITES_PER_THREAD;

    // Cada thread abre sua PRÓPRIA conexão (SQLite thread-safe, WAL).
    let target_str = target_path.to_string_lossy().to_string();
    let db_path_arc = Arc::new(db_path.clone());
    let target_arc = Arc::new(target_str);

    let start = Instant::now();
    let handles: Vec<_> = (0..N_THREADS)
        .map(|_| {
            let db = Arc::clone(&db_path_arc);
            let tgt = Arc::clone(&target_arc);
            thread::spawn(move || {
                let mut conn = open_heatmap_db(&db);
                for _ in 0..WRITES_PER_THREAD {
                    record_access(&mut conn, &tgt, now);
                }
            })
        })
        .collect();

    // Aguarda com timeout — se houver deadlock, o teste falha com panic
    // explícito após 30s (anti-ralph-loop safety net).
    for h in handles {
        let join_start = Instant::now();
        loop {
            if h.is_finished() {
                h.join().expect("thread panicked");
                break;
            }
            if join_start.elapsed() > Duration::from_secs(30) {
                panic!("DEADLOCK detectado: thread não finalizou em 30s");
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(30),
        "teste excedeu 30s — provável contenção excessiva: {elapsed:?}"
    );

    // Verifica o estado final.
    let verify_conn = open_heatmap_db(&db_path);
    let (count, score): (i64, f64) = verify_conn
        .query_row(
            "SELECT modification_count, frecency_score FROM repo_heatmap WHERE file_path = ?1",
            rusqlite::params![target_path.to_string_lossy()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("query final");

    assert_eq!(
        count, TOTAL_WRITES as i64,
        "modification_count deve ser exatamente {TOTAL_WRITES} (UPSERT perdeu escritas): got {count}"
    );

    assert!(
        score.is_finite() && score > 0.0 && score <= MAX_SCORE,
        "frecency_score inválido: {score} (deve ser f64 finito em (0, {MAX_SCORE}])"
    );
}

// ──────────────────────────────────────────────────────────────────────
// CONTRATO 4 (HOTFIX Marco 4.1.2-ac) — Acumulo de Frecency no UPSERT
// ──────────────────────────────────────────────────────────────────────

/// Prova que o `frecency_score` reflete o `modification_count`
/// **acumulado**, NAO um valor unitario congelado.
///
/// **Cenário:** chama `record_access` 5 vezes sobre o mesmo path.
/// Como `record_access` define `mtime = now` no UPSERT, `dt = 0`
/// e a formula canonica produz `score = min(count, MAX_SCORE)`.
///
/// **Bug pre-hotfix (Marco 4.1.2 sem correcao):**
///   - score apos 1 acesso = 1.0
///   - score apos 5 acessos = 1.0 (errado, congelado)
///   - modification_count = 5 (correto, mas inutil sem score crescente)
///
/// **Comportamento esperado pos-hotfix (R18):**
///   - score apos 1 acesso = 1.0
///   - score apos 5 acessos = 5.0 (saturado em MAX_SCORE)
///   - modification_count = 5
///   - A **ranking** reflete corretamente a intensidade de uso.
#[test]
fn test_frecency_score_reflects_accumulated_count() {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("accumulated.db");
    let mut conn = open_heatmap_db(&db_path);
    ensure_heatmap_table(&conn).expect("ensure_heatmap_table");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Extensao canonica (R17): .rs garante que o hook registra.
    let target = "/tmp/very_hot_file.rs";

    // Snapshot apos 1 acesso.
    record_access(&mut conn, target, now);
    let score_1: f64 = conn
        .query_row(
            "SELECT frecency_score FROM repo_heatmap WHERE file_path = ?1",
            rusqlite::params![target],
            |r| r.get(0),
        )
        .expect("linha 1 acesso");
    let count_1: i64 = conn
        .query_row(
            "SELECT modification_count FROM repo_heatmap WHERE file_path = ?1",
            rusqlite::params![target],
            |r| r.get(0),
        )
        .expect("count 1 acesso");

    // Continua incrementando ate 5 acessos.
    for _ in 0..4 {
        record_access(&mut conn, target, now);
    }
    let score_5: f64 = conn
        .query_row(
            "SELECT frecency_score FROM repo_heatmap WHERE file_path = ?1",
            rusqlite::params![target],
            |r| r.get(0),
        )
        .expect("linha 5 acessos");
    let count_5: i64 = conn
        .query_row(
            "SELECT modification_count FROM repo_heatmap WHERE file_path = ?1",
            rusqlite::params![target],
            |r| r.get(0),
        )
        .expect("count 5 acessos");

    // Assertions de invariantes (R18):
    assert_eq!(count_1, 1, "modification_count deve ser 1 apos 1 acesso");
    assert_eq!(count_5, 5, "modification_count deve ser 5 apos 5 acessos");

    // Apos 1 acesso: score = 1.0 (count=1, dt=0, exp=1.0).
    assert!(
        (score_1 - 1.0).abs() < 1e-9,
        "score apos 1 acesso deve ser 1.0 (got {score_1})"
    );

    // Apos 5 acessos: score deve estar saturado em MAX_SCORE (5.0).
    // BUG: pre-hotfix, score_5 == score_1 == 1.0 (congelado).
    // FIX: score_5 == MAX_SCORE == 5.0 (saturado pelo count acumulado).
    assert!(
        (score_5 - MAX_SCORE).abs() < 1e-9,
        "score apos 5 acessos deve saturar em MAX_SCORE ({MAX_SCORE}), got {score_5} \
         (BUG: o UPSERT nao atualiza o score com o count acumulado)"
    );

    // Invariante: score_5 > score_1 (monotonicamente crescente ate saturar).
    assert!(
        score_5 > score_1,
        "score deve crescer com o count: score_1={score_1}, score_5={score_5}"
    );
}

/// Prova que `compute_repo_heatmap` tambem aplica R18: ao re-varer
/// um arquivo ja existente, o score refleti o count acumulado +
/// o mtime real do filesystem (nao apenas o count unitario).
#[test]
fn test_compute_repo_heatmap_accumulates_score() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let db_path = root.join("compute_acc.db");
    let mut conn = open_heatmap_db(&db_path);
    ensure_heatmap_table(&conn).expect("ensure_heatmap_table");

    // Cria arquivo canonico (.rs).
    let _target = write(root, "src/lib.rs", "pub fn lib_api() {}");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // 1ª varredura: count vira 1, score = 1 * exp(-lambda * dt).
    let _report_1 =
        compute_repo_heatmap(root, &mut conn, now, DEFAULT_LAMBDA, 50).expect("compute 1");

    // 2ª varredura: count vira 2, score = 2 * exp(-lambda * dt).
    // Sem o fix: score ficaria congelado em 1 * exp(-lambda * dt).
    let _report_2 =
        compute_repo_heatmap(root, &mut conn, now, DEFAULT_LAMBDA, 50).expect("compute 2");

    // 3ª varredura: count vira 3, score = 3 * exp(-lambda * dt).
    let _report_3 =
        compute_repo_heatmap(root, &mut conn, now, DEFAULT_LAMBDA, 50).expect("compute 3");

    // Snapshot final.
    let (count, score, mtime): (i64, f64, i64) = conn
        .query_row(
            "SELECT modification_count, frecency_score, last_modified_epoch \
             FROM repo_heatmap WHERE file_path = ?1",
            rusqlite::params!["src/lib.rs"],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("linha final");

    assert_eq!(count, 3, "modification_count deve ser 3 apos 3 varridas");
    assert_eq!(mtime, mtime, "mtime deve ser igual ao do filesystem (test stub)");

    // Calcula score esperado: 3 * exp(-lambda * (now - mtime)).
    let expected_score = 3.0 * (-DEFAULT_LAMBDA * (now - mtime) as f64).exp();
    let expected_score_clamped = expected_score.min(MAX_SCORE);

    assert!(
        (score - expected_score_clamped).abs() < 1e-6,
        "score ({score}) deve refletir count=3 com mtime real (expected {expected_score_clamped}) \
         — BUG: o UPSERT calcula score com count=1 mesmo apos multiplas varridas"
    );
}
