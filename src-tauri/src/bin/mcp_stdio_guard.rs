use std::io;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tracing;

const HARD_LIMIT_TIMEOUT_MS: u64 = 30_000;

struct ProcessGuard {
    child: Option<Child>,
}

impl ProcessGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}

#[derive(Clone)]
struct GuardConfig {
    timeout: Duration,
    cmd: String,
    args: Vec<String>,
}

fn parse_cli_args() -> Result<GuardConfig, String> {
    let mut args = std::env::args();
    args.next();

    let mut timeout_ms = HARD_LIMIT_TIMEOUT_MS;
    let mut saw_sep = false;
    let mut cmd = None;
    let mut cmd_args = Vec::new();

    while let Some(arg) = args.next() {
        if saw_sep {
            if cmd.is_none() {
                cmd = Some(arg);
            } else {
                cmd_args.push(arg);
            }
            continue;
        }

        match arg.as_str() {
            "--timeout-ms" => {
                let v = args
                    .next()
                    .ok_or_else(|| "Missing value for --timeout-ms".to_string())?;
                timeout_ms = v
                    .parse::<u64>()
                    .map_err(|_| "Invalid value for --timeout-ms".to_string())?;
            }
            "--" => saw_sep = true,
            _ => {}
        }
    }

    let cmd = cmd.ok_or_else(|| "Missing command after --".to_string())?;
    let timeout_ms = timeout_ms.clamp(1, HARD_LIMIT_TIMEOUT_MS);

    Ok(GuardConfig {
        timeout: Duration::from_millis(timeout_ms),
        cmd,
        args: cmd_args,
    })
}

fn parse_id(v: &Value) -> Option<Value> {
    match v {
        Value::Number(n) => Some(Value::Number(n.clone())),
        Value::String(s) => Some(Value::String(s.clone())),
        _ => None,
    }
}

fn is_notification(v: &Value) -> bool {
    let Some(method) = v.get("method").and_then(|m| m.as_str()) else {
        return false;
    };
    method.starts_with("notifications/")
}

fn jsonrpc_timeout_error(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32000,
            "message": "Timeout da ferramenta: processo muito longo"
        }
    })
}

async fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let pid = pid.to_string();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new("taskkill")
                .args(["/T", "/F", "/PID", &pid])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        })
        .await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let pid = pid.to_string();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        })
        .await;
    }
}

async fn spawn_child(cfg: &GuardConfig) -> Result<Child, String> {
    let mut cmd = Command::new(&cfg.cmd);
    cmd.args(&cfg.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    cmd.spawn()
        .map_err(|e| format!("Falha ao spawnar servidor MCP (stdio): {e}"))
}

async fn write_line<W: AsyncWriteExt + Unpin>(w: &mut W, line: &str) -> Result<(), String> {
    w.write_all(line.as_bytes())
        .await
        .map_err(|e| format!("Falha ao escrever stdout: {e}"))?;
    w.write_all(b"\n")
        .await
        .map_err(|e| format!("Falha ao escrever stdout: {e}"))?;
    w.flush()
        .await
        .map_err(|e| format!("Falha ao flush stdout: {e}"))?;
    Ok(())
}

async fn write_json<W: AsyncWriteExt + Unpin>(w: &mut W, v: &Value) -> Result<(), String> {
    let line = serde_json::to_string(v).map_err(|e| format!("Falha ao serializar JSON: {e}"))?;
    write_line(w, &line).await
}

async fn forward_raw_to_child(
    child_stdin: &mut tokio::process::ChildStdin,
    line: &str,
) -> Result<(), String> {
    child_stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| format!("Falha ao escrever stdin do child: {e}"))?;
    child_stdin
        .write_all(b"\n")
        .await
        .map_err(|e| format!("Falha ao escrever stdin do child: {e}"))?;
    child_stdin
        .flush()
        .await
        .map_err(|e| format!("Falha ao flush stdin do child: {e}"))?;
    Ok(())
}

async fn wait_for_response_and_forward_others<R, W>(
    stdout_lines: &mut tokio::io::Lines<R>,
    expected_id: &Value,
    out: &mut W,
    timeout: Duration,
) -> Result<String, String>
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let fut = async {
        loop {
            let Some(line) = stdout_lines
                .next_line()
                .await
                .map_err(|e| format!("Falha ao ler stdout do child: {e}"))?
            else {
                return Err("EOF inesperado do servidor MCP (stdio)".to_string());
            };

            let parsed = serde_json::from_str::<Value>(&line).ok();
            if let Some(v) = parsed {
                if let Some(id) = v.get("id") {
                    if id == expected_id {
                        return Ok(line);
                    }
                }
            }

            write_line(out, &line).await?;
        }
    };

    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| "timeout".to_string())?
}

