use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use rusqlite::params;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use tracing::error;
use crate::harvester::sandbox::SandboxError;

/// Trait para abstrair a execução no sandbox, permitindo mocks nos testes.
#[allow(async_fn_in_trait)]
pub trait SandboxExecutor {
    async fn execute(&self, command: &str, args: &[&str], timeout_secs: u64) -> Result<Vec<u8>, SandboxError>;
    fn repo_path(&self) -> &Path;
}

/// Implementação da trait SandboxExecutor para o SandboxHandle concreto.
impl SandboxExecutor for crate::harvester::sandbox::SandboxHandle {
    async fn execute(&self, command: &str, args: &[&str], timeout_secs: u64) -> Result<Vec<u8>, SandboxError> {
        self.execute(command, args, timeout_secs).await
    }

    fn repo_path(&self) -> &Path {
        self.repo_path()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SymbolOutline {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DependencyEdge {
    pub source_file: String,
    pub target: String,
    pub edge_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AstPayload {
    pub symbols: Vec<SymbolOutline>,
    pub dependency_edges: Vec<DependencyEdge>,
    pub files_processed: u32,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SidecarError {
    #[error("Sidecar binary not found: {binary}")]
    BinaryNotFound { binary: String },

    #[error("Execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("Execution timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("Failed to parse sidecar output: {reason}")]
    ParseError { reason: String },
}

pub struct JCodemunchInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
    pub persist_artifact: Option<PersistArtifactConfig<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistArtifactConfig<'a> {
    pub repo_id: &'a str,
    pub artifact_type: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidecarExitPolicy {
    StrictZeroOnly,
    AllowFindingsExitOne,
}

fn stdout_is_blank(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| byte.is_ascii_whitespace())
}

impl AstPayload {
    fn is_empty(&self) -> bool {
        self.files_processed == 0 && self.symbols.is_empty() && self.dependency_edges.is_empty()
    }
}

fn digest_json_is_empty(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::Object(map) => map.is_empty(),
        serde_json::Value::Array(items) => items.is_empty(),
        serde_json::Value::String(text) => text.trim().is_empty(),
        _ => false,
    }
}

fn payload_from_digest_json(
    repo_path: &Path,
    digest: serde_json::Value,
) -> Result<AstPayload, SidecarError> {
    if digest_json_is_empty(&digest) {
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp returned an empty digest payload".to_string(),
        });
    }

    let summary = serde_json::to_string(&digest).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })?;

    Ok(AstPayload {
        symbols: vec![SymbolOutline {
            name: "repo_digest".to_string(),
            kind: "digest".to_string(),
            file_path: repo_path.display().to_string(),
            start_line: 1,
            end_line: 1,
            signature: Some(summary),
        }],
        dependency_edges: Vec::new(),
        files_processed: 1,
    })
}

fn code_index_path_for_repo(repo_path: &Path) -> String {
    repo_path
        .parent()
        .unwrap_or(repo_path)
        .join(".jcodemunch_index")
        .display()
        .to_string()
}

fn validate_index_response(bytes: &[u8]) -> Result<(), SidecarError> {
    if stdout_is_blank(bytes) {
        return Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp index returned empty stdout".to_string(),
        });
    }

    let index_json = serde_json::from_slice::<serde_json::Value>(bytes).map_err(|e| SidecarError::ParseError {
        reason: e.to_string(),
    })?;

    match index_json.get("success").and_then(|value| value.as_bool()) {
        Some(true) => Ok(()),
        Some(false) => {
            let detail = index_json
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown indexing failure");
            Err(SidecarError::ExecutionFailed {
                reason: format!("jcodemunch-mcp index failed: {}", detail),
            })
        }
        None => Err(SidecarError::ExecutionFailed {
            reason: "jcodemunch-mcp index returned a payload without success flag".to_string(),
        }),
    }
}

async fn persist_artifact_blob(config: PersistArtifactConfig<'_>, payload_blob: Vec<u8>) -> Result<(), SidecarError> {
    let repo_id = config.repo_id.to_string();
    let artifact_type = config.artifact_type.to_string();
    tokio::task::spawn_blocking(move || {
        let db_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
            .join(".soda_data")
            .join("soda_heuristic_vault.db");

        let conn = rusqlite::Connection::open(&db_path).map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao conectar no SQLite para persistir artefato: {}", e),
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SidecarError::ExecutionFailed {
                reason: format!("Falha ao calcular timestamp de extracao: {}", e),
            })?
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO artefatos_brutos (repo_id, artifact_type, payload_blob, timestamp_extracao) VALUES (?1, ?2, ?3, ?4)",
            params![repo_id, artifact_type, payload_blob, now],
        )
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao persistir artefato no SQLite: {}", e),
        })?;

        Ok(())
    })
    .await
    .map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao aguardar persistencia do artefato: {}", e),
    })?
}

