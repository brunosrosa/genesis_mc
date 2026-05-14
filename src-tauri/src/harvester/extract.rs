use tokio::fs;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use super::git::RepoPath;

/// Tamanho máximo permitido para um arquivo de manifesto (1 MiB).
const MAX_MANIFEST_SIZE: u64 = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestPayload {
    pub manifests: Vec<ManifestInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestInfo {
    pub file_name: String,
    pub dependencies: Vec<DependencyEntry>,
    pub dev_dependencies: Vec<DependencyEntry>,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyEntry {
    pub name: String,
    pub version_spec: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpsPayload {
    pub infra_files: Vec<InfraFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InfraFile {
    pub path: String,
    pub content: String,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ExtractionError {
    #[error("No manifest files found in repository root")]
    NotFound,

    #[error("Failed to parse manifest '{file}': {reason}")]
    ParseError { file: String, reason: String },

    #[error("Manifest file exceeds size limit ({size_bytes} bytes > {limit_bytes} bytes): {file}")]
    FileTooLarge {
        file: String,
        size_bytes: u64,
        limit_bytes: u64,
    },

    #[error("Filesystem error reading '{file}': {reason}")]
    IoError { file: String, reason: String },
}

pub struct ManifestInput<'a> {
    pub repo_path: &'a RepoPath,
}

pub struct ManifestExtractor;

pub struct OpsInput<'a> {
    pub repo_path: &'a RepoPath,
}

pub struct OpsBlueprintExtractor;

impl OpsBlueprintExtractor {
    pub async fn extract(input: OpsInput<'_>) -> Result<OpsPayload, ExtractionError> {
        let root_targets = [
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.yaml",
            "Makefile",
        ];

        let mut infra_files = Vec::new();

        // 1. Root files
        for &file_name in &root_targets {
            let path = input.repo_path.join(file_name);
            if let Some(infra) = Self::read_infra_file(&path, file_name).await? {
                infra_files.push(infra);
            }
        }

        // 2. Workflows (.github/workflows/) - Level 1 only
        let workflows_path = input.repo_path.join(".github/workflows");
        if let Ok(mut entries) = fs::read_dir(workflows_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".yml") || file_name.ends_with(".yaml") {
                        let path = entry.path();
                        let rel_path = format!(".github/workflows/{}", file_name);
                        if let Some(infra) = Self::read_infra_file(&path, &rel_path).await? {
                            infra_files.push(infra);
                        }
                    }
                }
            }
        }

        if infra_files.is_empty() {
            Err(ExtractionError::NotFound)
        } else {
            Ok(OpsPayload { infra_files })
        }
    }

    async fn read_infra_file(path: &std::path::Path, rel_path: &str) -> Result<Option<InfraFile>, ExtractionError> {
        let metadata = match fs::metadata(path).await {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ExtractionError::IoError {
                file: rel_path.to_string(),
                reason: e.to_string(),
            }),
        };

        let size = metadata.len();
        if size > MAX_MANIFEST_SIZE {
            return Err(ExtractionError::FileTooLarge {
                file: rel_path.to_string(),
                size_bytes: size,
                limit_bytes: MAX_MANIFEST_SIZE,
            });
        }

        let content = fs::read_to_string(path).await.map_err(|e| ExtractionError::IoError {
            file: rel_path.to_string(),
            reason: e.to_string(),
        })?;

        Ok(Some(InfraFile {
            path: rel_path.to_string(),
            content,
        }))
    }
}

use std::collections::BTreeMap;

