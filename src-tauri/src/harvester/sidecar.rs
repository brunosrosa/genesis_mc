use std::path::Path;
use thiserror::Error;
use serde::Deserialize;
use crate::harvester::sandbox::SandboxError;

/// Trait para abstrair a execução no sandbox, permitindo mocks nos testes.
#[allow(async_fn_in_trait)]
pub trait SandboxExecutor {
    async fn execute(&self, command: &str, args: &[&str]) -> Result<Vec<u8>, SandboxError>;
    fn repo_path(&self) -> &Path;
}

/// Implementação da trait SandboxExecutor para o SandboxHandle concreto.
impl SandboxExecutor for crate::harvester::sandbox::SandboxHandle {
    async fn execute(&self, command: &str, args: &[&str]) -> Result<Vec<u8>, SandboxError> {
        self.execute(command, args).await
    }

    fn repo_path(&self) -> &Path {
        self.repo_path()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SymbolOutline {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DependencyEdge {
    pub source_file: String,
    pub target: String,
    pub edge_type: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
}

/// Executa um binário sidecar no sandbox e retorna os bytes brutos do stdout.
/// Centraliza a tradução SandboxError → SidecarError para todos os sidecars.
async fn execute_sidecar<E: SandboxExecutor>(
    executor: &E,
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<Vec<u8>, SidecarError> {
    match executor.execute(binary, args).await {
        Ok(bytes) => Ok(bytes),
        Err(SandboxError::Timeout) => {
            Err(SidecarError::Timeout { timeout_secs })
        }
        Err(SandboxError::ProcessSpawnFailed { reason }) => {
            let lower_reason = reason.to_lowercase();
            if lower_reason.contains("not found") || lower_reason.contains("os error 2") {
                Err(SidecarError::BinaryNotFound {
                    binary: binary.to_string(),
                })
            } else {
                Err(SidecarError::ExecutionFailed { reason })
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
        let args = ["index", "--format", "json", "--stdout", "."];
        let bytes = execute_sidecar(input.executor, "jcodemunch", &args, input.timeout_secs).await?;
        serde_json::from_slice::<AstPayload>(&bytes).map_err(|e| SidecarError::ParseError {
            reason: e.to_string(),
        })
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
        let bytes = execute_sidecar(input.executor, "oxlint", &args, input.timeout_secs).await?;
        serde_json::from_slice::<UxContractsPayload>(&bytes).map_err(|e| SidecarError::ParseError {
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
        async fn execute(&self, _command: &str, _args: &[&str]) -> Result<Vec<u8>, SandboxError> {
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
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::BinaryNotFound {
                binary: "jcodemunch".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_execution_failed() {
        let run_err = SandboxError::ProcessSpawnFailed {
            reason: "Comando falhou com código Some(1): error: failed to compile".to_string(),
        };
        let executor = MockExecutor::new(Err(run_err));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert_eq!(
            result,
            Err(SidecarError::ExecutionFailed {
                reason: "Comando falhou com código Some(1): error: failed to compile".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_timeout_propagation() {
        let executor = MockExecutor::new(Err(SandboxError::Timeout));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 45,
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
    async fn test_empty_repo_valid_json() {
        let empty_json = r#"{
            "symbols": [],
            "dependency_edges": [],
            "files_processed": 0
        }"#;

        let executor = MockExecutor::new(Ok(empty_json.as_bytes().to_vec()));
        let input = JCodemunchInput {
            executor: &executor,
            timeout_secs: 30,
        };

        let result = JCodemunchSidecar::extract(input).await;
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert_eq!(payload.files_processed, 0);
        assert!(payload.symbols.is_empty());
        assert!(payload.dependency_edges.is_empty());
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
        let run_err = SandboxError::ProcessSpawnFailed {
            reason: "oxlint failed".to_string(),
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
                reason: "oxlint failed".to_string()
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
}
