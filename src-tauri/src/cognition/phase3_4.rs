use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const OFFICIAL_FORMATTER_MODEL: &str = "anthropic/claude-sonnet-4.6";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block0Context {
    pub project_name: String,
    pub repo_url: String,
    pub repo_version: String,
    pub ultima_versao_online: String,
    pub lote_id: String,
    pub data_ultima_analise: i64,
    pub analise_origem: String,
    pub licenca: String,
    pub stack_base: String,
    pub declared_description: String,
    pub lente_a_sentido_prod_ux: String,
    pub lente_b_estrutura_arq: String,
    pub lente_c_realidade_ops: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterSolutionsRow {
    pub project_name: String,
    pub repo_url: String,
    pub repo_version: String,
    pub ultima_versao_online: String,
    pub lote_id: String,
    pub data_ultima_analise: i64,
    pub analise_origem: String,
    pub licenca: String,
    pub stack_base: String,
    pub declared_description: String,
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
    pub classificacao_terminal: String,
    pub acao_de_canibalizacao: String,
    pub categoria_arquitetural: String,
    pub horizonte_extracao: String,
    pub tipo_integracao: String,
    pub capability_nature_primary: String,
    pub architectural_topology: String,
    pub temporal_stability: String,
    pub bare_metal_fit: String,
    pub extractability_level: String,
    pub runtime_sovereignty_fit: String,
    pub local_first_fit: String,
    pub adoptability_level: String,
    pub longitudinal_sustainability: String,
    pub maintenance_burden: String,
    pub onboarding_friction: String,
    pub observability_operational: String,
    pub recoverability_level: String,
    pub degradation_behavior: String,
    pub curation_burden: String,
    pub evolution_cost: String,
    pub operability_level: String,
    pub abandonment_risk: String,
    pub time_to_first_clear_value: String,
    pub imperfection_tolerance: String,
    pub entropy_risk: String,
    pub design_misuse_risk: String,
    pub intrinsic_ethics_risk: String,
    pub discipline_dependency: String,
    pub regulatory_risk: String,
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
            project_name: String::new(),
            repo_url: String::new(),
            repo_version: String::new(),
            ultima_versao_online: String::new(),
            lote_id: String::new(),
            data_ultima_analise: 0,
            analise_origem: String::new(),
            licenca: String::new(),
            stack_base: String::new(),
            declared_description: String::new(),
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
            classificacao_terminal: String::new(),
            acao_de_canibalizacao: String::new(),
            categoria_arquitetural: String::new(),
            horizonte_extracao: String::new(),
            tipo_integracao: String::new(),
            capability_nature_primary: String::new(),
            architectural_topology: String::new(),
            temporal_stability: String::new(),
            bare_metal_fit: String::new(),
            extractability_level: String::new(),
            runtime_sovereignty_fit: String::new(),
            local_first_fit: String::new(),
            adoptability_level: String::new(),
            longitudinal_sustainability: String::new(),
            maintenance_burden: String::new(),
            onboarding_friction: String::new(),
            observability_operational: String::new(),
            recoverability_level: String::new(),
            degradation_behavior: String::new(),
            curation_burden: String::new(),
            evolution_cost: String::new(),
            operability_level: String::new(),
            abandonment_risk: String::new(),
            time_to_first_clear_value: String::new(),
            imperfection_tolerance: String::new(),
            entropy_risk: String::new(),
            design_misuse_risk: String::new(),
            intrinsic_ethics_risk: String::new(),
            discipline_dependency: String::new(),
            regulatory_risk: String::new(),
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
        let mut row = Self::default();
        row.project_name = block0.project_name;
        row.repo_url = block0.repo_url;
        row.repo_version = block0.repo_version;
        row.ultima_versao_online = block0.ultima_versao_online;
        row.lote_id = block0.lote_id;
        row.data_ultima_analise = block0.data_ultima_analise;
        row.analise_origem = block0.analise_origem;
        row.licenca = block0.licenca;
        row.stack_base = block0.stack_base;
        row.declared_description = block0.declared_description;
        row.lente_a_sentido_prod_ux = block0.lente_a_sentido_prod_ux;
        row.lente_b_estrutura_arq = block0.lente_b_estrutura_arq;
        row.lente_c_realidade_ops = block0.lente_c_realidade_ops;
        row
    }

    pub fn to_sheet_row(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!(&self.project_name),
            serde_json::json!(&self.repo_url),
            serde_json::json!(&self.repo_version),
            serde_json::json!(&self.ultima_versao_online),
            serde_json::json!(&self.lote_id),
            serde_json::json!(&self.data_ultima_analise),
            serde_json::json!(&self.analise_origem),
            serde_json::json!(&self.licenca),
            serde_json::json!(&self.stack_base),
            serde_json::json!(&self.declared_description),
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
            serde_json::json!(&self.classificacao_terminal),
            serde_json::json!(&self.acao_de_canibalizacao),
            serde_json::json!(&self.categoria_arquitetural),
            serde_json::json!(&self.horizonte_extracao),
            serde_json::json!(&self.tipo_integracao),
            serde_json::json!(&self.capability_nature_primary),
            serde_json::json!(&self.architectural_topology),
            serde_json::json!(&self.temporal_stability),
            serde_json::json!(&self.bare_metal_fit),
            serde_json::json!(&self.extractability_level),
            serde_json::json!(&self.runtime_sovereignty_fit),
            serde_json::json!(&self.local_first_fit),
            serde_json::json!(&self.adoptability_level),
            serde_json::json!(&self.longitudinal_sustainability),
            serde_json::json!(&self.maintenance_burden),
            serde_json::json!(&self.onboarding_friction),
            serde_json::json!(&self.observability_operational),
            serde_json::json!(&self.recoverability_level),
            serde_json::json!(&self.degradation_behavior),
            serde_json::json!(&self.curation_burden),
            serde_json::json!(&self.evolution_cost),
            serde_json::json!(&self.operability_level),
            serde_json::json!(&self.abandonment_risk),
            serde_json::json!(&self.time_to_first_clear_value),
            serde_json::json!(&self.imperfection_tolerance),
            serde_json::json!(&self.entropy_risk),
            serde_json::json!(&self.design_misuse_risk),
            serde_json::json!(&self.intrinsic_ethics_risk),
            serde_json::json!(&self.discipline_dependency),
            serde_json::json!(&self.regulatory_risk),
            serde_json::json!(&self.score_philosophical_fit),
            serde_json::json!(&self.score_bare_metal_fit),
            serde_json::json!(&self.score_architectural_extractability),
            serde_json::json!(&self.score_operability),
            serde_json::json!(&self.score_creep_risk),
            serde_json::json!(&self.score_runtime_sovereignty),
            serde_json::json!(&self.score_model_logic_value),
            serde_json::json!(&self.score_ethics_safety),
            serde_json::json!(&self.score_intrinsic_risk),
            serde_json::json!(&self.score_final),
            serde_json::json!(&self.score_fit_geral_soda),
            serde_json::json!(&self.score_architectural_priority),
            serde_json::json!(&self.score_human_product_priority),
            serde_json::json!(&self.score_absorption_readiness),
            serde_json::json!(&self.score_operational_priority),
            serde_json::json!(&self.score_sustainability_adjusted_fit),
            serde_json::json!(&self.valid_from),
            match self.valid_to {
                Some(value) => serde_json::json!(value),
                None => serde_json::Value::Null,
            },
            serde_json::json!(&self.embargo_status),
        ]
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
    proposta_original_resumo: String,
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
    classificacao_terminal: String,
    acao_de_canibalizacao: String,
    categoria_arquitetural: String,
    horizonte_extracao: String,
    tipo_integracao: String,
    capability_nature_primary: String,
    architectural_topology: String,
    temporal_stability: String,
    bare_metal_fit: String,
    extractability_level: String,
    runtime_sovereignty_fit: String,
    local_first_fit: String,
    adoptability_level: String,
    longitudinal_sustainability: String,
    maintenance_burden: String,
    onboarding_friction: String,
    observability_operational: String,
    recoverability_level: String,
    degradation_behavior: String,
    curation_burden: String,
    evolution_cost: String,
    operability_level: String,
    abandonment_risk: String,
    time_to_first_clear_value: String,
    imperfection_tolerance: String,
    entropy_risk: String,
    design_misuse_risk: String,
    intrinsic_ethics_risk: String,
    discipline_dependency: String,
    regulatory_risk: String,
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
    if let Some(error) = last_error {
        prompt.push_str("\nPREVIOUS_SCHEMA_ERROR:\n");
        prompt.push_str(error);
        prompt.push('\n');
    }
    prompt.push_str("\nCONTEXT_ROW_PARTIAL_JSON:\n");
    prompt.push_str(&serde_json::to_string(prior).unwrap_or_default());
    prompt
}

