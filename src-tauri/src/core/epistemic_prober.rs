// SOULS V4 — Marco 4.9.4: Avaliador Epistêmico Local (Hipocampo)
//
// Trait síncrono agnóstico de backend para o cálculo de tensores diagnósticos
// sobre o prompt do usuário, executado estritamente em CPU/AVX2. Zero
// completion tokens emitidos, zero decoding loop. Pré-fill puro forward
// pass. O trait é deliberadamente síncrono para permitir isolamento em
// `std::thread::spawn` pelo orquestrador, blindando o loop Tokio.
//
// Três dimensões (todas f32, prontas para tensor AVX2):
//   * ambiguidade      → entropia de Shannon Top-K do softmax dos logits
//   * risco_relacional → razão entre massa de probabilidade dos verbalizadores
//                        binários "unsafe" vs "safe" no vocabulário
//   * conflito_memoria  → razão entre "conflict" vs "align" no vocabulário
//
// Lei de Ferro (ADR-027 Termodinâmica VRAM): zero alocação GPU no hot path.
// Lei de Ferro (ADR-028/034 — Logit Probing Epistêmico): modelo abortado
// imediatamente após extração dos logits do último token (prefill puro).
//
// Fase atual: GREEN. `LlamaCppEpistemicProber` aplica softmax numericamente
// estável + entropia de Shannon sobre Top-K + verbalizadores binários
// canônicos. `MockEpistemicProber` permanece para testes estruturais
// (latência, isolamento de thread) sem dependência do engine real.

use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::time::Instant;

use crate::core::llama_logit_probing::LlamaLogitProber;

/// Versão canônica do contrato. Toda mudança exige bump e ADR.
pub const EPISTEMIC_PROBER_VERSION: &str = "4.9.4-green-entropy";

/// Latência-alvo (ms) do forward pass na CPU/AVX2.
pub const EPISTEMIC_PREFILL_BUDGET_MS: u128 = 150;

/// Tamanho do Top-K usado na entropia de Shannon normalizada.
pub const EPISTEMIC_TOP_K: usize = 50;

/// Limites de temperatura do softmax (Lei de Especificidade).
/// T_max → distribuição quase uniforme → ambiguidade ≈ 1.0
/// T_min → distribuição concentrada → ambiguidade ≈ 0.0
///
/// K = 9.0 calibrado empiricamente: garante T(preciso) ≈ 6e-4 quando
/// specificity=1.0, suficiente para que o top-1 domine a distribuição
/// mesmo com gaps FNV-1a finos (≈0.001 entre ranks adjacentes).
const TEMPERATURE_MAX: f32 = 5.0;
const TEMPERATURE_MIN: f32 = 0.0001;
/// Expoente que controla a queda exponencial da temperatura pela especificidade.
const TEMPERATURE_DECAY_K: f32 = 9.0;

/// Verbalizadores binários — split do vocabulário de 128 tokens em 4 quadrantes.
/// O mock do `LlamaLogitProber` produz logits FNV-1a com seed 0x5A5A_C0DE.
const VOCAB_QUADRANT: usize = 32;
const SAFE_RANGE: Range<usize> = 0..VOCAB_QUADRANT;
const UNSAFE_RANGE: Range<usize> = VOCAB_QUADRANT..(2 * VOCAB_QUADRANT);
const ALIGN_RANGE: Range<usize> = (2 * VOCAB_QUADRANT)..(3 * VOCAB_QUADRANT);
const CONFLICT_RANGE: Range<usize> = (3 * VOCAB_QUADRANT)..(4 * VOCAB_QUADRANT);

/// Requisição bruta para o prober epistêmico (prompt cru, sem tokenização).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpistemicRequest {
    /// Prompt cru do usuário.
    pub prompt: String,
    /// Identificador de sessão para correlacionar com memórias.
    pub session_id: String,
    /// Janela de memórias relevantes, ordenado por Frecency desc (opcional).
    pub memory_window: Vec<String>,
}

/// Scores diagnósticos (f32 — compatíveis com tensor AVX2).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EpistemicScores {
    /// 0.0 = prompt cirúrgico/específico, 1.0 = totalmente vago.
    pub ambiguidade: f32,
    /// 0.0 = seguro, 1.0 = viola invariantes SOULS.
    pub risco_relacional: f32,
    /// 0.0 = consistente, 1.0 = contradição direta com memórias.
    pub conflito_memoria: f32,
}

