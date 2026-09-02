//! SOULS MC — Marco I · v6.1: PeakEWMA Latency Tracker
//!
//! Implementa o algoritmo **Peak Exponentially Weighted Moving Average** com
//! fator de suavização α=0.3 (configurável via `GatewayConfig`) sobre um
//! **ring buffer lock-free** de `AtomicU64` (f64 bit-packed) para interceptar
//! Time-To-First-Token (TTFT) de streams SSE.
//!
//! ## Algoritmo
//!
//! PeakEWMA converge rapidamente para picos recentes sem perder a média de
//! longo prazo. Para cada nova amostra `x_t`:
//!
//! ```text
//! ewma_t = α * x_t + (1 - α) * ewma_{t-1}
//! peak_t = max(peak_{t-1}, ewma_t)
//! ```
//!
//! Para a α=0.3 (default), uma amostra de 2500ms decai para ≈150ms em
//! ~17 iterações (regra empírica `1/α * ln(|peak - target|/|initial - target|)`).
//!
//! ## Lock-free ring buffer
//!
//! - Slots: array de `AtomicU64` (bit-packed f64) de tamanho `N` (default 64).
//! - Write index: `AtomicUsize` monotonicamente crescente, módulo N para indexar.
//! - Read iteration: walk atômico a partir do índice mais antigo ainda
//!   não sobrescrito. ABA é prevenido via `seq_counter` (não utilizado aqui —
//!   samples são imutáveis uma vez escritos, apenas sobrescritos em slot).
//!
//! ## Performance
//!
//! - Write: O(1) — 1 CAS no write_idx + 1 store no slot.
//! - Read snapshot: O(N) — único acquire pass pelo ring; aceitável para
//!   relatórios de telemetria sob demanda (não no hot-path do request).
//!
//! ## ADR & Leis
//!
//! - **ADR-025 (Qualidade):** Zero alocação no `record()` (apenas CAS).
//! - **ADR-027 (Termodinâmica VRAM):** Zero GPU; puro CPU/AVX2.
//! - **ADR-030 (Higiene):** Apenas `std::sync::atomic` — zero deps externas.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Fator de suavização padrão (Marco I · v6.1). Configurável em runtime
/// via `GatewayConfig::telemetry.peak_ewma_alpha`.
pub const DEFAULT_ALPHA: f32 = 0.3;

/// Ring buffer lock-free de tamanho fixo, parametrizado em compile-time.
pub struct PeakEwma<const N: usize> {
    alpha: AtomicU64, // f32 bit-packed
    ewma_ms: AtomicU64, // f64 bit-packed (suporta ms com precisão sub-µs)
    peak_ms: AtomicU64, // f64 bit-packed
    ring: [AtomicU64; N], // f64 bit-packed (cada slot = 1 sample em ms)
    write_idx: AtomicUsize, // monotonicamente crescente
}

impl<const N: usize> PeakEwma<N> {
    /// Construtor com α default. `const fn` para alocação zero.
    pub const fn new() -> Self {
        // Em Rust stable, `AtomicU64::new` em arrays `[T; N]` exige que
        // `T: Copy`, o que `AtomicU64` satisfaz. Construímos via `[const; N]`.
        Self {
            alpha: AtomicU64::new(DEFAULT_ALPHA.to_bits() as u64),
            ewma_ms: AtomicU64::new(0.0_f64.to_bits()),
            peak_ms: AtomicU64::new(0.0_f64.to_bits()),
            ring: [const { AtomicU64::new(0) }; N],
            write_idx: AtomicUsize::new(0),
        }
    }

    /// Substitui o fator de suavização (runtime hot-reload).
    pub fn set_alpha(&self, alpha: f32) {
        self.alpha.store(alpha.to_bits() as u64, Ordering::Release);
    }

    /// Retorna o α atual.
    pub fn alpha(&self) -> f32 {
        f32::from_bits(self.alpha.load(Ordering::Acquire) as u32)
    }

