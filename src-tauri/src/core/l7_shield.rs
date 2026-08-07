// SOULS V4 — Marco 4.10.0 ETAPA 4: L7 Shield (Escudo de Mutação Relacional)
//
// Intercepta chamadas HTTP mutantes (POST/PUT/DELETE/PATCH) que carregam
// prompts vagos ou de alto risco relacional antes de atingirem o upstream
// LLM. Read-only (GET/HEAD/OPTIONS) passa direto.
//
// Lei de Ferro (DIRETRIZ 2 — Marco 4.10.0): o prober epistêmico é
// estritamente síncrono. Para evitar stalls na thread assíncrona do
// proxy, ele roda em uma thread OS dedicada (`std::thread::spawn`)
// comunicando via canal MPSC com `oneshot` de resposta. O proxy `await`
// apenas o `oneshot::Receiver`, sem nunca tocar no tensor.
//
// Decisões possíveis:
//   - `Bypass`           → request forwarded ao upstream sem modificação.
//   - `Intercepted`      → request bloqueado, retorna JSON-RPC -32001.

use tokio::sync::mpsc;
use std::thread;

use serde::{Deserialize, Serialize};

use crate::core::epistemic_prober::{
    EpistemicProber, EpistemicRequest, EpistemicScores, MockEpistemicProber,
};

/// Limiar de risco_relacional acima do qual a chamada é interceptada
/// (alinhado com `disjuntor_ativo` do disjuntor socrático).
pub const SHIELD_RISK_THRESHOLD: f32 = 0.70;

/// Métodos HTTP considerados mutantes (escrita/exclusão).
const MUTATING_METHODS: &[&str] = &["POST", "PUT", "DELETE", "PATCH"];

/// Métodos HTTP considerados read-only (bypass garantido).
const READ_ONLY_METHODS: &[&str] = &["GET", "HEAD", "OPTIONS"];

/// Decisão do shield sobre uma requisição HTTP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShieldDecision {
    /// Request seguro: encaminhar ao upstream sem modificação.
    Bypass { reason: &'static str },
    /// Request mutante com risco elevado: bloquear e devolver -32001.
    Intercepted {
        risco_relacional: f32,
        ambiguidade: f32,
        motivo: String,
    },
}

impl ShieldDecision {
    pub fn is_intercepted(&self) -> bool {
        matches!(self, ShieldDecision::Intercepted { .. })
    }
}

/// Heurística pura: classifica o método HTTP.
pub fn is_mutating_method(method: &str) -> bool {
    let upper = method.to_ascii_uppercase();
    MUTATING_METHODS.contains(&upper.as_str())
}

/// Heurística pura: extrai o prompt do corpo JSON.
/// Aceita `{"prompt":"..."}`, `{"messages":[{"content":"..."}]}` e variantes.
pub fn extract_prompt_from_body(body: &[u8]) -> Option<String> {
    let val: serde_json::Value = serde_json::from_slice(body).ok()?;
    if let Some(p) = val.get("prompt").and_then(|v| v.as_str()) {
        return Some(p.to_string());
    }
    if let Some(msgs) = val.get("messages").and_then(|v| v.as_array()) {
        for msg in msgs.iter().rev() {
            if let Some(c) = msg.get("content").and_then(|v| v.as_str()) {
                return Some(c.to_string());
            }
        }
    }
    if let Some(input) = val.get("input").and_then(|v| v.as_str()) {
        return Some(input.to_string());
    }
    None
}

/// Sessão de identificação para o trace de auditoria do shield.
#[derive(Debug, Clone)]
pub struct ShieldContext {
    pub session_id: String,
    pub method: String,
    pub path: String,
}

impl ShieldContext {
    pub fn new(session_id: impl Into<String>, method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            method: method.into(),
            path: path.into(),
        }
    }
}

