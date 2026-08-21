// SOULS V6 MARCO 5.11.0 — Socratic CLI & CPU Logit Probing Controller (socratic_cli.rs)
// Governança HITL: Interrupção Socrática com Probing de Logits em CPU Host (AVX2) e Gitoxide.
// Conforme ADR-001, ADR-003, ADR-010, ADR-027, ADR-028/034 e Marco V.

use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::core::llama_logit_probing::LlamaCpp4LogitEngine;
use crate::core::socratic_interrupt::{
    generate_two_legged_socratic_question, get_shadow_workspace_diff_at,
};

/// Limiar crítico de Entropia de Shannon (H >= 0.75) para disparo da Interrupção Socrática.
pub const SOCRATIC_ENTROPY_THRESHOLD: f32 = 0.75;

/// Teto máximo de falhas consecutivas de compilação do Ralph Loop (3 tentativas) antes de travar.
pub const RALPH_LOOP_MAX_FAILURES: usize = 3;

/// Computa a entropia de Shannon calibrada sobre uma distribuição binária de probabilidades (p0, p1):
/// H = - (p0 * log2(p0) + p1 * log2(p1))
/// Retorna valor normalizado no intervalo [0.0, 1.0].
pub fn compute_shannon_entropy_binary(p0: f32, p1: f32) -> f32 {
    let eps = 1e-12f32;
    let p0_clamped = p0.clamp(eps, 1.0 - eps);
    let p1_clamped = p1.clamp(eps, 1.0 - eps);

    let h = -(p0_clamped * p0_clamped.log2() + p1_clamped * p1_clamped.log2());
    if h.is_nan() || h < 0.0 {
        0.0
    } else {
        h.clamp(0.0, 1.0)
    }
}

/// Avalia logits para os tokens de controle verbalizador ("0" e "1") usando o LlamaCpp4LogitEngine na CPU (AVX2).
/// Retorna as probabilidades calibradas (p0, p1) e a Entropia de Shannon calculada.
pub fn probe_verbalizer_binary_entropy(
    logit_engine: &LlamaCpp4LogitEngine,
    prompt: &str,
) -> (f32, f32, f32) {
    let logits = logit_engine.probe_prompt_logits(prompt);

    // Mapeamento canônico dos tokens de controle no vocabulário de 128 tokens
    // Índice 0 = token "0" (unsafe / discordância), Índice 1 = token "1" (safe / concordância)
    let l0 = logits.first().copied().unwrap_or(0.0);
    let l1 = logits.get(1).copied().unwrap_or(0.0);

    // Softmax numericamente estável
    let max_l = l0.max(l1);
    let e0 = (l0 - max_l).exp();
    let e1 = (l1 - max_l).exp();
    let sum_e = e0 + e1;

    let p0 = if sum_e > 0.0 { e0 / sum_e } else { 0.5 };
    let p1 = if sum_e > 0.0 { e1 / sum_e } else { 0.5 };

    let entropy = compute_shannon_entropy_binary(p0, p1);
    (p0, p1, entropy)
}

/// Avalia se a execução deve ser interrompida para inspeção humana (HITL).
pub fn should_trigger_socratic_gate(entropy: f32, consecutive_compiler_failures: usize) -> bool {
    entropy >= SOCRATIC_ENTROPY_THRESHOLD || consecutive_compiler_failures >= RALPH_LOOP_MAX_FAILURES
}

/// Executor completo do fluxo de Interrupção Socrática com streams I/O genéricos.
pub async fn execute_socratic_gate_with_io<R, W>(
    workspace_path: &Path,
    entropy: f32,
    compiler_failures: usize,
    reader: &mut R,
    writer: &mut W,
) -> Result<(), String>
where
    R: AsyncBufReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let diff = get_shadow_workspace_diff_at(workspace_path)?;
    let question = generate_two_legged_socratic_question(&diff);

    let banner = format!(
        "\n=============================================================================\n\
         [INTERRUPÇÃO SOCRÁTICA CLI - GOVERNANÇA HITL SOULS MC]\n\
         =============================================================================\n\
         ⚠️ Gatilho Ativado: Entropia H = {:.4} (Threshold >= {:.2}) | Falhas de Compilação = {} (Max {})\n\
         📄 Blast Radius (Git Diff - Gitoxide):\n{}\n\
         ❓ Pergunta Socrática:\n  {}\n\
         -----------------------------------------------------------------------------\n\
         👉 Digite 'approve' / 'accept' / 'yes' para autorizar ou 'reject' / 'no' / 'abort' para descartar:\n",
        entropy, SOCRATIC_ENTROPY_THRESHOLD, compiler_failures, RALPH_LOOP_MAX_FAILURES, diff, question
    );

    let _ = writer.write_all(banner.as_bytes()).await;
    let _ = writer.flush().await;

    let mut user_input = String::new();
    reader.read_line(&mut user_input).await.map_err(|e| format!("Erro ao ler stdin: {e}"))?;

    let input_clean = user_input.trim().to_lowercase();
    if input_clean == "approve" || input_clean == "accept" || input_clean == "yes" {
        let approval_msg = "[HITL APPROVED] Rebase semântico atômico na main AUTORIZADO.\n";
        let _ = writer.write_all(approval_msg.as_bytes()).await;
        let _ = writer.flush().await;
        Ok(())
    } else if input_clean == "reject" || input_clean == "no" || input_clean == "abort" {
        let reject_msg = "[HITL REJECTED] Descartando Shadow Workspace e abortando alterações.\n";
        let _ = writer.write_all(reject_msg.as_bytes()).await;
        let _ = writer.flush().await;
        Err("HITL Rejection: Shadow workspace descartado pelo operador humano.".to_string())
    } else {
        let invalid_msg = format!("[HITL INVALID] Resposta '{input_clean}' não autorizada. Operação abortada.\n");
        let _ = writer.write_all(invalid_msg.as_bytes()).await;
        let _ = writer.flush().await;
        Err("HITL Invalid Response: Operação abortada por segurança.".to_string())
    }
}
