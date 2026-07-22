use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tracing::{info, warn};

use souls_mc_lib::persist::ssot_injector::ReqwestGoogleWorkspaceSheetsClient;
use souls_mc_lib::telemetry::{enable_virtual_terminal, init_cli_tracing, parse_log_level_from_env};

const MASTER_SOLUTIONS_SHEET: &str = "MASTER_SOLUTIONS";

#[cfg(not(test))]
const POST_BATCH_WRITE_DELAY: Duration = Duration::from_millis(1_500);
#[cfg(test)]
const POST_BATCH_WRITE_DELAY: Duration = Duration::from_millis(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsyncedRepoRow {
    pub project_name: String,
    pub repo_url: String,
    pub lote_id: String,
    pub status_processamento: String,
    pub repo_version: String,
    pub ultima_versao_online: String,
    pub proposta_original_resumo: String,
    pub categoria_arquitetural: String,
}

pub type SheetsDataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<String>>, String>> + Send + 'a>>;
pub type SheetsFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

pub trait OutboxSheetsClient: Send + Sync {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a>;

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a>;
}

impl OutboxSheetsClient for ReqwestGoogleWorkspaceSheetsClient {
    fn get_sheet_data<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        range: String,
    ) -> SheetsDataFuture<'a> {
        souls_mc_lib::persist::ssot_injector::SheetsClient::get_sheet_data(
            self,
            spreadsheet_id,
            sheet,
            range,
        )
    }

    fn batch_update_cells<'a>(
        &'a self,
        spreadsheet_id: &'a str,
        sheet: &'a str,
        ranges: Value,
    ) -> SheetsFuture<'a> {
        souls_mc_lib::persist::ssot_injector::SheetsClient::batch_update_cells(
            self,
            spreadsheet_id,
            sheet,
            ranges,
        )
    }
}

pub struct OutboxSynchronizer<S: OutboxSheetsClient> {
    sheets: Arc<S>,
    db_path: PathBuf,
    spreadsheet_id: String,
}

impl<S: OutboxSheetsClient> OutboxSynchronizer<S> {
    pub fn new(sheets: Arc<S>, db_path: PathBuf, spreadsheet_id: String) -> Self {
        Self {
            sheets,
            db_path,
            spreadsheet_id,
        }
    }

    fn col_idx_to_a1(col_idx0: usize) -> String {
        let mut n = col_idx0 + 1;
        let mut out = String::new();
        while n > 0 {
            let rem = (n - 1) % 26;
            out.insert(0, (b'A' + rem as u8) as char);
            n = (n - 1) / 26;
        }
        out
    }

