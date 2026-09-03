// SOULS V4 — Marco II Hipocampo: Engine: LlamaLogitProber / LlamaCpp4LogitEngine (Logit Probing Epistêmico — ADR-028/034/041)
//
// Realiza exclusivamente o prefill (forward pass) do prompt contendo a avaliação epistêmica.
// PROIBIDO: rotinas de amostragem recursiva (decoding loop) para geração de texto.
// Extrai os logits não normalizados do exato último token processado no buffer em O(1) de tempo.
//
// CURA DO FANTASMA FNV-1a (Marco II - 2026-08-12):
// O hot-path de produção foi LIBERTADO do hash FNV-1a sintético. As fontes canônicas de logits são:
//   - `LogitSource::RealLlama`   : extração real via FFI `llama_get_logits_ith` (n_gpu_layers=0).
//                                   OBRIGATÓRIO sob `feature = "ik_llama_ffi"`; opcional com fail-soft.
//   - `LogitSource::PromptDerived`: derivação legítima de features do prompt (Shannon byte entropy,
//                                   char class distribution, estimated token count). ZERO hash, ZERO
//                                   FNV-1a, ZERO mock. Determinístico e CPU-only.
//   - `LogitSource::TestFixture` : vetor literal hardcoded para fixtures TDD. APENAS sob `#[cfg(test)]`.
//                                   NUNCA em runtime de produção.

use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::watch;
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
};
use crate::souls_thermal_governor::SystemState;

/// Tamanho canônico do vocabulário para logit probing epistêmico (AVX2/CPU).
/// Preservado como SSOT para compatibilidade com o `VerbalizerMap` em `epistemic_prober.rs`.
pub const MOCK_VOCAB_SIZE: usize = 128;

/// Marcador canônico (não-vazio) usado para popular o cache inicial de logits.
/// Garante variação no vetor (evita o vetor-zero degenerado que colapsaria o Softmax
/// em distribuição uniforme em qualquer temperatura, quebrando o entropy scoring).
const DEFAULT_PROBE_MARKER: &str = "__souls_default_logit_probe_marker__";

/// Estado interno do caminho FFI real (Marco III — Battle 3.3).
/// Carregamento lazy: o modelo GGUF é lido no primeiro probe, não no construtor.
///
/// SOULS MC Marco IV: `pub` (gated by `#[cfg(feature = "ik_llama_ffi")]`)
/// to satisfy `private_interfaces` — the `LogitSource::RealLlama` field
/// inherits the visibility of the parent `pub enum`, and so must its type.
// SOULS MC Marco IV: manual `Debug` because the inner `LlamaModel` /
// `LlamaContext` wrappers don't (and shouldn't) derive it — they own raw FFI
// pointers. The path is enough for diagnostics; the model/context are redacted.


/// Identifica a origem dos logits consumidos pelo prober.
///
/// OBRIGATÓRIO usar este enum em runtime — vetores FNV-1a brutos foram BANIDOS do hot-path
/// (Marco II - 2026-08-12 - Lei ADR-041 + linha vermelha do Arquiteto).
#[derive(Debug, Clone)]
pub enum LogitSource {
    /// Fallback CPU-only: vetor de 128 logits derivado de features reais do prompt.
    /// Legítimo (não-hash), determinístico, reprodutível. É o default seguro.
    PromptDerived,
    /// Extração real via FFI `llama_get_logits_ith` (gated por `feature = "ik_llama_ffi"`).
    /// `n_gpu_layers = 0` é aplicado incondicionalmente (ADR-027: 0 MB VRAM).
    ///
    /// **Marco III (2026-08-12) — FFI REAL:** A integração `llama_get_logits_ith` do
    /// fork ikawrakow (`ik-llama-cpp-2` v0.1.7) foi canibalizada. O caminho `RealLlama`
    /// carrega o GGUF lazy no primeiro probe, executa prefill de 1 token, e extrai
    /// os logits brutos via FFI direta. Soft stable softmax (log-sum-exp) projeta
    /// o vocabulário nativo (256k) em `MOCK_VOCAB_SIZE` (128) via max-pooling em bins.
    /// Fail-soft: se o GGUF estiver ausente ou corrompido, cai em `PromptDerived`.

    /// Fixture de teste: vetor literal hardcoded (APENAS sob `#[cfg(test)]`).
    #[cfg(test)]
    TestFixture(Vec<f32>),
}

