//! SOULS MC — Zero-Copy IPC Bridge (Wry <-> Svelte 5 Runes)
//!
//! Roteia mensagens recebidas via `window.ipc.postMessage` para o `souls_core`
//! e despacha eventos assíncronos de volta para o Svelte 5 via `evaluate_script`.

use std::sync::Arc;
use souls_mc_lib::CoreEngine;
use souls_protocol::{BackendResponse, FrontendCommand, IpcEnvelope};
use tokio::sync::mpsc;

pub struct IpcBridge {
    engine: Arc<CoreEngine>,
}

impl IpcBridge {
    pub fn new(engine: Arc<CoreEngine>) -> Self {
        Self { engine }
    }

    /// Processa mensagem textual recebida do WebView
    pub fn handle_incoming(&self, raw_message: &str, webview_proxy: &WebViewProxy) {
        let raw = raw_message.to_string();
        let engine = self.engine.clone();
        let proxy = webview_proxy.clone();

        tokio::spawn(async move {
            // Tenta deserializar como IpcEnvelope ou comando direto
            if let Ok(envelope) = serde_json::from_str::<IpcEnvelope>(&raw) {
                let cmd: Result<FrontendCommand, _> = serde_json::from_value(envelope.payload.clone());
                let response = match cmd {
                    Ok(command) => engine.handle_command(command).await,
                    Err(err) => BackendResponse::Error {
                        code: "INVALID_PAYLOAD".to_string(),
                        message: err.to_string(),
                    },
                };

                let resp_json = serde_json::to_string(&response).unwrap_or_default();
                let js_dispatch = format!(
                    "if (window.__SOULS_DISPATCH__) {{ window.__SOULS_DISPATCH__('{}', {}); }}",
                    envelope.id, resp_json
                );
                proxy.dispatch_script(&js_dispatch);
            } else if let Ok(cmd) = serde_json::from_str::<FrontendCommand>(&raw) {
                let response = engine.handle_command(cmd).await;
                let resp_json = serde_json::to_string(&response).unwrap_or_default();
                let js_dispatch = format!(
                    "if (window.__SOULS_EVENT__) {{ window.__SOULS_EVENT__('response', {}); }}",
                    resp_json
                );
                proxy.dispatch_script(&js_dispatch);
            } else {
                tracing::warn!("[souls_ui_shell::ipc] Mensagem com formato inválido: {}", raw);
            }
        });
    }

    /// Inicia o ouvinte de eventos do CoreEngine para enviar para a WebView
    pub fn spawn_event_listener(
        mut rx: mpsc::UnboundedReceiver<IpcEnvelope>,
        proxy: WebViewProxy,
    ) {
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let channel = event.channel;
                let payload_json = serde_json::to_string(&event.payload).unwrap_or_default();
                let js_dispatch = format!(
                    "if (window.__SOULS_EVENT__) {{ window.__SOULS_EVENT__('{}', {}); }}",
                    channel, payload_json
                );
                proxy.dispatch_script(&js_dispatch);
            }
        });
    }
}

/// Proxy para executar scripts na WebView através de canais seguros
#[derive(Clone)]
pub struct WebViewProxy {
    script_tx: mpsc::UnboundedSender<String>,
}

impl WebViewProxy {
    pub fn new(script_tx: mpsc::UnboundedSender<String>) -> Self {
        Self { script_tx }
    }

    pub fn dispatch_script(&self, script: &str) {
        let _ = self.script_tx.send(script.to_string());
    }
}
