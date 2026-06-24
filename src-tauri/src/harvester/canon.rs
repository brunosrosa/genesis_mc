use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use tracing::info;

use super::persist::ArtifactBlob;

const BLOB_10_TYPE: &str = "blob_10_soda_canon_context";
pub const CANON_GLOBAL_REPO_ID: &str = "__SODA_CANON_GLOBAL__";
const CANON_CACHE_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;
const CANON_SCHEMA_TAG: &str = "SODA_CANON_V5_ADRS_ALL";
const CANON_MANIFEST_RELATIVE_PATH: &str = "docs/SODA_CANON_MANIFEST.md";
const CANON_LOCAL_CONTEXT: &str = "SODA_CANON_V5_ADRS_ALL
Raio-X Macro do SODA / Genesis MC:

O nucleo do sistema e soberania bare-metal. A regra estrutural e backend em Rust com Tokio, ownership explicito, fail-closed e zero panic em producao. O frontend existe como casca passiva em Svelte 5, renderizando estado sem tomar para si logica de negocio, orquestracao, memoria ou inferencia. Python, Node.js e sidecars externos nao definem o produto; quando aparecem, existem apenas como ferramentas efemeras de fabrica, confinadas e descartadas ao fim da tarefa.

A RTX 2060m com 6 GB de VRAM nao representa o destino final do produto. Ela e o treino de gravidade, o piso minimo de validacao local para provar que a arquitetura elastica sobrevive sob restricao severa. O desenho correto precisa escalar sem mutacao filosofica: hoje valida em hardware modesto, amanha sobe para classes superiores mantendo o mesmo eixo Rust nativo, workers isolados e aceleracao progressiva em Burn e CubeCL quando a computacao vetorial entrar em cena.

O contrato entre backend e interface rejeita serializacao volumosa e lixo transiente. O norte e IPC zero-copy ou zero-garbage, com buffers binarios, ownership claro e transporte previsivel, para que a UI nao seja sufocada por JSON massivo, GC desnecessario ou copias redundantes de memoria. A disciplina de throughput vale tanto para inferencia quanto para telemetria.

Toda decisao de execucao e governada por FinOps local-first. O ParetoBandit escolhe a trilha de menor custo, menor latencia e risco controlado antes de escalar para qualquer recurso premium. A nuvem nao e fundamento ontologico: e apenas opcao subordinada. Se o ambiente nao honra as garantias mecanicas, o sistema falha fechado em vez de improvisar com dependencias caras ou opacas.

A memoria cognitiva e uma triade local e soberana. SQLite ancora o estado transacional, a trilha auditavel e os fatos episodicos; LanceDB serve a recuperacao semantica vetorial; LadybugDB sustenta relacoes estruturais e causais. A triade existe para impedir memoria orfa, grounding fraco e dependencia de bancos externos que dissolvem o contexto critico do usuario.

A experiencia do operador precisa ser neuro-inclusiva e anti-Flow-Debt. A interface privilegia navegacao espacial em Tiling 2D, telemetria ambiental e estabilidade de foco. O sistema rejeita caos de janelas, spinners ansiosos, reflow agressivo e qualquer ritual visual que sacrifique orientacao espacial em troca de ornamento. UX aqui e mecanismo cognitivo, nao decoracao.

A avaliacao de qualquer ecossistema externo obedece a doutrina da Canibalizacao Cirurgica e do Pessimismo da Razao. Nenhuma arquitetura alienigena e absorvida integralmente se carregar lixo toxico (dependencias massivas, Node.js, Electron). O objetivo do SODA e amputar e extrair puramente a alma matematica, a heuristica invisivel e o padrao de UX, transmutando-os para o nosso motor em Rust/Svelte 5 ou confinando-os em sidecars efemeros. A estocasticidade da IA nunca deve ultrapassar o Cercadinho do Determinismo: qualquer alteracao estrutural ou exclusao deve ser retida na Agent Inbox para aprovacao humana (Human-in-the-Loop), garantindo protecao contra a Corrupcao Silenciosa de Dados.";

#[derive(Error, Debug)]
pub enum CanonError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Canonical context query returned empty content")]
    EmptyPayload,
}

#[derive(Debug, Clone)]
struct CanonCacheEntry {
    repo_id: String,
    payload_blob: Vec<u8>,
    timestamp_extracao: i64,
}

pub struct SodaCanonExtractor;

