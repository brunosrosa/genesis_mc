// SOULS V6 MARCO 5.12.0 / PACOTE 5 — Test Suite TDD do VRAM Scheduler e Algema AVX2
//
// Valida:
// 1. Histerese Anti-Flap no KvCacheSwapController (2 amostras consecutivas).
// 2. Coerção de Logits JSON via llguidance em Registradores SIMD AVX2 (< 50µs).
// 3. Integridade de Bit-Masking do AtomicU64 do Hardware Watchdog.

use super::*;
use llguidance::toktrie;
use std::sync::atomic::AtomicU32;
use std::time::Instant;

/// Sink de teste programável para simulação de pressão de VRAM.
struct MockPressureSink {
    pct: AtomicU32,
    used_mb: AtomicU32,
    total_mb: u32,
}

impl MockPressureSink {
    fn new(pct: f32) -> Self {
        let total_mb = 6144;
        let used_mb = ((pct / 100.0) * total_mb as f32) as u32;
        Self {
            pct: AtomicU32::new(pct.to_bits()),
            used_mb: AtomicU32::new(used_mb),
            total_mb,
        }
    }

    fn set(&self, pct: f32) {
        self.pct.store(pct.to_bits(), Ordering::Release);
        let used = ((pct / 100.0) * self.total_mb as f32) as u32;
        self.used_mb.store(used, Ordering::Release);
    }
}

impl VramPressureSink for MockPressureSink {
    fn current_vram_pct(&self) -> f32 {
        f32::from_bits(self.pct.load(Ordering::Acquire))
    }

    fn vram_metrics_mb(&self) -> (u32, u32) {
        (self.used_mb.load(Ordering::Acquire), self.total_mb)
    }
}

#[test]
fn test_vram_scheduler_hysteresis_anti_flap() {
    let ctrl = KvCacheSwapController::with_thresholds(90.0, 80.0, 2);
    let sink = MockPressureSink::new(50.0);

    // Estado inicial: Gpu
    assert!(!ctrl.is_swapped_out());
    assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

    // Flutuação oscilante: 89% (Hold) -> 91% (1ª amostra, Hold) -> 89% (Reset, Hold) -> 91% (1ª amostra, Hold)
    sink.set(89.0);
    assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

    sink.set(91.0);
    assert_eq!(
        ctrl.evaluate(&sink),
        VramAction::Hold,
        "1ª amostra em 91% não deve acionar SwapOut (histerese anti-flap)"
    );

    sink.set(89.0);
    assert_eq!(
        ctrl.evaluate(&sink),
        VramAction::Hold,
        "Queda para 89% deve resetar o contador consecutivo"
    );

    sink.set(91.0);
    assert_eq!(
        ctrl.evaluate(&sink),
        VramAction::Hold,
        "1ª amostra em 91% após reset ainda é Hold"
    );

    // 2ª leitura consecutiva estável em 92% (>= 90%) -> SwapOut
    sink.set(92.0);
    assert_eq!(
        ctrl.evaluate(&sink),
        VramAction::SwapOut,
        "2ª amostra consecutiva >= 90% deve disparar SwapOut"
    );
    ctrl.mark_swapped_out();
    assert!(ctrl.is_swapped_out());

    // Ainda em 92% -> Hold
    assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

    // Flutuação de retorno: 81% -> 79% (1ª amostra < 80%, Hold) -> 82% (Reset) -> 78% (1ª amostra) -> 77% (2ª amostra -> SwapIn)
    sink.set(81.0);
    assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

    sink.set(79.0);
    assert_eq!(
        ctrl.evaluate(&sink),
        VramAction::Hold,
        "1ª amostra em 79% não deve acionar SwapIn"
    );

    sink.set(82.0);
    assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

    sink.set(78.0);
    assert_eq!(ctrl.evaluate(&sink), VramAction::Hold);

    sink.set(77.0);
    assert_eq!(
        ctrl.evaluate(&sink),
        VramAction::SwapIn,
        "2ª amostra consecutiva < 80% deve disparar SwapIn"
    );
    ctrl.mark_swapped_in();
    assert!(!ctrl.is_swapped_out());
}

#[tokio::test]
async fn test_kv_cache_physical_dma_swapping() {
    let ctrl = KvCacheSwapController::new();
    assert!(!ctrl.is_swapped_out());
    assert_eq!(ctrl.swapped_bytes(), 0);

    // Executa SwapOut físico
    ctrl.swap_out_kv_cache_q4k().await.unwrap();
    assert!(ctrl.is_swapped_out());
    assert_eq!(ctrl.swapped_bytes(), 128 * 1024 * 1024);
    assert!(ctrl.last_swap_timestamp() > 0);

    // Executa SwapIn físico (reidratação JIT)
    ctrl.swap_in_kv_cache_q4k().await.unwrap();
    assert!(!ctrl.is_swapped_out());
    assert_eq!(ctrl.swapped_bytes(), 0);
}

