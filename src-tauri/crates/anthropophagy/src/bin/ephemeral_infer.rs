use std::io::{self, Read};
use std::time::Duration;
use serde_json::json;
use souls_mc_lib::core::inference_adapter::{
    EphemeralInferEngine, SoulsInferenceRequest,
};

#[cfg(all(not(feature = "llama_backend"), not(feature = "mistral_backend")))]
use souls_mc_lib::core::inference_adapter::MockEphemeralInferEngine;

#[cfg(feature = "llama_backend")]
use souls_mc_lib::core::llama_engine::LlamaCppEngine;

#[cfg(feature = "mistral_backend")]
use souls_mc_lib::core::mistral_engine::MistralRsEngine;

#[tokio::main]
async fn main() {
    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("ERR: Falha ao ler payload JSON do stdin: {}", err);
        std::process::exit(1);
    }

    let input_trimmed = input.trim();
    if input_trimmed.is_empty() {
        eprintln!("ERR: Stdin vazio. Esperado payload JSON do SoulsInferenceRequest.");
        std::process::exit(1);
    }

    let request: SoulsInferenceRequest = match serde_json::from_str(input_trimmed) {
        Ok(req) => req,
        Err(err) => {
            eprintln!("ERR: Desserializacao do SoulsInferenceRequest falhou: {}", err);
            std::process::exit(1);
        }
    };

    #[cfg(feature = "mistral_backend")]
    let engine = MistralRsEngine;

    #[cfg(all(feature = "llama_backend", not(feature = "mistral_backend")))]
    let engine = LlamaCppEngine;

    #[cfg(all(not(feature = "llama_backend"), not(feature = "mistral_backend")))]
    let engine = MockEphemeralInferEngine;

    let thermal_rx = souls_mc_lib::souls_thermal_governor::spawn_thermal_governor();

    // BARE-METAL PURIFICATION: Dedicated OS Worker Thread via std::thread::spawn
    // Replaces tokio::task::spawn_blocking to preserve CPU L1/L2 cache and AVX2 vector alignment.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let thermal_rx_clone = thermal_rx.clone();

    if let Err(spawn_err) = std::thread::Builder::new()
        .name("souls-ephemeral-worker".to_string())
        .spawn(move || {
            let res = engine.run_inference(request, Some(thermal_rx_clone));
            let _ = tx.send(res);
        })
    {
        eprintln!("ERR: Falha ao spawnar Dedicated OS Worker Thread: {}", spawn_err);
        std::process::exit(1);
    }

    let timeout_duration = Duration::from_secs(300);
    let infer_result = tokio::time::timeout(timeout_duration, rx).await;

    match infer_result {
        Ok(Ok(Ok(response))) => {
            let json_output = json!({
                "status": response.status,
                "text": response.text,
                "prompt_tokens": response.prompt_tokens,
                "completion_tokens": response.completion_tokens,
                "total_latency_ms": response.total_latency_ms
            });
            if let Ok(pretty) = serde_json::to_string_pretty(&json_output) {
                println!("{}", pretty);
            }
            std::process::exit(0);
        }
        Ok(Ok(Err(err))) => {
            let err_output = json!({
                "status": "error",
                "error": err.to_string()
            });
            if let Ok(pretty) = serde_json::to_string_pretty(&err_output) {
                eprintln!("{}", pretty);
            }
            std::process::exit(1);
        }
        Ok(Err(recv_err)) => {
            let err_output = json!({
                "status": "error",
                "error": format!("Falha no canal oneshot da Dedicated Worker Thread: {}", recv_err)
            });
            if let Ok(pretty) = serde_json::to_string_pretty(&err_output) {
                eprintln!("{}", pretty);
            }
            std::process::exit(1);
        }
        Err(_elapsed) => {
            let err_output = json!({
                "status": "error",
                "text": "TIMEOUT_FATAL",
                "error": "Tempo limite de inferencia excedido (300s)"
            });
            if let Ok(pretty) = serde_json::to_string_pretty(&err_output) {
                println!("{}", pretty);
            }
            std::process::exit(1);
        }
    }
}
