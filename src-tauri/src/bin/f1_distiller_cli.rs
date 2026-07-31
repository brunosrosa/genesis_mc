use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use souls_mc_lib::finops::finops_router::{FinOpsRouter, RoutingDestination, RoutingZone as FinopsZone};
use souls_mc_lib::finops::phase1_5::cloud_cascade::CloudCascade;
use souls_mc_lib::finops::phase1_5::local_distiller::{LocalDistiller, TruncatingInferenceEngine};
use souls_mc_lib::finops::phase1_5::package_assembler::{DbReader as PackageDbReader, PackageAssembler};
use souls_mc_lib::telemetry::{append_plaintext_report, enable_virtual_terminal, init_cli_tracing, now_brt_rfc3339, parse_log_level_from_env};
use rusqlite::{params, Connection};
use tempfile::NamedTempFile;
use tracing::{error, info};

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn now_epoch_secs() -> io::Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("Falha ao calcular timestamp atual: {}", e)))?
        .as_secs() as i64)
}

fn sanitize_repo_id(repo_id: &str) -> String {
    repo_id
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn etl_report_path(root_dir: &Path, repo_id: &str) -> io::Result<PathBuf> {
    let reports_dir = root_dir.join(".souls_scratchpad").join("reports");
    std::fs::create_dir_all(&reports_dir)
        .map_err(|e| io::Error::other(format!("Falha ao criar reports_dir: {}", e)))?;

    let trimmed = repo_id.trim();
    let mut parts = trimmed.split('/').map(|s| s.trim()).filter(|s| !s.is_empty());
    let owner = parts.next().unwrap_or(trimmed);
    let repo = parts.next().unwrap_or(trimmed);
    Ok(reports_dir.join(format!(
        "_ETL_REPORT_{}_{}.txt",
        sanitize_repo_id(owner),
        sanitize_repo_id(repo)
    )))
}

fn parse_repo_id_from_args() -> String {
    let mut args = std::env::args();
    args.next();
    let mut repo_id = String::from("aaif-goose/goose");
    while let Some(arg) = args.next() {
        if arg == "--repo" {
            if let Some(value) = args.next() {
                repo_id = value;
            }
        }
    }
    repo_id
}

fn ensure_phase1_5_schema(conn: &Connection) -> io::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS artefatos_destilados (
            distilled_id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL,
            essence_name TEXT NOT NULL,
            routing_zone TEXT NOT NULL,
            token_count INTEGER NOT NULL,
            destination TEXT NOT NULL,
            payload_essence TEXT NOT NULL,
            timestamp_destilacao INTEGER NOT NULL,
            UNIQUE(repo_id, essence_name)
        )",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar tabela artefatos_destilados: {}", e)))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pacotes_destilados (
            package_id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL,
            package_name TEXT NOT NULL,
            payload_package TEXT NOT NULL,
            timestamp_empacotamento INTEGER NOT NULL,
            UNIQUE(repo_id, package_name)
        )",
        [],
    )
    .map_err(|e| io::Error::other(format!("Falha ao criar tabela pacotes_destilados: {}", e)))?;

    Ok(())
}

#[derive(Debug, Clone)]
struct RawBlob {
    artifact_type: String,
    payload: String,
}

fn fetch_required_blobs(conn: &Connection, repo_id: &str) -> io::Result<Vec<RawBlob>> {
    let required = [
        "blob_01_promessa_readme",
        "blob_02_dependency_manifest",
        "blob_03_test_intent",
        "blob_04_repo_outline",
        "blob_05_architecture_map",
        "blob_06_unsafe_hotspots",
        "blob_07_ops_blueprint",
        "blob_08_health_report",
        "blob_09_community_meta",
        "blob_10_soda_canon_context",
        "blob_11_ux_contracts",
    ];

    let mut out = Vec::with_capacity(required.len());
    for artifact_type in required {
        let payload: String = conn
            .query_row(
                "SELECT CAST(payload_blob AS TEXT)
                 FROM artefatos_brutos
                 WHERE repo_id = ?1 AND artifact_type = ?2
                 LIMIT 1",
                params![repo_id, artifact_type],
                |row| row.get(0),
            )
            .map_err(|e| {
                io::Error::other(format!(
                    "Blob ausente no vault: repo_id={} artifact_type={} err={}",
                    repo_id, artifact_type, e
                ))
            })?;
        out.push(RawBlob {
            artifact_type: artifact_type.to_string(),
            payload,
        });
    }

    Ok(out)
}

fn zone_to_string(zone: FinopsZone) -> &'static str {
    match zone {
        FinopsZone::Green => "Green",
        FinopsZone::Yellow => "Yellow",
        FinopsZone::Red => "Red",
    }
}

