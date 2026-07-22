use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::persist::ssot_injector::SsotInjector;
use crate::persist::sheets_utils::col_idx_to_a1;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

pub const OFFICIAL_FORMATTER_MODEL: &str = "deepseek/deepseek-chat";
pub const DEFAULT_BLOCK3_MODEL_CANDIDATES: &[&str] = &[
    "qwen/qwen3.7-plus",
    "moonshotai/kimi-k2.5",
    "openai/gpt-5.4-mini",
];

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Debug, Clone)]
struct Phase3TelemetryState {
    block: u8,
    label: String,
    block_started: Instant,
}

fn spawn_phase3_total_telemetry(
    repo_id: String,
    started_total: Instant,
    state: Arc<tokio::sync::Mutex<Phase3TelemetryState>>,
) -> AbortOnDrop {
    let handle = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        tick.tick().await;
        loop {
            tick.tick().await;
            let snapshot = state.lock().await.clone();
            info!(
                repo_id = %repo_id,
                total_s = started_total.elapsed().as_secs(),
                block = snapshot.block,
                block_label = %snapshot.label,
                block_s = snapshot.block_started.elapsed().as_secs(),
                "F3 Telemetry"
            );
        }
    });
    AbortOnDrop(handle)
}

fn spawn_phase3_block_telemetry(
    repo_id: String,
    started_total: Instant,
    state: Arc<tokio::sync::Mutex<Phase3TelemetryState>>,
) -> AbortOnDrop {
    let handle = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        tick.tick().await;
        loop {
            tick.tick().await;
            let snapshot = state.lock().await.clone();
            let total_s = started_total.elapsed().as_secs();
            if total_s > 0 && total_s.is_multiple_of(60) {
                continue;
            }
            info!(
                repo_id = %repo_id,
                total_s = total_s,
                block = snapshot.block,
                block_label = %snapshot.label,
                block_s = snapshot.block_started.elapsed().as_secs(),
                "F3 Telemetry"
            );
        }
    });
    AbortOnDrop(handle)
}

async fn set_phase3_block(state: &Arc<tokio::sync::Mutex<Phase3TelemetryState>>, block: u8, label: &str) {
    let mut guard = state.lock().await;
    guard.block = block;
    guard.label = label.to_string();
    guard.block_started = Instant::now();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TerminalClassification {
    #[serde(rename = "STACK_CORE_PLANO_A1")]
    StackCorePlanoA1,
    #[serde(rename = "STACK_CORE_PLANO_A2")]
    StackCorePlanoA2,
    #[serde(rename = "STACK_CORE_PLANO_B")]
    StackCorePlanoB,
    #[serde(rename = "INTEGRATE_AS_COMPONENT")]
    IntegrateAsComponent,
    #[serde(rename = "ABSORB_PARTIALLY", alias = "APROVADO_COM_RESSALVAS", alias = "APPROVED_WITH_REMARKS")]
    AbsorbPartially,
    #[serde(rename = "ABSORB_CONCEPT")]
    AbsorbConcept,
    #[serde(rename = "USE_AS_INSPIRATION_ONLY")]
    UseAsInspirationOnly,
    #[serde(rename = "REJECT", alias = "REJEITADO_DESCARTE", alias = "REJECT_DISCARD", alias = "REJECTED_DISCARD")]
    Reject,
    #[serde(rename = "SHORT-CIRCUIT")]
    ShortCircuit,
    #[default]
    #[serde(rename = "UNKNOWN", alias = "APROVADO_PARA_PRODUCAO", alias = "APPROVED_FOR_PRODUCTION")]
    Unknown,
}

impl TerminalClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StackCorePlanoA1 => "STACK_CORE_PLANO_A1",
            Self::StackCorePlanoA2 => "STACK_CORE_PLANO_A2",
            Self::StackCorePlanoB => "STACK_CORE_PLANO_B",
            Self::IntegrateAsComponent => "INTEGRATE_AS_COMPONENT",
            Self::AbsorbPartially => "ABSORB_PARTIALLY",
            Self::AbsorbConcept => "ABSORB_CONCEPT",
            Self::UseAsInspirationOnly => "USE_AS_INSPIRATION_ONLY",
            Self::Reject => "REJECT",
            Self::ShortCircuit => "SHORT-CIRCUIT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CannibalizationAction {
    #[serde(rename = "Data Model / Schema")]
    DataModelSchema,
    #[serde(rename = "Prompt / Heuristic Seed", alias = "EXTRAIR_SCRIPTS", alias = "EXTRACT_SCRIPTS")]
    PromptHeuristicSeed,
    #[serde(rename = "Protocol / Standard")]
    ProtocolStandard,
    #[serde(rename = "Concept", alias = "ABSORVER_LOGICA", alias = "ABSORB_LOGIC")]
    Concept,
    #[serde(rename = "UX Pattern")]
    UxPattern,
    #[serde(rename = "Canvas Refinement")]
    CanvasRefinement,
    #[serde(rename = "New Canvas")]
    NewCanvas,
    #[serde(rename = "Cognitive Layer")]
    CognitiveLayer,
    #[serde(rename = "Infra Capability")]
    InfraCapability,
    #[serde(rename = "Technical Runtime")]
    TechnicalRuntime,
    #[serde(rename = "Sandbox")]
    Sandbox,
    #[serde(rename = "Plugin")]
    Plugin,
    #[serde(rename = "External Contract")]
    ExternalContract,
    #[serde(rename = "No Absorption", alias = "NENHUMA", alias = "NONE")]
    NoAbsorption,
    #[default]
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl CannibalizationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DataModelSchema => "Data Model / Schema",
            Self::PromptHeuristicSeed => "Prompt / Heuristic Seed",
            Self::ProtocolStandard => "Protocol / Standard",
            Self::Concept => "Concept",
            Self::UxPattern => "UX Pattern",
            Self::CanvasRefinement => "Canvas Refinement",
            Self::NewCanvas => "New Canvas",
            Self::CognitiveLayer => "Cognitive Layer",
            Self::InfraCapability => "Infra Capability",
            Self::TechnicalRuntime => "Technical Runtime",
            Self::Sandbox => "Sandbox",
            Self::Plugin => "Plugin",
            Self::ExternalContract => "External Contract",
            Self::NoAbsorption => "No Absorption",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ArchitecturalCategory {
    #[default]
    #[serde(rename = "")]
    Unspecified,
    #[serde(rename = "CanvasUI")]
    CanvasUi,
    #[serde(rename = "UILibrary")]
    UiLibrary,
    #[serde(rename = "Memoria", alias = "Memoria_RAG", alias = "MEMORIA_RAG")]
    Memoria,
    #[serde(rename = "Roteamento", alias = "Roteamento_FinOps", alias = "ROTEAMENTO_FINOPS")]
    Roteamento,
    #[serde(rename = "Orquestracao", alias = "Orquestracao_Agentes", alias = "ORQUESTRACAO_AGENTES")]
    Orquestracao,
    #[serde(rename = "Seguranca", alias = "Seguranca_Sandbox", alias = "SEGURANCA_SANDBOX")]
    Seguranca,
    #[serde(
        rename = "Infraestrutura",
        alias = "Infraestrutura_Core",
        alias = "INFRAESTRUTURA_CORE",
        alias = "Knowledge_Extraction",
        alias = "KNOWLEDGE_EXTRACTION",
        alias = "Model_Serving",
        alias = "MODEL_SERVING"
    )]
    Infraestrutura,
    #[serde(rename = "Tooling", alias = "Tooling_Dev", alias = "TOOLING_DEV")]
    Tooling,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl ArchitecturalCategory {
    pub fn parse_strict(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(Self::Unspecified);
        }
        let out = match trimmed {
            "CanvasUI" => Self::CanvasUi,
            "UILibrary" => Self::UiLibrary,
            "Memoria" | "Memoria_RAG" => Self::Memoria,
            "Roteamento" | "Roteamento_FinOps" => Self::Roteamento,
            "Orquestracao" | "Orquestracao_Agentes" => Self::Orquestracao,
            "Seguranca" | "Seguranca_Sandbox" => Self::Seguranca,
            "Infraestrutura" | "Infraestrutura_Core" | "Knowledge_Extraction" | "Model_Serving" => {
                Self::Infraestrutura
            }
            "Tooling" | "Tooling_Dev" => Self::Tooling,
            _ => {
                return Err(format!(
                    "categoria_arquitetural invalida: '{}'. Valores permitidos: CanvasUI, UILibrary, Memoria, Roteamento, Orquestracao, Seguranca, Infraestrutura, Tooling",
                    trimmed
                ));
            }
        };
        Ok(out)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unspecified => "",
            Self::CanvasUi => "CanvasUI",
            Self::UiLibrary => "UILibrary",
            Self::Memoria => "Memoria",
            Self::Roteamento => "Roteamento",
            Self::Orquestracao => "Orquestracao",
            Self::Seguranca => "Seguranca",
            Self::Infraestrutura => "Infraestrutura",
            Self::Tooling => "Tooling",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IntegrationType {
    #[default]
    #[serde(rename = "UNKNOWN", alias = "REJECT", alias = "REJEITAR")]
    Unknown,
    #[serde(
        rename = "Biblioteca / Crate Nativa",
        alias = "INTEGRATE_AS_COMPONENT",
        alias = "INTEGRAR_COMO_COMPONENTE"
    )]
    BibliotecaCrateNativa,
    #[serde(rename = "Sidecar Efêmero")]
    SidecarEfemero,
    #[serde(rename = "Daemon / Background Service")]
    DaemonBackgroundService,
    #[serde(
        rename = "App Nativo / CLI Independente",
        alias = "REIMPLEMENT_INTERNALLY",
        alias = "REIMPLEMENTAR_INTERNAMENTE"
    )]
    AppNativoCliIndependente,
    #[serde(rename = "Middleware / Proxy")]
    MiddlewareProxy,
}

impl IntegrationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BibliotecaCrateNativa => "Biblioteca / Crate Nativa",
            Self::SidecarEfemero => "Sidecar Efêmero",
            Self::DaemonBackgroundService => "Daemon / Background Service",
            Self::AppNativoCliIndependente => "App Nativo / CLI Independente",
            Self::MiddlewareProxy => "Middleware / Proxy",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CapabilityNaturePrimary {
    #[serde(rename = "Context")]
    Context,
    #[serde(rename = "Memory")]
    Memory,
    #[serde(rename = "Perception")]
    Perception,
    #[serde(rename = "Expression")]
    Expression,
    #[serde(rename = "Execution")]
    Execution,
    #[serde(rename = "Observation")]
    Observation,
    #[serde(rename = "Documentation")]
    Documentation,
    #[serde(rename = "Planning")]
    Planning,
    #[serde(rename = "Curation")]
    Curation,
    #[serde(rename = "Identity")]
    Identity,
    #[serde(rename = "Infrastructure")]
    Infrastructure,
    #[serde(rename = "Multimodal IO")]
    MultimodalIo,
    #[serde(rename = "Sandbox")]
    Sandbox,
    #[serde(rename = "Serving")]
    Serving,
    #[serde(rename = "Retrieval")]
    Retrieval,
    #[serde(rename = "Synchronization")]
    Synchronization,
    #[default]
    #[serde(
        rename = "UNKNOWN",
        alias = "LIBRARY",
        alias = "BIBLIOTECA",
        alias = "TOOLING",
        alias = "FERRAMENTA",
        alias = "SERVICE",
        alias = "SERVICO",
        alias = "SERVIÇO",
        alias = "APPLICATION",
        alias = "APLICACAO",
        alias = "APLICAÇÃO",
        alias = "SYSTEM",
        alias = "SISTEMA",
        alias = "ALGORITHM",
        alias = "ALGORITMO",
        alias = "DATA_STRUCTURE",
        alias = "ESTRUTURA_DE_DADOS"
    )]
    Unknown,
}

