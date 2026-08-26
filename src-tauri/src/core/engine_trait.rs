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

        let path_lower = model_path.to_string_lossy().to_lowercase();
        let fam_lower = topology.family_raw.to_lowercase();

        // RWKV puro utiliza motor linear próprio não-GGUF
        if fam_lower.contains("rwkv") || path_lower.contains("rwkv") {
            return EngineSupportLevel::Unsupported("Arquitetura RWKV requer runtime linear dedicado".to_string());
        }

        match topology.file_format {
            FileFormat::Gguf => EngineSupportLevel::Native(100),
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

        let path_lower = model_path.to_string_lossy().to_lowercase();
        let fam_lower = topology.family_raw.to_lowercase();

        // RWKV puro utiliza motor linear próprio não-GGUF
        if fam_lower.contains("rwkv") || path_lower.contains("rwkv") {
            return EngineSupportLevel::Unsupported("Arquitetura RWKV requer runtime linear dedicado".to_string());
        }

        match topology.file_format {
            FileFormat::Gguf => EngineSupportLevel::Native(200),
            _ => EngineSupportLevel::Unsupported("Formato incompatível com LlamaVanguard (requer GGUF)".to_string()),
        }
    }
}

// =============================================================================
// SOULS V4 — 6 novos probes para o cascade de 8 motores.
// =============================================================================

pub struct LlamaCpp4LogitProbe;

impl EngineProbe for LlamaCpp4LogitProbe {
    fn engine_id(&self) -> &'static str {
        "llama_cpp4_logit"
    }

    fn probe_support(&self, model_path: &Path, topology: &TopologyFeatures) -> EngineSupportLevel {
        let path_lower = model_path.to_string_lossy().to_lowercase();
        let fam_lower = topology.family_raw.to_lowercase();
        if fam_lower.contains("rwkv") || fam_lower.contains("zamba") || fam_lower.contains("mamba") || path_lower.contains("mamba") {
            return EngineSupportLevel::Unsupported(format!(
                "Arquitetura '{}' nao suporta logit probing (apenas transformers)",
                topology.family_raw
            ));
        }
        EngineSupportLevel::Native(150)
    }
}

pub struct MistralRsSidecarProbe;

impl EngineProbe for MistralRsSidecarProbe {
    fn engine_id(&self) -> &'static str {
        "mistral_rs_sidecar"
    }

    fn probe_support(&self, _model_path: &Path, _topology: &TopologyFeatures) -> EngineSupportLevel {
        #[cfg(feature = "mistral_backend")]
        return EngineSupportLevel::Native(210);

        #[cfg(not(feature = "mistral_backend"))]
        EngineSupportLevel::Fallback(80)
    }
}

pub struct BitnetProbe;

impl EngineProbe for BitnetProbe {
    fn engine_id(&self) -> &'static str {
        "bitnet"
    }

    fn probe_support(&self, model_path: &Path, topology: &TopologyFeatures) -> EngineSupportLevel {
        let path_lower = model_path.to_string_lossy().to_lowercase();
        let fam_lower = topology.family_raw.to_lowercase();
        if fam_lower.contains("bitnet") || path_lower.contains("i2_s") || path_lower.contains("i1_s") || path_lower.contains("bitnet") || path_lower.contains("ternary") {
            EngineSupportLevel::Native(220)
        } else {
            EngineSupportLevel::Unsupported("Bitnet só suporta modelos ternários (i2_s, i1_s, bitnet)".to_string())
        }
    }
}

pub struct PulpLeleProbe;

impl EngineProbe for PulpLeleProbe {
    fn engine_id(&self) -> &'static str {
        "pulp_lele"
    }

    fn probe_support(&self, _model_path: &Path, _topology: &TopologyFeatures) -> EngineSupportLevel {
        // Linear algebra AOT em CPU; sempre disponivel. Native medio (120).
        EngineSupportLevel::Native(120)
    }
}

pub struct BurnAgnosticProbe;

impl EngineProbe for BurnAgnosticProbe {
    fn engine_id(&self) -> &'static str {
        "burn_agnostic"
    }

    fn probe_support(&self, _model_path: &Path, _topology: &TopologyFeatures) -> EngineSupportLevel {
        // Agnostico de hardware (CubeCL). Fallback (90) ate integracao completa.
        EngineSupportLevel::Fallback(90)
    }
}