    pub async fn sync_once(&self) -> Result<usize, String> {
        let db_path = self.db_path.clone();

        // 1. LEITURA DE LINHAS DESSINCRONIZADAS DO SQLITE (sheets_synced = 0)
        let unsynced_rows: Vec<UnsyncedRepoRow> = tokio::task::spawn_blocking(move || -> Result<Vec<UnsyncedRepoRow>, String> {
            let conn = Connection::open(&db_path)
                .map_err(|e| format!("Outbox: falha ao abrir SQLite: {}", e))?;

            // Garantia de Migração Idempotente de Colunas
            let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN sheets_synced INTEGER DEFAULT 0", []);
            let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN proposta_original_resumo TEXT", []);
            let _ = conn.execute("ALTER TABLE repositorios ADD COLUMN categoria_arquitetural TEXT", []);

            let mut stmt = conn
                .prepare(
                    "SELECT project_name, repo_url, lote_id, status_processamento,
                            COALESCE(repo_version, ''), COALESCE(ultima_versao_online, ''),
                            COALESCE(proposta_original_resumo, ''), COALESCE(categoria_arquitetural, '')
                     FROM repositorios
                     WHERE sheets_synced = 0 OR sheets_synced IS NULL",
                )
                .map_err(|e| format!("Outbox: erro ao consultar SQLite: {e}"))?;

            let rows = stmt
                .query_map([], |r| {
                    Ok(UnsyncedRepoRow {
                        project_name: r.get(0)?,
                        repo_url: r.get(1)?,
                        lote_id: r.get(2)?,
                        status_processamento: r.get(3)?,
                        repo_version: r.get(4)?,
                        ultima_versao_online: r.get(5)?,
                        proposta_original_resumo: r.get(6)?,
                        categoria_arquitetural: r.get(7)?,
                    })
                })
                .map_err(|e| format!("Outbox: erro na leitura do cursor: {e}"))?;

            let mut out = Vec::new();
            for item in rows {
                out.push(item.map_err(|e| e.to_string())?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| format!("Join error: {e}"))??;

        if unsynced_rows.is_empty() {
            info!("Outbox: Nenhuma linha pendente de sincronização no SQLite.");
            return Ok(0);
        }

        // 2. BULK READ O(1) DAS COLUNAS DE IDENTIFICAÇÃO NO GOOGLE SHEETS
        let sheet_data = self
            .sheets
            .get_sheet_data(&self.spreadsheet_id, MASTER_SOLUTIONS_SHEET, "A1:Z2000".to_string())
            .await?;

        if sheet_data.is_empty() {
            return Err("Outbox: Planilha Google Sheets vazia".to_string());
        }

        let header_row = &sheet_data[0];

        let repo_url_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("repo_url"))
            .unwrap_or(1);

        let lote_id_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("lote_id"))
            .unwrap_or(4);

        let status_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("status_fase") || h.trim().eq_ignore_ascii_case("status_processamento"));

        let version_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("ultima_versao_online") || h.trim().eq_ignore_ascii_case("repo_version"));