impl ManifestExtractor {
    pub async fn extract(input: ManifestInput<'_>) -> Result<ManifestPayload, ExtractionError> {
        let targets = [
            "Cargo.toml",
            "package.json",
            "go.mod",
            "pyproject.toml",
            "requirements.txt",
            "pom.xml",
            "build.gradle",
            "build.gradle.kts",
        ];

        let mut manifests = Vec::new();
        let mut last_error = None;

        for &file_name in &targets {
            let path = input.repo_path.join(file_name);
            
            let metadata = match fs::metadata(&path).await {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    last_error = Some(ExtractionError::IoError { 
                        file: file_name.to_string(), 
                        reason: e.to_string() 
                    });
                    continue;
                }
            };

            let size = metadata.len();
            if size > MAX_MANIFEST_SIZE {
                last_error = Some(ExtractionError::FileTooLarge {
                    file: file_name.to_string(),
                    size_bytes: size,
                    limit_bytes: MAX_MANIFEST_SIZE,
                });
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    last_error = Some(ExtractionError::IoError { 
                        file: file_name.to_string(), 
                        reason: e.to_string() 
                    });
                    continue;
                }
            };

            let info_res = match file_name {
                "Cargo.toml" => Self::parse_cargo_toml(&content, file_name, size),
                "package.json" => Self::parse_package_json(&content, file_name, size),
                "requirements.txt" => Ok(Self::parse_requirements_txt(&content, file_name, size)),
                "pyproject.toml" => Self::parse_pyproject_toml(&content, file_name, size),
                _ => Ok(ManifestInfo {
                    file_name: file_name.to_string(),
                    dependencies: Vec::new(),
                    dev_dependencies: Vec::new(),
                    file_size_bytes: size,
                }),
            };