impl CapabilityNaturePrimary {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Context => "Context",
            Self::Memory => "Memory",
            Self::Perception => "Perception",
            Self::Expression => "Expression",
            Self::Execution => "Execution",
            Self::Observation => "Observation",
            Self::Documentation => "Documentation",
            Self::Planning => "Planning",
            Self::Curation => "Curation",
            Self::Identity => "Identity",
            Self::Infrastructure => "Infrastructure",
            Self::MultimodalIo => "Multimodal IO",
            Self::Sandbox => "Sandbox",
            Self::Serving => "Serving",
            Self::Retrieval => "Retrieval",
            Self::Synchronization => "Synchronization",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ArchitecturalTopology {
    #[serde(rename = "Monolith", alias = "MONOLITH", alias = "MONOLITO")]
    Monolith,
    #[serde(rename = "Modular", alias = "MODULAR", alias = "PLUGIN", alias = "PLUGAVEL", alias = "PLUGÁVEL")]
    Modular,
    #[serde(rename = "Layered", alias = "LAYERED", alias = "EM_CAMADAS", alias = "CAMADAS")]
    Layered,
    #[serde(rename = "Contract-Driven")]
    ContractDriven,
    #[serde(rename = "Runtime-Centric", alias = "MICROSERVICES", alias = "MICROSSERVICOS", alias = "MICROSSERVIÇOS")]
    RuntimeCentric,
    #[serde(rename = "Event-Driven", alias = "EVENT_DRIVEN", alias = "DIRIGIDO_A_EVENTOS")]
    EventDriven,
    #[serde(rename = "Graph-Centric")]
    GraphCentric,
    #[serde(rename = "Pipeline-Centric", alias = "PIPELINE")]
    PipelineCentric,
    #[serde(rename = "Hybrid")]
    Hybrid,
    #[default]
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl ArchitecturalTopology {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Monolith => "Monolith",
            Self::Modular => "Modular",
            Self::Layered => "Layered",
            Self::ContractDriven => "Contract-Driven",
            Self::RuntimeCentric => "Runtime-Centric",
            Self::EventDriven => "Event-Driven",
            Self::GraphCentric => "Graph-Centric",
            Self::PipelineCentric => "Pipeline-Centric",
            Self::Hybrid => "Hybrid",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalStability {
    #[default]
    #[serde(alias = "ESTAVEL", alias = "ESTÁVEL")]
    Stable,
    #[serde(alias = "EVOLUTIVO", alias = "EM_EVOLUCAO", alias = "EM_EVOLUÇÃO")]
    Evolving,
    #[serde(other)]
    Unknown,
}

impl TemporalStability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stable => "STABLE",
            Self::Evolving => "EVOLVING",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FitLevel4 {
    #[default]
    #[serde(alias = "BAIXA", alias = "BAIXO")]
    Low,
    #[serde(alias = "MEDIA", alias = "MÉDIA", alias = "MEDIO", alias = "MÉDIO")]
    Medium,
    #[serde(alias = "ALTA", alias = "ALTO")]
    High,
    #[serde(alias = "EXCELENTE")]
    Excellent,
    #[serde(other)]
    Unknown,
}

impl FitLevel4 {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Excellent => "EXCELLENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel4 {
    #[default]
    #[serde(alias = "BAIXA", alias = "BAIXO")]
    Low,
    #[serde(alias = "MEDIA", alias = "MÉDIA", alias = "MEDIO", alias = "MÉDIO")]
    Medium,
    #[serde(alias = "ALTA", alias = "ALTO")]
    High,
    #[serde(alias = "CRITICA", alias = "CRÍTICA", alias = "CRITICO", alias = "CRÍTICO")]
    Critical,
    #[serde(other)]
    Unknown,
}

impl RiskLevel4 {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DisciplineDependency {
    #[serde(rename = "Nenhuma", alias = "NENHUMA", alias = "NONE")]
    Nenhuma,
    #[serde(rename = "Baixa", alias = "BAIXA", alias = "LOW")]
    Baixa,
    #[serde(rename = "Média", alias = "MEDIA", alias = "MEDIUM", alias = "MÉDIA")]
    Media,
    #[serde(rename = "Alta", alias = "ALTA", alias = "HIGH")]
    Alta,
    #[serde(rename = "Crítica", alias = "CRITICA", alias = "CRITICAL", alias = "CRÍTICA")]
    Critica,
    #[default]
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl DisciplineDependency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nenhuma => "Nenhuma",
            Self::Baixa => "Baixa",
            Self::Media => "Média",
            Self::Alta => "Alta",
            Self::Critica => "Crítica",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DegradationBehavior {
    #[default]
    #[serde(alias = "GRACIOSO", alias = "GRACIOSA")]
    Graceful,
    #[serde(alias = "ACEITAVEL", alias = "ACEITÁVEL")]
    Acceptable,
    #[serde(alias = "FRAGIL", alias = "FRÁGIL")]
    Fragile,
    #[serde(alias = "CATASTROFICO", alias = "CATASTRÓFICO")]
    Catastrophic,
    #[serde(other)]
    Unknown,
}

impl DegradationBehavior {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Graceful => "GRACEFUL",
            Self::Acceptable => "ACCEPTABLE",
            Self::Fragile => "FRAGILE",
            Self::Catastrophic => "CATASTROPHIC",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Scale5 {
    #[default]
    #[serde(alias = "MUITO_BAIXA", alias = "MUITO_BAIXO")]
    VeryLow,
    #[serde(alias = "BAIXA", alias = "BAIXO")]
    Low,
    #[serde(alias = "MEDIA", alias = "MÉDIA", alias = "MEDIO", alias = "MÉDIO")]
    Medium,
    #[serde(alias = "ALTA", alias = "ALTO")]
    High,
    #[serde(alias = "EXCELENTE")]
    Excellent,
    #[serde(other)]
    Unknown,
}

impl Scale5 {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VeryLow => "VERY_LOW",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Excellent => "EXCELLENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BurdenLevel {
    #[default]
    #[serde(alias = "BAIXA", alias = "BAIXO")]
    Low,
    #[serde(alias = "MEDIA", alias = "MÉDIA", alias = "MEDIO", alias = "MÉDIO")]
    Medium,
    #[serde(alias = "ALTA", alias = "ALTO")]
    High,
    #[serde(alias = "MUITO_ALTA", alias = "MUITO_ALTO")]
    VeryHigh,
    #[serde(other)]
    Unknown,
}

impl BurdenLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::VeryHigh => "VERY_HIGH",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExtractionHorizon {
    #[default]
    #[serde(rename = "UNKNOWN")]
    Unknown,
    #[serde(rename = "IMEDIATO", alias = "IMMEDIATE", alias = "IMEDIATA")]
    Imediato,
    #[serde(rename = "CURTO_PRAZO", alias = "SHORT", alias = "CURTO", alias = "CURTA")]
    CurtoPrazo,
    #[serde(rename = "CURTO_MEDIO_PRAZO")]
    CurtoMedioPrazo,
    #[serde(rename = "MEDIO_PRAZO", alias = "MEDIUM", alias = "MEDIO", alias = "MÉDIO", alias = "MEDIA", alias = "MÉDIA")]
    MedioPrazo,
    #[serde(rename = "LONGO_PRAZO", alias = "LONG", alias = "LONGO", alias = "LONGA")]
    LongoPrazo,
    #[serde(rename = "REFERENCIAL_TEORICO")]
    ReferencialTeorico,
    #[serde(rename = "NUNCA", alias = "VERY_LONG", alias = "MUITO_LONGO", alias = "MUITO_LONGA")]
    Nunca,
}

impl ExtractionHorizon {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Imediato => "IMEDIATO",
            Self::CurtoPrazo => "CURTO_PRAZO",
            Self::CurtoMedioPrazo => "CURTO_MEDIO_PRAZO",
            Self::MedioPrazo => "MEDIO_PRAZO",
            Self::LongoPrazo => "LONGO_PRAZO",
            Self::ReferencialTeorico => "REFERENCIAL_TEORICO",
            Self::Nunca => "NUNCA",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeHorizon {
    #[default]
    #[serde(alias = "IMEDIATO", alias = "IMEDIATA")]
    Immediate,
    #[serde(alias = "CURTO", alias = "CURTA")]
    Short,
    #[serde(alias = "MEDIO", alias = "MÉDIO", alias = "MEDIA", alias = "MÉDIA")]
    Medium,
    #[serde(alias = "LONGO", alias = "LONGA")]
    Long,
    #[serde(alias = "MUITO_LONGO", alias = "MUITO_LONGA")]
    VeryLong,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl TimeHorizon {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Immediate => "IMMEDIATE",
            Self::Short => "SHORT",
            Self::Medium => "MEDIUM",
            Self::Long => "LONG",
            Self::VeryLong => "VERY_LONG",
            Self::Unknown => "UNKNOWN",
        }
    }
}

fn deserialize_lossy_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| serde::de::Error::custom("numero float invalido")),
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                warn!(raw = %raw, "Score float vazio; degradando para 0.0");
                return Ok(0.0);
            }
            let normalized = trimmed.replace(',', ".");
            normalized.parse::<f64>().map_err(|err| {
                serde::de::Error::custom(format!("float invalido '{}': {}", trimmed, err))
            })
        }
        serde_json::Value::Null => {
            warn!("Score float nulo; degradando para 0.0");
            Ok(0.0)
        }
        other => Err(serde::de::Error::custom(format!(
            "tipo invalido para float: {}",
            other
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block0Context {
    #[serde(rename = "status_atualizacao")]
    pub status_atualizacao: String,
    #[serde(rename = "status_fase")]
    pub status_fase: String,
    #[serde(rename = "project_name")]
    pub project_name: String,
    #[serde(rename = "repo_url")]
    pub repo_url: String,
    #[serde(rename = "repo_analised_version")]
    pub repo_analised_version: String,
    #[serde(rename = "ultima_versao_online")]
    pub ultima_versao_online: String,
    #[serde(rename = "lote_id")]
    pub lote_id: String,
    #[serde(rename = "data_ultima_analise")]
    pub data_ultima_analise: i64,
    #[serde(rename = "analise_origem")]
    pub analise_origem: String,
    #[serde(rename = "licenca")]
    pub licenca: String,
    #[serde(rename = "stack_base")]
    pub stack_base: String,
    #[serde(rename = "declared_description")]
    pub declared_description: String,
    #[serde(rename = "proposta_original_resumo")]
    pub proposta_original_resumo: Option<String>,
    #[serde(rename = "categoria_arquitetural")]
    pub categoria_arquitetural: Option<String>,
    #[serde(rename = "lente_a_sentido_prod_ux")]
    pub lente_a_sentido_prod_ux: String,
    #[serde(rename = "lente_b_estrutura_arq")]
    pub lente_b_estrutura_arq: String,
    #[serde(rename = "lente_c_realidade_ops")]
    pub lente_c_realidade_ops: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterSolutionsRow {
    pub status_atualizacao: String,
    pub status_fase: String,
    pub project_name: String,
    pub repo_url: String,
    pub repo_analised_version: String,
    pub ultima_versao_online: String,
    #[serde(default)]
    pub indicacao_otimista_canibalizacao: String,
    pub lote_id: String,
    pub data_ultima_analise: i64,
    pub analise_origem: String,
    pub licenca: String,
    pub stack_base: String,
    pub declared_description: String,
    pub declared_description_ptbr: String,
    pub lente_a_sentido_prod_ux: String,
    pub lente_b_estrutura_arq: String,
    pub lente_c_realidade_ops: String,
    pub proposta_original_resumo: String,
    pub visao_do_enxame: String,
    pub justificativa_decisao: String,
    pub executive_verdict: String,
    pub risco_principal: String,
    pub risco_linha_vermelha: String,
    pub observacoes: String,
    pub ouro_a_extrair: String,
    pub deep_pattern: String,
    pub transplantable_core: String,
    pub logic_math_heuristic: String,
    pub real_structural_problem: String,
    pub categoria_nuance_tecnica: String,
    pub integracao_papel_exato: String,
    pub must_components_prod_ux: String,
    pub must_components_arq: String,
    pub must_components_ops: String,
    pub detected_toxic_deps: String,
    pub do_not_absorb: String,
    pub where_ai_should_not_enter: String,
    pub classificacao_terminal: TerminalClassification,
    pub acao_de_canibalizacao: CannibalizationAction,
    pub categoria_arquitetural: ArchitecturalCategory,
    pub horizonte_extracao: ExtractionHorizon,
    pub tipo_integracao: IntegrationType,
    pub capability_nature_primary: CapabilityNaturePrimary,
    pub architectural_topology: ArchitecturalTopology,
    pub temporal_stability: TemporalStability,
    pub bare_metal_fit: FitLevel4,
    pub extractability_level: FitLevel4,
    pub runtime_sovereignty_fit: FitLevel4,
    pub local_first_fit: FitLevel4,
    pub adoptability_level: Scale5,
    pub longitudinal_sustainability: Scale5,
    pub maintenance_burden: BurdenLevel,
    pub onboarding_friction: BurdenLevel,
    pub observability_operational: Scale5,
    pub recoverability_level: Scale5,
    pub degradation_behavior: DegradationBehavior,
    pub curation_burden: BurdenLevel,
    pub evolution_cost: BurdenLevel,
    pub operability_level: FitLevel4,
    pub abandonment_risk: RiskLevel4,
    pub time_to_first_clear_value: TimeHorizon,
    pub imperfection_tolerance: Scale5,
    pub entropy_risk: RiskLevel4,
    pub design_misuse_risk: RiskLevel4,
    pub intrinsic_ethics_risk: RiskLevel4,
    pub discipline_dependency: DisciplineDependency,
    pub regulatory_risk: RiskLevel4,
    pub score_philosophical_fit: i64,
    pub score_bare_metal_fit: i64,
    pub score_architectural_extractability: i64,
    pub score_operability: i64,
    pub score_creep_risk: i64,
    pub score_runtime_sovereignty: i64,
    pub score_model_logic_value: i64,
    pub score_ethics_safety: i64,
    pub score_intrinsic_risk: i64,
    #[serde(default, deserialize_with = "deserialize_lossy_f64")]
    pub score_final: f64,
    #[serde(default, deserialize_with = "deserialize_lossy_f64")]
    pub score_fit_geral_soda: f64,
    #[serde(default, deserialize_with = "deserialize_lossy_f64")]
    pub score_architectural_priority: f64,
    #[serde(default, deserialize_with = "deserialize_lossy_f64")]
    pub score_human_product_priority: f64,
    #[serde(default, deserialize_with = "deserialize_lossy_f64")]
    pub score_absorption_readiness: f64,
    #[serde(default, deserialize_with = "deserialize_lossy_f64")]
    pub score_operational_priority: f64,
    #[serde(default, deserialize_with = "deserialize_lossy_f64")]
    pub score_sustainability_adjusted_fit: f64,
    pub valid_from: i64,
    pub valid_to: Option<i64>,
    pub embargo_status: i64,
}

impl Default for MasterSolutionsRow {
    fn default() -> Self {
        Self {
            status_atualizacao: String::new(),
            status_fase: String::new(),
            project_name: String::new(),
            repo_url: String::new(),
            repo_analised_version: String::new(),
            ultima_versao_online: String::new(),
            indicacao_otimista_canibalizacao: String::new(),
            lote_id: String::new(),
            data_ultima_analise: 0,
            analise_origem: String::new(),
            licenca: String::new(),
            stack_base: String::new(),
            declared_description: String::new(),
            declared_description_ptbr: String::new(),
            lente_a_sentido_prod_ux: String::new(),
            lente_b_estrutura_arq: String::new(),
            lente_c_realidade_ops: String::new(),
            proposta_original_resumo: String::new(),
            visao_do_enxame: String::new(),
            justificativa_decisao: String::new(),
            executive_verdict: String::new(),
            risco_principal: String::new(),
            risco_linha_vermelha: String::new(),
            observacoes: String::new(),
            ouro_a_extrair: String::new(),
            deep_pattern: String::new(),
            transplantable_core: String::new(),
            logic_math_heuristic: String::new(),
            real_structural_problem: String::new(),
            categoria_nuance_tecnica: String::new(),
            integracao_papel_exato: String::new(),
            must_components_prod_ux: String::new(),
            must_components_arq: String::new(),
            must_components_ops: String::new(),
            detected_toxic_deps: String::new(),
            do_not_absorb: String::new(),
            where_ai_should_not_enter: String::new(),
            classificacao_terminal: TerminalClassification::default(),
            acao_de_canibalizacao: CannibalizationAction::default(),
            categoria_arquitetural: ArchitecturalCategory::default(),
            horizonte_extracao: ExtractionHorizon::default(),
            tipo_integracao: IntegrationType::default(),
            capability_nature_primary: CapabilityNaturePrimary::default(),
            architectural_topology: ArchitecturalTopology::default(),
            temporal_stability: TemporalStability::default(),
            bare_metal_fit: FitLevel4::default(),
            extractability_level: FitLevel4::default(),
            runtime_sovereignty_fit: FitLevel4::default(),
            local_first_fit: FitLevel4::default(),
            adoptability_level: Scale5::default(),
            longitudinal_sustainability: Scale5::default(),
            maintenance_burden: BurdenLevel::default(),
            onboarding_friction: BurdenLevel::default(),
            observability_operational: Scale5::default(),
            recoverability_level: Scale5::default(),
            degradation_behavior: DegradationBehavior::default(),
            curation_burden: BurdenLevel::default(),
            evolution_cost: BurdenLevel::default(),
            operability_level: FitLevel4::default(),
            abandonment_risk: RiskLevel4::default(),
            time_to_first_clear_value: TimeHorizon::default(),
            imperfection_tolerance: Scale5::default(),
            entropy_risk: RiskLevel4::default(),
            design_misuse_risk: RiskLevel4::default(),
            intrinsic_ethics_risk: RiskLevel4::default(),
            discipline_dependency: DisciplineDependency::default(),
            regulatory_risk: RiskLevel4::default(),
            score_philosophical_fit: 0,
            score_bare_metal_fit: 0,
            score_architectural_extractability: 0,
            score_operability: 0,
            score_creep_risk: 0,
            score_runtime_sovereignty: 0,
            score_model_logic_value: 0,
            score_ethics_safety: 0,
            score_intrinsic_risk: 0,
            score_final: 0.0,
            score_fit_geral_soda: 0.0,
            score_architectural_priority: 0.0,
            score_human_product_priority: 0.0,
            score_absorption_readiness: 0.0,
            score_operational_priority: 0.0,
            score_sustainability_adjusted_fit: 0.0,
            valid_from: 0,
            valid_to: None,
            embargo_status: 0,
        }
    }
}

impl MasterSolutionsRow {
    pub fn from_block0(block0: Block0Context) -> Self {
        let proposta_original_resumo = block0
            .proposta_original_resumo
            .unwrap_or_default()
            .trim()
            .to_string();
        let categoria_arquitetural = match block0.categoria_arquitetural.as_deref() {
            Some(raw) => ArchitecturalCategory::parse_strict(raw).unwrap_or(ArchitecturalCategory::Unknown),
            None => ArchitecturalCategory::default(),
        };
        Self {
            status_atualizacao: block0.status_atualizacao,
            status_fase: block0.status_fase,
            project_name: block0.project_name,
            repo_url: block0.repo_url,
            repo_analised_version: block0.repo_analised_version,
            ultima_versao_online: block0.ultima_versao_online,
            lote_id: block0.lote_id,
            data_ultima_analise: block0.data_ultima_analise,
            analise_origem: block0.analise_origem,
            licenca: block0.licenca,
            stack_base: block0.stack_base,
            declared_description: block0.declared_description,
            lente_a_sentido_prod_ux: normalize_lens_bullets(&block0.lente_a_sentido_prod_ux),
            lente_b_estrutura_arq: normalize_lens_bullets(&block0.lente_b_estrutura_arq),
            lente_c_realidade_ops: normalize_lens_bullets(&block0.lente_c_realidade_ops),
            proposta_original_resumo,
            categoria_arquitetural,
            ..Self::default()
        }
    }

    pub fn to_sheet_row(&self) -> Vec<String> {
        let pretty_project_name = self.project_name.replace("/", " / ");
        let declared_description = if self.declared_description_ptbr.trim().is_empty() {
            self.declared_description.clone()
        } else {
            self.declared_description_ptbr.clone()
        };

        let classificacao_terminal = self.classificacao_terminal.as_str();
        let acao_de_canibalizacao = self.acao_de_canibalizacao.as_str();
        let categoria_arquitetural = self.categoria_arquitetural.as_str();
        let horizonte_extracao = self.horizonte_extracao.as_str();
        let tipo_integracao = self.tipo_integracao.as_str();
        let capability_nature_primary = self.capability_nature_primary.as_str();
        let architectural_topology = self.architectural_topology.as_str();
        let temporal_stability = self.temporal_stability.as_str();
        let bare_metal_fit = self.bare_metal_fit.as_str();
        let extractability_level = self.extractability_level.as_str();
        let runtime_sovereignty_fit = self.runtime_sovereignty_fit.as_str();
        let local_first_fit = self.local_first_fit.as_str();
        let adoptability_level = self.adoptability_level.as_str();
        let longitudinal_sustainability = self.longitudinal_sustainability.as_str();
        let maintenance_burden = self.maintenance_burden.as_str();
        let onboarding_friction = self.onboarding_friction.as_str();
        let observability_operational = self.observability_operational.as_str();
        let recoverability_level = self.recoverability_level.as_str();
        let degradation_behavior = self.degradation_behavior.as_str();
        let curation_burden = self.curation_burden.as_str();
        let evolution_cost = self.evolution_cost.as_str();
        let operability_level = self.operability_level.as_str();
        let abandonment_risk = self.abandonment_risk.as_str();
        let time_to_first_clear_value = self.time_to_first_clear_value.as_str();
        let imperfection_tolerance = self.imperfection_tolerance.as_str();
        let entropy_risk = self.entropy_risk.as_str();
        let design_misuse_risk = self.design_misuse_risk.as_str();
        let intrinsic_ethics_risk = self.intrinsic_ethics_risk.as_str();
        let discipline_dependency = self.discipline_dependency.as_str();
        let regulatory_risk = self.regulatory_risk.as_str();

        let data_ultima_analise = format_epoch_utc(self.data_ultima_analise);
        let valid_from = format_epoch_utc(self.valid_from);
        let valid_to = self.valid_to.map(format_epoch_utc).unwrap_or_default();
        let embargo_status = embargo_label(self.embargo_status).to_string();

        Vec::from([
            pretty_project_name,
            self.repo_url.clone(),
            self.repo_analised_version.clone(),
            self.ultima_versao_online.clone(),
            self.lote_id.clone(),
            data_ultima_analise,
            self.analise_origem.clone(),
            self.licenca.clone(),
            self.stack_base.clone(),
            declared_description,
            self.lente_a_sentido_prod_ux.clone(),
            self.lente_b_estrutura_arq.clone(),
            self.lente_c_realidade_ops.clone(),
            self.proposta_original_resumo.clone(),
            self.visao_do_enxame.clone(),
            self.justificativa_decisao.clone(),
            self.executive_verdict.clone(),
            self.risco_principal.clone(),
            self.risco_linha_vermelha.clone(),
            self.observacoes.clone(),
            self.ouro_a_extrair.clone(),
            self.deep_pattern.clone(),
            self.transplantable_core.clone(),
            self.logic_math_heuristic.clone(),
            self.real_structural_problem.clone(),
            self.categoria_nuance_tecnica.clone(),
            self.integracao_papel_exato.clone(),
            self.must_components_prod_ux.clone(),
            self.must_components_arq.clone(),
            self.must_components_ops.clone(),
            self.detected_toxic_deps.clone(),
            self.do_not_absorb.clone(),
            self.where_ai_should_not_enter.clone(),
            classificacao_terminal.to_string(),
            acao_de_canibalizacao.to_string(),
            categoria_arquitetural.to_string(),
            horizonte_extracao.to_string(),
            tipo_integracao.to_string(),
            capability_nature_primary.to_string(),
            architectural_topology.to_string(),
            temporal_stability.to_string(),
            bare_metal_fit.to_string(),
            extractability_level.to_string(),
            runtime_sovereignty_fit.to_string(),
            local_first_fit.to_string(),
            adoptability_level.to_string(),
            longitudinal_sustainability.to_string(),
            maintenance_burden.to_string(),
            onboarding_friction.to_string(),
            observability_operational.to_string(),
            recoverability_level.to_string(),
            degradation_behavior.to_string(),
            curation_burden.to_string(),
            evolution_cost.to_string(),
            operability_level.to_string(),
            abandonment_risk.to_string(),
            time_to_first_clear_value.to_string(),
            imperfection_tolerance.to_string(),
            entropy_risk.to_string(),
            design_misuse_risk.to_string(),
            intrinsic_ethics_risk.to_string(),
            discipline_dependency.to_string(),
            regulatory_risk.to_string(),
            self.score_philosophical_fit.to_string(),
            self.score_bare_metal_fit.to_string(),
            self.score_architectural_extractability.to_string(),
            self.score_operability.to_string(),
            self.score_creep_risk.to_string(),
            self.score_runtime_sovereignty.to_string(),
            self.score_model_logic_value.to_string(),
            self.score_ethics_safety.to_string(),
            self.score_intrinsic_risk.to_string(),
            format_float_1(self.score_final),
            format_float_1(self.score_fit_geral_soda),
            format_float_1(self.score_architectural_priority),
            format_float_1(self.score_human_product_priority),
            format_float_1(self.score_absorption_readiness),
            format_float_1(self.score_operational_priority),
            format_float_1(self.score_sustainability_adjusted_fit),
            valid_from,
            valid_to,
            embargo_status,
        ])
    }
}

fn format_float_1(value: f64) -> String {
    let raw = format!("{:.1}", value);
    raw.replace(',', ".")
}

fn format_epoch_utc(epoch: i64) -> String {
    if epoch <= 0 {
        return String::new();
    }
    crate::telemetry::format_brt_rfc3339(epoch)
}

fn embargo_label(value: i64) -> &'static str {
    if value == 1 { "EMBARGADO" } else { "LIVRE" }
}

fn format_bullets(lines: &[String]) -> String {
    let mut cleaned = Vec::new();
    for item in lines {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        cleaned.push(trimmed.to_string());
    }
    if cleaned.is_empty() {
        return String::new();
    }
    format!("- {}", cleaned.join("\n- "))
}

fn format_dot_bullets(lines: &[String]) -> String {
    let mut out = String::new();
    for item in lines {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("• ");
        out.push_str(trimmed);
    }
    out
}

fn normalize_string_vec(mut raw: Vec<String>, min_items: usize, max_items: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in raw.drain(..) {
        let t = item.trim();
        if t.is_empty() {
            continue;
        }
        if t.contains('\n') || t.starts_with("- ") || t.starts_with("• ") {
            for line in t.lines() {
                let lt = line.trim();
                if lt.is_empty() {
                    continue;
                }
                let stripped = lt
                    .strip_prefix("- ")
                    .or_else(|| lt.strip_prefix("• "))
                    .unwrap_or(lt)
                    .trim();
                if stripped.is_empty() {
                    continue;
                }
                out.push(stripped.to_string());
                if out.len() >= max_items {
                    break;
                }
            }
        } else {
            out.push(t.to_string());
        }
        if out.len() >= max_items {
            break;
        }
    }
    if out.len() >= min_items {
        return out;
    }

    let mut expanded: Vec<String> = Vec::new();
    for item in out {
        for seg in split_bullet_segments(&item) {
            if expanded.len() >= max_items {
                break;
            }
            expanded.push(seg);
        }
        if expanded.len() >= max_items {
            break;
        }
    }
    expanded
}

fn normalize_lens_bullets(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(bullets) = extract_bullets_array(&val) {
            return format_bullets(&bullets);
        }
        let mut leaves = Vec::new();
        collect_leaf_strings(&val, &mut leaves);
        return format_bullets(&leaves);
    }

    if let Some(snippet) = salvage_balanced_json(trimmed) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&snippet) {
            if let Some(bullets) = extract_bullets_array(&val) {
                return format_bullets(&bullets);
            }
            let mut leaves = Vec::new();
            collect_leaf_strings(&val, &mut leaves);
            let out = format_bullets(&leaves);
            if !out.is_empty() {
                return out;
            }
        }
    }

    scrub_json_syntax_to_text(trimmed)
}

#[cfg(test)]
fn normalize_pydantic_list_field(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("- ") {
        return trimmed.replace("\r\n", "\n");
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(arr) = val.as_array() {
            let mut items = Vec::new();
            for item in arr {
                match item {
                    serde_json::Value::String(s) => items.push(s.clone()),
                    other => {
                        let mut leaves = Vec::new();
                        collect_leaf_strings(other, &mut leaves);
                        items.extend(leaves);
                    }
                }
            }
            return format_bullets(&items);
        }
        let mut leaves = Vec::new();
        collect_leaf_strings(&val, &mut leaves);
        let out = format_bullets(&leaves);
        if !out.is_empty() {
            return out;
        }
    }

    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len().saturating_sub(1)];
        let mut items = Vec::new();
        for piece in inner.split(',') {
            let token = piece.trim().trim_matches('"').trim_matches('\'').trim();
            if token.is_empty() {
                continue;
            }
            items.push(token.to_string());
        }
        let out = format_bullets(&items);
        if !out.is_empty() {
            return out;
        }
    }

    scrub_json_syntax_to_text(trimmed)
}

fn split_bullet_segments(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let cleaned = value.replace("•", "-").replace("—", "-");
    for chunk in cleaned.split(['\n', ';', '|']) {
        let c = chunk.trim().trim_start_matches("- ").trim();
        if c.is_empty() {
            continue;
        }
        for sentence in c.split(". ") {
            let s = sentence.trim().trim_end_matches('.').trim();
            if s.is_empty() {
                continue;
            }
            out.push(s.to_string());
        }
    }
    out
}

fn truncate_chars_simple(content: &str, max_chars: usize) -> String {
    let trimmed = content.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out = String::new();
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx >= max_chars {
            break;
        }
        out.push(ch);
    }
    out
}

