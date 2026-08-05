use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use super::git::RepoPath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SingleStack {
    Rust,
    CCpp,
    Elixir,
    NodeJS,
    Go,
    Python,
    JVM,
    DotNet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackProfile {
    Rust,
    CCpp,
    Elixir,
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

fn stack_priority(stack: &SingleStack) -> usize {
    match stack {
        SingleStack::Rust => 1,
        SingleStack::CCpp => 2,
        SingleStack::Elixir => 3,
        SingleStack::NodeJS => 4,
        SingleStack::Go => 5,
        SingleStack::Python => 6,
        SingleStack::JVM => 7,
        SingleStack::DotNet => 8,
    }
}

fn root_marker_stack(file_name: &str) -> Option<SingleStack> {
    match file_name {
        "Cargo.toml" | "rust-toolchain.toml" => Some(SingleStack::Rust),
        "CMakeLists.txt" | "compile_commands.json" | "meson.build" | "Makefile" => {
            Some(SingleStack::CCpp)
        }
        "mix.exs" | "mix.lock" => Some(SingleStack::Elixir),
        "package.json"
        | "pnpm-workspace.yaml"
        | "package-lock.json"
        | "yarn.lock"
        | "bun.lockb"
        | "svelte.config.js"
        | "svelte.config.ts"
        | "vite.config.js"
        | "vite.config.ts" => Some(SingleStack::NodeJS),
        "go.mod" => Some(SingleStack::Go),
        "requirements.txt" | "pyproject.toml" | "setup.py" | "Pipfile" => Some(SingleStack::Python),
        "pom.xml" | "build.gradle" | "build.gradle.kts" => Some(SingleStack::JVM),
        value if value.ends_with(".sln") || value.ends_with(".csproj") => Some(SingleStack::DotNet),
        _ => None,
    }
}

fn extension_stack(path: &Path) -> Option<SingleStack> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    match extension.as_deref() {
        Some("rs") => Some(SingleStack::Rust),
        Some("c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx") => Some(SingleStack::CCpp),
        Some("ex" | "exs") => Some(SingleStack::Elixir),
        Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" | "svelte") => {
            Some(SingleStack::NodeJS)
        }
        Some("go") => Some(SingleStack::Go),
        Some("py") => Some(SingleStack::Python),
        Some("java" | "kt" | "kts" | "scala") => Some(SingleStack::JVM),
        Some("cs") => Some(SingleStack::DotNet),
        _ => None,
    }
}

fn should_skip_walk_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".jj"
            | ".svn"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | "vendor"
            | ".venv"
            | "venv"
    )
}

fn profile_from_sorted_stacks(mut stacks: Vec<SingleStack>) -> StackProfile {
    stacks.sort_by_key(stack_priority);
    stacks.dedup();
    match stacks.as_slice() {
        [] => StackProfile::Unknown,
        [SingleStack::Rust] => StackProfile::Rust,
        [SingleStack::CCpp] => StackProfile::CCpp,
        [SingleStack::Elixir] => StackProfile::Elixir,
        [SingleStack::NodeJS] => StackProfile::NodeJS,
        [SingleStack::Go] => StackProfile::Go,
        [SingleStack::Python] => StackProfile::Python,
        [SingleStack::JVM] => StackProfile::JVM,
        [SingleStack::DotNet] => StackProfile::DotNet,
        _ => StackProfile::Mixed(stacks),
    }
}

impl LanguageDetector {
    pub async fn detect(repo_path: &RepoPath) -> Result<StackProfile, DetectionError> {
        let repo_root = repo_path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let root_entries = std::fs::read_dir(&repo_root).map_err(|e| DetectionError::FilesystemError {
                reason: e.to_string(),
            })?;
            let mut has_entries = false;
            let mut scores = HashMap::<SingleStack, usize>::new();

            for entry in root_entries {
                let entry = entry.map_err(|e| DetectionError::FilesystemError {
                    reason: e.to_string(),
                })?;
                has_entries = true;
                let file_name = entry.file_name();
                let file_name = match file_name.to_str() {
                    Some(value) => value,
                    None => continue,
                };
                if let Some(stack) = root_marker_stack(file_name) {
                    *scores.entry(stack).or_default() += 100;
                }
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() || file_type.is_symlink() {
                        if let Some(stack) = extension_stack(&entry.path()) {
                            *scores.entry(stack).or_default() += 6;
                        }
                    }
                }
            }

