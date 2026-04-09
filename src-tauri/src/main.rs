// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // genesis_mc_lib::run()
    tauri::Builder::default()
        .setup(|_app| {
            // Inicializa o AgentGateway silenciosamente no background com o PATH turbinado
            Command::new("agentgateway.exe")
                .arg("-f")
                .arg("gateway-config.yaml")
                .env("PATH", build_dynamic_path())
                .spawn()
                .expect("Falha crítica: Não foi possível iniciar o Roteador AgentGateway");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao rodar a aplicação tauri");
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