#[test]
fn test_llguidance_avx2_json_coercion_speed() {
    // Cria vocabulário sintético de teste (128 tokens para simular vocabulário de código)
    let vocab_strings: Vec<String> = (0..128)
        .map(|i| match i {
            0 => "{\"".to_string(),
            1 => "status".to_string(),
            2 => "\":".to_string(),
            3 => "\"ok\"".to_string(),
            4 => "}".to_string(),
            5 => " [INVALID_TOKEN] ".to_string(),
            6 => " <MALFORMED_SLOP> ".to_string(),
            _ => format!(" tok_{i} "),
        })
        .collect();

    let vocab_bytes: Vec<&[u8]> = vocab_strings.iter().map(|s| s.as_bytes()).collect();

    // Cria máscara de teste SimpleVob onde apenas os tokens 0..5 são válidos
    let mut mask = toktrie::SimpleVob::alloc(128);
    mask.set(0, true);
    mask.set(1, true);
    mask.set(2, true);
    mask.set(3, true);
    mask.set(4, true);

    // Vetor de logits de ruído estocástico
    let mut logits: Vec<f32> = (0..128).map(|i| (i as f32 * 0.37).sin() * 5.0).collect();

    // Mede a velocidade de coerção na CPU
    let start = Instant::now();
    for _ in 0..1000 {
        let mut sample_logits = logits.clone();
        mask_logits(&mut sample_logits, &mask);
    }
    let elapsed = start.elapsed();
    let per_token_us = elapsed.as_micros() as f64 / 1000.0;

    // Assevera tempo de coerção < 50 microssegundos
    assert!(
        per_token_us < 50.0,
        "Coerção AVX2 levou {per_token_us:.2}µs por token (teto: 50.0µs)"
    );

    // Aplica na cópia final e valida que os tokens inválidos foram mascarados para -inf
    mask_logits(&mut logits, &mask);
    for idx in 0..5 {
        assert!(logits[idx] > -100.0, "Token {idx} deveria ser permitido");
    }
    for idx in 5..128 {
        assert_eq!(
            logits[idx],
            f32::NEG_INFINITY,
            "Token {idx} deveria estar mascarado para -inf"
        );
    }

    // Valida inicialização do motor llguidance com schema JSON SODA
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "status": { "type": "string" }
        },
        "required": ["status"]
    });

    let mut engine = LlguidanceJsonEngine::new_json_schema(schema, &vocab_bytes, 4).unwrap();
    let mut live_logits = vec![0.0f32; 128];
    let coerce_res = engine.coerce_and_mask_logits(&mut live_logits);
    assert!(coerce_res.is_ok(), "Fail-closed check deve retornar Ok");
}

#[test]
fn test_watchdog_state_bit_masking_integrity() {
    // Casos de teste com valores arbitrários
    let test_cases = vec![
        (5632u32, 16384u32, 65.0f32, 78.5f32, 0u64),
        (1024u32, 8192u32, 45.0f32, 85.0f32, hardware_watchdog::FLAG_THERMAL_THROTTLE >> 60),
        (6144u32, 32768u32, 82.5f32, 91.0f32, 0x1u64),
        (0u32, 512u32, 30.0f32, 35.0f32, 0u64),
    ];

    for (vram, ram, cpu, gpu, flags) in test_cases {
        let packed = hardware_watchdog::pack_state(vram, ram, cpu, gpu, flags);

        assert_eq!(
            hardware_watchdog::decode_vram_mb(packed),
            vram,
            "VRAM mismatch"
        );
        assert_eq!(
            hardware_watchdog::decode_ram_mb(packed),
            ram,
            "RAM mismatch"
        );
        assert!(
            (hardware_watchdog::decode_cpu_temp_c(packed) - cpu).abs() <= 0.5,
            "CPU temp mismatch: got {}, expected {}",
            hardware_watchdog::decode_cpu_temp_c(packed),
            cpu
        );
        assert!(
            (hardware_watchdog::decode_gpu_temp_c(packed) - gpu).abs() <= 0.5,
            "GPU temp mismatch: got {}, expected {}",
            hardware_watchdog::decode_gpu_temp_c(packed),
            gpu
        );
    }
}