/// Função pura (testável, síncrona) que computa a decisão do shield.
///
/// REGRAS (DIRETRIZ 2 — Marco 4.10.0):
///   1. Read-only (GET/HEAD/OPTIONS) → Bypass incondicional (custo zero).
///   2. Mutante + corpo vazio/sem prompt extraível → Bypass
///      (não há o que triar; deixa passar).
///   3. Mutante + prompt extraível → prober síncrono; se
///      `risco_relacional > SHIELD_RISK_THRESHOLD` (0.70) → Intercepted.
pub fn evaluate_shield<P: EpistemicProber>(
    prober: &P,
    ctx: &ShieldContext,
    body: &[u8],
) -> ShieldDecision {
    // Regra 1: read-only bypass.
    let method_upper = ctx.method.to_ascii_uppercase();
    if READ_ONLY_METHODS.contains(&method_upper.as_str()) {
        return ShieldDecision::Bypass {
            reason: "read-only method bypass",
        };
    }

    // Regra 2: mutante sem prompt extraível → bypass.
    if !is_mutating_method(&ctx.method) {
        // Método não-classificado (UNKNOWN): conservadoramente bypass.
        return ShieldDecision::Bypass {
            reason: "non-mutating method bypass",
        };
    }
    let prompt = match extract_prompt_from_body(body) {
        Some(p) if !p.trim().is_empty() => p,
        _ => {
            return ShieldDecision::Bypass {
                reason: "mutating method sem prompt extraível",
            };
        }
    };

    // Regra 3: probe síncrono CPU-bound.
    let req = EpistemicRequest {
        prompt,
        session_id: ctx.session_id.clone(),
        memory_window: vec![],
    };
    let scores: EpistemicScores = match prober.probe(&req) {
        Ok(s) => s,
        Err(e) => {
            // Fail-soft: erro do prober = bypass com warning (não bloqueia).
            tracing::warn!(
                "L7 Shield: prober falhou ({}), bypass por fail-soft",
                e
            );
            return ShieldDecision::Bypass {
                reason: "prober fail-soft bypass",
            };
        }
    };

    if scores.risco_relacional > SHIELD_RISK_THRESHOLD
        || scores.ambiguidade > 0.75
    {
        let motivo = if scores.ambiguidade > 0.75 {
            format!("ambiguidade {:.2} > 0.75", scores.ambiguidade)
        } else {
            format!("risco_relacional {:.2} > {:.2}", scores.risco_relacional, SHIELD_RISK_THRESHOLD)
        };
        ShieldDecision::Intercepted {
            risco_relacional: scores.risco_relacional,
            ambiguidade: scores.ambiguidade,
            motivo,
        }
    } else {
        ShieldDecision::Bypass {
            reason: "prober aprovou: risco abaixo do limiar",
        }
    }
}

// ============================================================================
// Canal assíncrono (produção): MPSC + oneshot para isolar prober síncrono
// ============================================================================

/// Mensagem enviada pelo proxy (Tokio) para a thread dedicada do prober.
pub struct ShieldRequest {
    pub ctx: ShieldContext,
    pub body: Vec<u8>,
    pub reply: tokio::sync::oneshot::Sender<ShieldDecision>,
}

/// Handle do canal L7 Shield. O proxy clona `tx` e usa `submit()` para
/// despachar requisições; `submit()` retorna um `oneshot::Receiver` que
/// pode ser `await`-ado sem bloquear o event loop.
#[derive(Clone)]
pub struct EpistemicShieldChannel {
    tx: mpsc::Sender<ShieldRequest>,
}

impl EpistemicShieldChannel {
    /// Spawna a thread OS dedicada que executa o prober síncrono.
    /// O prober é movido para dentro da thread batizada como 'souls-l7-shield'.
    /// A comunicação é governada por `tokio::sync::mpsc` com limite de 16 slots (backpressure).
    pub fn spawn<P: EpistemicProber + Send + 'static>(prober: P) -> Self {
        let (tx, rx) = mpsc::channel::<ShieldRequest>(16);
        thread::Builder::new()
            .name("souls-l7-shield".to_string())
            .spawn(move || {
                run_shield_loop(prober, rx);
            })
            .expect("falha ao spawnar thread souls-l7-shield");
        Self { tx }
    }

    /// Construtor alternativo com `MockEpistemicProber` (default, sem dependências).
    pub fn spawn_mock() -> Self {
        Self::spawn(MockEpistemicProber)
    }

    /// Despacha uma request ao shield de forma não-bloqueante.
    /// Retorna `oneshot::Receiver<ShieldDecision>` para `await` no Tokio.
    ///
    /// Comportamento fail-soft: se a fila de 16 slots encheu ou a thread morreu,
    /// o `try_send` falha; o `oneshot::Sender` é então dropado e o `Receiver::await`
    /// retorna `Err(_)`, que o handler trata como Bypass fail-soft.
    pub fn submit(&self, ctx: ShieldContext, body: Vec<u8>) -> tokio::sync::oneshot::Receiver<ShieldDecision> {
        let (otx, orx) = tokio::sync::oneshot::channel();
        let msg = ShieldRequest {
            ctx,
            body,
            reply: otx,
        };
        let _ = self.tx.try_send(msg);
        orx
    }
}

