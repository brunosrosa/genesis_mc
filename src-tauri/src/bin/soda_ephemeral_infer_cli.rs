use std::io::{self, Read};
use std::time::Duration;
use serde_json::json;
use souls_mc_lib::core::inference_adapter::{
    EphemeralInferEngine, SodaInferenceRequest,
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
        eprintln!("ERR: Stdin vazio. Esperado payload JSON do SodaInferenceRequest.");
        std::process::exit(1);
    }

    let request: SodaInferenceRequest = match serde_json::from_str(input_trimmed) {
        Ok(req) => req,
        Err(err) => {
            eprintln!("ERR: Desserializacao do SodaInferenceRequest falhou: {}", err);
            std::process::exit(1);
        }
    };

    #[cfg(feature = "mistral_backend")]
    let engine = MistralRsEngine;

    #[cfg(all(feature = "llama_backend", not(feature = "mistral_backend")))]
    let engine = LlamaCppEngine;

    #[cfg(all(not(feature = "llama_backend"), not(feature = "mistral_backend")))]
    let engine = MockEphemeralInferEngine;

    let timeout_duration = Duration::from_secs(300);
    let infer_result = tokio::time::timeout(
        timeout_duration,
        tokio::task::spawn_blocking(move || engine.run_inference(request))
    ).await;

    match infer_result {
        Ok(Ok(Ok(response))) => {
            let json_output = json!({
                "status": response.status,
                "text": response.text,
                "prompt_tokens": response.prompt_tokens,
                "completion_tokens": response.completion_tokens,
                "total_latency_ms": response.total_latency_ms
            });
            println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            std::process::exit(0);
        }
        Ok(Ok(Err(err))) => {
            let err_output = json!({
                "status": "error",
                "error": err.to_string()
            });
            eprintln!("{}", serde_json::to_string_pretty(&err_output).unwrap());
            std::process::exit(1);
        }
        Ok(Err(join_err)) => {
            let err_output = json!({
                "status": "error",
                "error": format!("Falha de thread no Tokio: {}", join_err)
            });
            eprintln!("{}", serde_json::to_string_pretty(&err_output).unwrap());
            std::process::exit(1);
        }
        Err(_elapsed) => {
            let err_output = json!({
                "status": "error",
                "text": "TIMEOUT_FATAL",
                "error": "Tempo limite de inferencia excedido (300s)"
            });
            println!("{}", serde_json::to_string_pretty(&err_output).unwrap());
            std::process::exit(1);
        }
    }
}
