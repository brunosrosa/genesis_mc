// SOULS MC — Marco V: Blast Radius Bridge.
//
// Re-emite o `ImpactReport` do `cognition::ast::repo_impact` como evento
// de controle Tauri (`app.emit("blast_radius_pending", report)`).
//
// ## Por que evento (e não Channel)?
//
// O `ImpactReport` é um payload **discreto e raro** (≤ algumas unidades
// por sessão), não um stream contínuo. Não viola ADR-003 §37-38 porque
// o JSON aqui é a "alfândega externa" (control plane) — não é Data
// Plane. O Data Plane estritamente binário fica reservado para o stream
// de telemetria do watchdog (`watchdog_ipc.rs`).
//
// ## Restrições
//
// - O `ImpactReport` deve ser ≤ 50KB (caso contrário abortar com erro —
//   não serializar payloads gigantes por evento).
// - Logs em `stderr` (NUNCA `stdout`, conforme ADR-003 §32-36).

use serde::Serialize;
use tauri::Emitter;

/// Tamanho máximo aceito do `ImpactReport` serializado (50KB).
/// Acima disso, abortamos com erro explícito em vez de entupir o
/// barramento de eventos com payloads absurdos.
const MAX_REPORT_BYTES: usize = 50 * 1024;

/// Serializa o `ImpactReport` para `serde_json::Value` (tamanho controlado)
/// e o emite como evento `blast_radius_pending` para a Webview.
///
/// Retorna `Ok(())` em caso de sucesso, ou `Err(String)` com a razão.
///
/// # Erros
///
/// - `PayloadTooLarge` se a serialização exceder `MAX_REPORT_BYTES`.
/// - `EmitFailed` se `app.emit` falhar (janela fechada / runtime down).
pub fn emit_blast_pending<S: Serialize>(
    app: &tauri::AppHandle,
    report: &S,
) -> Result<(), String> {
    // 1) Serializa para JSON (≤ 50KB). Usa `to_vec` para medir bytes brutos
    //    ANTES de alocar a `Value` da heap (early bail-out).
    let bytes = serde_json::to_vec(report).map_err(|e| {
        eprintln!("[blast_bridge] serialização falhou: {e}");
        format!("PayloadSerialize: {e}")
    })?;

    if bytes.len() > MAX_REPORT_BYTES {
        let err = format!(
            "PayloadTooLarge: {} bytes (max {} bytes)",
            bytes.len(),
            MAX_REPORT_BYTES
        );
        eprintln!("[blast_bridge] {err}");
        return Err(err);
    }

    // 2) Emite como evento. O `emit` é non-blocking do lado Tauri.
    app.emit("blast_radius_pending", report).map_err(|e| {
        eprintln!("[blast_bridge] emit falhou: {e}");
        format!("EmitFailed: {e}")
    })?;

    eprintln!(
        "[blast_bridge] blast_radius_pending emitido ({} bytes)",
        bytes.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct MockReport {
        target: String,
        affected_files: Vec<String>,
    }

    /// TDD: o limite `MAX_REPORT_BYTES` deve ser ≥ 1KB (para acomodar
    /// relatórios reais) e ≤ 1MB (para respeitar ADR-003 §37).
    #[test]
    fn max_report_bytes_is_within_safe_envelope() {
        assert!(MAX_REPORT_BYTES >= 1024);
        assert!(MAX_REPORT_BYTES <= 1024 * 1024);
    }

    /// TDD: a serialização de um relatório pequeno deve caber no limite.
    /// Apenas valida a aritmética — o teste de `emit` em si requer
    /// um `AppHandle` mock (vide `tauri::test::mock_app` em runtime).
    #[test]
    fn mock_report_serializes_under_limit() {
        let r = MockReport {
            target: "src/lib.rs".to_string(),
            affected_files: (0..10).map(|i| format!("file_{i}.rs")).collect(),
        };
        let bytes = serde_json::to_vec(&r).expect("serializa mock");
        assert!(
            bytes.len() < MAX_REPORT_BYTES,
            "mock serializou em {} bytes (limite {})",
            bytes.len(),
            MAX_REPORT_BYTES
        );
    }
}