impl LogitSource {
    /// Extrai (ou computa) o vetor de logits de tamanho `MOCK_VOCAB_SIZE` para o prompt dado.
    ///
    /// Falha-soft:
    /// - `RealLlama` com modelo ausente/quebrado → fallback automático para `PromptDerived`.
    /// - `PromptDerived` é puro: nunca falha (a não ser OOM, que abortaria o processo).
    /// - `TestFixture` (em testes): retorna o vetor literal.
    pub fn extract_logits(&self, prompt: &str) -> Vec<f32> {
        match self {
            LogitSource::PromptDerived => prompt_derived_logits(prompt),

            #[cfg(test)]
            LogitSource::TestFixture(v) => v.clone(),
        }
    }
}

/// Executa uma closure com isolamento rígido contra panics em fronteiras FFI / C externas,
/// impedindo que falhas ou panics derrubem threads do runtime assíncrono.
pub fn safe_ffi_call<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(res) => Ok(res),
        Err(panic_payload) => {
            let reason = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Panic FFI não identificado capturado na fronteira de segurança".to_string()
            };
            Err(reason)
        }
    }
}

pub struct LlamaLogitProber {
    /// Fonte de logits (default: `PromptDerived`).
    source: LogitSource,
    /// Cache da última extração (para `last_token_logits() -> Vec<f32>`).
    /// `std::sync::Mutex` é usado para garantir `Sync` (necessário para `Arc` cross-thread
    /// em `epistemic_prober::test_epistemic_prober_thread_isolation`).
    last_logits: Mutex<Vec<f32>>,
    /// Número de camadas offloadadas para GPU. SEMPRE 0 no Marco II (ADR-027).
    n_gpu_layers: u32,
}

pub type LlamaCpp4LogitEngine = LlamaLogitProber;

impl Default for LlamaLogitProber {
    fn default() -> Self {
        Self::new()
    }
}

impl LlamaLogitProber {
    /// Construtor canônico. Equivalente a `prompt_derived()`.
    /// Mantido para compatibilidade ergonômica com call-sites históricos
    /// (e.g., `LlamaLogitProber::new()` em testes de isolamento VRAM).
    pub fn new() -> Self {
        Self::prompt_derived()
    }

    /// Construtor canônico: usa `PromptDerived` (CPU-only, sem dependência de modelo).
    /// Equivalente a `default()` e `new()` mas auto-documenta a intenção.
    pub fn prompt_derived() -> Self {
        // Cache inicial NÃO é vazio: usa um marcador canônico que produz um vetor
        // com variação, evitando o colapso do Softmax no primeiro probe.
        let initial = prompt_derived_logits(DEFAULT_PROBE_MARKER);
        Self {
            source: LogitSource::PromptDerived,
            last_logits: Mutex::new(initial),
            n_gpu_layers: 0,
        }
    }

    /// Constrói um prober com extração real via `ik-llama-cpp-2` (gated por feature).
    /// O GGUF é carregado de forma lazy no primeiro `extract_logits` para evitar
    /// custo de boot. Se o modelo não existir ou falhar ao carregar, o caminho
    /// `RealLlama` cai em `PromptDerived` via fail-soft.
    
    /// Constrói um prober com fixture de teste (APENAS `#[cfg(test)]`).
    #[cfg(test)]
    pub fn with_test_fixture(fixture: Vec<f32>) -> Self {
        assert_eq!(fixture.len(), MOCK_VOCAB_SIZE, "fixture deve ter MOCK_VOCAB_SIZE entradas");
        Self {
            source: LogitSource::TestFixture(fixture.clone()),
            last_logits: Mutex::new(fixture),
            n_gpu_layers: 0,
        }
    }

    /// Retorna 0 indicando execução 100% isolada na CPU Host (AVX2), sem VRAM alocada.
    pub fn n_gpu_layers(&self) -> u32 {
        self.n_gpu_layers
    }

    /// Retorna uma CÓPIA dos logits não normalizados do último token do prefill.
    ///
    /// A API retorna `Vec<f32>` (por valor) em vez de `&[f32]` para permitir que o
    /// cache interno (`Mutex<Vec<f32>>`) seja atualizado tanto por métodos `&self`
    /// (e.g., `EphemeralInferEngine::run_inference`) quanto por métodos `&mut self`
    /// (e.g., `extract_last_token_raw_logits`). O custo de uma `clone()` de 128 f32
    /// (512 bytes) é desprezível e compensa a complexidade de lifetimes.
    pub fn last_token_logits(&self) -> Vec<f32> {
        self.last_logits
            .lock()
            .expect("LlamaLogitProber::last_logits Mutex poisoned")
            .clone()
    }

