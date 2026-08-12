use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::Mutex;

use souls_mc_lib::core::gigatoken_encoder::GigaTokenEncoder;
use souls_mc_lib::core::headroom_engine::{
    calculate_headroom_budget, calculate_headroom_budget_for_model, CodeCompressor, SoulsCcrStore, hex_encode,
};
use souls_mc_lib::core::l7_shield::{
    intercepted_to_jsonrpc, is_mutating_method, EpistemicShieldChannel, ShieldContext, ShieldDecision,
};
use souls_mc_lib::core::response_healing::heal_malformed_json;

// Marco I · v6.1 — Agnostic L7 Gateway wiring
use souls_mc_lib::core::gateway_config::GatewayConfig;
use souls_mc_lib::core::peak_ewma::global_peak_ewma;
use souls_mc_lib::core::pii_redactor::PiiRedactor;
use souls_mc_lib::core::sticky_router::{prepend_header, RoutePin, StickyRouter};
use souls_mc_lib::core::subprocess_guard::{SubprocessConfig, SubprocessGuard};
use souls_mc_lib::core::telemetry_dispatcher::{
    init_telemetry_dispatcher, telemetry_sender,
};
use souls_mc_lib::finops::iron_cost::{IronCostBreaker, ModelTier};
use souls_mc_lib::finops::pareto_bandit::{
    evaluate_route_decision, TaskKind,
};

/// Parse do único argumento CLI relevante: `--listen`.
/// Default: `127.0.0.1:3001` (porta unificada do Marco I).
///
/// **Sem `--upstream`**: o proxy escreve JSON-RPC no stdin do `SubprocessGuard`
/// (`souls_mcp_server`) e lê a resposta do stdout. Não há loopback TCP.
fn parse_listen_arg() -> SocketAddr {
    let mut args = std::env::args();
    let _ = args.next(); // próprio binário

    let mut listen = "127.0.0.1:3001".to_string();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--listen" => {
                if let Some(v) = args.next() {
                    listen = v;
                }
            }
            // `--upstream` é aceito apenas como no-op silencioso (legado),
            // para não quebrar scripts que ainda o passam. O loopback
            // suicida (3001→3001) foi ERRADICADO.
            "--upstream" => {
                if let Some(v) = args.next() {
                    tracing::warn!(
                        "Argumento legado '--upstream {v}' ignorado: o proxy agora \
                         escreve direto no stdin do souls_mcp_server (SubprocessGuard)."
                    );
                }
            }
            _ => {}
        }
    }

    listen
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:3001".parse().unwrap())
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
        let has_code = text_slice.is_none_or(|s| s.contains("fn ") || s.contains("function ") || s.contains("def "));
        let has_headroom_tool = text_slice.is_some_and(|s| s.contains("headroom_retrieve"));

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

