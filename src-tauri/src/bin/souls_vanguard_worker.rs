use std::io::{self, BufRead, Write};
use souls_mc_lib::core::inference_adapter::{
    EphemeralInferEngine, SoulsInferenceRequest,
};

#[cfg(feature = "llama_backend")]
use souls_mc_lib::core::llama_engine::LlamaCppEngine;

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let handle = stdin.lock();

    for line in handle.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let req: SoulsInferenceRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = serde_json::json!({
                    "status": "error",
                    "error": format!("IPC JSON deserialization failure: {}", e)
                });
                let _ = writeln!(stdout, "{}", err_resp);
                let _ = stdout.flush();
                continue;
            }
        };

        #[cfg(feature = "llama_backend")]
        {
            let engine = LlamaCppEngine;
            match engine.run_inference(req, None) {
                Ok(resp) => {
                    let resp_json = serde_json::to_string(&resp).unwrap_or_default();
                    let _ = writeln!(stdout, "{}", resp_json);
                    let _ = stdout.flush();
                }
                Err(e) => {
                    let err_resp = serde_json::json!({
                        "status": "error",
                        "error": format!("{:?}", e)
                    });
                    let _ = writeln!(stdout, "{}", err_resp);
                    let _ = stdout.flush();
                }
            }
        }

        #[cfg(not(feature = "llama_backend"))]
        {
            let err_resp = serde_json::json!({
                "status": "error",
                "error": "llama_backend feature is not active in souls_vanguard_worker"
            });
            let _ = writeln!(stdout, "{}", err_resp.to_string());
            let _ = stdout.flush();
        }
    }
}
