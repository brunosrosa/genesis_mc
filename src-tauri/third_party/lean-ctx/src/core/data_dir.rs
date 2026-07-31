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
    let souls_data = cwd.join(".souls_data");
    if souls_data.exists() || cwd.join("Cargo.toml").exists() {
        let _ = std::fs::create_dir_all(&souls_data);
        return Ok(souls_data);
    }

    let fallback = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".souls_data");
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
    let souls_cache = cwd.join(".souls_cache");
    if souls_cache.exists() || cwd.join("Cargo.toml").exists() {
        let _ = std::fs::create_dir_all(&souls_cache);
        return Ok(souls_cache);
    }

    let fallback = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".souls_cache");
    let _ = std::fs::create_dir_all(&fallback);
    Ok(fallback)
}

pub fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