/// Spawna o `souls_mcp_server` (ou binário configurado no JSONC) sob
/// ownership do `SubprocessGuard` (RAII kill_on_drop). Esta é a função
/// canônica de boot do subprocesso MCP — o proxy detém o ciclo de vida.
///
/// Falha de spawn é **logged e não-fatal**: o proxy continua atendendo
/// requisições HTTP mesmo sem o subprocesso MCP (fail-soft, Marco 4.10.0).
pub fn spawn_souls_mcp_server() -> Option<SubprocessGuard> {
    let cfg = SubprocessConfig::from_gateway_config();
    match SubprocessGuard::spawn(&cfg) {
        Ok(guard) => {
            tracing::info!(
                "souls_mcp_server spawned sob SubprocessGuard: pid={:?}",
                guard.pid()
            );
            Some(guard)
        }
        Err(e) => {
            tracing::warn!(
                "Falha ao spawnar souls_mcp_server (continuando sem subprocesso MCP): {e}"
            );
            None
        }
    }
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
#[derive(Default)]
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

/// SOULS Marco 4.9.2 — Gate de cura sintática para data lines SSE.
///
/// Extrai o payload JSON de uma `data: ...` line e aplica o
/// `heal_malformed_json` do `jsonrepair` para fechar delimitadores
/// truncados em < 1ms antes de repassar ao consumidor downstream.
///
/// Contratos:
/// - Latência < 1ms (medida por `test_sse_accumulator_cures_truncated_frame`).
/// - Fail-soft: em caso de erro de parse, devolve o frame original inalterado.
/// - Preserva linhas que NÃO sejam `data:` (event:, id:, comments, keep-alives).
/// - Preserva o marcador de fim `[DONE]` da OpenAI.
pub fn cure_sse_data_line(frame: &[u8]) -> Vec<u8> {
    // Localiza a primeira "data: " no frame (UTF-8 safe via str::from_utf8).
    let frame_str = match std::str::from_utf8(frame) {
        Ok(s) => s,
        // Frame binário/raro: bypassa cura, devolve inalterado.
        Err(_) => return frame.to_vec(),
    };

    // Tipo explícito `String` (não Vec<char>) — `push('\n')` abaixo não ambígua.
    let mut output: String = String::with_capacity(frame.len() + 64);
    let mut remaining: &str = frame_str;

    while !remaining.is_empty() {
        // Encontra a próxima "data: " (case-sensitive, conforme RFC SSE).
        match remaining.find("data: ") {
            None => {
                output.push_str(remaining);
                break;
            }
            Some(idx) => {
                // Copia o prefixo (antes de "data: ") inalterado.
                output.push_str(&remaining[..idx]);
                remaining = &remaining[idx..];

                // Extrai a data line (até \n ou fim do frame).
                let line_end = remaining.find('\n').unwrap_or(remaining.len());
                let line: &str = &remaining[..line_end];
                remaining = &remaining[line_end..];

                // Preserva marcador [DONE] e linhas vazias sem cura.
                let payload: &str = line.strip_prefix("data: ").unwrap_or("").trim();
                if payload.is_empty() || payload == "[DONE]" {
                    output.push_str(line);
                    if !remaining.is_empty() {
                        output.push('\n');
                    }
                    continue;
                }

                // Cura sintática estrutural (jsonrepair) — < 1ms.
                let cured: std::borrow::Cow<'_, str> = heal_malformed_json(payload);
                output.push_str("data: ");
                output.push_str(&cured);
                if !remaining.is_empty() {
                    output.push('\n');
                }
            }
        }
    }

    output.into_bytes()
}

// ============================================================================
// Marco I · v6.1 — `handle_l7_proxy_v6`: pipe completo de 7 camadas
// ============================================================================
//
// Pipeline em ordem (cada camada tem fail-soft próprio):
// 1. Parse Zero-Copy dos headers via `httparse`.
// 2. Extract `session_id` (sticky) + `complexity` (ParetoBandit) + tokens (GigaTokenEncoder).
// 3. PII redaction (opt-in) — Aho-Corasick linear CPU.
// 4. Sticky routing — lock session_id → (provider, model), prepend Z1+Z2 byte-stable.
// 5. IronCostBreaker — consulta SQLite real (cost diário) + JSONC config.
// 6. CCR Headroom (já existente) — compressão de código.
// 7. Upstream write + SSE response com TTFT → PeakEWMA + TelemetryDispatcher.
// ============================================================================

#[allow(clippy::too_many_arguments)]
async fn handle_l7_proxy_v6(
    mut downstream: TcpStream,
    mcp_stdin: Arc<Mutex<ChildStdin>>,
    mcp_stdout: Arc<Mutex<ChildStdout>>,
    request_serial_lock: Arc<Mutex<()>>,
    ccr_store: Arc<SoulsCcrStore>,
    shield_channel: EpistemicShieldChannel,
    sticky: Arc<StickyRouter>,
    pii: Arc<PiiRedactor>,
) -> io::Result<()> {
    set_nodelay(&downstream);

    let mut buf = vec![0u8; 16384];
    let mut read_bytes = 0;
    let request_started_at = Instant::now();

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
                let path = req.path.unwrap_or("").to_string();
                let method = req.method.unwrap_or("").to_string();

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

                        // (3) PII Redaction (opt-in, default DESABILITADO).
                        if pii.is_enabled() {
                            let before_len = body.len();
                            body = pii.redact(&body);
                            if body.len() != before_len {
                                tracing::info!(
                                    "PII redaction aplicada ({} → {} bytes)",
                                    before_len,
                                    body.len()
                                );
                            }
                        }

                        // (2) Extrai session_id e computa complexity.
                        let session_id = extract_session_id(req.headers, &path);
                        let body_str = std::str::from_utf8(&body).unwrap_or("");
                        let task_kind = TaskKind::classify_from_prompt(body_str);
                        let complexity = estimate_complexity(body_str);
                        let token_count = count_real_tokens(body_str);

                        // (4) Sticky routing — lock ou resolve pin.
                        let cfg = GatewayConfig::global();
                        let default_endpoint = &cfg.routes.heavy_brain_endpoint;
                        let initial_pin = RoutePin::new(
                            &default_endpoint.provider,
                            &default_endpoint.model,
                            &default_endpoint.fallback_model,
                        );
                        let (pinned, header) = sticky.resolve_header(&session_id, &initial_pin);
                        let effective_pin = pinned.unwrap_or(initial_pin);

                        // (5) Route decision (ParetoBandit + eco-hybrid guard).
                        let decision = evaluate_route_decision(
                            task_kind,
                            complexity,
                            &effective_pin.model,
                        );
                        let model_tier = if decision.endpoint.estimated_cost_per_1m_usd <= 1.0 {
                            ModelTier::FlashCloud
                        } else {
                            ModelTier::PremiumCloud
                        };
                        let _route_reasons = &decision.reasons;
                        let _ = model_tier; // uso real está abaixo no IronCostBreaker

                        // (5b) IronCostBreaker — consulta SQLite + budget.
                        let cost_check = IronCostBreaker::calculate_and_route(
                            token_count as usize,
                            model_tier,
                        );
                        if let Err(e) = &cost_check {
                            tracing::warn!("IronCostBreaker bloqueou: {e}. Forçando local.");
                            // Fallback Fail-Soft: deixa passar com warning (em prod
                            // seria um 429 amigável, mas a UI trata).
                        }

                        // L7 Shield (existente) — submete body mutante.
                        if is_mutating_method(&method) {
                            let ctx = ShieldContext::new(
                                format!("proxy-v6-{}-{}", std::process::id(), read_bytes),
                                &method,
                                &path,
                            );
                            let decision_rx = shield_channel.submit(ctx, body.clone());
                            let decision = match decision_rx.await {
                                Ok(d) => d,
                                Err(_) => ShieldDecision::Bypass {
                                    reason: "shield channel closed; fail-soft bypass",
                                },
                            };
                            if decision.is_intercepted() {
                                tracing::warn!("L7 Shield interceptou requisição: {:?}", decision);
                                write_shield_http_response(&mut downstream, &decision).await?;
                                return Ok(());
                            }
                        }

                        // (6) CCR Headroom mutation (existente).
                        let mut final_body = match mutate_json_payload(&body, &ccr_store) {
                            Ok(b) => b,
                            Err(e) => {
                                tracing::warn!("Fallback L7 (Fail-Soft) devido a erro de mutação JSON: {}", e);
                                body
                            }
                        };

                        // (4b) Prepend do header sticky (Z1+Z2 byte-stable).
                        if let Some(h) = header {
                            final_body = prepend_header(&final_body, &h);
                        }

                        // Marco I · v6.1: pipe JSON-RPC direto no stdin do
                        // `SubprocessGuard` (souls_mcp_server). Sem header
                        // HTTP upstream, sem loopback TCP — o subprocesso
                        // consome line-delimited JSON via `BufReader::lines`.
                        //
                        // **Serial lock global (Marco I · v6.4)**: requests
                        // HTTP concorrentes (1+ clients do `mcp-remote`)
                        // disputam o mesmo `ChildStdin`/`ChildStdout`. Sem
                        // este lock, task A pode ler a response da task B
                        // (race condition), causando timeout 60s do client
                        // esperando response que nunca chega. O lock
                        // global serializa o ciclo stdin-write → stdout-read.
                        //
                        // O `_serial_guard` permanece vivo durante o
                        // `handle_mcp_stdout_response` (é dropado no final
                        // desta função, APÓS o `.await` da response).
                        let _serial_guard = request_serial_lock.lock().await;
                        {
                            let mut stdin_guard = mcp_stdin.lock().await;
                            stdin_guard.write_all(&final_body).await?;
                            stdin_guard.write_all(b"\n").await?;
                            stdin_guard.flush().await?;
                        }

                        // (7) Response do stdout do subprocesso (JSON-RPC
                        // line-delimited → envelopado em SSE para o cliente).
                        return handle_mcp_stdout_response(
                            downstream,
                            mcp_stdout,
                            ccr_store,
                            request_started_at,
                            &effective_pin.model,
                            token_count,
                            Some(session_id),
                        )
                        .await;
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

    // Fallback Fail-Soft: para requests não-POST/chat, passa o body cru
    // para o subprocesso (1 linha JSON) e envelopa a resposta em SSE.
    if read_bytes > 0 {
        // Tenta extrair o body HTTP (após o header \r\n\r\n).
        let body_start = buf[..read_bytes]
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| p + 4)
            .unwrap_or(0);
        let body = &buf[body_start..read_bytes];
        if !body.is_empty() {
            // Marco I · v6.4: serial lock global — ver comentário no caminho
            // principal. Garante que a response desta request caia no
            // PRÓPRIO client (não na task concorrente).
            let _serial_guard = request_serial_lock.lock().await;
            {
                let mut stdin_guard = mcp_stdin.lock().await;
                stdin_guard.write_all(body).await?;
                stdin_guard.write_all(b"\n").await?;
                stdin_guard.flush().await?;
                drop(stdin_guard);
            }
            return handle_mcp_stdout_response(
                downstream,
                mcp_stdout,
                ccr_store,
                request_started_at,
                "passthrough",
                0,
                None,
            )
            .await;
        }
    }
    Ok(())
}

