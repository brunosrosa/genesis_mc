pub mod harvester;
pub mod persist;
pub mod finops;
pub mod cognition;

#[tauri::command]
fn genesis_ping(payload: &str) -> String {
    format!("Genesis Core Online. Recebido: {}", payload)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![genesis_ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
