use std::sync::{Arc, Mutex};
use url::Url;
use rusqlite::Connection;
use genesis_mc_lib::harvester::orchestrator::HarvesterOrchestrator;
use tracing::{info, error};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar Logger (Janela de Vidro)
    // PT-OPS-1: Observabilidade máxima no terminal local.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("SODA Lote Piloto: Iniciando Teste de Fogo (Goose)");

    // 2. Setup Banco de Dados Real (soda_heuristic_vault.db) na raiz do projeto
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let root_dir = std::path::Path::new(manifest_dir).parent().expect("Falha ao resolver raiz do projeto");
    let soda_data_dir = root_dir.join(".soda_data");
    
    tokio::fs::create_dir_all(&soda_data_dir).await?;
    let db_path = soda_data_dir.join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path)?;
    
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

            // --- INÍCIO DO E2E ---
            use genesis_mc_lib::cognition::swarm_dispatcher::CognitiveSwarmDispatcher;
            use genesis_mc_lib::finops::iron_cost::ModelTier;
            use genesis_mc_lib::cognition::sgr_synthesizer::SgrSynthesizer;
            use genesis_mc_lib::persist::ssot_injector::SsotInjector;

            // 7. Cognitive Swarm (Fase 2)
            info!("Engatilhando Fase 2 (Enxame Cognitivo)...");
            // Usaremos 10k tokens e a Nuvem como teste (vai passar o budget se configurado certo, simulamos LocalGPU)
            let debate = CognitiveSwarmDispatcher::dispatch_swarm(repo_id, 10000, ModelTier::LocalGPU).await.unwrap();
            info!("Debate gerado em paralelo via tokio::join! (Free-MAD).");

            // 8. SGR Synthesizer (Fase 3)
            info!("Engatilhando Fase 3 (SGR Synthesizer)...");
            let payload = SgrSynthesizer::synthesize_debate(debate).unwrap();
            info!("Decodificação Restrita aplicada (SGR Law). Score final: {}", payload.score_final);

            // 9. SSOT Injector (Fase 4)
            info!("Engatilhando Fase 4 (SSOT Injector)...");
            SsotInjector::inject_ssot(repo_id, payload).await.unwrap();
            info!("Dados selados no SQLite (Durabilidade L2).");
            info!("Payload fatiado e injetado na nuvem via batch_update_cells (Manobra Anti-503).");
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

    info!("Lote Piloto concluído com êxito E2E.");
    Ok(())
}