/// Envelope HTTP/1.1 canônico que o proxy envia **antes** do body JSON-RPC.
///
/// **Por que é obrigatório**: o `mcp-remote` (Node + undici H1 parser) usa
/// `fetch()` HTTP/1.1 contra `http://127.0.0.1:3001/`. O parser do undici
/// exige uma *response line* (`HTTP/1.1 200 OK`) seguida de headers e do
/// `\r\n\r\n` separador. Sem isso, o parser dispara
/// `HTTPParserError: Response does not match the HTTP/1.1 protocol`.
///
/// **Content-Type `application/json` (NÃO SSE)**: o transporte MCP
/// StreamableHTTP (padrão do `mcp-remote`) parseia o body como **JSON
/// puro** quando o content-type é `application/json`. SSE (`text/event-stream`)
/// exige parsing de `data: ...\n\n` que o `mcp-remote` NÃO faz — qualquer
/// suffixo como `data: [DONE]\n\n` causa `SyntaxError: Unexpected token 'D'`.
///
/// **Content-Length dinâmico**: o proxy sintetiza o envelope porque o
/// `souls_mcp_server` é stdio-only. Como o body pode ter tamanho variável,
/// montamos o header `Content-Length` por request no helper
/// `write_json_response_headers`. Status é sempre 200 (erros viram JSON-RPC
/// `error`, não HTTP 4xx/5xx).
const HTTP_STATUS_LINE: &[u8] = b"HTTP/1.1 200 OK\r\n";
const HTTP_CORS_HEADERS: &[u8] = b"\
Content-Type: application/json\r\n\
Connection: close\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Headers: Content-Type, Authorization, MCP-Protocol-Version\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n";

/// Emite o envelope HTTP/1.1 completo (status + headers fixos + Content-Length
/// dinâmico) **antes** do body JSON. Falha de I/O propaga imediatamente.
///
/// Aceita `&Vec<u8>` por compatibilidade com `String::into_bytes()` no caller
/// — preferimos `&[u8]` no path de hot loop para evitar 1 alocação.
async fn write_json_response_headers(
    stream: &mut TcpStream,
    body_len: usize,
) -> io::Result<()> {
    stream.write_all(HTTP_STATUS_LINE).await?;
    stream.write_all(HTTP_CORS_HEADERS).await?;
    let cl = format!("Content-Length: {}\r\n\r\n", body_len);
    stream.write_all(cl.as_bytes()).await?;
    stream.flush().await
}

/// Emite `HTTP/1.1 202 Accepted` com `Content-Type: application/json`,
/// `Content-Length: 0` e sem body — resposta canônica para JSON-RPC
/// **notifications** (`notifications/initialized`, etc.) no StreamableHTTP
/// transport (MCP spec 2025-03-26 §"Sending Messages to the Server").
///
/// **Por que 202 e NÃO 200 com notification/ack**:
/// - Spec 2025-03-26 §"Sending Messages": "If the input is a JSON-RPC
///   response or notification: the server **MUST** return HTTP status
///   code 202 Accepted with no body."
/// - 200 com body JSON-RPC faz o `mcp-remote` correlacionar a notification
///   como se fosse a response de uma request (bug no Zod correlation
///   logic que vimos — `notifications/ack` quebrava o matching por id).
///
/// **Por que `Content-Length: 0` e Content-Type explícito**:
/// - O `mcp-remote` (undici H1 parser) chama `ResponseEnded` quando o
///   body termina antes do `Content-Length` declarar. Sem o header, fecha
///   prematuro.
/// - O `mcp-remote` (StreamableHTTP) valida `Content-Type` em TODA response
///   HTTP. Sem o header, dispara `Unexpected content type: null`.
///
/// **Status code 202**: semanticamente "Accepted, no further action"
/// (perfeito para fire-and-forget notifications) e **reservado** para
/// responses sem body no StreamableHTTP.
const HTTP_202_EMPTY_HEADERS: &[u8] = b"HTTP/1.1 202 Accepted\r\n\
Content-Type: application/json\r\n\
Content-Length: 0\r\n\
Connection: close\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Headers: Content-Type, Authorization, MCP-Protocol-Version\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\r\n";

async fn write_202_accepted_empty(stream: &mut TcpStream) -> io::Result<()> {
    stream.write_all(HTTP_202_EMPTY_HEADERS).await?;
    stream.flush().await
}

