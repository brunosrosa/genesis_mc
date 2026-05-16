use std::io;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;
use rusqlite::Connection;
use genesis_mc_lib::harvester::orchestrator::HarvesterOrchestrator;
use genesis_mc_lib::cognition::sgr_synthesizer::{SgrPayload, SwarmDebate};
use tracing::{info, error};

#[derive(Debug, Clone)]
struct RepoMetadata {
    description: String,
    default_branch: String,
    license_spdx: String,
}

#[derive(serde::Deserialize)]
struct GithubRepoApiResponse {
    description: Option<String>,
    default_branch: Option<String>,
    license: Option<GithubLicense>,
}

#[derive(serde::Deserialize)]
struct GithubLicense {
    spdx_id: Option<String>,
}

async fn fetch_repo_metadata(repo_id: &str) -> Result<RepoMetadata, io::Error> {
    let url = format!("https://api.github.com/repos/{}", repo_id);
    let response = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "SODA-Harvester/1.0")
        .send()
        .await
        .map_err(|e| io::Error::other(format!("Falha ao consultar GitHub API: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        return Err(io::Error::other(format!(
            "GitHub API retornou status {} ao consultar metadados do repositório",
            status
        )));
    }

    let body: GithubRepoApiResponse = response
        .json()
        .await
        .map_err(|e| io::Error::other(format!("Falha ao decodificar resposta do GitHub: {}", e)))?;

    let description = body
        .description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::other("GitHub API retornou description vazio"))?;

    let default_branch = body
        .default_branch
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::other("GitHub API retornou default_branch vazio"))?;

    let license_spdx = body
        .license
        .and_then(|license| license.spdx_id)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "NOASSERTION")
        .ok_or_else(|| io::Error::other("GitHub API retornou licenca SPDX vazia ou invalida"))?;

    Ok(RepoMetadata {
        description,
        default_branch,
        license_spdx,
    })
}

fn now_epoch_secs() -> Result<i64, io::Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("Falha ao calcular timestamp atual: {}", e)))?
        .as_secs() as i64)
}

fn load_lote_id(conn: &Connection, repo_id: &str) -> Result<String, io::Error> {
    conn.query_row(
        "SELECT lote_id FROM repositorios WHERE project_name = ?1",
        [&repo_id],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| io::Error::other(format!("Falha ao consultar lote_id: {}", e)))
}

fn load_readme_excerpt(conn: &Connection, repo_id: &str) -> Result<String, io::Error> {
    let bytes = conn
        .query_row(
            "SELECT payload_blob FROM artefatos_brutos WHERE repo_id = ?1 AND artifact_type = 'blob_01_promessa_readme' ORDER BY timestamp_extracao DESC LIMIT 1",
            [&repo_id],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(|e| io::Error::other(format!("Falha ao consultar README bruto: {}", e)))?;

    let text = String::from_utf8(bytes)
        .map_err(|e| io::Error::other(format!("Falha ao decodificar README bruto: {}", e)))?;
    let excerpt = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");

    if excerpt.is_empty() {
        Err(io::Error::other("README bruto vazio apos normalizacao"))
    } else {
        Ok(excerpt)
    }
}

fn derive_stack_base(conn: &Connection, repo_id: &str) -> Result<String, io::Error> {
    let mut stmt = conn
        .prepare("SELECT artifact_type FROM artefatos_brutos WHERE repo_id = ?1")
        .map_err(|e| io::Error::other(format!("Falha ao preparar consulta de stack_base: {}", e)))?;

    let artifact_types = stmt
        .query_map([repo_id], |row| row.get::<_, String>(0))
        .map_err(|e| io::Error::other(format!("Falha ao consultar artefatos para stack_base: {}", e)))?;

    let mut has_rust = false;
    let mut has_node = false;
    let mut has_go = false;
    let mut has_ops = false;

    for artifact_type in artifact_types {
        let artifact_type = artifact_type
            .map_err(|e| io::Error::other(format!("Falha ao iterar artefatos para stack_base: {}", e)))?;
        match artifact_type.as_str() {
            "Manifest:Cargo.toml" => has_rust = true,
            "Manifest:package.json" => has_node = true,
            "Manifest:go.mod" => has_go = true,
            artifact if artifact.starts_with("OpsBlueprint:") || artifact == "blob_07_ops_blueprint" => has_ops = true,
            _ => {}
        }
    }

    let mut parts = Vec::new();
    if has_rust {
        parts.push("Rust");
    }
    if has_node {
        parts.push("Node.js");
    }
    if has_go {
        parts.push("Go");
    }
    if has_ops {
        parts.push("Docker/CI");
    }

    if parts.is_empty() {
        Err(io::Error::other("Nao foi possivel derivar stack_base a partir dos artefatos da Fase 1"))
    } else {
        Ok(parts.join(" + "))
    }
}

async fn enrich_sgr_payload(
    conn_arc: &Arc<Mutex<Connection>>,
    repo_id: &str,
    repo_url: &str,
    debate: &SwarmDebate,
    mut payload: SgrPayload,
) -> Result<SgrPayload, io::Error> {
    let metadata = fetch_repo_metadata(repo_id).await?;
    let analyzed_at = now_epoch_secs()?;

    let (lote_id, readme_excerpt, stack_base) = {
        let conn_lock = conn_arc.lock().map_err(|e| {
            io::Error::other(format!("Falha ao adquirir lock do banco para enriquecer payload SGR: {}", e))
        })?;
        (
            load_lote_id(&conn_lock, repo_id)?,
            load_readme_excerpt(&conn_lock, repo_id)?,
            derive_stack_base(&conn_lock, repo_id)?,
        )
    };

    payload.project_name = repo_id.to_string();
    payload.repo_url = repo_url.to_string();
    payload.repo_version = metadata.default_branch.clone();
    payload.ultima_versao_online = Some(metadata.default_branch.clone());
    payload.lote_id = lote_id;
    payload.data_ultima_analise = analyzed_at;
    payload.analise_origem = "SGR_HARVESTER".to_string();
    payload.declared_description = metadata.description;
    payload.proposta_original_resumo = if payload.visao_do_enxame.trim().is_empty() {
        readme_excerpt.clone()
    } else {
        payload.visao_do_enxame.trim().to_string()
    };
    payload.stack_base = stack_base;
    payload.licenca = Some(metadata.license_spdx);
    payload.lente_a_sentido_prod_ux = Some(debate.lente_a.clone());
    payload.lente_b_estrutura_arq = Some(debate.lente_b.clone());
    payload.lente_c_realidade_ops = Some(debate.lente_c.clone());

    if payload.justificativa_decisao.trim().is_empty() {
        payload.justificativa_decisao = readme_excerpt;
    }

    Ok(payload)
}


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
            let payload = match SgrSynthesizer::synthesize_debate(debate.clone()).await {
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
            let payload = enrich_sgr_payload(&conn_arc, &repo_id, &repo_url_str, &debate, payload).await?;
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
