// SOULS V6 MARCO 5.11.0 — Canal de Interrupção Socrática CLI Híbrido (socratic_interrupt.rs)
// Governança HITL: Intercepta a execução quando a entropia epistêmica ultrapassa H >= 0.75
// ou quando o Ralph Loop atinge 3 falhas de compilação.
//
// MODO STANDALONE (is_terminal == true): exibe diff do gitoxide, pergunta socrática e bloqueia stdin.
// MODO SERVIDOR MCP (is_terminal == false): NUNCA bloqueia stdin; retorna imediatamente erro cognitivo HITL Denied.

use std::io::IsTerminal;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// Extrai o diff das alterações pendentes no Shadow Workspace utilizando `gitoxide` (`gix`).
pub fn get_shadow_workspace_diff() -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Falha ao obter diretório corrente: {e}"))?;
    get_shadow_workspace_diff_at(&cwd)
}

/// Extrai o diff de um diretório específico via `gitoxide` (`gix`).
pub fn get_shadow_workspace_diff_at(workspace_path: &Path) -> Result<String, String> {
    if !workspace_path.exists() {
        return Ok("Shadow Workspace inexistente.".to_string());
    }

    let repo = match gix::discover(workspace_path) {
        Ok(r) => r,
        Err(e) => return Ok(format!("Repositório Git não encontrado em {}: {e}", workspace_path.display())),
    };

    let mut diff_summary = String::new();

    if let Ok(index) = repo.index() {
        for entry in index.entries() {
            let path_str = String::from_utf8_lossy(entry.path(&index));
            diff_summary.push_str(&format!("  modified: {}\n", path_str));
        }
    }

    if diff_summary.is_empty() {
        diff_summary = "  (Nenhuma modificação pendente detectada no Shadow Workspace)\n".to_string();
    }

    Ok(diff_summary)
}

/// Formula uma Pergunta Socrática de Duas Pernas focada em "Como/O que/Para que".
/// LEI DE FERRO: É TERMINANTEMENTE PROIBIDO usar o inquisitório "Por que".
pub fn generate_two_legged_socratic_question(diff: &str) -> String {
    let diff_preview = if diff.len() > 200 {
        format!("{}...", &diff[..200])
    } else {
        diff.trim().to_string()
    };

    format!(
        "O que estas alterações no Blast Radius representam para a integridade do sistema ({diff_preview}), \
        e como pretendemos tratar as potenciais regressões antes de autorizar o rebase semântico na main?"
    )
}

/// Despachador principal da interrupção socrática CLI híbrida.
pub async fn trigger_socratic_cli_interrupt(diff: &str, question: &str) -> Result<(), String> {
    if std::io::stdin().is_terminal() {
        // Modo Terminal Standalone: captura interativa de stdin
        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        trigger_socratic_cli_interrupt_with_io(diff, question, &mut reader, &mut stdout).await
    } else {
        // Modo Servidor MCP: proíbe bloqueio de stdin para não asfixiar o trabrigo JSON-RPC
        Err(format!(
            "Socratic Interrupt: Incerteza epistêmica violada. HITL exigido.\nDiff:\n{}\nQuestion:\n{}",
            diff, question
        ))
    }
}

/// Helper síncrono/assíncrono testável que aceita streams customizados de I/O.
pub async fn trigger_socratic_cli_interrupt_with_io<R, W>(
    diff: &str,
    question: &str,
    reader: &mut R,
    writer: &mut W,
) -> Result<(), String>
where
    R: AsyncBufReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let banner = format!(
        "\n=============================================================================\n\
         [INTERRUPÇÃO SOCRÁTICA CLI - GOVERNANÇA HITL SODA V6]\n\
         =============================================================================\n\
         📄 Blast Radius (Git Diff):\n{}\n\
         ❓ Pergunta Socrática:\n  {}\n\
         -----------------------------------------------------------------------------\n\
         👉 Digite 'approve' / 'accept' / 'yes' para autorizar ou 'reject' / 'no' / 'abort' para descartar:\n",
        diff, question
    );

    // Escreve via stderr
    eprint!("{}", banner);
    let _ = writer.write_all(banner.as_bytes()).await;
    let _ = writer.flush().await;

    let mut user_input = String::new();
    reader.read_line(&mut user_input).await.map_err(|e| format!("Erro ao ler stdin: {e}"))?;

    let input_clean = user_input.trim().to_lowercase();
    if input_clean == "approve" || input_clean == "accept" || input_clean == "yes" {
        eprintln!("[HITL APPROVED] Rebase semântico atômico na main AUTORIZADO.");
        Ok(())
    } else if input_clean == "reject" || input_clean == "no" || input_clean == "abort" {
        eprintln!("[HITL REJECTED] Descartando Shadow Workspace e limpando memória RAM...");
        Err("HITL Rejection: Shadow workspace descartado pelo operador.".to_string())
    } else {
        eprintln!("[HITL INVALID] Resposta '{input_clean}' não reconhecida como aprovação. Operação descartada.");
        Err("HITL Invalid Response: Operação abortada.".to_string())
    }
}