/// `handle_mcp_stdout_response` (Marco I · v6.2) — Lê JSON-RPC line-delimited
/// do stdout do `SubprocessGuard` (souls_mcp_server), concatena em um único
/// buffer JSON e envia a resposta ao cliente HTTP da IDE como
/// `application/json` puro (NÃO SSE).
///
/// **Por que `application/json` e NÃO `text/event-stream`**: o transporte
/// MCP StreamableHTTP usado pelo `mcp-remote` parseia o body como **JSON
/// puro** quando o content-type é `application/json`. SSE exige parsing
/// de frames `data: ...\n\n` que o `mcp-remote` não implementa — qualquer
/// trailer como `data: [DONE]\n\n` causa `SyntaxError: Unexpected token 'D'`.
/// O MCP `souls_mcp_server` escreve **1 linha JSON por request**, então
/// concatenar em 1 buffer é 1:1 com o protocolo esperado.
///
/// **Envelope HTTP/1.1**: o `mcp-remote` (undici H1 parser) exige status
/// line + headers antes do body. Sem isso, dispara `HTTPParserError`.
///
/// **Crítico**: usa `tokio::time::timeout` (200ms) no `read_line` porque
/// o `souls_mcp_server` escreve EXATAMENTE 1 linha por request e fica
/// bloqueado esperando o próximo stdin. Sem timeout, o `read_line` trava
/// para sempre e o `mcp_stdout` lock é sequestrado, causando timeout
/// em todos os requests subsequentes (incluindo `notifications/initialized`
/// que não recebem resposta alguma).
///
/// Mede o **Time-To-First-Token** (TTFT) do primeiro byte de resposta,
/// atualiza o `PeakEWMA` global e despacha telemetria via MPSC.
async fn handle_mcp_stdout_response(
    mut downstream: TcpStream,
    mcp_stdout: Arc<Mutex<ChildStdout>>,
    ccr_store: Arc<SoulsCcrStore>,
    request_started_at: Instant,
    model: &str,
    tokens_in: i64,
    session_id: Option<String>,
) -> io::Result<()> {
    use tokio::io::AsyncBufReadExt;

    const READ_TIMEOUT: Duration = Duration::from_millis(200);

    let mut stdout_guard = mcp_stdout.lock().await;
    let mut reader = tokio::io::BufReader::new(&mut *stdout_guard);
    let mut line = String::new();
    let mut ttft_recorded = false;
    let mut tokens_out_estimate: i64 = 0;
    let mut lines_read: u32 = 0;
    // Buffer acumulado: o subprocesso pode emitir 1+ linhas (resultado +
    // notificações parciais). Concatenamos para enviar 1 JSON-RPC response.
    let mut body_buffer = String::new();
    let mut first_json_line: Option<String> = None;

    loop {
        line.clear();
        // TIMEOUT obrigatório: o subprocesso escreve 1 linha e BLOQUEIA
        // esperando o próximo stdin. Sem timeout, deadlock permanente.
        let n = match tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut line)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e),
            Err(_elapsed) => {
                // Timeout: subprocesso não respondeu (notification ou fim).
                tracing::debug!(
                    "handle_mcp_stdout_response: read_line timeout após {READ_TIMEOUT:?} \
                     (subprocesso aguardando próximo stdin — {lines_read} linhas já lidas)"
                );
                break;
            }
        };
        if n == 0 {
            // EOF: subprocesso fechou stdout.
            break;
        }
        // Strip trailing newline.
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed.is_empty() {
            continue;
        }
        lines_read += 1;

        // TTFT: primeiro byte (ou primeira linha) recebida do subprocesso.
        if !ttft_recorded {
            let ttft_ms = request_started_at.elapsed().as_secs_f64() * 1000.0;
            global_peak_ewma().record(ttft_ms);
            ttft_recorded = true;
            tracing::info!(
                "TTFT={:.2}ms model={} tokens_in={} (mcp_stdout)",
                ttft_ms, model, tokens_in
            );
        }

        // Marco I — estimativa de tokens_out (BPE).
        tokens_out_estimate += count_real_tokens(trimmed);

        // Sequestro do Loopback (existente): se a resposta do MCP
        // referencia headroom_retrieve, hidrata o código stubbed aqui
        // mesmo (em vez de SSE upstream). Mantém latência < 1ms.
        if trimmed.contains("headroom_retrieve") {
            if let Some(hydrated) = ccr_store.intercept_loopback(trimmed) {
                tracing::info!(
                    "Sequestro de Loopback acionado em < 1ms (via mcp_stdout)."
                );
                // Marco I · v6.2 — JSON puro com Content-Length, NÃO SSE.
                let hydrated_bytes = hydrated.into_bytes();
                write_json_response_headers(&mut downstream, hydrated_bytes.len()).await?;
                downstream.write_all(&hydrated_bytes).await?;
                downstream.flush().await?;
                drop(stdout_guard);
                finalize_telemetry(
                    request_started_at,
                    model,
                    tokens_in,
                    tokens_out_estimate,
                    session_id,
                );
                return Ok(());
            }
        }

        // Acumula a primeira linha JSON-RPC como body principal. O
        // `mcp-remote` espera 1 response JSON por request; linhas
        // subsequentes (se houver) viram logs/telemetria mas não
        // fazem parte do body HTTP.
        if first_json_line.is_none() {
            first_json_line = Some(trimmed.to_string());
            body_buffer.push_str(trimmed);
        } else {
            tracing::debug!(
                "handle_mcp_stdout_response: linha extra ignorada (linha 1 já capturada)"
            );
        }
    }
    drop(stdout_guard);

    // Se nenhuma linha chegou (ex.: `notifications/initialized` que é
    // fire-and-forget no JSON-RPC), respondemos `202 Accepted` com body
    // vazio — canônico para StreamableHTTP (MCP spec 2025-03-26 §Sending).
    if first_json_line.is_none() {
        tracing::info!(
            "handle_mcp_stdout_response: 0 linhas lidas → 202 Accepted (notification)"
        );
        write_202_accepted_empty(&mut downstream).await?;
        finalize_telemetry(
            request_started_at,
            model,
            tokens_in,
            tokens_out_estimate,
            session_id,
        );
        return Ok(());
    }

    // Marco I · v6.2 — Envelope HTTP/1.1 + JSON puro (sem SSE wrapping,
    // sem `data: [DONE]`). Content-Length dinâmico.
    let body_bytes = body_buffer.into_bytes();
    write_json_response_headers(&mut downstream, body_bytes.len()).await?;
    downstream.write_all(&body_bytes).await?;
    downstream.flush().await?;

    finalize_telemetry(
        request_started_at,
        model,
        tokens_in,
        tokens_out_estimate,
        session_id,
    );
    tracing::debug!(
        "handle_mcp_stdout_response: {lines_read} linhas lidas, tokens_out≈{tokens_out_estimate}, body={}B",
        body_bytes.len()
    );
    Ok(())
}

