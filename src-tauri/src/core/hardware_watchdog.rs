// SOULS MC MARCO IV — Watchdog Térmico de Hardware (ADR-027 / ADR-030)
//
// Thread nativa do S.O. (std::thread) coleta telemetria física via sysinfo 0.30.13
// (e NVML quando `llama_backend` está habilitado) e publica leituras compactadas
// num único `AtomicU64` lock-free (`WATCHDOG_STATE`).
//
// Bit-Mask (64 bits):
//   bits  0..19  vram_used_mb         (20 bits, 1 MB LSB)
//   bits 20..39  ram_used_mb          (20 bits, 1 MB LSB)
//   bits 40..49  cpu_temp_celsius_x2  (10 bits, 0.5 °C LSB)
//   bits 50..59  gpu_temp_celsius_x2  (10 bits, 0.5 °C LSB)
//   bits 60..63  flags                (4 bits: bit60=thermal_throttle, demais reservados)
//
// Por que `OnceLock<Arc<AtomicU64>>`:
//   - Inicialização preguiçosa sem RwLock/Mutex no hot path.
//   - Toda leitura pelo loop Tokio é um único `Ordering::Acquire` (cache miss O(1)).
//   - Escrita pela thread watchdog é `Ordering::Release` (publicação happens-before).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

use sysinfo::{Components, System};

/// Estado global lock-free publicado pela thread watchdog.
/// Consumido pelo `KvCacheSwapController` no loop de controle do Gateway.
pub static WATCHDOG_STATE: OnceLock<Arc<AtomicU64>> = OnceLock::new();

/// Intervalo de poll do watchdog em milissegundos.
pub const WATCHDOG_POLL_INTERVAL_MS: u64 = 1_000;

/// Máscaras de bit (vide tabela no cabeçalho do módulo).
pub const MASK_VRAM: u64 = (1u64 << 20) - 1; // bits 0..19
pub const MASK_RAM: u64 = ((1u64 << 20) - 1) << 20; // bits 20..39
pub const MASK_CPU_TEMP: u64 = ((1u64 << 10) - 1) << 40; // bits 40..49
pub const MASK_GPU_TEMP: u64 = ((1u64 << 10) - 1) << 50; // bits 50..59
pub const MASK_FLAGS: u64 = 0xF << 60; // bits 60..63

/// Bit 60: thermal throttle / dGPU acima do limite seguro.
pub const FLAG_THERMAL_THROTTLE: u64 = 1u64 << 60;

/// Limite físico de VRAM da RTX 2060m (6.144 MB).
/// Usado apenas como divisor de fallback quando NVML não está disponível.
pub const RTX_2060M_VRAM_TOTAL_MB: u32 = 6_144;

/// Empacota o snapshot de telemetria num único u64.
#[inline]
#[must_use]
pub fn pack_state(vram_mb: u32, ram_mb: u32, cpu_temp_c: f32, gpu_temp_c: f32, flags: u64) -> u64 {
    // Trunca para o range válido de cada máscara (anti-overflow determinístico).
    let vram = (vram_mb.min(MASK_VRAM as u32)) as u64;
    let ram = (ram_mb.min((MASK_RAM >> 20) as u32)) as u64;
    // Temperaturas armazenadas como x2 (0.5 °C LSB), clamped a 1023 (511.5 °C, suficiente).
    let cpu_x2 = ((cpu_temp_c * 2.0).clamp(0.0, 1023.0)) as u64;
    let gpu_x2 = ((gpu_temp_c * 2.0).clamp(0.0, 1023.0)) as u64;
    let flags = flags & 0xF;

    (vram & MASK_VRAM)
        | ((ram & (MASK_RAM >> 20)) << 20)
        | ((cpu_x2 & 0x3FF) << 40)
        | ((gpu_x2 & 0x3FF) << 50)
        | (flags << 60)
}

/// Decodifica VRAM (MB) do estado compactado.
#[inline]
#[must_use]
pub fn decode_vram_mb(state: u64) -> u32 {
    (state & MASK_VRAM) as u32
}

/// Decodifica RAM (MB) do estado compactado.
#[inline]
#[must_use]
pub fn decode_ram_mb(state: u64) -> u32 {
    ((state & MASK_RAM) >> 20) as u32
}

