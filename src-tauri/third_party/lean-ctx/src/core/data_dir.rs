use std::path::PathBuf;

pub fn lean_ctx_data_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("LEAN_CTX_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            let _ = std::fs::create_dir_all(&path);
            return Ok(path);
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let soda_data = cwd.join(".soda_data");
    if soda_data.exists() || cwd.join("Cargo.toml").exists() {
        let _ = std::fs::create_dir_all(&soda_data);
        return Ok(soda_data);
    }

    let fallback = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".soda_data");
    let _ = std::fs::create_dir_all(&fallback);
    Ok(fallback)
}

pub fn lean_ctx_cache_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("LEAN_CTX_CACHE_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            let _ = std::fs::create_dir_all(&path);
            return Ok(path);
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let soda_cache = cwd.join(".soda_cache");
    if soda_cache.exists() || cwd.join("Cargo.toml").exists() {
        let _ = std::fs::create_dir_all(&soda_cache);
        return Ok(soda_cache);
    }

    let fallback = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".soda_cache");
    let _ = std::fs::create_dir_all(&fallback);
    Ok(fallback)
}

pub fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

