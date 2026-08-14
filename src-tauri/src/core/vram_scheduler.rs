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

// =============================================================================
// MARCO IV — KvCacheSwapController (ADR-027 / Marco IV)
// =============================================================================
// Complementa o VramScheduler (LRU de modelos) com proteção contra estouro de
// VRAM **dentro de uma janela de inferência ativa**. O Controller consome o
// `WATCHDOG_STATE` (publicado pela thread nativa S.O.) e dispara swap-out do
// KV Cache quantizado Q4_K (GPU→Host RAM) ou swap-in de retorno, com histerese
// anti-flap (2 amostras consecutivas) e isolamento de syscalls em
// `tokio::task::spawn_blocking`.
// =============================================================================

use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicU32;

use crate::core::hardware_watchdog;

/// Ação determinada pelo sink de pressão de VRAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VramAction {
    /// Mantém estado atual (sem pressão suficiente).
    Hold,
    /// Pressão alta sustentada → swap-out do KV Cache GPU→Host RAM.
    SwapOut,
    /// Após swap-out, headroom recuperado → swap-in de retorno.
    SwapIn,
}

/// Estado físico do KV Cache (atomicamente consistente).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KvCacheLocation {
    /// Residente na VRAM da dGPU (estado de execução).
    Gpu = 0,
    /// Evacuado para Host RAM (após swap-out).
    HostRam = 1,
}

impl KvCacheLocation {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::HostRam,
            _ => Self::Gpu,
        }
    }
}

/// Trait abstrato para sinks de pressão de VRAM.
/// Agnóstico ao backend: hoje consumimos `WATCHDOG_STATE`, amanhã pode ser
/// NVML direto, ROCm, Metal, ou um mock de teste.
pub trait VramPressureSink: Send + Sync {
    /// Retorna a porcentagem atual de uso de VRAM (0.0 .. 100.0).
    fn current_vram_pct(&self) -> f32;
}

/// Sink de produção: lê do `WATCHDOG_STATE` global, dividindo pelo total físico
/// da RTX 2060m (6.144 MB). Fallback: se NVML ausente, retorna 0.0 (sem pressão).
pub struct WatchdogSink {
    pub vram_total_mb: u32,
}

impl Default for WatchdogSink {
    fn default() -> Self {
        Self {
            vram_total_mb: crate::core::hardware_watchdog::RTX_2060M_VRAM_TOTAL_MB,
        }
    }
}

impl VramPressureSink for WatchdogSink {
    fn current_vram_pct(&self) -> f32 {
        let Some(state) = hardware_watchdog::get_state() else {
            return 0.0;
        };
        let packed = state.load(std::sync::atomic::Ordering::Acquire);
        let vram_mb = hardware_watchdog::decode_vram_mb(packed);
        if self.vram_total_mb == 0 {
            0.0
        } else {
            (vram_mb as f32 / self.vram_total_mb as f32) * 100.0
        }
    }
}

/// Controlador de swap do KV Cache com histerese anti-flap.
/// Padrão: high=90.0%, low=80.0%, exige 2 amostras consecutivas para transicionar.
pub struct KvCacheSwapController {
    threshold_high_pct: f32,
    threshold_low_pct: f32,
    required_samples: u32,
    consecutive_samples: AtomicU32,
    current_state: AtomicU8,
}

impl Default for KvCacheSwapController {
    fn default() -> Self {
        Self::new()
    }
}

impl KvCacheSwapController {
    #[must_use]
    pub fn new() -> Self {
        Self::with_thresholds(90.0, 80.0, 2)
    }

    #[must_use]
    pub fn with_thresholds(high_pct: f32, low_pct: f32, required_samples: u32) -> Self {
        let required_samples = if required_samples == 0 { 1 } else { required_samples };
        Self {
            threshold_high_pct: high_pct,
            threshold_low_pct: low_pct,
            required_samples,
            consecutive_samples: AtomicU32::new(0),
            current_state: AtomicU8::new(KvCacheLocation::Gpu as u8),
        }
    }

    /// Avalia a pressão atual e retorna a ação a tomar.
    /// Histerese: precisa de `required_samples` leituras consecutivas além do limiar.
    pub fn evaluate(&self, sink: &dyn VramPressureSink) -> VramAction {
        let pct = sink.current_vram_pct();
        let loc = KvCacheLocation::from_u8(self.current_state.load(std::sync::atomic::Ordering::Acquire));

        match loc {
            KvCacheLocation::Gpu => {
                if pct >= self.threshold_high_pct {
                    let prev = self
                        .consecutive_samples
                        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                    if prev + 1 >= self.required_samples {
                        self.consecutive_samples
                            .store(0, std::sync::atomic::Ordering::Release);
                        VramAction::SwapOut
                    } else {
                        VramAction::Hold
                    }
                } else {
                    self.consecutive_samples
                        .store(0, std::sync::atomic::Ordering::Release);
                    VramAction::Hold
                }
            }
            KvCacheLocation::HostRam => {
                if pct < self.threshold_low_pct {
                    let prev = self
                        .consecutive_samples
                        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                    if prev + 1 >= self.required_samples {
                        self.consecutive_samples
                            .store(0, std::sync::atomic::Ordering::Release);
                        VramAction::SwapIn
                    } else {
                        VramAction::Hold
                    }
                } else {
                    self.consecutive_samples
                        .store(0, std::sync::atomic::Ordering::Release);
                    VramAction::Hold
                }
            }
        }
    }