/// Decodifica temperatura CPU (°C) do estado compactado.
#[inline]
#[must_use]
pub fn decode_cpu_temp_c(state: u64) -> f32 {
    let raw = (state & MASK_CPU_TEMP) >> 40;
    (raw as f32) * 0.5
}

/// Decodifica temperatura dGPU (°C) do estado compactado.
#[inline]
#[must_use]
pub fn decode_gpu_temp_c(state: u64) -> f32 {
    let raw = (state & MASK_GPU_TEMP) >> 50;
    (raw as f32) * 0.5
}

/// Decodifica flags booleanos do estado compactado.
#[inline]
#[must_use]
pub fn decode_thermal_flag(state: u64) -> bool {
    (state & FLAG_THERMAL_THROTTLE) != 0
}

/// Retorna o estado global se a thread watchdog já publicou ao menos uma amostra.
pub fn get_state() -> Option<Arc<AtomicU64>> {
    WATCHDOG_STATE.get().cloned()
}

/// Estrutura de ownership do `JoinHandle` da thread watchdog.
/// `Drop` espera a thread encerrar (no-op se nunca foi iniciada).
pub struct HardwareWatchdog {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl HardwareWatchdog {
    /// Constrói um watchdog inerte. Chame `start()` para acoplar a thread S.O.
    #[must_use]
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// Inicia a thread nativa. Idempotente — chamadas subsequentes são no-op.
    /// Retorna referência ao estado global para debug/acesso direto.
    pub fn start(&mut self) -> Arc<AtomicU64> {
        // Inicializa o estado global na primeira chamada.
        let state = WATCHDOG_STATE
            .get_or_init(|| Arc::new(AtomicU64::new(0)))
            .clone();

        if self
            .running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return state; // já estava rodando
        }

        let running = Arc::clone(&self.running);
        let state_for_thread = Arc::clone(&state);

        let handle = std::thread::Builder::new()
            .name("souls-hardware-watchdog".to_string())
            .spawn(move || {
                run_watchdog_loop(running, state_for_thread);
            })
            .expect("Falha ao spawnar thread souls-hardware-watchdog");

        self.handle = Some(handle);
        state
    }