fn extract_bullets_array(val: &serde_json::Value) -> Option<Vec<String>> {
    let obj = val.as_object()?;
    let bullets = obj.get("bullets")?.as_array()?;
    let mut out = Vec::new();
    for item in bullets {
        let s = item.as_str()?.trim();
        if s.is_empty() {
            continue;
        }
        out.push(s.to_string());
    }
    Some(out)
}

fn collect_leaf_strings(val: &serde_json::Value, out: &mut Vec<String>) {
    match val {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_leaf_strings(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map {
                collect_leaf_strings(v, out);
            }
        }
        _ => {}
    }
}

fn scrub_json_syntax_to_text(raw: &str) -> String {
    let mut s = raw.replace("\r\n", "\n");
    s = s.replace(['{', '}', '[', ']', '"'], "");
    s = s.replace(", ", "\n");
    s = s.replace(',', "\n");
    s = s.replace(':', " ");
    let mut lines = Vec::new();
    for line in s.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let lower = l.to_ascii_lowercase();
        if lower.starts_with("lens_id")
            || lower.starts_with("schema")
            || lower.starts_with("version")
            || lower.starts_with("lens")
        {
            continue;
        }
        lines.push(l.to_string());
    }
    if lines.is_empty() {
        String::new()
    } else if lines.len() == 1 {
        lines[0].clone()
    } else {
        format!("- {}", lines.join("\n- "))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase3Config {
    pub model: String,
    pub model_block3_candidates: Vec<String>,
    pub max_attempts_per_block: usize,
}

impl Phase3Config {
    fn block3_models(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut ordered = Vec::new();
        for candidate in self
            .model_block3_candidates
            .iter()
            .map(String::as_str)
            .chain(DEFAULT_BLOCK3_MODEL_CANDIDATES.iter().copied())
        {
            let trimmed = candidate.trim();
            if trimmed.is_empty() {
                continue;
            }
            if seen.insert(trimmed.to_ascii_lowercase()) {
                ordered.push(trimmed.to_string());
            }
        }
        if ordered.is_empty() {
            ordered.push(self.model.clone());
        }
        ordered
    }
}

pub trait FormatterClient: Send + Sync {
    fn format<'a>(
        &'a self,
        model: &'a str,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Phase3Output {
    pub model_used: String,
    pub row: MasterSolutionsRow,
    pub block3_justifications: HashMap<String, String>,
}

fn block3_looks_homogeneous(payload: &serde_json::Value) -> bool {
    let fields = match payload.get("fields").and_then(|v| v.as_object()) {
        Some(v) => v,
        None => return false,
    };
    let get = |k: &str| -> Option<&str> { fields.get(k).and_then(|v| v.as_str()).map(str::trim) };
    let eq = |k: &str, expected: &str| -> bool {
        get(k)
            .map(|v| v.eq_ignore_ascii_case(expected))
            .unwrap_or(false)
    };
    eq("adoptability_level", "MEDIUM")
        && eq("bare_metal_fit", "MEDIUM")
        && eq("maintenance_burden", "MEDIUM")
        && eq("runtime_sovereignty_fit", "MEDIUM")
        && eq("longitudinal_sustainability", "MEDIUM")
        && eq("local_first_fit", "MEDIUM")
        && eq("onboarding_friction", "MEDIUM")
        && eq("observability_operational", "MEDIUM")
        && eq("recoverability_level", "MEDIUM")
        && eq("degradation_behavior", "ACCEPTABLE")
        && eq("curation_burden", "MEDIUM")
        && eq("evolution_cost", "MEDIUM")
        && eq("operability_level", "MEDIUM")
        && eq("imperfection_tolerance", "MEDIUM")
        && eq("discipline_dependency", "MEDIA")
        && eq("abandonment_risk", "MEDIUM")
        && eq("design_misuse_risk", "MEDIUM")
        && eq("intrinsic_ethics_risk", "MEDIUM")
        && eq("entropy_risk", "MEDIUM")
        && eq("regulatory_risk", "MEDIUM")
}

fn block3_row_looks_homogeneous(row: &MasterSolutionsRow) -> bool {
    let eq = |value: &str, expected: &str| value.trim().eq_ignore_ascii_case(expected);
    eq(row.adoptability_level.as_str(), "MEDIUM")
        && eq(row.bare_metal_fit.as_str(), "MEDIUM")
        && eq(row.maintenance_burden.as_str(), "MEDIUM")
        && eq(row.runtime_sovereignty_fit.as_str(), "MEDIUM")
        && eq(row.longitudinal_sustainability.as_str(), "MEDIUM")
        && eq(row.local_first_fit.as_str(), "MEDIUM")
        && eq(row.onboarding_friction.as_str(), "MEDIUM")
        && eq(row.observability_operational.as_str(), "MEDIUM")
        && eq(row.recoverability_level.as_str(), "MEDIUM")
        && eq(row.degradation_behavior.as_str(), "ACCEPTABLE")
        && eq(row.curation_burden.as_str(), "MEDIUM")
        && eq(row.evolution_cost.as_str(), "MEDIUM")
        && eq(row.operability_level.as_str(), "MEDIUM")
        && eq(row.imperfection_tolerance.as_str(), "MEDIUM")
        && eq(row.discipline_dependency.as_str(), "MEDIA")
        && eq(row.abandonment_risk.as_str(), "MEDIUM")
        && eq(row.design_misuse_risk.as_str(), "MEDIUM")
        && eq(row.intrinsic_ethics_risk.as_str(), "MEDIUM")
        && eq(row.entropy_risk.as_str(), "MEDIUM")
        && eq(row.regulatory_risk.as_str(), "MEDIUM")
}

fn normalize_justification_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn block3_justifications_show_discrimination(justifications: &HashMap<String, String>) -> bool {
    let normalized: Vec<String> = justifications
        .values()
        .map(|value| normalize_justification_text(value))
        .filter(|value| !value.is_empty())
        .collect();
    if normalized.len() < 8 {
        return false;
    }
    let unique = normalized.iter().collect::<HashSet<_>>().len();
    let substantive = normalized.iter().filter(|value| value.len() >= 18).count();
    unique >= 5 && substantive >= 8
}

fn homogeneous_medium_conflicts_with_block4(block4: &Block4Fields) -> bool {
    let mapped_scores = [
        block4.score_bare_metal_fit,
        block4.score_runtime_sovereignty,
        block4.score_operability,
        block4.score_architectural_extractability,
    ];
    let outside_middle_band = mapped_scores
        .iter()
        .filter(|score| !(4..=6).contains(*score))
        .count();
    let strong_signals = mapped_scores
        .iter()
        .filter(|score| **score <= 2 || **score >= 8)
        .count();
    outside_middle_band >= 2 && strong_signals >= 1
}

fn block3_score_conflict_feedback(block4: &Block4Fields) -> String {
    format!(
        "BLOCK_3 anterior achatou MEDIUM, mas o bloco 4 mostrou sinais fortes: score_bare_metal_fit={}, score_runtime_sovereignty={}, score_operability={}, score_architectural_extractability={}. Reclassifique com discriminacao fina. MEDIUM so eh valido quando o eixo realmente cair no miolo; nao homogenize por default.",
        block4.score_bare_metal_fit,
        block4.score_runtime_sovereignty,
        block4.score_operability,
        block4.score_architectural_extractability
    )
}

const BLOCK_1: u8 = 1;
const BLOCK_2A: u8 = 21;
const BLOCK_2B: u8 = 22;
const BLOCK_3: u8 = 3;
const BLOCK_4: u8 = 4;

#[cfg(test)]
const BLOCK0_CONTEXT_COLUMNS: &[&str] = &[
    "status_atualizacao",
    "status_fase",
    "project_name",
    "repo_url",
    "repo_analised_version",
    "ultima_versao_online",
    "lote_id",
    "data_ultima_analise",
    "analise_origem",
    "licenca",
    "stack_base",
    "declared_description",
    "proposta_original_resumo",
    "categoria_arquitetural",
    "lente_a_sentido_prod_ux",
    "lente_b_estrutura_arq",
    "lente_c_realidade_ops",
];

const BLOCK1_FIELDS_COLUMNS: &[&str] = &[
    "proposta_original_resumo",
    "declared_description_ptbr",
    "visao_do_enxame",
    "justificativa_decisao",
    "executive_verdict",
    "risco_principal",
    "risco_linha_vermelha",
    "observacoes",
];

const BLOCK2A_FIELDS_COLUMNS: &[&str] = &[
    "indicacao_otimista_canibalizacao",
    "ouro_a_extrair",
    "deep_pattern",
    "transplantable_core",
    "logic_math_heuristic",
    "real_structural_problem",
    "categoria_nuance_tecnica",
    "integracao_papel_exato",
];

const BLOCK2B_FIELDS_COLUMNS: &[&str] = &[
    "must_components_prod_ux",
    "must_components_arq",
    "must_components_ops",
    "detected_toxic_deps",
    "do_not_absorb",
    "where_ai_should_not_enter",
];

const BLOCK3_FIELDS_COLUMNS: &[&str] = &[
    "classificacao_terminal",
    "acao_de_canibalizacao",
    "categoria_arquitetural",
    "horizonte_extracao",
    "tipo_integracao",
    "capability_nature_primary",
    "architectural_topology",
    "temporal_stability",
    "bare_metal_fit",
    "extractability_level",
    "runtime_sovereignty_fit",
    "local_first_fit",
    "adoptability_level",
    "longitudinal_sustainability",
    "maintenance_burden",
    "onboarding_friction",
    "observability_operational",
    "recoverability_level",
    "degradation_behavior",
    "curation_burden",
    "evolution_cost",
    "operability_level",
    "abandonment_risk",
    "time_to_first_clear_value",
    "imperfection_tolerance",
    "entropy_risk",
    "design_misuse_risk",
    "intrinsic_ethics_risk",
    "discipline_dependency",
    "regulatory_risk",
];

const BLOCK4_FIELDS_COLUMNS: &[&str] = &[
    "score_philosophical_fit",
    "score_bare_metal_fit",
    "score_architectural_extractability",
    "score_operability",
    "score_creep_risk",
    "score_runtime_sovereignty",
    "score_model_logic_value",
    "score_ethics_safety",
    "score_intrinsic_risk",
];

#[cfg(test)]
const PHASE4_DERIVED_COLUMNS: &[&str] = &[
    "score_final",
    "score_fit_geral_soda",
    "score_architectural_priority",
    "score_human_product_priority",
    "score_absorption_readiness",
    "score_operational_priority",
    "score_sustainability_adjusted_fit",
    "valid_from",
    "valid_to",
    "embargo_status",
];

fn block_prompt_tag(block: u8) -> &'static str {
    match block {
        BLOCK_1 => "1",
        BLOCK_2A => "2A",
        BLOCK_2B => "2B",
        BLOCK_3 => "3",
        BLOCK_4 => "4",
        _ => "0",
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Phase3Error {
    #[error("Falha no parse do code-fence JSON: {0}")]
    JsonFenceParse(String),
    #[error("Falha na validação do schema do Bloco {block}: {message}")]
    SchemaFailure { block: u8, message: String },
    #[error("Falha terminal após {attempts} tentativas no Bloco {block}: {message}")]
    RetryExhausted {
        block: u8,
        attempts: usize,
        message: String,
    },
    #[error("Falha de transporte do formatador: {0}")]
    Transport(String),
    #[error("Falha de persistência L2 (SQLite): {0}")]
    L2Failure(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockResponse<T> {
    fields: T,
    #[serde(default)]
    justifications: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct Block3Execution {
    envelope: BlockResponse<Block3Fields>,
    model_used: String,
    model_index: usize,
    homogeneous_medium: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block1Fields {
    #[serde(rename = "proposta_original_resumo")]
    proposta_original_resumo: Option<String>,
    #[serde(rename = "declared_description_ptbr")]
    declared_description_ptbr: String,
    #[serde(rename = "visao_do_enxame")]
    visao_do_enxame: String,
    #[serde(rename = "justificativa_decisao")]
    justificativa_decisao: String,
    #[serde(rename = "executive_verdict")]
    executive_verdict: String,
    #[serde(rename = "risco_principal")]
    risco_principal: String,
    #[serde(rename = "risco_linha_vermelha")]
    risco_linha_vermelha: String,
    #[serde(rename = "observacoes")]
    observacoes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block2NarrativeFields {
    #[serde(rename = "indicacao_otimista_canibalizacao")]
    indicacao_otimista_canibalizacao: String,
    #[serde(rename = "ouro_a_extrair")]
    ouro_a_extrair: String,
    #[serde(rename = "deep_pattern")]
    deep_pattern: String,
    #[serde(rename = "transplantable_core")]
    transplantable_core: String,
    #[serde(rename = "logic_math_heuristic")]
    logic_math_heuristic: String,
    #[serde(rename = "real_structural_problem")]
    real_structural_problem: String,
    #[serde(rename = "categoria_nuance_tecnica")]
    categoria_nuance_tecnica: String,
    #[serde(rename = "integracao_papel_exato")]
    integracao_papel_exato: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block2MatrixFields {
    #[serde(rename = "must_components_prod_ux")]
    must_components_prod_ux: Vec<String>,
    #[serde(rename = "must_components_arq")]
    must_components_arq: Vec<String>,
    #[serde(rename = "must_components_ops")]
    must_components_ops: Vec<String>,
    #[serde(rename = "detected_toxic_deps")]
    detected_toxic_deps: Vec<String>,
    #[serde(rename = "do_not_absorb")]
    do_not_absorb: Vec<String>,
    #[serde(rename = "where_ai_should_not_enter")]
    where_ai_should_not_enter: Vec<String>,
}

impl Block2MatrixFields {
    fn sanitize(self) -> Self {
        let max_items = 8usize;
        let mut must_components_prod_ux = normalize_string_vec(self.must_components_prod_ux, 3, max_items);
        let mut must_components_arq = normalize_string_vec(self.must_components_arq, 3, max_items);
        let mut must_components_ops = normalize_string_vec(self.must_components_ops, 3, max_items);
        let mut detected_toxic_deps = normalize_string_vec(self.detected_toxic_deps, 1, max_items);
        let mut do_not_absorb = normalize_string_vec(self.do_not_absorb, 1, max_items);
        let mut where_ai_should_not_enter =
            normalize_string_vec(self.where_ai_should_not_enter, 1, max_items);
        if must_components_prod_ux.len() > max_items {
            must_components_prod_ux.truncate(max_items);
        }
        if must_components_arq.len() > max_items {
            must_components_arq.truncate(max_items);
        }
        if must_components_ops.len() > max_items {
            must_components_ops.truncate(max_items);
        }
        if detected_toxic_deps.len() > max_items {
            detected_toxic_deps.truncate(max_items);
        }
        if do_not_absorb.len() > max_items {
            do_not_absorb.truncate(max_items);
        }
        if where_ai_should_not_enter.len() > max_items {
            where_ai_should_not_enter.truncate(max_items);
        }
        Self {
            must_components_prod_ux,
            must_components_arq,
            must_components_ops,
            detected_toxic_deps,
            do_not_absorb,
            where_ai_should_not_enter,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block3Fields {
    #[serde(rename = "classificacao_terminal")]
    classificacao_terminal: TerminalClassification,
    #[serde(rename = "acao_de_canibalizacao")]
    acao_de_canibalizacao: CannibalizationAction,
    #[serde(rename = "categoria_arquitetural")]
    categoria_arquitetural: Option<ArchitecturalCategory>,
    #[serde(rename = "horizonte_extracao")]
    horizonte_extracao: ExtractionHorizon,
    #[serde(rename = "tipo_integracao")]
    tipo_integracao: IntegrationType,
    #[serde(rename = "capability_nature_primary")]
    capability_nature_primary: CapabilityNaturePrimary,
    #[serde(rename = "architectural_topology")]
    architectural_topology: ArchitecturalTopology,
    #[serde(rename = "temporal_stability")]
    temporal_stability: TemporalStability,
    #[serde(rename = "bare_metal_fit")]
    bare_metal_fit: FitLevel4,
    #[serde(rename = "extractability_level")]
    extractability_level: FitLevel4,
    #[serde(rename = "runtime_sovereignty_fit")]
    runtime_sovereignty_fit: FitLevel4,
    #[serde(rename = "local_first_fit")]
    local_first_fit: FitLevel4,
    #[serde(rename = "adoptability_level")]
    adoptability_level: Scale5,
    #[serde(rename = "longitudinal_sustainability")]
    longitudinal_sustainability: Scale5,
    #[serde(rename = "maintenance_burden")]
    maintenance_burden: BurdenLevel,
    #[serde(rename = "onboarding_friction")]
    onboarding_friction: BurdenLevel,
    #[serde(rename = "observability_operational")]
    observability_operational: Scale5,
    #[serde(rename = "recoverability_level")]
    recoverability_level: Scale5,
    #[serde(rename = "degradation_behavior")]
    degradation_behavior: DegradationBehavior,
    #[serde(rename = "curation_burden")]
    curation_burden: BurdenLevel,
    #[serde(rename = "evolution_cost")]
    evolution_cost: BurdenLevel,
    #[serde(rename = "operability_level")]
    operability_level: FitLevel4,
    #[serde(rename = "abandonment_risk")]
    abandonment_risk: RiskLevel4,
    #[serde(rename = "time_to_first_clear_value")]
    time_to_first_clear_value: TimeHorizon,
    #[serde(rename = "imperfection_tolerance")]
    imperfection_tolerance: Scale5,
    #[serde(rename = "entropy_risk")]
    entropy_risk: RiskLevel4,
    #[serde(rename = "design_misuse_risk")]
    design_misuse_risk: RiskLevel4,
    #[serde(rename = "intrinsic_ethics_risk")]
    intrinsic_ethics_risk: RiskLevel4,
    #[serde(rename = "discipline_dependency")]
    discipline_dependency: DisciplineDependency,
    #[serde(rename = "regulatory_risk")]
    regulatory_risk: RiskLevel4,
    #[serde(rename = "stack_base", default)]
    stack_base: Option<String>,
    #[serde(rename = "licenca", default)]
    licenca: Option<String>,
}

impl Block3Fields {
    fn sanitize(self, needs_stack_base_cure: bool, needs_licenca_cure: bool) -> Result<Self, String> {
        let mut out = self;
        let mut strict_errors: Vec<&'static str> = Vec::new();
        if matches!(out.classificacao_terminal, TerminalClassification::Unknown) {
            strict_errors.push("classificacao_terminal");
        }
        if matches!(out.acao_de_canibalizacao, CannibalizationAction::Unknown) {
            strict_errors.push("acao_de_canibalizacao");
        }
        if matches!(out.categoria_arquitetural, Some(ArchitecturalCategory::Unknown)) {
            strict_errors.push("categoria_arquitetural");
        }
        if matches!(out.categoria_arquitetural, Some(ArchitecturalCategory::Unspecified)) {
            out.categoria_arquitetural = None;
        }
        if matches!(out.horizonte_extracao, ExtractionHorizon::Unknown) {
            strict_errors.push("horizonte_extracao");
        }
        if matches!(out.tipo_integracao, IntegrationType::Unknown) {
            strict_errors.push("tipo_integracao");
        }
        if matches!(out.capability_nature_primary, CapabilityNaturePrimary::Unknown) {
            strict_errors.push("capability_nature_primary");
        }
        if matches!(out.architectural_topology, ArchitecturalTopology::Unknown) {
            strict_errors.push("architectural_topology");
        }
        if matches!(out.temporal_stability, TemporalStability::Unknown) {
            warn!("Bloco 3: `temporal_stability` caiu em fallback UNKNOWN");
        }
        if matches!(out.bare_metal_fit, FitLevel4::Unknown) {
            warn!("Bloco 3: `bare_metal_fit` caiu em fallback UNKNOWN");
        }
        if matches!(out.extractability_level, FitLevel4::Unknown) {
            warn!("Bloco 3: `extractability_level` caiu em fallback UNKNOWN");
        }
        if matches!(out.runtime_sovereignty_fit, FitLevel4::Unknown) {
            warn!("Bloco 3: `runtime_sovereignty_fit` caiu em fallback UNKNOWN");
        }
        if matches!(out.local_first_fit, FitLevel4::Unknown) {
            warn!("Bloco 3: `local_first_fit` caiu em fallback UNKNOWN");
        }
        if matches!(out.adoptability_level, Scale5::Unknown) {
            warn!("Bloco 3: `adoptability_level` caiu em fallback UNKNOWN");
        }
        if matches!(out.longitudinal_sustainability, Scale5::Unknown) {
            warn!("Bloco 3: `longitudinal_sustainability` caiu em fallback UNKNOWN");
        }
        if matches!(out.maintenance_burden, BurdenLevel::Unknown) {
            warn!("Bloco 3: `maintenance_burden` caiu em fallback UNKNOWN");
        }
        if matches!(out.onboarding_friction, BurdenLevel::Unknown) {
            warn!("Bloco 3: `onboarding_friction` caiu em fallback UNKNOWN");
        }
        if matches!(out.observability_operational, Scale5::Unknown) {
            warn!("Bloco 3: `observability_operational` caiu em fallback UNKNOWN");
        }
        if matches!(out.recoverability_level, Scale5::Unknown) {
            warn!("Bloco 3: `recoverability_level` caiu em fallback UNKNOWN");
        }
        if matches!(out.degradation_behavior, DegradationBehavior::Unknown) {
            warn!("Bloco 3: `degradation_behavior` caiu em fallback UNKNOWN");
        }
        if matches!(out.curation_burden, BurdenLevel::Unknown) {
            warn!("Bloco 3: `curation_burden` caiu em fallback UNKNOWN");
        }
        if matches!(out.evolution_cost, BurdenLevel::Unknown) {
            warn!("Bloco 3: `evolution_cost` caiu em fallback UNKNOWN");
        }
        if matches!(out.operability_level, FitLevel4::Unknown) {
            warn!("Bloco 3: `operability_level` caiu em fallback UNKNOWN");
        }
        if matches!(out.abandonment_risk, RiskLevel4::Unknown) {
            warn!("Bloco 3: `abandonment_risk` caiu em fallback UNKNOWN");
        }
        if matches!(out.time_to_first_clear_value, TimeHorizon::Unknown) {
            warn!("Bloco 3: `time_to_first_clear_value` caiu em fallback UNKNOWN");
        }
        if matches!(out.imperfection_tolerance, Scale5::Unknown) {
            warn!("Bloco 3: `imperfection_tolerance` caiu em fallback UNKNOWN");
        }
        if matches!(out.entropy_risk, RiskLevel4::Unknown) {
            warn!("Bloco 3: `entropy_risk` caiu em fallback UNKNOWN");
        }
        if matches!(out.design_misuse_risk, RiskLevel4::Unknown) {
            warn!("Bloco 3: `design_misuse_risk` caiu em fallback UNKNOWN");
        }
        if matches!(out.intrinsic_ethics_risk, RiskLevel4::Unknown) {
            warn!("Bloco 3: `intrinsic_ethics_risk` caiu em fallback UNKNOWN");
        }
        if matches!(out.discipline_dependency, DisciplineDependency::Unknown) {
            strict_errors.push("discipline_dependency");
        }
        if matches!(out.regulatory_risk, RiskLevel4::Unknown) {
            warn!("Bloco 3: `regulatory_risk` caiu em fallback UNKNOWN");
        }
        if needs_stack_base_cure {
            let raw = out.stack_base.as_deref().unwrap_or_default();
            let normalized = normalize_stack_base_cure_value(raw).ok_or_else(|| {
                "Cura falhou: stack_base ausente/UNKNOWN ou fora da taxonomia permitida".to_string()
            })?;
            out.stack_base = Some(normalized.to_string());
        } else {
            out.stack_base = None;
        }
        if needs_licenca_cure {
            let raw = out.licenca.as_deref().unwrap_or_default();
            let normalized = normalize_licenca_cure_value(raw).ok_or_else(|| {
                "Cura falhou: licenca ausente/UNKNOWN ou fora da taxonomia permitida".to_string()
            })?;
            out.licenca = Some(normalized);
        } else {
            out.licenca = None;
        }
        if strict_errors.is_empty() {
            Ok(out)
        } else {
            Err(format!(
                "Bloco 3 recebeu enums fora do catálogo estrito: {}",
                strict_errors.join(", ")
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block4Fields {
    #[serde(rename = "score_philosophical_fit")]
    score_philosophical_fit: i64,
    #[serde(rename = "score_bare_metal_fit")]
    score_bare_metal_fit: i64,
    #[serde(rename = "score_architectural_extractability")]
    score_architectural_extractability: i64,
    #[serde(rename = "score_operability")]
    score_operability: i64,
    #[serde(rename = "score_creep_risk")]
    score_creep_risk: i64,
    #[serde(rename = "score_runtime_sovereignty")]
    score_runtime_sovereignty: i64,
    #[serde(rename = "score_model_logic_value")]
    score_model_logic_value: i64,
    #[serde(rename = "score_ethics_safety")]
    score_ethics_safety: i64,
    #[serde(rename = "score_intrinsic_risk")]
    score_intrinsic_risk: i64,
}

fn validate_score_0_10(field: &str, value: i64, block: u8) -> Result<(), Phase3Error> {
    if (0..=10).contains(&value) {
        Ok(())
    } else {
        Err(Phase3Error::SchemaFailure {
            block,
            message: format!("{} fora do intervalo [0,10]: {}", field, value),
        })
    }
}

fn build_prompt(block: u8, block0: &Block0Context, prior: &MasterSolutionsRow, last_error: Option<&str>) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!("BLOCK={}\n", block_prompt_tag(block)));
    prompt.push_str(&format!("project_name={}\n", block0.project_name));
    prompt.push_str(&format!("repo_url={}\n", block0.repo_url));
    prompt.push_str("OUTPUT: responda com um bloco Markdown ```json ... ``` contendo um objeto JSON.\n");
    prompt.push_str("O JSON deve conter: {\"fields\":{...},\"justifications\":{...}}.\n");
    prompt.push_str("STRICTNESS: nenhum texto fora do code-fence. Nenhuma chave extra fora de fields/justifications. Em fields, use SOMENTE as chaves listadas para este bloco (todas obrigatórias).\n");
    prompt.push_str("FIELDS_KEYS_EXATAS:\n");
    prompt.push_str(&fields_keys_for_block(block, prior));
    prompt.push('\n');
    match block {
        BLOCK_1 => {
            prompt.push_str("IDIOMA_BLOCK1: todos os textos descritivos devem estar em Português (PT-BR).\n");
            prompt.push_str("STYLE_BLOCK1_BASE: proposta_original_resumo e declared_description_ptbr podem ser objetivos, mas sem amputar fatos relevantes.\n");
            prompt.push_str("STYLE_BLOCK1_DIALETICO: visao_do_enxame, executive_verdict, risco_principal e risco_linha_vermelha DEVEM ter profundidade analitica, causalidade explicita, tensao entre ganhos e riscos e rigor dialetico. Nao use limite artificial de linhas.\n");
            prompt.push_str("STYLE_BLOCK1_ARGUMENTACAO: justificativa_decisao e observacoes DEVEM conectar as 3 lentes do enxame, explicando trade-offs, pre-condicoes e por que a decisao faz sentido no SODA.\n");
            prompt.push_str("TRANSLATE_BLOCK1: gere declared_description_ptbr como tradução fiel para PT-BR de project.declared_description. Comece com letra maiúscula. Não adicione comentários sobre tradução.\n");
        }
        BLOCK_2A => {
            prompt.push_str("IDIOMA_BLOCK2A: todos os textos descritivos devem estar em Português (PT-BR).\n");
            prompt.push_str("STYLE_BLOCK2A: evite generalidades. Use termos concretos, testáveis e com densidade técnica.\n");
            prompt.push_str("INDICACAO_OTIMISTA_BLOCK2A: gere indicacao_otimista_canibalizacao como Arquiteto-Chefe SODA. Use as 3 lentes para propor estrategicamente o que canibalizar (valor de produto, núcleo arquitetural transplantável, riscos/limites). Não concatene colunas; sintetize uma proposta inteligente.\n");
        }
        BLOCK_2B => {
            prompt.push_str("IDIOMA_BLOCK2B: todos os itens devem estar em Português (PT-BR).\n");
            prompt.push_str("STYLE_BLOCK2B: entregue somente listas concretas, sem narrativa extra, sem parágrafos e sem markdown.\n");
            prompt.push_str("ARRAYS_BLOCK2B: must_components_prod_ux, must_components_arq e must_components_ops DEVEM ser arrays JSON com NO MÍNIMO 3 itens, cada item detalhado (componente + papel + por quê).\n");
            prompt.push_str("ARRAYS_BLOCK2B_LIGHT: detected_toxic_deps, do_not_absorb e where_ai_should_not_enter DEVEM ser arrays JSON com NO MÍNIMO 1 item cada.\n");
        }
        BLOCK_3 => {
            prompt.push_str("LIMITS_BLOCK3: cada valor string em fields deve conter apenas o label exato do catálogo. Nenhuma frase explicativa em fields.\n");
            prompt.push_str("MODO_ROBOTICO_ENUMS_BLOCK3: para TODOS os campos ENUM do Bloco 3, fields deve conter APENAS o valor do catálogo (1 token).\n");
            prompt.push_str("PROIBIDO: hífens, ':' , parênteses, frases, ou duas opções no mesmo campo.\n");
            prompt.push_str("ANTI_HOMOGENEIZACAO_BLOCK3: PROIBIDO responder tudo como MEDIUM/ACCEPTABLE/MEDIA por default. Distribua os valores conforme as 3 lentes e o contexto concreto do repo. Se a maioria dos campos sair igual, revise antes de responder.\n");
            prompt.push_str("MEDIUM_LEGITIMO_BLOCK3: use MEDIUM apenas quando o eixo estiver realmente no meio-termo. Se houver sinais fortes de aptidao ou fragilidade, use LOW/HIGH/EXCELLENT/VERY_LOW/VERY_HIGH/CRITICAL conforme o catalogo.\n");
            prompt.push_str("JUSTIFICATIONS_BLOCK3: além de fields, DEVEM vir justifications com 1 frase curta por campo crítico do bloco 3, explicando por que o valor categórico foi escolhido.\n");
            prompt.push_str("SGR_BLOCK3: preencha mentalmente justifications primeiro e só depois emita os ENUMs correspondentes em fields.\n");
            prompt.push_str("KNOWLEDGE_MODE_BLOCK3: se project.stack_base == \"UNKNOWN\" (ou context_alert presente), trate como repositorio de Conhecimento/Metodologia. Nesse caso, bare_metal_fit, runtime_sovereignty_fit e local_first_fit DEVEM ser HIGH ou EXCELLENT (nunca LOW/VERY_LOW), pois não há runtime externo.\n");
            prompt.push_str(enum_catalog_block3());
            if needs_stack_base_cure_for_block3(prior) {
                prompt.push_str("stack_base: Rust|Python|NodeJS|Go|JVM|DotNet|Mixed\n");
                prompt.push_str("CURA_STACK_BASE: Se você precisar curar stack_base, deduza as linguagens primárias pela árvore de arquivos e sinais no README (extensões, manifests, pastas src/, package.json, Cargo.toml, go.mod, pom.xml, *.csproj). Responda com 1 valor do catálogo acima.\n");
            }
            if needs_licenca_cure_for_block3(prior) {
                prompt.push_str("licenca: MIT|Apache-2.0|GPL-3.0|GPL-2.0|LGPL-3.0|AGPL-3.0|BSD-3-Clause|BSD-2-Clause|MPL-2.0|ISC|Unlicense|Copyrighted|NÃO ESPECIFICADO\n");
                prompt.push_str("CURA_LICENCA: Se você precisar curar licenca, inferir do README/AST (spdx identifiers e padrões comuns). Se não houver evidência suficiente, use 'Copyrighted' ou 'NÃO ESPECIFICADO'. Responda com 1 valor do catálogo acima.\n");
            }
        }
        _ => {}
    }
    if block == BLOCK_4 {
        prompt.push_str("CONSTRAINTS_BLOCK4: todos os campos em fields são inteiros no intervalo [0,10].\n");
    }
    if let Some(error) = last_error {
        prompt.push_str("\nPREVIOUS_SCHEMA_ERROR:\n");
        prompt.push_str(error);
        prompt.push('\n');
    }
    prompt.push_str("\nCONTEXT_ROW_PARTIAL_JSON:\n");
    prompt.push_str(&serde_json::to_string(&compact_context_for_block(block0, prior, block)).unwrap_or_default());
    prompt
}

fn fields_keys_for_block(block: u8, prior: &MasterSolutionsRow) -> String {
    match block {
        BLOCK_1 => {
            let mut keys = BLOCK1_FIELDS_COLUMNS.to_vec();
            if !prior.proposta_original_resumo.trim().is_empty() {
                keys.retain(|k| *k != "proposta_original_resumo");
            }
            serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string())
        }
        BLOCK_2A => serde_json::to_string(BLOCK2A_FIELDS_COLUMNS).unwrap_or_else(|_| "[]".to_string()),
        BLOCK_2B => serde_json::to_string(BLOCK2B_FIELDS_COLUMNS).unwrap_or_else(|_| "[]".to_string()),
        BLOCK_3 => {
            let mut keys = BLOCK3_FIELDS_COLUMNS.to_vec();
            let categoria_is_present = !matches!(
                prior.categoria_arquitetural,
                ArchitecturalCategory::Unspecified | ArchitecturalCategory::Unknown
            );
            if categoria_is_present {
                keys.retain(|k| *k != "categoria_arquitetural");
            }
            if needs_stack_base_cure_for_block3(prior) {
                keys.push("stack_base");
            }
            if needs_licenca_cure_for_block3(prior) {
                keys.push("licenca");
            }
            serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string())
        }
        BLOCK_4 => serde_json::to_string(BLOCK4_FIELDS_COLUMNS).unwrap_or_else(|_| "[]".to_string()),
        _ => "[]".to_string(),
    }
}

fn enum_catalog_block3() -> &'static str {
    "CATALOGO_ENUMS_BLOCK3:\n\
classificacao_terminal: STACK_CORE_PLANO_A1|STACK_CORE_PLANO_A2|STACK_CORE_PLANO_B|INTEGRATE_AS_COMPONENT|ABSORB_PARTIALLY|ABSORB_CONCEPT|USE_AS_INSPIRATION_ONLY|REJECT|SHORT-CIRCUIT\n\
acao_de_canibalizacao: Data Model / Schema|Prompt / Heuristic Seed|Protocol / Standard|Concept|UX Pattern|Canvas Refinement|New Canvas|Cognitive Layer|Infra Capability|Technical Runtime|Sandbox|Plugin|External Contract|No Absorption\n\
categoria_arquitetural: CanvasUI|UILibrary|Memoria|Roteamento|Orquestracao|Seguranca|Infraestrutura|Tooling\n\
horizonte_extracao: IMEDIATO|CURTO_PRAZO|CURTO_MEDIO_PRAZO|MEDIO_PRAZO|LONGO_PRAZO|REFERENCIAL_TEORICO|NUNCA\n\
tipo_integracao: Biblioteca / Crate Nativa|Sidecar Efêmero|Daemon / Background Service|App Nativo / CLI Independente|Middleware / Proxy\n\
capability_nature_primary: Context|Memory|Perception|Expression|Execution|Observation|Documentation|Planning|Curation|Identity|Infrastructure|Multimodal IO|Sandbox|Serving|Retrieval|Synchronization\n\
architectural_topology: Monolith|Modular|Layered|Contract-Driven|Runtime-Centric|Event-Driven|Graph-Centric|Pipeline-Centric|Hybrid\n\
temporal_stability: STABLE|EVOLVING\n\
bare_metal_fit: LOW|MEDIUM|HIGH|EXCELLENT\n\
extractability_level: LOW|MEDIUM|HIGH|EXCELLENT\n\
operability_level: LOW|MEDIUM|HIGH|EXCELLENT\n\
runtime_sovereignty_fit: LOW|MEDIUM|HIGH|EXCELLENT\n\
local_first_fit: LOW|MEDIUM|HIGH|EXCELLENT\n\
\n\
entropy_risk: LOW|MEDIUM|HIGH|CRITICAL\n\
design_misuse_risk: LOW|MEDIUM|HIGH|CRITICAL\n\
intrinsic_ethics_risk: LOW|MEDIUM|HIGH|CRITICAL\n\
regulatory_risk: LOW|MEDIUM|HIGH|CRITICAL\n\
abandonment_risk: LOW|MEDIUM|HIGH|CRITICAL\n\
\n\
discipline_dependency: Nenhuma|Baixa|Média|Alta|Crítica\n\
degradation_behavior: GRACEFUL|ACCEPTABLE|FRAGILE|CATASTROPHIC\n\
\n\
adoptability_level: VERY_LOW|LOW|MEDIUM|HIGH|EXCELLENT\n\
longitudinal_sustainability: VERY_LOW|LOW|MEDIUM|HIGH|EXCELLENT\n\
observability_operational: VERY_LOW|LOW|MEDIUM|HIGH|EXCELLENT\n\
recoverability_level: VERY_LOW|LOW|MEDIUM|HIGH|EXCELLENT\n\
imperfection_tolerance: VERY_LOW|LOW|MEDIUM|HIGH|EXCELLENT\n\
\n\
maintenance_burden: LOW|MEDIUM|HIGH|VERY_HIGH\n\
onboarding_friction: LOW|MEDIUM|HIGH|VERY_HIGH\n\
curation_burden: LOW|MEDIUM|HIGH|VERY_HIGH\n\
evolution_cost: LOW|MEDIUM|HIGH|VERY_HIGH\n\
\n\
time_to_first_clear_value: IMMEDIATE|SHORT|MEDIUM|LONG|VERY_LONG\n\n"
}

fn needs_stack_base_cure_for_block3(prior: &MasterSolutionsRow) -> bool {
    let value = prior.stack_base.trim();
    value.is_empty() || value.eq_ignore_ascii_case("unknown") || value.eq_ignore_ascii_case("n/a")
}

fn needs_licenca_cure_for_block3(prior: &MasterSolutionsRow) -> bool {
    let value = prior.licenca.trim();
    value.is_empty() || value.eq_ignore_ascii_case("unknown") || value.eq_ignore_ascii_case("n/a")
}

fn normalize_stack_base_cure_value(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") || trimmed.eq_ignore_ascii_case("n/a") {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "rust" => Some("Rust"),
        "python" => Some("Python"),
        "nodejs" | "node.js" | "node" | "javascript" | "typescript" => Some("NodeJS"),
        "go" | "golang" => Some("Go"),
        "jvm" | "java" | "kotlin" | "scala" => Some("JVM"),
        "dotnet" | ".net" | "c#" | "csharp" | "f#" | "fsharp" => Some("DotNet"),
        "mixed" => Some("Mixed"),
        _ => None,
    }
}

fn normalize_licenca_cure_value(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") || trimmed.eq_ignore_ascii_case("n/a") {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    let normalized = match upper.as_str() {
        "MIT" => "MIT",
        "APACHE-2.0" | "APACHE 2.0" | "APACHE2.0" => "Apache-2.0",
        "GPL" | "GPL-3.0" | "GPL-3" => "GPL-3.0",
        "GPL-2.0" | "GPL-2" => "GPL-2.0",
        "LGPL-3.0" | "LGPL-3" => "LGPL-3.0",
        "AGPL-3.0" | "AGPL-3" => "AGPL-3.0",
        "BSD-3-CLAUSE" | "BSD 3-CLAUSE" | "BSD-3" => "BSD-3-Clause",
        "BSD-2-CLAUSE" | "BSD 2-CLAUSE" | "BSD-2" => "BSD-2-Clause",
        "MPL-2.0" | "MPL 2.0" | "MPL2.0" => "MPL-2.0",
        "ISC" => "ISC",
        "UNLICENSE" => "Unlicense",
        "COPYRIGHTED" | "COPYRIGHT" => "Copyrighted",
        "NÃO ESPECIFICADO" | "NAO ESPECIFICADO" | "NAO_ESPECIFICADO" | "NÃO_ESPECIFICADO" => {
            "NÃO ESPECIFICADO"
        }
        _ => return None,
    };
    Some(normalized.to_string())
}

fn compact_context_for_block(
    block0: &Block0Context,
    row: &MasterSolutionsRow,
    block: u8,
) -> serde_json::Value {
    let mut ctx = serde_json::Map::new();
    let stack_trimmed = block0.stack_base.trim();
    if stack_trimmed.is_empty()
        || stack_trimmed.eq_ignore_ascii_case("unknown")
        || stack_trimmed.eq_ignore_ascii_case("n/a")
    {
        ctx.insert(
            "context_alert".to_string(),
            serde_json::json!(
                "ALERTA: Repositório de Conhecimento/Metodologia (stack_base UNKNOWN). Ignore exigências de código fonte/hardware (AVX2/Bare-Metal). Avalie prompts, padrões teóricos e processo para canibalização."
            ),
        );
    }
    ctx.insert(
        "project".to_string(),
        serde_json::json!({
            "project_name": &block0.project_name,
            "repo_url": &block0.repo_url,
            "repo_analised_version": &block0.repo_analised_version,
            "ultima_versao_online": &block0.ultima_versao_online,
            "lote_id": &block0.lote_id,
            "data_ultima_analise": block0.data_ultima_analise,
            "analise_origem": &block0.analise_origem,
            "licenca": &block0.licenca,
            "stack_base": &block0.stack_base,
            "declared_description": &block0.declared_description
        }),
    );
    ctx.insert(
        "debates_enxame".to_string(),
        serde_json::json!({
            "lente_a_sentido_prod_ux": truncate_chars_simple(&row.lente_a_sentido_prod_ux, 4200),
            "lente_b_estrutura_arq": truncate_chars_simple(&row.lente_b_estrutura_arq, 4200),
            "lente_c_realidade_ops": truncate_chars_simple(&row.lente_c_realidade_ops, 4200)
        }),
    );

    if matches!(block, BLOCK_2A | BLOCK_2B | BLOCK_3 | BLOCK_4) {
        ctx.insert(
            "curation".to_string(),
            serde_json::json!({
                "proposta_original_resumo": truncate_chars_simple(&row.proposta_original_resumo, 2200)
            }),
        );
    }
    if matches!(block, BLOCK_2B | BLOCK_3 | BLOCK_4) {
        ctx.insert(
            "block2a".to_string(),
            serde_json::json!({
                "indicacao_otimista_canibalizacao": truncate_chars_simple(&row.indicacao_otimista_canibalizacao, 3000),
                "ouro_a_extrair": &row.ouro_a_extrair,
                "deep_pattern": &row.deep_pattern,
                "transplantable_core": &row.transplantable_core,
                "logic_math_heuristic": &row.logic_math_heuristic,
                "real_structural_problem": &row.real_structural_problem,
                "categoria_nuance_tecnica": &row.categoria_nuance_tecnica,
                "integracao_papel_exato": &row.integracao_papel_exato
            }),
        );
    }
    if matches!(block, BLOCK_3 | BLOCK_4) {
        ctx.insert(
            "block2b".to_string(),
            serde_json::json!({
                "must_components_prod_ux": &row.must_components_prod_ux,
                "must_components_arq": &row.must_components_arq,
                "must_components_ops": &row.must_components_ops,
                "detected_toxic_deps": &row.detected_toxic_deps,
                "do_not_absorb": &row.do_not_absorb,
                "where_ai_should_not_enter": &row.where_ai_should_not_enter
            }),
        );
    }
    if matches!(block, BLOCK_4) {
        ctx.insert(
            "block3".to_string(),
            serde_json::json!({
                "classificacao_terminal": &row.classificacao_terminal,
                "acao_de_canibalizacao": &row.acao_de_canibalizacao,
                "categoria_arquitetural": &row.categoria_arquitetural,
                "horizonte_extracao": &row.horizonte_extracao,
                "tipo_integracao": &row.tipo_integracao,
                "capability_nature_primary": &row.capability_nature_primary,
                "architectural_topology": &row.architectural_topology,
                "temporal_stability": &row.temporal_stability,
                "bare_metal_fit": &row.bare_metal_fit,
                "extractability_level": &row.extractability_level,
                "runtime_sovereignty_fit": &row.runtime_sovereignty_fit,
                "local_first_fit": &row.local_first_fit,
                "adoptability_level": &row.adoptability_level,
                "longitudinal_sustainability": &row.longitudinal_sustainability,
                "maintenance_burden": &row.maintenance_burden,
                "onboarding_friction": &row.onboarding_friction,
                "observability_operational": &row.observability_operational,
                "recoverability_level": &row.recoverability_level,
                "degradation_behavior": &row.degradation_behavior,
                "curation_burden": &row.curation_burden,
                "evolution_cost": &row.evolution_cost,
                "operability_level": &row.operability_level,
                "abandonment_risk": &row.abandonment_risk,
                "time_to_first_clear_value": &row.time_to_first_clear_value,
                "imperfection_tolerance": &row.imperfection_tolerance,
                "entropy_risk": &row.entropy_risk,
                "design_misuse_risk": &row.design_misuse_risk,
                "intrinsic_ethics_risk": &row.intrinsic_ethics_risk,
                "discipline_dependency": &row.discipline_dependency,
                "regulatory_risk": &row.regulatory_risk
            }),
        );
    }

    serde_json::Value::Object(ctx)
}

async fn run_block<T: for<'de> Deserialize<'de> + Send>(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block: u8,
    block0: &Block0Context,
    row: &MasterSolutionsRow,
) -> Result<T, Phase3Error> {
    Ok(run_block_envelope::<T>(client, cfg, block, block0, row).await?.fields)
}

async fn run_block_envelope<T: for<'de> Deserialize<'de> + Send>(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block: u8,
    block0: &Block0Context,
    row: &MasterSolutionsRow,
) -> Result<BlockResponse<T>, Phase3Error> {
    let mut last_error: Option<String> = None;
    let attempts = cfg.max_attempts_per_block.max(1);
    let model = &cfg.model;
    for attempt in 1..=attempts {
        if attempt == 1 {
            info!(block, attempts, "F3 (Sintetizador SGR): iniciando sub-chamada do bloco");
        } else {
            warn!(block, attempt, "F3 (Sintetizador SGR): retry do bloco (injetando erro anterior no prompt)");
        }
        let prompt = build_prompt(block, block0, row, last_error.as_deref());
        let formatted = client
            .format(model, &prompt)
            .await
            .map_err(Phase3Error::Transport)?;
        let json_text = match extract_json_fence(&formatted) {
            Ok(json) => json,
            Err(err) => {
                last_error = Some(err.to_string());
                warn!(block, attempt, error = %err, "F3 (Sintetizador SGR): falha ao extrair JSON (code-fence ou bruto)");
                if attempt == attempts {
                    return Err(Phase3Error::RetryExhausted {
                        block,
                        attempts,
                        message: last_error.unwrap_or_else(|| "unknown".to_string()),
                    });
                }
                continue;
            }
        };

        let parsed: Result<BlockResponse<T>, _> = serde_json::from_str(&json_text);
        match parsed {
            Ok(envelope) => {
                info!(block, attempt, "F3 (Sintetizador SGR): bloco concluído");
                return Ok(envelope);
            }
            Err(e) => {
                last_error = Some(e.to_string());
                warn!(block, attempt, error = %e, "F3 (Sintetizador SGR): falha de schema/serde no JSON do bloco");
                if attempt == attempts {
                    return Err(Phase3Error::RetryExhausted {
                        block,
                        attempts,
                        message: last_error.unwrap_or_else(|| "unknown".to_string()),
                    });
                }
            }
        }
    }

    Err(Phase3Error::RetryExhausted {
        block,
        attempts: cfg.max_attempts_per_block.max(1),
        message: last_error.unwrap_or_else(|| "unknown".to_string()),
    })
}

async fn run_block3_with_fallback(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block0: &Block0Context,
    row: &MasterSolutionsRow,
    start_model_index: usize,
    feedback_seed: Option<&str>,
) -> Result<Block3Execution, Phase3Error> {
    let models = cfg.block3_models();
    if start_model_index >= models.len() {
        return Err(Phase3Error::SchemaFailure {
            block: BLOCK_3,
            message: "BLOCK_3 ficou sem modelos de fallback disponiveis".to_string(),
        });
    }

    let attempts = cfg.max_attempts_per_block.max(1);
    let mut model_errors: Vec<String> = Vec::new();
    for (model_index, model) in models.iter().enumerate().skip(start_model_index) {
        let mut last_error = feedback_seed.map(|value| value.to_string());
        for attempt in 1..=attempts {
            if attempt == 1 {
                info!(
                    block = BLOCK_3,
                    attempt,
                    model = %model,
                    "F3 (Sintetizador SGR): iniciando bloco 3 com candidato atual"
                );
            } else {
                warn!(
                    block = BLOCK_3,
                    attempt,
                    model = %model,
                    "F3 (Sintetizador SGR): retry do bloco 3 no mesmo modelo"
                );
            }
            let prompt = build_prompt(BLOCK_3, block0, row, last_error.as_deref());
            let formatted = match client.format(model, &prompt).await {
                Ok(value) => value,
                Err(err) => {
                    last_error = Some(err.clone());
                    warn!(
                        block = BLOCK_3,
                        attempt,
                        model = %model,
                        error = %err,
                        "F3 (Sintetizador SGR): falha de transporte no bloco 3"
                    );
                    if attempt == attempts {
                        break;
                    }
                    continue;
                }
            };
            let json_text = match extract_json_fence(&formatted) {
                Ok(json) => json,
                Err(err) => {
                    last_error = Some(err.to_string());
                    warn!(
                        block = BLOCK_3,
                        attempt,
                        model = %model,
                        error = %err,
                        "F3 (Sintetizador SGR): falha ao extrair JSON do bloco 3"
                    );
                    if attempt == attempts {
                        break;
                    }
                    continue;
                }
            };
            let payload: serde_json::Value =
                serde_json::from_str(&json_text).unwrap_or(serde_json::Value::Null);
            let parsed: Result<BlockResponse<Block3Fields>, _> = serde_json::from_str(&json_text);
            let envelope = match parsed {
                Ok(envelope) => envelope,
                Err(err) => {
                    last_error = Some(err.to_string());
                    warn!(
                        block = BLOCK_3,
                        attempt,
                        model = %model,
                        error = %err,
                        "F3 (Sintetizador SGR): falha de schema/serde no bloco 3"
                    );
                    if attempt == attempts {
                        break;
                    }
                    continue;
                }
            };
            if envelope.justifications.is_empty() {
                last_error =
                    Some("BLOCK_3 requires non-empty justifications; model returned empty".to_string());
                warn!(
                    block = BLOCK_3,
                    attempt,
                    model = %model,
                    "F3 (Sintetizador SGR): bloco 3 rejeitado por justifications vazias"
                );
                if attempt == attempts {
                    break;
                }
                continue;
            }
            let homogeneous_medium = block3_looks_homogeneous(&payload);
            if homogeneous_medium && !block3_justifications_show_discrimination(&envelope.justifications) {
                last_error = Some(
                    "BLOCK_3 homogeneous output detected with weak justifications; regenerate with finer discrimination"
                        .to_string(),
                );
                warn!(
                    block = BLOCK_3,
                    attempt,
                    model = %model,
                    "F3 (Sintetizador SGR): bloco 3 rejeitado por homogeneização sem lastro"
                );
                if attempt == attempts {
                    break;
                }
                continue;
            }
            info!(
                block = BLOCK_3,
                attempt,
                model = %model,
                homogeneous_medium,
                "F3 (Sintetizador SGR): bloco 3 concluído"
            );
            return Ok(Block3Execution {
                envelope,
                model_used: model.clone(),
                model_index,
                homogeneous_medium,
            });
        }
        let model_error = last_error.unwrap_or_else(|| "unknown".to_string());
        model_errors.push(format!("{model}: {model_error}"));
        warn!(
            block = BLOCK_3,
            model = %model,
            error = %model_error,
            "F3 (Sintetizador SGR): modelo do bloco 3 esgotado; tentando fallback"
        );
    }
    Err(Phase3Error::RetryExhausted {
        block: BLOCK_3,
        attempts: attempts * (models.len() - start_model_index),
        message: model_errors.join(" | "),
    })
}

fn apply_block3_fields_to_row(row: &mut MasterSolutionsRow, block3: &Block3Fields) {
    row.classificacao_terminal = block3.classificacao_terminal;
    row.acao_de_canibalizacao = block3.acao_de_canibalizacao;
    if let Some(value) = block3.categoria_arquitetural {
        row.categoria_arquitetural = value;
    }
    row.horizonte_extracao = block3.horizonte_extracao;
    row.tipo_integracao = block3.tipo_integracao;
    row.capability_nature_primary = block3.capability_nature_primary;
    row.architectural_topology = block3.architectural_topology;
    row.temporal_stability = block3.temporal_stability;
    row.bare_metal_fit = block3.bare_metal_fit;
    row.extractability_level = block3.extractability_level;
    row.runtime_sovereignty_fit = block3.runtime_sovereignty_fit;
    row.local_first_fit = block3.local_first_fit;
    row.adoptability_level = block3.adoptability_level;
    row.longitudinal_sustainability = block3.longitudinal_sustainability;
    row.maintenance_burden = block3.maintenance_burden;
    row.onboarding_friction = block3.onboarding_friction;
    row.observability_operational = block3.observability_operational;
    row.recoverability_level = block3.recoverability_level;
    row.degradation_behavior = block3.degradation_behavior;
    row.curation_burden = block3.curation_burden;
    row.evolution_cost = block3.evolution_cost;
    row.operability_level = block3.operability_level;
    row.abandonment_risk = block3.abandonment_risk;
    row.time_to_first_clear_value = block3.time_to_first_clear_value;
    row.imperfection_tolerance = block3.imperfection_tolerance;
    row.entropy_risk = block3.entropy_risk;
    row.design_misuse_risk = block3.design_misuse_risk;
    row.intrinsic_ethics_risk = block3.intrinsic_ethics_risk;
    row.discipline_dependency = block3.discipline_dependency;
    row.regulatory_risk = block3.regulatory_risk;
    if let Some(value) = block3.stack_base.as_deref() {
        row.stack_base = value.to_string();
    }
    if let Some(value) = block3.licenca.as_deref() {
        row.licenca = value.to_string();
    }
}

fn persist_block3_checkpoint(
    repo_id: &str,
    row: &mut MasterSolutionsRow,
    block3_justifications: &HashMap<String, String>,
    now_epoch: i64,
) -> Result<(), Phase3Error> {
    let conn = SsotInjector::open_vault_connection().map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
    SsotInjector::ensure_repo_heuristics_schema(&conn).map_err(Phase3Error::L2Failure)?;
    SsotInjector::ensure_repo_heuristics_justifications_schema(&conn).map_err(Phase3Error::L2Failure)?;
    let _ = conn.execute(
        "UPDATE repo_heuristics
         SET classificacao_terminal = ?2,
             acao_de_canibalizacao = ?3,
             categoria_arquitetural = ?4,
             horizonte_extracao = ?5,
             tipo_integracao = ?6,
             capability_nature_primary = ?7,
             architectural_topology = ?8,
             temporal_stability = ?9,
             bare_metal_fit = ?10,
             extractability_level = ?11,
             runtime_sovereignty_fit = ?12,
             local_first_fit = ?13,
             adoptability_level = ?14,
             longitudinal_sustainability = ?15,
             maintenance_burden = ?16,
             onboarding_friction = ?17,
             observability_operational = ?18,
             recoverability_level = ?19,
             degradation_behavior = ?20,
             curation_burden = ?21,
             evolution_cost = ?22,
             operability_level = ?23,
             abandonment_risk = ?24,
             time_to_first_clear_value = ?25,
             imperfection_tolerance = ?26,
             entropy_risk = ?27,
             design_misuse_risk = ?28,
             intrinsic_ethics_risk = ?29,
             discipline_dependency = ?30,
             regulatory_risk = ?31,
             status_fase = ?32
         WHERE project_name = ?1",
        params![
            repo_id,
            row.classificacao_terminal.as_str(),
            row.acao_de_canibalizacao.as_str(),
            row.categoria_arquitetural.as_str(),
            row.horizonte_extracao.as_str(),
            row.tipo_integracao.as_str(),
            row.capability_nature_primary.as_str(),
            row.architectural_topology.as_str(),
            row.temporal_stability.as_str(),
            row.bare_metal_fit.as_str(),
            row.extractability_level.as_str(),
            row.runtime_sovereignty_fit.as_str(),
            row.local_first_fit.as_str(),
            row.adoptability_level.as_str(),
            row.longitudinal_sustainability.as_str(),
            row.maintenance_burden.as_str(),
            row.onboarding_friction.as_str(),
            row.observability_operational.as_str(),
            row.recoverability_level.as_str(),
            row.degradation_behavior.as_str(),
            row.curation_burden.as_str(),
            row.evolution_cost.as_str(),
            row.operability_level.as_str(),
            row.abandonment_risk.as_str(),
            row.time_to_first_clear_value.as_str(),
            row.imperfection_tolerance.as_str(),
            row.entropy_risk.as_str(),
            row.design_misuse_risk.as_str(),
            row.intrinsic_ethics_risk.as_str(),
            row.discipline_dependency.as_str(),
            row.regulatory_risk.as_str(),
            "FASE_3_BLOCK_3_OK"
        ],
    )
    .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
    if !block3_justifications.is_empty() {
        let json_text = serde_json::to_string(block3_justifications).unwrap_or_else(|_| "{}".to_string());
        let _ = conn.execute(
            "INSERT INTO repo_heuristics_justifications (project_name, block, justifications_json, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_name, block) DO UPDATE SET
                justifications_json = excluded.justifications_json,
                created_at = excluded.created_at",
            params![repo_id, 3_i64, json_text, now_epoch],
        );
    }
    row.status_fase = "FASE_3_BLOCK_3_OK".to_string();
    Ok(())
}

async fn run_block4_validated(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block0: &Block0Context,
    row: &MasterSolutionsRow,
) -> Result<Block4Fields, Phase3Error> {
    let block: u8 = 4;
    let mut last_error: Option<String> = None;
    let attempts = cfg.max_attempts_per_block.max(1);

    for attempt in 1..=attempts {
        if attempt == 1 {
            info!(block, attempts, "F3 (Sintetizador SGR): iniciando sub-chamada do bloco");
        } else {
            warn!(block, attempt, "F3 (Sintetizador SGR): retry do bloco (injetando erro anterior no prompt)");
        }

        let prompt = build_prompt(block, block0, row, last_error.as_deref());
        let formatted = client
            .format(&cfg.model, &prompt)
            .await
            .map_err(Phase3Error::Transport)?;
        let json_text = match extract_json_fence(&formatted) {
            Ok(json) => json,
            Err(err) => {
                last_error = Some(err.to_string());
                warn!(
                    block,
                    attempt,
                    error = %err,
                    "F3 (Sintetizador SGR): falha ao extrair JSON (code-fence ou bruto)"
                );
                if attempt == attempts {
                    return Err(Phase3Error::RetryExhausted {
                        block,
                        attempts,
                        message: last_error.unwrap_or_else(|| "unknown".to_string()),
                    });
                }
                continue;
            }
        };

        let parsed: Result<BlockResponse<Block4Fields>, _> = serde_json::from_str(&json_text);
        let envelope = match parsed {
            Ok(envelope) => envelope,
            Err(e) => {
                last_error = Some(e.to_string());
                warn!(
                    block,
                    attempt,
                    error = %e,
                    "F3 (Sintetizador SGR): falha de schema/serde no JSON do bloco"
                );
                if attempt == attempts {
                    return Err(Phase3Error::RetryExhausted {
                        block,
                        attempts,
                        message: last_error.unwrap_or_else(|| "unknown".to_string()),
                    });
                }
                continue;
            }
        };

        let fields = envelope.fields;
        let validations = [
            validate_score_0_10("score_philosophical_fit", fields.score_philosophical_fit, 4),
            validate_score_0_10("score_bare_metal_fit", fields.score_bare_metal_fit, 4),
            validate_score_0_10(
                "score_architectural_extractability",
                fields.score_architectural_extractability,
                4,
            ),
            validate_score_0_10("score_operability", fields.score_operability, 4),
            validate_score_0_10("score_creep_risk", fields.score_creep_risk, 4),
            validate_score_0_10(
                "score_runtime_sovereignty",
                fields.score_runtime_sovereignty,
                4,
            ),
            validate_score_0_10("score_model_logic_value", fields.score_model_logic_value, 4),
            validate_score_0_10("score_ethics_safety", fields.score_ethics_safety, 4),
            validate_score_0_10("score_intrinsic_risk", fields.score_intrinsic_risk, 4),
        ];

        if let Some(err) = validations.into_iter().find_map(|res| res.err()) {
            last_error = Some(err.to_string());
            warn!(block, attempt, error = %err, "F3 (Sintetizador SGR): falha de validação no Bloco 4");
            if attempt == attempts {
                return Err(Phase3Error::RetryExhausted {
                    block,
                    attempts,
                    message: last_error.unwrap_or_else(|| "unknown".to_string()),
                });
            }
            continue;
        }

        info!(block, attempt, "F3 (Sintetizador SGR): bloco concluído");
        return Ok(fields);
    }

    Err(Phase3Error::RetryExhausted {
        block,
        attempts,
        message: last_error.unwrap_or_else(|| "unknown".to_string()),
    })
}

fn reconcile_checkpoint_stage(status_stage: u8, content_stage: u8) -> u8 {
    if status_stage >= 5 && content_stage < 5 {
        content_stage
    } else {
        status_stage.max(content_stage)
    }
}

fn is_block2a_complete(row: &MasterSolutionsRow) -> bool {
    !row.indicacao_otimista_canibalizacao.trim().is_empty()
        && !row.ouro_a_extrair.trim().is_empty()
        && !row.deep_pattern.trim().is_empty()
        && !row.transplantable_core.trim().is_empty()
        && !row.logic_math_heuristic.trim().is_empty()
        && !row.real_structural_problem.trim().is_empty()
        && !row.categoria_nuance_tecnica.trim().is_empty()
        && !row.integracao_papel_exato.trim().is_empty()
}

pub async fn run_phase3_sgr(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block0: Block0Context,
) -> Result<Phase3Output, Phase3Error> {
    fn stage_from_status_fase(status_fase: &str) -> u8 {
        match status_fase.trim() {
            "FASE_3_BLOCK_1_OK" => 1,
            "FASE_3_BLOCK_2A_OK" => 2,
            "FASE_3_BLOCK_2B_OK" => 3,
            "FASE_3_BLOCK_3_OK" => 4,
            "FASE_3_SINTETIZADOR_OK" | "FASE_3_SYNTHESIZER_OK" | "FASE_4_SHEETS_UPDATED" | "ERRO_FASE_4" => 5,
            _ => 0,
        }
    }

    fn infer_stage_from_row(row: &MasterSolutionsRow) -> u8 {
        let mut stage = 0u8;
        let block1_ok = !row.justificativa_decisao.trim().is_empty()
            && !row.executive_verdict.trim().is_empty()
            && !row.visao_do_enxame.trim().is_empty();
        if block1_ok {
            stage = 1;
        }
        let block2a_ok = stage >= 1 && is_block2a_complete(row);
        // region debug-point phase3-block2a-completeness
        if stage >= 1 && block2a_ok && row.indicacao_otimista_canibalizacao.trim().is_empty() {
            warn!(
                repo_id = %row.project_name,
                block2a_ok,
                indicacao_empty = row.indicacao_otimista_canibalizacao.trim().is_empty(),
                ouro_empty = row.ouro_a_extrair.trim().is_empty(),
                deep_pattern_empty = row.deep_pattern.trim().is_empty(),
                transplantable_core_empty = row.transplantable_core.trim().is_empty(),
                logic_math_heuristic_empty = row.logic_math_heuristic.trim().is_empty(),
                real_structural_problem_empty = row.real_structural_problem.trim().is_empty(),
                categoria_nuance_tecnica_empty = row.categoria_nuance_tecnica.trim().is_empty(),
                integracao_papel_exato_empty = row.integracao_papel_exato.trim().is_empty(),
                "F3 debug: bloco 2A considerado completo com indicacao_otimista_canibalizacao vazia"
            );
        }
        // endregion debug-point phase3-block2a-completeness
        if block2a_ok {
            stage = 2;
        }
        let block2b_ok = stage >= 2
            && !row.must_components_prod_ux.trim().is_empty()
            && !row.must_components_arq.trim().is_empty()
            && !row.must_components_ops.trim().is_empty();
        if block2b_ok {
            stage = 3;
        }
        stage
    }

    info!(
        repo_id = %block0.project_name,
        model = %cfg.model,
        block3_models = ?cfg.block3_models(),
        "F3 (Sintetizador SGR): iniciando SGR em cascata (Blocos 1 -> 2A -> 2B -> 3 -> 4)"
    );
    let started_total = Instant::now();
    let state = Arc::new(tokio::sync::Mutex::new(Phase3TelemetryState {
        block: 0,
        label: "init".to_string(),
        block_started: Instant::now(),
    }));
    let _telemetry_total = spawn_phase3_total_telemetry(block0.project_name.clone(), started_total, state.clone());
    let repo_id = block0.project_name.clone();
    let now_epoch = block0.data_ultima_analise;
    let mut row = MasterSolutionsRow::from_block0(block0.clone());
    let mut block3_justifications: HashMap<String, String> = HashMap::new();
    let mut block3_model_used = cfg
        .block3_models()
        .into_iter()
        .next()
        .unwrap_or_else(|| cfg.model.clone());
    let mut block3_model_index = 0usize;
    let mut stage: u8 = 0;

    if let Some(existing) = SsotInjector::try_load_repo_heuristics_row(&repo_id)
        .map_err(|e| Phase3Error::L2Failure(e.to_string()))?
    {
        let stage_from_status = stage_from_status_fase(existing.status_fase.as_str());
        let stage_from_content = infer_stage_from_row(&existing);
        stage = if stage_from_status >= 5 && stage_from_content < 5 {
            warn!(
                repo_id = %repo_id,
                status_fase = %existing.status_fase,
                stage_from_status,
                stage_from_content,
                "F3: checkpoint terminal invalidado; payload persistido incompleto"
            );
            reconcile_checkpoint_stage(stage_from_status, stage_from_content)
        } else {
            reconcile_checkpoint_stage(stage_from_status, stage_from_content)
        };
        if stage > 0 {
            row = existing;
            block3_justifications = SsotInjector::load_block3_justifications(&repo_id)
                .unwrap_or_default();
        }
    }
    let mut block3_homogeneous_medium = stage >= 4 && block3_row_looks_homogeneous(&row);

    if stage >= 5 {
        info!(
            repo_id = %repo_id,
            status_fase = %row.status_fase,
            "F3 (Sintetizador SGR): checkpoint detectado; pulando Fase 3 (já concluída)"
        );
        return Ok(Phase3Output {
            model_used: cfg.model.clone(),
            row,
            block3_justifications,
        });
    }

    if stage < 1 {
        set_phase3_block(&state, 1, "Bloco 1").await;
        let _telemetry_block1 =
            spawn_phase3_block_telemetry(block0.project_name.clone(), started_total, state.clone());
        let block1: Block1Fields = run_block(client, cfg, BLOCK_1, &block0, &row).await?;
        drop(_telemetry_block1);
        if let Some(value) = block1.proposta_original_resumo {
            if !value.trim().is_empty() {
                row.proposta_original_resumo = value;
            }
        }
        row.declared_description_ptbr = block1.declared_description_ptbr;
        row.visao_do_enxame = block1.visao_do_enxame;
        row.justificativa_decisao = block1.justificativa_decisao;
        row.executive_verdict = block1.executive_verdict;
        row.risco_principal = block1.risco_principal;
        row.risco_linha_vermelha = block1.risco_linha_vermelha;
        row.observacoes = block1.observacoes;

        row.status_fase = "FASE_3_BLOCK_1_OK".to_string();
        SsotInjector::checkpoint_upsert_repo_heuristics_full(
            &repo_id,
            &row,
            row.status_atualizacao.as_str(),
            row.status_fase.as_str(),
            &HashMap::new(),
            now_epoch,
        )
        .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
        stage = 1;
        info!("F3 (Sintetizador SGR): Bloco 1 concluído (checkpoint OK)");
    } else {
        info!(repo_id = %repo_id, "F3 (Sintetizador SGR): Bloco 1 já está no SQLite; skip");
    }

    if stage < 2 {
        set_phase3_block(&state, 2, "Bloco 2A").await;
        let _telemetry_block2a =
            spawn_phase3_block_telemetry(block0.project_name.clone(), started_total, state.clone());
        let block2a: Block2NarrativeFields =
            run_block::<Block2NarrativeFields>(client, cfg, BLOCK_2A, &block0, &row).await?;
        drop(_telemetry_block2a);
        row.indicacao_otimista_canibalizacao = block2a.indicacao_otimista_canibalizacao;
        row.ouro_a_extrair = block2a.ouro_a_extrair;
        row.deep_pattern = block2a.deep_pattern;
        row.transplantable_core = block2a.transplantable_core;
        row.logic_math_heuristic = block2a.logic_math_heuristic;
        row.real_structural_problem = block2a.real_structural_problem;
        row.categoria_nuance_tecnica = block2a.categoria_nuance_tecnica;
        row.integracao_papel_exato = block2a.integracao_papel_exato;

        {
            let conn = SsotInjector::open_vault_connection()
                .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
            SsotInjector::ensure_repo_heuristics_schema(&conn)
                .map_err(Phase3Error::L2Failure)?;
            let _ = conn.execute(
                "UPDATE repo_heuristics
                 SET indicacao_otimista_canibalizacao = ?2,
                     ouro_a_extrair = ?3,
                     deep_pattern = ?4,
                     transplantable_core = ?5,
                     logic_math_heuristic = ?6,
                     real_structural_problem = ?7,
                     categoria_nuance_tecnica = ?8,
                     integracao_papel_exato = ?9,
                     status_fase = ?10
                 WHERE project_name = ?1",
                params![
                    repo_id.as_str(),
                    &row.indicacao_otimista_canibalizacao,
                    &row.ouro_a_extrair,
                    &row.deep_pattern,
                    &row.transplantable_core,
                    &row.logic_math_heuristic,
                    &row.real_structural_problem,
                    &row.categoria_nuance_tecnica,
                    &row.integracao_papel_exato,
                    "FASE_3_BLOCK_2A_OK"
                ],
            )
            .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
        }
        row.status_fase = "FASE_3_BLOCK_2A_OK".to_string();
        stage = 2;
        info!("F3 (Sintetizador SGR): Bloco 2A concluído (checkpoint OK)");
    } else {
        info!(repo_id = %repo_id, "F3 (Sintetizador SGR): Bloco 2A já está no SQLite; skip");
    }

    if stage < 3 {
        set_phase3_block(&state, 2, "Bloco 2B").await;
        let _telemetry_block2b =
            spawn_phase3_block_telemetry(block0.project_name.clone(), started_total, state.clone());
        let block2b: Block2MatrixFields = run_block::<Block2MatrixFields>(client, cfg, BLOCK_2B, &block0, &row)
            .await?
            .sanitize();
        drop(_telemetry_block2b);
        row.must_components_prod_ux = format_dot_bullets(&block2b.must_components_prod_ux);
        row.must_components_arq = format_dot_bullets(&block2b.must_components_arq);
        row.must_components_ops = format_dot_bullets(&block2b.must_components_ops);
        row.detected_toxic_deps = format_dot_bullets(&block2b.detected_toxic_deps);
        row.do_not_absorb = format_dot_bullets(&block2b.do_not_absorb);
        row.where_ai_should_not_enter = format_dot_bullets(&block2b.where_ai_should_not_enter);

        {
            let conn = SsotInjector::open_vault_connection()
                .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
            SsotInjector::ensure_repo_heuristics_schema(&conn)
                .map_err(Phase3Error::L2Failure)?;
            let _ = conn.execute(
                "UPDATE repo_heuristics
                 SET must_components_prod_ux = ?2,
                     must_components_arq = ?3,
                     must_components_ops = ?4,
                     detected_toxic_deps = ?5,
                     do_not_absorb = ?6,
                     where_ai_should_not_enter = ?7,
                     status_fase = ?8
                 WHERE project_name = ?1",
                params![
                    repo_id.as_str(),
                    &row.must_components_prod_ux,
                    &row.must_components_arq,
                    &row.must_components_ops,
                    &row.detected_toxic_deps,
                    &row.do_not_absorb,
                    &row.where_ai_should_not_enter,
                    "FASE_3_BLOCK_2B_OK"
                ],
            )
            .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
        }
        row.status_fase = "FASE_3_BLOCK_2B_OK".to_string();
        stage = 3;
        info!("F3 (Sintetizador SGR): Bloco 2B concluído (checkpoint OK)");
    } else {
        info!(repo_id = %repo_id, "F3 (Sintetizador SGR): Bloco 2B já está no SQLite; skip");
    }

    if stage < 4 {
        set_phase3_block(&state, 3, "Bloco 3 (ENUMs)").await;
        let _telemetry_block3 =
            spawn_phase3_block_telemetry(block0.project_name.clone(), started_total, state.clone());
        let block3_exec = run_block3_with_fallback(client, cfg, &block0, &row, 0, None).await?;
        drop(_telemetry_block3);
        block3_model_used = block3_exec.model_used.clone();
        block3_model_index = block3_exec.model_index;
        block3_homogeneous_medium = block3_exec.homogeneous_medium;
        block3_justifications = block3_exec.envelope.justifications;
        // region debug-point phase3-block3-raw-fields
        info!(
            repo_id = %repo_id,
            block3_model_used = %block3_model_used,
            raw_adoptability_level = %format!("{:?}", block3_exec.envelope.fields.adoptability_level),
            raw_bare_metal_fit = %format!("{:?}", block3_exec.envelope.fields.bare_metal_fit),
            raw_maintenance_burden = %format!("{:?}", block3_exec.envelope.fields.maintenance_burden),
            raw_runtime_sovereignty_fit = %format!("{:?}", block3_exec.envelope.fields.runtime_sovereignty_fit),
            raw_observability_operational = %format!("{:?}", block3_exec.envelope.fields.observability_operational),
            raw_recoverability_level = %format!("{:?}", block3_exec.envelope.fields.recoverability_level),
            raw_degradation_behavior = %format!("{:?}", block3_exec.envelope.fields.degradation_behavior),
            raw_entropy_risk = %format!("{:?}", block3_exec.envelope.fields.entropy_risk),
            raw_regulatory_risk = %format!("{:?}", block3_exec.envelope.fields.regulatory_risk),
            justifications_keys = block3_justifications.len(),
            "F3 debug: bloco 3 fields brutos antes de sanitize/persist"
        );
        // endregion debug-point phase3-block3-raw-fields
        let block3 = block3_exec
            .envelope
            .fields
            .sanitize(
                needs_stack_base_cure_for_block3(&row),
                needs_licenca_cure_for_block3(&row),
            )
            .map_err(|message| Phase3Error::SchemaFailure { block: BLOCK_3, message })?;
        apply_block3_fields_to_row(&mut row, &block3);
        persist_block3_checkpoint(&repo_id, &mut row, &block3_justifications, now_epoch)?;
        stage = 4;
        info!("F3 (Sintetizador SGR): Bloco 3 concluído (checkpoint OK)");
    } else {
        info!(repo_id = %repo_id, "F3 (Sintetizador SGR): Bloco 3 já está no SQLite; skip");
        block3_justifications = SsotInjector::load_block3_justifications(&repo_id)
            .unwrap_or_default();
    }

    if stage < 5 {
        set_phase3_block(&state, 4, "Bloco 4 (Scores)").await;
        let _telemetry_block4 =
            spawn_phase3_block_telemetry(block0.project_name.clone(), started_total, state.clone());
        let mut block4: Block4Fields = run_block4_validated(client, cfg, &block0, &row).await?;
        while block3_homogeneous_medium && homogeneous_medium_conflicts_with_block4(&block4) {
            let feedback = block3_score_conflict_feedback(&block4);
            warn!(
                repo_id = %repo_id,
                block3_model_used = %block3_model_used,
                feedback = %feedback,
                "F3 (Sintetizador SGR): bloco 3 homogeneo conflitou com os scores do bloco 4; acionando fallback"
            );
            let block3_exec = run_block3_with_fallback(
                client,
                cfg,
                &block0,
                &row,
                block3_model_index + 1,
                Some(feedback.as_str()),
            )
            .await?;
            block3_model_used = block3_exec.model_used.clone();
            block3_model_index = block3_exec.model_index;
            block3_homogeneous_medium = block3_exec.homogeneous_medium;
            block3_justifications = block3_exec.envelope.justifications;
            let block3 = block3_exec
                .envelope
                .fields
                .sanitize(
                    needs_stack_base_cure_for_block3(&row),
                    needs_licenca_cure_for_block3(&row),
                )
                .map_err(|message| Phase3Error::SchemaFailure { block: BLOCK_3, message })?;
            apply_block3_fields_to_row(&mut row, &block3);
            persist_block3_checkpoint(&repo_id, &mut row, &block3_justifications, now_epoch)?;
            block4 = run_block4_validated(client, cfg, &block0, &row).await?;
        }
        drop(_telemetry_block4);

        row.score_philosophical_fit = block4.score_philosophical_fit;
        row.score_bare_metal_fit = block4.score_bare_metal_fit;
        row.score_architectural_extractability = block4.score_architectural_extractability;
        row.score_operability = block4.score_operability;
        row.score_creep_risk = block4.score_creep_risk;
        row.score_runtime_sovereignty = block4.score_runtime_sovereignty;
        row.score_model_logic_value = block4.score_model_logic_value;
        row.score_ethics_safety = block4.score_ethics_safety;
        row.score_intrinsic_risk = block4.score_intrinsic_risk;

        {
            let conn = SsotInjector::open_vault_connection()
                .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
            SsotInjector::ensure_repo_heuristics_schema(&conn)
                .map_err(Phase3Error::L2Failure)?;
            let _ = conn.execute(
                "UPDATE repo_heuristics
                 SET score_philosophical_fit = ?2,
                     score_bare_metal_fit = ?3,
                     score_architectural_extractability = ?4,
                     score_operability = ?5,
                     score_creep_risk = ?6,
                     score_runtime_sovereignty = ?7,
                     score_model_logic_value = ?8,
                     score_ethics_safety = ?9,
                     score_intrinsic_risk = ?10,
                     status_fase = ?11
                 WHERE project_name = ?1",
                params![
                    repo_id.as_str(),
                    row.score_philosophical_fit,
                    row.score_bare_metal_fit,
                    row.score_architectural_extractability,
                    row.score_operability,
                    row.score_creep_risk,
                    row.score_runtime_sovereignty,
                    row.score_model_logic_value,
                    row.score_ethics_safety,
                    row.score_intrinsic_risk,
                    "FASE_3_SINTETIZADOR_OK"
                ],
            )
            .map_err(|e| Phase3Error::L2Failure(e.to_string()))?;
            let _ = conn.execute(
                "UPDATE repositorios SET status_processamento = ?1 WHERE project_name = ?2",
                params!["FASE_3_SINTETIZADOR_OK", repo_id.as_str()],
            );
        }

        row.status_fase = "FASE_3_SINTETIZADOR_OK".to_string();
        info!("F3 (Sintetizador SGR): Bloco 4 concluído (checkpoint OK)");
    } else {
        info!(repo_id = %repo_id, "F3 (Sintetizador SGR): Bloco 4 já está no SQLite; skip");
    }

    Ok(Phase3Output {
        model_used: block3_model_used,
        row,
        block3_justifications,
    })
}

pub fn extract_json_fence(text: &str) -> Result<String, Phase3Error> {
    let trimmed = text.trim();
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
    {
        return Ok(trimmed.to_string());
    }
    let Some(start) = text.find("```json") else {
        if let Some(salvaged) = salvage_balanced_json(text) {
            return Ok(salvaged);
        }
        return Err(Phase3Error::JsonFenceParse("missing ```json fence".to_string()));
    };
    let after = &text[start + "```json".len()..];
    let after = after
        .strip_prefix("\r\n")
        .or_else(|| after.strip_prefix('\n'))
        .unwrap_or(after);
    if let Some(end) = after.find("```") {
        return Ok(after[..end].trim().to_string());
    }
    if let Some(salvaged) = salvage_balanced_json(after).or_else(|| salvage_balanced_json(text)) {
        return Ok(salvaged);
    }
    Err(Phase3Error::JsonFenceParse(
        "missing closing ``` fence".to_string(),
    ))
}

fn salvage_balanced_json(text: &str) -> Option<String> {
    let (start_idx, open) = text
        .char_indices()
        .find_map(|(idx, ch)| match ch {
            '{' => Some((idx, '{')),
            '[' => Some((idx, '[')),
            _ => None,
        })?;
    let close = if open == '{' { '}' } else { ']' };

    let mut depth: i64 = 0;
    let mut in_str = false;
    let mut escape = false;

    for (idx, ch) in text[start_idx..].char_indices() {
        let abs = start_idx + idx;
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_str = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start_idx..=abs].trim().to_string());
                }
            }
            _ => {}
        }
    }
    None
}

pub fn sheet_range_for_row(row_number_1based: u32) -> String {
    let end_col = col_idx_to_a1(MASTER_SOLUTIONS_CANONICAL_COLUMNS.len().saturating_sub(1));
    format!("A{}:{}{}", row_number_1based, end_col, row_number_1based)
}

pub fn master_solutions_header_range() -> String {
    let end_col = col_idx_to_a1(MASTER_SOLUTIONS_CANONICAL_COLUMNS.len().saturating_sub(1));
    format!("A1:{}1", end_col)
}

pub const MASTER_SOLUTIONS_CANONICAL_COLUMNS: [&str; 82] = [
    "project_name",
    "repo_url",
    "repo_version",
    "ultima_versao_online",
    "lote_id",
    "data_ultima_analise",
    "analise_origem",
    "licenca",
    "stack_base",
    "declared_description",
    "lente_a_sentido_prod_ux",
    "lente_b_estrutura_arq",
    "lente_c_realidade_ops",
    "proposta_original_resumo",
    "visao_do_enxame",
    "justificativa_decisao",
    "executive_verdict",
    "risco_principal",
    "risco_linha_vermelha",
    "observacoes",
    "ouro_a_extrair",
    "deep_pattern",
    "transplantable_core",
    "logic_math_heuristic",
    "real_structural_problem",
    "categoria_nuance_tecnica",
    "integracao_papel_exato",
    "must_components_prod_ux",
    "must_components_arq",
    "must_components_ops",
    "detected_toxic_deps",
    "do_not_absorb",
    "where_ai_should_not_enter",
    "classificacao_terminal",
    "acao_de_canibalizacao",
    "categoria_arquitetural",
    "horizonte_extracao",
    "tipo_integracao",
    "capability_nature_primary",
    "architectural_topology",
    "temporal_stability",
    "bare_metal_fit",
    "extractability_level",
    "runtime_sovereignty_fit",
    "local_first_fit",
    "adoptability_level",
    "longitudinal_sustainability",
    "maintenance_burden",
    "onboarding_friction",
    "observability_operational",
    "recoverability_level",
    "degradation_behavior",
    "curation_burden",
    "evolution_cost",
    "operability_level",
    "abandonment_risk",
    "time_to_first_clear_value",
    "imperfection_tolerance",
    "discipline_dependency",
    "entropy_risk",
    "design_misuse_risk",
    "intrinsic_ethics_risk",
    "regulatory_risk",
    "score_philosophical_fit",
    "score_bare_metal_fit",
    "score_architectural_extractability",
    "score_operability",
    "score_runtime_sovereignty",
    "score_model_logic_value",
    "score_ethics_safety",
    "score_creep_risk",
    "score_intrinsic_risk",
    "score_final",
    "score_fit_geral_soda",
    "score_architectural_priority",
    "score_human_product_priority",
    "score_absorption_readiness",
    "score_operational_priority",
    "score_sustainability_adjusted_fit",
    "valid_from",
    "valid_to",
    "embargo_status",
];

pub fn build_batch_update_payload(
    row_number_1based: u32,
    row: &MasterSolutionsRow,
) -> HashMap<String, Vec<Vec<String>>> {
    let mut map = HashMap::new();
    map.insert(sheet_range_for_row(row_number_1based), vec![row.to_sheet_row()]);
    map
}

fn clamp_0_10(value: f64) -> f64 {
    value.clamp(0.0, 10.0)
}

fn round_1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

pub fn apply_phase4_block5(now_epoch: i64, row: &mut MasterSolutionsRow) {
    let creep_good = (10 - row.score_creep_risk).max(0) as f64;
    let intrinsic_good = (10 - row.score_intrinsic_risk).max(0) as f64;
    let positive = [
        row.score_philosophical_fit as f64,
        row.score_bare_metal_fit as f64,
        row.score_architectural_extractability as f64,
        row.score_operability as f64,
        row.score_runtime_sovereignty as f64,
        row.score_model_logic_value as f64,
        row.score_ethics_safety as f64,
        creep_good,
        intrinsic_good,
    ];
    let score_final = positive.iter().sum::<f64>() / (positive.len() as f64);

    let fit_raw = 0.05 * (row.score_philosophical_fit as f64)
        + 0.15 * (row.score_bare_metal_fit as f64)
        + 0.15 * (row.score_architectural_extractability as f64)
        + 0.10 * (row.score_operability as f64)
        + 0.15 * (row.score_runtime_sovereignty as f64)
        + 0.30 * (row.score_model_logic_value as f64)
        + 0.10 * (row.score_ethics_safety as f64)
        - 0.10 * ((row.score_creep_risk as f64) / 10.0) * 10.0
        - 0.10 * ((row.score_intrinsic_risk as f64) / 10.0) * 10.0;

    let architectural_priority = (row.score_model_logic_value + row.score_architectural_extractability + row.score_bare_metal_fit) as f64 / 3.0;
    let human_product_priority = (row.score_philosophical_fit + row.score_ethics_safety) as f64 / 2.0;
    let absorption_readiness = (row.score_architectural_extractability as f64
        + row.score_runtime_sovereignty as f64
        + creep_good
        + intrinsic_good)
        / 4.0;
    let operational_priority =
        (row.score_operability as f64 + row.score_ethics_safety as f64 + intrinsic_good) / 3.0;
    let sustainability_adjusted_fit = fit_raw - 0.2 * ((row.score_creep_risk + row.score_intrinsic_risk) as f64) / 2.0;

    row.score_final = round_1(clamp_0_10(score_final));
    row.score_fit_geral_soda = round_1(clamp_0_10(fit_raw));
    row.score_architectural_priority = round_1(clamp_0_10(architectural_priority));
    row.score_human_product_priority = round_1(clamp_0_10(human_product_priority));
    row.score_absorption_readiness = round_1(clamp_0_10(absorption_readiness));
    row.score_operational_priority = round_1(clamp_0_10(operational_priority));
    row.score_sustainability_adjusted_fit = round_1(clamp_0_10(sustainability_adjusted_fit));

    row.valid_from = now_epoch;
    let stable = matches!(row.temporal_stability, TemporalStability::Stable);
    row.valid_to = if stable {
        None
    } else {
        Some(now_epoch.saturating_add(180 * 24 * 60 * 60))
    };

    let scores = [
        row.score_philosophical_fit,
        row.score_bare_metal_fit,
        row.score_architectural_extractability,
        row.score_operability,
        row.score_runtime_sovereignty,
        row.score_model_logic_value,
        row.score_ethics_safety,
    ];
    let high = scores.iter().filter(|&&v| v >= 9).count();
    row.embargo_status = if high >= 5 && row.score_creep_risk <= 1 && row.score_intrinsic_risk <= 1 {
        1
    } else {
        0
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockFormatterClient {
        calls: Arc<Mutex<Vec<String>>>,
        models: Arc<Mutex<Vec<String>>>,
        responses: Arc<Mutex<Vec<Result<String, String>>>>,
    }

    impl MockFormatterClient {
        fn new(responses: Vec<Result<String, String>>) -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                models: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    impl FormatterClient for MockFormatterClient {
        fn format<'a>(
            &'a self,
            model: &'a str,
            prompt: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
            Box::pin(async move {
                self.calls.lock().await.push(prompt.to_string());
                self.models.lock().await.push(model.to_string());
                let mut guard = self.responses.lock().await;
                if guard.is_empty() {
                    return Err("no more responses".to_string());
                }
                guard.remove(0)
            })
        }
    }

    fn block0() -> Block0Context {
        Block0Context {
            status_atualizacao: "PENDENTE".to_string(),
            status_fase: "F3".to_string(),
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_analised_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SODA_ETL".to_string(),
            licenca: "MIT".to_string(),
            stack_base: "Rust".to_string(),
            declared_description: "Desc".to_string(),
            proposta_original_resumo: None,
            categoria_arquitetural: None,
            lente_a_sentido_prod_ux: "{\"lens\":\"a\"}".to_string(),
            lente_b_estrutura_arq: "{\"lens\":\"b\"}".to_string(),
            lente_c_realidade_ops: "{\"lens\":\"c\"}".to_string(),
        }
    }

    fn phase3_cfg() -> Phase3Config {
        Phase3Config {
            model: OFFICIAL_FORMATTER_MODEL.to_string(),
            model_block3_candidates: Vec::new(),
            max_attempts_per_block: 3,
        }
    }

    fn block3_payload_json(
        fields: serde_json::Value,
        justifications: serde_json::Value,
    ) -> String {
        format!(
            "```json\n{}\n```",
            serde_json::json!({
                "fields": fields,
                "justifications": justifications
            })
        )
    }

    fn block3_homogeneous_fields_json() -> serde_json::Value {
        serde_json::json!({
            "classificacao_terminal": "APROVADO_PARA_PRODUCAO",
            "acao_de_canibalizacao":"NENHUMA",
            "categoria_arquitetural":"UILibrary",
            "horizonte_extracao":"MEDIUM",
            "tipo_integracao":"INTEGRATE_AS_COMPONENT",
            "capability_nature_primary":"LIBRARY",
            "architectural_topology":"MODULAR",
            "temporal_stability":"STABLE",
            "bare_metal_fit":"MEDIUM",
            "extractability_level":"MEDIUM",
            "runtime_sovereignty_fit":"MEDIUM",
            "local_first_fit":"MEDIUM",
            "adoptability_level":"MEDIUM",
            "longitudinal_sustainability":"MEDIUM",
            "maintenance_burden":"MEDIUM",
            "onboarding_friction":"MEDIUM",
            "observability_operational":"MEDIUM",
            "recoverability_level":"MEDIUM",
            "degradation_behavior":"ACCEPTABLE",
            "curation_burden":"MEDIUM",
            "evolution_cost":"MEDIUM",
            "operability_level":"MEDIUM",
            "abandonment_risk":"MEDIUM",
            "time_to_first_clear_value":"MEDIUM",
            "imperfection_tolerance":"MEDIUM",
            "entropy_risk":"MEDIUM",
            "design_misuse_risk":"MEDIUM",
            "intrinsic_ethics_risk":"MEDIUM",
            "discipline_dependency":"MEDIA",
            "regulatory_risk":"MEDIUM"
        })
    }

    fn block3_discriminated_fields_json() -> serde_json::Value {
        serde_json::json!({
            "classificacao_terminal": "APROVADO_COM_RESSALVAS",
            "acao_de_canibalizacao":"ABSORVER_LOGICA",
            "categoria_arquitetural":"UILibrary",
            "horizonte_extracao":"SHORT",
            "tipo_integracao":"INTEGRATE_AS_COMPONENT",
            "capability_nature_primary":"LIBRARY",
            "architectural_topology":"MODULAR",
            "temporal_stability":"STABLE",
            "bare_metal_fit":"HIGH",
            "extractability_level":"HIGH",
            "runtime_sovereignty_fit":"HIGH",
            "local_first_fit":"MEDIUM",
            "adoptability_level":"HIGH",
            "longitudinal_sustainability":"MEDIUM",
            "maintenance_burden":"LOW",
            "onboarding_friction":"MEDIUM",
            "observability_operational":"HIGH",
            "recoverability_level":"HIGH",
            "degradation_behavior":"GRACEFUL",
            "curation_burden":"LOW",
            "evolution_cost":"LOW",
            "operability_level":"HIGH",
            "abandonment_risk":"LOW",
            "time_to_first_clear_value":"SHORT",
            "imperfection_tolerance":"HIGH",
            "entropy_risk":"LOW",
            "design_misuse_risk":"LOW",
            "intrinsic_ethics_risk":"LOW",
            "discipline_dependency":"BAIXA",
            "regulatory_risk":"LOW"
        })
    }

    #[test]
    fn extracts_json_code_fence_strictly() {
        let text = "aaa\n```json\n{\"ok\":true}\n```\nbbb";
        let extracted = extract_json_fence(text).unwrap();
        assert_eq!(extracted, "{\"ok\":true}");
    }

    #[test]
    fn extracts_raw_json_when_no_code_fence_is_present() {
        let text = "{\"ok\":true}";
        let extracted = extract_json_fence(text).unwrap();
        assert_eq!(extracted, "{\"ok\":true}");
    }

    #[test]
    fn terminal_checkpoint_does_not_override_incomplete_payload_stage() {
        assert_eq!(reconcile_checkpoint_stage(5, 0), 0);
        assert_eq!(reconcile_checkpoint_stage(5, 1), 1);
        assert_eq!(reconcile_checkpoint_stage(5, 4), 4);
        assert_eq!(reconcile_checkpoint_stage(5, 5), 5);
        assert_eq!(reconcile_checkpoint_stage(2, 3), 3);
    }

    #[test]
    fn block2a_requires_indicacao_and_all_narrative_fields() {
        let mut row = MasterSolutionsRow {
            indicacao_otimista_canibalizacao: String::new(),
            ouro_a_extrair: "ouro".into(),
            deep_pattern: "pattern".into(),
            transplantable_core: "core".into(),
            logic_math_heuristic: "heuristic".into(),
            real_structural_problem: "problem".into(),
            categoria_nuance_tecnica: "nuance".into(),
            integracao_papel_exato: "integracao".into(),
            ..Default::default()
        };
        assert!(!is_block2a_complete(&row));

        row.indicacao_otimista_canibalizacao = "canibalizar".into();
        assert!(is_block2a_complete(&row));
    }

    #[test]
    fn detects_homogeneous_block3_payload() {
        let payload = serde_json::json!({
            "fields": {
                "adoptability_level":"MEDIUM",
                "bare_metal_fit":"MEDIUM",
                "maintenance_burden":"MEDIUM",
                "runtime_sovereignty_fit":"MEDIUM",
                "longitudinal_sustainability":"MEDIUM",
                "local_first_fit":"MEDIUM",
                "onboarding_friction":"MEDIUM",
                "observability_operational":"MEDIUM",
                "recoverability_level":"MEDIUM",
                "degradation_behavior":"ACCEPTABLE",
                "curation_burden":"MEDIUM",
                "evolution_cost":"MEDIUM",
                "operability_level":"MEDIUM",
                "imperfection_tolerance":"MEDIUM",
                "discipline_dependency":"MEDIA",
                "abandonment_risk":"MEDIUM",
                "design_misuse_risk":"MEDIUM",
                "intrinsic_ethics_risk":"MEDIUM",
                "entropy_risk":"MEDIUM",
                "regulatory_risk":"MEDIUM"
            }
        });
        assert!(block3_looks_homogeneous(&payload));

        let payload_non_homogeneous = serde_json::json!({
            "fields": {
                "adoptability_level":"LOW",
                "bare_metal_fit":"HIGH",
                "maintenance_burden":"VERY_HIGH",
                "runtime_sovereignty_fit":"HIGH",
                "longitudinal_sustainability":"LOW",
                "local_first_fit":"HIGH",
                "onboarding_friction":"HIGH",
                "observability_operational":"VERY_LOW",
                "recoverability_level":"VERY_LOW",
                "degradation_behavior":"CATASTROPHIC",
                "curation_burden":"HIGH",
                "evolution_cost":"HIGH",
                "operability_level":"LOW",
                "imperfection_tolerance":"LOW",
                "discipline_dependency":"ALTA",
                "abandonment_risk":"CRITICAL",
                "design_misuse_risk":"CRITICAL",
                "intrinsic_ethics_risk":"LOW",
                "entropy_risk":"CRITICAL",
                "regulatory_risk":"LOW"
            }
        });
        assert!(!block3_looks_homogeneous(&payload_non_homogeneous));
    }

    #[test]
    fn block3_models_default_to_preferred_order() {
        let cfg = phase3_cfg();
        assert_eq!(
            cfg.block3_models(),
            vec![
                "qwen/qwen3.7-plus".to_string(),
                "moonshotai/kimi-k2.5".to_string(),
                "openai/gpt-5.4-mini".to_string(),
            ]
        );
    }

    #[test]
    fn homogeneous_medium_only_conflicts_when_scores_leave_middle_band() {
        let consistent = Block4Fields {
            score_philosophical_fit: 5,
            score_bare_metal_fit: 5,
            score_architectural_extractability: 6,
            score_operability: 4,
            score_creep_risk: 5,
            score_runtime_sovereignty: 5,
            score_model_logic_value: 5,
            score_ethics_safety: 5,
            score_intrinsic_risk: 5,
        };
        assert!(!homogeneous_medium_conflicts_with_block4(&consistent));

        let conflicting = Block4Fields {
            score_philosophical_fit: 5,
            score_bare_metal_fit: 9,
            score_architectural_extractability: 8,
            score_operability: 2,
            score_creep_risk: 5,
            score_runtime_sovereignty: 8,
            score_model_logic_value: 5,
            score_ethics_safety: 5,
            score_intrinsic_risk: 5,
        };
        assert!(homogeneous_medium_conflicts_with_block4(&conflicting));
    }

    #[tokio::test]
    async fn block3_fallback_tries_next_model_after_weak_homogeneous_medium() {
        let weak_justifications = serde_json::json!({
            "adoptability_level": "mesma justificativa para tudo",
            "bare_metal_fit": "mesma justificativa para tudo",
            "runtime_sovereignty_fit": "mesma justificativa para tudo",
            "local_first_fit": "mesma justificativa para tudo",
            "maintenance_burden": "mesma justificativa para tudo",
            "observability_operational": "mesma justificativa para tudo",
            "recoverability_level": "mesma justificativa para tudo",
            "operability_level": "mesma justificativa para tudo"
        });
        let strong_justifications = serde_json::json!({
            "adoptability_level": "A adocao fica alta porque a API central ja isola o fluxo principal.",
            "bare_metal_fit": "O nucleo roda sem dependencia interpretada e respeita integracao local.",
            "runtime_sovereignty_fit": "A execucao principal permanece sob controle local com pouca dependencia externa.",
            "local_first_fit": "A proposta funciona offline em boa parte do fluxo e sincroniza depois.",
            "maintenance_burden": "A manutencao e baixa porque o escopo da biblioteca e pequeno e previsivel.",
            "observability_operational": "A instrumentacao e boa porque os eventos criticos sao claros e rastreaveis.",
            "recoverability_level": "A recuperacao e alta porque o estado pode ser reconstruido sem cascata longa.",
            "operability_level": "A operacao e alta porque o setup e objetivo e o caminho de suporte e curto."
        });
        let responses = vec![
            Ok(block3_payload_json(
                block3_homogeneous_fields_json(),
                weak_justifications,
            )),
            Ok(block3_payload_json(
                block3_discriminated_fields_json(),
                strong_justifications,
            )),
        ];
        let client = MockFormatterClient::new(responses);
        let mut cfg = phase3_cfg();
        cfg.max_attempts_per_block = 1;
        let row = MasterSolutionsRow::from_block0(block0());

        let result = run_block3_with_fallback(&client, &cfg, &block0(), &row, 0, None)
            .await
            .unwrap();

        assert_eq!(result.model_used, "moonshotai/kimi-k2.5");
        let models = client.models.lock().await;
        assert_eq!(
            *models,
            vec![
                "qwen/qwen3.7-plus".to_string(),
                "moonshotai/kimi-k2.5".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn retries_up_to_three_injecting_error() {
        let responses = vec![
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"x\"}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"r\",\"declared_description_ptbr\":\"Descricao\",\"visao_do_enxame\":\"v\",\"justificativa_decisao\":\"j\",\"executive_verdict\":\"t\",\"risco_principal\":\"rp\",\"risco_linha_vermelha\":\"rlv\",\"observacoes\":\"o\"}, \"justifications\": {\"proposta_original_resumo\":\"k\"}}\n```".to_string()),
        ];
        let client = MockFormatterClient::new(responses);
        let cfg = phase3_cfg();
        let initial_row = MasterSolutionsRow::from_block0(block0());

        let res = run_block::<Block1Fields>(&client, &cfg, BLOCK_1, &block0(), &initial_row).await;
        assert!(res.is_ok());

        let calls = client.calls.lock().await;
        assert_eq!(calls.len(), 2);
        assert!(calls[1].contains("PREVIOUS_SCHEMA_ERROR"));
    }

    #[test]
    fn finops_skip_omits_proposta_original_resumo_from_block1_field_keys_when_already_present() {
        let block0 = block0();
        let mut prior = MasterSolutionsRow::from_block0(block0.clone());
        prior.proposta_original_resumo = "Resumo vindo do N2".to_string();

        let prompt = build_prompt(1, &block0, &prior, None);
        let marker = "FIELDS_KEYS_EXATAS:\n";
        let start = prompt.find(marker).unwrap() + marker.len();
        let end = prompt[start..].find('\n').unwrap() + start;
        let json_list = &prompt[start..end];
        let keys: Vec<String> = serde_json::from_str(json_list).unwrap();

        assert!(!keys.contains(&"proposta_original_resumo".to_string()));
    }

    #[test]
    fn finops_skip_omits_categoria_arquitetural_from_block3_field_keys_when_already_present() {
        let block0 = block0();
        let mut prior = MasterSolutionsRow::from_block0(block0.clone());
        prior.categoria_arquitetural = ArchitecturalCategory::UiLibrary;

        let prompt = build_prompt(3, &block0, &prior, None);
        let marker = "FIELDS_KEYS_EXATAS:\n";
        let start = prompt.find(marker).unwrap() + marker.len();
        let end = prompt[start..].find('\n').unwrap() + start;
        let json_list = &prompt[start..end];
        let keys: Vec<String> = serde_json::from_str(json_list).unwrap();

        assert!(!keys.contains(&"categoria_arquitetural".to_string()));
    }

    #[test]
    fn block3_injects_cure_fields_when_stack_base_or_licenca_are_unknown() {
        let block0 = block0();
        let mut prior = MasterSolutionsRow::from_block0(block0.clone());
        prior.stack_base = "UNKNOWN".to_string();
        prior.licenca = "UNKNOWN".to_string();

        let prompt = build_prompt(3, &block0, &prior, None);
        let marker = "FIELDS_KEYS_EXATAS:\n";
        let start = prompt.find(marker).unwrap() + marker.len();
        let end = prompt[start..].find('\n').unwrap() + start;
        let json_list = &prompt[start..end];
        let keys: Vec<String> = serde_json::from_str(json_list).unwrap();

        assert!(keys.contains(&"stack_base".to_string()));
        assert!(keys.contains(&"licenca".to_string()));
        assert!(prompt.contains("CURA_STACK_BASE"));
        assert!(prompt.contains("CURA_LICENCA"));
    }

    #[test]
    fn cure_normalizers_accept_known_aliases_and_reject_unknown() {
        assert_eq!(normalize_stack_base_cure_value("typescript"), Some("NodeJS"));
        assert_eq!(normalize_stack_base_cure_value("UNKNOWN"), None);
        assert_eq!(normalize_licenca_cure_value("Apache 2.0"), Some("Apache-2.0".to_string()));
        assert_eq!(
            normalize_licenca_cure_value("nao especificado"),
            Some("NÃO ESPECIFICADO".to_string())
        );
        assert_eq!(normalize_licenca_cure_value("UNKNOWN"), None);
    }

    #[test]
    fn batch_payload_maps_dynamic_range_and_82_columns() {
        let row = MasterSolutionsRow::from_block0(block0());
        let payload = build_batch_update_payload(2, &row);
        let range = sheet_range_for_row(2);
        let rows = payload.get(&range).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 82);
        assert_eq!(rows[0][0], "owner / repo");
    }

    #[test]
    fn canonical_82_columns_are_covered_by_block_contracts_or_phase4_derivation() {
        let mut covered = BTreeSet::new();
        for name in BLOCK0_CONTEXT_COLUMNS
            .iter()
            .chain(BLOCK1_FIELDS_COLUMNS.iter())
            .chain(BLOCK2A_FIELDS_COLUMNS.iter())
            .chain(BLOCK2B_FIELDS_COLUMNS.iter())
            .chain(BLOCK3_FIELDS_COLUMNS.iter())
            .chain(BLOCK4_FIELDS_COLUMNS.iter())
            .chain(PHASE4_DERIVED_COLUMNS.iter())
        {
            covered.insert(*name);
        }
        covered.insert("declared_description");
        covered.remove("declared_description_ptbr");

        // Remove internal/sqlite-only columns not present in MASTER_SOLUTIONS_CANONICAL_COLUMNS
        covered.remove("status_atualizacao");
        covered.remove("status_fase");
        covered.remove("indicacao_otimista_canibalizacao");

        // Map repo_analised_version to repo_version
        if covered.remove("repo_analised_version") {
            covered.insert("repo_version");
        }

        let expected: BTreeSet<&str> = MASTER_SOLUTIONS_CANONICAL_COLUMNS.iter().copied().collect();
        assert_eq!(covered, expected);
    }

    #[test]
    fn phase4_populates_block5_locally() {
        let mut row = MasterSolutionsRow::from_block0(block0());
        row.temporal_stability = TemporalStability::Evolving;
        row.score_philosophical_fit = 8;
        row.score_bare_metal_fit = 9;
        row.score_architectural_extractability = 10;
        row.score_operability = 7;
        row.score_creep_risk = 2;
        row.score_runtime_sovereignty = 9;
        row.score_model_logic_value = 10;
        row.score_ethics_safety = 8;
        row.score_intrinsic_risk = 1;

        apply_phase4_block5(1_000, &mut row);

        assert_eq!(row.valid_from, 1_000);
        assert_eq!(row.valid_to, Some(1_000 + 180 * 24 * 60 * 60));
        assert!(row.score_final > 0.0);
        assert!(row.score_fit_geral_soda > 0.0);
    }

    #[test]
    fn lens_json_is_reduced_to_bullets_only() {
        let raw = r#"{"lens_id":"SODA_LENS_A","bullets":["um","dois"]}"#;
        let out = normalize_lens_bullets(raw);
        assert_eq!(out, "- um\n- dois");
    }

    #[test]
    fn list_fields_are_humanized_from_json_array_string() {
        let raw = r#"["a","b"]"#;
        let out = normalize_pydantic_list_field(raw);
        assert_eq!(out, "- a\n- b");
    }

    #[test]
    fn sheet_row_exports_floats_as_dot_strings_and_valid_to_as_empty() {
        let row = MasterSolutionsRow {
            status_atualizacao: "CONCLUIDO".to_string(),
            status_fase: "F4".to_string(),
            project_name: "owner/repo".to_string(),
            score_final: 1.2,
            score_fit_geral_soda: 2.3,
            score_architectural_priority: 3.4,
            score_human_product_priority: 4.5,
            score_absorption_readiness: 5.6,
            score_operational_priority: 6.7,
            score_sustainability_adjusted_fit: 7.8,
            valid_from: 1_700_000_000,
            valid_to: None,
            embargo_status: 0,
            ..Default::default()
        };
        let arr = row.to_sheet_row();
        let idx = |name: &str| {
            MASTER_SOLUTIONS_CANONICAL_COLUMNS
                .iter()
                .position(|col| *col == name)
                .unwrap()
        };
        assert_eq!(arr.len(), 82);
        assert_eq!(arr[idx("score_final")], "1.2");
        assert_eq!(arr[idx("score_fit_geral_soda")], "2.3");
        assert_eq!(arr[idx("score_architectural_priority")], "3.4");
        assert_eq!(arr[idx("score_human_product_priority")], "4.5");
        assert_eq!(arr[idx("score_absorption_readiness")], "5.6");
        assert_eq!(arr[idx("score_operational_priority")], "6.7");
        assert_eq!(arr[idx("score_sustainability_adjusted_fit")], "7.8");
        assert_eq!(arr[idx("valid_from")], format_epoch_utc(1_700_000_000));
        assert_eq!(arr[idx("valid_to")], "");
        assert_eq!(arr[idx("embargo_status")], "LIVRE");
    }

    #[test]
    fn master_row_accepts_ptbr_enum_aliases_and_string_floats() {
        let mut value = serde_json::to_value(MasterSolutionsRow::default()).unwrap();
        let obj = value.as_object_mut().unwrap();
        obj.insert("classificacao_terminal".to_string(), serde_json::json!("APPROVED_WITH_REMARKS"));
        obj.insert("acao_de_canibalizacao".to_string(), serde_json::json!("ABSORB_LOGIC"));
        obj.insert("bare_metal_fit".to_string(), serde_json::json!("MÉDIA"));
        obj.insert("abandonment_risk".to_string(), serde_json::json!("CRÍTICO"));
        obj.insert("discipline_dependency".to_string(), serde_json::json!("MEDIUM"));
        obj.insert("score_final".to_string(), serde_json::json!("8,5"));
        obj.insert("score_fit_geral_soda".to_string(), serde_json::json!("8.7"));
        obj.insert("score_architectural_priority".to_string(), serde_json::json!(9));
        obj.insert("score_human_product_priority".to_string(), serde_json::json!(null));
        obj.insert("score_absorption_readiness".to_string(), serde_json::json!("7,1"));
        obj.insert("score_operational_priority".to_string(), serde_json::json!("6.4"));
        obj.insert("score_sustainability_adjusted_fit".to_string(), serde_json::json!("5,0"));

        let row: MasterSolutionsRow = serde_json::from_value(value).unwrap();
        assert_eq!(
            row.classificacao_terminal,
            TerminalClassification::AbsorbPartially
        );
        assert_eq!(row.acao_de_canibalizacao, CannibalizationAction::Concept);
        assert_eq!(row.bare_metal_fit, FitLevel4::Medium);
        assert_eq!(row.abandonment_risk, RiskLevel4::Critical);
        assert_eq!(row.discipline_dependency, DisciplineDependency::Media);
        assert_eq!(row.score_final, 8.5);
        assert_eq!(row.score_fit_geral_soda, 8.7);
        assert_eq!(row.score_architectural_priority, 9.0);
        assert_eq!(row.score_human_product_priority, 0.0);
        assert_eq!(row.score_absorption_readiness, 7.1);
        assert_eq!(row.score_operational_priority, 6.4);
        assert_eq!(row.score_sustainability_adjusted_fit, 5.0);
    }

    #[test]
    fn block3_invalid_enum_fails_parse_under_strict_catalog() {
        let payload = serde_json::json!({
            "classificacao_terminal": "FORA_DO_CATALOGO",
            "acao_de_canibalizacao": "FORA_DO_CATALOGO",
            "categoria_arquitetural": "FORA_DO_CATALOGO",
            "horizonte_extracao": "FORA_DO_CATALOGO",
            "tipo_integracao": "FORA_DO_CATALOGO",
            "capability_nature_primary": "FORA_DO_CATALOGO",
            "architectural_topology": "FORA_DO_CATALOGO",
            "temporal_stability": "FORA_DO_CATALOGO",
            "bare_metal_fit": "FORA_DO_CATALOGO",
            "extractability_level": "FORA_DO_CATALOGO",
            "runtime_sovereignty_fit": "FORA_DO_CATALOGO",
            "local_first_fit": "FORA_DO_CATALOGO",
            "adoptability_level": "FORA_DO_CATALOGO",
            "longitudinal_sustainability": "FORA_DO_CATALOGO",
            "maintenance_burden": "FORA_DO_CATALOGO",
            "onboarding_friction": "FORA_DO_CATALOGO",
            "observability_operational": "FORA_DO_CATALOGO",
            "recoverability_level": "FORA_DO_CATALOGO",
            "degradation_behavior": "FORA_DO_CATALOGO",
            "curation_burden": "FORA_DO_CATALOGO",
            "evolution_cost": "FORA_DO_CATALOGO",
            "operability_level": "FORA_DO_CATALOGO",
            "abandonment_risk": "FORA_DO_CATALOGO",
            "time_to_first_clear_value": "FORA_DO_CATALOGO",
            "imperfection_tolerance": "FORA_DO_CATALOGO",
            "entropy_risk": "FORA_DO_CATALOGO",
            "design_misuse_risk": "FORA_DO_CATALOGO",
            "intrinsic_ethics_risk": "FORA_DO_CATALOGO",
            "discipline_dependency": "FORA_DO_CATALOGO",
            "regulatory_risk": "FORA_DO_CATALOGO"
        });

        let parsed = serde_json::from_value::<Block3Fields>(payload);
        assert!(parsed.is_err());
    }
}