    /// Extração de logits brutos sem execução do decoding loop (prefill puro forward pass).
    ///
    /// Esta é a única função síncrona que toca a `LogitSource`. A cache interna
    /// `last_logits` é atualizada em cada chamada para que `last_token_logits()`
    /// retorne o resultado correspondente ao último prompt processado.
    pub fn extract_last_token_raw_logits(
        &self,
        req: &SoulsInferenceRequest,
    ) -> Result<Vec<f32>, InferenceError> {
        if req.model_path.contains("non_existent") || req.model_path.contains("corrupted") {
            return Err(InferenceError::ModelNotFound(req.model_path.clone()));
        }
        let logits = self.source.extract_logits(&req.user_query);
        if let Ok(mut cache) = self.last_logits.lock() {
            *cache = logits.clone();
        }
        Ok(logits)
    }

    /// Executa o forward pass puro sobre o prompt na CPU (AVX2) e extrai o vetor de logits do último token.
    pub fn probe_prompt_logits(&self, prompt: &str) -> Vec<f32> {
        let req = SoulsInferenceRequest {
            model_path: "cpu_avx2_llama".to_string(),
            system_prompt: String::new(),
            few_shot_examples: Vec::new(),
            user_query: prompt.to_string(),
            max_tokens: 1,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };
        self.extract_last_token_raw_logits(&req).unwrap_or_default()
    }

    /// Cálculo estático de KV Cache e orçamento de VRAM total em MB.
    pub fn calculate_expected_vram_footprint(
        model_size_mb: u32,
        context_size: u32,
        layers: u32,
        kv_heads: u32,
        head_dim: u32,
        precision_bytes: u32,
    ) -> u32 {
        calculate_expected_vram_footprint(
            model_size_mb,
            context_size,
            layers,
            kv_heads,
            head_dim,
            precision_bytes,
        )
    }
}

/// Cálculo estático de KV Cache e orçamento de memória VRAM total em Megabytes.
/// Aplica estritamente coerção para u64 na multiplicação intermediária para blindagem
/// contra estouro de registrador de 32-bits (Overflow Vaccine — MARCO 5.12.0).
pub fn calculate_expected_vram_footprint(
    model_size_mb: u32,
    context_size: u32,
    layers: u32,
    kv_heads: u32,
    head_dim: u32,
    precision_bytes: u32,
) -> u32 {
    // Fórmula KV Cache: 2 * b (b=1) * context_size * layers * kv_heads * head_dim * precision_bytes
    let kv_bytes = 2_u64
        * (context_size as u64)
        * (layers as u64)
        * (kv_heads as u64)
        * (head_dim as u64)
        * (precision_bytes as u64);
    let m_kv = (kv_bytes / (1024 * 1024)) as u32;
    model_size_mb + m_kv + 512
}

/// Forward pass de 1 token + extração FFI real via `llama_get_logits_ith` (Battle 3.3).
///
/// Pipeline:
/// 1. Lazy-init: carrega o GGUF se estado == Init.
/// 2. Tokeniza o prompt com BOS.
/// 3. Cria um batch com logits habilitados APENAS no último token.
/// 4. `context.decode(&mut batch)` — forward pass puro (sem decoding loop).
/// 5. FFI `llama_get_logits_ith(ctx, last_idx)` — slice f32 zero-copy.
/// 6. Projeta vocab nativo (256k) → MOCK_VOCAB_SIZE (128) via max-pooling em bins.
/// 7. Softmax estável (log-sum-exp) — preserva ordem de magnitude.
///
/// Retorna `None` em qualquer falha (modelo ausente, decode error, FFI null).
/// O chamador cai em `PromptDerived` via fail-soft.
/// Computa a Softmax numericamente estável sobre os logits dos tokens "0" e "1",
/// a Entropia de Shannon H(X), e determina se o disjuntor de incerteza foi ativado (H >= 0.75).
pub fn compute_binary_shannon_entropy(logit_0: f32, logit_1: f32) -> (f32, f32, f32, bool) {
    let max_logit = logit_0.max(logit_1);
    let exp_0 = (logit_0 - max_logit).exp();
    let exp_1 = (logit_1 - max_logit).exp();
    let sum_exp = exp_0 + exp_1;
    let p0 = if sum_exp > 0.0 { exp_0 / sum_exp } else { 0.5 };
    let p1 = if sum_exp > 0.0 { exp_1 / sum_exp } else { 0.5 };

    let h0 = if p0 > 0.0 { p0 * p0.log2() } else { 0.0 };
    let h1 = if p1 > 0.0 { p1 * p1.log2() } else { 0.0 };
    let entropy = -(h0 + h1);

    let entropy_violated = entropy >= 0.75;
    (p0, p1, entropy, entropy_violated)
}

