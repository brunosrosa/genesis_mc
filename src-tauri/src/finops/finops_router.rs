use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingZone {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDestination {
    PassThrough,
    LocalModel { path: String },
    CloudCascade,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    pub token_count: usize,
    pub zone: RoutingZone,
    pub destination: RoutingDestination,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FinOpsRouterError {
    #[error("Falha ao ler arquivo blob: {0}")]
    FileReadError(String),
    #[error("Falha na codificação tiktoken: {0}")]
    TiktokenError(String),
}

const GREEN_THRESHOLD: usize = 16_000;
const YELLOW_MAX: usize = 64_000;

fn qwen_model_path() -> String {
    std::env::var("SODA_QWEN_MODEL_PATH").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".soda_data")
            .join("models")
            .join("Qwen3.5-4B-Q4_K_M.gguf")
            .to_string_lossy()
            .to_string()
    })
}

fn is_factory_cloud_only() -> bool {
    match std::env::var("SODA_FACTORY_CLOUD_ONLY") {
        Ok(val) => val.eq_ignore_ascii_case("true") || val == "1",
        Err(_) => false,
    }
}

pub struct FinOpsRouter;

impl FinOpsRouter {
    pub fn classify_blob(path: &PathBuf) -> Result<RoutingDecision, FinOpsRouterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| FinOpsRouterError::FileReadError(e.to_string()))?;

        let token_count = tiktoken_count(&content)?;

        let (zone, destination) = if token_count < GREEN_THRESHOLD {
            (RoutingZone::Green, RoutingDestination::PassThrough)
        } else if token_count <= YELLOW_MAX {
            if is_factory_cloud_only() {
                (RoutingZone::Yellow, RoutingDestination::CloudCascade)
            } else {
                (
                    RoutingZone::Yellow,
                    RoutingDestination::LocalModel {
                        path: qwen_model_path(),
                    },
                )
            }
        } else {
            (RoutingZone::Red, RoutingDestination::CloudCascade)
        };

        Ok(RoutingDecision { token_count, zone, destination })
    }
}