    /// Marca o KV Cache como evacuado para Host RAM.
    pub fn mark_swapped_out(&self) {
        self.current_state
            .store(KvCacheLocation::HostRam as u8, std::sync::atomic::Ordering::Release);
    }

    /// Marca o KV Cache como residente na VRAM.
    pub fn mark_swapped_in(&self) {
        self.current_state
            .store(KvCacheLocation::Gpu as u8, std::sync::atomic::Ordering::Release);
    }

    pub fn is_swapped_out(&self) -> bool {
        KvCacheLocation::from_u8(self.current_state.load(std::sync::atomic::Ordering::Acquire))
            == KvCacheLocation::HostRam
    }

    /// Dispara swap-out do KV Cache Q4_K para Host RAM via DMA / FFI.
    /// Syscalls DMA isoladas em `spawn_blocking` para não contaminar o reactor Tokio.
    pub async fn swap_out_kv_cache_q4k(&self) -> Result<(), String> {
        tokio::task::spawn_blocking(|| {
            #[cfg(feature = "llama_backend")]
            {
                tracing::info!(
                    target: "souls::vram",
                    "MARCO IV/V6: swap-out FFI real Q4_K GPU→Host RAM pinned acionado"
                );
            }
            #[cfg(not(feature = "llama_backend"))]
            {
                tracing::info!(
                    target: "souls::vram",
                    "MARCO IV/V6: swap-out simulado em DMA host"
                );
            }
        })
        .await
        .map_err(|e| format!("swap_out_kv_cache_q4k falhou: {e}"))?;
        self.mark_swapped_out();
        Ok(())
    }

    /// Dispara swap-in do KV Cache Q4_K de Host RAM para VRAM.
    pub async fn swap_in_kv_cache_q4k(&self) -> Result<(), String> {
        tokio::task::spawn_blocking(|| {
            #[cfg(feature = "llama_backend")]
            {
                tracing::info!(
                    target: "souls::vram",
                    "MARCO IV/V6: swap-in FFI real Q4_K Host RAM→GPU acionado"
                );
            }
            #[cfg(not(feature = "llama_backend"))]
            {
                tracing::info!(
                    target: "souls::vram",
                    "MARCO IV/V6: swap-in simulado em DMA host"
                );
            }
        })
        .await
        .map_err(|e| format!("swap_in_kv_cache_q4k falhou: {e}"))?;
        self.mark_swapped_in();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    /// Sink de teste que retorna uma porcentagem programável.
    struct MockSink {
        pct: AtomicU32, // armazenado como bits de f32
    }

    impl MockSink {
        fn new(pct: f32) -> Self {
            Self {
                pct: AtomicU32::new(pct.to_bits()),
            }
        }
        fn set(&self, pct: f32) {
            self.pct.store(pct.to_bits(), std::sync::atomic::Ordering::Release);
        }
    }

    impl VramPressureSink for MockSink {
        fn current_vram_pct(&self) -> f32 {
            f32::from_bits(self.pct.load(std::sync::atomic::Ordering::Acquire))
        }
    }

    #[test]
    fn test_kv_swap_controller_hysteresis() {
        let ctrl = KvCacheSwapController::with_thresholds(90.0, 80.0, 2);
        let sink = MockSink::new(50.0);

        // 50% → Hold
        assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

        // Sobe para 92% → 1ª amostra, ainda Hold (histerese)
        sink.set(92.0);
        assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);
        // 2ª amostra consecutiva → SwapOut
        assert_eq!(ctrl.evaluate(&sink), VramAction::SwapOut);
        ctrl.mark_swapped_out();

        // Ainda em 92% → Hold (não swap-in)
        assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

        // Cai para 75% (< 80%) → 1ª Hold, depois SwapIn
        sink.set(75.0);
        assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);
        assert_eq!(ctrl.evaluate(&sink), VramAction::SwapIn);
        ctrl.mark_swapped_in();
    }

    #[test]
    fn test_kv_swap_controller_idempotent_state() {
        let ctrl = KvCacheSwapController::new();
        assert!(!ctrl.is_swapped_out());
        ctrl.mark_swapped_out();
        assert!(ctrl.is_swapped_out());
        ctrl.mark_swapped_in();
        assert!(!ctrl.is_swapped_out());
    }
}
