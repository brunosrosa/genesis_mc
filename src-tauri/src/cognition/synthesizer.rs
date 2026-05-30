use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

pub const OFFICIAL_FORMATTER_MODEL: &str = "deepseek/deepseek-v4-pro";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TerminalClassification {
    #[default]
    #[serde(rename = "APROVADO_PARA_PRODUCAO")]
    AprovadoParaProducao,
    #[serde(rename = "APROVADO_COM_RESSALVAS")]
    AprovadoComRessalvas,
    #[serde(rename = "REJEITADO_DESCARTE")]
    RejeitadoDescarte,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl TerminalClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AprovadoParaProducao => "APROVADO_PARA_PRODUCAO",
            Self::AprovadoComRessalvas => "APROVADO_COM_RESSALVAS",
            Self::RejeitadoDescarte => "REJEITADO_DESCARTE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CannibalizationAction {
    #[default]
    #[serde(rename = "NENHUMA")]
    Nenhuma,
    #[serde(rename = "ABSORVER_LOGICA")]
    AbsorverLogica,
    #[serde(rename = "EXTRAIR_SCRIPTS")]
    ExtrairScripts,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl CannibalizationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nenhuma => "NENHUMA",
            Self::AbsorverLogica => "ABSORVER_LOGICA",
            Self::ExtrairScripts => "EXTRAIR_SCRIPTS",
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
    #[serde(rename = "Memoria_RAG")]
    MemoriaRag,
    #[serde(rename = "Roteamento_FinOps")]
    RoteamentoFinOps,
    #[serde(rename = "Orquestracao_Agentes")]
    OrquestracaoAgentes,
    #[serde(rename = "Model_Serving")]
    ModelServing,
    #[serde(rename = "Knowledge_Extraction")]
    KnowledgeExtraction,
    #[serde(rename = "Seguranca_Sandbox")]
    SegurancaSandbox,
    #[serde(rename = "Infraestrutura_Core")]
    InfraestruturaCore,
    #[serde(rename = "Tooling_Dev")]
    ToolingDev,
    #[serde(other)]
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
            "Memoria_RAG" => Self::MemoriaRag,
            "Roteamento_FinOps" => Self::RoteamentoFinOps,
            "Orquestracao_Agentes" => Self::OrquestracaoAgentes,
            "Model_Serving" => Self::ModelServing,
            "Knowledge_Extraction" => Self::KnowledgeExtraction,
            "Seguranca_Sandbox" => Self::SegurancaSandbox,
            "Infraestrutura_Core" => Self::InfraestruturaCore,
            "Tooling_Dev" => Self::ToolingDev,
            _ => {
                return Err(format!(
                    "categoria_arquitetural invalida: '{}'. Valores permitidos: CanvasUI, UILibrary, Memoria_RAG, Roteamento_FinOps, Orquestracao_Agentes, Model_Serving, Knowledge_Extraction, Seguranca_Sandbox, Infraestrutura_Core, Tooling_Dev",
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
            Self::MemoriaRag => "Memoria_RAG",
            Self::RoteamentoFinOps => "Roteamento_FinOps",
            Self::OrquestracaoAgentes => "Orquestracao_Agentes",
            Self::ModelServing => "Model_Serving",
            Self::KnowledgeExtraction => "Knowledge_Extraction",
            Self::SegurancaSandbox => "Seguranca_Sandbox",
            Self::InfraestruturaCore => "Infraestrutura_Core",
            Self::ToolingDev => "Tooling_Dev",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationType {
    #[default]
    IntegrateAsComponent,
    ReimplementInternally,
    Reject,
    Unknown,
}

impl IntegrationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IntegrateAsComponent => "INTEGRATE_AS_COMPONENT",
            Self::ReimplementInternally => "REIMPLEMENT_INTERNALLY",
            Self::Reject => "REJECT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityNaturePrimary {
    #[default]
    Library,
    Tooling,
    Service,
    Application,
    System,
    Algorithm,
    DataStructure,
    Unknown,
}

impl CapabilityNaturePrimary {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Library => "LIBRARY",
            Self::Tooling => "TOOLING",
            Self::Service => "SERVICE",
            Self::Application => "APPLICATION",
            Self::System => "SYSTEM",
            Self::Algorithm => "ALGORITHM",
            Self::DataStructure => "DATA_STRUCTURE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArchitecturalTopology {
    #[default]
    Modular,
    Monolith,
    Layered,
    Microservices,
    EventDriven,
    Pipeline,
    Plugin,
    Unknown,
}

impl ArchitecturalTopology {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Modular => "MODULAR",
            Self::Monolith => "MONOLITH",
            Self::Layered => "LAYERED",
            Self::Microservices => "MICROSERVICES",
            Self::EventDriven => "EVENT_DRIVEN",
            Self::Pipeline => "PIPELINE",
            Self::Plugin => "PLUGIN",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalStability {
    #[default]
    Stable,
    Evolving,
}

impl TemporalStability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stable => "STABLE",
            Self::Evolving => "EVOLVING",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FitLevel4 {
    #[default]
    Low,
    Medium,
    High,
    Excellent,
}

impl FitLevel4 {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Excellent => "EXCELLENT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel4 {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel4 {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DisciplineDependency {
    #[default]
    #[serde(rename = "NENHUMA")]
    Nenhuma,
    #[serde(rename = "BAIXA")]
    Baixa,
    #[serde(rename = "MEDIA")]
    Media,
    #[serde(rename = "ALTA")]
    Alta,
    #[serde(rename = "CRITICA")]
    Critica,
}

impl DisciplineDependency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nenhuma => "NENHUMA",
            Self::Baixa => "BAIXA",
            Self::Media => "MEDIA",
            Self::Alta => "ALTA",
            Self::Critica => "CRITICA",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DegradationBehavior {
    #[default]
    Graceful,
    Acceptable,
    Fragile,
    Catastrophic,
}

impl DegradationBehavior {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Graceful => "GRACEFUL",
            Self::Acceptable => "ACCEPTABLE",
            Self::Fragile => "FRAGILE",
            Self::Catastrophic => "CATASTROPHIC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Scale5 {
    #[default]
    VeryLow,
    Low,
    Medium,
    High,
    Excellent,
}

impl Scale5 {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VeryLow => "VERY_LOW",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Excellent => "EXCELLENT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BurdenLevel {
    #[default]
    Low,
    Medium,
    High,
    VeryHigh,
}

impl BurdenLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::VeryHigh => "VERY_HIGH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeHorizon {
    #[default]
    Immediate,
    Short,
    Medium,
    Long,
    VeryLong,
}

impl TimeHorizon {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Immediate => "IMMEDIATE",
            Self::Short => "SHORT",
            Self::Medium => "MEDIUM",
            Self::Long => "LONG",
            Self::VeryLong => "VERY_LONG",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block0Context {
    pub status_atualizacao: String,
    pub status_fase: String,
    pub project_name: String,
    pub repo_url: String,
    pub repo_analised_version: String,
    pub ultima_versao_online: String,
    pub lote_id: String,
    pub data_ultima_analise: i64,
    pub analise_origem: String,
    pub licenca: String,
    pub stack_base: String,
    pub declared_description: String,
    pub proposta_original_resumo: Option<String>,
    pub categoria_arquitetural: Option<String>,
    pub lente_a_sentido_prod_ux: String,
    pub lente_b_estrutura_arq: String,
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
    pub horizonte_extracao: TimeHorizon,
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
    pub score_final: f64,
    pub score_fit_geral_soda: f64,
    pub score_architectural_priority: f64,
    pub score_human_product_priority: f64,
    pub score_absorption_readiness: f64,
    pub score_operational_priority: f64,
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
            horizonte_extracao: TimeHorizon::default(),
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

    pub fn to_sheet_row(&self) -> Vec<serde_json::Value> {
        let pretty_project_name = self.project_name.replace("/", " / ");
        let declared_description = if self.declared_description_ptbr.trim().is_empty() {
            self.declared_description.clone()
        } else {
            self.declared_description_ptbr.clone()
        };

        let classificacao_terminal = to_human_readable(self.classificacao_terminal.as_str());
        let acao_de_canibalizacao = to_human_readable(self.acao_de_canibalizacao.as_str());
        let categoria_arquitetural = to_human_readable(self.categoria_arquitetural.as_str());
        let horizonte_extracao = to_human_readable(self.horizonte_extracao.as_str());
        let tipo_integracao = to_human_readable(self.tipo_integracao.as_str());
        let capability_nature_primary = to_human_readable(self.capability_nature_primary.as_str());
        let architectural_topology = to_human_readable(self.architectural_topology.as_str());
        let temporal_stability = to_human_readable(self.temporal_stability.as_str());
        let bare_metal_fit = to_human_readable(self.bare_metal_fit.as_str());
        let extractability_level = to_human_readable(self.extractability_level.as_str());
        let runtime_sovereignty_fit = to_human_readable(self.runtime_sovereignty_fit.as_str());
        let local_first_fit = to_human_readable(self.local_first_fit.as_str());
        let adoptability_level = to_human_readable(self.adoptability_level.as_str());
        let longitudinal_sustainability = to_human_readable(self.longitudinal_sustainability.as_str());
        let maintenance_burden = to_human_readable(self.maintenance_burden.as_str());
        let onboarding_friction = to_human_readable(self.onboarding_friction.as_str());
        let observability_operational = to_human_readable(self.observability_operational.as_str());
        let recoverability_level = to_human_readable(self.recoverability_level.as_str());
        let degradation_behavior = to_human_readable(self.degradation_behavior.as_str());
        let curation_burden = to_human_readable(self.curation_burden.as_str());
        let evolution_cost = to_human_readable(self.evolution_cost.as_str());
        let operability_level = to_human_readable(self.operability_level.as_str());
        let abandonment_risk = to_human_readable(self.abandonment_risk.as_str());
        let time_to_first_clear_value = to_human_readable(self.time_to_first_clear_value.as_str());
        let imperfection_tolerance = to_human_readable(self.imperfection_tolerance.as_str());
        let entropy_risk = to_human_readable(self.entropy_risk.as_str());
        let design_misuse_risk = to_human_readable(self.design_misuse_risk.as_str());
        let intrinsic_ethics_risk = to_human_readable(self.intrinsic_ethics_risk.as_str());
        let discipline_dependency = to_human_readable(self.discipline_dependency.as_str());
        let regulatory_risk = to_human_readable(self.regulatory_risk.as_str());

        let data_ultima_analise = format_epoch_utc(self.data_ultima_analise);
        let valid_from = format_epoch_utc(self.valid_from);
        let valid_to = self.valid_to.map(format_epoch_utc).unwrap_or_default();
        let embargo_status = embargo_label(self.embargo_status).to_string();

        Vec::from([
            serde_json::json!(&self.status_atualizacao),
            serde_json::json!(&self.status_fase),
            serde_json::json!(pretty_project_name),
            serde_json::json!(&self.repo_url),
            serde_json::json!(&self.repo_analised_version),
            serde_json::json!(&self.ultima_versao_online),
            serde_json::json!(&self.lote_id),
            serde_json::json!(data_ultima_analise),
            serde_json::json!(&self.analise_origem),
            serde_json::json!(&self.licenca),
            serde_json::json!(&self.stack_base),
            serde_json::json!(declared_description),
            serde_json::json!(&self.lente_a_sentido_prod_ux),
            serde_json::json!(&self.lente_b_estrutura_arq),
            serde_json::json!(&self.lente_c_realidade_ops),
            serde_json::json!(&self.proposta_original_resumo),
            serde_json::json!(&self.visao_do_enxame),
            serde_json::json!(&self.justificativa_decisao),
            serde_json::json!(&self.executive_verdict),
            serde_json::json!(&self.risco_principal),
            serde_json::json!(&self.risco_linha_vermelha),
            serde_json::json!(&self.observacoes),
            serde_json::json!(&self.ouro_a_extrair),
            serde_json::json!(&self.deep_pattern),
            serde_json::json!(&self.transplantable_core),
            serde_json::json!(&self.logic_math_heuristic),
            serde_json::json!(&self.real_structural_problem),
            serde_json::json!(&self.categoria_nuance_tecnica),
            serde_json::json!(&self.integracao_papel_exato),
            serde_json::json!(&self.must_components_prod_ux),
            serde_json::json!(&self.must_components_arq),
            serde_json::json!(&self.must_components_ops),
            serde_json::json!(&self.detected_toxic_deps),
            serde_json::json!(&self.do_not_absorb),
            serde_json::json!(&self.where_ai_should_not_enter),
            serde_json::json!(classificacao_terminal),
            serde_json::json!(acao_de_canibalizacao),
            serde_json::json!(categoria_arquitetural),
            serde_json::json!(horizonte_extracao),
            serde_json::json!(tipo_integracao),
            serde_json::json!(capability_nature_primary),
            serde_json::json!(architectural_topology),
            serde_json::json!(temporal_stability),
            serde_json::json!(bare_metal_fit),
            serde_json::json!(extractability_level),
            serde_json::json!(runtime_sovereignty_fit),
            serde_json::json!(local_first_fit),
            serde_json::json!(adoptability_level),
            serde_json::json!(longitudinal_sustainability),
            serde_json::json!(maintenance_burden),
            serde_json::json!(onboarding_friction),
            serde_json::json!(observability_operational),
            serde_json::json!(recoverability_level),
            serde_json::json!(degradation_behavior),
            serde_json::json!(curation_burden),
            serde_json::json!(evolution_cost),
            serde_json::json!(operability_level),
            serde_json::json!(abandonment_risk),
            serde_json::json!(time_to_first_clear_value),
            serde_json::json!(imperfection_tolerance),
            serde_json::json!(entropy_risk),
            serde_json::json!(design_misuse_risk),
            serde_json::json!(intrinsic_ethics_risk),
            serde_json::json!(discipline_dependency),
            serde_json::json!(regulatory_risk),
            serde_json::json!(&self.score_philosophical_fit),
            serde_json::json!(&self.score_bare_metal_fit),
            serde_json::json!(&self.score_architectural_extractability),
            serde_json::json!(&self.score_operability),
            serde_json::json!(&self.score_creep_risk),
            serde_json::json!(&self.score_runtime_sovereignty),
            serde_json::json!(&self.score_model_logic_value),
            serde_json::json!(&self.score_ethics_safety),
            serde_json::json!(&self.score_intrinsic_risk),
            serde_json::json!(format_float_1(self.score_final)),
            serde_json::json!(format_float_1(self.score_fit_geral_soda)),
            serde_json::json!(format_float_1(self.score_architectural_priority)),
            serde_json::json!(format_float_1(self.score_human_product_priority)),
            serde_json::json!(format_float_1(self.score_absorption_readiness)),
            serde_json::json!(format_float_1(self.score_operational_priority)),
            serde_json::json!(format_float_1(self.score_sustainability_adjusted_fit)),
            serde_json::json!(valid_from),
            serde_json::json!(valid_to),
            serde_json::json!(embargo_status),
        ])
    }
}

fn format_float_1(value: f64) -> String {
    let raw = format!("{:.1}", value);
    raw.replace(',', ".")
}

fn normalize_enum_value(raw: &str) -> String {
    let line = raw.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(line.len());
    for ch in line.chars() {
        let ch = match ch {
            '"' | '\'' | '`' => continue,
            '-' | '—' | '–' => '_',
            c if c.is_ascii_alphanumeric() || c == '_' => c,
            c if c.is_whitespace() => '_',
            _ => '_',
        };
        out.push(ch);
    }
    let mut compact = String::with_capacity(out.len());
    let mut last_underscore = false;
    for ch in out.chars() {
        if ch == '_' {
            if last_underscore {
                continue;
            }
            last_underscore = true;
            compact.push('_');
            continue;
        }
        last_underscore = false;
        compact.push(ch);
    }
    compact.trim_matches('_').to_ascii_uppercase()
}

fn to_human_readable(enum_str: &str) -> String {
    let token = normalize_enum_value(enum_str);
    if token.is_empty() {
        return String::new();
    }
    match token.as_str() {
        "LOW" => "Baixo".to_string(),
        "MEDIUM" => "Médio".to_string(),
        "HIGH" => "Alto".to_string(),
        "EXCELLENT" => "Excelente".to_string(),
        "VERY_LOW" => "Muito Baixo".to_string(),
        "VERY_HIGH" => "Muito Alto".to_string(),
        "CRITICAL" => "Crítico".to_string(),
        "NENHUMA" => "Nenhuma".to_string(),
        "BAIXA" => "Baixa".to_string(),
        "MEDIA" => "Média".to_string(),
        "ALTA" => "Alta".to_string(),
        "CRITICA" => "Crítica".to_string(),
        "STABLE" => "Estável".to_string(),
        "EVOLVING" => "Evolutivo".to_string(),
        "GRACEFUL" => "Suave".to_string(),
        "ACCEPTABLE" => "Aceitável".to_string(),
        "FRAGILE" => "Frágil".to_string(),
        "CATASTROPHIC" => "Catastrófico".to_string(),
        "IMMEDIATE" => "Imediato".to_string(),
        "SHORT" => "Curto".to_string(),
        "LONG" => "Longo".to_string(),
        "VERY_LONG" => "Muito Longo".to_string(),
        "INTEGRATE_AS_COMPONENT" => "Integração como Componente".to_string(),
        "REIMPLEMENT_INTERNALLY" => "Reimplementação Interna".to_string(),
        "REJECT" => "Rejeitar".to_string(),
        _ => humanize_token_title(&token),
    }
}

fn humanize_token_title(token: &str) -> String {
    let parts = token
        .split('_')
        .filter(|p| !p.trim().is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return String::new();
    }
    let mut out = Vec::new();
    for part in parts {
        out.push(humanize_word(part));
    }
    out.join(" ")
}

fn humanize_word(word: &str) -> String {
    let mapped = match word {
        "CANIBALIZACAO" => "Canibalização",
        "CIRURGICA" => "Cirúrgica",
        "INTEGRACAO" => "Integração",
        "COMPONENTE" => "Componente",
        "EFEMERO" => "Efêmero",
        "ORQUESTRACAO" => "Orquestração",
        "MEMORIA" => "Memória",
        "ARQUITETURA" => "Arquitetura",
        "OPERACAO" => "Operação",
        "SOBERANIA" => "Soberania",
        "LOGICA" => "Lógica",
        "REIMPLEMENTACAO" => "Reimplementação",
        "NATIVA" => "Nativa",
        "CRATE" => "Crate",
        "RISCO" => "Risco",
        "HUMANO" => "Humano",
        "PRODUTO" => "Produto",
        "CURTO" => "Curto",
        "LONGO" => "Longo",
        _ => "",
    };
    if !mapped.is_empty() {
        return mapped.to_string();
    }

    let lower = word.to_ascii_lowercase();
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut s = String::new();
    s.push(first.to_ascii_uppercase());
    s.extend(chars);
    s
}

fn format_epoch_utc(epoch: i64) -> String {
    if epoch <= 0 {
        return String::new();
    }
    let Some(dt) = DateTime::<Utc>::from_timestamp(epoch, 0) else {
        return String::new();
    };
    dt.with_timezone(&FixedOffset::west_opt(3 * 3600).unwrap())
        .to_rfc3339_opts(SecondsFormat::Secs, true)
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
    pub max_attempts_per_block: usize,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockResponse<T> {
    fields: T,
    justifications: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block1Fields {
    proposta_original_resumo: Option<String>,
    declared_description_ptbr: String,
    visao_do_enxame: String,
    justificativa_decisao: String,
    executive_verdict: String,
    risco_principal: String,
    risco_linha_vermelha: String,
    observacoes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block2Fields {
    ouro_a_extrair: String,
    deep_pattern: String,
    transplantable_core: String,
    logic_math_heuristic: String,
    real_structural_problem: String,
    categoria_nuance_tecnica: String,
    integracao_papel_exato: String,
    must_components_prod_ux: String,
    must_components_arq: String,
    must_components_ops: String,
    detected_toxic_deps: String,
    do_not_absorb: String,
    where_ai_should_not_enter: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block3Fields {
    classificacao_terminal: TerminalClassification,
    acao_de_canibalizacao: CannibalizationAction,
    categoria_arquitetural: Option<ArchitecturalCategory>,
    horizonte_extracao: TimeHorizon,
    tipo_integracao: IntegrationType,
    capability_nature_primary: CapabilityNaturePrimary,
    architectural_topology: ArchitecturalTopology,
    temporal_stability: TemporalStability,
    bare_metal_fit: FitLevel4,
    extractability_level: FitLevel4,
    runtime_sovereignty_fit: FitLevel4,
    local_first_fit: FitLevel4,
    adoptability_level: Scale5,
    longitudinal_sustainability: Scale5,
    maintenance_burden: BurdenLevel,
    onboarding_friction: BurdenLevel,
    observability_operational: Scale5,
    recoverability_level: Scale5,
    degradation_behavior: DegradationBehavior,
    curation_burden: BurdenLevel,
    evolution_cost: BurdenLevel,
    operability_level: FitLevel4,
    abandonment_risk: RiskLevel4,
    time_to_first_clear_value: TimeHorizon,
    imperfection_tolerance: Scale5,
    entropy_risk: RiskLevel4,
    design_misuse_risk: RiskLevel4,
    intrinsic_ethics_risk: RiskLevel4,
    discipline_dependency: DisciplineDependency,
    regulatory_risk: RiskLevel4,
}

impl Block3Fields {
    fn sanitize(self) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block4Fields {
    score_philosophical_fit: i64,
    score_bare_metal_fit: i64,
    score_architectural_extractability: i64,
    score_operability: i64,
    score_creep_risk: i64,
    score_runtime_sovereignty: i64,
    score_model_logic_value: i64,
    score_ethics_safety: i64,
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
    prompt.push_str(&format!("BLOCK={}\n", block));
    prompt.push_str(&format!("project_name={}\n", block0.project_name));
    prompt.push_str(&format!("repo_url={}\n", block0.repo_url));
    prompt.push_str("OUTPUT: responda com um bloco Markdown ```json ... ``` contendo um objeto JSON.\n");
    prompt.push_str("O JSON deve conter: {\"fields\":{...},\"justifications\":{...}}.\n");
    prompt.push_str("STRICTNESS: nenhum texto fora do code-fence. Nenhuma chave extra fora de fields/justifications. Em fields, use SOMENTE as chaves listadas para este bloco (todas obrigatórias).\n");
    prompt.push_str("FIELDS_KEYS_EXATAS:\n");
    prompt.push_str(&fields_keys_for_block(block, prior));
    prompt.push('\n');
    match block {
        1 => {
            prompt.push_str("LIMITS_BLOCK1: cada valor string em fields deve ter no máximo 600 caracteres.\n");
            prompt.push_str("TRANSLATE_BLOCK1: gere declared_description_ptbr como tradução fiel para PT-BR de project.declared_description. Comece com letra maiúscula. Não adicione comentários sobre tradução.\n");
        }
        2 => {
            prompt.push_str("LIMITS_BLOCK2: cada valor string em fields deve ter no máximo 400 caracteres. Para listas, escreva em bullets (ex: \"- item\\n- item\").\n");
        }
        3 => {
            prompt.push_str("LIMITS_BLOCK3: cada valor string em fields deve ter no máximo 180 caracteres. Use termos curtos, 1 linha por campo (sem parágrafos).\n");
            prompt.push_str("MODO_ROBOTICO_ENUMS_BLOCK3: para TODOS os campos ENUM do Bloco 3, fields deve conter APENAS o valor do catálogo (1 token). Qualquer explicação deve ir EXCLUSIVAMENTE em justifications[mesma_chave].\n");
            prompt.push_str("PROIBIDO: hífens, ':' , parênteses, frases, ou duas opções no mesmo campo.\n");
            prompt.push_str(enum_catalog_block3());
        }
        _ => {}
    }
    if block == 4 {
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
        1 => {
            let mut keys = vec![
                "proposta_original_resumo",
                "declared_description_ptbr",
                "visao_do_enxame",
                "justificativa_decisao",
                "executive_verdict",
                "risco_principal",
                "risco_linha_vermelha",
                "observacoes",
            ];
            if !prior.proposta_original_resumo.trim().is_empty() {
                keys.retain(|k| *k != "proposta_original_resumo");
            }
            serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string())
        }
        2 => r#"["ouro_a_extrair","deep_pattern","transplantable_core","logic_math_heuristic","real_structural_problem","categoria_nuance_tecnica","integracao_papel_exato","must_components_prod_ux","must_components_arq","must_components_ops","detected_toxic_deps","do_not_absorb","where_ai_should_not_enter"]"#.to_string(),
        3 => {
            let mut keys = vec![
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
            let categoria_is_present = !matches!(
                prior.categoria_arquitetural,
                ArchitecturalCategory::Unspecified | ArchitecturalCategory::Unknown
            );
            if categoria_is_present {
                keys.retain(|k| *k != "categoria_arquitetural");
            }
            serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string())
        }
        4 => r#"["score_philosophical_fit","score_bare_metal_fit","score_architectural_extractability","score_operability","score_creep_risk","score_runtime_sovereignty","score_model_logic_value","score_ethics_safety","score_intrinsic_risk"]"#.to_string(),
        _ => "[]".to_string(),
    }
}

fn enum_catalog_block3() -> &'static str {
    "CATALOGO_ENUMS_BLOCK3:\n\
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
discipline_dependency: NENHUMA|BAIXA|MEDIA|ALTA|CRITICA\n\
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

fn compact_context_for_block(
    block0: &Block0Context,
    row: &MasterSolutionsRow,
    block: u8,
) -> serde_json::Value {
    let mut ctx = serde_json::Map::new();
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
            "lente_a_sentido_prod_ux": &row.lente_a_sentido_prod_ux,
            "lente_b_estrutura_arq": &row.lente_b_estrutura_arq,
            "lente_c_realidade_ops": &row.lente_c_realidade_ops
        }),
    );

    if block >= 2 {
        ctx.insert(
            "block1".to_string(),
            serde_json::json!({
                "proposta_original_resumo": &row.proposta_original_resumo,
                "visao_do_enxame": &row.visao_do_enxame,
                "justificativa_decisao": &row.justificativa_decisao,
                "executive_verdict": &row.executive_verdict,
                "risco_principal": &row.risco_principal,
                "risco_linha_vermelha": &row.risco_linha_vermelha,
                "observacoes": &row.observacoes
            }),
        );
    }
    if block >= 3 {
        ctx.insert(
            "block2".to_string(),
            serde_json::json!({
                "ouro_a_extrair": &row.ouro_a_extrair,
                "deep_pattern": &row.deep_pattern,
                "transplantable_core": &row.transplantable_core,
                "logic_math_heuristic": &row.logic_math_heuristic,
                "real_structural_problem": &row.real_structural_problem,
                "categoria_nuance_tecnica": &row.categoria_nuance_tecnica,
                "integracao_papel_exato": &row.integracao_papel_exato,
                "must_components_prod_ux": &row.must_components_prod_ux,
                "must_components_arq": &row.must_components_arq,
                "must_components_ops": &row.must_components_ops,
                "detected_toxic_deps": &row.detected_toxic_deps,
                "do_not_absorb": &row.do_not_absorb,
                "where_ai_should_not_enter": &row.where_ai_should_not_enter
            }),
        );
    }
    if block >= 4 {
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

pub async fn run_phase3_sgr(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block0: Block0Context,
) -> Result<Phase3Output, Phase3Error> {
    info!(
        repo_id = %block0.project_name,
        model = %cfg.model,
        "F3 (Sintetizador SGR): iniciando SGR em cascata (Blocos 1..4)"
    );
    let mut row = MasterSolutionsRow::from_block0(block0.clone());

    let block1: Block1Fields = run_block(client, cfg, 1, &block0, &row).await?;
    if let Some(value) = block1.proposta_original_resumo {
        row.proposta_original_resumo = value;
    }
    row.declared_description_ptbr = block1.declared_description_ptbr;
    row.visao_do_enxame = block1.visao_do_enxame;
    row.justificativa_decisao = block1.justificativa_decisao;
    row.executive_verdict = block1.executive_verdict;
    row.risco_principal = block1.risco_principal;
    row.risco_linha_vermelha = block1.risco_linha_vermelha;
    row.observacoes = block1.observacoes;
    info!("F3 (Sintetizador SGR): Bloco 1 concluído");

    let block2: Block2Fields = run_block(client, cfg, 2, &block0, &row).await?;
    row.ouro_a_extrair = block2.ouro_a_extrair;
    row.deep_pattern = block2.deep_pattern;
    row.transplantable_core = block2.transplantable_core;
    row.logic_math_heuristic = block2.logic_math_heuristic;
    row.real_structural_problem = block2.real_structural_problem;
    row.categoria_nuance_tecnica = block2.categoria_nuance_tecnica;
    row.integracao_papel_exato = block2.integracao_papel_exato;
    row.must_components_prod_ux = normalize_pydantic_list_field(&block2.must_components_prod_ux);
    row.must_components_arq = normalize_pydantic_list_field(&block2.must_components_arq);
    row.must_components_ops = normalize_pydantic_list_field(&block2.must_components_ops);
    row.detected_toxic_deps = normalize_pydantic_list_field(&block2.detected_toxic_deps);
    row.do_not_absorb = normalize_pydantic_list_field(&block2.do_not_absorb);
    row.where_ai_should_not_enter = normalize_pydantic_list_field(&block2.where_ai_should_not_enter);
    info!("F3 (Sintetizador SGR): Bloco 2 concluído");

    let block3_env = run_block_envelope::<Block3Fields>(client, cfg, 3, &block0, &row).await?;
    let block3_justifications = block3_env.justifications;
    let block3 = block3_env.fields.sanitize();
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
    info!("F3 (Sintetizador SGR): Bloco 3 concluído");

    let block4: Block4Fields = run_block4_validated(client, cfg, &block0, &row).await?;

    row.score_philosophical_fit = block4.score_philosophical_fit;
    row.score_bare_metal_fit = block4.score_bare_metal_fit;
    row.score_architectural_extractability = block4.score_architectural_extractability;
    row.score_operability = block4.score_operability;
    row.score_creep_risk = block4.score_creep_risk;
    row.score_runtime_sovereignty = block4.score_runtime_sovereignty;
    row.score_model_logic_value = block4.score_model_logic_value;
    row.score_ethics_safety = block4.score_ethics_safety;
    row.score_intrinsic_risk = block4.score_intrinsic_risk;
    info!("F3 (Sintetizador SGR): Bloco 4 concluído");

    Ok(Phase3Output {
        model_used: cfg.model.clone(),
        row,
        block3_justifications,
    })
}

pub fn extract_json_fence(text: &str) -> Result<String, Phase3Error> {
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
    format!("A{}:CF{}", row_number_1based, row_number_1based)
}

pub const MASTER_SOLUTIONS_CANONICAL_COLUMNS: [&str; 84] = [
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
    "entropy_risk",
    "design_misuse_risk",
    "intrinsic_ethics_risk",
    "discipline_dependency",
    "regulatory_risk",
    "score_philosophical_fit",
    "score_bare_metal_fit",
    "score_architectural_extractability",
    "score_operability",
    "score_creep_risk",
    "score_runtime_sovereignty",
    "score_model_logic_value",
    "score_ethics_safety",
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
) -> HashMap<String, Vec<Vec<serde_json::Value>>> {
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
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockFormatterClient {
        calls: Arc<Mutex<Vec<String>>>,
        responses: Arc<Mutex<Vec<Result<String, String>>>>,
    }

    impl MockFormatterClient {
        fn new(responses: Vec<Result<String, String>>) -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    impl FormatterClient for MockFormatterClient {
        fn format<'a>(
            &'a self,
            _model: &'a str,
            prompt: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
            Box::pin(async move {
                self.calls.lock().await.push(prompt.to_string());
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

    #[tokio::test]
    async fn retries_up_to_three_injecting_error() {
        let responses = vec![
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"x\"}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"x\"}, \"justifications\": {}}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"r\",\"declared_description_ptbr\":\"Descricao\",\"visao_do_enxame\":\"v\",\"justificativa_decisao\":\"j\",\"executive_verdict\":\"t\",\"risco_principal\":\"rp\",\"risco_linha_vermelha\":\"rlv\",\"observacoes\":\"o\"}, \"justifications\": {\"proposta_original_resumo\":\"k\"}}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"ouro_a_extrair\": \"1\",\"deep_pattern\":\"2\",\"transplantable_core\":\"3\",\"logic_math_heuristic\":\"4\",\"real_structural_problem\":\"5\",\"categoria_nuance_tecnica\":\"6\",\"integracao_papel_exato\":\"7\",\"must_components_prod_ux\":\"8\",\"must_components_arq\":\"9\",\"must_components_ops\":\"10\",\"detected_toxic_deps\":\"11\",\"do_not_absorb\":\"12\",\"where_ai_should_not_enter\":\"13\"}, \"justifications\": {\"ouro_a_extrair\":\"k\"}}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"classificacao_terminal\": \"APROVADO_PARA_PRODUCAO\",\"acao_de_canibalizacao\":\"NENHUMA\",\"categoria_arquitetural\":\"LIBRARY\",\"horizonte_extracao\":\"SHORT\",\"tipo_integracao\":\"INTEGRATE_AS_COMPONENT\",\"capability_nature_primary\":\"LIBRARY\",\"architectural_topology\":\"MODULAR\",\"temporal_stability\":\"STABLE\",\"bare_metal_fit\":\"HIGH\",\"extractability_level\":\"HIGH\",\"runtime_sovereignty_fit\":\"HIGH\",\"local_first_fit\":\"HIGH\",\"adoptability_level\":\"HIGH\",\"longitudinal_sustainability\":\"HIGH\",\"maintenance_burden\":\"LOW\",\"onboarding_friction\":\"LOW\",\"observability_operational\":\"HIGH\",\"recoverability_level\":\"HIGH\",\"degradation_behavior\":\"GRACEFUL\",\"curation_burden\":\"LOW\",\"evolution_cost\":\"LOW\",\"operability_level\":\"HIGH\",\"abandonment_risk\":\"LOW\",\"time_to_first_clear_value\":\"SHORT\",\"imperfection_tolerance\":\"HIGH\",\"entropy_risk\":\"LOW\",\"design_misuse_risk\":\"LOW\",\"intrinsic_ethics_risk\":\"LOW\",\"discipline_dependency\":\"NENHUMA\",\"regulatory_risk\":\"LOW\"}, \"justifications\": {\"classificacao_terminal\":\"k\"}}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"score_philosophical_fit\": 1,\"score_bare_metal_fit\":2,\"score_architectural_extractability\":3,\"score_operability\":4,\"score_creep_risk\":5,\"score_runtime_sovereignty\":6,\"score_model_logic_value\":7,\"score_ethics_safety\":8,\"score_intrinsic_risk\":9}, \"justifications\": {\"score_philosophical_fit\":\"k\"}}\n```".to_string()),
        ];
        let client = MockFormatterClient::new(responses);
        let cfg = Phase3Config {
            model: OFFICIAL_FORMATTER_MODEL.to_string(),
            max_attempts_per_block: 3,
        };

        let res = run_phase3_sgr(&client, &cfg, block0()).await;
        assert!(res.is_ok());

        let calls = client.calls.lock().await;
        assert_eq!(calls.len(), 6);
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
    fn batch_payload_maps_a_to_cf_and_84_columns() {
        let row = MasterSolutionsRow::from_block0(block0());
        let payload = build_batch_update_payload(2, &row);
        let range = sheet_range_for_row(2);
        let rows = payload.get(&range).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 84);
        assert_eq!(rows[0][2], serde_json::json!("owner / repo"));
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
        let mut row = MasterSolutionsRow::default();
        row.status_atualizacao = "CONCLUIDO".to_string();
        row.status_fase = "F4".to_string();
        row.project_name = "owner/repo".to_string();
        row.score_final = 1.2;
        row.score_fit_geral_soda = 2.3;
        row.score_architectural_priority = 3.4;
        row.score_human_product_priority = 4.5;
        row.score_absorption_readiness = 5.6;
        row.score_operational_priority = 6.7;
        row.score_sustainability_adjusted_fit = 7.8;
        row.valid_from = 1_700_000_000;
        row.valid_to = None;
        row.embargo_status = 0;
        let arr = row.to_sheet_row();
        assert_eq!(arr.len(), 84);
        assert_eq!(arr[74], serde_json::json!("1.2"));
        assert_eq!(arr[75], serde_json::json!("2.3"));
        assert_eq!(arr[76], serde_json::json!("3.4"));
        assert_eq!(arr[77], serde_json::json!("4.5"));
        assert_eq!(arr[78], serde_json::json!("5.6"));
        assert_eq!(arr[79], serde_json::json!("6.7"));
        assert_eq!(arr[80], serde_json::json!("7.8"));
        assert_eq!(arr[81], serde_json::json!(format_epoch_utc(1_700_000_000)));
        assert_eq!(arr[82], serde_json::json!(""));
        assert_eq!(arr[83], serde_json::json!("LIVRE"));
    }
}