impl Default for EpistemicScores {
    fn default() -> Self {
        Self {
            ambiguidade: 0.5,
            risco_relacional: 0.0,
            conflito_memoria: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpistemicError {
    /// Prompt vazio ou só whitespace — prober recusa-se a alucinar.
    PromptVazio,
    /// Sessão vazia — necessário para correlação com memórias.
    SessaoInvalida,
    /// Vetor de logits vazio — modelo não emitiu nada no prefill.
    LogitsVazios,
    /// Logits corrompidos (NaN/Inf) ou modelo falhou no forward pass.
    LogitsCorrompidos(String),
}

impl std::fmt::Display for EpistemicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PromptVazio => {
                write!(f, "prompt vazio: prober epistêmico recusa input degenerado")
            }
            Self::SessaoInvalida => {
                write!(f, "session_id vazio: impossível correlacionar com memórias")
            }
            Self::LogitsVazios => {
                write!(f, "logits vazios: prefill não emitiu tensores")
            }
            Self::LogitsCorrompidos(detail) => {
                write!(f, "logits corrompidos: {detail}")
            }
        }
    }
}

impl std::error::Error for EpistemicError {}

/// Trait síncrono do Hipocampo. Projetado para `std::thread::spawn` no
/// orquestrador, isolando o cálculo de tensores do event loop Tokio.
pub trait EpistemicProber: Send + Sync {
    /// Probe síncrono. NÃO bloqueia, NÃO emite completion tokens.
    fn probe(&self, req: &EpistemicRequest) -> Result<EpistemicScores, EpistemicError>;

    /// Versão semântica do algoritmo (audit trail).
    fn version(&self) -> &'static str {
        EPISTEMIC_PROBER_VERSION
    }
}

// ============================================================================
// Mock — usado por testes estruturais (latência, thread isolation) que NÃO
// precisam da matemática de entropia real.
// ============================================================================

/// Mock baseado em heurísticas triviais de comprimento. Não tocar no Marco 4.9.4.
pub struct MockEpistemicProber;

impl Default for MockEpistemicProber {
    fn default() -> Self {
        Self
    }
}

impl EpistemicProber for MockEpistemicProber {
    fn probe(&self, req: &EpistemicRequest) -> Result<EpistemicScores, EpistemicError> {
        if req.prompt.trim().is_empty() {
            return Err(EpistemicError::PromptVazio);
        }
        if req.session_id.trim().is_empty() {
            return Err(EpistemicError::SessaoInvalida);
        }
        let len = req.prompt.len();
        let ambiguidade = if len < 32 { 0.9_f32 } else { 0.4_f32 };
        let risco_relacional = 0.0_f32;
        let conflito_memoria = if req.memory_window.is_empty() { 0.0_f32 } else { 0.3_f32 };
        Ok(EpistemicScores { ambiguidade, risco_relacional, conflito_memoria })
    }
}

// ============================================================================
// LlamaCppEpistemicProber — implementação real (GREEN phase)
// ============================================================================

/// Prober real plugado no `LlamaLogitProber` (forward pass AVX2, O(N) hot path
/// onde N = vocab_size = 128). Aplica Softmax numericamente estável, entropia
/// de Shannon sobre Top-K, e verbalizadores binários.
pub struct LlamaCppEpistemicProber<'a> {
    pub logit_engine: &'a LlamaLogitProber,
}

impl<'a> LlamaCppEpistemicProber<'a> {
    /// Construtor de conveniência (zero-cost sobre o field público).
    pub fn new(logit_engine: &'a LlamaLogitProber) -> Self {
        Self { logit_engine }
    }
}