fn dest_to_string(dest: &RoutingDestination) -> String {
    match dest {
        RoutingDestination::PassThrough => "PassThrough".to_string(),
        RoutingDestination::LocalModel { path } => format!("LocalModel:{}", path),
        RoutingDestination::CloudCascade => "CloudCascade".to_string(),
    }
}

fn convert_to_essence_name(artifact_type: &str) -> String {
    if let Some(rest) = artifact_type.strip_prefix("blob_") {
        format!("_essence_{}", rest)
    } else {
        format!("_essence_{}", artifact_type)
    }
}

fn persist_essence(
    conn: &Connection,
    repo_id: &str,
    essence_name: &str,
    routing_zone: &str,
    token_count: usize,
    destination: &str,
    payload_essence: &str,
) -> io::Result<()> {
    conn.execute(
        "INSERT INTO artefatos_destilados
            (repo_id, essence_name, routing_zone, token_count, destination, payload_essence, timestamp_destilacao)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(repo_id, essence_name) DO UPDATE SET
            routing_zone = excluded.routing_zone,
            token_count = excluded.token_count,
            destination = excluded.destination,
            payload_essence = excluded.payload_essence,
            timestamp_destilacao = excluded.timestamp_destilacao",
        params![
            repo_id,
            essence_name,
            routing_zone,
            token_count as i64,
            destination,
            payload_essence,
            now_epoch_secs()?,
        ],
    )
    .map_err(|e| io::Error::other(format!("Falha ao persistir essencia {}: {}", essence_name, e)))?;
    Ok(())
}

fn persist_package(conn: &Connection, repo_id: &str, name: &str, payload: &str) -> io::Result<()> {
    conn.execute(
        "INSERT INTO pacotes_destilados
            (repo_id, package_name, payload_package, timestamp_empacotamento)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(repo_id, package_name) DO UPDATE SET
            payload_package = excluded.payload_package,
            timestamp_empacotamento = excluded.timestamp_empacotamento",
        params![repo_id, name, payload, now_epoch_secs()?],
    )
    .map_err(|e| io::Error::other(format!("Falha ao persistir pacote {}: {}", name, e)))?;
    Ok(())
}

struct VaultDb {
    conn: Mutex<Connection>,
}

impl VaultDb {
    fn new(conn: Connection) -> Self {
        VaultDb { conn: Mutex::new(conn) }
    }
}

impl PackageDbReader for VaultDb {
    fn fetch_essence(&self, repo_id: &str, essence_name: &str) -> Result<String, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("VaultDb lock poisoned: {}", e))?;
        conn
            .query_row(
                "SELECT payload_essence
                 FROM artefatos_destilados
                 WHERE repo_id = ?1 AND essence_name = ?2
                 LIMIT 1",
                params![repo_id, essence_name],
                |row| row.get(0),
            )
            .map_err(|e| format!("fetch_essence failed: {} {}: {}", repo_id, essence_name, e))
    }

    fn fetch_raw_blob(&self, repo_id: &str, artifact_type: &str) -> Result<String, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("VaultDb lock poisoned: {}", e))?;
        conn
            .query_row(
                "SELECT CAST(payload_blob AS TEXT)
                 FROM artefatos_brutos
                 WHERE repo_id = ?1 AND artifact_type = ?2
                 LIMIT 1",
                params![repo_id, artifact_type],
                |row| row.get(0),
            )
            .map_err(|e| format!("fetch_raw_blob failed: {} {}: {}", repo_id, artifact_type, e))
    }
}

fn count_persisted_essences(conn: &Connection, repo_id: &str) -> io::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM artefatos_destilados WHERE repo_id = ?1",
        params![repo_id],
        |row| row.get(0),
    )
    .map_err(|e| io::Error::other(format!("Falha ao contar essencias persistidas: {}", e)))
}

fn count_persisted_packages(conn: &Connection, repo_id: &str) -> io::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM pacotes_destilados WHERE repo_id = ?1",
        params![repo_id],
        |row| row.get(0),
    )
    .map_err(|e| io::Error::other(format!("Falha ao contar pacotes persistidos: {}", e)))
}

