#[cfg(test)]
pub(crate) use mock_executor::*;

#[cfg(test)]
mod mock_executor {
    pub(crate) use std::collections::VecDeque;
    pub(crate) use std::path::{Path, PathBuf};
    pub(crate) use std::sync::{Arc, Mutex};
    pub(crate) use std::future::Future;
    pub(crate) use std::pin::Pin;
    pub(crate) use tempfile::TempDir;
    pub(crate) use crate::harvester::sandbox::SandboxError;
    pub(crate) use super::super::SandboxExecutor;

    pub(crate) struct MockExecutor {
        pub(crate) _temp_dir: TempDir,
        pub(crate) repo_path: PathBuf,
        pub(crate) responses: Mutex<VecDeque<Result<Vec<u8>, SandboxError>>>,
        pub(crate) calls: Mutex<Vec<String>>,
    }

    impl MockExecutor {
        pub(crate) fn new(responses: Vec<Result<Vec<u8>, SandboxError>>) -> Self {
            let temp_dir = TempDir::new().unwrap();
            let owner_dir = temp_dir.path().join("owner");
            let repo_path = owner_dir.join("repo");
            std::fs::create_dir_all(&repo_path).unwrap();

            let index_dir = owner_dir.join(".native_ast_cache");
            std::fs::create_dir_all(&index_dir).unwrap();
            let db_path = index_dir.join("owner-repo.db");
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute(
                "CREATE TABLE files (
                    path TEXT PRIMARY KEY,
                    hash TEXT,
                    mtime_ns INTEGER,
                    language TEXT,
                    summary TEXT,
                    blob_sha TEXT,
                    imports TEXT,
                    size_bytes INTEGER
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO files (path, imports) VALUES (?1, ?2)",
                rusqlite::params![
                    "src/main.rs",
                    r#"[{"specifier":"crate::config","names":["AppConfig"]},{"specifier":"goose_cli::session","names":["run_session"]},{"specifier":"serde_json","names":["json"]}]"#
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO files (path, imports) VALUES (?1, ?2)",
                rusqlite::params![
                    "src/lib.rs",
                    r#"[{"specifier":"super::utils","names":["normalize"]},{"specifier":"../shared/logger","names":["logger"]}]"#
                ],
            )
            .unwrap();

            Self {
                _temp_dir: temp_dir,
                repo_path,
                responses: Mutex::new(VecDeque::from(responses)),
                calls: Mutex::new(Vec::new()),
            }
        }

        pub(crate) fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        pub(crate) fn write_repo_file(&self, relative_path: &str, contents: &str) {
            let path = self.repo_path.join(relative_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, contents).unwrap();
        }
    }

    pub(crate) fn canonicalize_or_self(path: PathBuf) -> PathBuf {
        path.canonicalize().unwrap_or(path)
    }

    pub(crate) fn test_clean_files(repo_root: &Path, rels: &[&str]) -> Arc<Vec<PathBuf>> {
        Arc::new(
            rels.iter()
                .map(|rel| canonicalize_or_self(repo_root.join(rel)))
                .collect(),
        )
    }

    impl SandboxExecutor for MockExecutor {
        fn execute<'a>(
            &'a self,
            command: &'a str,
            args: &'a [&'a str],
            _timeout_secs: u64,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>> {
            Box::pin(async move {
                self.calls
                    .lock()
                    .unwrap()
                    .push(format!("{} {}", command, args.join(" ")).trim().to_string());
                let mut guard = self.responses.lock().unwrap();
                guard.pop_front().unwrap_or_else(|| {
                    Err(SandboxError::ProcessSpawnFailed {
                        reason: "no mock response configured".to_string(),
                    })
                })
            })
        }

        fn execute_in_dir<'a>(
            &'a self,
            command: &'a str,
            args: &'a [&'a str],
            _timeout_secs: u64,
            execution_root: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SandboxError>> + Send + 'a>> {
            Box::pin(async move {
                self.calls.lock().unwrap().push(format!(
                    "{} {} [cwd={}]",
                    command,
                    args.join(" "),
                    execution_root.display()
                ).trim().to_string());
                
                if command == "opengrep" {
                    let is_security = args.iter().any(|arg| arg.contains("security.yml") || arg.contains("security.yaml") || arg.contains("security.json"));
                    let is_health = args.iter().any(|arg| arg.contains("health.yml") || arg.contains("health.yaml") || arg.contains("health.json"));
                    let mut guard = self.responses.lock().unwrap();
                    if is_security {
                        if let Some(pos) = guard.iter().position(|r| {
                            if let Ok(bytes) = r {
                                bytes.is_empty() || String::from_utf8_lossy(bytes).contains("\"results\":[]")
                            } else {
                                false
                            }
                        }) {
                            return guard.remove(pos).unwrap();
                        }
                    } else if is_health {
                        if let Some(pos) = guard.iter().position(|r| {
                            if let Ok(bytes) = r {
                                String::from_utf8_lossy(bytes).contains("soda.tech-debt.todo-fixme")
                            } else {
                                false
                            }
                        }) {
                            return guard.remove(pos).unwrap();
                        }
                    }
                }

                let mut guard = self.responses.lock().unwrap();
                guard.pop_front().unwrap_or_else(|| {
                    Err(SandboxError::ProcessSpawnFailed {
                        reason: "no mock response configured".to_string(),
                    })
                })
            })
        }

        fn repo_path(&self) -> &Path {
            &self.repo_path
        }
    }
}