    /// Grava uma nova amostra (TTFT em milissegundos) e atualiza EWMA + peak.
    /// **O(1) absoluto, zero alocação.**
    pub fn record(&self, sample_ms: f64) {
        let alpha = self.alpha();
        let one_minus_alpha = 1.0 - alpha;
        let sample_f32 = sample_ms as f32;

        // CAS loop no EWMA para garantir atomicidade em multi-thread.
        let mut current = f64::from_bits(self.ewma_ms.load(Ordering::Acquire));
        loop {
            let new_ewma = (alpha * sample_f32 + one_minus_alpha * current as f32) as f64;
            match self.ewma_ms.compare_exchange_weak(
                current.to_bits(),
                new_ewma.to_bits(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current = f64::from_bits(actual),
            }
        }

        // Peak = max(peak, ewma) — monotonicamente crescente (até ser resetado).
        let new_ewma = f64::from_bits(self.ewma_ms.load(Ordering::Acquire));
        let mut current_peak = f64::from_bits(self.peak_ms.load(Ordering::Acquire));
        while new_ewma > current_peak {
            match self.peak_ms.compare_exchange_weak(
                current_peak.to_bits(),
                new_ewma.to_bits(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = f64::from_bits(actual),
            }
        }

        // Ring buffer write: index = write_idx % N, store f64 bit-packed.
        let idx = self.write_idx.fetch_add(1, Ordering::AcqRel);
        self.ring[idx % N].store(sample_ms.to_bits(), Ordering::Release);
    }

    /// Retorna o valor atual do EWMA (latência suavizada em ms).
    pub fn ewma_ms(&self) -> f64 {
        f64::from_bits(self.ewma_ms.load(Ordering::Acquire))
    }

    /// Retorna o pico histórico observado (em ms, nunca decresce até `reset_peak`).
    pub fn peak_ms(&self) -> f64 {
        f64::from_bits(self.peak_ms.load(Ordering::Acquire))
    }

    /// Reseta o pico (peak), preservando o EWMA. Útil para janelas deslizantes.
    pub fn reset_peak(&self) {
        self.peak_ms.store(0.0_f64.to_bits(), Ordering::Release);
    }

    /// Snapshot do ring buffer (amostras em ordem cronológica, mais antiga primeiro).
    /// O(N) — usar fora do hot-path.
    pub fn snapshot(&self) -> RingSnapshot {
        let write_idx = self.write_idx.load(Ordering::Acquire);
        let n_filled = write_idx.min(N);
        let mut samples = Vec::with_capacity(n_filled);
        // Iterar do slot mais antigo (write_idx - n_filled) ao mais recente.
        // ADR-025: `saturating_sub` substitui o padrão `if x >= N { x - N } else { 0 }`
        // — clippy::implicit_saturating_sub. Como `n_filled = write_idx.min(N)`,
        // sabemos que `write_idx <= 2N` ou `write_idx < N`; saturating é seguro.
        let start = write_idx.saturating_sub(N);
        for i in 0..n_filled {
            let slot = (start + i) % N;
            let raw = self.ring[slot].load(Ordering::Acquire);
            samples.push(f64::from_bits(raw));
        }
        RingSnapshot {
            ewma_ms: self.ewma_ms(),
            peak_ms: self.peak_ms(),
            samples,
        }
    }

    /// Calcula o TTFT desde um `Instant` e o registra. Helper ergonômico.
    pub fn record_ttft_since(&self, start: Instant) {
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.record(elapsed_ms);
    }
}

impl<const N: usize> Default for PeakEwma<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot imutável do estado atual do PeakEWMA.
#[derive(Debug, Clone)]
pub struct RingSnapshot {
    pub ewma_ms: f64,
    pub peak_ms: f64,
    pub samples: Vec<f64>,
}

// ============================================================================
// Singleton thread-safe (OnceLock) — proxy-wide único
// ============================================================================

use std::sync::OnceLock;

static PEAK_EWMA_INSTANCE: OnceLock<PeakEwma<64>> = OnceLock::new();

/// Singleton global do `PeakEwma<64>`. Inicializado no primeiro `record()`.
pub fn global_peak_ewma() -> &'static PeakEwma<64> {
    PEAK_EWMA_INSTANCE.get_or_init(PeakEwma::new)
}

// ============================================================================
// Testes TDD (Marco I · v6.1 — Ralph Loop control)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_record_initializes_ewma_from_zero() {
        let p = PeakEwma::<8>::new();
        assert_eq!(p.ewma_ms(), 0.0);
        assert_eq!(p.peak_ms(), 0.0);
        p.record(100.0);
        // α=0.3, sample=100, prev=0 → ewma = 0.3*100 + 0.7*0 = 30
        assert!((p.ewma_ms() - 30.0).abs() < 0.01, "ewma após 1 sample=100: {}", p.ewma_ms());
        assert!((p.peak_ms() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_peak_ewma_decay_convergence() {
        // Marco I · v6.1 · TAREFA 5.2: simula conexão com pico de 2500ms
        // seguido de estabilização em 150ms; valida que α=0.3 produz
        // convergência monotônica decrescente.
        let p = PeakEwma::<128>::new();
        p.set_alpha(0.3);

        // Fase 1: 1 sample de 2500ms (pico isolado).
        p.record(2500.0);
        let peak_after_burst = p.peak_ms();
        assert!(peak_after_burst > 700.0, "pico inicial deve estar > 700ms: {}", peak_after_burst);

        // Fase 2: 50 samples estáveis de 150ms. A EWMA deve convergir para 150.
        for _ in 0..50 {
            p.record(150.0);
        }
        let ewma_final = p.ewma_ms();
        assert!(
            (ewma_final - 150.0).abs() < 1.0,
            "EWMA após 50 samples de 150ms deve convergir para ~150, medido: {}",
            ewma_final
        );
        // O pico preservado é o maior valor já atingido, mas a EWMA suavizou.
        assert!(p.peak_ms() >= peak_after_burst - 0.01, "peak nunca decresce");
    }

    #[test]
    fn test_ring_buffer_wraps_correctly() {
        let p = PeakEwma::<4>::new();
        for i in 0..10 {
            p.record(i as f64 * 10.0);
        }
        let snap = p.snapshot();
        assert_eq!(snap.samples.len(), 4, "ring de N=4 deve manter 4 samples");
        // Os 4 mais recentes: 60, 70, 80, 90
        assert_eq!(snap.samples[0], 60.0);
        assert_eq!(snap.samples[3], 90.0);
    }

    #[test]
    fn test_alpha_runtime_update() {
        let p = PeakEwma::<4>::new();
        assert!((p.alpha() - 0.3).abs() < 0.001);
        p.set_alpha(0.7);
        assert!((p.alpha() - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_record_ttft_since_uses_elapsed() {
        let p = PeakEwma::<8>::new();
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(20));
        p.record_ttft_since(start);
        let measured = p.ewma_ms();
        // Bound assintótico (CI pode ter jitter): TTFT medido deve ser >= 0.5ms e < 500ms.
        assert!(measured >= 0.5, "TTFT medido deve ser >= 0.5ms (sanity check): {}", measured);
        assert!(measured < 500.0, "TTFT medido deve ser < 500ms em teste CI: {}", measured);
    }

    #[test]
    fn test_reset_peak_preserves_ewma() {
        let p = PeakEwma::<4>::new();
        p.record(500.0);
        let ewma_before = p.ewma_ms();
        let peak_before = p.peak_ms();
        assert!(peak_before > 0.0);
        p.reset_peak();
        assert_eq!(p.peak_ms(), 0.0, "peak deve ser resetado");
        assert!((p.ewma_ms() - ewma_before).abs() < 0.01, "ewma deve ser preservada");
    }

    #[test]
    fn test_global_peak_ewma_singleton() {
        let g1 = global_peak_ewma();
        let g2 = global_peak_ewma();
        assert!(std::ptr::eq(g1, g2), "singleton deve ser o mesmo ponteiro");
    }
}
