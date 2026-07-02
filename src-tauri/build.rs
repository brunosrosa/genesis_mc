fn main() {
    if std::env::var_os("CLIPPY_ARGS").is_some() {
        return;
    }
    if std::env::var_os("CARGO_FEATURE_TAURI_APP").is_none()
        || std::env::var_os("GENESIS_SKIP_TAURI_BUILD").is_some()
    {
        return;
    }
    tauri_build::build()
}