async fn run_block<T: for<'de> Deserialize<'de> + Send>(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block: u8,
    block0: &Block0Context,
    row: &MasterSolutionsRow,
) -> Result<T, Phase3Error> {
    let mut last_error: Option<String> = None;
    let attempts = cfg.max_attempts_per_block.max(1);
    for attempt in 1..=attempts {
        let prompt = build_prompt(block, block0, row, last_error.as_deref());
        let formatted = client
            .format(&cfg.model, &prompt)
            .await
            .map_err(Phase3Error::Transport)?;
        let json_text = match extract_json_fence(&formatted) {
            Ok(json) => json,
            Err(err) => {
                last_error = Some(err.to_string());
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
            Ok(envelope) => return Ok(envelope.fields),
            Err(e) => {
                last_error = Some(e.to_string());
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

pub async fn run_phase3_sgr(
    client: &dyn FormatterClient,
    cfg: &Phase3Config,
    block0: Block0Context,
) -> Result<Phase3Output, Phase3Error> {
    let mut row = MasterSolutionsRow::from_block0(block0.clone());

    let block1: Block1Fields = run_block(client, cfg, 1, &block0, &row).await?;
    row.proposta_original_resumo = block1.proposta_original_resumo;
    row.visao_do_enxame = block1.visao_do_enxame;
    row.justificativa_decisao = block1.justificativa_decisao;
    row.executive_verdict = block1.executive_verdict;
    row.risco_principal = block1.risco_principal;
    row.risco_linha_vermelha = block1.risco_linha_vermelha;
    row.observacoes = block1.observacoes;

    let block2: Block2Fields = run_block(client, cfg, 2, &block0, &row).await?;
    row.ouro_a_extrair = block2.ouro_a_extrair;
    row.deep_pattern = block2.deep_pattern;
    row.transplantable_core = block2.transplantable_core;
    row.logic_math_heuristic = block2.logic_math_heuristic;
    row.real_structural_problem = block2.real_structural_problem;
    row.categoria_nuance_tecnica = block2.categoria_nuance_tecnica;
    row.integracao_papel_exato = block2.integracao_papel_exato;
    row.must_components_prod_ux = block2.must_components_prod_ux;
    row.must_components_arq = block2.must_components_arq;
    row.must_components_ops = block2.must_components_ops;
    row.detected_toxic_deps = block2.detected_toxic_deps;
    row.do_not_absorb = block2.do_not_absorb;
    row.where_ai_should_not_enter = block2.where_ai_should_not_enter;

    let block3: Block3Fields = run_block(client, cfg, 3, &block0, &row).await?;
    row.classificacao_terminal = block3.classificacao_terminal;
    row.acao_de_canibalizacao = block3.acao_de_canibalizacao;
    row.categoria_arquitetural = block3.categoria_arquitetural;
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

    let block4: Block4Fields = run_block(client, cfg, 4, &block0, &row).await?;
    validate_score_0_10("score_philosophical_fit", block4.score_philosophical_fit, 4)?;
    validate_score_0_10("score_bare_metal_fit", block4.score_bare_metal_fit, 4)?;
    validate_score_0_10(
        "score_architectural_extractability",
        block4.score_architectural_extractability,
        4,
    )?;
    validate_score_0_10("score_operability", block4.score_operability, 4)?;
    validate_score_0_10("score_creep_risk", block4.score_creep_risk, 4)?;
    validate_score_0_10(
        "score_runtime_sovereignty",
        block4.score_runtime_sovereignty,
        4,
    )?;
    validate_score_0_10("score_model_logic_value", block4.score_model_logic_value, 4)?;
    validate_score_0_10("score_ethics_safety", block4.score_ethics_safety, 4)?;
    validate_score_0_10("score_intrinsic_risk", block4.score_intrinsic_risk, 4)?;

    row.score_philosophical_fit = block4.score_philosophical_fit;
    row.score_bare_metal_fit = block4.score_bare_metal_fit;
    row.score_architectural_extractability = block4.score_architectural_extractability;
    row.score_operability = block4.score_operability;
    row.score_creep_risk = block4.score_creep_risk;
    row.score_runtime_sovereignty = block4.score_runtime_sovereignty;
    row.score_model_logic_value = block4.score_model_logic_value;
    row.score_ethics_safety = block4.score_ethics_safety;
    row.score_intrinsic_risk = block4.score_intrinsic_risk;

    Ok(Phase3Output {
        model_used: cfg.model.clone(),
        row,
    })
}

pub fn extract_json_fence(text: &str) -> Result<String, Phase3Error> {
    let start = text
        .find("```json")
        .ok_or_else(|| Phase3Error::JsonFenceParse("missing ```json fence".to_string()))?;
    let after = &text[start + "```json".len()..];
    let after = after
        .strip_prefix("\r\n")
        .or_else(|| after.strip_prefix('\n'))
        .unwrap_or(after);
    let end = after
        .find("```")
        .ok_or_else(|| Phase3Error::JsonFenceParse("missing closing ``` fence".to_string()))?;
    Ok(after[..end].trim().to_string())
}

pub fn sheet_range_for_row(row_number_1based: u32) -> String {
    format!("A{}:CD{}", row_number_1based, row_number_1based)
}

pub fn build_batch_update_payload(
    row_number_1based: u32,
    row: &MasterSolutionsRow,
) -> HashMap<String, Vec<Vec<serde_json::Value>>> {
    let mut map = HashMap::new();
    map.insert(sheet_range_for_row(row_number_1based), vec![row.to_sheet_row()]);
    map
}

fn clamp_0_10(value: f64) -> f64 {
    if value < 0.0 {
        0.0
    } else if value > 10.0 {
        10.0
    } else {
        value
    }
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
    let stable = row.temporal_stability.trim().eq_ignore_ascii_case("STABLE");
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
            project_name: "owner/repo".to_string(),
            repo_url: "https://github.com/owner/repo".to_string(),
            repo_version: "v1.0.0".to_string(),
            ultima_versao_online: "v1.0.1".to_string(),
            lote_id: "LOTE_01".to_string(),
            data_ultima_analise: 1_715_000_000,
            analise_origem: "SODA_ETL".to_string(),
            licenca: "MIT".to_string(),
            stack_base: "Rust".to_string(),
            declared_description: "Desc".to_string(),
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

    #[tokio::test]
    async fn retries_up_to_three_injecting_error() {
        let responses = vec![
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"x\"}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"x\"}, \"justifications\": {}}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"proposta_original_resumo\": \"r\",\"visao_do_enxame\":\"v\",\"justificativa_decisao\":\"j\",\"executive_verdict\":\"t\",\"risco_principal\":\"rp\",\"risco_linha_vermelha\":\"rlv\",\"observacoes\":\"o\"}, \"justifications\": {\"proposta_original_resumo\":\"k\"}}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"ouro_a_extrair\": \"1\",\"deep_pattern\":\"2\",\"transplantable_core\":\"3\",\"logic_math_heuristic\":\"4\",\"real_structural_problem\":\"5\",\"categoria_nuance_tecnica\":\"6\",\"integracao_papel_exato\":\"7\",\"must_components_prod_ux\":\"8\",\"must_components_arq\":\"9\",\"must_components_ops\":\"10\",\"detected_toxic_deps\":\"11\",\"do_not_absorb\":\"12\",\"where_ai_should_not_enter\":\"13\"}, \"justifications\": {\"ouro_a_extrair\":\"k\"}}\n```".to_string()),
            Ok("```json\n{\"fields\": {\"classificacao_terminal\": \"ct\",\"acao_de_canibalizacao\":\"ac\",\"categoria_arquitetural\":\"ca\",\"horizonte_extracao\":\"he\",\"tipo_integracao\":\"ti\",\"capability_nature_primary\":\"cnp\",\"architectural_topology\":\"at\",\"temporal_stability\":\"ts\",\"bare_metal_fit\":\"bm\",\"extractability_level\":\"el\",\"runtime_sovereignty_fit\":\"rs\",\"local_first_fit\":\"lf\",\"adoptability_level\":\"ad\",\"longitudinal_sustainability\":\"ls\",\"maintenance_burden\":\"mb\",\"onboarding_friction\":\"of\",\"observability_operational\":\"oo\",\"recoverability_level\":\"rl\",\"degradation_behavior\":\"db\",\"curation_burden\":\"cb\",\"evolution_cost\":\"ec\",\"operability_level\":\"op\",\"abandonment_risk\":\"ar\",\"time_to_first_clear_value\":\"tt\",\"imperfection_tolerance\":\"it\",\"entropy_risk\":\"er\",\"design_misuse_risk\":\"dm\",\"intrinsic_ethics_risk\":\"ie\",\"discipline_dependency\":\"dd\",\"regulatory_risk\":\"rr\"}, \"justifications\": {\"classificacao_terminal\":\"k\"}}\n```".to_string()),
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
    fn batch_payload_maps_a_to_cd_and_82_columns() {
        let row = MasterSolutionsRow::from_block0(block0());
        let payload = build_batch_update_payload(2, &row);
        let range = sheet_range_for_row(2);
        let rows = payload.get(&range).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 82);
        assert_eq!(rows[0][0], serde_json::json!("owner/repo"));
    }

    #[test]
    fn phase4_populates_block5_locally() {
        let mut row = MasterSolutionsRow::from_block0(block0());
        row.temporal_stability = "EVOLVING".to_string();
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
}
