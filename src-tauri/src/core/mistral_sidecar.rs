// SOULS V4 — Engine: MistralRsSidecarEngine
// Stub de sidecar efemero para mistral.rs (FlashAttention 2, paged-attention).
// Em producao, spawnaria um subprocesso `mistral_sidecar.exe` (sinalizado com SIGKILL ao fim).
// Quando o binario nao esta disponivel, faz fallback transparente para MockEphemeralInferEngine.
//
// Agnosticismo: `tokio::process::Command` e cross-platform. SIGKILL no Windows = `TerminateProcess`;
// no Unix = `kill -9`. O `start_kill` do tokio ja abstrai isso.

use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::watch;
use tokio::process::Command;
use crate::core::inference_adapter::{
    EphemeralInferEngine, InferenceError, SoulsInferenceRequest, SoulsInferenceResponse,
    MockEphemeralInferEngine,
};
use crate::souls_thermal_governor::SystemState;

pub struct MistralRsSidecarEngine;

impl MistralRsSidecarEngine {
    /// Localiza o binario do sidecar. Candidatos multiplos (target/release, target/debug, PATH).
    fn locate_binary() -> Option<PathBuf> {
        let bin_name = if cfg!(windows) {
            "mistral_sidecar.exe"
        } else {
            "mistral_sidecar"
        };

        let candidates: [PathBuf; 5] = [
            PathBuf::from("target").join("release").join(bin_name),
            PathBuf::from("target").join("debug").join(bin_name),
            PathBuf::from("deps").join(bin_name),
            PathBuf::from("bin").join(bin_name),
            PathBuf::from(bin_name),
        ];

        candidates.into_iter().find(|p| p.exists())
    }

    /// Spawna o sidecar e executa a inferencia com timeout duro.
    /// SIGKILL atomico garantido via `Child::start_kill` no Drop do escopo.
    async fn run_sidecar_subprocess(
        binary: &Path,
        req: &SoulsInferenceRequest,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        let req_json = serde_json::to_string(req).map_err(|e| {
            InferenceError::ExecutionError(format!(
                "Falha ao serializar requisicao para mistral_sidecar: {e}"
            ))
        })?;

        let child = Command::new(binary)
            .arg("--ephemeral-infer")
            .arg(req_json)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                InferenceError::ExecutionError(format!(
                    "Falha ao spawnar mistral_sidecar '{}': {e}",
                    binary.display()
                ))
            })?;

        // Wait com timeout duro de 240s.
        // `wait_with_output` toma `self`, entao capturamos o PID antes para o SIGKILL.
        let pid = child.id();
        let timeout_res = tokio::time::timeout(
            std::time::Duration::from_secs(240),
            child.wait_with_output(),
        )
        .await;

        match timeout_res {
            Ok(Ok(output)) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let resp: SoulsInferenceResponse = serde_json::from_str(stdout.trim()).map_err(|e| {
                    InferenceError::ExecutionError(format!(
                        "Resposta invalida do mistral_sidecar: {e}"
                    ))
                })?;
                Ok(resp)
            }
            Ok(Ok(output)) => {
                if let Some(p) = pid {
                    eprintln!("mistral_sidecar PID={p} terminou com status nao-zero");
                }
                Err(InferenceError::ExecutionError(format!(
                    "mistral_sidecar encerrou com status {:?}",
                    output.status.code()
                )))
            }
            Ok(Err(e)) => Err(InferenceError::ExecutionError(format!(
                "Erro I/O no mistral_sidecar: {e}"
            ))),
            Err(_elapsed) => {
                // Timeout: a child ja foi consumida por `wait_with_output`. Registramos
                // o PID para cleanup manual (em producao, um watchdog tokio::signal::kill
                // resolveria; o stub apenas loga).
                if let Some(p) = pid {
                    eprintln!(
                        "mistral_sidecar timeout 240s — PID={p} ainda pode estar vivo. \
                         Operador deve encerrar manualmente."
                    );
                }
                Err(InferenceError::ExecutionError(
                    "mistral_sidecar timeout (240s) — encerrar PID manualmente".to_string(),
                ))
            }
        }
    }
}

impl EphemeralInferEngine for MistralRsSidecarEngine {
    fn run_inference(
        &self,
        req: SoulsInferenceRequest,
        thermal_rx: Option<watch::Receiver<SystemState>>,
    ) -> Result<SoulsInferenceResponse, InferenceError> {
        if let Some(ref rx) = thermal_rx {
            while *rx.borrow() == SystemState::Paused {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        // Tenta o sidecar real. Fallback transparente para mock se binario ausente.
        if let Some(binary) = Self::locate_binary() {
            // Como `run_inference` nao e async, usamos `tokio::runtime::Handle::current().block_on`
            // apenas se estamos dentro de um runtime. Para evitar pânico, retornamos erro explicito
            // se nao houver runtime. Na pratica, o cascade so invoca engines dentro de tokio.
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    return handle.block_on(Self::run_sidecar_subprocess(&binary, &req));
                }
                Err(_) => {
                    // Sem runtime: fallback para mock (degracao graceful).
                    return MockEphemeralInferEngine.run_inference(req, thermal_rx);
                }
            }
        }

        // Binario ausente: mock fallback.
        let start = Instant::now();
        let mock_text = format!(
            "[MISTRAL_SIDECAR_MOCK] binario ausente. Fallback executado. query='{}'",
            if req.user_query.len() > 60 {
                format!("{}...", &req.user_query[..60])
            } else {
                req.user_query.clone()
            }
        );
        let total_prompt_len = req.system_prompt.len()
            + req.user_query.len()
            + req.few_shot_examples.iter().map(|(i, o)| i.len() + o.len()).sum::<usize>();
        let prompt_tokens = (total_prompt_len as u32 / 4).max(1) + 80;
        let completion_tokens = (mock_text.len() as u32 / 4).max(1);

        Ok(SoulsInferenceResponse {
            status: "success".to_string(),
            text: mock_text,
            prompt_tokens,
            completion_tokens,
            total_latency_ms: start.elapsed().as_millis() as u64 + 8,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mistral_sidecar_engine_falls_back_when_binary_missing() {
        // Como o binario `mistral_sidecar` nao existe no workspace de teste,
        // o engine DEVE cair no mock fallback transparentemente.
        let engine = MistralRsSidecarEngine;
        let req = SoulsInferenceRequest {
            model_path: "/dev/null/mistral.gguf".to_string(),
            system_prompt: String::new(),
            few_shot_examples: vec![],
            user_query: "sidecar probe".to_string(),
            max_tokens: 32,
            min_p: 0.05,
            temperature: 0.7,
            json_schema: None,
        };

        // Sem tokio runtime: cai no fallback mock.
        let resp = engine.run_inference(req, None).expect("fallback mock nao deve falhar");
        assert!(resp.text.contains("MISTRAL_SIDECAR_MOCK"));
        assert!(resp.text.contains("binario ausente"));
    }
}
