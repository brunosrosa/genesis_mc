use std::sync::{Arc, Mutex};
use url::Url;
use rusqlite::Connection;
use genesis_mc_lib::harvester::orchestrator::HarvesterOrchestrator;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar Logger (Janela de Vidro)
    // PT-OPS-1: Observabilidade máxima no terminal local.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("SODA Lote Piloto: Iniciando Teste de Fogo (Goose)");

    // 2. Setup Banco de Dados Real (soda_heuristic_vault.db)
    let db_path = "soda_heuristic_vault.db";
    let conn = Connection::open(db_path)?;
    
    // 3. Criar Tabelas (Dicionário SODA V3)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS repositorios (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            status TEXT NOT NULL,
            last_processed TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS artefatos_brutos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            payload_blob BLOB NOT NULL,
            FOREIGN KEY(repo_id) REFERENCES repositorios(id)
        )",
        [],
    )?;

    // 4. Inserir Registro Base para o Goose
    let repo_url_str = "https://github.com/aaif-goose/goose";
    // O ID deve casar com o gerado pelo Orchestrator (path sem / trocado por _)
    let repo_id = "aaif-goose_goose";
    
    conn.execute(
        "INSERT OR IGNORE INTO repositorios (id, url, status) VALUES (?1, ?2, ?3)",
        [repo_id, repo_url_str, "PENDENTE"],
    )?;

    info!(repo_id = %repo_id, "Registro base inserido/verificado. Iniciando orquestração...");

    // 5. Instanciar e Rodar Orquestrador (N14)
    let repo_url = Url::parse(repo_url_str)?;
    let conn_arc = Arc::new(Mutex::new(conn));

    match HarvesterOrchestrator::run(&repo_url, Arc::clone(&conn_arc)).await {
        Ok(_) => {
            info!("Orquestração finalizada com SUCESSO para {}", repo_id);
            
            // 6. Atualizar Status para FASE_1_OK
            let conn_lock = conn_arc.lock().unwrap();
            conn_lock.execute(
                "UPDATE repositorios SET status = ?1, last_processed = datetime('now') WHERE id = ?2",
                ["FASE_1_OK", repo_id],
            )?;
            info!("Status do repositório atualizado: FASE_1_OK");
        }
        Err(e) => {
            error!("Falha crítica na orquestração: {}", e);
            let conn_lock = conn_arc.lock().unwrap();
            conn_lock.execute(
                "UPDATE repositorios SET status = ?1 WHERE id = ?2",
                ["ERRO_FASE_1", repo_id],
            )?;
            return Err(e.into());
        }
    }

    info!("Lote Piloto concluído com êxito.");
    Ok(())
}
