use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tracing;

use souls_mc_lib::core::headroom_engine::{
    calculate_headroom_budget, calculate_headroom_budget_for_model, CodeCompressor, SoulsCcrStore, hex_encode,
};

fn parse_cli_args() -> (SocketAddr, SocketAddr) {
    let mut args = std::env::args();
    args.next();

    let mut listen = "127.0.0.1:3000".to_string();
    let mut upstream = "127.0.0.1:3001".to_string();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--listen" => {
                if let Some(v) = args.next() {
                    listen = v;
                }
            }
            "--upstream" => {
                if let Some(v) = args.next() {
                    upstream = v;
                }
            }
            _ => {}
        }
    }

    let listen = listen.parse().unwrap_or_else(|_| "127.0.0.1:3000".parse().unwrap());
    let upstream = upstream.parse().unwrap_or_else(|_| "127.0.0.1:3001".parse().unwrap());
    (listen, upstream)
}

fn set_nodelay(stream: &TcpStream) {
    let _ = stream.set_nodelay(true);
}

/// Tool Schema para a ferramenta fantasma headroom_retrieve (PRD-10.3)
pub fn phantom_headroom_tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "headroom_retrieve",
            "description": "Resgata trecho de código original compactado pelo CCR via hash Hex de 16 bytes",
            "parameters": {
                "type": "object",
                "properties": {
                    "hash": {
                        "type": "string",
                        "description": "Hash hexadecimal de 16 bytes do payload original no CCR"
                    }
                },
                "required": ["hash"]
            }
        }
    })
}

