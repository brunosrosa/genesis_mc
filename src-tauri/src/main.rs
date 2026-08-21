// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::ipc::Channel;
use tauri::Manager;

/// `souls_ping` — health check trivial.
#[tauri::command]
fn souls_ping(payload: &str) -> String {
    format!("Souls MC Core Online. Recebido: {}", payload)
}

// =============================================================================
// SOULS MC Marco V — SODA Canvas v0.1: Tauri v2 IPC Zero-Copy Telemetry Bridge.
//
// `start_watchdog_stream` recebe um `Channel<Vec<u8>>` do frontend (Svelte 5)
// e o injeta no `watchdog_ipc::spawn_watchdog_channel`. O canal emite
// EXATAMENTE 8 bytes (u64 packed little-endian) a cada 1Hz — sem JSON.
//
// O frontend chama via:
//   const channel = new Channel<Uint8Array>();
//   await invoke('start_watchdog_stream', { channel });
// =============================================================================
#[tauri::command]
async fn start_watchdog_stream(app: tauri::AppHandle, channel: Channel<Vec<u8>>) {
    souls_mc_lib::telemetry::watchdog_ipc::spawn_watchdog_channel(app, channel);
}

// =============================================================================
// SOULS-CANIBALIZED Marco 3.9 Fase E.2: Tauri v2 IPC commands para Svelte 5.
//
// Padrão: `Result<Value, String>` na fronteira (zero-cost: Svelte 5 captura
// `invoke().then().catch()` graciosamente sem travar o renderer).
// As 3 funções delegam 100% para `souls_mc_lib::cognition::thinking::handlers`
// (canibalização: 0 lógica de negócios no main.rs, conforme ADR-005 §Frontend
// Passivo).
// =============================================================================

/// `socratic_export_session` — exporta árvore de pensamentos socráticos.
#[tauri::command]
async fn socratic_export_session(
    session_id: String,
    format: Option<String>,
) -> Result<serde_json::Value, String> {
    souls_mc_lib::cognition::thinking::handlers::handle_export_session(
        &session_id,
        format.as_deref(),
        None,
    )
    .map_err(|e| format!("export_session falhou: {e}"))
}

/// `socratic_analyze_session` — métricas FinOps cognitivas por sessão.
#[tauri::command]
async fn socratic_analyze_session(session_id: String) -> Result<serde_json::Value, String> {
    souls_mc_lib::cognition::thinking::handlers::handle_analyze_session(&session_id, None)
        .map_err(|e| format!("analyze_session falhou: {e}"))
}

/// `socratic_merge_sessions` — fusão atômica via barramento MPSC HIPER-FORWARD.
#[tauri::command]
async fn socratic_merge_sessions(
    source_session_id: String,
    target_session_id: String,
) -> Result<serde_json::Value, String> {
    souls_mc_lib::cognition::thinking::handlers::handle_merge_sessions(
        &source_session_id,
        &target_session_id,
        None,
        None, // síncrono por padrão (Tauri frontend prefere transação explícita)
    )
    .map_err(|e| format!("merge_sessions falhou: {e}"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            souls_ping,
            start_watchdog_stream,
            socratic_export_session,
            socratic_analyze_session,
            socratic_merge_sessions,
        ])
        .setup(|app| {
            // souls_mc é puramente um tray daemon (Tauri TrayIconBuilder).
            // O ciclo de vida do proxy L7 é de responsabilidade do `boot.ps1`
            // (orquestrador central), que já o inicia com guarda de expurgo
            // no step 1 + spawn soberano no step 6. **NÃO** spawnar nenhum
            // binário aqui — qualquer tentativa de supervisão interna
            // violaria a arquitetura desacoplada.
            //
            // ADR-005: zero lógica de negócios no frontend (Tauri é passivo).

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            let toggle_overlay = tauri::menu::MenuItem::with_id(
                app,
                "toggle_overlay",
                "Toggle Overlay",
                true,
                None::<&str>,
            )?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Sair / Quit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&toggle_overlay, &quit])?;

            let icon = app.default_window_icon().cloned().unwrap();
            tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Souls MC • Online")
                .menu(&menu)
                .on_menu_event(|app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id().as_ref() {
                    "toggle_overlay" => toggle_overlay_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event: tauri::tray::TrayIconEvent| {
                    if let tauri::tray::TrayIconEvent::DoubleClick {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        toggle_overlay_window(tray.app_handle());
                    }
                })
                .build(app)?;

            #[cfg(target_os = "windows")]
            spawn_global_shortcut_listener(app.handle().clone());

            // OPERAÇÃO NERVO ÓPTICO: Inicialização dos emissores de alta performance Tauri v2 IPC
            let telemetry_sink = std::sync::Arc::new(souls_mc_lib::core::ipc_bridge::TauriTelemetrySink::new(app.handle().clone()));
            souls_mc_lib::core::ipc_bridge::set_telemetry_sink(telemetry_sink.clone());
            let bridge = souls_mc_lib::core::ipc_bridge::WatchdogIpcBridge::from_hz(5);
            bridge.spawn(telemetry_sink);

            let thought_sink = std::sync::Arc::new(souls_mc_lib::core::socratic_thought_stream::TauriSocraticThoughtSink::new(app.handle().clone()));
            souls_mc_lib::core::socratic_thought_stream::init_global_thought_broadcaster(thought_sink);

            let terminal_sink = std::sync::Arc::new(souls_mc_lib::core::terminal_drawer_stream::TauriTerminalStreamSink::new(app.handle().clone()));
            souls_mc_lib::core::terminal_drawer_stream::init_global_terminal_batcher(terminal_sink);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("erro ao rodar a aplicação tauri");
}

#[cfg(target_os = "windows")]
fn spawn_global_shortcut_listener(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            RegisterHotKey, MOD_ALT, MOD_NOREPEAT, VK_SPACE,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};
        use tauri::Emitter;

        const HOTKEY_ID: i32 = 0x50DA; // SODA Alt+Space Hotkey ID
        unsafe {
            let registered = RegisterHotKey(
                0 as _,
                HOTKEY_ID,
                MOD_ALT | MOD_NOREPEAT,
                VK_SPACE as u32,
            );
            if registered == 0 {
                let _ = RegisterHotKey(0 as _, HOTKEY_ID, MOD_ALT, VK_SPACE as u32);
            }

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg as *mut _, 0 as _, 0, 0) > 0 {
                if msg.message == WM_HOTKEY {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                        let _ = window.emit("toggle-spotlight", ());
                    }
                }
            }
        }
    });
}

