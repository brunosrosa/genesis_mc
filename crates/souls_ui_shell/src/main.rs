// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use souls_mc_lib::CoreEngine;
use souls_protocol::IpcEnvelope;
use souls_ui_shell::{
    apply_native_dwm_acrylic, auto_deactivate_caps_lock, register_global_hotkey,
    unregister_global_hotkey, ComApartmentGuard, IpcBridge, SuspensionController, WebViewProxy,
};
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use wry::{dpi::{PhysicalPosition, PhysicalSize, Position, Size}, Rect, WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;

struct SoulsApp {
    window: Option<Window>,
    webview: Option<WebView>,
    engine: Option<Arc<CoreEngine>>,
    script_rx: Option<mpsc::UnboundedReceiver<String>>,
    suspension: SuspensionController,
    is_visible: bool,
}

impl SoulsApp {
    fn new() -> Self {
        Self {
            window: None,
            webview: None,
            engine: None,
            script_rx: None,
            suspension: SuspensionController::new(),
            is_visible: false,
        }
    }
}

impl ApplicationHandler for SoulsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // 1. Criar a janela do Winit
        let window_attributes = Window::default_attributes()
            .with_title("SOULS MC // SODA MISSION CONTROL (BARE-METAL)")
            .with_transparent(true)
            .with_decorations(false)
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0))
            .with_visible(false);

        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Falha ao criar janela Winit: {:?}", e);
                return;
            }
        };

        // 2. Extrair HWND e aplicar composição DWM Acrylic nativa
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = window.window_handle() {
                if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                    let hwnd = win32_handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
                    unsafe {
                        let _ = apply_native_dwm_acrylic(hwnd);
                        register_global_hotkey(hwnd);
                    }
                }
            }
        }

        // 3. Inicializar o Motor de Negócios (CoreEngine)
        let (ui_event_tx, ui_event_rx) = mpsc::unbounded_channel::<IpcEnvelope>();
        let (engine, _) = CoreEngine::new(ui_event_tx);
        let engine_arc = Arc::new(engine);
        engine_arc.start_background_tasks();
        self.engine = Some(engine_arc.clone());

        let (script_tx, script_rx) = mpsc::unbounded_channel::<String>();
        self.script_rx = Some(script_rx);
        let proxy = WebViewProxy::new(script_tx);

        IpcBridge::spawn_event_listener(ui_event_rx, proxy.clone());

        // 4. Inicializar a WebView2 transparente via Wry
        let ipc_bridge = IpcBridge::new(engine_arc);
        let ipc_proxy = proxy.clone();

        let mut builder = WebViewBuilder::new()
            .with_transparent(true)
            .with_ipc_handler(move |msg| {
                ipc_bridge.handle_incoming(&msg.body(), &ipc_proxy);
            });

        if let Ok(dev_url) = std::env::var("SOULS_DEV_URL") {
            tracing::info!("Modo Dev: Carregando frontend a partir de {}", dev_url);
            builder = builder.with_url(&dev_url);
        } else {
            let dist_dir = resolve_dist_dir();
            tracing::info!("Modo Standalone: Servindo assets locais a partir de {}", dist_dir.display());
            let dist_dir_clone = dist_dir.clone();
            builder = builder
                .with_custom_protocol("souls".into(), move |_id, request| {
                    let path = request.uri().path();
                    let clean_path = if path == "/" || path.is_empty() {
                        "index.html"
                    } else {
                        path.trim_start_matches('/')
                    };
                    let file_path = dist_dir_clone.join(clean_path);
                    let (mime, content): (&'static str, Vec<u8>) = if file_path.exists() && file_path.is_file() {
                        let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        let mime = match ext {
                            "html" => "text/html; charset=utf-8",
                            "css" => "text/css; charset=utf-8",
                            "js" => "application/javascript; charset=utf-8",
                            "svg" => "image/svg+xml",
                            "png" => "image/png",
                            "jpg" | "jpeg" => "image/jpeg",
                            "ico" => "image/x-icon",
                            "woff2" => "font/woff2",
                            "woff" => "font/woff",
                            "ttf" => "font/ttf",
                            "json" => "application/json; charset=utf-8",
                            _ => "application/octet-stream",
                        };
                        (mime, std::fs::read(&file_path).unwrap_or_default())
                    } else {
                        let fallback = dist_dir_clone.join("index.html");
                        ("text/html; charset=utf-8", std::fs::read(&fallback).unwrap_or_default())
                    };

                    wry::http::Response::builder()
                        .status(wry::http::StatusCode::OK)
                        .header(wry::http::header::CONTENT_TYPE, mime)
                        .header(wry::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(std::borrow::Cow::Owned(content))
                        .unwrap_or_else(|_| {
                            let empty: &'static [u8] = &[];
                            wry::http::Response::builder()
                                .status(wry::http::StatusCode::INTERNAL_SERVER_ERROR)
                                .body(std::borrow::Cow::Borrowed(empty))
                                .unwrap()
                        })
                })
                .with_url("http://souls.localhost/index.html");
        }

        // Configuração de argumentos para GPU integrada de baixo consumo
        #[cfg(target_os = "windows")]
        {
            builder = builder.with_additional_browser_args(
                "--force_low_power_gpu --enable-low-power-gpu --disable-backgrounding-occluded-windows=false",
            );
        }

        let webview = match builder.build(&window) {
            Ok(wv) => wv,
            Err(e) => {
                tracing::error!("Falha ao instanciar WebView2 Wry: {:?}", e);
                return;
            }
        };

        self.window = Some(window);
        self.webview = Some(webview);
        self.is_visible = true;

        if let Some(ref w) = self.window {
            w.set_visible(true);
            w.focus_window();
        }

        unsafe {
            auto_deactivate_caps_lock();
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(ref w) = self.window {
                    w.set_visible(false);
                    self.is_visible = false;
                    self.suspension.suspend();
                }
            }
            WindowEvent::Focused(focused) => {
                if focused {
                    self.suspension.resume();
                    unsafe {
                        auto_deactivate_caps_lock();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(ref mut wv) = self.webview {
                    let _ = wv.set_bounds(Rect {
                        position: Position::Physical(PhysicalPosition::new(0, 0)),
                        size: Size::Physical(PhysicalSize::new(size.width, size.height)),
                    });
                }
            }
            WindowEvent::RedrawRequested => {
                // Esvaziar fila de scripts pendentes
                if let Some(ref mut rx) = self.script_rx {
                    while let Ok(script) = rx.try_recv() {
                        if let Some(ref wv) = self.webview {
                            let _ = wv.evaluate_script(&script);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Despacha scripts pendentes para a WebView
        if let Some(ref mut rx) = self.script_rx {
            while let Ok(script) = rx.try_recv() {
                if let Some(ref wv) = self.webview {
                    let _ = wv.evaluate_script(&script);
                }
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Some(ref w) = self.window {
                if let Ok(handle) = w.window_handle() {
                    if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                        let hwnd = win32_handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
                        unsafe {
                            unregister_global_hotkey(hwnd);
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    // 1. Inicializar observabilidade e tracing
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .try_init();

    tracing::info!("============================================================");
    tracing::info!("SOULS MC // BARE-METAL DESKTOP CHASSIS (WINDOWS 11 DWM)");
    tracing::info!("============================================================");

    // 2. Inicializar COM Apartment Threading para suporte seguro à WebView2
    let _com_guard = ComApartmentGuard::init_apartment_threaded();

    // 3. Inicializar Runtime Assíncrono do Tokio em background
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Falha ao inicializar Tokio Runtime");

    let _enter = rt.enter();

    // 4. Executar EventLoop Winit na Main Thread
    let event_loop = EventLoop::new().expect("Falha ao criar EventLoop do Winit");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = SoulsApp::new();
    let _ = event_loop.run_app(&mut app);
}

fn resolve_dist_dir() -> std::path::PathBuf {
    // 1. Tenta a partir do CWD atual (ex: raiz do workspace)
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("dist");
        if p.join("index.html").exists() {
            return p;
        }
    }

    // 2. Tenta a partir da árvore de diretórios do executável atual
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            let candidate = dir.join("dist");
            if candidate.join("index.html").exists() {
                return candidate;
            }
            cur = dir.parent();
        }
    }

    std::path::PathBuf::from("dist")
}
