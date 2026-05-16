#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SgrPayload {
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub repo_url: String,
    #[serde(default)]
    pub repo_version: String,
    #[serde(default)]
    pub ultima_versao_online: Option<String>,
    #[serde(default)]
    pub lote_id: String,
    #[serde(default)]
    pub data_ultima_analise: i64,
    #[serde(default)]
    pub analise_origem: String,
    #[serde(default)]
    pub declared_description: String,
    #[serde(default)]
    pub proposta_original_resumo: String,
    #[serde(default)]
    pub stack_base: String,
    #[serde(default)]
    pub licenca: Option<String>,
    #[serde(default)]
    pub lente_a_sentido_prod_ux: Option<String>,
    #[serde(default)]
    pub lente_b_estrutura_arq: Option<String>,
    #[serde(default)]
    pub lente_c_realidade_ops: Option<String>,
    #[serde(default)]
    pub visao_do_enxame: String,
    #[serde(default)]
    pub justificativa_decisao: String,
    #[serde(default)]
    pub executive_verdict: TerminalClassification,
    #[serde(default)]
    pub classificacao_terminal: String,
    #[serde(default)]
    pub acao_de_canibalizacao: CannibalizationAction,
    #[serde(default)]
    pub categoria_arquitetural: String,
    #[serde(default)]
    pub horizonte_extracao: String,
    #[serde(default)]
    pub tipo_integracao: String,
    #[serde(default)]
    pub categoria_nuance_tecnica: String,
    #[serde(default)]
    pub integracao_papel_exato: String,
    #[serde(default)]
    pub ouro_a_extrair: String,
    #[serde(default)]
    pub deep_pattern: String,
    #[serde(default)]
    pub transplantable_core: String,
    #[serde(default)]
    pub logic_math_heuristic: String,
    #[serde(default)]
    pub real_structural_problem: String,
    #[serde(default)]
    pub must_components_prod_ux: String,
    #[serde(default)]
    pub must_components_arq: String,
    #[serde(default)]
    pub must_components_ops: String,
    #[serde(default)]
    pub detected_toxic_deps: String,
    #[serde(default)]
    pub do_not_absorb: String,
    #[serde(default)]
    pub where_ai_should_not_enter: String,
    #[serde(default)]
    pub bare_metal_fit: String,
    #[serde(default)]
    pub extractability_level: String,
    #[serde(default)]
    pub operability_level: String,
    #[serde(default)]
    pub entropy_risk: String,
    #[serde(default)]
    pub design_misuse_risk: String,
    #[serde(default)]
    pub intrinsic_ethics_risk: String,
    #[serde(default)]
    pub discipline_dependency: String,
    #[serde(default)]
    pub risco_principal: String,
    #[serde(default)]
    pub risco_linha_vermelha: String,
    #[serde(default)]
    pub observacoes: String,
    #[serde(default)]
    pub score_final: f64,
    #[serde(default)]
    pub score_fit_geral_soda: f64,
    #[serde(default)]
    pub score_philosophical_fit: i64,
    #[serde(default)]
    pub score_bare_metal_fit: i64,
    #[serde(default)]
    pub score_architectural_extractability: i64,
    #[serde(default)]
    pub score_operability: i64,
    #[serde(default)]
    pub score_creep_risk: i64,
    #[serde(default)]
    pub score_runtime_sovereignty: i64,
    #[serde(default)]
    pub score_model_logic_value: i64,
    #[serde(default)]
    pub score_ethics_safety: i64,
    #[serde(default)]
    pub score_intrinsic_risk: i64,
    #[serde(default)]
    pub capability_nature_primary: String,
    #[serde(default)]
    pub architectural_topology: String,
    #[serde(default)]
    pub runtime_sovereignty_fit: String,
    #[serde(default)]
    pub local_first_fit: String,
    #[serde(default)]
    pub temporal_stability: String,
    #[serde(default)]
    pub adoptability_level: String,
    #[serde(default)]
    pub longitudinal_sustainability: String,
    #[serde(default)]
    pub abandonment_risk: String,
    #[serde(default)]
    pub maintenance_burden: String,
    #[serde(default)]
    pub onboarding_friction: String,
    #[serde(default)]
    pub observability_operational: String,
    #[serde(default)]
    pub recoverability_level: String,
    #[serde(default)]
    pub degradation_behavior: String,
    #[serde(default)]
    pub curation_burden: String,
    #[serde(default)]
    pub time_to_first_clear_value: String,
    #[serde(default)]
    pub imperfection_tolerance: String,
    #[serde(default)]
    pub evolution_cost: String,
    #[serde(default)]
    pub regulatory_risk: String,
    #[serde(default)]
    pub score_architectural_priority: f64,
    #[serde(default)]
    pub score_human_product_priority: f64,
    #[serde(default)]
    pub score_absorption_readiness: f64,
    #[serde(default)]
    pub score_operational_priority: f64,
    #[serde(default)]
    pub score_sustainability_adjusted_fit: f64,
    #[serde(default)]
    pub valid_from: i64,
    #[serde(default)]
    pub valid_to: Option<i64>,
    #[serde(default)]
    pub embargo_status: i64,
}

