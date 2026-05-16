use std::io;
use std::sync::{Arc, Mutex};
use url::Url;
use rusqlite::Connection;
use genesis_mc_lib::harvester::orchestrator::HarvesterOrchestrator;
use tracing::{info, error};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // 1. Inicializar Logger (Janela de Vidro)
    // PT-OPS-1: Observabilidade máxima no terminal local.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("SODA Lote Piloto: Iniciando Teste de Fogo (Goose)");

    // 2. Setup Banco de Dados Real (soda_heuristic_vault.db) na raiz do projeto
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let root_dir = std::path::Path::new(manifest_dir)
        .parent()
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))?;
    let soda_data_dir = root_dir.join(".soda_data");
    
    tokio::fs::create_dir_all(&soda_data_dir).await?;
    let db_path = soda_data_dir.join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path)?;
    
    // 3. Tabelas Forjadas (Dicionário SODA V3) - Respeitando o forge_db.py
    conn.execute(
        "CREATE TABLE IF NOT EXISTS repositorios (
            project_name TEXT PRIMARY KEY,
            lote_id TEXT NOT NULL,
            repo_url TEXT NOT NULL UNIQUE,
            soda_universal_uuid TEXT NOT NULL UNIQUE,
            status_processamento TEXT NOT NULL,
            timestamp_fase_1 INTEGER,
            timestamp_fase_3 INTEGER,
            retry_count INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS artefatos_brutos (
            artifact_id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL REFERENCES repositorios(project_name),
            payload_blob TEXT NOT NULL,
            timestamp_extracao INTEGER NOT NULL,
            artifact_type TEXT NOT NULL
        )",
        [],
    )?;

    let mut args = std::env::args();
    args.next(); // skip bin
    let mut repo_id = String::from("aaif-goose/goose"); // default
    while let Some(arg) = args.next() {
        if arg == "--repo" {
            if let Some(r) = args.next() {
                repo_id = r;
            }
        }
    }
    
    let repo_url_str = format!("https://github.com/{}", repo_id);
    conn.execute(
        "INSERT OR IGNORE INTO repositorios (project_name, lote_id, repo_url, soda_universal_uuid, status_processamento, retry_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![&repo_id, "LOTE_01_ALPHA", &repo_url_str, format!("UUID-{}", repo_id), "PENDENTE", 0],
    )?;

    info!(repo_id = %repo_id, "Registro base inserido/verificado. Iniciando orquestração...");

    // 5. Instanciar e Rodar Orquestrador (N14)
    let repo_url = Url::parse(&repo_url_str)?;
    let conn_arc = Arc::new(Mutex::new(conn));

    match HarvesterOrchestrator::run(&repo_id, &repo_url, Arc::clone(&conn_arc)).await {
        Ok(_) => {
            info!("Orquestração finalizada com SUCESSO para {}", repo_id);
            
            // 6. Atualizar Status para FASE_1_OK
            {
                let conn_lock = conn_arc.lock().map_err(|e| {
                    io::Error::other(format!("Falha ao adquirir lock do banco apos Fase 1: {}", e))
                })?;
                conn_lock.execute(
                    "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                    ["FASE_1_OK", &repo_id],
                )?;
            }
            info!("Status do repositório atualizado: FASE_1_OK");

            // --- INÍCIO DO E2E ---
            use genesis_mc_lib::cognition::swarm_dispatcher::CognitiveSwarmDispatcher;
            use genesis_mc_lib::finops::iron_cost::ModelTier;
            use genesis_mc_lib::cognition::sgr_synthesizer::SgrSynthesizer;
            use genesis_mc_lib::persist::ssot_injector::SsotInjector;

            // 7. Cognitive Swarm (Fase 2)
            info!("Engatilhando Fase 2 (Enxame Cognitivo)...");
            // Usaremos 10k tokens e a Nuvem como teste (vai passar o budget se configurado certo, simulamos LocalGPU)
            let debate = match CognitiveSwarmDispatcher::dispatch_swarm(&repo_id, 10000, ModelTier::LocalGPU).await {
                Ok(debate) => debate,
                Err(e) => {
                    error!(repo_id = %repo_id, error = %e, "Falha ao executar Fase 2");
                    let conn_lock = conn_arc.lock().map_err(|lock_err| {
                        io::Error::other(format!("Falha ao adquirir lock do banco na Fase 2: {}", lock_err))
                    })?;
                    conn_lock.execute(
                        "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                        ["ERRO_FASE_2", &repo_id],
                    )?;
                    return Err(io::Error::other(format!("Falha na Fase 2: {}", e)).into());
                }
            };
            info!("Debate gerado em paralelo via tokio::join! (Free-MAD).");

            // 8. SGR Synthesizer (Fase 3)
            info!("Engatilhando Fase 3 (SGR Synthesizer)...");
            let payload = match SgrSynthesizer::synthesize_debate(debate).await {
                Ok(payload) => payload,
                Err(e) => {
                    error!(repo_id = %repo_id, error = %e, "Falha ao executar Fase 3");
                    let conn_lock = conn_arc.lock().map_err(|lock_err| {
                        io::Error::other(format!("Falha ao adquirir lock do banco na Fase 3: {}", lock_err))
                    })?;
                    conn_lock.execute(
                        "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                        ["ERRO_FASE_3", &repo_id],
                    )?;
                    return Err(io::Error::other(format!("Falha na Fase 3: {}", e)).into());
                }
            };
            info!("Decodificação Restrita aplicada (SGR Law). Score final: {}", payload.score_final);

            // 9. SSOT Injector (Fase 4)
            info!("Engatilhando Fase 4 (SSOT Injector)...");
            if let Err(e) = SsotInjector::inject_ssot(&repo_id, payload).await {
                error!(repo_id = %repo_id, error = %e, "Falha ao executar Fase 4");
                let conn_lock = conn_arc.lock().map_err(|lock_err| {
                    io::Error::other(format!("Falha ao adquirir lock do banco na Fase 4: {}", lock_err))
                })?;
                conn_lock.execute(
                    "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                    ["ERRO_FASE_4", &repo_id],
                )?;
                return Err(io::Error::other(format!("Falha na Fase 4: {}", e)).into());
            }
            info!("Dados selados no SQLite (Durabilidade L2).");
            info!("Payload fatiado e injetado na nuvem via batch_update_cells (Manobra Anti-503).");
        }
        Err(e) => {
            error!("Falha crítica na orquestração: {}", e);
            let conn_lock = conn_arc.lock().map_err(|lock_err| {
                io::Error::other(format!("Falha ao adquirir lock do banco no erro da Fase 1: {}", lock_err))
            })?;
            conn_lock.execute(
                "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                ["ERRO_FASE_1", &repo_id],
            )?;
            return Err(e.into());
        }
    }

    info!("Lote Piloto concluído com êxito E2E.");
    Ok(())
}