impl SodaCanonExtractor {
    pub async fn extract(
        repo_id: &str,
        conn: Arc<Mutex<Connection>>,
    ) -> Result<ArtifactBlob, CanonError> {
        if let Some(entry) = Self::load_cache(repo_id, Arc::clone(&conn)).await? {
            if Self::is_fresh(entry.timestamp_extracao)? && payload_matches_schema(&entry.payload_blob) {
                if entry.repo_id == CANON_GLOBAL_REPO_ID {
                    Self::persist_blob(repo_id, entry.payload_blob.clone(), Arc::clone(&conn)).await?;
                }
                info!(repo_id = %repo_id, "blob_10_soda_canon_context servido do cache SQLite");
                return Ok(ArtifactBlob {
                    artifact_type: BLOB_10_TYPE.to_string(),
                    payload_blob: entry.payload_blob,
                });
            }
        }

        let payload_text = tokio::task::spawn_blocking(render_canon_context)
            .await
            .map_err(|e| CanonError::Storage(format!("Falha ao aguardar renderizacao do canon: {}", e)))??;
        if payload_text.trim().is_empty() {
            return Err(CanonError::EmptyPayload);
        }

        let payload_blob = payload_text.into_bytes();
        Self::persist_blob(repo_id, payload_blob.clone(), Arc::clone(&conn)).await?;
        Self::persist_blob(CANON_GLOBAL_REPO_ID, payload_blob.clone(), conn).await?;

        Ok(ArtifactBlob {
            artifact_type: BLOB_10_TYPE.to_string(),
            payload_blob,
        })
    }

    async fn load_cache(
        repo_id: &str,
        conn: Arc<Mutex<Connection>>,
    ) -> Result<Option<CanonCacheEntry>, CanonError> {
        let repo_id = repo_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| CanonError::Storage(format!("Falha ao adquirir lock do SQLite: {}", e)))?;

            let mut stmt = conn
                .prepare(
                    "SELECT repo_id, payload_blob, timestamp_extracao
                     FROM artefatos_brutos
                     WHERE artifact_type = ?1
                       AND repo_id IN (?2, ?3)
                     ORDER BY CASE WHEN repo_id = ?2 THEN 0 ELSE 1 END, timestamp_extracao DESC
                     LIMIT 1",
                )
                .map_err(|e| CanonError::Storage(format!("Falha ao preparar query do cache canonico: {}", e)))?;

            stmt.query_row(params![BLOB_10_TYPE, repo_id, CANON_GLOBAL_REPO_ID], |row| {
                Ok(CanonCacheEntry {
                    repo_id: row.get(0)?,
                    payload_blob: row.get(1)?,
                    timestamp_extracao: row.get(2)?,
                })
            })
            .optional()
            .map_err(|e| CanonError::Storage(format!("Falha ao consultar cache canonico: {}", e)))
        })
        .await
        .map_err(|e| CanonError::Storage(format!("Falha ao aguardar leitura do cache canonico: {}", e)))?
    }

    async fn persist_blob(
        repo_id: &str,
        payload_blob: Vec<u8>,
        conn: Arc<Mutex<Connection>>,
    ) -> Result<(), CanonError> {
        let repo_id = repo_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| CanonError::Storage(format!("Falha ao adquirir lock do SQLite: {}", e)))?;
            let now = now_epoch_secs()?;
            conn.execute(
                "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(repo_id, artifact_type) DO UPDATE SET
                    payload_blob = excluded.payload_blob,
                    timestamp_extracao = excluded.timestamp_extracao",
                params![repo_id, BLOB_10_TYPE, payload_blob, now],
            )
            .map_err(|e| CanonError::Storage(format!("Falha ao persistir blob_10 no SQLite: {}", e)))?;
            Ok(())
        })
        .await
        .map_err(|e| CanonError::Storage(format!("Falha ao aguardar persistencia do blob_10: {}", e)))?
    }

    fn is_fresh(timestamp_extracao: i64) -> Result<bool, CanonError> {
        Ok(now_epoch_secs()? - timestamp_extracao <= CANON_CACHE_MAX_AGE_SECS)
    }
}

fn now_epoch_secs() -> Result<i64, CanonError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CanonError::Storage(format!("Falha ao calcular timestamp atual: {}", e)))?
        .as_secs() as i64)
}

fn render_canon_context() -> Result<String, CanonError> {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| CanonError::Storage("Falha ao resolver raiz do projeto".to_string()))?;

    let manifest_path = root_dir.join(CANON_MANIFEST_RELATIVE_PATH);
    let source = match fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(_) => CANON_LOCAL_CONTEXT.to_string(),
    };

    if source.contains(CANON_SCHEMA_TAG) {
        return Ok(source.trim().to_string());
    }
    Ok(format!("{CANON_SCHEMA_TAG}\n\n{}", source.trim()))
}

fn payload_matches_schema(payload: &[u8]) -> bool {
    std::str::from_utf8(payload)
        .map(|text| text.contains(CANON_SCHEMA_TAG))
        .unwrap_or(false)
}
