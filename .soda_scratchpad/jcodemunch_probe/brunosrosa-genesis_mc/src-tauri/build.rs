fn main() {
    if std::env::var_os("GENESIS_SKIP_TAURI_BUILD").is_some() {
        println!("cargo:warning=Skipping tauri_build for CLI-only flow");
        return;
    }
    tauri_build::build()
}
