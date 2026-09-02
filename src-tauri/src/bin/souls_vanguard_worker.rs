use std::io::{self, BufRead, Write};
use souls_mc_lib::core::inference_adapter::{
    EphemeralInferEngine, SoulsInferenceRequest,
};
use souls_mc_lib::core::llama_upstream_engine::LlamaUpstreamEngine;

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

        let engine = LlamaUpstreamEngine::new_gpu();
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
}