            if !has_entries {
                return Err(DetectionError::EmptyRepository { path: repo_root.clone() });
            }

            let walk = ignore::WalkBuilder::new(&repo_root)
                .hidden(false)
                .filter_entry(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(|name| !should_skip_walk_dir(name))
                        .unwrap_or(true)
                })
                .build();
            for entry in walk.take(4_000) {
                let entry = match entry {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                if !entry
                    .file_type()
                    .map(|file_type| file_type.is_file())
                    .unwrap_or(false)
                {
                    continue;
                }
                if let Some(stack) = extension_stack(entry.path()) {
                    *scores.entry(stack).or_default() += 2;
                }
            }

            let mut ranked = scores.into_iter().collect::<Vec<_>>();
            ranked.sort_by(|(stack_left, score_left), (stack_right, score_right)| {
                score_right
                    .cmp(score_left)
                    .then_with(|| stack_priority(stack_left).cmp(&stack_priority(stack_right)))
            });
            let stacks = ranked.into_iter().map(|(stack, _)| stack).collect::<Vec<_>>();
            Ok(profile_from_sorted_stacks(stacks))
        })
        .await
        .map_err(|e| DetectionError::FilesystemError {
            reason: format!("Falha ao aguardar lang_detect: {}", e),
        })?
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
        let unique_dir = format!("souls_detect_test_{}_{}", now, count);
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
    async fn test_detect_mixed_rust_and_cpp_prefers_root_stack_order() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;

        tokio::fs::write(temp.join("Cargo.toml"), b"[package]\nname='demo'\nversion='0.1.0'\n").await.unwrap();
        tokio::fs::write(temp.join("CMakeLists.txt"), b"cmake_minimum_required(VERSION 3.10)\nproject(demo)\n").await.unwrap();
        tokio::fs::create_dir_all(temp.join("native")).await.unwrap();
        tokio::fs::write(temp.join("native").join("bridge.cpp"), b"int main() { return 0; }\n").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(
            profile,
            StackProfile::Mixed(vec![SingleStack::Rust, SingleStack::CCpp])
        );

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_elixir_from_root_mix_file() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;

        tokio::fs::write(
            temp.join("mix.exs"),
            b"defmodule Demo.MixProject do\nend\n",
        )
        .await
        .unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::Elixir);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_svelte_without_package_json_still_maps_to_nodejs_family() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;

        tokio::fs::write(temp.join("App.svelte"), b"<script>export let name;</script>\n").await.unwrap();
        tokio::fs::write(temp.join("svelte.config.js"), b"export default {};\n").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::NodeJS);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_nested_source_file_via_recursive_walk() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;

        tokio::fs::create_dir_all(temp.join("src").join("deep")).await.unwrap();
        tokio::fs::write(
            temp.join("src").join("deep").join("main.go"),
            b"package main\nfunc main() {}\n",
        )
        .await
        .unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::Go);

        let _ = tokio::fs::remove_dir_all(&temp).await;
    }

    #[tokio::test]
    async fn test_detect_skips_ignored_dirs_during_recursive_walk() {
        let _guard = get_test_mutex().await.lock().await;
        let temp = create_temp_test_dir().await;

        tokio::fs::create_dir_all(temp.join("node_modules").join("pkg")).await.unwrap();
        tokio::fs::write(
            temp.join("node_modules").join("pkg").join("main.go"),
            b"package main\nfunc main() {}\n",
        )
        .await
        .unwrap();
        tokio::fs::write(temp.join("README.md"), b"root only\n").await.unwrap();

        let repo_path = RepoPath(temp.clone());
        let profile = LanguageDetector::detect(&repo_path).await.unwrap();

        assert_eq!(profile, StackProfile::Unknown);

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