fn run_shield_loop<P: EpistemicProber>(prober: P, mut rx: mpsc::Receiver<ShieldRequest>) {
    while let Some(msg) = rx.blocking_recv() {
        let decision = evaluate_shield(&prober, &msg.ctx, &msg.body);
        // Falha no envio = receptor foi dropado (cliente desconectou).
        let _ = msg.reply.send(decision);
    }
}

// ============================================================================
// Helper de produção: serializa `Intercepted` em payload JSON-RPC
// ============================================================================

/// Serializa a decisão `Intercepted` no formato JSON-RPC de erro -32001
/// (HitlDenied) com payload em `data.shield_decision`.
pub fn intercepted_to_jsonrpc(decision: &ShieldDecision) -> serde_json::Value {
    match decision {
        ShieldDecision::Intercepted { risco_relacional, ambiguidade, motivo } => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32001,
                    "message": format!("HitlDenied: L7 Shield interceptou ({motivo})"),
                    "data": {
                        "hitl_required": true,
                        "shield": true,
                        "risco_relacional": risco_relacional,
                        "ambiguidade": ambiguidade,
                        "motivo": motivo,
                    }
                }
            })
        }
        ShieldDecision::Bypass { .. } => {
            // Bypass nunca é serializado como erro.
            serde_json::json!({})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `LlamaCppEpistemicProber` tem lifetime emprestado do engine; a sua
    // cobertura de testes está em `epistemic_prober.rs`. O canal
    // `EpistemicShieldChannel` é testado aqui com `MockEpistemicProber`,
    // que satisfaz `EpistemicProber + Send + 'static`.

    // ========================================================================
    // TDD net-new (Marco 4.10.0 DIRETRIZ 4): 2 testes `test_l7_shield_*`
    // ========================================================================

    /// Spec DIRETRIZ 4: read-only (GET) sempre passa pelo shield, sem
    /// nem tocar no prober. Latência-alvo = O(1) (sem alocação).
    #[test]
    fn test_l7_shield_readonly_method_bypasses_without_probe() {
        // Usa MockEpistemicProber: se o probe fosse invocado, o mock
        // retornaria ambiguidade=0.9 (prompt < 32 chars). Mas como o
        // método é GET, o shield deve bypassar SEM chamar o prober.
        // Verificação: inserimos um body propositalmente "perigoso" que
        // dispararia o disjuntor se fosse processado.
        let prober = MockEpistemicProber;
        let ctx = ShieldContext::new("sess-test-bypass", "GET", "/v1/models");
        let dangerous_body = br#"{"prompt":"x"}"#; // curto → mock daria amb=0.9
        let decision = evaluate_shield(&prober, &ctx, dangerous_body);
        assert!(
            matches!(decision, ShieldDecision::Bypass { .. }),
            "GET deve bypassar incondicionalmente, foi {decision:?}"
        );
    }

    /// Spec DIRETRIZ 4: POST mutante com prompt de alto risco (curto
    /// no mock → risco=0 amb=0.9) deve ser interceptado. Verifica que
    /// o JSON-RPC de erro -32001 carrega o `shield_decision` correto.
    #[test]
    fn test_l7_shield_mutating_intercepts_risk_above_threshold() {
        let prober = MockEpistemicProber;
        let ctx = ShieldContext::new("sess-test-intercept", "POST", "/v1/chat/completions");
        // Mock: prompt < 32 chars → ambiguidade=0.9 > 0.80 → disjuntor.
        let body_vago = br#"{"prompt":"edite o config"}"#;
        let decision = evaluate_shield(&prober, &ctx, body_vago);
        assert!(
            decision.is_intercepted(),
            "POST vago deve ser interceptado, foi {decision:?}"
        );
        // Serialização para JSON-RPC.
        let jsonrpc = intercepted_to_jsonrpc(&decision);
        assert_eq!(jsonrpc["error"]["code"], serde_json::Value::from(-32001));
        assert_eq!(jsonrpc["error"]["data"]["shield"], serde_json::Value::Bool(true));
        assert_eq!(jsonrpc["error"]["data"]["hitl_required"], serde_json::Value::Bool(true));
    }

    // ========================================================================
    // Testes estruturais complementares
    // ========================================================================

    #[test]
    fn test_is_mutating_method_classifies_correctly() {
        // HTTP methods são case-insensitive (RFC 7230); a função normaliza
        // para upper-case internamente.
        for m in &["POST", "PUT", "DELETE", "PATCH", "post", "Post", "pOsT"] {
            assert!(is_mutating_method(m), "{m} deve ser mutante");
        }
        for m in &["GET", "HEAD", "OPTIONS", "get", "Get", "head", "options"] {
            assert!(!is_mutating_method(m), "{m} deve ser read-only");
        }
    }

    #[test]
    fn test_extract_prompt_from_body_variants() {
        // Caso 1: campo top-level `prompt`.
        let body1 = br#"{"prompt":"hello world"}"#;
        assert_eq!(extract_prompt_from_body(body1), Some("hello world".to_string()));
        // Caso 2: array `messages` (formato OpenAI).
        let body2 = br#"{"messages":[{"role":"user","content":"oi"}]}"#;
        assert_eq!(extract_prompt_from_body(body2), Some("oi".to_string()));
        // Caso 3: campo `input` (formato genérico).
        let body3 = br#"{"input":"pergunta"}"#;
        assert_eq!(extract_prompt_from_body(body3), Some("pergunta".to_string()));
        // Caso 4: corpo inválido → None.
        assert_eq!(extract_prompt_from_body(b"not json"), None);
    }

    /// POST com prompt longo (mock: ambiguidade=0.4) deve bypassar.
    /// Cobre o caminho "risco abaixo do limiar".
    #[test]
    fn test_l7_shield_mutating_long_prompt_bypasses() {
        let prober = MockEpistemicProber;
        let ctx = ShieldContext::new("sess-test-long", "POST", "/v1/chat/completions");
        // Prompt > 32 chars: ambiguidade = 0.4 (mock), risco = 0.
        // Mantem ASCII puro para satisfazer o requisito de br#"..."#.
        let body_longo = br#"{"prompt":"Edite o arquivo src-tauri/src/core/l7_shield.rs adicionando testes de integracao"}"#;
        let decision = evaluate_shield(&prober, &ctx, body_longo);
        assert!(
            matches!(decision, ShieldDecision::Bypass { .. }),
            "POST com prompt longo deve bypassar, foi {decision:?}"
        );
    }

    /// PUT/DELETE/PATCH sem body parseável → bypass (fail-soft).
    #[test]
    fn test_l7_shield_mutating_empty_body_bypasses() {
        let prober = MockEpistemicProber;
        let ctx = ShieldContext::new("sess-test-empty", "DELETE", "/v1/resource/123");
        let empty_body = b"";
        let decision = evaluate_shield(&prober, &ctx, empty_body);
        assert!(
            matches!(decision, ShieldDecision::Bypass { .. }),
            "DELETE sem body deve bypassar, foi {decision:?}"
        );
    }

    /// Canal MPSC: despacha via thread dedicada e recebe via oneshot
    /// sem bloquear o runtime do Tokio.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_l7_shield_channel_dispatches_async() {
        let channel = EpistemicShieldChannel::spawn_mock();
        let ctx = ShieldContext::new("sess-async", "POST", "/v1/chat/completions");
        let body = br#"{"prompt":"edite o config"}"#.to_vec();
        let rx = channel.submit(ctx, body);
        let decision = rx.await.expect("oneshot não deve ser dropado");
        assert!(
            decision.is_intercepted(),
            "canal async deve entregar decisão intercepted, foi {decision:?}"
        );
    }

    /// O canal aceita qualquer `EpistemicProber + Send + 'static`.
    /// Aqui validamos que a API funciona com um wrapper que satisfaz
    /// a trait via `MockEpistemicProber` (já coberto em `spawn_mock`).
    /// O prober real (`LlamaCppEpistemicProber`) tem lifetime emprestado
    /// do engine e é exercitado nos testes de `epistemic_prober.rs`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_l7_shield_channel_clone_works() {
        let channel = EpistemicShieldChannel::spawn_mock();
        let channel2 = channel.clone();
        // 2 clones = 2 handles, mesma thread dedicada.
        let ctx = ShieldContext::new("sess-clone", "POST", "/v1/chat/completions");
        let body = br#"{"prompt":"edite o config"}"#.to_vec();
        let decision = channel2.submit(ctx, body).await.expect("oneshot");
        assert!(decision.is_intercepted());
        // Canal original ainda funcional.
        let _ = channel; // silence unused
    }
}