async fn run_guarded_session<R, W>(
    cfg: GuardConfig,
    input: R,
    out: &mut W,
) -> Result<(), String>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let child = spawn_child(&cfg).await?;
    let mut child_guard = ProcessGuard::new(child);

    let mut child_stdin = child_guard.child.as_mut().unwrap()
        .stdin
        .take()
        .ok_or_else(|| "stdin indisponível no child".to_string())?;
    let child_stdout = child_guard.child.as_mut().unwrap()
        .stdout
        .take()
        .ok_or_else(|| "stdout indisponível no child".to_string())?;
    let mut stdout_lines = BufReader::new(child_stdout).lines();

    let mut stdin_lines = BufReader::new(input).lines();

    while let Some(line) = stdin_lines
        .next_line()
        .await
        .map_err(|e| format!("Falha ao ler stdin: {e}"))?
    {
        let parsed = serde_json::from_str::<Value>(&line).ok();
        let (id, is_notif) = if let Some(v) = parsed.as_ref() {
            (v.get("id").and_then(parse_id), is_notification(v))
        } else {
            (None, false)
        };

        forward_raw_to_child(&mut child_stdin, &line).await?;

        let Some(id) = id else {
            continue;
        };
        if is_notif {
            continue;
        }

        let res = wait_for_response_and_forward_others(&mut stdout_lines, &id, out, cfg.timeout).await;
        match res {
            Ok(resp_line) => {
                write_line(out, &resp_line).await?;
            }
            Err(e) if e == "timeout" => {
                tracing::error!("Timeout da ferramenta MCP acionado");
                if let Some(pid) = child_guard.child.as_ref().unwrap().id() {
                    kill_process_tree(pid).await;
                } else {
                    let _ = child_guard.child.as_mut().unwrap().kill().await;
                }
                let _ = child_guard.child.as_mut().unwrap().wait().await;

                write_json(out, &jsonrpc_timeout_error(id)).await?;

                let new_child = spawn_child(&cfg).await?;
                child_guard = ProcessGuard::new(new_child);
                child_stdin = child_guard.child.as_mut().unwrap()
                    .stdin
                    .take()
                    .ok_or_else(|| "stdin indisponível no child".to_string())?;
                let child_stdout = child_guard.child.as_mut().unwrap()
                    .stdout
                    .take()
                    .ok_or_else(|| "stdout indisponível no child".to_string())?;
                stdout_lines = BufReader::new(child_stdout).lines();
            }
            Err(e) => return Err(e),
        }
    }

    if let Some(pid) = child_guard.child.as_ref().unwrap().id() {
        kill_process_tree(pid).await;
    } else {
        let _ = child_guard.child.as_mut().unwrap().kill().await;
    }
    let _ = child_guard.child.as_mut().unwrap().wait().await;
    let _ = child_guard.child.take(); // Desarma o ProcessGuard
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = parse_cli_args().map_err(io::Error::other)?;
    let mut stdout = tokio::io::stdout();
    let stdin = tokio::io::stdin();
    run_guarded_session(cfg, stdin, &mut stdout)
        .await
        .map_err(io::Error::other)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, AsyncReadExt};

    #[tokio::test]
    async fn guard_times_out_and_returns_jsonrpc_error_and_kills_child() {
        let cfg = GuardConfig {
            timeout: Duration::from_millis(120),
            cmd: "powershell.exe".to_string(),
            args: vec![
                "-NoLogo".to_string(),
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                "while ($true) { Start-Sleep -Seconds 60 }".to_string(),
            ],
        };

        let (mut w_in, r_in) = duplex(4096);
        let (mut w_out, mut r_out) = duplex(4096);

        let run = tokio::spawn(async move { run_guarded_session(cfg, r_in, &mut w_out).await });

        let req = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": { "name": "webcrawl_scrape", "arguments": { "url": "https://example.com" } }
        });
        w_in
            .write_all(format!("{}\n", req).as_bytes())
            .await
            .unwrap();

        let mut buf = vec![0u8; 4096];
        let n = tokio::time::timeout(Duration::from_secs(2), r_out.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0);
        let text = String::from_utf8_lossy(&buf[..n]);
        assert!(text.contains("\"id\":7"));
        assert!(text.contains("Timeout da ferramenta"));

        drop(w_in);
        let _ = tokio::time::timeout(Duration::from_secs(2), run).await.unwrap();
    }
}