fn tiktoken_count(text: &str) -> Result<usize, FinOpsRouterError> {
    let encoding = tiktoken_rs::cl100k_base()
        .map_err(|e| FinOpsRouterError::TiktokenError(e.to_string()))?;
    let tokens = encoding.encode_ordinary(text);
    Ok(tokens.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_ID: AtomicU32 = AtomicU32::new(0);

    fn next_test_id() -> u32 {
        TEST_ID.fetch_add(1, Ordering::SeqCst)
    }

    fn create_temp_blob(content: &str) -> PathBuf {
        let dir = std::env::temp_dir();
        let id = next_test_id();
        let path = dir.join(format!("test_blob_{}_{}.txt", std::process::id(), id));
        fs::write(&path, content).unwrap();
        path
    }

    fn generate_tokens(count: usize) -> String {
        " hello".repeat(count).trim().to_string()
    }

    #[test]
    fn test_10k_tokens_returns_green_zone() {
        let content = generate_tokens(10_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Green);
        assert_eq!(decision.destination, RoutingDestination::PassThrough);
        assert!(decision.token_count >= 9_000 && decision.token_count <= 11_000);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_30k_tokens_returns_yellow_zone_with_qwen() {
        let content = generate_tokens(30_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Yellow);
        match decision.destination {
            RoutingDestination::LocalModel { ref path } => {
                assert!(path.contains("Qwen3.5-4B-Q4_K_M.gguf"));
            }
            _ => panic!("Expected LocalModel destination for Yellow zone"),
        }
        assert!(decision.token_count >= 28_000 && decision.token_count <= 32_000);

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_70k_tokens_returns_red_zone_with_cloud_cascade() {
        let content = generate_tokens(70_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Red);
        assert_eq!(decision.destination, RoutingDestination::CloudCascade);
        assert!(decision.token_count >= 68_000 && decision.token_count <= 72_000);

        fs::remove_file(path).ok();
    }

    #[test]
    fn smoke_test_real_blob_08_health_report() {
        let blob_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("semgrep")
            .join("blob_08_health.yml");

        if !blob_path.exists() {
            eprintln!("SKIP: blob_08_health.yml not found at {:?}", blob_path);
            return;
        }

        let result = FinOpsRouter::classify_blob(&blob_path);
        let decision = result.expect("Should be able to classify real blob");

        println!("\n=== SMOKE TEST: REAL BLOB_08_HEALTH.YML ===");
        println!("Arquivo: blob_08_health.yml");
        println!("Tokens: {}", decision.token_count);
        let zone_str = match decision.zone {
            RoutingZone::Green => "VERDE (Green)",
            RoutingZone::Yellow => "AMARELA (Yellow)",
            RoutingZone::Red => "VERMELHA (Red)",
        };
        println!("Zona: {}", zone_str);
        let dest_str = match &decision.destination {
            RoutingDestination::PassThrough => "Pass-Through",
            RoutingDestination::LocalModel { path } => path.as_str(),
            RoutingDestination::CloudCascade => "OpenRouter Cascade",
        };
        println!("Destino: {}", dest_str);
        println!("===========================================\n");
    }

    #[test]
    fn smoke_test_blob_08_from_sqlite_vault() {
        let db_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".soda_data")
            .join("soda_heuristic_vault.db");

        if !db_path.exists() {
            println!("\n=== SMOKE TEST: BLOB_08 FROM SQLITE VAULT ===");
            println!("STATUS: Database not found at {:?}", db_path);
            println!("O Harvester precisa ser executado primeiro para popular o vault.");
            println!("==============================================\n");
            return;
        }

        let conn = rusqlite::Connection::open(&db_path).expect("Failed to open vault DB");

        let result: Result<String, _> = conn.query_row(
            "SELECT cast(payload_blob as text) FROM artefatos_brutos WHERE artifact_type = 'blob_08_health_report' AND repo_id = 'aaif-goose/goose' LIMIT 1",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(payload) => {
                let token_count = {
                    let encoding = tiktoken_rs::cl100k_base().expect("Failed to create tiktoken encoder");
                    encoding.encode_ordinary(&payload).len()
                };

                let zone = if token_count < GREEN_THRESHOLD {
                    RoutingZone::Green
                } else if token_count <= YELLOW_MAX {
                    RoutingZone::Yellow
                } else {
                    RoutingZone::Red
                };

                let destination = match zone {
                    RoutingZone::Green => RoutingDestination::PassThrough,
                    RoutingZone::Yellow => RoutingDestination::LocalModel {
                        path: qwen_model_path(),
                    },
                    RoutingZone::Red => RoutingDestination::CloudCascade,
                };

                println!("\n=== SMOKE TEST: BLOB_08 FROM SQLITE VAULT ===");
                println!("Artefato: blob_08_health_report (from SQLite)");
                println!("Tokens: {}", token_count);
                let zone_str = match zone {
                    RoutingZone::Green => "VERDE (Green)",
                    RoutingZone::Yellow => "AMARELA (Yellow)",
                    RoutingZone::Red => "VERMELHA (Red)",
                };
                println!("Zona: {}", zone_str);
                let dest_str = match &destination {
                    RoutingDestination::PassThrough => "Pass-Through",
                    RoutingDestination::LocalModel { path } => path.as_str(),
                    RoutingDestination::CloudCascade => "OpenRouter Cascade",
                };
                println!("Destino: {}", dest_str);
                println!("==============================================\n");
            }
            Err(e) => {
                println!("\n=== SMOKE TEST: BLOB_08 FROM SQLITE VAULT ===");
                println!("STATUS: Query failed - {}", e);
                println!("O blob pode nao existir ainda no vault.");
                println!("==============================================\n");
            }
        }
    }

    #[test]
    fn test_30k_yellow_without_bypass_routes_to_local() {
        std::env::remove_var("SODA_FACTORY_CLOUD_ONLY");

        let content = generate_tokens(30_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Yellow);
        match decision.destination {
            RoutingDestination::LocalModel { ref path } => {
                assert!(path.contains("Qwen3.5-4B-Q4_K_M.gguf"));
            }
            _ => panic!("Expected LocalModel destination for Yellow zone without bypass"),
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_30k_yellow_with_bypass_true_routes_to_cloud() {
        std::env::set_var("SODA_FACTORY_CLOUD_ONLY", "true");

        let content = generate_tokens(30_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Yellow);
        assert_eq!(decision.destination, RoutingDestination::CloudCascade);

        fs::remove_file(path).ok();
        std::env::remove_var("SODA_FACTORY_CLOUD_ONLY");
    }

    #[test]
    fn test_30k_yellow_with_bypass_1_routes_to_cloud() {
        std::env::set_var("SODA_FACTORY_CLOUD_ONLY", "1");

        let content = generate_tokens(30_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Yellow);
        assert_eq!(decision.destination, RoutingDestination::CloudCascade);

        fs::remove_file(path).ok();
        std::env::remove_var("SODA_FACTORY_CLOUD_ONLY");
    }

    #[test]
    fn test_70k_red_ignores_bypass() {
        std::env::set_var("SODA_FACTORY_CLOUD_ONLY", "true");

        let content = generate_tokens(70_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Red);
        assert_eq!(decision.destination, RoutingDestination::CloudCascade);

        fs::remove_file(path).ok();
        std::env::remove_var("SODA_FACTORY_CLOUD_ONLY");
    }

    #[test]
    fn test_10k_green_ignores_bypass() {
        std::env::set_var("SODA_FACTORY_CLOUD_ONLY", "true");

        let content = generate_tokens(10_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");

        assert_eq!(decision.zone, RoutingZone::Green);
        assert_eq!(decision.destination, RoutingDestination::PassThrough);

        fs::remove_file(path).ok();
        std::env::remove_var("SODA_FACTORY_CLOUD_ONLY");
    }

    #[test]
    fn test_bypass_case_insensitive_true() {
        std::env::set_var("SODA_FACTORY_CLOUD_ONLY", "TRUE");

        let content = generate_tokens(30_000);
        let path = create_temp_blob(&content);

        let decision = FinOpsRouter::classify_blob(&path).expect("Should succeed");
        assert_eq!(decision.destination, RoutingDestination::CloudCascade);

        fs::remove_file(path).ok();
        std::env::remove_var("SODA_FACTORY_CLOUD_ONLY");
    }
}