            match info_res {
                Ok(info) => manifests.push(info),
                Err(e) => last_error = Some(e),
            }
        }

        if manifests.is_empty() {
            Err(last_error.unwrap_or(ExtractionError::NotFound))
        } else {
            Ok(ManifestPayload { manifests })
        }
    }

    fn parse_cargo_toml(content: &str, file: &str, size: u64) -> Result<ManifestInfo, ExtractionError> {
        #[derive(Deserialize)]
        struct CargoManifest {
            dependencies: Option<BTreeMap<String, toml::Value>>,
            #[serde(rename = "dev-dependencies")]
            dev_dependencies: Option<BTreeMap<String, toml::Value>>,
        }

        let manifest: CargoManifest = toml::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        Ok(ManifestInfo {
            file_name: file.to_string(),
            dependencies: Self::map_toml_deps(manifest.dependencies),
            dev_dependencies: Self::map_toml_deps(manifest.dev_dependencies),
            file_size_bytes: size,
        })
    }

    fn map_toml_deps(deps: Option<BTreeMap<String, toml::Value>>) -> Vec<DependencyEntry> {
        deps.unwrap_or_default()
            .into_iter()
            .map(|(name, value)| {
                let version = match value {
                    toml::Value::String(s) => s,
                    toml::Value::Table(t) => t.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };
                DependencyEntry { name, version_spec: version }
            })
            .collect()
    }

    fn parse_package_json(content: &str, file: &str, size: u64) -> Result<ManifestInfo, ExtractionError> {
        #[derive(Deserialize)]
        struct PackageJson {
            dependencies: Option<BTreeMap<String, String>>,
            #[serde(rename = "devDependencies")]
            dev_dependencies: Option<BTreeMap<String, String>>,
        }

        let manifest: PackageJson = serde_json::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        Ok(ManifestInfo {
            file_name: file.to_string(),
            dependencies: manifest.dependencies.unwrap_or_default()
                .into_iter()
                .map(|(name, version_spec)| DependencyEntry { name, version_spec })
                .collect(),
            dev_dependencies: manifest.dev_dependencies.unwrap_or_default()
                .into_iter()
                .map(|(name, version_spec)| DependencyEntry { name, version_spec })
                .collect(),
            file_size_bytes: size,
        })
    }

    fn parse_requirements_txt(content: &str, file: &str, size: u64) -> ManifestInfo {
        let mut dependencies = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Simple parsing: name==version, name>=version, or just name
            let parts: Vec<&str> = if line.contains("==") {
                line.split("==").collect()
            } else if line.contains(">=") {
                line.split(">=").collect()
            } else {
                vec![line]
            };

            let name = parts[0].trim().to_string();
            let version = if parts.len() > 1 {
                parts[1].split_whitespace().next().unwrap_or("*").to_string()
            } else {
                "*".to_string()
            };

            dependencies.push(DependencyEntry { name, version_spec: version });
        }

        ManifestInfo {
            file_name: file.to_string(),
            dependencies,
            dev_dependencies: Vec::new(),
            file_size_bytes: size,
        }
    }

    fn parse_pyproject_toml(content: &str, file: &str, size: u64) -> Result<ManifestInfo, ExtractionError> {
        // pyproject.toml structure can vary (poetry, setuptools, flit)
        // We'll look for [project.dependencies] or [tool.poetry.dependencies]
        let doc: toml::Value = toml::from_str(content).map_err(|e| ExtractionError::ParseError {
            file: file.to_string(),
            reason: e.to_string(),
        })?;

        let mut dependencies = Vec::new();
        let dev_dependencies = Vec::new();

        // 1. Standard PEP 621 [project.dependencies]
        if let Some(deps) = doc.get("project").and_then(|p| p.get("dependencies")).and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(s) = dep.as_str() {
                    dependencies.push(DependencyEntry { name: s.to_string(), version_spec: "*".to_string() });
                }
            }
        }

        // 2. Poetry [tool.poetry.dependencies]
        if let Some(deps) = doc.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("dependencies")).and_then(|d| d.as_table()) {
            for (name, val) in deps {
                if name == "python" { continue; }
                let version = match val {
                    toml::Value::String(s) => s.clone(),
                    _ => "*".to_string(),
                };
                dependencies.push(DependencyEntry { name: name.clone(), version_spec: version });
            }
        }

        Ok(ManifestInfo {
            file_name: file.to_string(),
            dependencies,
            dev_dependencies,
            file_size_bytes: size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;

    #[tokio::test]
    async fn test_extract_cargo_toml() {
        let dir = TempDir::new().unwrap();
        let content = r#"[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
tempfile = "3"
"#;
        fs::write(dir.path().join("Cargo.toml"), content).await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        assert_eq!(result.manifests.len(), 1);
        let m = &result.manifests[0];
        assert_eq!(m.file_name, "Cargo.toml");
        assert!(m.dependencies.iter().any(|d| d.name == "serde" && d.version_spec == "1.0"));
        assert!(m.dev_dependencies.iter().any(|d| d.name == "tempfile" && d.version_spec == "3"));
    }

    #[tokio::test]
    async fn test_extract_package_json() {
        let dir = TempDir::new().unwrap();
        let content = r#"{
            "dependencies": {
                "react": "^18.0.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        }"#;
        fs::write(dir.path().join("package.json"), content).await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        let m = result.manifests.iter().find(|m| m.file_name == "package.json").unwrap();
        assert!(m.dependencies.iter().any(|d| d.name == "react" && d.version_spec == "^18.0.0"));
        assert!(m.dev_dependencies.iter().any(|d| d.name == "typescript" && d.version_spec == "^5.0.0"));
    }

    #[tokio::test]
    async fn test_extract_requirements_txt() {
        let dir = TempDir::new().unwrap();
        let content = "flask==2.0.0\nrequests>=2.25.0\n# comment\npydantic\n";
        fs::write(dir.path().join("requirements.txt"), content).await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        let m = result.manifests.iter().find(|m| m.file_name == "requirements.txt").unwrap();
        assert!(m.dependencies.iter().any(|d| d.name == "flask" && d.version_spec == "2.0.0"));
        assert!(m.dependencies.iter().any(|d| d.name == "requests" && d.version_spec == "2.25.0"));
        assert!(m.dependencies.iter().any(|d| d.name == "pydantic" && d.version_spec == "*"));
    }

    #[tokio::test]
    async fn test_no_manifests() {
        let dir = TempDir::new().unwrap();
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;
        assert_eq!(result.unwrap_err(), ExtractionError::NotFound);
    }

    #[tokio::test]
    async fn test_file_too_large() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Cargo.toml");
        
        // Criar um arquivo com mais de 1MB
        let mut file = std::fs::File::create(&path).unwrap();
        let buffer = vec![0u8; (MAX_MANIFEST_SIZE + 100) as usize];
        file.write_all(&buffer).unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;
        
        match result {
            Err(ExtractionError::FileTooLarge { file, .. }) => assert_eq!(file, "Cargo.toml"),
            _ => panic!("Deveria ter falhado com FileTooLarge, mas retornou {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_corrupted_manifest() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "invalid [ toml").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await;
        
        match result {
            Err(ExtractionError::ParseError { file, .. }) => assert_eq!(file, "Cargo.toml"),
            // Se houver apenas um manifesto e ele falhar, o erro propaga.
            // Se houver mais, ele seria ignorado e se sobrasse nenhum, NotFound ou ParseError do último?
            // O PRD diz: "O erro só propaga se TODOS os manifestos falharem."
            _ => panic!("Deveria ter falhado com ParseError, mas retornou {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_multiple_manifests() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[dependencies]").await.unwrap();
        fs::write(dir.path().join("package.json"), "{}").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        assert_eq!(result.manifests.len(), 2);
    }

    #[tokio::test]
    async fn test_partial_failure() {
        let dir = TempDir::new().unwrap();
        // Um válido e um corrompido
        fs::write(dir.path().join("Cargo.toml"), "[dependencies]").await.unwrap();
        fs::write(dir.path().join("package.json"), "invalid { json").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = ManifestExtractor::extract(ManifestInput { repo_path: &repo_path }).await.unwrap();
        
        // Deve conter apenas o Cargo.toml
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].file_name, "Cargo.toml");
    }

    #[tokio::test]
    async fn test_ops_extract_root_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM rust").await.unwrap();
        fs::write(dir.path().join("Makefile"), "build:").await.unwrap();
        fs::write(dir.path().join("docker-compose.yml"), "version: '3'").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = OpsBlueprintExtractor::extract(OpsInput { repo_path: &repo_path }).await.unwrap();
        
        assert_eq!(result.infra_files.len(), 3);
        assert!(result.infra_files.iter().any(|f| f.path == "Dockerfile"));
        assert!(result.infra_files.iter().any(|f| f.path == "Makefile"));
    }

    #[tokio::test]
    async fn test_ops_extract_workflows_shallow() {
        let dir = TempDir::new().unwrap();
        let workflows_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&workflows_dir).await.unwrap();
        
        fs::write(workflows_dir.join("ci.yml"), "name: CI").await.unwrap();
        fs::write(workflows_dir.join("deploy.yaml"), "name: Deploy").await.unwrap();
        
        // Criar subdiretório para provar que a recursão ignora
        let nested_dir = workflows_dir.join("nested");
        fs::create_dir_all(&nested_dir).await.unwrap();
        fs::write(nested_dir.join("ignored.yml"), "should be ignored").await.unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = OpsBlueprintExtractor::extract(OpsInput { repo_path: &repo_path }).await.unwrap();
        
        // Dockerfile/Makefile não existem aqui, então só os 2 workflows da raiz da pasta
        assert_eq!(result.infra_files.len(), 2);
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/ci.yml"));
        assert!(result.infra_files.iter().any(|f| f.path == ".github/workflows/deploy.yaml"));
        assert!(!result.infra_files.iter().any(|f| f.path.contains("ignored.yml")));
    }

    #[tokio::test]
    async fn test_ops_file_too_large() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Dockerfile");
        
        let mut file = std::fs::File::create(&path).unwrap();
        let buffer = vec![0u8; (MAX_MANIFEST_SIZE + 100) as usize];
        file.write_all(&buffer).unwrap();
        
        let repo_path = RepoPath(dir.path().to_path_buf());
        let result = OpsBlueprintExtractor::extract(OpsInput { repo_path: &repo_path }).await;
        
        match result {
            Err(ExtractionError::FileTooLarge { file, .. }) => assert_eq!(file, "Dockerfile"),
            _ => panic!("Deveria ter falhado com FileTooLarge para Dockerfile gigante"),
        }
    }
}