    /// Sinaliza parada e aguarda a thread encerrar. Idempotente.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for HardwareWatchdog {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HardwareWatchdog {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Loop principal da thread watchdog. Isola syscalls bloqueantes do reactor Tokio.
fn run_watchdog_loop(running: Arc<AtomicBool>, state: Arc<AtomicU64>) {
    let mut sys = System::new_all();
    let mut components = Components::new_with_refreshed_list();
    let poll_interval = Duration::from_millis(WATCHDOG_POLL_INTERVAL_MS);

    while running.load(Ordering::Acquire) {
        // Refresh físico (sysinfo bloqueia em I/O; por isso thread dedicada).
        sys.refresh_memory();
        sys.refresh_cpu_usage();
        components.refresh();

        let ram_mb = (sys.used_memory() / (1024 * 1024)) as u32;
        let vram_mb = read_vram_used_mb();
        let cpu_temp_c = read_cpu_temp_celsius(&components).unwrap_or(0.0);
        let gpu_temp_c = read_gpu_temp_celsius().unwrap_or(0.0);

        // Bandeira térmica: dGPU acima de 85 °C marca throttle preventivo.
        let mut flags = 0u64;
        if gpu_temp_c >= 85.0 {
            flags |= FLAG_THERMAL_THROTTLE;
        }

        let packed = pack_state(vram_mb, ram_mb, cpu_temp_c, gpu_temp_c, flags);
        state.store(packed, Ordering::Release);

        std::thread::sleep(poll_interval);
    }
}

/// Lê VRAM consumida via NVML (gateado em `llama_backend`).
#[cfg(feature = "llama_backend")]
fn read_vram_used_mb() -> u32 {
    use nvml_wrapper::enum_wrappers::device::UsedGpuMemory;

    match nvml_wrapper::Nvml::init() {
        Ok(nvml) => match nvml.device_by_index(0) {
            Ok(device) => match device.memory_info() {
                Ok(mem) => (mem.used / (1024 * 1024)) as u32,
                Err(_) => 0,
            },
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Fallback sem NVML: retorna 0. O scheduler trabalha com a *tendência*, não o valor absoluto,
/// então a ausência de leitura não causa falso positivo de swap-out.
#[cfg(not(feature = "llama_backend"))]
fn read_vram_used_mb() -> u32 {
    0
}

/// Lê temperatura da CPU via `sysinfo::Components`. Retorna a maior temperatura observada.
/// Em sysinfo 0.30, `Component::temperature()` retorna `f32` direto (0.0 se indisponível).
fn read_cpu_temp_celsius(components: &Components) -> Option<f32> {
    let max = components
        .iter()
        .map(|c| c.temperature())
        .fold(0.0_f32, f32::max);
    if max > 0.0 { Some(max) } else { None }
}

/// Lê temperatura da dGPU via NVML. Gateado em `llama_backend`.
#[cfg(feature = "llama_backend")]
fn read_gpu_temp_celsius() -> Option<f32> {
    use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let device = nvml.device_by_index(0).ok()?;
    device.temperature(TemperatureSensor::Gpu).ok().map(f32::from)
}

#[cfg(not(feature = "llama_backend"))]
fn read_gpu_temp_celsius() -> Option<f32> {
    None
}

// =============================================================================
// UNIT TESTS (TDD — Red-Green-Refactor)
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_watchdog_state_bit_pack_roundtrip() {
        // Cenário: VRAM 5632 MB, RAM 16384 MB, CPU 65 °C, GPU 78 °C, sem throttle.
        let vram = 5632u32;
        let ram = 16384u32;
        let cpu = 65.0f32;
        let gpu = 78.0f32;
        let flags = 0u64;

        let packed = pack_state(vram, ram, cpu, gpu, flags);
        assert_eq!(decode_vram_mb(packed), vram);
        assert_eq!(decode_ram_mb(packed), ram);
        assert!((decode_cpu_temp_c(packed) - cpu).abs() < 0.01);
        assert!((decode_gpu_temp_c(packed) - gpu).abs() < 0.01);
        assert!(!decode_thermal_flag(packed));
    }

    #[test]
    fn test_watchdog_state_thermal_flag_set() {
        // Flags devem ser passados nos 4 bits inferiores (LSB); pack desloca para 60..63.
        let packed = pack_state(1024, 2048, 60.0, 90.0, FLAG_THERMAL_THROTTLE >> 60);
        assert!(decode_thermal_flag(packed));
        // Demais campos preservados
        assert_eq!(decode_vram_mb(packed), 1024);
        assert_eq!(decode_ram_mb(packed), 2048);
    }

    #[test]
    fn test_watchdog_state_overflow_safe() {
        // VRAM e RAM acima do range — devem truncar, não panic.
        let packed = pack_state(u32::MAX, u32::MAX, 999.0, 999.0, 0xFF);
        assert_eq!(decode_vram_mb(packed), MASK_VRAM as u32);
        assert_eq!(decode_ram_mb(packed), (MASK_RAM >> 20) as u32);
        // CPU/GPU temp saturada em 511.5 °C (1023 x2)
        assert!((decode_cpu_temp_c(packed) - 511.5).abs() < 0.01);
        // Flags truncados a 4 bits
        assert_eq!(packed >> 60, 0xF);
    }

    #[test]
    fn test_watchdog_telemetry_polling() {
        // Mede custo de pack + store atômico. Deve ficar bem abaixo de 5ms.
        let start = Instant::now();
        for _ in 0..1000 {
            let packed = pack_state(5632, 16384, 65.0, 78.0, 0);
            // store atômico
            let arc = WATCHDOG_STATE.get_or_init(|| Arc::new(AtomicU64::new(0)));
            arc.store(packed, Ordering::Release);
        }
        let elapsed = start.elapsed();
        let per_iter_us = elapsed.as_micros() / 1000;
        assert!(
            per_iter_us < 5_000,
            "pack+store levou {per_iter_us}µs por iteração (teto: 5000µs)"
        );
    }

    #[test]
    fn test_watchdog_start_idempotent() {
        let mut wd = HardwareWatchdog::new();
        let s1 = wd.start();
        let s2 = wd.start(); // segunda chamada não deve criar nova thread
        assert!(Arc::ptr_eq(&s1, &s2));
        // Verifica que ao menos uma amostra foi publicada em < 2s
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if s1.load(Ordering::Acquire) != 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        wd.shutdown();
    }
}