/// Mutação in-place (lazy) do payload JSON da requisição HTTP
pub fn mutate_json_payload(
    body_bytes: &[u8],
    ccr_store: &SoulsCcrStore,
) -> Result<Vec<u8>, String> {
    let mut json_val: Value = serde_json::from_slice(body_bytes)
        .map_err(|e| format!("Erro ao parsear JSON: {e}"))?;

    let model = json_val
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gemma4")
        .to_string();

    // Injetar a ferramenta fantasma headroom_retrieve no array "tools"
    if let Some(tools) = json_val.get_mut("tools").and_then(|t| t.as_array_mut()) {
        let exists = tools.iter().any(|t| {
            t.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                == Some("headroom_retrieve")
        });
        if !exists {
            tools.push(phantom_headroom_tool());
        }
    } else {
        json_val["tools"] = json!([phantom_headroom_tool()]);
    }

    // Calcular o orçamento de Headroom (H_in)
    let budget = calculate_headroom_budget_for_model(
        &model,
        128_000,
        4_096,
        512,
        1_000,
        2_000,
        10_000,
        2_000,
    )
    .unwrap_or_else(|_| {
        calculate_headroom_budget(32_768, 4_096, 512, 1_000, 2_000, 10_000, 2_000)
    });

    // Se o headroom acionar a compressão (trigger == true)
    if budget.trigger {
        if let Some(messages) = json_val.get_mut("messages").and_then(|m| m.as_array_mut()) {
            for msg in messages.iter_mut() {
                if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
                    if role == "user" {
                        if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                            if content.contains("fn ") || content.contains("function ") || content.contains("def ") {
                                let compressed = CodeCompressor::compress_ast_zero_copy(content);
                                if compressed.len() < content.len() {
                                    let hash = ccr_store.store(content.as_bytes());
                                    let hex_hash = hex_encode(&hash);
                                    let stubbed_content = format!(
                                        "[CCR STUBBED HASH:{}]\n{}",
                                        hex_hash, compressed
                                    );
                                    msg["content"] = Value::String(stubbed_content);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    serde_json::to_vec(&json_val).map_err(|e| format!("Erro ao serializar JSON mutado: {e}"))
}

/// Invocação de subprocesso MCP com garantia de observabilidade
pub async fn spawn_mcp_subprocess(command_path: &str, args: &[&str]) -> io::Result<tokio::process::Child> {
    tracing::info!("Tentando executar subprocesso: {}", command_path);
    let mut cmd = tokio::process::Command::new(command_path);
    cmd.args(args);
    cmd.spawn().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Falha ao executar subprocesso em '{}': {e}", command_path),
        )
    })
}

/// Pipeline de interceção e resposta SSE Sans-I/O do Upstream LLM
async fn handle_upstream_response(
    mut downstream: TcpStream,
    mut upstream: TcpStream,
    ccr_store: Arc<SoulsCcrStore>,
) -> io::Result<()> {
    let mut resp_buf = vec![0u8; 8192];
    loop {
        let n = upstream.read(&mut resp_buf).await?;
        if n == 0 {
            break;
        }
        let chunk = &resp_buf[..n];
        let chunk_str = String::from_utf8_lossy(chunk);

        // Sequestro do Loopback SSE: Se a IA acionou headroom_retrieve(hash), interceptar
        if chunk_str.contains("headroom_retrieve") {
            if let Some(hydrated) = ccr_store.intercept_loopback(&chunk_str) {
                tracing::info!("Sequestro de Loopback SSE acionado em < 1ms! Contexto hidratado.");
                let sse_event = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: {}\n\ndata: [DONE]\n\n",
                    hydrated
                );
                downstream.write_all(sse_event.as_bytes()).await?;
                downstream.flush().await?;
                return Ok(());
            }
        }

        downstream.write_all(chunk).await?;
        downstream.flush().await?;
    }
    Ok(())
}

/// Proxy L7 HTTP/1.1 Zero-Copy Interceptor
async fn handle_l7_proxy(
    mut downstream: TcpStream,
    mut upstream: TcpStream,
    ccr_store: Arc<SoulsCcrStore>,
) -> io::Result<()> {
    set_nodelay(&downstream);
    set_nodelay(&upstream);

    let mut buf = vec![0u8; 16384];
    let mut read_bytes = 0;

    // Parse Zero-Copy dos cabeçalhos HTTP/1.1 via httparse (0 alocações de Heap)
    loop {
        if read_bytes >= buf.len() {
            break;
        }
        let n = downstream.read(&mut buf[read_bytes..]).await?;
        if n == 0 {
            break;
        }
        read_bytes += n;

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse(&buf[..read_bytes]) {
            Ok(httparse::Status::Complete(header_len)) => {
                let path = req.path.unwrap_or("");
                let method = req.method.unwrap_or("");

                if method == "POST" && path.contains("/v1/chat/completions") {
                    let mut content_length = None;
                    for h in req.headers.iter() {
                        if h.name.eq_ignore_ascii_case("content-length") {
                            if let Ok(s) = std::str::from_utf8(h.value) {
                                content_length = s.parse::<usize>().ok();
                            }
                        }
                    }

                    if let Some(cl) = content_length {
                        let mut body = buf[header_len..read_bytes].to_vec();
                        while body.len() < cl {
                            let mut temp = vec![0u8; (cl - body.len()).min(8192)];
                            let n = downstream.read(&mut temp).await?;
                            if n == 0 {
                                break;
                            }
                            body.extend_from_slice(&temp[..n]);
                        }

                        // Mutação in-place do JSON (Headroom & CodeCompressor)
                        let mut final_body = match mutate_json_payload(&body, &ccr_store) {
                            Ok(b) => b,
                            Err(e) => {
                                tracing::warn!("Fallback L7 (Fail-Soft) devido a erro de mutação JSON: {}", e);
                                body
                            }
                        };

                        let req_header = format!(
                            "POST {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            path,
                            final_body.len()
                        );
                        let mut full_req = req_header.into_bytes();
                        full_req.append(&mut final_body);

                        upstream.write_all(&full_req).await?;
                        upstream.flush().await?;

                        return handle_upstream_response(downstream, upstream, ccr_store).await;
                    }
                }
                break;
            }
            Ok(httparse::Status::Partial) => {
                continue;
            }
            Err(e) => {
                tracing::warn!("Fail-Soft: Erro no parse de cabeçalhos HTTP/1.1: {}", e);
                break;
            }
        }
    }

    // Fallback Fail-Soft: repassar bytes brutos e copiar de forma bidirecional
    if read_bytes > 0 {
        upstream.write_all(&buf[..read_bytes]).await?;
    }
    let _ = copy_bidirectional(&mut downstream, &mut upstream).await;
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();

    let (listen, upstream) = parse_cli_args();
    let ccr_store = Arc::new(SoulsCcrStore::from_env());
    let listener = TcpListener::bind(listen).await?;

    tracing::info!("Proxy L7 Zero-Copy escutando em {} -> upstream {}", listen, upstream);

    loop {
        let (downstream, _) = listener.accept().await?;
        let Ok(up) = TcpStream::connect(upstream).await else {
            tracing::warn!("Falha ao conectar com o upstream LLM em {}", upstream);
            continue;
        };

        let store_clone = Arc::clone(&ccr_store);
        tokio::spawn(async move {
            if let Err(e) = handle_l7_proxy(downstream, up, store_clone).await {
                tracing::error!("Erro no man-in-the-middle L7 proxy: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phantom_headroom_tool_injection() {
        let store = SoulsCcrStore::new(16 * 1024 * 1024);
        let input_json = r#"{
            "model": "gemma4",
            "messages": [{"role": "user", "content": "Olá!"}]
        }"#;

        let mutated = mutate_json_payload(input_json.as_bytes(), &store).unwrap();
        let val: Value = serde_json::from_slice(&mutated).unwrap();

        let tools = val.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "headroom_retrieve");
    }

    #[test]
    fn test_headroom_ast_compression_on_user_code() {
        let store = SoulsCcrStore::new(16 * 1024 * 1024);
        let long_code = r#"
fn process_heavy_data(input: &str) -> String {
    let x = input.to_lowercase();
    let y = x.trim();
    format!("RESULT: {}", y)
}
"#;
        let input_json = json!({
            "model": "gemma4",
            "messages": [
                {
                    "role": "user",
                    "content": long_code
                }
            ]
        });

        let input_bytes = serde_json::to_vec(&input_json).unwrap();
        let mutated = mutate_json_payload(&input_bytes, &store).unwrap();
        let val: Value = serde_json::from_slice(&mutated).unwrap();

        let user_msg = val["messages"][0]["content"].as_str().unwrap();
        assert!(user_msg.contains("[CCR STUBBED HASH:"));
        assert!(user_msg.contains("/* stubbed */"));
    }

    #[tokio::test]
    async fn test_spawn_mcp_observability_logging() {
        let err = spawn_mcp_subprocess("invalid_mcp_executable_999.exe", &[])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("invalid_mcp_executable_999.exe"));
    }
}
