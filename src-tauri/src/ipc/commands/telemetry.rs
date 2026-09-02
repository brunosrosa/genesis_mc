use tauri::ipc::Channel;

/// `souls_ping` — health check trivial.
#[tauri::command]
pub fn souls_ping(payload: &str) -> String {
    format!("Souls MC Core Online. Recebido: {}", payload)
}

/// `start_watchdog_stream` — canal binário de telemetria 1Hz Zero-Copy.
#[tauri::command]
pub async fn start_watchdog_stream(app: tauri::AppHandle, channel: Channel<Vec<u8>>) {
    crate::telemetry::watchdog_ipc::spawn_watchdog_channel(app, channel);
}