fn write_f1_report(root_dir: &Path, conn: &Connection, repo_id: &str) -> io::Result<PathBuf> {
    let report_path = etl_report_path(root_dir, repo_id)?;

    let mut report = String::new();
    report.push_str(&format!("\n\n=== FASE 1.5: DESTILADOR @ {} ===\n\n", now_brt_rfc3339()));
    report.push_str(&format!("repo_id={}\n", repo_id));
    report.push_str("== ESSENCES ==\n");
    report.push_str("essence_name\trouting_zone\ttoken_count\tdestination\tpayload_bytes\n");

    {
        let mut stmt = conn
            .prepare(
                "SELECT essence_name, routing_zone, token_count, destination, LENGTH(payload_essence)
                 FROM artefatos_destilados
                 WHERE repo_id = ?1
                 ORDER BY essence_name ASC",
            )
            .map_err(|e| io::Error::other(format!("Falha ao preparar query de essences: {}", e)))?;

        let iter = stmt
            .query_map(params![repo_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|e| io::Error::other(format!("Falha ao executar query de essences: {}", e)))?;

        let mut any = false;
        for row in iter {
            let (name, zone, token_count, dest, bytes) = row
                .map_err(|e| io::Error::other(format!("Falha ao ler linha de essences: {}", e)))?;
            any = true;
            report.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                name, zone, token_count, dest, bytes
            ));
        }

        if !any {
            return Err(io::Error::other(
                "Nenhuma essência encontrada em artefatos_destilados para o repo_id",
            ));
        }
    }

    report.push_str("== PACKAGES ==\n");
    report.push_str("package_name\tpayload_bytes\n");
    {
        let mut stmt = conn
            .prepare(
                "SELECT package_name, LENGTH(payload_package)
                 FROM pacotes_destilados
                 WHERE repo_id = ?1
                 ORDER BY package_name ASC",
            )
            .map_err(|e| io::Error::other(format!("Falha ao preparar query de pacotes: {}", e)))?;

        let iter = stmt
            .query_map(params![repo_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(|e| io::Error::other(format!("Falha ao executar query de pacotes: {}", e)))?;

        let mut any = false;
        for row in iter {
            let (name, bytes) =
                row.map_err(|e| io::Error::other(format!("Falha ao ler linha de pacotes: {}", e)))?;
            any = true;
            report.push_str(&format!("{}\t{}\n", name, bytes));
        }

        if !any {
            return Err(io::Error::other(
                "Nenhum pacote encontrado em pacotes_destilados para o repo_id",
            ));
        }
    }

    append_plaintext_report(&report_path, &report)
        .map_err(|e| io::Error::other(format!("Falha ao anexar relatório ETL {}: {}", report_path.display(), e)))?;

    Ok(report_path)
}

fn get_system_prompt_for_artifact(artifact_type: &str) -> &'static str {
    match artifact_type {
        "blob_01_repo_identity" => {
            "Você é o Destilador de Promessas. Extraia a visão original do produto e o público-alvo. DESTRUA impiedosamente qualquer jargão de marketing, adjetivos comerciais ou hype. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_02_product_specs" => {
            "Você é o Destilador de Dependências. Mapeie a stack tecnológica e liste os pacotes. Destaque evidências de 'Lixo Tóxico' (dependência excessiva de Node.js, Electron, VMs pesadas). PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_03_test_intent" => {
            "Você é o Destilador de Intenções. Extraia estritamente as regras de negócio, fluxos e validações provadas nas assinaturas de testes unitários e E2E. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_04_repo_outline" => {
            "Você é o Destilador de Estrutura AST. Preserve RIGOROSAMENTE todas as assinaturas matemáticas, rotas, tipos e interfaces. IGNORE as lógicas internas de implementação (o miolo das funções). PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_05_architecture_map" => {
            "Você é o Destilador Topológico. Mapeie o grafo de diretórios e a hierarquia de importações, evidenciando o acoplamento da arquitetura. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_06_unsafe_hotspots" => {
            "Você é o Destilador de Risco. Mapeie cicatrizes no código, uso de 'unsafe', chaves hardcoded e alertas de segurança estrutural. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_07_ops_blueprint" => {
            "Você é o Destilador de Operações. Mapeie o pipeline de CI/CD, Dockerfiles e atritos de infraestrutura. Foque na complexidade de deploy. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_08_health_report" => {
            "Você é o Destilador de Dívida Técnica. Realize a DEDUPLICAÇÃO ESTRITA de erros de linters, mapeando gargalos de complexidade ciclomática. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_09_community_meta" => {
            "Você é o Destilador de Comunidade. Condense apenas os fatos vitais: data de atualização, issues abertas e tração do repositório. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        "blob_11_ux_contracts" => {
            "Você é o Destilador de Contratos UX. Preserve estritamente as Props (entradas) e Eventos (saídas) dos componentes visuais. IGNORE marcações CSS, Tailwind e HTML decorativo. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
        _ => {
            "Você é o Destilador Factual da Fase 1.5 do SODA. Sua missão é extrair a ESSÊNCIA FACTUAL COMPACTA do artefato fornecido. PROIBIDO inventar fatos ou soluções. DIRETRIZ ABSOLUTA: Entregue EXCLUSIVAMENTE o Markdown denso, neutro e técnico. Zero introduções, zero saudações, zero 'Aqui está o resumo'."
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let _ = enable_ansi_support::enable_ansi_support();
    enable_virtual_terminal();
    let level = parse_log_level_from_env();
    init_cli_tracing(level);

    let started = Instant::now();
    let repo_id = parse_repo_id_from_args();

    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();
    let db_path = root_dir.join(".souls_data").join("souls_heuristic_vault.db");
    let conn = Connection::open(&db_path).map_err(|e| {
        io::Error::other(format!("Falha ao abrir vault em {}: {}", db_path.display(), e))
    })?;
    ensure_phase1_5_schema(&conn)?;

    let blobs = fetch_required_blobs(&conn, &repo_id)?;
    info!(repo_id = %repo_id, blobs = blobs.len(), "F1 (Destilador FinOps): blobs carregados do vault");

    let cascade = CloudCascade::new().map_err(|e| io::Error::other(format!("{:?}", e)))?;

    let mut yellow_total = 0_u32;
    let mut yellow_cloud = 0_u32;
    let mut red_total = 0_u32;
    let mut green_total = 0_u32;

    for blob in blobs {
        let mut tmp = NamedTempFile::new()?;
        std::io::Write::write_all(&mut tmp, blob.payload.as_bytes())?;
        let tmp_path = tmp.path().to_path_buf();

        let decision = FinOpsRouter::classify_blob(&tmp_path)
            .map_err(|e| io::Error::other(format!("Falha ao classificar blob {}: {}", blob.artifact_type, e)))?;

        match decision.zone {
            FinopsZone::Green => green_total += 1,
            FinopsZone::Yellow => yellow_total += 1,
            FinopsZone::Red => red_total += 1,
        }
        if decision.zone == FinopsZone::Yellow && decision.destination == RoutingDestination::CloudCascade {
            yellow_cloud += 1;
        }

        if blob.artifact_type == "blob_10_soda_canon_context" {
            info!(repo_id = %repo_id, "blob_10 mantido no Prompt Caching / Canon sem destilacao iterativa");
            continue;
        }

        let prompt = get_system_prompt_for_artifact(&blob.artifact_type);
        let essence_payload = match &decision.destination {
            RoutingDestination::PassThrough => blob.payload.clone(),
            RoutingDestination::LocalModel { path } => {
                let _distiller: LocalDistiller<TruncatingInferenceEngine> =
                    LocalDistiller::new(path).map_err(|e| io::Error::other(format!("{:?}", e)))?;
                cascade
                    .cascade_distill(&blob.payload, prompt)
                    .await
                    .map_err(|e| io::Error::other(format!("{:?}", e)))?
            }
            RoutingDestination::CloudCascade => cascade
                .cascade_distill(&blob.payload, prompt)
                .await
                .map_err(|e| io::Error::other(format!("{:?}", e)))?,
        };

        let essence_name = convert_to_essence_name(&blob.artifact_type);
        persist_essence(
            &conn,
            &repo_id,
            &essence_name,
            zone_to_string(decision.zone),
            decision.token_count,
            &dest_to_string(&decision.destination),
            &essence_payload,
        )?;

        info!(
            repo_id = %repo_id,
            artifact_type = %blob.artifact_type,
            essence_name = %essence_name,
            zone = %zone_to_string(decision.zone),
            token_count = decision.token_count,
            "Essencia persistida"
        );
    }

    let vault_for_assemble = VaultDb::new(Connection::open(&db_path)?);
    let assembler = PackageAssembler::new(&vault_for_assemble);
    let packages = assembler
        .assemble(&repo_id)
        .map_err(|e| io::Error::other(format!("{:?}", e)))?;

    persist_package(&conn, &repo_id, "A", &packages.package_a)?;
    persist_package(&conn, &repo_id, "B", &packages.package_b)?;
    persist_package(&conn, &repo_id, "C", &packages.package_c)?;

    let essences_count = count_persisted_essences(&conn, &repo_id)?;
    let packages_count = count_persisted_packages(&conn, &repo_id)?;
    let report_path = write_f1_report(&root_dir, &conn, &repo_id)?;

    info!(
        repo_id = %repo_id,
        elapsed_ms = started.elapsed().as_millis(),
        green_total = green_total,
        yellow_total = yellow_total,
        yellow_cloud = yellow_cloud,
        red_total = red_total,
        essences_count = essences_count,
        packages_count = packages_count,
        report = %report_path.display(),
        "F1 (Destilador FinOps) concluída"
    );

    if packages_count < 3 {
        error!(
            repo_id = %repo_id,
            packages_count = packages_count,
            "Persistencia incompleta dos pacotes"
        );
        return Err("Persistencia incompleta dos pacotes".into());
    }

    Ok(())
}