INSERT OR REPLACE INTO repo_heuristics (
    project_name, repo_url, repo_version, ultima_versao_online, lote_id, data_ultima_analise, analise_origem, declared_description, proposta_original_resumo, stack_base, licenca, lente_a_sentido_prod_ux, lente_b_estrutura_arq, lente_c_realidade_ops, visao_do_enxame, justificativa_decisao, executive_verdict, classificacao_terminal, acao_de_canibalizacao, categoria_arquitetural, horizonte_extracao, tipo_integracao, categoria_nuance_tecnica, integracao_papel_exato, ouro_a_extrair, deep_pattern, transplantable_core, logic_math_heuristic, real_structural_problem, must_components_prod_ux, must_components_arq, must_components_ops, detected_toxic_deps, do_not_absorb, where_ai_should_not_enter, bare_metal_fit, extractability_level, operability_level, entropy_risk, design_misuse_risk, intrinsic_ethics_risk, discipline_dependency, risco_principal, risco_linha_vermelha, observacoes, score_final, score_fit_geral_soda, score_philosophical_fit, score_bare_metal_fit, score_architectural_extractability, score_operability, score_creep_risk, score_runtime_sovereignty, score_model_logic_value, score_ethics_safety, score_intrinsic_risk, capability_nature_primary, architectural_topology, runtime_sovereignty_fit, local_first_fit, temporal_stability, adoptability_level, longitudinal_sustainability, abandonment_risk, maintenance_burden, onboarding_friction, observability_operational, recoverability_level, degradation_behavior, curation_burden, time_to_first_clear_value, imperfection_tolerance, evolution_cost, regulatory_risk, score_architectural_priority, score_human_product_priority, score_absorption_readiness, score_operational_priority, score_sustainability_adjusted_fit, valid_from, valid_to, embargo_status
) VALUES (
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, ?81, ?82
)