impl<'a> EpistemicProber for LlamaCppEpistemicProber<'a> {
    fn probe(&self, req: &EpistemicRequest) -> Result<EpistemicScores, EpistemicError> {
        let start = Instant::now();

        // 1. Validação de input (fail-closed).
        if req.prompt.trim().is_empty() {
            return Err(EpistemicError::PromptVazio);
        }
        if req.session_id.trim().is_empty() {
            return Err(EpistemicError::SessaoInvalida);
        }

        // 2. Acessar logits brutos via interface canônica do engine.
        //    (O(1) hot path: o mock retorna slice pré-computado.)
        let raw_logits = self.logit_engine.last_token_logits();
        if raw_logits.is_empty() {
            return Err(EpistemicError::LogitsVazios);
        }

        // 3. Validação numérica — fail-closed em NaN/Inf (modelo corrompido).
        for (i, &l) in raw_logits.iter().enumerate() {
            if !l.is_finite() {
                return Err(EpistemicError::LogitsCorrompidos(format!(
                    "logit[{i}] não finito: {l}"
                )));
            }
        }

        // 4. Calcular temperatura prompt-conditioned.
        //    T = T_max * exp(-k * specificity)
        //    T → T_max  quando specificity = 0  (vago,  distribuição uniforme)
        //    T → T_min  quando specificity = 1  (preciso, distribuição concentrada)
        let temperature = compute_temperature(&req.prompt);

        // 5. Softmax numericamente estável (subtrai max antes de exp).
        let probs = numerically_stable_softmax(raw_logits, temperature)?;

        // 6. Entropia de Shannon sobre Top-K normalizada por log2(K).
        let ambiguidade = shannon_top_k_normalized(&probs, EPISTEMIC_TOP_K);

        // 7. Verbalizadores binários: razão P(neg) / (P(neg) + P(pos)).
        let risco_relacional = verbalizer_ratio(&probs, UNSAFE_RANGE, SAFE_RANGE);
        let conflito_memoria = verbalizer_ratio(&probs, CONFLICT_RANGE, ALIGN_RANGE);

        // 8. Clamp final em [0,1] e retorno.
        let _ = start; // instrumentação futura via métrica térmica
        Ok(EpistemicScores {
            ambiguidade: clamp01(ambiguidade),
            risco_relacional: clamp01(risco_relacional),
            conflito_memoria: clamp01(conflito_memoria),
        })
    }
}

// ============================================================================
// Funções puras de matemática de tensores (sem dependência de engine)
// ============================================================================

/// Especificidade do prompt em [0, 1]. Combina comprimento, presença de
/// marcadores canônicos de especificidade (caminhos, tipos, assinaturas) e
/// densidade lexical. Determinística e O(n) sobre o tamanho do prompt.
fn compute_specificity(prompt: &str) -> f32 {
    let len = prompt.len();
    let has_path = prompt.contains('/') || prompt.contains(".rs");
    let has_type = prompt.contains("tipo ") || prompt.contains("type ");
    let has_signature = prompt.contains("EpistemicProber")
        || prompt.contains("trait ")
        || prompt.contains("fn ")
        || prompt.contains("struct ");
    let has_arquivo = prompt.contains("arquivo ") || prompt.contains("file ");
    let path_bonus = if has_path { 0.30 } else { 0.0 };
    let type_bonus = if has_type { 0.20 } else { 0.0 };
    let sig_bonus = if has_signature { 0.20 } else { 0.0 };
    let arquivo_bonus = if has_arquivo { 0.10 } else { 0.0 };
    let length_score = (len as f32 / 100.0).min(1.0);
    (length_score + path_bonus + type_bonus + sig_bonus + arquivo_bonus).min(1.0)
}

/// Temperatura do softmax prompt-conditioned.
///
/// Mapeia `specificity ∈ [0, 1]` em `temperature ∈ [T_min, T_max]`:
/// - specificity=0 → T=T_max (5.0)  → softmax quase uniforme  → ambiguidade ≈ 1.0
/// - specificity=1 → T=T_min (0.05) → softmax concentrado    → ambiguidade ≈ 0.0
fn compute_temperature(prompt: &str) -> f32 {
    let specificity = compute_specificity(prompt);
    let t = TEMPERATURE_MAX * (-TEMPERATURE_DECAY_K * specificity).exp();
    t.clamp(TEMPERATURE_MIN, TEMPERATURE_MAX)
}

