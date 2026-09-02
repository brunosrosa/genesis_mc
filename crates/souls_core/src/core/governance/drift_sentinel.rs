// SOULS V6 — Core Engine: Olheiro de Drift Reativo de Fase -1 (Task 138 / ADR-010 / ADR-025)
//
// Operação em repouso tático com verificação ultrarrápida UDP (DNS 1.1.1.1:53)
// e portão de cooldown de 24 horas sobre `repo_heuristics` para proteção FinOps de cotas.

use std::path::Path;
use std::time::Duration;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

pub const DRIFT_INTERVAL_SECS: u64 = 3600; // 1 hora
pub const DRIFT_COOLDOWN_SECS: i64 = 86400; // 24 horas

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoDriftCandidate {
    pub repo_url: String,
    pub repo_version: String,
    pub online_version: Option<String>,
}

/// Sondagem de conectividade de rede ultrarrápida e não-bloqueante via UDP.
///
/// Tenta abrir um soquete UDP local ('0.0.0.0:0') e conectar temporariamente ao IP público
/// "1.1.1.1:53" (DNS Cloudflare) com timeout estrito de 200ms.
/// Retorna `true` se houver rota ativa; retorna `false` silenciosamente em modo de repouso tático offline.
pub async fn check_internet_udp() -> bool {
    let socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[DRIFT_SENTINEL] Sistema offline ou restrito. Olheiro em repouso tático.");
            return false;
        }
    };
    let timeout = Duration::from_millis(200);
    match tokio::time::timeout(timeout, socket.connect("1.1.1.1:53")).await {
        Ok(Ok(_)) => true,
        _ => {
            eprintln!("[DRIFT_SENTINEL] Sistema offline ou restrito. Olheiro em repouso tático.");
            false
        }
    }
}

/// Avalia se a última análise de um repositório está dentro do período de cooldown de 24 horas.
/// Retorna `true` se a requisição externa DEVE SER BLOQUEADA (cooldown ativo), `false` se pode prosseguir.
pub fn is_within_cooldown_24h(last_analyzed_epoch: i64, current_epoch: i64) -> bool {
    if last_analyzed_epoch <= 0 {
        return false;
    }
    (current_epoch - last_analyzed_epoch) < DRIFT_COOLDOWN_SECS
}

/// Varre o banco SQLite buscando candidatos a verificação de drift que não estejam em cooldown.
pub fn fetch_drift_candidates(conn: &Connection, current_epoch: i64) -> Result<Vec<RepoDriftCandidate>, rusqlite::Error> {
    let cutoff_seconds = current_epoch - DRIFT_COOLDOWN_SECS;

    let query = "SELECT r.repo_url, rh.repo_version, rh.ultima_versao_online \
                 FROM repositorios r \
                 JOIN repo_heuristics rh ON (r.repo_url = rh.solution_id OR r.project_name = rh.project_name) \
                 WHERE (r.status_processamento IN ('PENDENTE', 'F0_OK') OR rh.status_atualizacao IN ('PENDENTE', 'F0_OK', 'CONCLUIDO')) \
                   AND (rh.data_ultima_analise IS NULL OR rh.data_ultima_analise = 0 OR rh.data_ultima_analise < ?1)";

    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map([cutoff_seconds], |row| {
        Ok(RepoDriftCandidate {
            repo_url: row.get(0)?,
            repo_version: row.get(1)?,
            online_version: row.get(2)?,
        })
    })?;

    let candidates = rows.flatten().collect();
    Ok(candidates)
}


/// Aplica a transição de estado de drift quando detectada versão mais recente online.
pub fn record_repo_drift(
    conn: &Connection,
    repo_url: &str,
    online_version: &str,
    analyzed_at_epoch: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE repo_heuristics SET \
            ultima_versao_online = ?1, \
            status_atualizacao = 'PENDENTE_FASE_0', \
            data_ultima_analise = ?2 \
         WHERE solution_id = ?3 OR project_name = (SELECT project_name FROM repositorios WHERE repo_url = ?3 OR project_name = ?3)",
        rusqlite::params![online_version, analyzed_at_epoch, repo_url],
    )?;

    conn.execute(
        "UPDATE repositorios SET \
            status_processamento = 'PENDENTE' \
         WHERE repo_url = ?1 OR project_name = ?1",
        rusqlite::params![repo_url],
    )?;

    Ok(())
}

/// Inicia o olheiro reativo assíncrono sob intervalo de 1 hora sem bloquear o bootstrap.
pub fn spawn_reactive_drift_sentinel(db_path: impl AsRef<Path> + Send + 'static) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(DRIFT_INTERVAL_SECS));
        loop {
            interval.tick().await;

            if !check_internet_udp().await {
                continue;
            }

            let path = db_path.as_ref();
            let Ok(conn) = Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
            ) else {
                continue;
            };
            let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let _ = fetch_drift_candidates(&conn, now);
        }
    })
}
