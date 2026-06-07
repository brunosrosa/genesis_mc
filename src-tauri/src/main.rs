// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::Manager;

#[tauri::command]
fn genesis_ping(payload: &str) -> String {
    format!("Genesis Core Online. Recebido: {}", payload)
}

#[allow(clippy::zombie_processes)]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![genesis_ping])
        .setup(|app| {
            let fallback_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("debug");

            let bin_dir = match resolve_running_bin_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("Falha ao resolver diretório do supervisor: {e}");
                    fallback_dir
                }
            };

            let project_root = resolve_project_root();
            let agentgateway = spawn_supervised(
                ProgramSpec::global("agentgateway.exe"),
                vec!["-f".to_string(), "gateway-config.yaml".to_string()],
                project_root.clone(),
            );
            let tcp_proxy = spawn_supervised(
                ProgramSpec::path(bin_dir.join("agentgateway_tcp_proxy.exe")),
                vec![
                    "--listen".to_string(),
                    "127.0.0.1:3000".to_string(),
                    "--upstream".to_string(),
                    "127.0.0.1:3001".to_string(),
                ],
                project_root,
            );

            app.manage(Supervisor {
                processes: vec![agentgateway, tcp_proxy],
            });

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
                .menu(&menu)
                .on_menu_event(|app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id().as_ref() {
                    "toggle_overlay" => toggle_overlay_window(app),
                    "quit" => {
                        app.state::<Supervisor>().shutdown();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event: tauri::tray::TrayIconEvent| {
                    if let tauri::tray::TrayIconEvent::DoubleClick {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        toggle_overlay_window(&tray.app_handle());
                    }
                })
                .build(app)?;

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

fn resolve_running_bin_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe falhou: {e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "current_exe sem parent()".to_string())?;
    Ok(dir.to_path_buf())
}

#[derive(Clone)]
struct ProgramSpec {
    command: OsString,
    label: String,
}

impl ProgramSpec {
    fn path(path: PathBuf) -> Self {
        Self {
            command: path.clone().into_os_string(),
            label: path.display().to_string(),
        }
    }

    fn global(name: &str) -> Self {
        Self {
            command: OsString::from(name),
            label: name.to_string(),
        }
    }
}

#[derive(Clone)]
struct Supervisor {
    processes: Vec<SupervisedProcess>,
}

impl Supervisor {
    fn shutdown(&self) {
        for process in &self.processes {
            process.shutdown();
        }
    }
}

#[derive(Clone)]
struct SupervisedProcess {
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl SupervisedProcess {
    fn shutdown(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn resolve_project_root() -> PathBuf {
    let cwd = std::env::current_dir().expect("Falha ao obter CWD");
    let mut project_root = cwd.clone();
    if project_root
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("src-tauri"))
        .unwrap_or(false)
    {
        project_root = project_root
            .parent()
            .unwrap_or(&project_root)
            .to_path_buf();
    }
    if !project_root.join("gateway-config.yaml").is_file() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if let Some(parent) = manifest_dir.parent() {
            project_root = parent.to_path_buf();
        }
    }

    project_root
}

fn spawn_supervised(program: ProgramSpec, args: Vec<String>, project_root: PathBuf) -> SupervisedProcess {
    let stop = Arc::new(AtomicBool::new(false));
    let child = Arc::new(Mutex::new(None));

    let stop_thread = stop.clone();
    let child_thread = child.clone();
    std::thread::spawn(move || {
        let mut consecutive_fast_failures: u32 = 0;
        loop {
            if stop_thread.load(Ordering::SeqCst) {
                break;
            }

            let started = Instant::now();
            let mut cmd = Command::new(&program.command);
            cmd.args(&args)
                .env("PATH", build_dynamic_path())
                .current_dir(&project_root);

            let spawned = match cmd.spawn() {
                Ok(child) => child,
                Err(e) => {
                    eprintln!("Falha ao spawnar {}: {}", program.label, e);
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            };

            if let Ok(mut guard) = child_thread.lock() {
                *guard = Some(spawned);
            }

            loop {
                if stop_thread.load(Ordering::SeqCst) {
                    if let Ok(mut guard) = child_thread.lock() {
                        if let Some(mut child) = guard.take() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                    return;
                }

                let status = {
                    let mut guard = match child_thread.lock() {
                        Ok(g) => g,
                        Err(_) => {
                            std::thread::sleep(Duration::from_millis(200));
                            continue;
                        }
                    };

                    match guard.as_mut() {
                        Some(child) => match child.try_wait() {
                            Ok(Some(status)) => {
                                let _ = guard.take();
                                Some(status)
                            }
                            Ok(None) => None,
                            Err(_) => None,
                        },
                        None => None,
                    }
                };

                if let Some(status) = status {
                    let lived = started.elapsed();
                    if lived < Duration::from_secs(2) {
                        consecutive_fast_failures = consecutive_fast_failures.saturating_add(1);
                        eprintln!(
                            "Falha rápida no processo {} ({}). lived_ms={} status={:?}",
                            program.label,
                            consecutive_fast_failures,
                            lived.as_millis(),
                            status
                        );
                        if consecutive_fast_failures >= 3 {
                            panic!("Falha crítica persistente no processo {}", program.label);
                        }
                    } else if lived > Duration::from_secs(5) {
                        consecutive_fast_failures = 0;
                    }
                    break;
                }

                std::thread::sleep(Duration::from_millis(200));
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    });

    SupervisedProcess { stop, child }
}

/// Descobre onde os gerenciadores de pacote instalam os binários no Windows
fn get_soda_essential_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(user_profile) = env::var_os("USERPROFILE") {
        let base = PathBuf::from(user_profile);
        paths.push(base.join(".cargo").join("bin")); // Para ferramentas Rust (mcp-server-time)
        paths.push(
            base.join("AppData")
                .join("Roaming")
                .join("uv")
                .join("tools"),
        ); // Para ferramentas uv
        paths.push(base.join("AppData").join("Roaming").join("npm")); // Para ferramentas Node (se usar npx no futuro)
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    paths.push(manifest_dir.join("target").join("debug"));
    paths.push(manifest_dir.join("target").join("release"));

    paths
}

/// Cria a nova variável PATH fundindo o sistema atual com os caminhos do SODA
fn build_dynamic_path() -> String {
    let current_path = env::var_os("PATH").unwrap_or_default();
    let mut dynamic_paths = env::split_paths(&current_path).collect::<Vec<_>>();

    for essential_path in get_soda_essential_paths() {
        if essential_path.exists() && !dynamic_paths.contains(&essential_path) {
            dynamic_paths.insert(0, essential_path); // Injeta no início para ter prioridade
        }
    }

    env::join_paths(dynamic_paths)
        .unwrap()
        .into_string()
        .unwrap()
}