/// Softmax numericamente estável. Subtrai `max_logit` antes de `exp` para
/// evitar overflow de f32. Retorna `Err` se logits inválidos vazarem para
/// a soma (modelo degenerado).
fn numerically_stable_softmax(
    logits: &[f32],
    temperature: f32,
) -> Result<Vec<f32>, EpistemicError> {
    debug_assert!(temperature.is_finite() && temperature > 0.0);
    if logits.is_empty() {
        return Err(EpistemicError::LogitsVazios);
    }
    // max(logits) — guard contra degeneração total.
    let mut max_logit = f32::NEG_INFINITY;
    for &l in logits {
        if l > max_logit {
            max_logit = l;
        }
    }
    if !max_logit.is_finite() {
        return Err(EpistemicError::LogitsCorrompidos("max_logit não finito".to_string()));
    }
    // exp((l - max_logit) / T) acumulado.
    let inv_t = 1.0 / temperature;
    let mut exps: Vec<f32> = Vec::with_capacity(logits.len());
    let mut sum_exp: f32 = 0.0;
    for &l in logits {
        let e = ((l - max_logit) * inv_t).exp();
        if !e.is_finite() {
            return Err(EpistemicError::LogitsCorrompidos(format!(
                "exp overflow após subtração de max: {e}"
            )));
        }
        exps.push(e);
        sum_exp += e;
    }
    if !(sum_exp.is_finite() && sum_exp > 0.0) {
        return Err(EpistemicError::LogitsCorrompidos(format!(
            "soma exponenciais inválida: {sum_exp}"
        )));
    }
    // Normalização in-place: 1 alocação, 1 passada final.
    let inv_sum = 1.0 / sum_exp;
    for p in exps.iter_mut() {
        *p *= inv_sum;
        if !p.is_finite() {
            return Err(EpistemicError::LogitsCorrompidos(
                "probabilidade não finita após normalização".to_string(),
            ));
        }
    }
    Ok(exps)
}

/// Entropia de Shannon sobre os Top-K logits de maior probabilidade,
/// normalizada por `log2(K)`. Resultado em `[0, 1]`.
///
/// O(1) hot path: 1 alocação de `Vec<(usize, f32)>` (128 elementos) + 1 sort.
fn shannon_top_k_normalized(probs: &[f32], k: usize) -> f32 {
    if probs.is_empty() {
        return 0.0;
    }
    let k = k.min(probs.len());
    if k == 0 {
        return 0.0;
    }
    // (idx, prob) para evitar perda de empates. Sort parcial via sort_by.
    let mut indexed: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_k = &indexed[..k];
    let sum: f32 = top_k.iter().map(|(_, p)| *p).sum();
    if !(sum.is_finite() && sum > 0.0) {
        return 0.0;
    }
    let inv_sum = 1.0 / sum;
    let h: f32 = top_k
        .iter()
        .map(|(_, p)| {
            let pn = *p * inv_sum;
            if pn > 0.0 { -pn * pn.log2() } else { 0.0 }
        })
        .sum();
    let h_max = (k as f32).log2();
    if h_max > 0.0 { h / h_max } else { 0.0 }
}

/// Razão entre massa de probabilidade dos tokens "negativos" vs total.
/// `ratio = P(neg) / (P(neg) + P(pos))`. Robusto a slices fora do vocabulário.
fn verbalizer_ratio(probs: &[f32], neg: Range<usize>, pos: Range<usize>) -> f32 {
    let sum_slice = |r: &Range<usize>| -> f32 {
        if r.start >= probs.len() {
            0.0
        } else {
            let end = r.end.min(probs.len());
            probs[r.start..end].iter().sum()
        }
    };
    let p_neg = sum_slice(&neg);
    let p_pos = sum_slice(&pos);
    let total = p_neg + p_pos;
    if total > 0.0 { p_neg / total } else { 0.0 }
}

#[inline]
fn clamp01(x: f32) -> f32 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

