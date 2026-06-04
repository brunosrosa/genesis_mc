use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum OrchestratorError {
    #[error("Repositorio invalido ou vazio: {0}")]
    InvalidRepoId(String),
    #[error("Falha ao buscar blobs no banco: {0}")]
    DatabaseReadError(String),
    #[error("Falha na destilacao: {0}")]
    DistillationError(String),
    #[error("Falha ao persistir essencia: {0}")]
    PersistError(String),
    #[error("Nenhum blob encontrado para o repo_id: {0}")]
    NoBlobsFound(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingZone {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub token_count: usize,
    pub zone: RoutingZone,
}

#[derive(Debug, Clone)]
pub struct EssenceResult {
    pub essence_name: String,
    pub payload_essence: String,
    pub token_count: usize,
    pub routing_zone: String,
}

pub trait Router: Send + Sync {
    fn classify(&self, payload: &str) -> RoutingDecision;
}

pub trait Distiller: Send + Sync {
    fn distill(&self, payload: &str, system_prompt: &str) -> Result<String, String>;
}

pub trait CloudCascadeTrait: Send + Sync {
    fn cascade_distill(&self, payload: &str, system_prompt: &str) -> Result<String, String>;
}

pub trait DbReader: Send + Sync {
    fn fetch_blobs(&self, repo_id: &str) -> Result<Vec<BlobRecord>, String>;
}

pub trait DbWriter: Send + Sync {
    fn insert_essence(&self, essence: &EssenceResult, repo_id: &str) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct BlobRecord {
    pub artifact_type: String,
    pub payload_blob: String,
}

pub struct Phase1_5Orchestrator<R: Router, D: Distiller, C: CloudCascadeTrait, Reader: DbReader, Writer: DbWriter> {
    router: R,
    local_distiller: D,
    cloud_cascade: C,
    db_reader: Reader,
    db_writer: Writer,
}

impl<R: Router, D: Distiller, C: CloudCascadeTrait, Reader: DbReader, Writer: DbWriter>
    Phase1_5Orchestrator<R, D, C, Reader, Writer>
{
    pub fn new(
        router: R,
        local_distiller: D,
        cloud_cascade: C,
        db_reader: Reader,
        db_writer: Writer,
    ) -> Self {
        Phase1_5Orchestrator {
            router,
            local_distiller,
            cloud_cascade,
            db_reader,
            db_writer,
        }
    }

    pub fn orchestrate(&self, repo_id: &str) -> Result<(), OrchestratorError> {
        if repo_id.trim().is_empty() {
            return Err(OrchestratorError::InvalidRepoId(repo_id.to_string()));
        }

        let blobs = self
            .db_reader
            .fetch_blobs(repo_id)
            .map_err(OrchestratorError::DatabaseReadError)?;

        if blobs.is_empty() {
            return Err(OrchestratorError::NoBlobsFound(repo_id.to_string()));
        }

        for blob in blobs {
            let essence_name = self.convert_to_essence_name(&blob.artifact_type);

            let routing = self.router.classify(&blob.payload_blob);

            let (payload_essence, routing_zone) = match routing.zone {
                RoutingZone::Green => {
                    (blob.payload_blob.clone(), "Green".to_string())
                }
                RoutingZone::Yellow => {
                    let result = self
                        .local_distiller
                        .distill(&blob.payload_blob, "Distil this repository artifact");
                    match result {
                        Ok(essence) => (essence, "Yellow".to_string()),
                        Err(e) => {
                            return Err(OrchestratorError::DistillationError(e));
                        }
                    }
                }
                RoutingZone::Red => {
                    let result = self
                        .cloud_cascade
                        .cascade_distill(&blob.payload_blob, "Distil this repository artifact");
                    match result {
                        Ok(essence) => (essence, "Red".to_string()),
                        Err(e) => {
                            return Err(OrchestratorError::DistillationError(e));
                        }
                    }
                }
            };

            let essence_result = EssenceResult {
                essence_name,
                payload_essence,
                token_count: routing.token_count,
                routing_zone,
            };

            self.db_writer
                .insert_essence(&essence_result, repo_id)
                .map_err(OrchestratorError::PersistError)?;
        }

        Ok(())
    }

    fn convert_to_essence_name(&self, artifact_type: &str) -> String {
        if let Some(rest) = artifact_type.strip_prefix("blob_") {
            format!("_essence_{}", rest)
        } else {
            format!("_essence_{}", artifact_type)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    type EssenceInsertRecord = (String, String, usize, String);
    type SharedEssenceRecords = Arc<Mutex<Vec<EssenceInsertRecord>>>;

    struct MockRouter {
        decisions: Vec<RoutingDecision>,
        call_count: Arc<AtomicUsize>,
        call_sequence: Arc<Mutex<Vec<usize>>>,
    }

    impl MockRouter {
        fn new(decisions: Vec<RoutingDecision>) -> Self {
            MockRouter {
                decisions,
                call_count: Arc::new(AtomicUsize::new(0)),
                call_sequence: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl Router for MockRouter {
        fn classify(&self, _payload: &str) -> RoutingDecision {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            self.call_sequence.lock().unwrap().push(idx);
            if idx < self.decisions.len() {
                self.decisions[idx].clone()
            } else {
                self.decisions.last().unwrap().clone()
            }
        }
    }

    struct MockDistiller {
        results: Vec<Result<String, String>>,
        call_count: Arc<AtomicUsize>,
    }

    impl MockDistiller {
        fn new(results: Vec<Result<String, String>>) -> Self {
            MockDistiller {
                results,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl Distiller for MockDistiller {
        fn distill(&self, _payload: &str, _prompt: &str) -> Result<String, String> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            if idx < self.results.len() {
                self.results[idx].clone()
            } else {
                self.results.last().unwrap().clone()
            }
        }
    }

    struct MockCloudCascade {
        results: Vec<Result<String, String>>,
        call_count: Arc<AtomicUsize>,
    }

    impl MockCloudCascade {
        fn new(results: Vec<Result<String, String>>) -> Self {
            MockCloudCascade {
                results,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl CloudCascadeTrait for MockCloudCascade {
        fn cascade_distill(&self, _payload: &str, _prompt: &str) -> Result<String, String> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            if idx < self.results.len() {
                self.results[idx].clone()
            } else {
                self.results.last().unwrap().clone()
            }
        }
    }

    struct MockDbReader {
        blobs: Vec<BlobRecord>,
    }

    impl MockDbReader {
        fn new(blobs: Vec<BlobRecord>) -> Self {
            MockDbReader { blobs }
        }
    }

    impl DbReader for MockDbReader {
        fn fetch_blobs(&self, _repo_id: &str) -> Result<Vec<BlobRecord>, String> {
            Ok(self.blobs.clone())
        }
    }

    struct MockDbWriter {
        essences: SharedEssenceRecords,
        insert_count: Arc<AtomicUsize>,
    }

    impl Clone for MockDbWriter {
        fn clone(&self) -> Self {
            MockDbWriter {
                essences: Arc::clone(&self.essences),
                insert_count: Arc::clone(&self.insert_count),
            }
        }
    }

    impl MockDbWriter {
        fn new() -> Self {
            MockDbWriter {
                essences: Arc::new(Mutex::new(Vec::new())),
                insert_count: Arc::new(AtomicUsize::new(0)),
            }
        }
        fn get_insert_count(&self) -> usize {
            self.insert_count.load(Ordering::SeqCst)
        }
        fn get_essences(&self) -> Vec<EssenceInsertRecord> {
            self.essences.lock().unwrap().clone()
        }
    }

    impl DbWriter for MockDbWriter {
        fn insert_essence(&self, essence: &EssenceResult, repo_id: &str) -> Result<(), String> {
            self.insert_count.fetch_add(1, Ordering::SeqCst);
            self.essences.lock().unwrap().push((
                repo_id.to_string(),
                essence.essence_name.clone(),
                essence.token_count,
                essence.routing_zone.clone(),
            ));
            Ok(())
        }
    }

    #[test]
    fn test_essence_naming_conversion() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![RoutingDecision {
            token_count: 1000,
            zone: RoutingZone::Green,
        }]);
        let distiller = MockDistiller::new(vec![Ok("essence".to_string())]);
        let cloud = MockCloudCascade::new(vec![Ok("essence".to_string())]);
        let reader = MockDbReader::new(vec![BlobRecord {
            artifact_type: "blob_08_health_report".to_string(),
            payload_blob: "content".to_string(),
        }]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("aaif-goose/goose");

        assert!(result.is_ok());
        let essences = writer.get_essences();
        assert!(essences.iter().any(|(_, name, _, _)| name == "_essence_08_health_report"));
    }

    #[test]
    fn test_blob_10_naming_conversion() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![RoutingDecision {
            token_count: 1000,
            zone: RoutingZone::Green,
        }]);
        let distiller = MockDistiller::new(vec![Ok("essence".to_string())]);
        let cloud = MockCloudCascade::new(vec![Ok("essence".to_string())]);
        let reader = MockDbReader::new(vec![BlobRecord {
            artifact_type: "blob_10_soda_canon_context".to_string(),
            payload_blob: "content".to_string(),
        }]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("aaif-goose/goose");

        assert!(result.is_ok());
        let essences = writer.get_essences();
        assert!(essences.iter().any(|(_, name, _, _)| name == "_essence_10_soda_canon_context"));
    }

    #[test]
    fn test_sequential_processing_no_parallelism() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![
            RoutingDecision { token_count: 30_000, zone: RoutingZone::Yellow },
            RoutingDecision { token_count: 70_000, zone: RoutingZone::Red },
            RoutingDecision { token_count: 10_000, zone: RoutingZone::Green },
        ]);
        let distiller = MockDistiller::new(vec![Ok("distilled".to_string())]);
        let cloud = MockCloudCascade::new(vec![Ok("cloud_essence".to_string())]);
        let reader = MockDbReader::new(vec![
            BlobRecord { artifact_type: "blob_01".to_string(), payload_blob: "payload1".to_string() },
            BlobRecord { artifact_type: "blob_02".to_string(), payload_blob: "payload2".to_string() },
            BlobRecord { artifact_type: "blob_03".to_string(), payload_blob: "payload3".to_string() },
        ]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("aaif-goose/goose");

        assert!(result.is_ok());
        assert_eq!(writer.get_insert_count(), 3);
    }

    #[test]
    fn test_zone_routing_green() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![RoutingDecision {
            token_count: 10_000,
            zone: RoutingZone::Green,
        }]);
        let distiller = MockDistiller::new(vec![Ok("distilled".to_string())]);
        let cloud = MockCloudCascade::new(vec![Ok("cloud".to_string())]);
        let reader = MockDbReader::new(vec![BlobRecord {
            artifact_type: "blob_01".to_string(),
            payload_blob: "small_content".to_string(),
        }]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("aaif-goose/goose");

        assert!(result.is_ok());
        let essences = writer.get_essences();
        assert!(essences.iter().any(|(_, _, _, zone)| zone == "Green"));
    }

    #[test]
    fn test_zone_routing_yellow_routes_to_local_distiller() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![RoutingDecision {
            token_count: 30_000,
            zone: RoutingZone::Yellow,
        }]);
        let distiller = MockDistiller::new(vec![Ok("local_distilled".to_string())]);
        let cloud = MockCloudCascade::new(vec![Ok("cloud".to_string())]);
        let reader = MockDbReader::new(vec![BlobRecord {
            artifact_type: "blob_01".to_string(),
            payload_blob: "medium_content".to_string(),
        }]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("aaif-goose/goose");

        assert!(result.is_ok());
    }

    #[test]
    fn test_zone_routing_red_routes_to_cloud_cascade() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![RoutingDecision {
            token_count: 70_000,
            zone: RoutingZone::Red,
        }]);
        let distiller = MockDistiller::new(vec![Ok("local".to_string())]);
        let cloud = MockCloudCascade::new(vec![Ok("cloud_distilled".to_string())]);
        let reader = MockDbReader::new(vec![BlobRecord {
            artifact_type: "blob_01".to_string(),
            payload_blob: "large_content".to_string(),
        }]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("aaif-goose/goose");

        assert!(result.is_ok());
        let essences = writer.get_essences();
        assert!(essences.iter().any(|(_, _, _, zone)| zone == "Red"));
    }

    #[test]
    fn test_error_propagation() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![RoutingDecision {
            token_count: 30_000,
            zone: RoutingZone::Yellow,
        }]);
        let distiller = MockDistiller::new(vec![Err("GPU OOM".to_string())]);
        let cloud = MockCloudCascade::new(vec![Ok("cloud".to_string())]);
        let reader = MockDbReader::new(vec![BlobRecord {
            artifact_type: "blob_01".to_string(),
            payload_blob: "content".to_string(),
        }]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("aaif-goose/goose");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::DistillationError(_)));
    }

    #[test]
    fn test_invalid_repo_id() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![]);
        let distiller = MockDistiller::new(vec![]);
        let cloud = MockCloudCascade::new(vec![]);
        let reader = MockDbReader::new(vec![]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::InvalidRepoId(_)));
    }

    #[test]
    fn test_no_blobs_found_error() {
        let writer = MockDbWriter::new();

        let router = MockRouter::new(vec![]);
        let distiller = MockDistiller::new(vec![]);
        let cloud = MockCloudCascade::new(vec![]);
        let reader = MockDbReader::new(vec![]);

        let orchestrator =
            Phase1_5Orchestrator::new(router, distiller, cloud, reader, writer.clone());

        let result = orchestrator.orchestrate("nonexistent/repo");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::NoBlobsFound(_)));
    }
}