pub struct OrtScorerProbe;

impl EngineProbe for OrtScorerProbe {
    fn engine_id(&self) -> &'static str {
        "ort_scorer"
    }

    fn probe_support(&self, _model_path: &Path, _topology: &TopologyFeatures) -> EngineSupportLevel {
        // Scorers ONNX pequenos (GLiClass, BGE-reranker). Fallback (70) ate ONNX disponivel.
        EngineSupportLevel::Fallback(70)
    }
}

pub struct EngineCascade {
    probes: Vec<Box<dyn EngineProbe>>,
}

impl EngineCascade {
    pub fn new() -> Self {
        let mut cascade = Self { probes: Vec::new() };
        // V4 Topologia: 8 motores em ordem de prioridade decrescente.
        cascade.register(Box::new(LlamaVanguardProbe));
        cascade.register(Box::new(LlamaCpp4LogitProbe));
        cascade.register(Box::new(MistralRsSidecarProbe));
        cascade.register(Box::new(BitnetProbe));
        cascade.register(Box::new(PulpLeleProbe));
        cascade.register(Box::new(BurnAgnosticProbe));
        cascade.register(Box::new(OrtScorerProbe));
        cascade.register(Box::new(DefaultLlamaCppProbe));
        cascade
    }

    pub fn register(&mut self, probe: Box<dyn EngineProbe>) {
        self.probes.push(probe);
    }

    /// Numero de probes registrados no cascade (util para testes TDD).
    pub fn probe_count(&self) -> usize {
        self.probes.len()
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
        let topology = TopologyFeatures {
            family_raw: "nemotron_h".to_string(),
            file_format: FileFormat::Gguf,
            ..Default::default()
        };

        let dummy_path = Path::new("Cargo.toml"); // arquivo existente no workspace
        let (engine_id, support) = cascade.probe_best_engine(dummy_path, &topology);

        assert_eq!(engine_id, "llama_vanguard");
        assert!(matches!(support, EngineSupportLevel::Native(200)));
    }

    #[test]
    fn test_engine_cascade_unsupported_arch() {
        // V4 RESILIENCE: com 8 probes, o cascade SEMPRE encontra um engine (mesmo que
        // seja um architecture-agnostic como pulp_lele/burn/ort). O retorno "unsupported"
        // so acontece quando NENHUM probe casa.
        let cascade = EngineCascade::new();
        let topology = TopologyFeatures {
            family_raw: "rwkv".to_string(),
            file_format: FileFormat::Gguf,
            ..Default::default()
        };

        let dummy_path = Path::new("Cargo.toml");
        let (engine_id, support) = cascade.probe_best_engine(dummy_path, &topology);

        // V4: rwkv nao casa com llama-vanguard nem com llama_cpp4_logit, MAS casa
        // com pulp_lele (linear algebra agnostic). O cascade DEVE retornar um engine
        // architecture-agnostic em vez de "unsupported".
        assert_ne!(
            engine_id, "unsupported",
            "V4 cascade deve ser resiliente: architecture-agnostic probes cobrem rwkv"
        );
        // Confirma que o engine escolhido NAO e da familia llama (que rejeita state-space).
        assert_ne!(
            engine_id, "llama_vanguard",
            "Vanguard rejeita rwkv; engine deve ser outro"
        );
        assert!(matches!(support, EngineSupportLevel::Native(_) | EngineSupportLevel::Fallback(_)));
    }

    /// V4: o cascade DEVE expor 8 probes (Vanguard + 4 novos + 3 legacy/default).
    #[test]
    fn test_engine_cascade_has_8_probes() {
        let cascade = EngineCascade::new();
        assert_eq!(cascade.probe_count(), 8, "V4 cascade deve ter 8 probes");
    }

    /// V4: probe Bitnet discrimina modelos ternarios.
    #[test]
    fn test_bitnet_probe_only_supports_ternary_models() {
        let probe = BitnetProbe;
        let path_ternary = Path::new("/dev/null/model.i2_s.gguf");
        let path_normal = Path::new("/dev/null/model.gguf");

        assert!(matches!(
            probe.probe_support(path_ternary, &TopologyFeatures::default()),
            EngineSupportLevel::Native(180)
        ));
        assert!(matches!(
            probe.probe_support(path_normal, &TopologyFeatures::default()),
            EngineSupportLevel::Unsupported(_)
        ));
    }
}