fn toggle_overlay_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg(test)]
mod tests {
    const MAIN_SOURCE: &str = include_str!("main.rs");

    fn production_source() -> &'static str {
        MAIN_SOURCE
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(MAIN_SOURCE)
    }

    /// TDD anti-regressão: o `souls_mc` (tray daemon) **NUNCA** deve tentar
    /// spawnar o binário zumbi `agentgateway.exe` (eliminado no Marco I em
    /// favor do `agentgateway_tcp_proxy.exe`). O ciclo de vida do proxy é
    /// de responsabilidade exclusiva do `boot.ps1` (orquestrador central).
    ///
    /// Qualquer regressão aqui reintroduziria o panic em
    /// `src\main.rs:338:25` que vimos no log:
    ///   `Falha crítica persistente no processo agentgateway.exe`
    #[test]
    fn main_rs_does_not_reference_agentgateway_phantom() {
        let source = production_source();
        assert!(
            !source.contains("agentgateway.exe"),
            "REGRESSÃO: `agentgateway.exe` (fantasma) foi reintroduzido no main.rs. \
             O ciclo de vida do proxy L7 é exclusivo do `boot.ps1` (Marco I)."
        );
        assert!(
            !source.contains("ProgramSpec::global(\"agentgateway.exe\")"),
            "REGRESSÃO: spawn supervisionado do fantasma `agentgateway.exe` reintroduzido."
        );
    }

    /// TDD anti-regressão: o `souls_mc` **NUNCA** deve tentar spawnar o
    /// `agentgateway_tcp_proxy.exe` internamente (o `boot.ps1` step 6 já
    /// é o dono soberano do spawn do proxy em :3000).
    #[test]
    fn main_rs_does_not_spawn_tcp_proxy() {
        let source = production_source();
        assert!(
            !source.contains("spawn_supervised"),
            "REGRESSÃO: `spawn_supervised` foi reintroduzido. O `souls_mc` é \
             puramente um tray daemon; a orquestração é 100% do `boot.ps1`."
        );
        assert!(
            !source.contains("agentgateway_tcp_proxy.exe"),
            "REGRESSÃO: spawn interno do `agentgateway_tcp_proxy.exe` reintroduzido. \
             Use `boot.ps1` step 6 em vez disso."
        );
    }

    /// TDD anti-regressão: as structs mortas (`ProgramSpec`, `Supervisor`,
    /// `SupervisedProcess`) não devem voltar. Sua presença é um marcador
    /// de que alguém tentou reintroduzir supervisão interna.
    #[test]
    fn main_rs_does_not_contain_dead_supervisor_infra() {
        let source = production_source();
        for dead_symbol in [
            "struct ProgramSpec",
            "struct Supervisor",
            "struct SupervisedProcess",
            "fn ensure_program_path_exists",
            "fn build_dynamic_path",
            "fn get_souls_essential_paths",
            "fn resolve_tcp_proxy_path",
        ] {
            assert!(
                !source.contains(dead_symbol),
                "REGRESSÃO: símbolo morto `{dead_symbol}` reintroduzido. \
                 A supervisão foi externalizada para `boot.ps1` (Marco I)."
            );
        }
    }
}