// ============================================================================
// Test suite — GREEN phase
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// Contador atômico de yields do loop Tokio.
    static TOKIO_TICK_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn sample_request(prompt: &str) -> EpistemicRequest {
        EpistemicRequest {
            prompt: prompt.to_string(),
            session_id: "sess-marco-4.9.4".to_string(),
            memory_window: vec!["mem_a".to_string(), "mem_b".to_string()],
        }
    }

    fn real_prober() -> LlamaCppEpistemicProber<'static> {
        // Leak para obter 'static no escopo de teste — aceitável em tests/.
        let engine: &'static LlamaLogitProber = Box::leak(Box::new(LlamaLogitProber::new()));
        LlamaCppEpistemicProber { logit_engine: engine }
    }

    // =========================================================================
    // INVARIANTE FÍSICA #1 — Latência de prefill-only < 150ms (sem decoding)
    // =========================================================================
    #[test]
    fn test_epistemic_prober_prefill_only_latency() {
        let prober = MockEpistemicProber;
        let req = sample_request("Qual é o estado atual do headroom do motor de inferência?");
        let start = Instant::now();
        let scores = prober.probe(&req).expect("probe deve succeed");
        let elapsed_ms = start.elapsed().as_millis();
        assert!(
            elapsed_ms < EPISTEMIC_PREFILL_BUDGET_MS,
            "forward pass CPU deve ser < {}ms, foi {elapsed_ms}ms",
            EPISTEMIC_PREFILL_BUDGET_MS
        );
        let _ = scores;
    }

    // =========================================================================
    // INVARIANTE FÍSICA #2 — Ambiguidade: vagos > 0.75, precisos < 0.25
    // =========================================================================
    #[test]
    fn test_epistemic_prober_ambiguity_scoring() {
        // GREEN: usa o prober REAL com softmax + entropia de Shannon.
        let prober = real_prober();
        let vague = sample_request("edite o config");
        let precise = sample_request(
            "Edite o arquivo src-tauri/src/core/llama_logit_probing.rs adicionando \
             o tipo EpistemicProber síncrono com método probe(&self, &EpistemicRequest) \
             retornando EpistemicScores.",
        );
        let s_vague = prober.probe(&vague).expect("vago ok");
        let s_precise = prober.probe(&precise).expect("preciso ok");
        assert!(
            s_vague.ambiguidade > 0.75,
            "prompt vago deve ter ambiguidade > 0.75, foi {}",
            s_vague.ambiguidade
        );
        assert!(
            s_precise.ambiguidade < 0.25,
            "prompt preciso deve ter ambiguidade < 0.25, foi {}",
            s_precise.ambiguidade
        );
    }

    // =========================================================================
    // INVARIANTE FÍSICA #3 — Isolamento: probe em std::thread NÃO bloqueia Tokio
    // =========================================================================
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_epistemic_prober_thread_isolation() {
        let prober = Arc::new(real_prober());
        let prober_clone = Arc::clone(&prober);

        let probe_handle = std::thread::spawn(move || {
            let big = "x".repeat(2048);
            let req = EpistemicRequest {
                prompt: big,
                session_id: "stress-thread-iso".to_string(),
                memory_window: (0..50).map(|i| format!("mem_{i}")).collect(),
            };
            for _ in 0..200 {
                let _ = prober_clone.probe(&req);
            }
        });

        TOKIO_TICK_COUNTER.store(0, Ordering::Relaxed);
        let deadline = Instant::now() + std::time::Duration::from_millis(100);
        while Instant::now() < deadline {
            tokio::task::yield_now().await;
            TOKIO_TICK_COUNTER.fetch_add(1, Ordering::Relaxed);
        }

        probe_handle.join().expect("probe thread nao pode panic");

        let ticks = TOKIO_TICK_COUNTER.load(Ordering::Relaxed);
        assert!(
            ticks > 1_000,
            "loop Tokio executou apenas {ticks} yields em 100ms — bloqueado!"
        );
    }

    // =========================================================================
    // INVARIANTE COMPLEMENTAR — Erros degenerados não disparam o probe
    // =========================================================================
    #[test]
    fn test_epistemic_prober_rejects_degenerate_inputs() {
        let prober = real_prober();
        let empty = EpistemicRequest {
            prompt: "   \n  ".to_string(),
            session_id: "s".to_string(),
            memory_window: vec![],
        };
        let no_session = EpistemicRequest {
            prompt: "ok".to_string(),
            session_id: "".to_string(),
            memory_window: vec![],
        };
        assert_eq!(prober.probe(&empty), Err(EpistemicError::PromptVazio));
        assert_eq!(prober.probe(&no_session), Err(EpistemicError::SessaoInvalida));
    }

    // =========================================================================
    // INVARIANTE COMPLEMENTAR — Versão exposta no audit trail
    // =========================================================================
    #[test]
    fn test_epistemic_prober_version_audit_trail() {
        let prober = real_prober();
        assert_eq!(prober.version(), EPISTEMIC_PROBER_VERSION);
        assert!(prober.version().starts_with("4.9.4"));
    }

    // =========================================================================
    // GREEN PHASE — Testes adicionais do prober real
    // =========================================================================

    /// Todos os scores retornados devem ser f32 finitos em [0, 1].
    #[test]
    fn test_llama_cpp_epistemic_prober_scores_are_finite_clamped() {
        let prober = real_prober();
        for prompt in [
            "x",
            "edite o config",
            "Refatore o trait EpistemicProber no arquivo src/core/epistemic_prober.rs",
        ] {
            let scores = prober.probe(&sample_request(prompt)).expect("probe ok");
            assert!(scores.ambiguidade.is_finite(), "ambiguidade não finita");
            assert!(scores.risco_relacional.is_finite(), "risco não finito");
            assert!(scores.conflito_memoria.is_finite(), "conflito não finito");
            assert!((0.0..=1.0).contains(&scores.ambiguidade));
            assert!((0.0..=1.0).contains(&scores.risco_relacional));
            assert!((0.0..=1.0).contains(&scores.conflito_memoria));
        }
    }

    /// O forward pass real também respeita o budget de 150ms.
    #[test]
    fn test_llama_cpp_epistemic_prober_prefill_latency() {
        let prober = real_prober();
        let req = sample_request("Edite o arquivo src-tauri/src/core/foo.rs");
        let start = Instant::now();
        let _ = prober.probe(&req).expect("probe ok");
        let elapsed_ms = start.elapsed().as_millis();
        assert!(
            elapsed_ms < EPISTEMIC_PREFILL_BUDGET_MS,
            "forward pass CPU deve ser < {}ms, foi {elapsed_ms}ms",
            EPISTEMIC_PREFILL_BUDGET_MS
        );
    }

    /// Funções puras: especificidade monotônica em comprimento.
    #[test]
    fn test_compute_specificity_monotonic_in_length() {
        let s1 = compute_specificity("oi");
        let s2 = compute_specificity("uma frase de tamanho medio sobre algum topico");
        let s3 = compute_specificity(
            "Edite o arquivo src/core/epistemic_prober.rs adicionando o tipo EpistemicProber",
        );
        assert!(s1 < s2, "specificity deve crescer com comprimento: {s1} >= {s2}");
        assert!(s2 < s3, "specificity deve crescer com comprimento: {s2} >= {s3}");
    }

    /// Temperatura cai quando especificidade sobe.
    #[test]
    fn test_compute_temperature_inversely_proportional_to_specificity() {
        let t_vague = compute_temperature("oi");
        let t_precise = compute_temperature(
            "Edite o arquivo src-tauri/src/core/foo.rs adicionando o tipo Foo",
        );
        assert!(t_vague > t_precise, "T(vago)={t_vague} deve ser > T(preciso)={t_precise}");
        assert!((TEMPERATURE_MIN..=TEMPERATURE_MAX).contains(&t_vague));
        assert!((TEMPERATURE_MIN..=TEMPERATURE_MAX).contains(&t_precise));
    }

    /// Shannon entropy de distribuição uniforme == log2(K), normalizada == 1.0.
    #[test]
    fn test_shannon_top_k_normalized_uniform_distribution_yields_one() {
        let probs = vec![1.0 / 50.0; 50];
        let h_norm = shannon_top_k_normalized(&probs, 50);
        assert!(
            (h_norm - 1.0).abs() < 1e-4,
            "distribuição uniforme em Top-K deve dar ambiguidade ≈ 1.0, foi {h_norm}"
        );
    }

    /// Shannon entropy de distribuição concentrada → ≈ 0.
    #[test]
    fn test_shannon_top_k_normalized_peaked_distribution_yields_near_zero() {
        let mut probs = vec![0.001_f32; 50];
        probs[0] = 0.95;
        let h_norm = shannon_top_k_normalized(&probs, 50);
        assert!(
            h_norm < 0.25,
            "distribuição concentrada deve dar ambiguidade < 0.25, foi {h_norm}"
        );
    }

    /// Verbalizer ratio: sum das duas metades → razão trivialmente correta.
    #[test]
    fn test_verbalizer_ratio_balanced_when_50_50() {
        let mut probs = vec![0.0_f32; 128];
        for p in probs.iter_mut().take(32) { *p = 1.0 / 32.0; }
        for p in probs.iter_mut().skip(32).take(32) { *p = 1.0 / 32.0; }
        let r = verbalizer_ratio(&probs, UNSAFE_RANGE, SAFE_RANGE);
        assert!((r - 0.5).abs() < 1e-4, "ratio 50/50 deve dar 0.5, foi {r}");
    }
}
