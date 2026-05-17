use std::path::PathBuf;
use thiserror::Error;
use super::git::RepoPath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SingleStack {
    Rust,
    NodeJS,
    Go,
    Python,
    JVM,
    DotNet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackProfile {
    Rust,
    NodeJS,
    Go,
    Python,
    JVM,
    DotNet,
    Mixed(Vec<SingleStack>),
    Unknown,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DetectionError {
    #[error("Repository is empty: {path}")]
    EmptyRepository { path: PathBuf },

    #[error("Filesystem error: {reason}")]
    FilesystemError { reason: String },
}

pub struct LanguageDetector;

impl LanguageDetector {
    pub async fn detect(repo_path: &RepoPath) -> Result<StackProfile, DetectionError> {
        let mut entries = match tokio::fs::read_dir(repo_path.as_ref()).await {
            Ok(entries) => entries,
            Err(e) => return Err(DetectionError::FilesystemError { reason: e.to_string() }),
        };

        let mut has_entries = false;
        let mut detected_stacks = std::collections::HashSet::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            has_entries = true;
            
            // Verifica o tipo de arquivo
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(e) => return Err(DetectionError::FilesystemError { reason: e.to_string() }),
            };

            if file_type.is_file() || file_type.is_symlink() {
                if let Some(name_str) = entry.file_name().to_str() {
                    match name_str {
                        "Cargo.toml" => {
                            detected_stacks.insert(SingleStack::Rust);
                        }
                        "package.json" => {
                            detected_stacks.insert(SingleStack::NodeJS);
                        }
                        "go.mod" => {
                            detected_stacks.insert(SingleStack::Go);
                        }
                        "requirements.txt" | "pyproject.toml" | "setup.py" | "Pipfile" => {
                            detected_stacks.insert(SingleStack::Python);
                        }
                        "pom.xml" | "build.gradle" | "build.gradle.kts" => {
                            detected_stacks.insert(SingleStack::JVM);
                        }
                        _ => {
                            if name_str.ends_with(".sln") || name_str.ends_with(".csproj") {
                                detected_stacks.insert(SingleStack::DotNet);
                            }
                        }
                    }
                }
            }
        }

        if !has_entries {
            return Err(DetectionError::EmptyRepository { path: repo_path.as_ref().to_path_buf() });
        }

        match detected_stacks.len() {
            0 => Ok(StackProfile::Unknown),
            1 => {
                let mut detected = detected_stacks.into_iter();
                let Some(single) = detected.next() else {
                    return Err(DetectionError::FilesystemError {
                        reason: "Invariante violada: len()==1 mas nenhum stack foi retornado".to_string(),
                    });
                };
                Ok(match single {
                    SingleStack::Rust => StackProfile::Rust,
                    SingleStack::NodeJS => StackProfile::NodeJS,
                    SingleStack::Go => StackProfile::Go,
                    SingleStack::Python => StackProfile::Python,
                    SingleStack::JVM => StackProfile::JVM,
                    SingleStack::DotNet => StackProfile::DotNet,
                })
            }
            _ => {
                // Coleta em um vetor ordenado de forma determinística
                let mut vec_stacks: Vec<SingleStack> = detected_stacks.into_iter().collect();
                vec_stacks.sort_by_key(|s| match s {
                    SingleStack::Rust => 1,
                    SingleStack::NodeJS => 2,
                    SingleStack::Go => 3,
                    SingleStack::Python => 4,
                    SingleStack::JVM => 5,
                    SingleStack::DotNet => 6,
                });
                Ok(StackProfile::Mixed(vec_stacks))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::SystemTime;

    static TEST_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    static DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    async fn get_test_mutex() -> &'static tokio::sync::Mutex<()> {
        TEST_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    async fn create_temp_test_dir() -> PathBuf {
        let mut temp = std::env::temp_dir();
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let count = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let unique_dir = format!("soda_detect_test_{}_{}", now, count);
        temp.push(unique_dir);
        tokio::fs::create_dir_all(&temp).await.unwrap();
        temp
    }

    #[tokio::test]
    async fn test_detect_rust() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("Cargo.toml"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::Rust);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_nodejs() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("package.json"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::NodeJS);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_go() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("go.mod"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::Go);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_python() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("pyproject.toml"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::Python);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_jvm() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("build.gradle"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::JVM);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_dotnet() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("Solution.sln"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::DotNet);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_mixed() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("Cargo.toml"), b"").await.unwrap();
        tokio::fs::write(temp.join("package.json"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        if let StackProfile::Mixed(stacks) = profile {
            assert!(stacks.contains(&SingleStack::Rust));
            assert!(stacks.contains(&SingleStack::NodeJS));
            assert_eq!(stacks.len(), 2);
        } else {
            panic!("Deveria retornar StackProfile::Mixed");
        }

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_unknown() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        tokio::fs::write(temp.join("README.md"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::Unknown);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_empty_repo() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;

        let repo_path = RepoPath(temp.clone());
        let res = LanguageDetector::detect(&repo_path).await;

        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            DetectionError::EmptyRepository { path: temp.clone() }
        );

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_no_recursive_walk() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;
        
        // Coloca manifestos apenas em subpastas, não na raiz
        let subdir = temp.join("src");
        tokio::fs::create_dir(&subdir).await.unwrap();
        tokio::fs::write(subdir.join("Cargo.toml"), b"").await.unwrap();
        
        // Coloca um arquivo qualquer na raiz para o repositório não ser considerado vazio
        tokio::fs::write(temp.join("README.md"), b"").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        // Deve ignorar o Cargo.toml que está na subpasta!
        assert_eq!(profile, StackProfile::Unknown);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }
}