/// Executa um binário sidecar no sandbox e retorna os bytes brutos do stdout.
/// Centraliza a tradução SandboxError → SidecarError para todos os sidecars.
async fn execute_sidecar<E: SandboxExecutor>(
    executor: &E,
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
    exit_policy: SidecarExitPolicy,
) -> Result<Vec<u8>, SidecarError> {
    match executor.execute(binary, args, timeout_secs).await {
        Ok(bytes) => Ok(bytes),
        Err(SandboxError::Timeout) => {
            Err(SidecarError::Timeout { timeout_secs })
        }
        Err(SandboxError::ProcessSpawnFailed { reason }) => {
            error!(binary = %binary, reason = %reason, "Falha ao iniciar sidecar");
            let lower_reason = reason.to_lowercase();
            if lower_reason.contains("not found") || lower_reason.contains("os error 2") {
                Err(SidecarError::BinaryNotFound {
                    binary: binary.to_string(),
                })
            } else {
                Err(SidecarError::ExecutionFailed { reason })
            }
        }
        // Match numérico explícito no exit code — sem manipulação de string.
        // Exit code 1: linters sinalizam violações encontradas (sucesso de negócio).
        // Exit code 2+: erro real de execução (config inválida, crash).
        Err(SandboxError::ProcessNonZeroExit { exit_code, stderr, stdout }) => {
            if exit_code == 1 && matches!(exit_policy, SidecarExitPolicy::AllowFindingsExitOne) {
                Ok(stdout)
            } else {
                error!(
                    binary = %binary,
                    exit_code,
                    stderr = %stderr,
                    "Sidecar terminou com exit code nao zero"
                );
                Err(SidecarError::ExecutionFailed {
                    reason: format!("exit code {exit_code}: {stderr}"),
                })
            }
        }
        Err(e) => {
            Err(SidecarError::ExecutionFailed {
                reason: e.to_string(),
            })
        }
    }
}

pub struct JCodemunchSidecar;

impl JCodemunchSidecar {
    /// Extrai a AST e o grafo de dependências usando o jcodemunch no sandbox.
    pub async fn extract<E: SandboxExecutor>(
        input: JCodemunchInput<'_, E>,
    ) -> Result<AstPayload, SidecarError> {
        let storage_path = code_index_path_for_repo(input.executor.repo_path());
        let index_args = vec!["index".to_string(), "--no-ai-summaries".to_string()];
        let index_arg_refs: Vec<&str> = index_args.iter().map(String::as_str).collect();
        let index_bytes = execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &index_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await?;
        validate_index_response(&index_bytes)?;

        let digest_args = vec![
            "digest".to_string(),
            "--json".to_string(),
            "--storage-path".to_string(),
            storage_path,
        ];
        let digest_arg_refs: Vec<&str> = digest_args.iter().map(String::as_str).collect();
        let bytes = execute_sidecar(
            input.executor,
            "jcodemunch-mcp",
            &digest_arg_refs,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await?;

        if stdout_is_blank(&bytes) {
            error!(binary = "jcodemunch-mcp", "Sidecar AST retornou stdout vazio");
            return Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch-mcp returned empty stdout".to_string(),
            });
        }

        if let Some(config) = input.persist_artifact {
            persist_artifact_blob(config, bytes.clone()).await?;
        }

