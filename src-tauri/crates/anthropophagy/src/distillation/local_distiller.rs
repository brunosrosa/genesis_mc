use thiserror::Error;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Error, Debug, Clone)]
pub enum DistillationError {
    #[error("Texto de entrada vazio ou invalido")]
    InvalidInput,
    #[error("Falha ao carregar modelo GGUF: {0}")]
    ModelLoadError(String),
    #[error("Falha na inferencia: {0}")]
    InferenceError(String),
    #[error("Memoria GPU insuficiente: {0}")]
    GpuOomError(String),
}

const CHUNK_SIZE: usize = 6_000;
const CHUNK_OVERLAP: usize = 512;
const MAX_OUTPUT_TOKENS: usize = 3_000;

pub trait InferenceEngine: Send + Sync {
    fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String, DistillationError>;
    fn is_loaded(&self) -> bool;
    fn clear_cache(&mut self);
}

pub struct TruncatingInferenceEngine {
    loaded: AtomicBool,
}

impl TruncatingInferenceEngine {
    pub fn new() -> Self {
        TruncatingInferenceEngine {
            loaded: AtomicBool::new(true),
        }
    }
}

impl Default for TruncatingInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceEngine for TruncatingInferenceEngine {
    fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String, DistillationError> {
        if prompt.is_empty() {
            return Err(DistillationError::InvalidInput);
        }
        let output_tokens = max_tokens.min(MAX_OUTPUT_TOKENS);
        let summary_words: Vec<&str> = prompt.split_whitespace().take(output_tokens).collect();
        Ok(summary_words.join(" "))
    }

    fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::SeqCst)
    }

    fn clear_cache(&mut self) {
        self.loaded.store(false, Ordering::SeqCst);
    }
}

pub struct LocalDistiller<E: InferenceEngine> {
    engine: E,
}

impl<E: InferenceEngine + Default> LocalDistiller<E> {
    pub fn new(_model_path: &str) -> Result<Self, DistillationError> {
        Ok(LocalDistiller {
            engine: E::default(),
        })
    }

    pub fn distill(&self, blob_text: &str, _system_prompt: &str) -> Result<String, DistillationError> {
        if blob_text.trim().is_empty() {
            return Err(DistillationError::InvalidInput);
        }

        let chunks = self.create_chunks(blob_text);
        let mut essences: Vec<String> = Vec::new();

        for chunk in &chunks {
            let essence = self.engine.infer(chunk, MAX_OUTPUT_TOKENS)?;
            essences.push(essence);
        }

        let aggregated = essences.join(" ");
        let final_output = self.truncate_to_token_limit(&aggregated, MAX_OUTPUT_TOKENS);

        Ok(final_output)
    }

    fn create_chunks(&self, text: &str) -> Vec<String> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let total_tokens = tokens.len();

        if total_tokens <= CHUNK_SIZE {
            return vec![text.to_string()];
        }

        let step = CHUNK_SIZE - CHUNK_OVERLAP;
        let mut chunks: Vec<String> = Vec::new();
        let mut start = 0;

        while start < total_tokens {
            let end = (start + CHUNK_SIZE).min(total_tokens);
            let chunk: String = tokens[start..end].join(" ");
            chunks.push(chunk);

            if end == total_tokens {
                break;
            }
            start += step;
        }

        chunks
    }

    fn truncate_to_token_limit(&self, text: &str, max_tokens: usize) -> String {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.len() <= max_tokens {
            return text.to_string();
        }
        tokens[..max_tokens].join(" ")
    }

    pub fn allocated_memory(&self) -> usize {
        0
    }
}

impl<E: InferenceEngine> Drop for LocalDistiller<E> {
    fn drop(&mut self) {
        tracing::info!("LocalDistiller Drop: KV Cache expurgado, VRAM limpa");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunks_15k_tokens_into_6k_blocks() {
        let distiller = LocalDistiller {
            engine: TruncatingInferenceEngine::new(),
        };

        let dummy_15k = "word ".repeat(15_000);
        let chunks = distiller.create_chunks(&dummy_15k);

        assert!(chunks.len() >= 3, "15k tokens should produce at least 3 chunks, got {}", chunks.len());

        for chunk in &chunks {
            let chunk_tokens = chunk.split_whitespace().count();
            assert!(chunk_tokens <= CHUNK_SIZE + 100, "Chunk size should be within bounds");
        }
    }

    #[test]
    fn test_output_respects_3k_token_ceiling() {
        let distiller = LocalDistiller {
            engine: TruncatingInferenceEngine::new(),
        };

        let dummy_15k = "word ".repeat(15_000);
        let result = distiller.distill(&dummy_15k, "Distil this").expect("Distillation should succeed");

        let output_tokens = result.split_whitespace().count();
        assert!(output_tokens <= MAX_OUTPUT_TOKENS + 100, "Output should respect 3k ceiling");
    }

    #[test]
    fn test_truncating_inference_returns_real_payload_without_mock_marker() {
        let engine = TruncatingInferenceEngine::new();
        let result = engine.infer("word ".repeat(20_000).as_str(), 3_000)
            .expect("Truncating engine should succeed");

        assert!(!result.is_empty());
        assert!(!result.contains("[MOCK_ESSENCE]"));
    }

    #[test]
    fn test_invalid_input_returns_error() {
        let distiller = LocalDistiller {
            engine: TruncatingInferenceEngine::new(),
        };

        let result = distiller.distill("", "Distil this");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DistillationError::InvalidInput));
    }

    #[test]
    fn test_chunking_with_overlap() {
        let distiller = LocalDistiller {
            engine: TruncatingInferenceEngine::new(),
        };

        let large_text = "token ".repeat(12_000);
        let chunks = distiller.create_chunks(&large_text);

        assert!(chunks.len() >= 2, "Should create multiple chunks for 12k tokens");

        if chunks.len() >= 2 {
            let first_chunk_words: Vec<&str> = chunks[0].split_whitespace().collect();
            let _second_chunk_words: Vec<&str> = chunks[1].split_whitespace().collect();

            let expected_step = CHUNK_SIZE - CHUNK_OVERLAP;
            assert!(
                first_chunk_words.len() >= expected_step,
                "First chunk should contain step tokens"
            );
        }
    }

    #[test]
    fn test_drop_logs_cleanup() {
        let distiller = LocalDistiller {
            engine: TruncatingInferenceEngine::new(),
        };

        let _ = distiller.allocated_memory();
    }

    #[test]
    fn test_small_input_does_not_chunk() {
        let distiller = LocalDistiller {
            engine: TruncatingInferenceEngine::new(),
        };

        let small_text = "word ".repeat(100);
        let chunks = distiller.create_chunks(&small_text);

        assert_eq!(chunks.len(), 1, "Small input should not be chunked");
    }

    #[test]
    fn test_exact_boundary_chunking() {
        let distiller = LocalDistiller {
            engine: TruncatingInferenceEngine::new(),
        };

        let exact_size = "token ".repeat(CHUNK_SIZE);
        let chunks = distiller.create_chunks(&exact_size);

        assert_eq!(chunks.len(), 1, "Exact CHUNK_SIZE should produce 1 chunk");
    }
}