rusqlite::params![
    payload.project_name,
    payload.repo_url,
    payload.repo_version,
    payload.ultima_versao_online,
    payload.lote_id,
    payload.data_ultima_analise,
    payload.analise_origem,
    payload.declared_description,
    payload.proposta_original_resumo,
    payload.stack_base,
    payload.licenca,
    payload.lente_a_sentido_prod_ux,
    payload.lente_b_estrutura_arq,
    payload.lente_c_realidade_ops,
    payload.visao_do_enxame,
    payload.justificativa_decisao,
    format!(\"{:?}\", payload.executive_verdict),
    payload.classificacao_terminal,
    format!(\"{:?}\", payload.acao_de_canibalizacao),
    payload.categoria_arquitetural,
    payload.horizonte_extracao,
    payload.tipo_integracao,
    payload.categoria_nuance_tecnica,
    payload.integracao_papel_exato,
    payload.ouro_a_extrair,
    payload.deep_pattern,
    payload.transplantable_core,
    payload.logic_math_heuristic,
    payload.real_structural_problem,
    payload.must_components_prod_ux,
    payload.must_components_arq,
    payload.must_components_ops,
    payload.detected_toxic_deps,
    payload.do_not_absorb,
    payload.where_ai_should_not_enter,
    payload.bare_metal_fit,
    payload.extractability_level,
    payload.operability_level,
    payload.entropy_risk,
    payload.design_misuse_risk,
    payload.intrinsic_ethics_risk,
    payload.discipline_dependency,
    payload.risco_principal,
    payload.risco_linha_vermelha,
    payload.observacoes,
    payload.score_final,
    payload.score_fit_geral_soda,
    payload.score_philosophical_fit,
    payload.score_bare_metal_fit,
    payload.score_architectural_extractability,
    payload.score_operability,
    payload.score_creep_risk,
    payload.score_runtime_sovereignty,
    payload.score_model_logic_value,
    payload.score_ethics_safety,
    payload.score_intrinsic_risk,
    payload.capability_nature_primary,
    payload.architectural_topology,
    payload.runtime_sovereignty_fit,
    payload.local_first_fit,
    payload.temporal_stability,
    payload.adoptability_level,
    payload.longitudinal_sustainability,
    payload.abandonment_risk,
    payload.maintenance_burden,
    payload.onboarding_friction,
    payload.observability_operational,
    payload.recoverability_level,
    payload.degradation_behavior,
    payload.curation_burden,
    payload.time_to_first_clear_value,
    payload.imperfection_tolerance,
    payload.evolution_cost,
    payload.regulatory_risk,
    payload.score_architectural_priority,
    payload.score_human_product_priority,
    payload.score_absorption_readiness,
    payload.score_operational_priority,
    payload.score_sustainability_adjusted_fit,
    payload.valid_from,
    payload.valid_to,
    payload.embargo_status,
]

let batch_payload = vec![
    json!(vec![
        json!(payload.project_name),
        json!(payload.repo_url),
        json!(payload.repo_version),
        json!(payload.ultima_versao_online),
        json!(payload.lote_id),
        json!(payload.data_ultima_analise),
        json!(payload.analise_origem),
        json!(payload.declared_description),
        json!(payload.proposta_original_resumo),
        json!(payload.stack_base),
        json!(payload.licenca),
        json!(payload.lente_a_sentido_prod_ux),
        json!(payload.lente_b_estrutura_arq),
        json!(payload.lente_c_realidade_ops),
        json!(payload.visao_do_enxame),
        json!(payload.justificativa_decisao),
        json!(format!(\"{:?}\", payload.executive_verdict)),
        json!(payload.classificacao_terminal),
        json!(format!(\"{:?}\", payload.acao_de_canibalizacao)),
        json!(payload.categoria_arquitetural),
        json!(payload.horizonte_extracao),
        json!(payload.tipo_integracao),
        json!(payload.categoria_nuance_tecnica),
        json!(payload.integracao_papel_exato),
        json!(payload.ouro_a_extrair),
        json!(payload.deep_pattern),
        json!(payload.transplantable_core),
        json!(payload.logic_math_heuristic),
        json!(payload.real_structural_problem),
        json!(payload.must_components_prod_ux),
        json!(payload.must_components_arq),
        json!(payload.must_components_ops),
        json!(payload.detected_toxic_deps),
        json!(payload.do_not_absorb),
        json!(payload.where_ai_should_not_enter),
        json!(payload.bare_metal_fit),
        json!(payload.extractability_level),
        json!(payload.operability_level),
        json!(payload.entropy_risk),
        json!(payload.design_misuse_risk),
        json!(payload.intrinsic_ethics_risk),
        json!(payload.discipline_dependency),
        json!(payload.risco_principal),
        json!(payload.risco_linha_vermelha),
        json!(payload.observacoes),
        json!(payload.score_final),
        json!(payload.score_fit_geral_soda),
        json!(payload.score_philosophical_fit),
        json!(payload.score_bare_metal_fit),
        json!(payload.score_architectural_extractability),
        json!(payload.score_operability),
        json!(payload.score_creep_risk),
        json!(payload.score_runtime_sovereignty),
        json!(payload.score_model_logic_value),
        json!(payload.score_ethics_safety),
        json!(payload.score_intrinsic_risk),
        json!(payload.capability_nature_primary),
        json!(payload.architectural_topology),
        json!(payload.runtime_sovereignty_fit),
        json!(payload.local_first_fit),
        json!(payload.temporal_stability),
        json!(payload.adoptability_level),
        json!(payload.longitudinal_sustainability),
        json!(payload.abandonment_risk),
        json!(payload.maintenance_burden),
        json!(payload.onboarding_friction),
        json!(payload.observability_operational),
        json!(payload.recoverability_level),
        json!(payload.degradation_behavior),
        json!(payload.curation_burden),
        json!(payload.time_to_first_clear_value),
        json!(payload.imperfection_tolerance),
        json!(payload.evolution_cost),
        json!(payload.regulatory_risk),
        json!(payload.score_architectural_priority),
        json!(payload.score_human_product_priority),
        json!(payload.score_absorption_readiness),
        json!(payload.score_operational_priority),
        json!(payload.score_sustainability_adjusted_fit),
        json!(payload.valid_from),
        json!(payload.valid_to),
        json!(payload.embargo_status),
    ])
];