        let digest_json = serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| SidecarError::ParseError {
            reason: e.to_string(),
        })?;
        let payload = serde_json::from_value::<AstPayload>(digest_json.clone())
            .or_else(|_| payload_from_digest_json(input.executor.repo_path(), digest_json))
            .map_err(|e| match e {
                SidecarError::ParseError { reason } => SidecarError::ParseError { reason },
                other => other,
            })?;

        if payload.is_empty() {
            error!(
                binary = "jcodemunch-mcp",
                files_processed = payload.files_processed,
                "Sidecar AST retornou payload vazio"
            );
            return Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch-mcp returned an empty AST payload".to_string(),
            });
        }

        Ok(payload)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PropDeclaration {
    pub name: String,
    pub prop_type: String,
    pub has_default: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ComponentContract {
    pub name: String,
    pub file_path: String,
    pub framework: String,
    pub props: Vec<PropDeclaration>,
    pub events: Vec<String>,
    pub is_default_export: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UxContractsPayload {
    pub components: Vec<ComponentContract>,
    pub files_analyzed: u32,
}

pub struct OxcInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}

pub struct OxcSidecar;

impl OxcSidecar {
    /// Extrai os contratos UX (props, events, etc) usando o oxlint no sandbox.
    pub async fn extract<E: SandboxExecutor>(
        input: OxcInput<'_, E>,
    ) -> Result<UxContractsPayload, SidecarError> {
        let args = ["lint", "--format", "json", "--quiet", "."];
        let bytes = execute_sidecar(
            input.executor,
            "oxlint",
            &args,
            input.timeout_secs,
            SidecarExitPolicy::StrictZeroOnly,
        )
        .await?;
        serde_json::from_slice::<UxContractsPayload>(&bytes).map_err(|e| SidecarError::ParseError {
            reason: e.to_string(),
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LintViolation {
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub file_path: String,
    pub line: u32,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StaticAnalysisPayload {
    pub violations: Vec<LintViolation>,
    pub files_analyzed: u32,
    pub linter_name: String,
}

pub struct StaticAnalysisInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
}

pub struct StaticAnalysisSidecar;

impl StaticAnalysisSidecar {
    /// Extrai as violações de qualidade de código usando um linter no sandbox.
    pub async fn extract<E: SandboxExecutor>(
        input: StaticAnalysisInput<'_, E>,
        linter: &str,
        args: &[&str],
    ) -> Result<StaticAnalysisPayload, SidecarError> {
        let bytes = execute_sidecar(
            input.executor,
            linter,
            args,
            input.timeout_secs,
            SidecarExitPolicy::AllowFindingsExitOne,
        )
        .await?;
        serde_json::from_slice::<StaticAnalysisPayload>(&bytes).map_err(|e| SidecarError::ParseError {
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::path::PathBuf;

    // Mock do SandboxExecutor que simula respostas customizadas para os testes.
    struct MockExecutor {
        repo_path: PathBuf,
        // Usamos Mutex por causa de interior mutability se precisarmos contar chamadas,
        // mas aqui uma resposta estática ou configurável por teste basta.
        response: Mutex<Result<Vec<u8>, SandboxError>>,
    }

    impl MockExecutor {
        fn new(response: Result<Vec<u8>, SandboxError>) -> Self {
            Self {
                repo_path: PathBuf::from("/mock/ramdisk/repo"),
                response: Mutex::new(response),
            }
        }
    }

    impl SandboxExecutor for MockExecutor {
        async fn execute(&self, _command: &str, _args: &[&str], _timeout_secs: u64) -> Result<Vec<u8>, SandboxError> {
            let guard = self.response.lock().unwrap();
            guard.clone()
        }

        fn repo_path(&self) -> &Path {
            &self.repo_path
        }
    }


    #[tokio::test]
    async fn test_extract_success() {
        let valid_json = r#"{
            "symbols": [
                {
                    "name": "fn_test",
                    "kind": "function",
                    "file_path": "src/lib.rs",
                    "start_line": 1,
                    "end_line": 5,
                    "signature": "fn fn_test()"
                }
            ],
            "dependency_edges": [
                {
                    "source_file": "src/main.rs",
                    "target": "src/lib.rs",
                    "edge_type": "use"
                }
            ],
            "files_processed": 2
        }"#;

        let executor = MockExecutor::new(Ok(valid_json.as_bytes().to_vec()));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ter sucesso: {:?}", result);
        let payload = result.unwrap();
        assert_eq!(payload.files_processed, 2);
        assert_eq!(payload.symbols.len(), 1);
        assert_eq!(payload.symbols[0].name, "fn_test");
        assert_eq!(payload.dependency_edges.len(), 1);
        assert_eq!(payload.dependency_edges[0].edge_type, "use");
    }

    #[tokio::test]
    async fn test_binary_not_found() {
        // Simula erro de comando não encontrado
        let spawn_err = SandboxError::ProcessSpawnFailed {
            reason: "program not found (os error 2)".to_string(),
        };
        let executor = MockExecutor::new(Err(spawn_err));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::BinaryNotFound {
                binary: "jcodemunch-mcp".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_execution_failed() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 2,
            stderr: "fatal error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(Err(run_err));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "exit code 2: fatal error".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_timeout_propagation() {
        let executor = MockExecutor::new(Err(SandboxError::Timeout));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 45,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::Timeout { timeout_secs: 45 })
        );
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let corrup_bytes = b"{invalid_json_here".to_vec();
        let executor = MockExecutor::new(Ok(corrup_bytes));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        match result {
            Err(SidecarError::ParseError { reason }) => {
                assert!(reason.contains("key must be a string") || reason.contains("expected"));
            }
            other => panic!("Esperava ParseError, obteve: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_repo_payload_fails_closed() {
        let empty_json = r#"{
            "symbols": [],
            "dependency_edges": [],
            "files_processed": 0
        }"#;

        let executor = MockExecutor::new(Ok(empty_json.as_bytes().to_vec()));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch-mcp returned an empty AST payload".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_empty_stdout_fails_closed() {
        let executor = MockExecutor::new(Ok(Vec::new()));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "jcodemunch-mcp returned empty stdout".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_exit_code_1_fails_for_jcodemunch() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "usage error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(Err(run_err));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
            persist_artifact: None,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "exit code 1: usage error".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_oxc_extract_success() {
        let valid_json = r#"{
            "components": [
                {
                    "name": "Button",
                    "file_path": "src/components/Button.tsx",
                    "framework": "react",
                    "props": [
                        {
                            "name": "disabled",
                            "prop_type": "boolean",
                            "has_default": true,
                            "required": false
                        }
                    ],
                    "events": ["click"],
                    "is_default_export": true
                }
            ],
            "files_analyzed": 5
        }"#;

        let executor = MockExecutor::new(Ok(valid_json.as_bytes().to_vec()));
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert!(result.is_ok(), "Extração do OXC deveria ter sucesso: {:?}", result);
        let payload = result.unwrap();
        assert_eq!(payload.files_analyzed, 5);
        assert_eq!(payload.components.len(), 1);
        assert_eq!(payload.components[0].name, "Button");
        assert_eq!(payload.components[0].framework, "react");
        assert_eq!(payload.components[0].props.len(), 1);
        assert_eq!(payload.components[0].props[0].name, "disabled");
        assert_eq!(payload.components[0].props[0].prop_type, "boolean");
        assert!(!payload.components[0].props[0].required);
        assert_eq!(payload.components[0].events.len(), 1);
        assert_eq!(payload.components[0].events[0], "click");
        assert!(payload.components[0].is_default_export);
    }

    #[tokio::test]
    async fn test_oxc_binary_not_found() {
        let spawn_err = SandboxError::ProcessSpawnFailed {
            reason: "program not found (os error 2)".to_string(),
        };
        let executor = MockExecutor::new(Err(spawn_err));
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::BinaryNotFound {
                binary: "oxlint".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_oxc_execution_failed() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 2,
            stderr: "oxlint crashed".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(Err(run_err));
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "exit code 2: oxlint crashed".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_oxc_timeout_propagation() {
        let executor = MockExecutor::new(Err(SandboxError::Timeout));
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 45,
        };

        let result = OxcSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::Timeout { timeout_secs: 45 })
        );
    }

    #[tokio::test]
    async fn test_oxc_invalid_json() {
        let corrup_bytes = b"{invalid_json".to_vec();
        let executor = MockExecutor::new(Ok(corrup_bytes));
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        match result {
            Err(SidecarError::ParseError { reason }) => {
                assert!(reason.contains("expected value") || reason.contains("key must be a string") || reason.contains("expected"));
            }
            other => panic!("Esperava ParseError, obteve: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_oxc_empty_repo_valid_json() {
        let empty_json = r#"{
            "components": [],
            "files_analyzed": 0
        }"#;

        let executor = MockExecutor::new(Ok(empty_json.as_bytes().to_vec()));
        let input = OxcInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = OxcSidecar::extract(input).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert_eq!(payload.files_analyzed, 0);
        assert!(payload.components.is_empty());
    }

    #[tokio::test]
    async fn test_static_analysis_success_exit_1() {
        let valid_json = r#"{
            "violations": [
                {
                    "rule_id": "rule_1",
                    "severity": "error",
                    "message": "msg 1",
                    "file_path": "src/main.rs",
                    "line": 10,
                    "column": 5
                }
            ],
            "files_analyzed": 1,
            "linter_name": "ruff"
        }"#;

        // Simula exit code 1 via variante estruturada
        let err_exit_1 = SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "issues found".to_string(),
            stdout: valid_json.as_bytes().to_vec(),
        };
        let executor = MockExecutor::new(Err(err_exit_1));
        let input = StaticAnalysisInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = StaticAnalysisSidecar::extract(input, "ruff", &["check"]).await;
        assert!(result.is_ok(), "Exit code 1 deveria ser sucesso para análise estática: {:?}", result);
        let payload = result.unwrap();
        assert_eq!(payload.violations.len(), 1);
        assert_eq!(payload.violations[0].rule_id, "rule_1");
    }

    #[tokio::test]
    async fn test_static_analysis_success_exit_0() {
        let empty_json = r#"{
            "violations": [],
            "files_analyzed": 10,
            "linter_name": "ruff"
        }"#;

        let executor = MockExecutor::new(Ok(empty_json.as_bytes().to_vec()));
        let input = StaticAnalysisInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = StaticAnalysisSidecar::extract(input, "ruff", &["check"]).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert!(payload.violations.is_empty());
        assert_eq!(payload.files_analyzed, 10);
    }

    #[tokio::test]
    async fn test_static_analysis_execution_failed_exit_2() {
        let run_err = SandboxError::ProcessNonZeroExit {
            exit_code: 2,
            stderr: "config error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(Err(run_err));
        let input = StaticAnalysisInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = StaticAnalysisSidecar::extract(input, "ruff", &["check"]).await;
        assert!(matches!(result, Err(SidecarError::ExecutionFailed { .. })));
    }
}
