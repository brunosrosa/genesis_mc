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

/// Mutação in-place (lazy/fast-path) do payload JSON da requisição HTTP
pub fn mutate_json_payload(
    body_bytes: &[u8],
    ccr_store: &SoulsCcrStore,
) -> Result<Vec<u8>, String> {
    // Fast-path Zero-Copy para payloads massivos (> 1MB) sem código de usuário a compactar
    if body_bytes.len() > 1_048_576 {
        let text_slice = std::str::from_utf8(body_bytes).ok();
        let has_code = text_slice.map_or(true, |s| s.contains("fn ") || s.contains("function ") || s.contains("def "));
        let has_headroom_tool = text_slice.map_or(false, |s| s.contains("headroom_retrieve"));

        if !has_code && has_headroom_tool {
            return Ok(body_bytes.to_vec());
        }

        // Se o payload não necessita de compressão de código (sem 'fn ', 'function ', 'def '),
        // evita desserialização de AST/DOM JSON pesada no Heap do Rust em O(1)
        if !has_code {
            return Ok(body_bytes.to_vec());
        }
    }

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

    // Calcular o orçamento de Headroom (H_in) com estimativa de tokens do payload
    let live_tokens = body_bytes.len() / 3;
    let budget = calculate_headroom_budget_for_model(
        &model,
        128_000,
        4_096,
        512,
        1_000,
        2_000,
        10_000,
        live_tokens,
    )
    .unwrap_or_else(|_| {
        calculate_headroom_budget(32_768, 4_096, 512, 1_000, 2_000, 10_000, live_tokens)
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

fn find_sse_frame_delimiter(buf: &[u8]) -> Option<(usize, usize)> {
    for i in 0..buf.len() {
        if buf[i..].starts_with(b"\n\n") {
            return Some((i, 2));
        }
        if buf[i..].starts_with(b"\r\n\r\n") {
            return Some((i, 4));
        }
    }
    None
}

/// Acumulador de Bytes Brutos para reconstrução de quadros SSE contra fragmentação TCP
pub struct SseFrameAccumulator {
    acc_buf: Vec<u8>,
}

impl SseFrameAccumulator {
    pub fn new() -> Self {
        Self {
            acc_buf: Vec::with_capacity(16384),
        }
    }

    pub fn push_chunk(&mut self, chunk: &[u8]) -> Vec<Vec<u8>> {
        self.acc_buf.extend_from_slice(chunk);
        let mut frames = Vec::new();

        while let Some((pos, delim_len)) = find_sse_frame_delimiter(&self.acc_buf) {
            let frame_end = pos + delim_len;
            let frame_bytes: Vec<u8> = self.acc_buf.drain(..frame_end).collect();
            frames.push(frame_bytes);
        }

        frames
    }

    pub fn flush_remaining(&mut self) -> Option<Vec<u8>> {
        if !self.acc_buf.is_empty() {
            Some(std::mem::take(&mut self.acc_buf))
        } else {
            None
        }
    }
}

/// Pipeline de interceção e resposta SSE com acumulador de linhas/quadros inteiros
async fn handle_upstream_response<D, U>(
    mut downstream: D,
    mut upstream: U,
    ccr_store: Arc<SoulsCcrStore>,
) -> io::Result<()>
where
    D: AsyncWriteExt + Unpin,
    U: AsyncReadExt + Unpin,
{
    let mut read_buf = vec![0u8; 8192];
    let mut acc = SseFrameAccumulator::new();

    loop {
        let n = upstream.read(&mut read_buf).await?;
        if n == 0 {
            break;
        }
        let frames = acc.push_chunk(&read_buf[..n]);

        for frame_bytes in frames {
            let frame_str = String::from_utf8_lossy(&frame_bytes);

            // Sequestro do Loopback SSE em frame reconstruído
            if frame_str.contains("headroom_retrieve") {
                if let Some(hydrated) = ccr_store.intercept_loopback(&frame_str) {
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

            downstream.write_all(&frame_bytes).await?;
            downstream.flush().await?;
        }
    }

    // Processar residual final ao encerrar a stream
    if let Some(remaining) = acc.flush_remaining() {
        let frame_str = String::from_utf8_lossy(&remaining);
        if frame_str.contains("headroom_retrieve") {
            if let Some(hydrated) = ccr_store.intercept_loopback(&frame_str) {
                let sse_event = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: {}\n\ndata: [DONE]\n\n",
                    hydrated
                );
                downstream.write_all(sse_event.as_bytes()).await?;
                downstream.flush().await?;
                return Ok(());
            }
        }
        downstream.write_all(&remaining).await?;
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
        let block = r#"
fn process_heavy_data(input: &str) -> String {
    let x = input.to_lowercase();
    let y = x.trim();
    format!("RESULT: {}", y)
}
"#;
        let long_code = block.repeat(500);
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

    #[test]
    fn test_mutate_json_payload_zero_copy_fast_path() {
        let store = SoulsCcrStore::new(16 * 1024 * 1024);
        let mut large_json = String::from(r#"{"model":"gemma4","tools":[{"type":"function","function":{"name":"headroom_retrieve"}}],"data":""#);
        large_json.push_str(&"A".repeat(1_100_000));
        large_json.push_str(r#""}"#);

        let start = std::time::Instant::now();
        let result = mutate_json_payload(large_json.as_bytes(), &store).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.len(), large_json.len());
        assert!(elapsed.as_millis() < 500, "Operação Zero-Copy executada em {} ms", elapsed.as_millis());
    }

    #[tokio::test]
    async fn test_sse_line_buffering_fragmented_chunks() {
        let store = Arc::new(SoulsCcrStore::new(16 * 1024 * 1024));
        let payload = b"fn secret_logic() {}";
        let hash = store.store(payload);
        let hex_hash = hex_encode(&hash);

        let part1 = format!("data: {{\"name\":\"headroom_");
        let part2 = format!("retrieve\",\"parameters\":{{\"hash\":\"{}\"}}}}\n\n", hex_hash);

        let (client_down, server_down) = tokio::io::duplex(1024);
        let (mut client_up, server_up) = tokio::io::duplex(1024);

        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            handle_upstream_response(client_down, server_up, store_clone).await
        });

        client_up.write_all(part1.as_bytes()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        client_up.write_all(part2.as_bytes()).await.unwrap();

        let mut reader = server_down;
        let mut resp_buf = vec![0u8; 1024];
        let n = reader.read(&mut resp_buf).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);

        assert!(resp_str.contains("secret_logic"));
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_sse_buffer_tcp_fragmentation() {
        let store = Arc::new(SoulsCcrStore::new(16 * 1024 * 1024));
        let payload = "fn cbor_utf8_coracao() { println!(\"Coração UTF-8\"); }".as_bytes();
        let hash = store.store(payload);
        let hex_hash = hex_encode(&hash);

        let full_sse = format!(
            "data: {{\"name\":\"headroom_retrieve\",\"parameters\":{{\"hash\":\"{}\"}},\"msg\":\"coracao utf8\"}}\n\n",
            hex_hash
        );

        let bytes = full_sse.as_bytes();
        let mut acc = SseFrameAccumulator::new();
        let mut reconstructed_frames = Vec::new();

        // Injeta o stream SSE fatiado em pedaços arbitrários de 10 bytes
        for chunk in bytes.chunks(10) {
            let frames = acc.push_chunk(chunk);
            for f in frames {
                reconstructed_frames.push(f);
            }
        }
        if let Some(rem) = acc.flush_remaining() {
            reconstructed_frames.push(rem);
        }

        assert_eq!(reconstructed_frames.len(), 1, "Deveria reconstruir exatamente 1 frame SSE completo");
        let frame_str = String::from_utf8_lossy(&reconstructed_frames[0]);
        assert!(frame_str.contains("headroom_retrieve"));

        let hydrated = store.intercept_loopback(&frame_str);
        assert!(hydrated.is_some());
        assert!(hydrated.unwrap().contains("coracao"));
    }
}