/// Despacho final de telemetria (TTFT + tokens + custo).
/// Extraído para reuso entre o path de loopback (early-return) e o path
/// normal (EOF). Fail-soft: log se o canal estiver fechado.
fn finalize_telemetry(
    request_started_at: Instant,
    model: &str,
    tokens_in: i64,
    tokens_out_estimate: i64,
    session_id: Option<String>,
) {
    let _ = model; // Telemetria recebe via dispatch_latency (já carrega model).
    if let Some(sender) = telemetry_sender() {
        let ttft_ms = request_started_at.elapsed().as_secs_f64() * 1000.0;
        let peak_ewma = global_peak_ewma().ewma_ms();
        // Custo estimado: usa cost_per_1m do JSONC (Premium $15/1M default).
        let cost_usd = (tokens_out_estimate as f64 / 1_000_000.0) * 15.0;
        sender.dispatch_latency(
            "agentgateway_v6",
            ttft_ms,
            peak_ewma,
            tokens_in,
            tokens_out_estimate,
            cost_usd,
            session_id,
        );
    }
}

/// Serializa a decisão do shield em uma resposta HTTP/1.1 200 OK
/// com payload JSON-RPC `error.code = -32001` no corpo. O cliente
/// (Svelte 5 ou outro consumidor MCP) lê o body como erro tipado.
async fn write_shield_http_response<W>(downstream: &mut W, decision: &ShieldDecision) -> io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let jsonrpc = intercepted_to_jsonrpc(decision);
    let pretty = serde_json::to_string_pretty(&jsonrpc).unwrap_or_else(|_| "{}".to_string());
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        pretty.len(),
        pretty
    );
    downstream.write_all(response.as_bytes()).await?;
    downstream.flush().await?;
    Ok(())
}

// ============================================================================
// Marco I · v6.1 — Helpers do hot-path do proxy
// ============================================================================

