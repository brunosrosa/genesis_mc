use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AssemblerError {
    #[error("Repositorio invalido: {0}")]
    InvalidRepoId(String),
    #[error("Falha ao buscar essencias: {0}")]
    DatabaseReadError(String),
    #[error("Essencia ausente no banco: {0}")]
    EssenceNotFound(String),
    #[error("Canon context ausente (blob_10): {0}")]
    CanonContextNotFound(String),
}

#[derive(Debug, Clone)]
pub struct Phase2Payloads {
    pub package_a: String,
    pub package_b: String,
    pub package_c: String,
}

pub trait DbReader: Send + Sync {
    fn fetch_essence(&self, repo_id: &str, essence_name: &str) -> Result<String, String>;
    fn fetch_raw_blob(&self, repo_id: &str, artifact_type: &str) -> Result<String, String>;
}

pub struct PackageAssembler<'a, DB: DbReader> {
    db: &'a DB,
}

fn compact_canon_context(blob_10: &str) -> String {
    let trimmed = blob_10.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let len_chars = trimmed.chars().count();
    let excerpt = trimmed.chars().take(4000).collect::<String>();
    format!(
        "canon_context_len_chars={}\ncanon_context_excerpt:\n{}\n",
        len_chars, excerpt
    )
}

fn detect_repo_kind(essence_02: &str, essence_04: &str, essence_05: &str) -> &'static str {
    let hay = format!("{}\n{}", essence_04, essence_05).to_ascii_lowercase();
    if hay.contains("kind: skilllibrary") {
        return "SkillLibrary";
    }
    if hay.contains("kind: contentrepo") {
        return "ContentRepo";
    }
    let dep = essence_02.to_ascii_lowercase();
    if dep.contains("stack_base: unknown")
        || dep.contains("stack_base: n/a")
        || dep.contains("stack_base: ")
            && dep.lines().any(|line| line.trim() == "stack_base:")
    {
        return "ContentRepo";
    }
    "CodeRepo"
}

impl<'a, DB: DbReader> PackageAssembler<'a, DB> {
    pub fn new(db: &'a DB) -> Self {
        PackageAssembler { db }
    }

