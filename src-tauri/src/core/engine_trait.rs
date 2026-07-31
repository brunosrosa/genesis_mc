use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileFormat {
    Gguf,
    Safetensors,
    Onnx,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttentionType {
    MultiHead,
    GroupedQuery,
    SlidingWindow,
    MixtureOfExperts,
    StateSpaceModel,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RopeScalingType {
    None,
    Linear,
    Yarn,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyFeatures {
    pub file_format: FileFormat,
    pub family_raw: String,
    pub attention_type: AttentionType,
    pub rope_scaling: RopeScalingType,
    pub block_count: u32,
    pub head_count: u32,
    pub head_count_kv: u32,
    pub embedding_length: u32,
    pub context_length: u64,
    pub has_vision_projector: bool,
    pub has_mtp_adapter: bool,
    pub chat_template: Option<String>,
}

impl Default for TopologyFeatures {
    fn default() -> Self {
        Self {
            file_format: FileFormat::Gguf,
            family_raw: String::new(),
            attention_type: AttentionType::MultiHead,
            rope_scaling: RopeScalingType::None,
            block_count: 0,
            head_count: 0,
            head_count_kv: 0,
            embedding_length: 0,
            context_length: 4096,
            has_vision_projector: false,
            has_mtp_adapter: false,
            chat_template: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineSupportLevel {
    Native(u32),
    Fallback(u32),
    Unsupported(String),
}

pub trait EngineProbe: Send + Sync {
    fn engine_id(&self) -> &'static str;
    fn probe_support(&self, model_path: &Path, topology: &TopologyFeatures) -> EngineSupportLevel;
}

pub struct DefaultLlamaCppProbe;

impl EngineProbe for DefaultLlamaCppProbe {
    fn engine_id(&self) -> &'static str {
        "llama_cpp"
    }

    fn probe_support(&self, model_path: &Path, topology: &TopologyFeatures) -> EngineSupportLevel {
        if !model_path.exists() {
            return EngineSupportLevel::Unsupported("Arquivo de modelo inexistente no disco".to_string());
        }

        match topology.file_format {
            FileFormat::Gguf => {
                let lower = topology.family_raw.to_lowercase();
                if lower == "rwkv" || lower == "zamba2" || lower == "mamba" || lower == "mamba-ssm" {
                    EngineSupportLevel::Unsupported(format!("Arquitetura '{}' incompatível com llama.cpp", topology.family_raw))
                } else {
                    EngineSupportLevel::Native(100)
                }
            }
            _ => EngineSupportLevel::Unsupported("Formato incompatível com motor llama.cpp (requer GGUF)".to_string()),
        }
    }
}

pub struct LlamaVanguardProbe;

impl EngineProbe for LlamaVanguardProbe {
    fn engine_id(&self) -> &'static str {
        "llama_vanguard"
    }

    fn probe_support(&self, model_path: &Path, topology: &TopologyFeatures) -> EngineSupportLevel {
        if !model_path.exists() {
            return EngineSupportLevel::Unsupported("Arquivo de modelo inexistente no disco".to_string());
        }

        match topology.file_format {
            FileFormat::Gguf => {
                let lower = topology.family_raw.to_lowercase();
                if lower == "rwkv" || lower == "zamba2" || lower == "mamba" || lower == "mamba-ssm" {
                    EngineSupportLevel::Unsupported(format!("Arquitetura '{}' incompatível com LlamaVanguard", topology.family_raw))
                } else {
                    // Vanguard tem prioridade (200) para acionar o worker isolado com fallback gracioso
                    EngineSupportLevel::Native(200)
                }
            }
            _ => EngineSupportLevel::Unsupported("Formato incompatível com LlamaVanguard (requer GGUF)".to_string()),
        }
    }
}

pub struct EngineCascade {
    probes: Vec<Box<dyn EngineProbe>>,
}

impl EngineCascade {
    pub fn new() -> Self {
        let mut cascade = Self { probes: Vec::new() };
        cascade.register(Box::new(LlamaVanguardProbe));
        cascade.register(Box::new(DefaultLlamaCppProbe));
        cascade
    }

    pub fn register(&mut self, probe: Box<dyn EngineProbe>) {
        self.probes.push(probe);
    }
}

impl Default for EngineCascade {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineCascade {
    pub fn probe_best_engine(&self, model_path: &Path, topology: &TopologyFeatures) -> (String, EngineSupportLevel) {
        let mut best_engine = "none".to_string();
        let mut max_level = EngineSupportLevel::Unsupported("Nenhum motor disponível".to_string());
        let mut max_score = 0u32;

        for probe in &self.probes {
            let level = probe.probe_support(model_path, topology);
            match &level {
                EngineSupportLevel::Native(score) => {
                    if *score > max_score {
                        max_score = *score;
                        best_engine = probe.engine_id().to_string();
                        max_level = level;
                    }
                }
                EngineSupportLevel::Fallback(score) => {
                    if max_score == 0 && *score > 0 {
                        best_engine = probe.engine_id().to_string();
                        max_level = level;
                    }
                }
                EngineSupportLevel::Unsupported(_) => {
                    if best_engine == "none" {
                        max_level = level;
                    }
                }
            }
        }

        if best_engine == "none" && !self.probes.is_empty() {
            // Se nenhum motor deu Native ou Fallback, retorna o erro da última probe
            return ("unsupported".to_string(), max_level);
        }

        (best_engine, max_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_cascade_gguf_support() {
        let cascade = EngineCascade::new();
        let mut topology = TopologyFeatures::default();
        topology.family_raw = "nemotron_h".to_string();
        topology.file_format = FileFormat::Gguf;

        let dummy_path = Path::new("Cargo.toml"); // arquivo existente no workspace
        let (engine_id, support) = cascade.probe_best_engine(dummy_path, &topology);

        assert_eq!(engine_id, "llama_vanguard");
        assert!(matches!(support, EngineSupportLevel::Native(200)));
    }

    #[test]
    fn test_engine_cascade_unsupported_arch() {
        let cascade = EngineCascade::new();
        let mut topology = TopologyFeatures::default();
        topology.family_raw = "rwkv".to_string();
        topology.file_format = FileFormat::Gguf;

        let dummy_path = Path::new("Cargo.toml");
        let (engine_id, support) = cascade.probe_best_engine(dummy_path, &topology);

        assert_eq!(engine_id, "unsupported");
        assert!(matches!(support, EngineSupportLevel::Unsupported(_)));
    }
}