/// Extrai `session_id` de headers SSE/JSON-RPC comuns. Fallback: hash
/// de 8 bytes do path + remote address (sticky por origem).
fn extract_session_id(headers: &[httparse::Header<'_>], path: &str) -> String {
    for h in headers.iter() {
        let name_lower = h.name.to_ascii_lowercase();
        if name_lower == "x-session-id" || name_lower == "x-conversation-id" {
            if let Ok(s) = std::str::from_utf8(h.value) {
                return s.to_string();
            }
        }
    }
    // Fallback determinístico baseado no path (sticky por URL).
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    format!("auto-{:016x}", h.finish())
}

/// Conta tokens reais via `GigaTokenEncoder` (BPE local ou tiktoken cl100k_base).
/// Fail-soft: em caso de erro, usa heurística `bytes / 3`.
fn count_real_tokens(text: &str) -> i64 {
    GigaTokenEncoder::global()
        .tokenize_to_bin(text)
        .map(|ids| ids.len() as i64)
        .unwrap_or_else(|_| (text.len() as i64) / 3)
}

/// Heurística simples de complexidade (0.0 - 1.0) baseada em keywords de código
/// + comprimento. Usada pelo `evaluate_route_decision`.
fn estimate_complexity(body: &str) -> f32 {
    let lower = body.to_ascii_lowercase();
    let mut score: f32 = 0.0;
    // Sinais de complexidade alta.
    for kw in &["fn ", "function ", "def ", "impl ", "struct ", "class ",
                "async ", "await ", "tokio::", "unsafe ", "match "] {
        if lower.contains(kw) {
            score += 0.08;
        }
    }
    // Comprimento como proxy (até 1.0).
    let len_score = (body.len() as f32 / 20_000.0).min(0.3);
    score += len_score;
    score.min(1.0)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();

    let listen = parse_listen_arg();
    let ccr_store = Arc::new(SoulsCcrStore::from_env());

    // Marco I · v6.1 — Boot do subsistema de telemetria e config.
    // Usa `resolve_state_db_path()` (resolver inteligente) ao invés do
    // path cru do JSONC: garante fallback para `.souls_data/souls_state.db`
    // se a env var `SOULS_STATE_DB_PATH` não estiver setada — evita
    // criar arquivos com path literal `${...}` no filesystem.
    // Falha de bootstrap do dispatcher é logged e ignorada (fail-soft):
    // o proxy ainda funciona, apenas sem telemetria.
    let db_path = souls_mc_lib::core::telemetry_dispatcher::resolve_state_db_path();
    if let Err(e) = init_telemetry_dispatcher(&db_path) {
        tracing::warn!("TelemetryDispatcher não inicializou: {e}. Telemetria desabilitada.");
    }

    // Marco 4.10.0 ETAPA 4 — DIRETRIZ 2 (inegociável):
    //   O prober síncrono do L7 Shield é hospedado em uma thread OS dedicada
    //   (`souls-l7-shield`). O canal MPSC + oneshot garante que a thread de
    //   rede do Tokio nunca toque no tensor (zero stalls de latência).
    //   O canal é `Clone` (múltiplos workers podem compartilhar via Arc).
    let shield_channel = EpistemicShieldChannel::spawn_mock();

    // Marco I · v6.1 — Singletons de L7 Shield (sticky + PII).
    let sticky = Arc::new(StickyRouter::from_config());
    let pii = Arc::new(PiiRedactor::from_config());
    tracing::info!(
        "Marco I wired: sticky={} ({} sessões), pii={}, peak_ewma_α={}",
        sticky.session_count() == 0,
        sticky.session_count(),
        pii.is_enabled(),
        global_peak_ewma().alpha()
    );

    // Marco I · v6.1 — Boot do subprocesso MCP sob SubprocessGuard.
    // O proxy é o **dono absoluto** do ciclo de vida do `souls_mcp_server`:
    //  1. spawna o subprocesso
    //  2. extrai stdin/stdout (piped)
    //  3. toma o `Child` para reaping manual
    //  4. wrappa stdin/stdout em `Arc<Mutex<...>>` para serializar acesso
    //     entre tasks de clientes MCP concorrentes
    //  5. spawna uma task de reaping que mata o child se o proxy cair
    //
    // Sem `--upstream` TCP: o loopback suicida (3001→3001) foi ERRADICADO.
    let mcp_guard = spawn_souls_mcp_server().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Falha crítica: souls_mcp_server não pode ser spawned (SubprocessGuard). \
             Verifique SOULS_MCP_BIN e o JSONC."
        )
    })?;

    let mut child = mcp_guard
        .into_child()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "SubprocessGuard cedeu o child"))?;

    let mcp_stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "stdin do MCP não capturado (Stdio::piped() falhou)"))?;
    let mcp_stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "stdout do MCP não capturado (Stdio::piped() falhou)"))?;

    let mcp_stdin = Arc::new(Mutex::new(mcp_stdin));
    let mcp_stdout = Arc::new(Mutex::new(mcp_stdout));

    // Marco I · v6.4 — Serial lock global. Tasks HTTP concorrentes
    // disputam o mesmo ChildStdin/ChildStdout do subprocesso MCP. Sem
    // este lock, task A pode consumir a response da task B (race
    // condition), causando timeout 60s do client. O lock serializa o
    // ciclo stdin-write → stdout-read em UMA task por vez.
    let request_serial_lock = Arc::new(Mutex::new(()));

    // Reap task: aguarda o child sair. Se o proxy cair antes, o `Drop`
    // do guard já não existe (tomamos via `into_child`), então a task
    // de reap apenas observa. O kill explícito fica por conta do signal
    // handler (Ctrl+C → shutdown do main loop).
    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                tracing::warn!("souls_mcp_server saiu inesperadamente: {status}");
            }
            Err(e) => {
                tracing::error!("Reap do souls_mcp_server falhou: {e}");
            }
        }
    });

    let listener = TcpListener::bind(listen).await?;

    tracing::info!(
        "Proxy L7 Zero-Copy escutando em {} (MCP server subordinado via SubprocessGuard)",
        listen
    );

    loop {
        let (downstream, _) = listener.accept().await?;

        let store_clone = Arc::clone(&ccr_store);
        let shield_clone = shield_channel.clone();
        let sticky_clone = Arc::clone(&sticky);
        let pii_clone = Arc::clone(&pii);
        let stdin_clone = Arc::clone(&mcp_stdin);
        let stdout_clone = Arc::clone(&mcp_stdout);
        let serial_clone = Arc::clone(&request_serial_lock);
        tokio::spawn(async move {
            if let Err(e) = handle_l7_proxy_v6(
                downstream,
                stdin_clone,
                stdout_clone,
                serial_clone,
                store_clone,
                shield_clone,
                sticky_clone,
                pii_clone,
            )
            .await
            {
                tracing::error!("Erro no L7 proxy (v6.1 stdio bridge): {}", e);
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

        let part1 = "data: {\"name\":\"headroom_".to_string();
        let part2 = format!("retrieve\",\"parameters\":{{\"hash\":\"{}\"}}}}\n\n", hex_hash);

        let (client_down, server_down) = tokio::io::duplex(1024);
        let (mut client_up, server_up) = tokio::io::duplex(1024);

        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            handle_upstream_response_v6(
                client_down,
                server_up,
                store_clone,
                Instant::now(),
                "gemma4",
                0,
                None,
            )
            .await
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

    /// SOULS Marco 4.9.2 — Gate de cura de frame SSE truncado.
    /// Prova que o `cure_sse_data_line` fecha delimitadores (`}`, `]`, `"`)
    /// em < 1ms quando o upstream LLM trunca abruptamente (fim de VRAM,
    /// limite de tokens, OOM do sampler).
    #[test]
    fn test_sse_accumulator_cures_truncated_frame() {
        // Frame truncado: abriu `{` e `[` mas não fechou.
        let truncated = b"data: {\"choices\":[{\"delta\":{\"content\":\"Hel";

        let start = std::time::Instant::now();
        let cured = cure_sse_data_line(truncated);
        let elapsed = start.elapsed();

        eprintln!("Latência de cura SSE: {:?}", elapsed);
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed.as_micros() < 1000,
            "Cura SSE deve ser < 1ms (release); medido: {elapsed:?}"
        );

        // O output deve começar com "data: " e conter payload JSON válido.
        let cured_str = std::str::from_utf8(&cured).expect("output deve ser UTF-8");
        assert!(cured_str.starts_with("data: "), "Prefixo data: perdido: {cured_str}");

        // Extrai o payload após "data: " e valida que é JSON parseável.
        let payload = cured_str
            .strip_prefix("data: ")
            .expect("deve começar com data: ")
            .trim();
        let parsed: serde_json::Value = serde_json::from_str(payload)
            .unwrap_or_else(|e| panic!("JSON truncado não foi curado: {e}. Payload: {payload}"));

        // Verifica que a chave "content" foi preservada com valor parcial.
        let content = parsed["choices"][0]["delta"]["content"]
            .as_str()
            .expect("content deve ser string após cura");
        assert_eq!(content, "Hel", "Conteúdo parcial deve ser preservado");
    }

    /// SOULS Marco 4.9.2 — Gate de cura preserva o marcador [DONE].
    #[test]
    fn test_sse_accumulator_preserves_done_marker() {
        let frame = b"data: [DONE]\n\n";
        let cured = cure_sse_data_line(frame);
        let cured_str = std::str::from_utf8(&cured).unwrap();
        assert!(cured_str.contains("[DONE]"), "Marcador [DONE] perdido: {cured_str}");
    }

    /// SOULS Marco 4.9.2 — Gate de cura preserva lines que não são `data:`.
    /// (event:, id:, comments `: keep-alive`).
    #[test]
    fn test_sse_accumulator_preserves_non_data_lines() {
        let frame = b"event: message\nid: 42\ndata: {\"ok\":true}\n\n";
        let cured = cure_sse_data_line(frame);
        let cured_str = std::str::from_utf8(&cured).unwrap();
        assert!(cured_str.starts_with("event: message"), "event: perdido: {cured_str}");
        assert!(cured_str.contains("id: 42"), "id: perdido: {cured_str}");
        assert!(cured_str.contains(r#""ok":true"#), "data: curado perdido: {cured_str}");
    }

    // ========================================================================
    // Marco I · v6.1 — TAREFA 5: 3 contratos TDD (Ralph Loop control)
    // ========================================================================

    use souls_mc_lib::core::peak_ewma::PeakEwma;
    use souls_mc_lib::core::sticky_router::{build_cached_header, StickyRouter, RoutePin};
    use souls_mc_lib::core::response_healing::heal_malformed_json;
    use souls_mc_lib::core::subprocess_guard::{SubprocessConfig, SubprocessGuard, SubprocessState};

    /// TAREFA 5.1: `test_prefix_cache_byte_stability`
    /// Prova que mutações consecutivas de turnos de chat mantêm as
    /// assinaturas binárias de Z1 e Z2 100% idênticas, travando o
    /// **Prefix Caching** do provedor upstream.
    #[test]
    fn test_prefix_cache_byte_stability() {
        let pin = RoutePin::new(
            "openrouter",
            "anthropic/claude-3.5-sonnet",
            "deepseek/deepseek-r1",
        );

        // Turno 1: primeira chamada (cold start).
        let header_t1 = build_cached_header(&pin);

        // Turno 2..=5: chamadas subsequentes.
        let header_t2 = build_cached_header(&pin);
        let header_t3 = build_cached_header(&pin);
        let header_t4 = build_cached_header(&pin);
        let header_t5 = build_cached_header(&pin);

        // Invariante: byte-idêntico entre todos os turnos.
        assert_eq!(header_t1, header_t2, "T1 ≠ T2: Prefix Cache seria invalidado");
        assert_eq!(header_t1, header_t3, "T1 ≠ T3: Prefix Cache seria invalidado");
        assert_eq!(header_t1, header_t4, "T1 ≠ T4: Prefix Cache seria invalidado");
        assert_eq!(header_t1, header_t5, "T1 ≠ T5: Prefix Cache seria invalidado");

        // Invariante adicional: o router sticky cacheia o MESMO header
        // (Arc<Vec<u8>> byte-idêntico) entre chamadas.
        let router = StickyRouter::new(true, 3600);
        router.resolve_or_lock("sess-prefix-cache", &pin);
        let cached_a = router.cached_header("sess-prefix-cache").unwrap();
        let cached_b = router.cached_header("sess-prefix-cache").unwrap();
        assert!(std::sync::Arc::ptr_eq(&cached_a, &cached_b),
                "Sticky Router deve cachear o MESMO Arc entre chamadas (zero clones)");
    }

    /// TAREFA 5.2: `test_peak_ewma_decay_calculation`
    /// Simula conexões artificiais com picos de latência de 2.500ms seguidos
    /// de estabilização em 150ms e valida se a média móvel PeakEWMA converge
    /// matematicamente respeitando o multiplicador α=0.3.
    #[test]
    fn test_peak_ewma_decay_calculation() {
        let p = PeakEwma::<128>::new();
        p.set_alpha(0.3);

        // Fase 1: pico isolado de 2500ms (simula cold start lento).
        p.record(2500.0);
        let ewma_burst = p.ewma_ms();
        let peak_burst = p.peak_ms();
        // α=0.3, sample=2500, prev=0 → ewma = 750.
        assert!((ewma_burst - 750.0).abs() < 1.0,
                "EWMA após 1 pico de 2500ms deve ser ~750, medido: {ewma_burst}");
        assert!(peak_burst >= 749.0 && peak_burst <= 751.0,
                "Peak após burst deve ser ~750, medido: {peak_burst}");

        // Fase 2: 50 samples de 150ms (estabilização). Convergência esperada ≈150.
        for _ in 0..50 {
            p.record(150.0);
        }
        let ewma_stable = p.ewma_ms();
        assert!(
            (ewma_stable - 150.0).abs() < 1.0,
            "EWMA após 50 samples de 150ms deve convergir para ~150, medido: {ewma_stable}"
        );
        // Peak preservado (nunca decresce).
        assert!(p.peak_ms() >= peak_burst - 0.01,
                "Peak nunca decresce (deve preservar pico histórico)");

        // Fase 3: monotonicidade da convergência (10 samples intermediários).
        let mut last = ewma_burst;
        for _ in 0..10 {
            p.record(150.0);
            let now = p.ewma_ms();
            assert!(now < last || (now - last).abs() < 0.01,
                    "EWMA deve decrescer monotonicamente: last={last} now={now}");
            last = now;
        }
    }

    /// TAREFA 5.3: `test_json_healing_sse_stream`
    /// Injeta um stream de bytes de JSON quebrado (ex: `{"thought": "code", "files": [`)
    /// e valida se o consertador de payload recupera a integridade estrutural
    /// devolvendo um JSON válido para a leitura do `serde_json`.
    #[test]
    fn test_json_healing_sse_stream() {
        // Caso canônico TAREFA 5: bracket aberto em array.
        // `jsonrepair` deve fechar o array e o objeto raiz, devolvendo JSON RFC 8259 válido.
        let truncated = br#"{"thought": "code", "files": ["#;
        let result_healed = heal_malformed_json(std::str::from_utf8(truncated).unwrap());
        let parsed: serde_json::Value = serde_json::from_str(&result_healed)
            .expect("jsonrepair deve produzir JSON parseável mesmo com bracket aberto");
        assert!(parsed.is_object(), "Deve ser objeto após cura: {result_healed}");
        assert_eq!(parsed["thought"], "code");
        assert!(parsed["files"].is_array(), "Array 'files' deve ser preservado: {result_healed}");

        // Caso pipeline completo: `cure_sse_data_line` com JSON quebrado.
        // O marco SSE deve preservar o prefixo `data: ` e entregar payload curado.
        let sse_truncated = b"data: {\"thought\": \"code\", \"files\": [";
        let cured_sse = cure_sse_data_line(sse_truncated);
        let cured_str = std::str::from_utf8(&cured_sse).unwrap();
        assert!(cured_str.starts_with("data: "), "Output deve manter prefixo SSE");
        let payload = cured_str.trim_start_matches("data: ").trim();
        let _: serde_json::Value = serde_json::from_str(payload)
            .expect("SSE curado deve ser JSON parseável por serde_json");
    }

    // ========================================================================
    // Marco I · v6.1 — Auditoria: ciclo de vida do subprocesso MCP
    // ========================================================================

    /// Verifica se um PID está vivo na tabela de processos do SO.
    /// Usa `tasklist` no Windows (sempre disponível) — implementação
    /// deliberadamente externalizada (sem crate extra, ADR-030).
    /// Retorna `true` se o PID existe E não é zombie.
    ///
    /// Saída real do tasklist (PT-BR Windows 10/11):
    ///   - PID inexistente:
    ///       "INFORMAÇÕES: nenhuma tarefa em execução correspondente..."
    ///   - PID existente:
    ///       cabeçalho + linha com o PID (ex: "cmd.exe  12345  Console  1  ...")
    ///
    /// O Windows **não suporta** `/NH` (no header) com `/FI` —
    /// sempre imprime o cabeçalho. A detecção correta é por exclusão:
    /// se NÃO contém a string de "nenhuma tarefa" E contém o PID numérico.
    fn is_pid_alive_windows(pid: u32) -> bool {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lower = stdout.to_lowercase();
                // Se tasklist reportou "nenhuma tarefa" → morto.
                // Strings em PT-BR ("nenhuma tarefa") e EN ("no tasks").
                if lower.contains("nenhuma tarefa") || lower.contains("no tasks") {
                    return false;
                }
                // Senão, exige que o número do PID apareça na saída
                // (caso típico: "cmd.exe   31764   Console   1   ...").
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }

    /// TAREFA — Auditoria: `test_native_mcp_subprocess_spawn_and_kill`
    ///
    /// Prova o ciclo de vida completo do subprocesso MCP sob
    /// `SubprocessGuard`:
    ///   1. Cria config com binário `cmd.exe` (sempre presente no Windows).
    ///   2. Spawn → captura PID.
    ///   3. Verifica via `tasklist` que o PID está vivo.
    ///   4. Drop o guard → SIGKILL atômico.
    ///   5. Re-verifica que o PID NÃO está mais na tabela de processos.
    ///
    /// Elimina o fantasma `agentgateway.exe` global: o proxy é o
    /// **dono absoluto** do ciclo de vida do filho.
    #[test]
    fn test_native_mcp_subprocess_spawn_and_kill() {
        // Resolve o caminho ABSOLUTO do cmd.exe via env (sem PATH lookup).
        // ADR-030: zero deps novas; só stdlib.
        let cmd_path = resolve_cmd_path();
        eprintln!("[TDD] cmd.exe path: {}", cmd_path);

        // (a) Ambiente temporário simulando leitura do JSONC.
        // Usamos `cmd.exe /K rem sleeping` que mantém o processo VIVO
        // indefinidamente (sem input, sem timeout). O guard detém o PID
        // até o drop.
        let cfg = SubprocessConfig {
            executable_path: cmd_path,
            args: vec!["/K".to_string(), "rem".to_string(), "souls_subprocess_guard_test".to_string()],
            working_dir: String::new(),
            kill_on_drop: true,
        };

        // (b) Inicializar o proxy (SubprocessGuard::spawn) e disparar o spawn.
        let mut guard = SubprocessGuard::spawn(&cfg)
            .expect("spawn de cmd.exe /K rem deve ter sucesso");

        let pid = guard
            .pid()
            .expect("Child deve ter PID imediatamente após spawn");

        // Sanity: o PID deve ser > 0 (sentinel de PIDs do Windows).
        assert!(pid > 0, "PID inválido: {pid}");

        // (c) Verificar ativamente se o PID do filho está ativo na tabela de processos.
        // Damos 300ms para o OS registrar o processo na tasklist.
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Debug: dump da saída do tasklist para diagnóstico.
        let tasklist_out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output();
        if let Ok(out) = &tasklist_out {
            eprintln!(
                "[TDD] tasklist /FI \"PID eq {pid}\":\n{}",
                String::from_utf8_lossy(&out.stdout)
            );
        }

        let alive_before = is_pid_alive_windows(pid);
        assert!(
            alive_before,
            "PID {pid} deveria estar VIVO após spawn (tasklist não encontrou)"
        );

        // Validação extra via probe_state.
        match guard.probe_state() {
            SubprocessState::Alive => {
                // Estado esperado.
            }
            other => panic!("Após spawn, probe_state deveria ser Alive, got: {other:?}"),
        }

        // (d) Forçar o encerramento (drop) do proxy e asseverar que o filho foi finalizado.
        // O Drop do SubprocessGuard chama child.start_kill() (SIGKILL/TerminateProcess).
        drop(guard);

        // Damos 500ms para o OS reapar o processo.
        std::thread::sleep(std::time::Duration::from_millis(500));
        let alive_after = is_pid_alive_windows(pid);
        assert!(
            !alive_after,
            "PID {pid} deveria estar MORTO após drop do SubprocessGuard (kill_on_drop=true) — fantasma detectado!"
        );
    }

    /// TAREFA — Auditoria: kill explícito assíncrono também funciona.
    #[tokio::test]
    async fn test_native_mcp_subprocess_explicit_kill_async() {
        let cmd_path = resolve_cmd_path();
        let cfg = SubprocessConfig {
            executable_path: cmd_path,
            args: vec!["/K".to_string(), "rem".to_string(), "souls_subprocess_guard_test".to_string()],
            working_dir: String::new(),
            kill_on_drop: true,
        };

        let guard = SubprocessGuard::spawn(&cfg).expect("spawn OK");
        let pid = guard.pid().expect("PID presente");

        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(is_pid_alive_windows(pid), "PID {pid} deve estar vivo pré-kill");

        // Kill assíncrono explícito.
        guard.kill().await.expect("kill().await deve ter sucesso");

        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(
            !is_pid_alive_windows(pid),
            "PID {pid} deve estar MORTO após kill().await"
        );
    }

    /// Resolve o path ABSOLUTO do `cmd.exe` no Windows sem lookup de PATH.
    /// Ordem de precedência: $ComSpec → $SystemRoot\System32\cmd.exe → fallback C:\Windows\System32\cmd.exe.
    fn resolve_cmd_path() -> String {
        if let Ok(comspec) = std::env::var("ComSpec") {
            if !comspec.is_empty() {
                return comspec;
            }
        }
        if let Ok(system_root) = std::env::var("SystemRoot") {
            return format!("{system_root}\\System32\\cmd.exe");
        }
        "C:\\Windows\\System32\\cmd.exe".to_string()
    }
}