    pub fn assemble(&self, repo_id: &str) -> Result<Phase2Payloads, AssemblerError> {
        if repo_id.trim().is_empty() {
            return Err(AssemblerError::InvalidRepoId(repo_id.to_string()));
        }

        let essence_01 = self
            .db
            .fetch_essence(repo_id, "_essence_01_promessa_readme")
            .map_err(AssemblerError::DatabaseReadError)?;
        let essence_03 = self
            .db
            .fetch_essence(repo_id, "_essence_03_test_intent")
            .map_err(AssemblerError::DatabaseReadError)?;
        let essence_11 = self
            .db
            .fetch_essence(repo_id, "_essence_11_ux_contracts")
            .map_err(AssemblerError::DatabaseReadError)?;

        let essence_04 = self
            .db
            .fetch_essence(repo_id, "_essence_04_repo_outline")
            .map_err(AssemblerError::DatabaseReadError)?;
        let essence_05 = self
            .db
            .fetch_essence(repo_id, "_essence_05_architecture_map")
            .map_err(AssemblerError::DatabaseReadError)?;

        let essence_02 = self
            .db
            .fetch_essence(repo_id, "_essence_02_dependency_manifest")
            .map_err(AssemblerError::DatabaseReadError)?;
        let essence_06 = self
            .db
            .fetch_essence(repo_id, "_essence_06_unsafe_hotspots")
            .map_err(AssemblerError::DatabaseReadError)?;
        let essence_07 = self
            .db
            .fetch_essence(repo_id, "_essence_07_ops_blueprint")
            .map_err(AssemblerError::DatabaseReadError)?;
        let essence_08 = self
            .db
            .fetch_essence(repo_id, "_essence_08_health_report")
            .map_err(AssemblerError::DatabaseReadError)?;
        let essence_09 = self
            .db
            .fetch_essence(repo_id, "_essence_09_community_meta")
            .map_err(AssemblerError::DatabaseReadError)?;

        let blob_10 = self
            .db
            .fetch_raw_blob(repo_id, "blob_10_souls_canon_context")
            .map_err(AssemblerError::DatabaseReadError)?;

        let canon_marker = "\n=== BLOB_10_CANON_CONTEXT ===\n";
        let canon_full = format!("{}{}", canon_marker, blob_10);
        let canon_compact = format!("{}{}", canon_marker, compact_canon_context(&blob_10));
        let repo_kind = detect_repo_kind(&essence_02, &essence_04, &essence_05);
        let kind_marker = format!("repo_kind={repo_kind}\n");

        let package_a = format!(
            "=== PACOTE A (PRODUTO/UX) ===\n{}{}\n{}\n{}\n{}\n=== FIM PACOTE A ===",
            kind_marker, essence_01, essence_03, essence_11, canon_compact
        );

        let package_b = format!(
            "=== PACOTE B (ARQUITETO) ===\n{}{}\n{}\n{}\n=== FIM PACOTE B ===",
            kind_marker, essence_04, essence_05, canon_full
        );

        let package_c = format!(
            "=== PACOTE C (OPS/AUDITOR) ===\n{}{}\n{}\n{}\n{}\n{}\n{}\n=== FIM PACOTE C ===",
            kind_marker, essence_02, essence_06, essence_07, essence_08, essence_09, canon_compact
        );

        Ok(Phase2Payloads {
            package_a,
            package_b,
            package_c,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct MockDbReader {
        essences: Arc<Mutex<HashMap<String, String>>>,
        raw_blobs: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MockDbReader {
        fn new() -> Self {
            MockDbReader {
                essences: Arc::new(Mutex::new(HashMap::new())),
                raw_blobs: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn add_essence(&self, name: &str, content: &str) {
            self.essences.lock().unwrap().insert(name.to_string(), content.to_string());
        }

        fn add_raw_blob(&self, name: &str, content: &str) {
            self.raw_blobs.lock().unwrap().insert(name.to_string(), content.to_string());
        }
    }

    impl DbReader for MockDbReader {
        fn fetch_essence(&self, _repo_id: &str, essence_name: &str) -> Result<String, String> {
            self.essences
                .lock()
                .unwrap()
                .get(essence_name)
                .cloned()
                .ok_or_else(|| format!("Essence not found: {}", essence_name))
        }

        fn fetch_raw_blob(&self, _repo_id: &str, artifact_type: &str) -> Result<String, String> {
            self.raw_blobs
                .lock()
                .unwrap()
                .get(artifact_type)
                .cloned()
                .ok_or_else(|| format!("Raw blob not found: {}", artifact_type))
        }
    }

    #[test]
    fn test_package_a_contains_correct_essences() {
        let db = MockDbReader::new();
        db.add_essence("_essence_01_promessa_readme", "[ESSENCE_01_CONTENT]");
        db.add_essence("_essence_03_test_intent", "[ESSENCE_03_CONTENT]");
        db.add_essence("_essence_11_ux_contracts", "[ESSENCE_11_CONTENT]");
        db.add_essence("_essence_04_repo_outline", "[ESSENCE_04_CONTENT]");
        db.add_essence("_essence_05_architecture_map", "[ESSENCE_05_CONTENT]");
        db.add_essence("_essence_02_dependency_manifest", "[ESSENCE_02_CONTENT]");
        db.add_essence("_essence_06_unsafe_hotspots", "[ESSENCE_06_CONTENT]");
        db.add_essence("_essence_07_ops_blueprint", "[ESSENCE_07_CONTENT]");
        db.add_essence("_essence_08_health_report", "[ESSENCE_08_CONTENT]");
        db.add_essence("_essence_09_community_meta", "[ESSENCE_09_CONTENT]");
        db.add_raw_blob("blob_10_souls_canon_context", "[BLOB_10_CANON_CONTEXT]");

        let assembler = PackageAssembler::new(&db);
        let result = assembler.assemble("aaif-goose/goose").expect("Should succeed");

        assert!(result.package_a.contains("[ESSENCE_01_CONTENT]"));
        assert!(result.package_a.contains("[ESSENCE_03_CONTENT]"));
        assert!(result.package_a.contains("[ESSENCE_11_CONTENT]"));

        assert!(!result.package_a.contains("[ESSENCE_04_CONTENT]"));
        assert!(!result.package_a.contains("[ESSENCE_05_CONTENT]"));
    }

    #[test]
    fn test_package_b_contains_correct_essences() {
        let db = MockDbReader::new();
        db.add_essence("_essence_01_promessa_readme", "[ESSENCE_01_CONTENT]");
        db.add_essence("_essence_03_test_intent", "[ESSENCE_03_CONTENT]");
        db.add_essence("_essence_11_ux_contracts", "[ESSENCE_11_CONTENT]");
        db.add_essence("_essence_04_repo_outline", "[ESSENCE_04_CONTENT]");
        db.add_essence("_essence_05_architecture_map", "[ESSENCE_05_CONTENT]");
        db.add_essence("_essence_02_dependency_manifest", "[ESSENCE_02_CONTENT]");
        db.add_essence("_essence_06_unsafe_hotspots", "[ESSENCE_06_CONTENT]");
        db.add_essence("_essence_07_ops_blueprint", "[ESSENCE_07_CONTENT]");
        db.add_essence("_essence_08_health_report", "[ESSENCE_08_CONTENT]");
        db.add_essence("_essence_09_community_meta", "[ESSENCE_09_CONTENT]");
        db.add_raw_blob("blob_10_souls_canon_context", "[BLOB_10_CANON_CONTEXT]");

        let assembler = PackageAssembler::new(&db);
        let result = assembler.assemble("aaif-goose/goose").expect("Should succeed");

        assert!(result.package_b.contains("[ESSENCE_04_CONTENT]"));
        assert!(result.package_b.contains("[ESSENCE_05_CONTENT]"));

        assert!(!result.package_b.contains("[ESSENCE_01_CONTENT]"));
        assert!(!result.package_b.contains("[ESSENCE_11_CONTENT]"));
        assert!(!result.package_b.contains("[ESSENCE_02_CONTENT]"));
    }

    #[test]
    fn test_package_c_contains_correct_essences() {
        let db = MockDbReader::new();
        db.add_essence("_essence_01_promessa_readme", "[ESSENCE_01_CONTENT]");
        db.add_essence("_essence_03_test_intent", "[ESSENCE_03_CONTENT]");
        db.add_essence("_essence_11_ux_contracts", "[ESSENCE_11_CONTENT]");
        db.add_essence("_essence_04_repo_outline", "[ESSENCE_04_CONTENT]");
        db.add_essence("_essence_05_architecture_map", "[ESSENCE_05_CONTENT]");
        db.add_essence("_essence_02_dependency_manifest", "[ESSENCE_02_CONTENT]");
        db.add_essence("_essence_06_unsafe_hotspots", "[ESSENCE_06_CONTENT]");
        db.add_essence("_essence_07_ops_blueprint", "[ESSENCE_07_CONTENT]");
        db.add_essence("_essence_08_health_report", "[ESSENCE_08_CONTENT]");
        db.add_essence("_essence_09_community_meta", "[ESSENCE_09_CONTENT]");
        db.add_raw_blob("blob_10_souls_canon_context", "[BLOB_10_CANON_CONTEXT]");

        let assembler = PackageAssembler::new(&db);
        let result = assembler.assemble("aaif-goose/goose").expect("Should succeed");

        assert!(result.package_c.contains("[ESSENCE_02_CONTENT]"));
        assert!(result.package_c.contains("[ESSENCE_06_CONTENT]"));
        assert!(result.package_c.contains("[ESSENCE_07_CONTENT]"));
        assert!(result.package_c.contains("[ESSENCE_08_CONTENT]"));
        assert!(result.package_c.contains("[ESSENCE_09_CONTENT]"));

        assert!(!result.package_c.contains("[ESSENCE_01_CONTENT]"));
        assert!(!result.package_c.contains("[ESSENCE_03_CONTENT]"));
        assert!(!result.package_c.contains("[ESSENCE_11_CONTENT]"));
        assert!(!result.package_c.contains("[ESSENCE_04_CONTENT]"));
        assert!(!result.package_c.contains("[ESSENCE_05_CONTENT]"));
    }

    #[test]
    fn test_blob_10_appended_to_all_packages() {
        let db = MockDbReader::new();
        db.add_essence("_essence_01_promessa_readme", "[ESSENCE_01_CONTENT]");
        db.add_essence("_essence_03_test_intent", "[ESSENCE_03_CONTENT]");
        db.add_essence("_essence_11_ux_contracts", "[ESSENCE_11_CONTENT]");
        db.add_essence("_essence_04_repo_outline", "[ESSENCE_04_CONTENT]");
        db.add_essence("_essence_05_architecture_map", "[ESSENCE_05_CONTENT]");
        db.add_essence("_essence_02_dependency_manifest", "[ESSENCE_02_CONTENT]");
        db.add_essence("_essence_06_unsafe_hotspots", "[ESSENCE_06_CONTENT]");
        db.add_essence("_essence_07_ops_blueprint", "[ESSENCE_07_CONTENT]");
        db.add_essence("_essence_08_health_report", "[ESSENCE_08_CONTENT]");
        db.add_essence("_essence_09_community_meta", "[ESSENCE_09_CONTENT]");
        db.add_raw_blob("blob_10_souls_canon_context", "[BLOB_10_CANON_CONTEXT]");

        let assembler = PackageAssembler::new(&db);
        let result = assembler.assemble("aaif-goose/goose").expect("Should succeed");

        assert!(result.package_a.contains("canon_context_len_chars="));
        assert!(result.package_b.contains("[BLOB_10_CANON_CONTEXT]"));
        assert!(result.package_c.contains("canon_context_len_chars="));
    }

    #[test]
    fn test_blob_10_fetched_from_raw_blobs_not_destiled() {
        let db = MockDbReader::new();
        db.add_essence("_essence_01_promessa_readme", "[ESSENCE_01_CONTENT]");
        db.add_essence("_essence_03_test_intent", "[ESSENCE_03_CONTENT]");
        db.add_essence("_essence_11_ux_contracts", "[ESSENCE_11_CONTENT]");
        db.add_essence("_essence_04_repo_outline", "[ESSENCE_04_CONTENT]");
        db.add_essence("_essence_05_architecture_map", "[ESSENCE_05_CONTENT]");
        db.add_essence("_essence_02_dependency_manifest", "[ESSENCE_02_CONTENT]");
        db.add_essence("_essence_06_unsafe_hotspots", "[ESSENCE_06_CONTENT]");
        db.add_essence("_essence_07_ops_blueprint", "[ESSENCE_07_CONTENT]");
        db.add_essence("_essence_08_health_report", "[ESSENCE_08_CONTENT]");
        db.add_essence("_essence_09_community_meta", "[ESSENCE_09_CONTENT]");

        db.add_raw_blob("blob_10_souls_canon_context", "[BLOB_10_FROM_RAW]");

        db.add_essence("_essence_10_souls_canon_context", "[WRONG_ESSENCE_10]");

        let assembler = PackageAssembler::new(&db);
        let result = assembler.assemble("aaif-goose/goose").expect("Should succeed");

        assert!(result.package_a.contains("[BLOB_10_FROM_RAW]"));
        assert!(result.package_b.contains("[BLOB_10_FROM_RAW]"));
        assert!(result.package_c.contains("[BLOB_10_FROM_RAW]"));

        assert!(!result.package_a.contains("[WRONG_ESSENCE_10]"));
    }

    #[test]
    fn test_invalid_repo_id_returns_error() {
        let db = MockDbReader::new();
        let assembler = PackageAssembler::new(&db);

        let result = assembler.assemble("");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AssemblerError::InvalidRepoId(_)));
    }

    #[test]
    fn test_missing_essence_returns_error() {
        let db = MockDbReader::new();
        db.add_raw_blob("blob_10_souls_canon_context", "[BLOB_10]");

        let assembler = PackageAssembler::new(&db);
        let result = assembler.assemble("aaif-goose/goose");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AssemblerError::DatabaseReadError(_)));
    }

    #[test]
    fn test_missing_blob_10_returns_error() {
        let db = MockDbReader::new();
        db.add_essence("_essence_01_promessa_readme", "[ESSENCE_01]");
        db.add_essence("_essence_03_test_intent", "[ESSENCE_03]");
        db.add_essence("_essence_11_ux_contracts", "[ESSENCE_11]");
        db.add_essence("_essence_04_repo_outline", "[ESSENCE_04]");
        db.add_essence("_essence_05_architecture_map", "[ESSENCE_05]");
        db.add_essence("_essence_02_dependency_manifest", "[ESSENCE_02]");
        db.add_essence("_essence_06_unsafe_hotspots", "[ESSENCE_06]");
        db.add_essence("_essence_07_ops_blueprint", "[ESSENCE_07]");
        db.add_essence("_essence_08_health_report", "[ESSENCE_08]");
        db.add_essence("_essence_09_community_meta", "[ESSENCE_09]");

        let assembler = PackageAssembler::new(&db);
        let result = assembler.assemble("aaif-goose/goose");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AssemblerError::DatabaseReadError(_)));
    }
}
