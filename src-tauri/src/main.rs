// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

#[tauri::command]
fn genesis_ping(payload: &str) -> String {
    format!("Genesis Core Online. Recebido: {}", payload)
}

#[allow(clippy::zombie_processes)]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![genesis_ping])
        .setup(|_app| {
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

            spawn_supervised(
                ProgramSpec::global("agentgateway.exe"),
                vec!["-f".to_string(), "gateway-config.yaml".to_string()],
            );
            spawn_supervised(
                ProgramSpec::path(bin_dir.join("agentgateway_tcp_proxy.exe")),
                vec![
                    "--listen".to_string(),
                    "127.0.0.1:3000".to_string(),
                    "--upstream".to_string(),
                    "127.0.0.1:3001".to_string(),
                ],
            );

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao rodar a aplicação tauri");
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

fn spawn_supervised(program: ProgramSpec, args: Vec<String>) {
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

    std::thread::spawn(move || {
        let mut consecutive_fast_failures: u32 = 0;
        loop {
        let started = Instant::now();
        let mut cmd = Command::new(&program.command);
        cmd.args(&args)
            .env("PATH", build_dynamic_path())
            .current_dir(&project_root);

        let status = match cmd.spawn() {
            Ok(mut child) => child.wait().ok(),
            Err(e) => {
                eprintln!("Falha ao spawnar {}: {}", program.label, e);
                None
            }
        };

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
                panic!(
                    "Falha crítica persistente no processo {}",
                    program.label
                );
            }
        } else if lived > Duration::from_secs(5) {
            consecutive_fast_failures = 0;
        }

        std::thread::sleep(Duration::from_millis(500));
        }
    });
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