        let resumo_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("proposta_original_resumo"));

        let categoria_idx = header_row
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case("categoria_arquitetural"));

        // Montagem do HashMap de Indexação O(1): Key = (repo_url, lote_id) -> row_number_1based
        let mut index_map: HashMap<(String, String), u32> = HashMap::new();
        for (i, row) in sheet_data.iter().enumerate().skip(1) {
            let row_number = (i + 1) as u32;
            let url = row.get(repo_url_idx).map(|s| s.trim().to_lowercase()).unwrap_or_default();
            let lote = row.get(lote_id_idx).map(|s| s.trim().to_lowercase()).unwrap_or_default();
            if !url.is_empty() {
                index_map.insert((url, lote), row_number);
            }
        }

        // 3. EMPACOTAMENTO ATÔMICO DE PAYLOAD BATCH
        let mut batch_map = serde_json::Map::new();
        let mut synced_urls: Vec<String> = Vec::new();

        for row in unsynced_rows {
            let key = (row.repo_url.trim().to_lowercase(), row.lote_id.trim().to_lowercase());
            let Some(&row_number) = index_map.get(&key) else {
                warn!(
                    repo_url = %row.repo_url,
                    lote_id = %row.lote_id,
                    "Outbox: repositório não encontrado na planilha do Sheets para sincronização"
                );
                continue;
            };

            // Atualização de Status
            if let Some(idx) = status_idx {
                let col = Self::col_idx_to_a1(idx);
                let range = format!("{col}{row_number}:{col}{row_number}");
                batch_map.insert(range, json!([[row.status_processamento]]));
            }

            // Atualização de Versão
            if let Some(idx) = version_idx {
                let col = Self::col_idx_to_a1(idx);
                let range = format!("{col}{row_number}:{col}{row_number}");
                let ver = if !row.ultima_versao_online.is_empty() {
                    &row.ultima_versao_online
                } else {
                    &row.repo_version
                };
                if !ver.is_empty() {
                    batch_map.insert(range, json!([[ver]]));
                }
            }

            // Atualização de Resumo
            if let Some(idx) = resumo_idx {
                if !row.proposta_original_resumo.is_empty() {
                    let col = Self::col_idx_to_a1(idx);
                    let range = format!("{col}{row_number}:{col}{row_number}");
                    batch_map.insert(range, json!([[row.proposta_original_resumo]]));
                }
            }

            // Atualização de Categoria
            if let Some(idx) = categoria_idx {
                if !row.categoria_arquitetural.is_empty() {
                    let col = Self::col_idx_to_a1(idx);
                    let range = format!("{col}{row_number}:{col}{row_number}");
                    batch_map.insert(range, json!([[row.categoria_arquitetural]]));
                }
            }

            synced_urls.push(row.repo_url);
        }

        if batch_map.is_empty() {
            info!("Outbox: Nenhuma alteração pendente mapeada para despacho no Sheets.");
            return Ok(0);
        }

        let synced_count = synced_urls.len();

        // 4. ENVIO ÚNICO BATCH UPDATE
        self.sheets
            .batch_update_cells(&self.spreadsheet_id, MASTER_SOLUTIONS_SHEET, Value::Object(batch_map))
            .await?;

        // 5. ATUALIZAÇÃO NO SQLITE PARA sheets_synced = 1
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let conn = Connection::open(&db_path)
                .map_err(|e| format!("Outbox: falha ao abrir SQLite para confirmação: {e}"))?;

            for url in synced_urls {
                conn.execute(
                    "UPDATE repositorios SET sheets_synced = 1 WHERE repo_url = ?1",
                    params![url],
                )
                .map_err(|e| format!("Outbox: falha ao atualizar sheets_synced: {e}"))?;
            }
            Ok(())
        })
        .await
        .map_err(|e| format!("Join error: {e}"))??;

        // 6. CADÊNCIA ANTI-429 JITTER SLEEP OBRIGATÓRIO (1.500ms)
        info!(
            synced_count,
            delay_ms = POST_BATCH_WRITE_DELAY.as_millis(),
            "Outbox: batchUpdate enviado com sucesso para o Sheets. Aplicando cadência anti-429..."
        );
        tokio::time::sleep(POST_BATCH_WRITE_DELAY).await;

        Ok(synced_count)
    }
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    #[cfg(windows)]
    let _ = enable_ansi_support::enable_ansi_support();
    enable_virtual_terminal();
    let level = parse_log_level_from_env();
    init_cli_tracing(level);

    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    let sheets_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Falta a variável de ambiente GOOGLE_SHEETS_ID"))?;

    let sheets_client = Arc::new(ReqwestGoogleWorkspaceSheetsClient);
    let synchronizer = OutboxSynchronizer::new(sheets_client, db_path, sheets_id);

    let synced = synchronizer.sync_once().await.map_err(io::Error::other)?;
    info!(synced, "Outbox Synchronizer: execução concluída");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::sync::Mutex;

    struct MockSheetsClient {
        get_data_calls: Mutex<usize>,
        batch_update_calls: Mutex<usize>,
        last_batch_payload: Mutex<Option<Value>>,
    }

    impl OutboxSheetsClient for MockSheetsClient {
        fn get_sheet_data<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            _range: String,
        ) -> SheetsDataFuture<'a> {
            *self.get_data_calls.lock().unwrap() += 1;
            Box::pin(async move {
                // Header + row 2
                Ok(vec![
                    vec![
                        "project_name".to_string(),
                        "repo_url".to_string(),
                        "status_fase".to_string(),
                        "repo_version".to_string(),
                        "lote_id".to_string(),
                        "proposta_original_resumo".to_string(),
                        "categoria_arquitetural".to_string(),
                    ],
                    vec![
                        "acme/widget".to_string(),
                        "https://github.com/acme/widget".to_string(),
                        "PENDENTE".to_string(),
                        "v1.0.0".to_string(),
                        "L1".to_string(),
                        "".to_string(),
                        "".to_string(),
                    ],
                ])
            })
        }

        fn batch_update_cells<'a>(
            &'a self,
            _spreadsheet_id: &'a str,
            _sheet: &'a str,
            ranges: Value,
        ) -> SheetsFuture<'a> {
            *self.batch_update_calls.lock().unwrap() += 1;
            *self.last_batch_payload.lock().unwrap() = Some(ranges);
            Box::pin(async move { Ok(()) })
        }
    }

    fn setup_test_db(db_path: &Path) -> Connection {
        let conn = Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE repositorios (
                project_name TEXT PRIMARY KEY,
                lote_id TEXT NOT NULL,
                repo_url TEXT NOT NULL UNIQUE,
                repo_analised_version TEXT,
                repo_version TEXT,
                ultima_versao_online TEXT,
                soda_universal_uuid TEXT NOT NULL UNIQUE,
                status_processamento TEXT NOT NULL,
                timestamp_fase_1 INTEGER,
                timestamp_fase_3 INTEGER,
                retry_count INTEGER NOT NULL,
                proposta_original_resumo TEXT,
                categoria_arquitetural TEXT,
                sheets_synced INTEGER DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[tokio::test]
    async fn test_outbox_bulk_read_and_batch_update_sync() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count, proposta_original_resumo, categoria_arquitetural, sheets_synced
            ) VALUES (
                'acme/widget', 'L1', 'https://github.com/acme/widget', 'v1.0.0', 'v1.0.0', 'v1.0.0',
                'UUID-1', 'PENDENTE_HARVESTER', 0, 'Resumo técnico da ferramenta.', 'Tooling_Dev - CLI_Utilities', 0
            )",
            [],
        )
        .unwrap();
        drop(conn);

        let mock_sheets = Arc::new(MockSheetsClient {
            get_data_calls: Mutex::new(0),
            batch_update_calls: Mutex::new(0),
            last_batch_payload: Mutex::new(None),
        });

        let sync = OutboxSynchronizer::new(
            mock_sheets.clone(),
            tmp.path().to_path_buf(),
            "SHEET_ID_TEST".to_string(),
        );

        let count = sync.sync_once().await.unwrap();

        assert_eq!(count, 1);

        let conn = Connection::open(tmp.path()).unwrap();
        let synced: i32 = conn
            .query_row(
                "SELECT sheets_synced FROM repositorios WHERE project_name = 'acme/widget'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(synced, 1);
        assert_eq!(*mock_sheets.get_data_calls.lock().unwrap(), 1); // 1 Bulk Read O(1)
        assert_eq!(*mock_sheets.batch_update_calls.lock().unwrap(), 1); // 1 Batch Update
    }

    #[tokio::test]
    async fn test_outbox_sync_skips_when_no_unsynced_rows() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = setup_test_db(tmp.path());
        conn.execute(
            "INSERT INTO repositorios (
                project_name, lote_id, repo_url, repo_analised_version, repo_version, ultima_versao_online,
                soda_universal_uuid, status_processamento, retry_count, proposta_original_resumo, categoria_arquitetural, sheets_synced
            ) VALUES (
                'acme/widget', 'L1', 'https://github.com/acme/widget', 'v1.0.0', 'v1.0.0', 'v1.0.0',
                'UUID-1', 'FASE_-1_OK', 0, '', '', 1
            )",
            [],
        )
        .unwrap();
        drop(conn);

        let mock_sheets = Arc::new(MockSheetsClient {
            get_data_calls: Mutex::new(0),
            batch_update_calls: Mutex::new(0),
            last_batch_payload: Mutex::new(None),
        });

        let sync = OutboxSynchronizer::new(
            mock_sheets.clone(),
            tmp.path().to_path_buf(),
            "SHEET_ID_TEST".to_string(),
        );

        let count = sync.sync_once().await.unwrap();

        assert_eq!(count, 0);
        assert_eq!(*mock_sheets.get_data_calls.lock().unwrap(), 0); // No read if 0 unsynced
        assert_eq!(*mock_sheets.batch_update_calls.lock().unwrap(), 0); // No batch update
    }
}
