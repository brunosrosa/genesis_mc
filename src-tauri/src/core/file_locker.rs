use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use dashmap::DashMap;
use uuid::Uuid;

static PATH_LOCKS: OnceLock<DashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>> = OnceLock::new();

/// Recupera ou insere uma trava assíncrona por caminho de arquivo no `PATH_LOCKS`.
/// Executa a poda de travas órfãs (`Arc::strong_count == 1`) para impedir vazamento de RAM.
pub fn acquire_file_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let map = PATH_LOCKS.get_or_init(DashMap::new);

    // Poda ativa de travas não disputadas por nenhuma outra thread (strong_count == 1)
    map.retain(|_k, lock| Arc::strong_count(lock) > 1);

    map.entry(canonical)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .value()
        .clone()
}

/// Gravação atômica em arquivo via swap temporário (`.tmp_uuid` -> `std::fs::rename`).
pub async fn atomic_write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let tmp_name = format!(".tmp_{}", Uuid::new_v4().simple());
    let tmp_path = parent.join(tmp_name);

    tokio::fs::write(&tmp_path, content.as_bytes()).await?;

    if let Err(e) = tokio::fs::rename(&tmp_path, path).await {
        // Fallback em caso de erro de rename (ex: cross-device link)
        if tokio::fs::copy(&tmp_path, path).await.is_ok() {
            let _ = tokio::fs::remove_file(&tmp_path).await;
        } else {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_lock_garbage_collection() {
        let path = PathBuf::from("test_orphan_lock.tmp");
        {
            let lock = acquire_file_lock(&path);
            assert_eq!(Arc::strong_count(&lock), 2); // 1 na função, 1 no DashMap
        }
        // Fora do escopo, Arc::strong_count no DashMap deve cair para 1.
        let map = PATH_LOCKS.get().unwrap();
        assert!(map.contains_key(&path.canonicalize().unwrap_or(path.clone())));

        // Ao requisitar uma nova trava, a poda deve limpar a chave antiga se não estiver em uso
        let other_path = PathBuf::from("test_other_lock.tmp");
        let _other_lock = acquire_file_lock(&other_path);

        assert!(!map.contains_key(&path.canonicalize().unwrap_or(path.clone())));
    }
}
