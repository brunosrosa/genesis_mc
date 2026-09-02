// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use souls_mc_lib::ipc::commands::{socratic, telemetry};

// Símbolos de driver para forçar o uso exclusivo da iGPU (GPU integrada)
// e blindar a dGPU (RTX 2060m / 6GB VRAM) para os modelos locais (CUDA/Candle/mistral.rs).
#[no_mangle]
pub static NvOptimusEnablement: u32 = 0x00000000;
#[no_mangle]
pub static AmdPowerXpressRequestHighPerformance: i32 = 0;

#[cfg(target_os = "windows")]
fn enforce_integrated_gpu_preference() {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_str) = exe_path.to_str() {
            let _ = std::process::Command::new("reg")
                .args(&[
                    "add",
                    "HKCU\\Software\\Microsoft\\DirectX\\UserGpuPreferences",
                    "/v",
                    exe_str,
                    "/t",
                    "REG_SZ",
                    "/d",
                    "GpuPreference=1;",
                    "/f",
                ])
                .output();
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "--force_low_power_gpu --enable-low-power-gpu --enable-features=Calculators,BackgroundTabThrottling,IntensiveWakeUpThrottling,ThrottleDisplayNoneAndVisibilityHiddenCrossOriginIframes --disable-backgrounding-occluded-windows=false",
        );
        enforce_integrated_gpu_preference();
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            telemetry::souls_ping,
            telemetry::start_watchdog_stream,
            socratic::socratic_export_session,
            socratic::socratic_analyze_session,
            socratic::socratic_merge_sessions,
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

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let bridge = souls_mc_lib::core::ipc_bridge::WatchdogIpcBridge::from_hz(5);
                bridge.spawn(telemetry_sink);

                let thought_sink = std::sync::Arc::new(souls_mc_lib::core::socratic_thought_stream::TauriSocraticThoughtSink::new(app_handle.clone()));
                souls_mc_lib::core::socratic_thought_stream::init_global_thought_broadcaster(thought_sink);

                let terminal_sink = std::sync::Arc::new(souls_mc_lib::core::terminal_drawer_stream::TauriTerminalStreamSink::new(app_handle.clone()));
                souls_mc_lib::core::terminal_drawer_stream::init_global_terminal_batcher(terminal_sink);
            });

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