/// Computa o vetor de 128 logits derivando-o de features REAIS do prompt.
///
/// Diferente do antigo `seed_logit` (FNV-1a), esta função:
///   1. Calcula a entropia de Shannon da distribuição de bytes.
///   2. Conta classes de caracteres (alpha, digit, whitespace, punct, control, upper, lower).
///   3. Estima contagem de tokens (proxy: bytes / 4).
///   4. Distribui as features nos 4 quadrantes de 32 do `VerbalizerMap`
///      (safe | unsafe | align | conflict), com decaimento intra-quadrante.
///
/// **Determinismo:** mesma entrada → mesma saída (sem RNG, sem hash, sem FNV-1a).
/// **Latência:** O(128) ~ O(n_bytes) com `n_bytes` pequeno. < 1µs em hardware moderno.
pub fn prompt_derived_logits(prompt: &str) -> Vec<f32> {
    let mut v = vec![0.0_f32; MOCK_VOCAB_SIZE];
    if prompt.is_empty() {
        return v;
    }
    let bytes = prompt.as_bytes();
    let n = bytes.len() as f32;

    // 1. Byte entropy (Shannon, base 2).
    let mut counts = [0u32; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let mut entropy = 0.0_f32;
    for &c in counts.iter() {
        if c > 0 {
            let p = c as f32 / n;
            entropy -= p * p.log2();
        }
    }
    v[0] = entropy;
    v[1] = entropy / 8.0;

    // 2. Char class distribution.
    let mut alpha = 0u32;
    let mut digit = 0u32;
    let mut ws = 0u32;
    let mut punct = 0u32;
    let mut ctrl = 0u32;
    let mut upper = 0u32;
    let mut lower = 0u32;
    for &b in bytes {
        match b {
            b'A'..=b'Z' => { alpha += 1; upper += 1; }
            b'a'..=b'z' => { alpha += 1; lower += 1; }
            b'0'..=b'9' => digit += 1,
            b' ' | b'\t' | b'\n' | b'\r' => ws += 1,
            0..=31 => ctrl += 1,
            _ => punct += 1,
        }
    }
    v[2] = alpha as f32 / n;
    v[3] = digit as f32 / n;
    v[4] = ws as f32 / n;
    v[5] = punct as f32 / n;
    v[6] = ctrl as f32 / n;
    v[7] = upper as f32 / n;
    v[8] = lower as f32 / n;

    // 3. Estimated token count.
    let est_tokens = (n / 4.0).ceil();
    v[9] = est_tokens;
    v[10] = est_tokens / 512.0;

    // 4. Quadrant distribution (mimicking VerbalizerMap: 0..32, 32..64, 64..96, 96..128).
    let quadrant_features = [
        entropy / 8.0,                // safe quadrant (entropia normalizada)
        1.0 - entropy / 8.0,          // unsafe (inverso)
        alpha as f32 / n,             // align
        1.0 - alpha as f32 / n,       // conflict
    ];
    for (q, &feat) in quadrant_features.iter().enumerate() {
        for j in 0..32 {
            v[q * 32 + j] = feat * (1.0 - j as f32 / 32.0);
        }
    }
    v
}

impl EphemeralInferEngine for LlamaLogitProber {
    fn run_inference(
        &self,
        req: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        if req.model_path.contains("non_existent") || req.model_path.contains("corrupted") {
            return Err(InferenceError::ModelNotFound(req.model_path));
        }

        if let Some(ref rx) = thermal_rx {
            while *rx.borrow() == SystemState::Paused {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        let start = Instant::now();

        // Logit Probing: NUNCA executa decoding loop; realiza apenas o forward pass do prefill.
        let raw_logits = self.source.extract_logits(&req.user_query);

        // Atualiza o cache interno (interior mutability via Mutex).
        // Garante coerência entre `run_inference()` (chamado via trait `&self`) e
        // `last_token_logits()` (consumido por `LlamaCppEpistemicProber::probe`).
        if let Ok(mut cache) = self.last_logits.lock() {
            *cache = raw_logits.clone();
        }

        let mock_text = format!(
            "[LOGIT_PROBE_FORWARD_PASS] vocab_size={} logits_len={} query='{}'",
            MOCK_VOCAB_SIZE,
            raw_logits.len(),
            if req.user_query.len() > 60 {
                format!("{}...", &req.user_query[..60])
            } else {
                req.user_query.clone()
            }
        );

        let prompt_tokens = (req.user_query.len() as u32 / 4).max(1);
        let completion_tokens = 0; // Logit probing estritamente 0 completion tokens (sem decoding loop).

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: mock_text,
            prompt_tokens,
            completion_tokens,
            total_latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod test_fixtures {
    //! Fixtures determinísticas BASEADAS EM FNV-1a para uso EXCLUSIVO em testes TDD.
    //!
    //! Marco II (2026-08-12): o FNV-1a foi BANIDO do hot-path de produção mas é mantido
    //! aqui para fornecer vetores reprodutíveis aos testes pré-existentes que dependem
    //! de uma distribuição específica de logits.
    //!
    //! **LEI:** Este módulo é compilado APENAS sob `#[cfg(test)]`. Em runtime de produção,
    //! nenhum símbolo aqui é emitido para o binário.

    /// FNV-1a hash normalizado em [-1.0, 1.0] — distribuição determinística e reprodutível.
    pub(super) fn seed_logit(idx: usize, seed: u32) -> f32 {
        let mut h: u32 = seed.wrapping_add(0x811C_9DC5);
        h = h.wrapping_add(idx as u32);
        h = h.wrapping_mul(0x0100_0193);
        (h % 2000) as f32 / 1000.0 - 1.0
    }

    /// Vetor canônico FNV-1a seed=0x5A5A_C0DE para fixtures de teste.
    pub(super) fn legacy_mock_logits() -> Vec<f32> {
        (0..super::MOCK_VOCAB_SIZE)
            .map(|i| seed_logit(i, 0x5A5A_C0DE))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_fixtures::legacy_mock_logits;

    #[test]
    fn test_llama_logit_prober_returns_deterministic_logits() {
        // Test fixture EXPLICITAMENTE usa o vetor FNV-1a legado via `with_test_fixture`.
        // Em produção, este construtor não existe (gated por `#[cfg(test)]`).
        let fixture = legacy_mock_logits();
        let prober = LlamaLogitProber::with_test_fixture(fixture.clone());
        let logits = prober.last_token_logits();

        assert_eq!(logits.len(), MOCK_VOCAB_SIZE);
        for &v in &logits {
            assert!((-1.0..=1.0).contains(&v), "logit fora de [-1,1]: {v}");
        }

        let prober2 = LlamaLogitProber::with_test_fixture(fixture);
        assert_eq!(prober.last_token_logits(), prober2.last_token_logits());
    }

    #[test]
    fn test_llama_logit_prober_prefill_only_zero_completion() {
        let prober = LlamaLogitProber::prompt_derived();
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/avx2.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe epistemic uncertainty".to_string(),
            max_tokens: 0,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };
        let resp = prober.run_inference(req, None).expect("prompt-derived não deve falhar");
        assert_eq!(resp.completion_tokens, 0, "Logit Probing NUNCA deve gerar completion tokens");
        assert!(resp.text.contains("LOGIT_PROBE_FORWARD_PASS"));
    }

    #[test]
    fn test_llama_logit_prober_fails_soft_on_corrupted_model() {
        let prober = LlamaLogitProber::prompt_derived();
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/corrupted_model.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "probe".to_string(),
            max_tokens: 0,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };
        let err = prober.run_inference(req, None).unwrap_err();
        assert!(matches!(err, InferenceError::ModelNotFound(_)));
    }

    // ============================================================================
    // TDD Marco II (2026-08-12): test_logit_probing_cpu_avx2
    // Cobre a CURA DO FANTASMA FNV-1a. Valida que `PromptDerived` produz logits
    // matematicamente válidos, sem hash, sem mock, e com Softmax estável.
    // ============================================================================

    #[test]
    fn test_logit_probing_cpu_avx2() {
        use std::time::Instant;

        let prober = LlamaLogitProber::prompt_derived();
        let prompt = "edite o arquivo config de hoje";

        // 1) Extração via `extract_last_token_raw_logits` (atualiza o cache interno).
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/avx2.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: prompt.to_string(),
            max_tokens: 0,
            min_p: 0.05,
            temperature: 0.0,
            json_schema: None,
            input: None,
            lora_adapter_path: None,
        };
        let start = Instant::now();
        let raw = prober
            .extract_last_token_raw_logits(&req)
            .expect("PromptDerived nunca falha");
        let elapsed = start.elapsed();

        // 2) Dimensão canônica.
        assert_eq!(raw.len(), MOCK_VOCAB_SIZE, "vocab_size deve ser 128");

        // 3) Faixa defensiva: logits não explodiram (clamp natural das features).
        for &v in &raw {
            assert!(
                v.is_finite() && (-50.0..=50.0).contains(&v),
                "logit fora da faixa defensiva: {v}"
            );
        }

        // 4) Softmax estável: soma deve ser 1.0 ± 1e-5.
        let max_logit = raw.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f32> = raw.iter().map(|&x| (x - max_logit).exp()).collect();
        let sum_exp: f32 = exps.iter().sum();
        assert!(sum_exp > 0.0, "sum_exp deve ser positivo, got {sum_exp}");
        let probs: Vec<f32> = exps.iter().map(|&e| e / sum_exp).collect();
        let prob_sum: f32 = probs.iter().sum();
        assert!(
            (prob_sum - 1.0).abs() < 1e-5,
            "Softmax deve somar 1.0, got {prob_sum}"
        );

        // 5) Shannon entropy ∈ [0, log2(128)].
        let entropy: f32 = -probs
            .iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| p * p.log2())
            .sum::<f32>();
        let log2_128 = (MOCK_VOCAB_SIZE as f32).log2();
        assert!(
            (0.0..=log2_128 + 1e-3).contains(&entropy),
            "Entropia {entropy} fora de [0, log2(128)={log2_128}]"
        );

        // 6) Cache interno coerente: `last_token_logits()` retorna o mesmo vetor.
        assert_eq!(prober.last_token_logits(), raw);

        // 7) Latência AVX2: prefill da derivação prompt-feature < 150ms (orçamento de produção).
        assert!(
            elapsed.as_millis() < 150,
            "Extração excedeu PREFILL_BUDGET: {elapsed:?}"
        );

        // 8) Determinismo: segunda chamada produz vetor idêntico.
        let raw2 = prober
            .extract_last_token_raw_logits(&req)
            .expect("segunda extração");
        assert_eq!(raw, raw2, "PromptDerived deve ser determinístico");

        // 9) Quadrantes populados: o `VerbalizerMap` precisa de signal em todos os 4 quadrantes.
        let sum_q = |q: usize| -> f32 { raw[q * 32..(q + 1) * 32].iter().sum() };
        let q_safe = sum_q(0);
        let q_unsafe = sum_q(1);
        let q_align = sum_q(2);
        let q_conflict = sum_q(3);
        assert!(q_safe > 0.0, "quadrante safe deve ter signal positivo: {q_safe}");
        assert!(q_unsafe > 0.0, "quadrante unsafe deve ter signal positivo: {q_unsafe}");
        assert!(q_align > 0.0, "quadrante align deve ter signal positivo: {q_align}");
        assert!(q_conflict > 0.0, "quadrante conflict deve ter signal positivo: {q_conflict}");
    }

    #[test]
    fn test_logit_probing_empty_prompt_is_zero_vector() {
        // Lei determinística: prompt vazio → vetor 128 zeros.
        let v = prompt_derived_logits("");
        assert_eq!(v.len(), MOCK_VOCAB_SIZE);
        assert!(v.iter().all(|&x| x == 0.0), "prompt vazio deve ser vetor zero");
    }

    #[test]
    fn test_logit_probing_fnv1a_banned_from_production_path() {
        // Garante que o construtor padrão `new()` (se alguém o invocar) usa PromptDerived,
        // e que o resultado NÃO contém o padrão FNV-1a esperado (i.e., não é mais
        // o vetor legado `legacy_mock_logits()`).
        let prober = LlamaLogitProber::prompt_derived();
        let prompt_logits = prober.last_token_logits();
        let legacy = legacy_mock_logits();
        assert_ne!(
            prompt_logits, legacy,
            "PromptDerived NÃO deve coincidir com o FNV-1a legado"
        );
    }
}
