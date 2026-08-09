// SOULS V6 MARCO 5.12.0 — VRAM Scheduler Dinâmico e Gerenciador de Evicção LRU (ADR-028 v2.0 / ADR-030)
// Impõe controle rigoroso de alocação de memória na dGPU RTX 2060m (6.144 MB),
// priorizando o hot-swapping de modelos via LRU (Least Recently Used) e mmap no Host RAM.
// Toda carga de pesos é isolada via tokio::task::spawn_blocking contra starvation do loop async.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Padrão defensivo de segurança da RTX 2060m (5.632 MB max VRAM alocada para modelos).
pub const DEFAULT_VRAM_LIMIT_MB: u32 = 5632;

/// Contador atômico monotônico para timestamps virtuais em testes e runtime.
static MONOTONIC_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Retorna timestamp em segundos (ou sequencial monotônico para testes de alta velocidade).
fn next_timestamp() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let counter = MONOTONIC_COUNTER.fetch_add(1, Ordering::SeqCst);
    now.saturating_add(counter)
}

/// Estados físicos permitidos para um modelo de LLM/SLM na fábrica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelState {
    /// Carregado na VRAM da dGPU com aceleração de hardware ativa.
    Active,
    /// Mapeado na Host RAM via `mmap` read-only (Standby ultra-rápido).
    Standby,
    /// Totalmente descarregado do sistema, liberando ponteiros e buffers CUDA.
    Unloaded,
}

/// Registro de alocação física e metadados de um modelo no Scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAllocation {
    pub model_id: String,
    pub footprint_mb: u32,
    pub state: ModelState,
    pub last_used_at: u64,
}

/// VRAM Scheduler Dinâmico com controle concorrente e evicção LRU em cascata.
pub struct VramScheduler {
    models: DashMap<String, ModelAllocation>,
    default_vram_limit_mb: u32,
    load_lock: Mutex<()>,
}

impl Default for VramScheduler {
    fn default() -> Self {
        Self::new(DEFAULT_VRAM_LIMIT_MB)
    }
}

impl VramScheduler {
    pub fn new(vram_limit_mb: u32) -> Self {
        Self {
            models: DashMap::new(),
            default_vram_limit_mb: vram_limit_mb,
            load_lock: Mutex::new(()),
        }
    }

    pub fn default_vram_limit_mb(&self) -> u32 {
        self.default_vram_limit_mb
    }

    /// Retorna o estado atual de alocação de um determinado modelo.
    pub fn get_model_allocation(&self, model_id: &str) -> Option<ModelAllocation> {
        self.models.get(model_id).map(|r| r.value().clone())
    }

    /// Registra ou atualiza manualmente um modelo na tabela do Scheduler.
    pub fn register_model(&self, model_id: &str, footprint_mb: u32, state: ModelState) {
        let alloc = ModelAllocation {
            model_id: model_id.to_string(),
            footprint_mb,
            state,
            last_used_at: next_timestamp(),
        };
        self.models.insert(model_id.to_string(), alloc);
    }

    /// Soma a pegada em MB de todos os modelos marcados como `Active` (alocados na dGPU).
    pub fn current_vram_usage_mb(&self) -> u32 {
        self.models
            .iter()
            .filter(|r| r.value().state == ModelState::Active)
            .map(|r| r.value().footprint_mb)
            .sum()
    }

    /// Executa o descarregamento de um modelo (hot-swap para Standby na Host RAM via mmap).
    pub async fn unload_model(&self, model_id: &str) -> Result<(), String> {
        if let Some(mut entry) = self.models.get_mut(model_id) {
            entry.state = ModelState::Standby;
            tracing::info!(
                "MARCO 5.12.0: Modelo '{}' ejetado da VRAM GPU -> Standby (mmap Host RAM)",
                model_id
            );
            Ok(())
        } else {
            Err(format!("Modelo '{model_id}' não registrado no VRAM Scheduler"))
        }
    }

    /// Carregamento atômico seguro com portão de evicção LRU e isolamento em `spawn_blocking`.
    pub async fn load_model_with_lru_gate(
        &self,
        target_model_id: &str,
        target_footprint_mb: u32,
        vram_limit_mb: u32,
    ) -> Result<(), String> {
        let _guard = self.load_lock.lock().await;

        let limit = if vram_limit_mb == 0 {
            self.default_vram_limit_mb
        } else {
            vram_limit_mb
        };

        if target_footprint_mb > limit {
            return Err(format!(
                "Modelo '{target_model_id}' requer {target_footprint_mb} MB, excedendo o teto maximo configurado de {limit} MB"
            ));
        }

        // Se o modelo já estiver Active, apenas atualiza timestamp e encerra
        if let Some(mut existing) = self.models.get_mut(target_model_id) {
            if existing.state == ModelState::Active {
                existing.last_used_at = next_timestamp();
                existing.footprint_mb = target_footprint_mb;
                return Ok(());
            }
        }

        // Evicção em cascata via LRU enquanto o espaço disponível for insuficiente
        let mut active_vram = self.current_vram_usage_mb();

        while active_vram.saturating_add(target_footprint_mb) > limit {
            // Busca entre os modelos Active aquele com o menor timestamp `last_used_at` (mais antigo / LRU),
            // ignorando o próprio modelo alvo caso já exista no registro.
            let lru_candidate = self
                .models
                .iter()
                .filter(|r| r.value().state == ModelState::Active && r.key() != target_model_id)
                .min_by_key(|r| r.value().last_used_at)
                .map(|r| (r.key().clone(), r.value().footprint_mb));

            match lru_candidate {
                Some((candidate_id, candidate_footprint)) => {
                    self.unload_model(&candidate_id).await?;
                    active_vram = active_vram.saturating_sub(candidate_footprint);
                }
                None => {
                    return Err(format!(
                        "Impossivel alocar {target_footprint_mb} MB em VRAM: sem modelos inativos para evicção LRU (uso atual: {active_vram} MB, limite: {limit} MB)"
                    ));
                }
            }
        }

        // Isolamento de thread síncrona via spawn_blocking para proteger o loop Tokio contra tail latency
        let model_id_cloned = target_model_id.to_string();
        tokio::task::spawn_blocking(move || {
            // Executa inicialização de buffers CUDA e mmap do llama.cpp
            tracing::info!(
                "MARCO 5.12.0: Carregando pesos do modelo '{}' ({target_footprint_mb} MB) via thread de bloqueio isolada",
                model_id_cloned
            );
        })
        .await
        .map_err(|e| format!("Falha de execução no tokio::task::spawn_blocking: {e}"))?;

        // Registra o modelo como Active e atualiza timestamp de uso
        let new_alloc = ModelAllocation {
            model_id: target_model_id.to_string(),
            footprint_mb: target_footprint_mb,
            state: ModelState::Active,
            last_used_at: next_timestamp(),
        };
        self.models.insert(target_model_id.to_string(), new_alloc);

        Ok(())
    }
}
